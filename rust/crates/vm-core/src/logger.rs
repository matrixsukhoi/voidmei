//! Logger 的 Rust 移植 (src/prog/util/Logger.java) — 一比一翻译。
//!
//! Structured logging utility for the application.
//! Supports log levels, timestamps, and component identifiers.
//!
//! 日志级别说明：
//! - TRACE: 最详细的跟踪信息（仅开发调试使用）
//! - DEBUG: 调试信息
//! - INFO: 一般信息（默认级别）
//! - WARN: 警告信息（不影响运行但需关注）
//! - ERROR: 错误信息（影响功能）
//!
//! PORT: Java 类仅含 static 成员 → Rust 模块级自由函数 + 静态
//! (format.rs / string_helper.rs 先例); Java 方法重载 (两参/单参) →
//! Rust 无重载, 主名留给两参版 (信息完整形态), 单参版加 `_default` 后缀
//! (语义 = javadoc "使用默认组件"; interpolation.rs / fm_power_extractor.rs
//! 的重载更名先例)。
//! PORT: §2.9 全局可变静态禁令 → DECISIONS.md **D5 显式豁免 (唯一例外)**:
//! 日志是横切面 + e2e 断言 (script/e2e_assert.py A1~A6) 依赖其输出格式,
//! 允许全局单例。currentLevel 需运行期 setMinLevel 变更, 采用 const 构造的
//! RwLock 静态 (RwLock::new 自 Rust 1.63 起 const 化, 无需再包一层
//! OnceLock; 豁免范围同等覆盖)。
//!
//! 输出格式逐字节保真 (e2e 断言依赖, 测试已钉住):
//!   INFO:    `[HH:mm:ss.SSS] [Component ] message`
//!   非 INFO: `[HH:mm:ss.SSS] [Component ] [LEVEL] message` (LEVEL 左对齐宽 5, 如 "WARN ")

use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::sync::{LazyLock, Mutex, RwLock};

use chrono::Local;

/// 对应 Java `public enum Level { TRACE(-1), DEBUG(0), INFO(1), WARN(2), ERROR(3) }`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    /// 对应 Java `final int value` 字段 (构造器入参, 过滤比较用)
    pub const fn value(self) -> i32 {
        match self {
            Level::Trace => -1,
            Level::Debug => 0,
            Level::Info => 1,
            Level::Warn => 2,
            Level::Error => 3,
        }
    }
}

/// Java 枚举默认 toString() = 常量名 (printf "%-5s" 直接吃它)。
/// 用 f.pad 落字而非 write_str — pad 才会应用 `{:<5}` 的宽度/对齐 spec
/// (write_str 静默丢弃宽度, "WARN " 会变 "WARN")。
impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        };
        f.pad(name)
    }
}

/// 对应 Java `private static Level currentLevel = Level.INFO`。
/// D5 豁免的全局单例 (见模块头)。每次读快照后立即释放锁, 输出 IO 不持锁
/// (对齐 Java 无同步原语的读写语义; 本模块无嵌套加锁点, §2.8 重入无涉)。
static CURRENT_LEVEL: RwLock<Level> = RwLock::new(Level::Info);

// 对应 Java `private static final SimpleDateFormat dateFormat =
// new SimpleDateFormat("HH:mm:ss.SSS");`
// PORT: SimpleDateFormat 共享静态实例本非线程安全 (Java 侧现存隐患),
// Rust 以无状态的 chrono 每次格式化等价替代; 默认时区语义一致
// (SimpleDateFormat 取 JVM 默认时区 = 系统本地时区 ↔ chrono::Local)。
// 落地在 log() 内的 timestamp 调用点。

/// 默认组件名，用于单参数日志方法
const DEFAULT_COMPONENT: &str = "App";

/// 对应 Java `public static void setMinLevel(Level level)`
pub fn set_min_level(level: Level) {
    // Java: currentLevel = level (plain 写, 无同步) → Rust 显式加锁写
    *CURRENT_LEVEL.write().expect("logger 级别锁中毒") = level;
}

// ===== stdout/stderr 文件重定向 (Application.setDebugLog/setErrLog, :362-382) =====
// Java debugLog 开关 (Application.main:550-553) 把 System.setOut/setErr 指到
// ./output.log / ./error.log — Logger 的输出口径整体随 System.out 走, 故 Rust 侧
// 由 Logger 输出面接管 (println!/eprintln! 的其余调用点为测试模式打字, 不走本面)。
// D5 豁免同源 (见模块头): 横切面全局静态。

/// stdout 重定向目标 (None = 控制台 stdout, 默认)
static OUT_REDIRECT: LazyLock<Mutex<Option<File>>> = LazyLock::new(|| Mutex::new(None));
/// stderr 重定向目标 (printStackTrace 面; None = 控制台 stderr)
static ERR_REDIRECT: LazyLock<Mutex<Option<File>>> = LazyLock::new(|| Mutex::new(None));

