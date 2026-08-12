use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use anyhow::Context;
use log::{error, info, warn};
use prop_rs_android::{resetprop::ResetProp, sys_prop};
use serde::{Deserialize, Serialize};
use zygisk_api::api::{V4, ZygiskApi};

const CONFIG_PATH: &str = "/data/adb/device_faker/config/config.toml";

// ── Companion 侧激活会话跟踪 ─────────────────────────────────────────────────
//
// companion 进程持续运行，static 状态可靠（不受 Zygisk 模块 DlClose 影响）。
// 每个 Apply 请求会先恢复上一个会话的备份，确保多应用并发时不会互相污染。

static ACTIVE_SESSION: Mutex<Option<ActiveSession>> = Mutex::new(None);

struct ActiveSession {
    package: String,
    pid: u32,
    backups: HashMap<String, String>,
}

/// 收割已退出的 watcher 子进程，避免僵尸进程积累。
fn reap_zombie_watchers() {
    loop {
        match unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) } {
            0 | -1 => break,
            _ => {} // 收割到一个僵尸，继续尝试
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CpuSpoofRequest {
    pub pid: u32,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CpuSpoofUnmountRequest {
    pub pid: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WriteLogRequest {
    pub lines: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReadConfigRequest {}

pub fn spoof_system_props_via_companion(
    api: &mut ZygiskApi<V4>,
    prop_map: &HashMap<String, String>,
    delete_props: &[String],
    package_name: &str,
) -> anyhow::Result<()> {
    if prop_map.is_empty() && delete_props.is_empty() {
        return Ok(());
    }

    let request = CompanionRequest::Apply(ResetpropSessionRequest {
        pid: std::process::id(),
        props: prop_map.clone(),
        delete_props: delete_props.to_vec(),
        package_name: package_name.to_string(),
    });

    let response = send_companion_command(api, &request)?;
    if response.status != 0 {
        anyhow::bail!(
            response
                .message
                .unwrap_or_else(|| "companion resetprop failed".to_string())
        );
    }

    // companion 侧现在自己管理会话状态和恢复逻辑；
    // Zygisk 模块侧不再需要 ACTIVE_RESET_SESSION。

    Ok(())
}

pub fn load_config_via_companion(api: &mut ZygiskApi<V4>) -> anyhow::Result<Option<String>> {
    let request = CompanionRequest::ReadConfig(ReadConfigRequest {});
    let response = send_companion_command(api, &request)?;
    if response.status != 0 {
        anyhow::bail!(
            response
                .message
                .unwrap_or_else(|| "companion failed to read config".to_string())
        );
    }

    Ok(response.config_content)
}

pub fn send_companion_command(
    api: &mut ZygiskApi<V4>,
    request: &CompanionRequest,
) -> anyhow::Result<CompanionResponse> {
    let payload = serde_json::to_vec(request)?;
    let response = api
        .with_companion(|stream| -> anyhow::Result<CompanionResponse> {
            stream.write_all(&(payload.len() as u32).to_le_bytes())?;
            stream.write_all(&payload)?;
            stream.flush()?;

            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf)?;
            let resp_len = u32::from_le_bytes(len_buf) as usize;
            let mut resp_buf = vec![0u8; resp_len];
            stream.read_exact(&mut resp_buf)?;

            let resp = serde_json::from_slice::<CompanionResponse>(&resp_buf)?;
            Ok(resp)
        })
        .map_err(|e| anyhow::anyhow!("Failed to talk to companion: {e}"))??;

    Ok(response)
}

pub fn handle_companion_request(stream: &mut UnixStream) {
    // companion 进程不会调用 ZygiskModule::on_load，因此需要自行初始化日志。
    #[cfg(target_os = "android")]
    crate::file_logger::init();

    let request = match read_companion_request(stream) {
        Ok(request) => request,
        Err(err) => {
            error!("Companion failed to parse request: {err}");
            let response = CompanionResponse::err("invalid request");
            if let Err(e) = write_companion_response(stream, &response) {
                warn!("Failed to write companion response: {e}");
            }
            return;
        }
    };

    match request {
        CompanionRequest::Apply(request) => {
            let response = match apply_resetprop_session(request) {
                Ok(backups) => CompanionResponse::ok_with_backups(backups),
                Err(err) => {
                    error!("Companion failed to apply resetprop session: {err}");
                    CompanionResponse::err(err.to_string())
                }
            };
            if let Err(e) = write_companion_response(stream, &response) {
                warn!("Failed to write companion response: {e}");
            }
        }
        CompanionRequest::Restore(request) => {
            let response = match restore_properties(request) {
                Ok(_) => CompanionResponse::ok(),
                Err(err) => {
                    error!("Companion failed to restore properties: {err}");
                    CompanionResponse::err(err.to_string())
                }
            };
            if let Err(e) = write_companion_response(stream, &response) {
                warn!("Failed to write companion response: {e}");
            }
        }
        CompanionRequest::CpuSpoof(request) => {
            crate::cpu_spoof::handle_companion_cpu_spoof(stream, request);
        }
        CompanionRequest::CpuSpoofUnmount(request) => {
            crate::cpu_spoof::handle_companion_cpu_unmount(stream, request);
        }
        CompanionRequest::WriteLog(request) => {
            let response = match write_log_lines(request) {
                Ok(_) => CompanionResponse::ok(),
                Err(err) => {
                    error!("Companion failed to write log: {err}");
                    CompanionResponse::err(err.to_string())
                }
            };
            if let Err(e) = write_companion_response(stream, &response) {
                warn!("Failed to write companion response: {e}");
            }
        }
        CompanionRequest::ReadConfig(_) => {
            let response = match read_config() {
                Ok(config_content) => CompanionResponse::ok_with_config(config_content),
                Err(err) => {
                    error!("Companion failed to read config: {err}");
                    CompanionResponse::err(err.to_string())
                }
            };
            if let Err(e) = write_companion_response(stream, &response) {
                warn!("Failed to write companion response: {e}");
            }
        }
    }
}

fn read_config() -> anyhow::Result<Option<String>> {
    read_config_file(Path::new(CONFIG_PATH))
}

fn read_config_file(path: &Path) -> anyhow::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => {
            Err(err).with_context(|| format!("Failed to read config at {}", path.display()))
        }
    }
}

fn read_companion_request(stream: &mut UnixStream) -> anyhow::Result<CompanionRequest> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let payload_len = u32::from_le_bytes(len_buf) as usize;
    if payload_len == 0 {
        anyhow::bail!("empty request payload");
    }

    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;
    let request = serde_json::from_slice::<CompanionRequest>(&payload)?;
    Ok(request)
}

