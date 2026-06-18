use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{Mutex, Once},
};

use log::{LevelFilter, Log, Metadata, Record};

const LOG_DIR: &str = "/data/adb/device_faker/logs";
const LOG_PATH: &str = "/data/adb/device_faker/logs/device_faker.log";

enum LoggerInner {
    /// 能直接写文件（如 companion 进程），直接追加。
    File(File),
    /// 不能直接写文件（如 Zygisk 进程），先缓冲，再通过 companion flush。
    Buffer(Vec<String>),
}

static LOGGER: Mutex<LoggerInner> = Mutex::new(LoggerInner::Buffer(Vec::new()));

struct AdaptiveLogger;

impl Log for AdaptiveLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let now = format_local_time();
        let line = format!(
            "[{now}] [{}] {} - {}",
            record.level(),
            record.target(),
            record.args()
        );

        if let Ok(mut inner) = LOGGER.lock() {
            match &mut *inner {
                LoggerInner::File(file) => {
                    let _ = writeln!(file, "{line}");
                    let _ = file.flush();
                }
                LoggerInner::Buffer(buf) => {
                    buf.push(line);
                }
            }
        }
    }

    fn flush(&self) {}
}

static ADAPTIVE_LOGGER: AdaptiveLogger = AdaptiveLogger;
static INIT_ONCE: Once = Once::new();

/// 初始化日志，尝试直接落盘。
/// 仅 companion 进程（有 root 权限）使用。
pub fn init() {
    INIT_ONCE.call_once(|| {
        let file = open_log_file();
        if let Ok(file) = file
            && let Ok(mut inner) = LOGGER.lock()
        {
            *inner = LoggerInner::File(file);
        }

        let _ = log::set_logger(&ADAPTIVE_LOGGER);
        log::set_max_level(LevelFilter::Debug);
    });
}

/// 仅初始化内存缓冲模式，不打开文件。
/// 用于 `on_load`，避免在 webview_zygote 等受限制进程中触发文件访问导致进程崩溃。
pub fn init_buffer_only() {
    INIT_ONCE.call_once(|| {
        let _ = log::set_logger(&ADAPTIVE_LOGGER);
        log::set_max_level(LevelFilter::Debug);
    });
}

/// 取出当前缓冲的所有日志行。仅 Zygisk 进程使用。
pub fn drain_lines() -> Vec<String> {
    if let Ok(mut inner) = LOGGER.lock()
        && let LoggerInner::Buffer(buf) = &mut *inner
    {
        return std::mem::take(buf);
    }
    Vec::new()
}

fn open_log_file() -> std::io::Result<File> {
    std::fs::create_dir_all(LOG_DIR)?;
    OpenOptions::new().create(true).append(true).open(LOG_PATH)
}

fn format_local_time() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };

    unsafe {
        libc::localtime_r(&secs, &mut tm);
    }

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
        now.subsec_millis()
    )
}