/// 对应 Java `setDebugLog(String path)` (Application.java:362-371):
/// 建文件失败 (FileNotFoundException) → logAndContinue("日志文件") 后维持原输出,
/// 不中断启动 (out=null 时 Java System.setOut(null) 会令后续打印 NPE, 属 Java 原
/// 坑; Rust 侧失败即保持控制台输出, 有意加强)。
pub fn set_debug_log(path: &str) {
    let created = File::create(path);
    let mut slot = OUT_REDIRECT.lock().expect("logger stdout 重定向锁中毒");
    match created {
        Ok(f) => *slot = Some(f),
        Err(e) => {
            // 日志文件创建失败，使用统一异常处理 (ExceptionHelper.logAndContinue:
            // 单参 warn, 组件取默认 "App", 消息 = context + ": " + message)
            drop(slot);
            warn_default(&format!("日志文件: {e}"));
        }
    }
}

/// 对应 Java `setErrLog(String path)` (Application.java:373-382), 同上语义
pub fn set_err_log(path: &str) {
    let created = File::create(path);
    let mut slot = ERR_REDIRECT.lock().expect("logger stderr 重定向锁中毒");
    match created {
        Ok(f) => *slot = Some(f),
        Err(e) => {
            // 错误日志文件创建失败，使用统一异常处理
            drop(slot);
            warn_default(&format!("错误日志文件: {e}"));
        }
    }
}

/// 单行落盘 (stdout 重定向态): writeln! 到 File 无 println! 的 panic 面, 同样吞错
fn write_out(line: &str) {
    let mut slot = OUT_REDIRECT.lock().expect("logger stdout 重定向锁中毒");
    if let Some(f) = slot.as_mut() {
        let _ = f.write_all(line.as_bytes());
        let _ = f.write_all(b"\n");
    } else {
        drop(slot);
        let _ = writeln!(io::stdout().lock(), "{line}");
    }
}

/// 堆栈行落盘 (stderr 重定向态, printStackTrace 面)
fn write_err(line: &str) {
    let mut slot = ERR_REDIRECT.lock().expect("logger stderr 重定向锁中毒");
    if let Some(f) = slot.as_mut() {
        let _ = f.write_all(line.as_bytes());
        let _ = f.write_all(b"\n");
    } else {
        drop(slot);
        let _ = writeln!(io::stderr().lock(), "{line}");
    }
}

/// 测试专用: 重定向激活状态 (out, err)
#[cfg(test)]
pub(crate) fn redirects_active_for_test() -> (bool, bool) {
    let o = OUT_REDIRECT.lock().expect("logger stdout 重定向锁中毒").is_some();
    let e = ERR_REDIRECT.lock().expect("logger stderr 重定向锁中毒").is_some();
    (o, e)
}

/// 测试专用: 清空重定向恢复控制台输出 (Drop 守卫恢复语义的显式面)
#[cfg(test)]
pub(crate) fn clear_redirects_for_test() {
    *OUT_REDIRECT.lock().expect("logger stdout 重定向锁中毒") = None;
    *ERR_REDIRECT.lock().expect("logger stderr 重定向锁中毒") = None;
}

/// 获取当前日志级别
/// @return 当前日志级别
pub fn get_level() -> Level {
    current_level()
}

/// Java 各处 `currentLevel.value` 读取点的统一映射 (log/error/event 每处独立读取)
fn current_level() -> Level {
    *CURRENT_LEVEL.read().expect("logger 级别锁中毒")
}

// ===== 两参数方法（指定组件） =====

pub fn trace(component: &str, message: &str) {
    log(Level::Trace, component, message);
}

pub fn debug(component: &str, message: &str) {
    log(Level::Debug, component, message);
}

pub fn info(component: &str, message: &str) {
    log(Level::Info, component, message);
}

pub fn warn(component: &str, message: &str) {
    log(Level::Warn, component, message);
}

pub fn error(component: &str, message: &str) {
    log(Level::Error, component, message);
}

/// 记录错误信息和异常详情
/// 在 DEBUG 级别时打印完整堆栈
/// PORT: Java `Throwable` → `&dyn std::error::Error`; `getMessage()` ↔ `to_string()`
/// (Rust 无 null message 形态, Java "msg: null" 拼接分支不可达);
/// `printStackTrace()` (打到 System.err) ↔ stderr 写 `{t:?}` — Rust 错误类型
/// 不携带 Java 形堆栈, 以 Debug repr 顶位 (调用方错误类型的 Debug 决定形态)。
pub fn error_with_throwable(component: &str, message: &str, t: &dyn std::error::Error) {
    // Java: log(Level.ERROR, component, message + ": " + t.getMessage())
    log(Level::Error, component, &format!("{message}: {t}"));
    // Java: if (currentLevel.value <= Level.DEBUG.value) — 与 log() 内各读一次, 两处独立读取
    if current_level().value() <= Level::Debug.value() {
        // Java: t.printStackTrace() — PrintStream 吞 IOException, 见 log() 内 PORT 注释
        write_err(&format!("{t:?}"));
    }
}