pub(crate) fn write_companion_response(
    stream: &mut UnixStream,
    response: &CompanionResponse,
) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec(response)?;
    stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}

/// Rebuild property areas for ALL distinct contexts touched by the given keys.
/// More complete than single-context rebuild; handles custom_props spanning
/// multiple SELinux contexts (e.g. ro.* + debug.* + gsm.*).
fn rebuild_all_contexts(keys_iter: impl Iterator<Item = impl AsRef<str>>) {
    let mut contexts: std::collections::HashSet<String> = std::collections::HashSet::new();
    for key in keys_iter {
        if let Ok(ctx) = sys_prop::get_context(key.as_ref()) {
            contexts.insert(ctx);
        }
    }
    for ctx in &contexts {
        if let Err(e) = sys_prop::rebuild(ctx) {
            warn!("prop area rebuild for {ctx} failed (non-fatal): {e}");
        }
    }
}

fn apply_resetprop_session(
    request: ResetpropSessionRequest,
) -> anyhow::Result<HashMap<String, String>> {
    if request.props.is_empty() && request.delete_props.is_empty() {
        return Ok(HashMap::new());
    }

    // ① 收割已退出的 watcher 僵尸进程
    reap_zombie_watchers();

    // ② 检查是否为同一 package 的重复请求（如多进程 app 的子进程）
    //    同一 package 且旧进程仍存活时跳过恢复 + 重新应用。
    //    如果旧进程已退出，清除旧会话并重新应用（属性可能已被恢复）。
    {
        let mut guard = ACTIVE_SESSION.lock().unwrap();
        if let Some(ref active) = *guard
            && active.package == request.package_name
        {
            // 检查旧进程是否仍存活
            let old_alive = unsafe { libc::kill(active.pid as i32, 0) } == 0;
            if old_alive {
                info!(
                    "Skipping duplicate Apply for package '{}' (pid {}), session already active (old pid {} alive)",
                    request.package_name, request.pid, active.pid
                );
                return Ok(active.backups.clone());
            } else {
                info!(
                    "Old session for package '{}' (pid {}) is dead, clearing and re-applying for new pid {}",
                    request.package_name, active.pid, request.pid
                );
                guard.take();
            }
        }
    }

    // ③ 如果存在旧会话（不同 package），先恢复旧会话的备份
    {
        let mut guard = ACTIVE_SESSION.lock().unwrap();
        if let Some(old) = guard.take() {
            info!(
                "Restoring previous session backups (package: {}, {} keys) before applying new session for '{}'",
                old.package,
                old.backups.len(),
                request.package_name
            );
            for entry in &old.backups {
                if let Err(e) = apply_resetprop(entry.0, entry.1) {
                    warn!("Failed to restore old session key '{}': {e}", entry.0);
                }
            }
            rebuild_all_contexts(old.backups.keys());
        }
    }

    // ④ 备份当前属性（旧会话已恢复，此时为真实值）
    let mut backups = Vec::with_capacity(request.props.len() + request.delete_props.len());

    for key in request.props.keys() {
        let original = backup_property(key)?;
        backups.push(PropBackup {
            key: key.clone(),
            original_value: original,
        });
    }

    for key in &request.delete_props {
        let original = backup_property(key)?;
        backups.push(PropBackup {
            key: key.clone(),
            original_value: original,
        });
    }

    let backups_for_response: HashMap<String, String> = backups
        .iter()
        .map(|entry| (entry.key.clone(), entry.original_value.clone()))
        .collect();

    // ⑤ 应用新伪装值
    for (key, value) in &request.props {
        apply_resetprop(key, value)?;
    }

    for key in &request.delete_props {
        resetprop_delete(key)?;
    }

    rebuild_all_contexts(request.props.keys().chain(request.delete_props.iter()));

    // ⑥ Fork 恢复 watcher
    if let Err(e) = spawn_restore_watcher(
        request.pid,
        request.props.clone(),
        request.delete_props.clone(),
        backups.clone(),
    ) {
        error!("Failed to spawn restore watcher: {e}, rolling back applied props");
        for entry in &backups {
            let _ = apply_resetprop(&entry.key, &entry.original_value);
        }
        rebuild_all_contexts(backups.iter().map(|b| &b.key));
        anyhow::bail!("failed to spawn restore watcher: {e}");
    }

    // ⑦ 存储新会话
    *ACTIVE_SESSION.lock().unwrap() = Some(ActiveSession {
        package: request.package_name.clone(),
        pid: request.pid,
        backups: backups
            .iter()
            .map(|b| (b.key.clone(), b.original_value.clone()))
            .collect(),
    });

    Ok(backups_for_response)
}

