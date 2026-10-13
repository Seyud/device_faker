//! 前台观察者（UidObserver）：companion 内嵌的"事件驱动前台真值"。
//!
//! 数据源 = ActivityManager 的 `PROCESS_STATE_TOP`（top resumed activity 所在
//! 进程的状态），**与 top-app cgroup 解耦**——OEM 把无焦点进程提升进 top-app
//! 不会污染前台判定（assistantscreen 之类没有 resumed activity，不是 TOP）。
//!
//! 机制（root，经 rsbinder，android_11_plus/binderfs）：
//! 1. `ProcessState::init_default()` + `hub::wait_for_interface("activity")`；
//! 2. rsbinder-aidl 生成的 `BnUidObserver` 实现回调（onUidStateChanged 事务码
//!    与真实 AOSP `android.app.IUidObserver` 一致）；
//! 3. `BpActivityManager::register_uid_observer(observer, which, cutpoint, pkg)`
//!    ——事务码 2（IActivityManager native 侧稳定区段，占位 openContentUri
//!    code=1 对齐真实顺序）；**cutpoint = -1（全量回调，用户态过滤）**——
//!    内核侧 cutpoint 过滤在部分 OEM ROM 上会丢掉非配置 app 首次进入 TOP 的
//!    事件，前台门控随之失效；正确性优先，保留全量回调；
//! 4. `ProcessState::start_thread_pool()` 接收回调；
//! 5. uid → 包名 用 `/data/system/packages.list`（root 可读）全量映射。
//!
//! 发布语义（喂给 companion 的 `handle_fg_event`）：
//! - 某 uid 进入 TOP → 发布该包名（A 前台）；
//! - 当前 TOP uid 离开（含 GONE）→ 发布 `-`（过渡态，保持现状，D1）；
//! - 下一个 TOP 事件（无论配置与否）→ 触发真正的 apply/restore。
//!
//! 失败兜底：无法注册（无 binder/服务不可用）→ 0.5s `dumpsys window` 轮询
//! 解析 mCurrentFocus/mFocusedApp。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use log::{debug, error, info};
use rsbinder::{ProcessState, Strong, hub};

include!(concat!(env!("OUT_DIR"), "/fg_binder.rs"));

use self::android::app::IActivityManager::IActivityManager;
use self::android::app::IUidObserver::{BnUidObserver, IUidObserver};

/// 前台源是否就绪（observer 注册成功，或轮询兜底已接管）。
pub static FG_READY: AtomicBool = AtomicBool::new(false);

/// 当前 TOP 包名（observer/轮询维护），供 Apply 关闭"事件先于 Apply"竞态。
static CURRENT_FG: Mutex<Option<String>> = Mutex::new(None);

/// 返回当前 TOP 包名（未知/过渡态为 None）。
pub fn current_fg() -> Option<String> {
    CURRENT_FG.lock().unwrap().clone()
}

/// AMS 进程状态：前台（top resumed activity 所在进程）。
pub const PROCESS_STATE_TOP: i32 = 2;
/// `ActivityManager.UID_OBSERVER_PROCSTATE = 1 << 0`（Android 定义）。
const UID_OBSERVER_PROCSTATE: i32 = 1 << 0;
/// `ActivityManager.UID_OBSERVER_GONE = 1 << 1`。
const UID_OBSERVER_GONE: i32 = 1 << 1;
const PACKAGES_LIST: &str = "/data/system/packages.list";
/// 兜底轮询周期。
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// 前台变化回调：pkg = 当前前台包名（`-` 表示过渡/未知）。
/// 用 Arc 包裹以便注册失败后轮询兜底复用同一闭包。
pub type FgSink = Arc<dyn Fn(&str) + Send + Sync>;

