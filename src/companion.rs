use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::net::UnixStream,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use log::{LevelFilter, error, info, warn};
use prop_rs_android::{resetprop::ResetProp, sys_prop};
use serde::{Deserialize, Serialize};
use zygisk_api::api::{V4, ZygiskApi};

use crate::config::{DPI_MAX, DPI_MIN};

// ── 前台门控会话（D1/D4）────────────────────────────────────────────────────
//
// 前后台真值由 companion 内嵌的 UidObserver 提供（src/fg_observer.rs，
// ActivityManager PROCESS_STATE_TOP 事件，0.5s dumpsys 轮询兜底）。Apply 只
// 采样备份并登记未激活会话，**不急切应用任何全局态**；`FG == 包名` 才中央
// 激活属性批量与密度，`FG != 包名` 恢复。前台源未就绪时一律不做全局变更
// （D2 宁缺毋滥）。

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// 单个前台门控会话：Apply 登记，FG 事件驱动激活/恢复。
struct Session {
    package: String,
    app_pid: u32,
    /// 本次伪装规格（来自 Apply 请求）
    spec: SessionSpec,
    /// Apply 时采样的原始值备份（此刻全局处于干净状态）
    originals: Originals,
    /// 是否已随 FG 事件中央激活
    activated: bool,
}

struct SessionSpec {
    props: HashMap<String, String>,
    delete_props: Vec<String>,
    density: Option<u32>,
}

struct Originals {
    prop_backups: HashMap<String, String>,
    /// 备份时**真机上不存在**的 key（`prop_backups` 里只是空串占位）。restore 时
    /// 必须 `delete` 而不是 `set ""`——否则会把"本来没有的属性"写成**空值属性**：
    /// key 存在但值为空，`getprop` 读出来同样是空串，与"不存在"难以区分，在
    /// `__system_property_foreach` / 全量 dump 里就是真机没有的可枚举幽灵属性。
    absent_props: HashSet<String>,
    orig_density: Option<u32>,
}

// ── 前台源（UidObserver 事件驱动，0.5s dumpsys 轮询兜底）───────────────────
//
// 首次 Apply 走 `spawn_fg_observer_once` 惰性启动，事件驱动
// （PROCESS_STATE_TOP）或轮询兜底喂 `handle_fg_event`。

static FG_OBSERVER_STARTED: OnceLock<()> = OnceLock::new();

/// 首次 Apply 时惰性启动前台源线程（UidObserver → 轮询兜底）。
fn spawn_fg_observer_once() {
    FG_OBSERVER_STARTED.get_or_init(|| {
        let sink: crate::fg_observer::FgSink =
            std::sync::Arc::new(|pkg: &str| handle_fg_event(pkg));
        crate::fg_observer::spawn_fg_source(sink);
    });
}

/// 前台源（observer 或轮询）是否已就绪。
fn fg_ready() -> bool {
    crate::fg_observer::FG_READY.load(std::sync::atomic::Ordering::Acquire)
}

/// D2 限频告警间隔：同一分钟内最多一条 WARN。
const FG_WARN_INTERVAL: Duration = Duration::from_secs(60);

/// D2 限频 WARN：前台源未就绪时属性/密度一律不应用。
fn warn_fg_unavailable() {
    static LAST: OnceLock<Mutex<Instant>> = OnceLock::new();
    let due = LAST
        .get_or_init(|| Mutex::new(Instant::now() - FG_WARN_INTERVAL))
        .lock()
        .map(|mut last| {
            let notify = last.elapsed() >= FG_WARN_INTERVAL;
            if notify {
                *last = Instant::now();
            }
            notify
        })
        .unwrap_or(false);
    if due {
        warn!(
            "foreground source not ready; session registration pending activation (will apply once FG events arrive)"
        );
    }
}

// ── 中央状态机：FG 事件驱动激活/恢复 ────────────────────────────────────────
//
// | 条件                                    | 动作                             |
// |-----------------------------------------|----------------------------------|
// | 死会话（kill 0 失败）且未激活             | 清空（搭车式惰性回收，D5）        |
// | FG == 包名 && !activated                 | 应用 props 批量 + density，激活   |
// | FG == 包名 && activated                  | 幂等跳过                          |
// | FG != 包名 && activated                  | 恢复 props + density，失活        |
// | FG == "-"（过渡态）                      | 保持现状（宁可晚不错）            |
// | 其它                                     | 无操作                            |