fn restore_properties(request: RestoreRequest) -> anyhow::Result<()> {
    if request.props.is_empty() {
        return Ok(());
    }

    for (key, value) in &request.props {
        apply_resetprop(key, value)?;
    }

    // Rebuild after restoring originals to reclaim any holes.
    rebuild_all_contexts(request.props.keys());

    Ok(())
}

fn backup_property(key: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new("getprop").arg(key).output()?;
    if !output.status.success() {
        anyhow::bail!("getprop failed for {key}");
    }

    let value = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    Ok(value)
}

fn new_resetprop() -> anyhow::Result<ResetProp> {
    sys_prop::init()
        .map_err(|e| anyhow::anyhow!("failed to initialize system property API: {e}"))?;

    Ok(ResetProp {
        // `-n`: bypass property_service, direct mmap write.
        // All properties we set (ro.*, persist.*, etc.) benefit from direct
        // mmap — no SELinux policy denials, no init service restarts, no
        // PROP_VALUE_MAX limit.  ro.* is forced to mmap regardless, but
        // skip_svc=true also covers non-ro keys in custom_props.
        skip_svc: true,
        persistent: false,
        persist_only: false,
        verbose: false,
        show_context: false,
        rebuild: false,
    })
}

fn apply_resetprop(key: &str, value: &str) -> anyhow::Result<()> {
    let rp = new_resetprop()?;

    if let Err(e) = rp.set(key, value) {
        // 值超过 PROP_VALUE_MAX 时，inline prop_info 无法原地扩展。
        // 先删除旧属性（释放 inline 空间），再重新创建为 long 模式。
        warn!("resetprop set failed for {key}, trying delete+set: {e}");
        let _ = rp.delete(key);
        rp.set(key, value)
            .map_err(|e2| anyhow::anyhow!("resetprop delete+set failed for {key}: {e2}"))?;
    }
    Ok(())
}