/// UidObserver 回调状态。
pub struct UidObserverState {
    /// uid → 包名（全量，来自 packages.list）。
    /// 注册时快照可能过期（新装/更新 app 不在快照内）：
    /// 查询 miss 时重读 packages.list 定向刷新，保证"即装即测"不被前台门控漏判。
    uid_to_pkg: Mutex<HashMap<i32, String>>,
    /// 当前处于 TOP 的 uid（全局同时至多一个）。
    top_uid: Mutex<Option<i32>>,
    sink: FgSink,
}

impl rsbinder::Interface for UidObserverState {}

impl UidObserverState {
    fn on_top_change(&self, uid: i32, is_top: bool) {
        let mut top = self.top_uid.lock().unwrap();
        if is_top {
            if *top == Some(uid) {
                return; // 幂等
            }
            *top = Some(uid);
            // 查 uid → 包名；miss 表示新装/更新包不在注册快照内，重读 packages.list 刷新。
            let pkg = {
                let mut map = self.uid_to_pkg.lock().unwrap();
                match map.get(&uid) {
                    Some(p) => p.clone(),
                    None => {
                        if let Ok(fresh) = read_packages_list() {
                            *map = fresh;
                        }
                        map.get(&uid).cloned().unwrap_or_else(|| "-".to_string())
                    }
                }
            };
            info!("fg observer: uid {uid} entered TOP -> {pkg}");
            drop(top);
            *CURRENT_FG.lock().unwrap() = Some(pkg.clone());
            (self.sink)(&pkg);
        } else if *top == Some(uid) {
            // 当前 TOP uid 离开 → 过渡态
            *top = None;
            info!("fg observer: uid {uid} left TOP -> -");
            drop(top);
            *CURRENT_FG.lock().unwrap() = None;
            (self.sink)("-");
        }
    }
}

impl IUidObserver for UidObserverState {
    fn r#onUidStateChanged(
        &self,
        uid: i32,
        proc_state: i32,
        _seq: i64,
        _capability: i32,
    ) -> rsbinder::BinderResult<()> {
        debug!("fg observer: onUidStateChanged uid={uid} proc_state={proc_state}");
        self.on_top_change(uid, proc_state == PROCESS_STATE_TOP);
        Ok(())
    }

    fn r#onUidGone(&self, uid: i32, _disabled: bool) -> rsbinder::BinderResult<()> {
        debug!("fg observer: onUidGone uid={uid}");
        self.on_top_change(uid, false);
        Ok(())
    }

    fn r#onUidActive(&self, uid: i32) -> rsbinder::BinderResult<()> {
        debug!("fg observer: onUidActive uid={uid}");
        Ok(())
    }

    fn r#onUidIdle(&self, uid: i32, _disabled: bool) -> rsbinder::BinderResult<()> {
        debug!("fg observer: onUidIdle uid={uid}");
        Ok(())
    }

    fn r#onUidProcAdjChanged(&self, uid: i32, adj: i32) -> rsbinder::BinderResult<()> {
        debug!("fg observer: onUidProcAdjChanged uid={uid} adj={adj}");
        Ok(())
    }

    fn r#onUidCachedChanged(&self, uid: i32, _cached: bool) -> rsbinder::BinderResult<()> {
        debug!("fg observer: onUidCachedChanged uid={uid}");
        Ok(())
    }
}

/// 启动前台源：先试 UidObserver（事件驱动），失败退 0.5s 轮询。
/// 顶层调用一次；内部线程常驻。
pub fn spawn_fg_source(sink: FgSink) {
    std::thread::Builder::new()
        .name("fg-observer".into())
        .spawn(move || {
            if let Err(e) = register_observer(sink.clone()) {
                error!("UidObserver registration failed ({e}); falling back to 0.5s dumpsys poll");
                poll_loop(sink);
            }
        })
        .expect("failed to spawn fg-observer thread");
}

