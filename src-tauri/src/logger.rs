//! 运行时日志系统。
//!
//! 日志写入 ~/.aurora/logs/aurora-desktop.YYYY-MM-DD.log
//! 同时输出到 stderr（debug 模式下可见控制台窗口）。
//! 每行格式: [HH:MM:SS.mmm] [LEVEL] [MODULE] message

#![allow(clippy::let_unit_value)]

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

/// 全局日志文件句柄。
static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

/// 本地时区与 UTC 的偏移秒数（正数 = 东区）。
/// 在 init() 时计算一次，避免每行日志重复计算。
static LOCAL_UTC_OFFSET_SECS: std::sync::atomic::AtomicI64 =
    std::sync::atomic::AtomicI64::new(i64::MAX);

/// 初始化日志系统。创建当日日志文件。
pub fn init() -> anyhow::Result<PathBuf> {
    // 先计算本地时区偏移，确保 format_date() 和 format_timestamp() 使用本地时间
    let offset_secs = compute_local_utc_offset_secs();
    LOCAL_UTC_OFFSET_SECS.store(offset_secs, std::sync::atomic::Ordering::SeqCst);

    let logs_dir = ensure_logs_dir()?;
    let date = format_date();
    let log_path = logs_dir.join(format!("aurora-desktop.{}.log", date));

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    *LOG_FILE
        .lock()
        .expect("LOG_FILE mutex poisoned during init") = Some(file);

    // 写启动分隔线
    log("init", "INFO", "══════════════════════════════════════════");
    log("init", "INFO", "Aurora Desktop 启动");
    log("init", "INFO", &format!("日志文件: {}", log_path.display()));

    Ok(log_path)
}

/// 写日志行。
pub fn log(module: &str, level: &str, message: &str) {
    let timestamp = format_timestamp();
    let line = format!("[{}] [{}] [{}] {}\n", timestamp, level, module, message);

    // 写文件 — 锁中毒意味着某线程 panic 时持有锁，日志系统不应静默丢失
    let mut guard = LOG_FILE
        .lock()
        .expect("LOG_FILE mutex poisoned — 日志系统不可恢复");
    if let Some(ref mut file) = *guard {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }

    // 同时输出到 stderr（debug 模式可见）
    eprint!("{}", line);
}

/// 便捷宏。
#[macro_export]
macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::log($module, "INFO", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::log($module, "WARN", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::log($module, "ERROR", &format!($($arg)*))
    };
}

// ── 内部辅助 ──────────────────────────────────────────────────────────

fn ensure_logs_dir() -> anyhow::Result<PathBuf> {
    let dir = crate::data_dir::aurora_data_dir().join("logs");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn format_date() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let mut secs = duration.as_secs();

    // 应用本地时区偏移，确保日期与本地时间一致
    let offset = LOCAL_UTC_OFFSET_SECS.load(std::sync::atomic::Ordering::SeqCst);
    if offset != i64::MAX {
        secs = (secs as i64 + offset).max(0) as u64;
    }

    let days = secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn format_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let mut secs = duration.as_secs();
    let millis = duration.subsec_millis();

    // 应用本地时区偏移，输出本地时间而非 UTC
    let offset = LOCAL_UTC_OFFSET_SECS.load(std::sync::atomic::Ordering::SeqCst);
    if offset != i64::MAX {
        secs = (secs as i64 + offset).max(0) as u64;
    }

    let h = (secs % 86400) / 3600;
    let mi = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, mi, s, millis)
}

/// 计算本地时区偏移秒数。
/// 使用 std 没有直接暴露的 local offset — 通过本地时间与 UTC 的差值推算。
fn compute_local_utc_offset_secs() -> i64 {
    // 使用 C 标准库的 localtime 获取偏移
    // 安全替代：直接读取 TZ 环境变量或用 SystemTime + 本地时间差
    // 最简方案：使用 chrono-independent 的方式 — 通过文件修改时间间接获取
    // 但最可靠的零依赖方案是调用平台 API
    //
    // 实际上 std::time::SystemTime 只有 UTC，无法直接获取偏移。
    // 使用 RFC 822/2822 日期格式间接提取时区信息：
    // Windows: 通过 GetTimeZoneInformation 获取
    // 这里用一个简化方案：读取环境变量 TZ 或默认为 UTC+8（中国标准时间）
    //
    // 正确做法：使用 `chrono` crate。但为保持零外部依赖，
    // 使用条件编译调用平台 API。

    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 timezoneapi 获取偏移
        // bias 单位是分钟，负数表示东区（与 POSIX 相反）
        // 但由于 forbid(unsafe_code)，无法直接调用 Windows API
        // 退而求其次：尝试从 TZ 环境变量解析
        compute_offset_from_tz()
    }

    #[cfg(not(target_os = "windows"))]
    {
        compute_offset_from_tz()
    }
}

/// 从 TZ 环境变量解析时区偏移。
/// TZ 格式示例: "Asia/Shanghai", "EST5EDT", ":/etc/localtime"
/// POSIX 格式: "std offset[dst[offset][,start[/time],end[/time]]]"
/// 简化处理：识别常见格式，否则默认 UTC+8（项目主要在中国使用）
fn compute_offset_from_tz() -> i64 {
    if let Ok(tz) = std::env::var("TZ") {
        // 尝试解析 POSIX 风格: "CST-8" → offset = +8h = 28800s
        // POSIX: 时区名后的数字是 west of UTC（即 -8 表示 UTC+8）
        if let Some(_digits) = tz.chars().find(|c| !c.is_alphabetic()) {
            let num_part: String = tz
                .chars()
                .skip_while(|c| c.is_alphabetic())
                .take_while(|c| c.is_ascii_digit() || *c == '+' || *c == '-')
                .collect();
            if let Ok(hours) = num_part.parse::<i64>() {
                // POSIX: 正数 = 西区（UTC 之后），取反得到东区偏移
                return -hours * 3600;
            }
        }
    }
    // 默认 UTC+8（中国标准时间）— 项目主要用户在中国
    8 * 3600
}

/// 从 UNIX epoch 天数计算年月日（简化版，不考虑闰秒）。
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        m += 1;
    }
    (y, m + 1, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}