fn resetprop_delete(key: &str) -> anyhow::Result<()> {
    let rp = new_resetprop()?;

    match rp.delete(key) {
        Ok(true) => Ok(()),
        Ok(false) => anyhow::bail!("resetprop delete failed for {key}: property not found"),
        Err(_) => anyhow::bail!("resetprop delete failed for {key}"),
    }
}

fn spawn_restore_watcher(
    pid: u32,
    props: HashMap<String, String>,
    delete_props: Vec<String>,
    backups: Vec<PropBackup>,
) -> anyhow::Result<i32> {
    unsafe {
        match libc::fork() {
            -1 => anyhow::bail!("fork failed: {}", std::io::Error::last_os_error()),
            0 => {
                if libc::setsid() == -1 {
                    libc::_exit(1);
                }
                if let Err(e) =
                    watch_process_state_and_sync_props(pid, &props, &delete_props, &backups)
                {
                    error!("Watcher failed for pid {}: {}", pid, e);
                }
                libc::_exit(0);
            }
            child_pid => {
                info!("Spawned restore watcher pid={child_pid} for app pid={pid}");
                Ok(child_pid)
            }
        }
    }
}

fn watch_process_state_and_sync_props(
    pid: u32,
    props: &HashMap<String, String>,
    delete_props: &[String],
    backups: &[PropBackup],
) -> anyhow::Result<()> {
    // 优先使用 inotify 监听 oom_score_adj（事件驱动，零轮询）。
    // 回退到 /proc/<pid>/cgroup 轮询（inotify 在部分设备/内核上不可用）。
    match watch_via_inotify(pid, props, delete_props, backups) {
        Ok(()) => return Ok(()),
        Err(e) => {
            warn!("inotify on oom_score_adj unavailable ({e}), falling back to cgroup polling");
        }
    }

    watch_via_cgroup_polling(pid, props, delete_props, backups)
}