fn handle_fg_event(pkg: &str) {
    let mut guard = SESSION.lock().unwrap();

    // 惰性回收：死且未激活的会话直接清空；激活中的等焦点移交触发的失活收敛。
    if let Some(sess) = guard.as_ref()
        && unsafe { libc::kill(sess.app_pid as i32, 0) } != 0
        && !sess.activated
    {
        info!(
            "Reaping dead session '{}' (pid {})",
            sess.package, sess.app_pid
        );
        *guard = None;
    }

    if pkg == "-" {
        return;
    }

    let Some(sess) = guard.as_mut() else {
        return;
    };
    if pkg == sess.package {
        if !sess.activated {
            activate_session(sess);
        }
    } else if sess.activated {
        let mut taken = guard.take().expect("session present");
        deactivate_session(&mut taken, pkg);
        *guard = Some(taken);
    }
}

fn activate_session(sess: &mut Session) {
    if let Err(e) = apply_props_batch(&sess.spec.props, &sess.spec.delete_props) {
        // 标记已激活（即使部分失败），保证失活路径总能恢复到采样基线。
        error!(
            "FG activation: prop batch failed for '{}': {e}",
            sess.package
        );
    }
    if let Some(density) = sess.spec.density
        && let Err(e) = set_density(Some(density))
    {
        error!("FG activation: density failed for '{}': {e}", sess.package);
    }
    sess.activated = true;
    info!(
        "FG == {}: density/props activated ({} set, {} delete, dpi={:?})",
        sess.package,
        sess.spec.props.len(),
        sess.spec.delete_props.len(),
        sess.spec.density
    );
}

fn deactivate_session(sess: &mut Session, focused: &str) {
    if let Err(e) = restore_props_batch(&sess.originals.prop_backups, &sess.originals.absent_props)
    {
        error!(
            "FG handoff: prop restore failed for '{}': {e}",
            sess.package
        );
    }
    if let Err(e) = restore_density(sess.originals.orig_density) {
        error!(
            "FG handoff: density restore failed for '{}': {e}",
            sess.package
        );
    }
    sess.activated = false;
    info!(
        "FG -> {focused}: density/props restored for '{}' (deactivated, orig_density={:?}, {} props)",
        sess.package,
        sess.originals.orig_density,
        sess.originals.prop_backups.len()
    );
}

// ── 协议与机械层（保持不变）──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct WriteLogRequest {
    pub lines: Vec<String>,
}

pub fn spoof_system_props_via_companion(
    api: &mut ZygiskApi<V4>,
    prop_map: &HashMap<String, String>,
    explicit_props: &HashSet<String>,
    delete_props: &[String],
    package_name: &str,
    density: Option<u32>,
    debug: bool,
) -> anyhow::Result<()> {
    if prop_map.is_empty() && delete_props.is_empty() && density.is_none() {
        return Ok(());
    }

    let request = CompanionRequest::Apply(ResetpropSessionRequest {
        pid: std::process::id(),
        props: prop_map.clone(),
        explicit_props: explicit_props.iter().cloned().collect(),
        delete_props: delete_props.to_vec(),
        package_name: package_name.to_string(),
        density,
        debug,
    });

    let response = send_companion_command(api, &request)?;
    if response.status != 0 {
        anyhow::bail!(
            response
                .message
                .unwrap_or_else(|| "companion resetprop failed".to_string())
        );
    }

    // companion 侧现在自己管理会话状态与前台门控逻辑；
    // Zygisk 模块侧不再需要 ACTIVE_RESET_SESSION。
    Ok(())
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
        // ksu_props ≥ ddb6ee7：`rebuild(context, check, appcompat)` 一次只处理一个
        // area；normal 与 appcompat 各调一次以覆盖两侧（与旧版单参 rebuild 等价）。
        // check=false：无条件重建，保留旧行为（长值 update 后旧 long buffer 会成为
        // 孤儿，需要重建回收）。
        if let Err(e) = sys_prop::rebuild(ctx, false, false) {
            warn!("prop area rebuild for {ctx} failed (non-fatal): {e}");
        }
        if let Err(e) = sys_prop::rebuild(ctx, false, true) {
            warn!("appcompat prop area rebuild for {ctx} failed (non-fatal): {e}");
        }
    }
}