// ===== 单参数方法（使用默认组件） =====

pub fn trace_default(message: &str) {
    log(Level::Trace, DEFAULT_COMPONENT, message);
}

pub fn debug_default(message: &str) {
    log(Level::Debug, DEFAULT_COMPONENT, message);
}

pub fn info_default(message: &str) {
    log(Level::Info, DEFAULT_COMPONENT, message);
}

pub fn warn_default(message: &str) {
    log(Level::Warn, DEFAULT_COMPONENT, message);
}

pub fn error_default(message: &str) {
    log(Level::Error, DEFAULT_COMPONENT, message);
}

/// 记录错误信息和异常详情（单参数版本）
/// 在 DEBUG 级别时打印完整堆栈
pub fn error_default_with_throwable(message: &str, t: &dyn std::error::Error) {
    log(Level::Error, DEFAULT_COMPONENT, &format!("{message}: {t}"));
    if current_level().value() <= Level::Debug.value() {
        // Java: t.printStackTrace() — PrintStream 吞 IOException, 见 log() 内 PORT 注释
        write_err(&format!("{t:?}"));
    }
}

/// Specialized logging for event transactions.
/// PORT: Java `(Object source, Object target)` 的 `getClass().getSimpleName()` /
/// `toString()` 反射取值在 Rust 无对应 (禁引反射库, §1), 由调用方直接给出字符串:
/// source 传类简单名 (如 "FMManager"), target 传 toString 结果 (如
/// "FMHandle[MISSING he_162]"); null 语义 ↔ `Option::None` ("Unknown"/"Global")。
pub fn event(action: &str, event_name: &str, source: Option<&str>, target: Option<&str>) {
    // Java: if (currentLevel.value <= Level.INFO.value)
    if current_level().value() <= Level::Info.value() {
        // Java: String.format("%s: %s -> %s: %s", action, source名, target串, eventName)
        let msg = format!(
            "{}: {} -> {}: {}",
            action,
            source.unwrap_or("Unknown"),
            target.unwrap_or("Global"),
            event_name
        );
        log(Level::Info, "EventBus", &msg);
    }
}

/// 对应 Java `private static void log(Level level, String component, String message)`
fn log(level: Level, component: &str, message: &str) {
    // Java: if (level.value >= currentLevel.value)
    if level.value() >= current_level().value() {
        // Java: String timestamp = dateFormat.format(new Date()) — "HH:mm:ss.SSS" 本地时区
        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
        // 单次持锁写入对齐 Java printf 单调用 (System.out 自带锁, 多线程行不交错);
        // setDebugLog 重定向态下改走文件 (write_out 内吞错, 见其注)。
        // Java %n 是平台行分隔符 (Windows \r\n) ↔ 此处固定 \n — e2e 断言按行
        // 解析 (splitlines/rstrip), 两者等价; 见 port_notes。
        // PORT: Java 8 System.out 按平台默认字符集编码 (zh-CN Windows=GBK),
        // Rust stdout 恒 UTF-8 — e2e_assert.py 以 utf-8+errors=replace 读取, 兼容
        // 且更优; 已接受偏差, 与 %n 条目并列备案。
        // PORT: Java PrintStream 按设计吞掉 IOException 永不抛出, 而 println!/
        // eprintln! 写失败 (broken pipe / Windows GUI 子系统无控制台句柄) 会 panic —
        // 故写入面统一吞错对齐之 (§6 catch_unwind 只兜 Service, 日志调用不该炸调用线程)。
        write_out(&format_line(level, component, message, &timestamp));
    }
}

/// Java log() 内两条 printf 的格式落地 (B 类适配层提取, 测试钉住 e2e 依赖的逐字节格式):
/// - INFO:    `System.out.printf("[%s] [%-10s] %s%n", timestamp, component, message)`
/// - 非 INFO: `System.out.printf("[%s] [%-10s] [%-5s] %s%n", timestamp, component, level, message)`
///
/// PORT: `%-10s`/`%-5s` 左对齐补空格只补不截; Java 宽度按 UTF-16 码元计,
/// Rust `{:<10}` 按字符计 — 域内组件名全 ASCII ("Service"/"EventBus"/"App"...), 无差异。
fn format_line(level: Level, component: &str, message: &str, timestamp: &str) -> String {
    if level == Level::Info {
        format!("[{timestamp}] [{component:<10}] {message}")
    } else {
        format!("[{timestamp}] [{component:<10}] [{level:<5}] {message}")
    }
}

// =====================================================================
// Tests — 格式断言 (e2e script/e2e_assert.py A1~A6 依赖本格式, 逐字节钉住)
// =====================================================================
#[cfg(test)]
mod tests;