/// 事件驱动方案：inotify 监听 /proc/<pid>/oom_score_adj + pidfd 监听进程退出。
///
/// Android 的 OomAdjuster 在 app 前后台切换时写入 oom_score_adj：
/// - 前台: 0
/// - 可见: 100
/// - 后台/缓存: 200-900+
///
/// inotify IN_MODIFY 在 procfs 的 oom_score_adj 上已验证可用（Android 内核）。
/// 使用 epoll 同时监听 inotify fd 和 pidfd，阻塞直到事件到达，零轮询。
fn watch_via_inotify(
    pid: u32,
    props: &HashMap<String, String>,
    delete_props: &[String],
    backups: &[PropBackup],
) -> anyhow::Result<()> {
    const BACKGROUND_THRESHOLD: i32 = 200;
    const BACKGROUND_DEBOUNCE: Duration = Duration::from_secs(2);

    // pidfd：事件驱动检测 app 退出
    let pidfd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0u32) };
    if pidfd < 0 {
        anyhow::bail!("pidfd_open failed");
    }
    let pidfd = pidfd as i32;

    // inotify：监听 oom_score_adj 变化
    let ifd = unsafe { libc::inotify_init() };
    if ifd < 0 {
        unsafe { libc::close(pidfd) };
        anyhow::bail!("inotify_init failed");
    }
    let oom_path = format!("/proc/{pid}/oom_score_adj\0");
    let wd = unsafe {
        libc::inotify_add_watch(
            ifd,
            oom_path.as_ptr() as *const libc::c_char,
            libc::IN_MODIFY,
        )
    };
    if wd < 0 {
        unsafe {
            libc::close(ifd);
            libc::close(pidfd);
        }
        anyhow::bail!("inotify_add_watch on oom_score_adj failed");
    }
    let wd = wd as u32;

    // epoll：同时监听 pidfd 和 inotify fd
    let efd = unsafe { libc::epoll_create1(0) };
    if efd < 0 {
        unsafe {
            libc::inotify_rm_watch(ifd, wd);
            libc::close(ifd);
            libc::close(pidfd);
        }
        anyhow::bail!("epoll_create1 failed");
    }
    let mut ev = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: pidfd as u64,
    };
    unsafe { libc::epoll_ctl(efd, libc::EPOLL_CTL_ADD, pidfd, &mut ev) };
    ev.u64 = ifd as u64;
    unsafe { libc::epoll_ctl(efd, libc::EPOLL_CTL_ADD, ifd, &mut ev) };

    let mut is_spoof_applied = true;
    let mut background_since: Option<Instant> = None;
    let mut events = [libc::epoll_event { events: 0, u64: 0 }; 2];

    info!("restore watcher: inotify monitoring oom_score_adj for pid {pid}");

    loop {
        let timeout = if let Some(bg_start) = background_since {
            // 后台 debounce 等待中，计算剩余时间
            let remaining = BACKGROUND_DEBOUNCE
                .checked_sub(bg_start.elapsed())
                .unwrap_or(Duration::ZERO);
            remaining.as_millis() as i32
        } else {
            -1 // 无限阻塞
        };

        let nfds = unsafe { libc::epoll_wait(efd, events.as_mut_ptr(), 2, timeout) };

        // debounce 到期检查
        if let Some(bg_start) = background_since
            && bg_start.elapsed() >= BACKGROUND_DEBOUNCE
        {
            if is_spoof_applied {
                restore_props_batch(backups)?;
                is_spoof_applied = false;
                info!("restore watcher restored props for pid {pid}");
            }
            background_since = None;
        }

        if nfds < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            // 非 EINTR 错误（如 EBADF），退出前务必恢复属性
            warn!("restore watcher: epoll_wait error: {err}, attempting restore before exit");
            if is_spoof_applied {
                let _ = restore_props_batch(backups);
            }
            break;
        }

        if nfds == 0 {
            // timeout — debounce 可能已处理
            continue;
        }

        // 检查是否有进程退出事件
        let process_exited = events
            .iter()
            .take(nfds as usize)
            .any(|e| e.u64 == pidfd as u64);
        if process_exited {
            if is_spoof_applied {
                restore_props_batch(backups)?;
            }
            info!("restore watcher: app pid {pid} exited (pidfd event)");
            break;
        }

        // oom_score_adj 变化
        for ev in events.iter().take(nfds as usize) {
            if ev.u64 == ifd as u64 {
                let mut buf = [0u8; 512];
                let _ =
                    unsafe { libc::read(ifd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };

                let oom_val = read_oom_score_adj(pid);
                if oom_val >= BACKGROUND_THRESHOLD {
                    let bg_start = *background_since.get_or_insert_with(Instant::now);
                    if is_spoof_applied && bg_start.elapsed() >= BACKGROUND_DEBOUNCE {
                        restore_props_batch(backups)?;
                        is_spoof_applied = false;
                        info!("restore watcher restored props for pid {pid} (oom={oom_val})");
                        background_since = None;
                    }
                } else {
                    background_since = None;
                    if !is_spoof_applied {
                        apply_props_batch(props, delete_props)?;
                        is_spoof_applied = true;
                        info!(
                            "restore watcher re-applied spoof props for pid {pid} (oom={oom_val})"
                        );
                    }
                }
            }
        }
    }

    unsafe {
        libc::epoll_ctl(efd, libc::EPOLL_CTL_DEL, ifd, std::ptr::null_mut());
        libc::epoll_ctl(efd, libc::EPOLL_CTL_DEL, pidfd, std::ptr::null_mut());
        libc::inotify_rm_watch(ifd, wd);
        libc::close(efd);
        libc::close(ifd);
        libc::close(pidfd);
    }
    Ok(())
}