/// 同步 companion 日志级别：debug=true 保持 Debug（保留 fg_observer/会话等观察性日志），
/// debug=false → Off（完全不写日志）。fork 出来的 CPU spoof 子进程会继承该级别，
/// 因此调用点必须在 fork 之前。
pub(crate) fn sync_log_level(debug: bool) {
    crate::file_logger::set_level(if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Off
    });
}

/// Apply 请求处理器：只采样备份 + 登记未激活会话（D1），
/// 不急切应用任何全局态；激活由 FG 事件驱动。
fn apply_resetprop_session(
    mut request: ResetpropSessionRequest,
) -> anyhow::Result<HashMap<String, String>> {
    sync_log_level(request.debug);

    if request.props.is_empty() && request.delete_props.is_empty() && request.density.is_none() {
        return Ok(HashMap::new());
    }

    if let Some(density) = request.density
        && !(DPI_MIN..=DPI_MAX).contains(&density)
    {
        anyhow::bail!(
            "DPI {} is outside the supported range {}..={}",
            density,
            DPI_MIN,
            DPI_MAX
        );
    }

    // 「本机可能不存在」的 key（ro.product.{dlkm 分区}.* / ro.{dlkm 分区}.build.*、
    // OEM 营销名族 `ro.product[.<分区>].marketname` 与别名 `ro.vendor.oplus.market.name`）
    // 只在部分 ROM 上存在，且都是模块**自动展开**出来的副本。resetprop `set()` 会在设备上
    // 不存在的 key 上创建**全局**幽灵属性（shell `getprop` 全量 dump 可见，比进程内副本
    // 显眼得多），且 backup/restore 还会把它留成空值 ⇒ 登记前先剔除真机不存在的 key
    // （连带不进 originals 备份，restore 也就不会重建它）。
    //
    // 例外：`custom_props` 里用户显式写出的 key（`explicit_props`）是明确指令，照写不误。
    //
    // 存在性用 `sys_prop::find`（bionic `__system_property_find`，本进程直接查属性区，
    // 首次查询触发该 context area 的惰性映射）——**不 fork/exec `getprop`**。判定不了
    // （bionic 符号加载失败）时保守放行，保持旧行为。
    if request
        .props
        .keys()
        .any(|key| crate::config::is_patch_only_key(key))
    {
        let explicit: HashSet<&str> = request.explicit_props.iter().map(String::as_str).collect();
        let mut dropped_keys: Vec<String> = Vec::new();
        request.props.retain(|key, _| {
            let keep = !crate::config::is_patch_only_key(key)
                || explicit.contains(key.as_str())
                || property_exists(key) != Some(false);
            if !keep {
                dropped_keys.push(key.clone());
            }
            keep
        });
        if !dropped_keys.is_empty() {
            info!(
                "Apply for '{}': dropped {} keys absent on this ROM: {}",
                request.package_name,
                dropped_keys.len(),
                dropped_keys.join(", ")
            );
        }
    }

    // 首次 Apply 时惰性启动前台源（UidObserver 事件驱动；失败退 0.5s 轮询）。
    spawn_fg_observer_once();

    // D2 语义：前台源未就绪时不急切应用任何全局态；但**照常采样登记 session**。
    // 之前这里直接 return（连 session 都不登记），observer 注册完成后到达的首次
    // TOP 事件会因 `SESSION == None` 无法激活 → 本次启动属性永不应用（首启竞态，
    // 冷启动后首个 companion app 必命中，极易误报"某版本属性失效"）。
    // 修复：不 return 不等待——session 登记后，registerUidObserver 完成时 AMS 会
    // 回放当前 uid 状态（或 0.5s dumpsys 轮询兜底）产生 FG 事件驱动激活。
    if !fg_ready() {
        warn_fg_unavailable();
        info!(
            "Apply for '{}' session will be registered; awaiting fg source ready to activate",
            request.package_name
        );
    }

    // 收割 CPU spoof 等 fork 子进程的僵尸（轻量兜底）。
    reap_zombie_children();

    let mut guard = SESSION.lock().unwrap();

    // 同包去重：同包 + 旧进程存活 + dpi 未变 → 幂等返回现有备份。
    if let Some(sess) = guard.as_ref()
        && sess.package == request.package_name
        && unsafe { libc::kill(sess.app_pid as i32, 0) } == 0
        && sess.spec.density == request.density
    {
        info!(
            "Skipping duplicate Apply for '{}' (pid {}), session already registered",
            request.package_name, request.pid
        );
        return Ok(sess.originals.prop_backups.clone());
    }

    // 接管：已有会话先收敛（激活中的恢复，未激活的直接丢弃），保证采样干净。
    if let Some(old) = guard.take() {
        if old.activated {
            info!(
                "Taking over active session '{}' (pid {}) for '{}'",
                old.package, old.app_pid, request.package_name
            );
            let mut old = old;
            deactivate_session(&mut old, &request.package_name);
        } else {
            info!(
                "Discarding inactive session '{}' for '{}'",
                old.package, request.package_name
            );
        }
    }

    // 采样原始值（此刻全局干净）。真机上不存在的 key 记入 `absent_props`，
    // restore 时走 delete（set 空串会造出"空值属性"这种幽灵 key）。
    let mut prop_backups = HashMap::new();
    let mut absent_props = HashSet::new();
    for key in request.props.keys().chain(request.delete_props.iter()) {
        if property_exists(key) == Some(false) {
            absent_props.insert(key.clone());
            prop_backups.insert(key.clone(), String::new());
        } else {
            prop_backups.insert(key.clone(), backup_property(key)?);
        }
    }
    let orig_density = if request.density.is_some() {
        query_density_override()?
    } else {
        None
    };

    let backups_for_response = prop_backups.clone();

    info!(
        "Apply registered session for '{}' (pid {}, density={:?}, orig_density={:?}, {} props, {} deletes, {} absent on this ROM)",
        request.package_name,
        request.pid,
        request.density,
        orig_density,
        prop_backups.len(),
        request.delete_props.len(),
        absent_props.len()
    );

    *guard = Some(Session {
        package: request.package_name.clone(),
        app_pid: request.pid,
        spec: SessionSpec {
            props: request.props,
            delete_props: request.delete_props,
            density: request.density,
        },
        originals: Originals {
            prop_backups,
            absent_props,
            orig_density,
        },
        activated: false,
    });
    drop(guard);

    // 关闭竞态：焦点早已在本包（事件先于 Apply 到达 / 热重载重登记），立即激活。
    let fg = crate::fg_observer::current_fg();
    if let Some(fg) = fg
        && fg == request.package_name
    {
        handle_fg_event(&fg);
    }

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

/// 属性在真机是否存在。返回 `None` = 无法判定（bionic 符号加载失败），调用方保守放行。
///
/// 走 `sys_prop::find`（本进程直接查属性区）——**不 fork/exec `getprop`**。
fn property_exists(key: &str) -> Option<bool> {
    sys_prop::init().ok()?;
    Some(sys_prop::find(key).is_some())
}

/// 读属性当前值（`None` = 本进程读不到：不存在，或该 area 未映射/无可读权限）。
fn property_value(key: &str) -> Option<String> {
    sys_prop::init().ok()?;
    sys_prop::get(key)
}

/// 备份单个属性的当前全局值。
///
/// 快路径：`sys_prop::get` 在本进程直接读属性区（已存在的属性**零 fork**）。
/// 回落：读不到时用 `getprop` 子进程取真值（root 视角，能覆盖"属性存在但本进程
/// 读不到该 area"的情形），保持与旧实现完全一致的备份语义（不存在 → 空串）。
/// 因此每次 Apply 的 `getprop` 子进程数量从「属性数」降到「不存在的属性数」。
fn backup_property(key: &str) -> anyhow::Result<String> {
    if let Some(value) = property_value(key) {
        return Ok(value);
    }

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

    // ksu_props ≥ ddb6ee7：`set()` 原地支持任意长度的新值（≥ PROP_VALUE_MAX 时内部
    // 重新分配 long buffer 并改写 prop_info 的 long offset），不再需要
    // delete+set 兜底——后者是全局可见的破坏性操作（delete 与 set 之间其他进程
    // 读到空值，set 再失败则属性永久丢失）。
    // 返回的 bool 表示旧 long buffer 成为孤儿、需要 rebuild 回收；调用方
    // （apply_props_batch / restore）结束时会统一 rebuild_all_contexts。
    rp.set(key, value)
        .map_err(|e| anyhow::anyhow!("resetprop set failed for {key}: {e}"))?;
    Ok(())
}

fn resetprop_delete(key: &str) -> anyhow::Result<()> {
    let rp = new_resetprop()?;

    match rp.delete(key) {
        Ok(true) => Ok(()),
        // 属性族展开后，删除列表包含设备上可能不存在的分区副本
        // （如旧设备没有 system_dlkm/vendor_dlkm 构建 props）；
        // 本来就不存在的属性视为已删除，避免整个 Apply 会话失败回滚。
        Ok(false) => {
            info!("resetprop delete: '{key}' not present, treating as deleted");
            Ok(())
        }
        Err(_) => anyhow::bail!("resetprop delete failed for {key}"),
    }
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

fn restore_props_batch(
    backups: &HashMap<String, String>,
    absent_props: &HashSet<String>,
) -> anyhow::Result<()> {
    for (key, value) in backups {
        if absent_props.contains(key) {
            // 备份时该 key 不存在 ⇒ 恢复"不存在"，而不是写成空值属性。
            // delete 一个本就不存在的 key 是幂等空操作，失败也不影响其它 key。
            if let Err(e) = resetprop_delete(key) {
                warn!("restore: delete of originally-absent '{key}' failed: {e}");
            }
        } else {
            apply_resetprop(key, value)?;
        }
    }

    // Rebuild using all backup keys to find the contexts.
    rebuild_all_contexts(backups.keys());

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

fn run_wm_density(args: &[&str]) -> anyhow::Result<String> {
    let mut command = std::process::Command::new("/system/bin/wm");
    command.arg("density").args(args);

    let output =
        match command.output() {
            Ok(output) => output,
            Err(first_error) => {
                // Some root environments expose Android tools through PATH only.
                let mut fallback = std::process::Command::new("wm");
                fallback.arg("density").args(args).output().map_err(|second_error| {
                anyhow::anyhow!(
                    "failed to execute /system/bin/wm ({first_error}) or wm ({second_error})"
                )
            })?
            }
        };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        anyhow::bail!(
            "wm density {:?} failed with status {}: {}",
            args,
            output.status,
            stderr.trim()
        );
    }

    let mut text = stdout.into_owned();
    if !stderr.trim().is_empty() {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&stderr);
    }
    Ok(text)
}

fn query_density_override() -> anyhow::Result<Option<u32>> {
    let output = run_wm_density(&[])?;
    for line in output.lines() {
        let Some((label, value)) = line.split_once(':') else {
            continue;
        };
        if !label.trim().eq_ignore_ascii_case("override density") {
            continue;
        }

        let value = value.trim();
        if value.is_empty() || value.eq_ignore_ascii_case("reset") || value == "0" {
            return Ok(None);
        }

        let density = value
            .parse::<u32>()
            .map_err(|e| anyhow::anyhow!("invalid Override density value '{value}': {e}"))?;
        return Ok(Some(density));
    }

    // AOSP omits the Override density line when no override is active.
    Ok(None)
}

fn set_density(density: Option<u32>) -> anyhow::Result<()> {
    let density_string;
    let args = if let Some(density) = density {
        density_string = density.to_string();
        vec![density_string.as_str()]
    } else {
        vec!["reset"]
    };
    run_wm_density(&args).map(|_| ())
}

fn restore_density(original_density: Option<u32>) -> anyhow::Result<()> {
    set_density(original_density)
}

/// 收割已退出的 fork 子进程（CPU spoof 挂载子进程等），避免僵尸积累。
fn reap_zombie_children() {
    loop {
        match unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) } {
            0 | -1 => break,
            _ => {} // 收割到一个僵尸，继续尝试
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ResetpropSessionRequest {
    pid: u32,
    props: HashMap<String, String>,
    /// `custom_props` 里用户显式写出的 key：不受「本机不存在就不写」门控。
    #[serde(default)]
    explicit_props: Vec<String>,
    delete_props: Vec<String>,
    package_name: String,
    #[serde(default)]
    density: Option<u32>,
    /// 全局 debug 开关：debug=false 时 companion 完全不打印日志。
    #[serde(default)]
    debug: bool,
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
    WriteLog(WriteLogRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompanionResponse {
    pub status: i32,
    pub message: Option<String>,
    pub backups: Option<HashMap<String, String>>,
}

impl CompanionResponse {
    pub fn ok() -> Self {
        Self {
            status: 0,
            message: None,
            backups: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            status: -1,
            message: Some(msg.into()),
            backups: None,
        }
    }

    pub fn ok_with_backups(backups: HashMap<String, String>) -> Self {
        Self {
            status: 0,
            message: None,
            backups: Some(backups),
        }
    }
}