/// 注册 UidObserver 并启动 binder 线程池接收回调。返回 Ok 表示已接管前台源。
fn register_observer(sink: FgSink) -> anyhow::Result<()> {
    ProcessState::init_default()
        .map_err(|e| anyhow::anyhow!("binder ProcessState init failed: {e}"))?;

    let uid_to_pkg = read_packages_list()?;
    if uid_to_pkg.is_empty() {
        return Err(anyhow::anyhow!("{PACKAGES_LIST} empty/unreadable"));
    }

    // 阻塞等 ActivityManager 服务可用，返回强类型代理（BpActivityManager）。
    let bp: Strong<dyn IActivityManager> = hub::wait_for_interface("activity")?;

    let state = UidObserverState {
        uid_to_pkg: Mutex::new(uid_to_pkg),
        top_uid: Mutex::new(None),
        sink,
    };
    let observer: Strong<dyn IUidObserver> = BnUidObserver::new_binder(state);

    bp.r#registerUidObserver(
        &observer,
        UID_OBSERVER_PROCSTATE | UID_OBSERVER_GONE,
        // 必须用 -1（全量回调，用户态过滤）：内核侧 cutpoint 过滤在部分 OEM ROM 上
        // 会丢掉非配置 app 首次进入 TOP 的事件，导致前台门控不恢复 density/props。
        // 正确性优先：全量回调 + on_top_change 的 proc_state==TOP 判断。
        -1,
        "device_faker",
    )?;

    // 启动 binder 线程池接收 onUidStateChanged/onUidGone 回调。
    ProcessState::start_thread_pool();

    FG_READY.store(true, Ordering::Release);
    info!("UidObserver active: event-driven foreground (PROCESS_STATE_TOP)");
    Ok(())
}

/// 读 `/data/system/packages.list`（每行 `<pkg> <uid> ...`）构建 uid→包名。
fn read_packages_list() -> anyhow::Result<HashMap<i32, String>> {
    let content = std::fs::read_to_string(PACKAGES_LIST)?;
    let mut map = HashMap::new();
    for line in content.lines() {
        let mut it = line.split_whitespace();
        let (Some(pkg), Some(uid)) = (it.next(), it.next()) else {
            continue;
        };
        let Ok(uid) = uid.parse::<i32>() else {
            continue;
        };
        map.entry(uid).or_insert_with(|| pkg.to_string());
    }
    if map.is_empty() {
        return Err(anyhow::anyhow!("no entries parsed from {PACKAGES_LIST}"));
    }
    Ok(map)
}

/// 兜底：0.5s `dumpsys window` 轮询解析 mCurrentFocus/mFocusedApp。
/// 只在变化时调用 sink（避免无效 churn）。
fn poll_loop(sink: FgSink) {
    FG_READY.store(true, Ordering::Release);
    info!("foreground fallback: 0.5s dumpsys window poll");
    let mut last = String::new();
    loop {
        let focus = exec_dumpsys_focus();
        if focus != last {
            last = focus.clone();
            *CURRENT_FG.lock().unwrap() = if focus == "-" {
                None
            } else {
                Some(focus.clone())
            };
            info!("fg poll focus -> {focus}");
            (sink)(&focus);
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn exec_dumpsys_focus() -> String {
    let out = std::process::Command::new("dumpsys")
        .args(["window"])
        .output();
    let text = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => return "-".to_string(),
    };
    // 优先 mCurrentFocus，其次 mFocusedApp
    for line in text.lines() {
        let l = line.trim();
        if let Some(v) = l.strip_prefix("mCurrentFocus=") {
            return parse_window_ref(v);
        }
    }
    for line in text.lines() {
        let l = line.trim();
        if let Some(v) = l.strip_prefix("mFocusedApp=") {
            return parse_window_ref(v);
        }
    }
    "-".to_string()
}

/// 从 `Window{... u0 com.example.app/...}` 或 `ActivityRecord{...}` 提取包名。
fn parse_window_ref(v: &str) -> String {
    for tok in v.split_whitespace() {
        if let Some(pkg) = tok.split('/').next()
            && pkg.contains('.')
            && !pkg.contains('{')
            && !pkg.contains('}')
        {
            return pkg.to_string();
        }
    }
    "-".to_string()
}