/// 读取 /proc/<pid>/oom_score_adj，失败返回 0（视为前台）。
fn read_oom_score_adj(pid: u32) -> i32 {
    let path = format!("/proc/{pid}/oom_score_adj");
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .unwrap_or(0)
}

/// 轮询回退方案：/proc/<pid>/cgroup 检查 top-app（与原实现相同）。
fn watch_via_cgroup_polling(
    pid: u32,
    props: &HashMap<String, String>,
    delete_props: &[String],
    backups: &[PropBackup],
) -> anyhow::Result<()> {
    const POLL_INTERVAL: Duration = Duration::from_millis(200);
    const BACKGROUND_DEBOUNCE: Duration = Duration::from_secs(2);

    let proc_path = format!("/proc/{pid}");
    let mut is_spoof_applied = true;
    let mut background_since: Option<Instant> = None;

    info!("restore watcher: cgroup polling for pid {pid}");

    loop {
        if !std::path::Path::new(&proc_path).exists() {
            if is_spoof_applied {
                restore_props_batch(backups)?;
            }
            break;
        }

        if is_process_in_top_app(pid) {
            background_since = None;
            if !is_spoof_applied {
                apply_props_batch(props, delete_props)?;
                is_spoof_applied = true;
                info!("restore watcher re-applied spoof props for pid {pid}");
            }
        } else {
            let bg_start = background_since.get_or_insert_with(Instant::now);
            if is_spoof_applied && bg_start.elapsed() >= BACKGROUND_DEBOUNCE {
                restore_props_batch(backups)?;
                is_spoof_applied = false;
                info!("restore watcher restored props for pid {pid}");
            }
        }

        thread::sleep(POLL_INTERVAL);
    }

    Ok(())
}

fn apply_props_batch(
    props: &HashMap<String, String>,
    delete_props: &[String],
) -> anyhow::Result<()> {
    for (key, value) in props {
        apply_resetprop(key, value)?;
    }

    for key in delete_props {
        resetprop_delete(key)?;
    }

    rebuild_all_contexts(props.keys().chain(delete_props.iter()));

    Ok(())
}

fn restore_props_batch(backups: &[PropBackup]) -> anyhow::Result<()> {
    for entry in backups {
        apply_resetprop(&entry.key, &entry.original_value)?;
    }

    // Rebuild using the first backup's key to find the context.
    rebuild_all_contexts(backups.iter().map(|b| &b.key));

    Ok(())
}

const LOG_PATH: &str = "/data/adb/device_faker/logs/device_faker.log";

fn write_log_lines(request: WriteLogRequest) -> anyhow::Result<()> {
    if request.lines.is_empty() {
        return Ok(());
    }

    write_log_lines_to_path(LOG_PATH, &request.lines)
}

fn write_log_lines_to_path(path: &str, lines: &[String]) -> anyhow::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    for line in lines {
        writeln!(file, "{line}")?;
    }

    file.flush()?;
    Ok(())
}

fn is_process_in_top_app(pid: u32) -> bool {
    let cgroup_path = format!("/proc/{pid}/cgroup");
    match fs::read_to_string(&cgroup_path) {
        Ok(content) => content.lines().any(|line| line.contains("top-app")),
        Err(_) => true,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ResetpropSessionRequest {
    pid: u32,
    props: HashMap<String, String>,
    delete_props: Vec<String>,
    package_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RestoreRequest {
    props: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "cmd", content = "payload")]
pub enum CompanionRequest {
    Apply(ResetpropSessionRequest),
    Restore(RestoreRequest),
    CpuSpoof(CpuSpoofRequest),
    CpuSpoofUnmount(CpuSpoofUnmountRequest),
    WriteLog(WriteLogRequest),
    ReadConfig(ReadConfigRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompanionResponse {
    pub status: i32,
    pub message: Option<String>,
    pub backups: Option<HashMap<String, String>>,
    pub config_content: Option<String>,
}

impl CompanionResponse {
    pub fn ok() -> Self {
        Self {
            status: 0,
            message: None,
            backups: None,
            config_content: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            status: -1,
            message: Some(msg.into()),
            backups: None,
            config_content: None,
        }
    }

    pub fn ok_with_backups(backups: HashMap<String, String>) -> Self {
        Self {
            status: 0,
            message: None,
            backups: Some(backups),
            config_content: None,
        }
    }

    pub fn ok_with_config(config_content: Option<String>) -> Self {
        Self {
            status: 0,
            message: None,
            backups: None,
            config_content,
        }
    }
}

#[derive(Clone)]
struct PropBackup {
    key: String,
    original_value: String,
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        path::PathBuf,
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    struct TempConfigFile {
        path: PathBuf,
    }

    impl TempConfigFile {
        fn new(name: &str) -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before UNIX epoch")
                .as_nanos();
            Self {
                path: env::temp_dir().join(format!(
                    "device_faker_{name}_{}_{}",
                    process::id(),
                    timestamp
                )),
            }
        }
    }

    impl Drop for TempConfigFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    #[test]
    fn read_config_file_reads_current_contents_each_time() -> anyhow::Result<()> {
        let file = TempConfigFile::new("reload");

        fs::write(&file.path, "manufacturer = \"first\"")?;
        assert_eq!(
            read_config_file(&file.path)?,
            Some("manufacturer = \"first\"".to_string())
        );

        fs::write(&file.path, "manufacturer = \"second\"")?;
        assert_eq!(
            read_config_file(&file.path)?,
            Some("manufacturer = \"second\"".to_string())
        );

        Ok(())
    }

    #[test]
    fn read_config_file_returns_none_when_config_is_missing() -> anyhow::Result<()> {
        let file = TempConfigFile::new("missing");

        assert_eq!(read_config_file(&file.path)?, None);

        Ok(())
    }

    #[test]
    fn read_config_uses_existing_ipc_framing() -> anyhow::Result<()> {
        let (mut client, mut companion) = UnixStream::pair()?;
        let request = CompanionRequest::ReadConfig(ReadConfigRequest {});
        let request_payload = serde_json::to_vec(&request)?;

        client.write_all(&(request_payload.len() as u32).to_le_bytes())?;
        client.write_all(&request_payload)?;

        assert!(matches!(
            read_companion_request(&mut companion)?,
            CompanionRequest::ReadConfig(_)
        ));

        let response = CompanionResponse::ok_with_config(Some("debug = true".to_string()));
        write_companion_response(&mut companion, &response)?;

        let mut len_buf = [0u8; 4];
        client.read_exact(&mut len_buf)?;
        let response_len = u32::from_le_bytes(len_buf) as usize;
        let mut response_payload = vec![0u8; response_len];
        client.read_exact(&mut response_payload)?;
        let response: CompanionResponse = serde_json::from_slice(&response_payload)?;

        assert_eq!(response.status, 0);
        assert_eq!(response.config_content.as_deref(), Some("debug = true"));

        Ok(())
    }
}
