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
use std::io::{self, Write};
use std::sync::RwLock;

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
        let _ = writeln!(io::stderr().lock(), "{t:?}");
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
        let _ = writeln!(io::stderr().lock(), "{t:?}");
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
        // 单次持锁 writeln 对齐 Java printf 单调用 (System.out 自带锁, 多线程行不交错)。
        // Java %n 是平台行分隔符 (Windows \r\n) ↔ 此处固定 \n — e2e 断言按行
        // 解析 (splitlines/rstrip), 两者等价; 见 port_notes。
        // PORT: Java 8 System.out 按平台默认字符集编码 (zh-CN Windows=GBK),
        // Rust stdout 恒 UTF-8 — e2e_assert.py 以 utf-8+errors=replace 读取, 兼容
        // 且更优; 已接受偏差, 与 %n 条目并列备案。
        // PORT: Java PrintStream 按设计吞掉 IOException 永不抛出, 而 println!/
        // eprintln! 写失败 (broken pipe / Windows GUI 子系统无控制台句柄) 会 panic —
        // 故用 `let _ = writeln!` 吞错写入对齐之 (§6 catch_unwind 只兜 Service,
        // 日志调用不该炸调用线程)。
        let _ = writeln!(
            io::stdout().lock(),
            "{}",
            format_line(level, component, message, &timestamp)
        );
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
mod tests {
    use super::*;
    use std::env;
    use std::process::Command;
    use std::sync::{Mutex, MutexGuard};

    /// CURRENT_LEVEL 是进程级静态 (D5 豁免), 触碰级别的测试用此锁串行
    static LEVEL_LOCK: Mutex<()> = Mutex::new(());

    /// Drop 恢复默认 INFO (panic 也不污染后续测试)
    struct LevelGuard;
    impl Drop for LevelGuard {
        fn drop(&mut self) {
            set_min_level(Level::Info);
        }
    }

    fn lock_level() -> (MutexGuard<'static, ()>, LevelGuard) {
        let g = LEVEL_LOCK.lock().expect("测试级别锁中毒");
        (g, LevelGuard)
    }

    /// script/e2e_assert.py RE_TIMESTAMP = `^\[(\d{2}):(\d{2}):(\d{2})\.(\d{3})\]`
    /// 的手工复刻 (vm-core 无 regex 依赖): 行首 `[HH:mm:ss.SSS]` 共 14 字节
    fn e2e_timestamp_prefix_ok(line: &str) -> bool {
        let b = line.as_bytes();
        b.len() >= 14
            && b[0] == b'['
            && b[1].is_ascii_digit()
            && b[2].is_ascii_digit()
            && b[3] == b':'
            && b[4].is_ascii_digit()
            && b[5].is_ascii_digit()
            && b[6] == b':'
            && b[7].is_ascii_digit()
            && b[8].is_ascii_digit()
            && b[9] == b'.'
            && b[10].is_ascii_digit()
            && b[11].is_ascii_digit()
            && b[12].is_ascii_digit()
            && b[13] == b']'
    }

    /// script/e2e_assert.py RE_WARN_ERR = `\[(WARN|ERROR)\s*\]` 对本格式输出的匹配复刻
    fn e2e_warn_err_marked(line: &str) -> bool {
        line.contains("[WARN ]") || line.contains("[ERROR]")
    }

    /// Java oracle: 两条 printf 格式串的逐字节期望 (各 level 全覆盖)
    #[test]
    fn java8_printf_format_all_levels() {
        // INFO: printf("[%s] [%-10s] %s%n", ts, component, message)
        assert_eq!(
            format_line(Level::Info, "Service", "启动成功", "14:23:45.123"),
            "[14:23:45.123] [Service   ] 启动成功"
        );
        // 非 INFO: printf("[%s] [%-10s] [%-5s] %s%n", ts, component, level, message)
        assert_eq!(
            format_line(Level::Trace, "Deep", "t", "01:02:03.004"),
            "[01:02:03.004] [Deep      ] [TRACE] t"
        );
        assert_eq!(
            format_line(Level::Debug, "Dbg", "d", "23:59:59.999"),
            "[23:59:59.999] [Dbg       ] [DEBUG] d"
        );
        assert_eq!(
            format_line(Level::Warn, "Config", "缺失", "00:00:00.000"),
            "[00:00:00.000] [Config    ] [WARN ] 缺失"
        );
        assert_eq!(
            format_line(Level::Error, "Network", "失败", "09:08:07.654"),
            "[09:08:07.654] [Network   ] [ERROR] 失败"
        );
    }

    /// `%-10s`/`%-5s` 边界: 只补不截 / 空串 / 恰好等宽 / 默认组件 "App" 形态
    #[test]
    fn java8_printf_padding_edges() {
        // 超宽组件名不截断 (Java %-10s 只补不截)
        assert_eq!(
            format_line(Level::Warn, "VoiceResourceManager", "x", "00:00:00.000"),
            "[00:00:00.000] [VoiceResourceManager] [WARN ] x"
        );
        // 空组件名 → 10 个空格
        assert_eq!(
            format_line(Level::Error, "", "m", "00:00:00.000"),
            "[00:00:00.000] [          ] [ERROR] m"
        );
        // 恰好 10 宽不补
        assert_eq!(
            format_line(Level::Info, "0123456789", "m", "00:00:00.000"),
            "[00:00:00.000] [0123456789] m"
        );
        // 单参方法的默认组件 "App" (3 字符) 宽度形态
        assert_eq!(
            format_line(Level::Warn, DEFAULT_COMPONENT, "单参警告", "00:00:00.000"),
            "[00:00:00.000] [App       ] [WARN ] 单参警告"
        );
    }

    /// 钉住 e2e 两个行级正则对本格式的可匹配性:
    /// A5 总量计数依赖 RE_TIMESTAMP 行首命中, A6 WARN/ERROR 速率依赖 RE_WARN_ERR
    #[test]
    fn e2e_regex_shape_pin() {
        for (lv, marked) in [
            (Level::Trace, false),
            (Level::Debug, false),
            (Level::Info, false),
            (Level::Warn, true),
            (Level::Error, true),
        ] {
            let line = format_line(lv, "Service", "msg", "12:34:56.789");
            assert!(e2e_timestamp_prefix_ok(&line), "时间戳前缀失形: {line}");
            assert_eq!(e2e_warn_err_marked(&line), marked, "WARN/ERROR 标记: {line}");
        }
        // 组件名超宽不改变时间戳前缀位置 (前缀恒定 14 字节)
        let line = format_line(Level::Error, "VoiceResourceManager", "m", "12:34:56.789");
        assert!(e2e_timestamp_prefix_ok(&line));
    }

    /// Java 构造器序值: TRACE(-1), DEBUG(0), INFO(1), WARN(2), ERROR(3)
    #[test]
    fn level_value_ordering() {
        assert_eq!(Level::Trace.value(), -1);
        assert_eq!(Level::Debug.value(), 0);
        assert_eq!(Level::Info.value(), 1);
        assert_eq!(Level::Warn.value(), 2);
        assert_eq!(Level::Error.value(), 3);
        // log 过滤方向: level.value >= currentLevel.value 放行
        assert!(Level::Trace.value() < Level::Info.value());
        assert!(Level::Error.value() >= Level::Info.value());
    }

    /// setMinLevel/getLevel 往返 (全局静态需串行 + 恢复)
    #[test]
    fn set_and_get_level_roundtrip() {
        let (_g, restore) = lock_level();
        set_min_level(Level::Debug);
        assert_eq!(get_level(), Level::Debug);
        set_min_level(Level::Error);
        assert_eq!(get_level(), Level::Error);
        drop(restore); // Drop 恢复默认 INFO
        assert_eq!(get_level(), Level::Info);
    }

    /// 测试用 Throwable 替身: Display = getMessage(), Debug 刻意复刻 Java
    /// printStackTrace 首行形态 ("类全名: 消息"), 对齐 e2e A2 的 RE_EXC_FIRST 域
    #[derive(Clone)]
    struct TestIoError(String);
    impl fmt::Display for TestIoError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }
    impl fmt::Debug for TestIoError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "java.io.IOException: {}", self.0)
        }
    }
    impl std::error::Error for TestIoError {}

    /// 子进程样本发射器: 由 stdout_format_e2e_pin 以
    /// `--exact logger::tests::child_emit_all_levels --nocapture` 拉起, 走真实
    /// println 路径产出各 level 日志; 正常套件内直接运行时同样成立 (输出进
    /// libtest 捕获, 级别改动经 LEVEL_LOCK 串行并恢复)。
    #[test]
    fn child_emit_all_levels() {
        // 与其余触级测试一致: lock_level() 串行 + LevelGuard panic 安全恢复
        let (_g, _restore) = lock_level();
        // -- 默认 INFO 级: trace/debug 滤除, 其余放行; throwable 不打堆栈 --
        set_min_level(Level::Info);
        trace("Hidden", "t");
        debug("Hidden", "d");
        info("Service", "启动成功");
        warn("Config", "配置缺失: key");
        error("Network", "Connection failed");
        info_default("App started");
        warn_default("单参警告");
        error_default_with_throwable("软失败", &TestIoError("quiet".to_string()));
        event(
            "PUBLISH",
            "fmChanged",
            Some("FMManager"),
            Some("FMHandle[MISSING he_162]"),
        );
        event("SUBSCRIBE", "uiChanged", Some("MainForm"), None);
        // -- TRACE 级: 全量放行, throwable 堆栈走 stderr --
        set_min_level(Level::Trace);
        trace("Deep", "trace msg");
        debug("Dbg", "debug msg");
        error_with_throwable("FMLoader", "FM加载异常(he_162)", &TestIoError("boom".to_string()));
        // -- ERROR 级: event 静音 (INFO 闸门) --
        set_min_level(Level::Error);
        event("PUBLISH", "muted", Some("X"), Some("Y"));
        set_min_level(Level::Info); // 恢复默认
    }

    /// e2e 端到端格式钉子: 拉起子进程跑真实 stdout, 逐行断言
    /// "[HH:mm:ss.SSS] [Component ] ([LEVEL] ) message" 逐字节形态与级别过滤。
    #[test]
    fn stdout_format_e2e_pin() {
        let out = Command::new(env::current_exe().expect("定位测试二进制失败"))
            .args(["--exact", "logger::tests::child_emit_all_levels", "--nocapture"])
            .output()
            .expect("拉起子进程失败");
        assert!(out.status.success(), "子进程测试失败: {out:?}");
        let stdout = String::from_utf8(out.stdout).expect("stdout 非 UTF-8");
        let stderr = String::from_utf8(out.stderr).expect("stderr 非 UTF-8");

        // 只留 Logger 产出行 (滤掉 libtest 自身输出 "running 1 test" 等)
        let lines: Vec<&str> = stdout
            .lines()
            .filter(|l| e2e_timestamp_prefix_ok(l))
            .collect();
        // INFO 段 8 行 + TRACE 段 3 行; Hidden×2 与 ERROR 级 event 被滤除
        assert_eq!(lines.len(), 11, "期望 11 条日志行, 实得: {lines:?}");
        assert!(!stdout.contains("muted"), "ERROR 级 event 不应输出");
        assert!(!stdout.contains("Hidden"), "INFO 级下 trace/debug 应被滤除");

        // 时间戳前缀 `[HH:mm:ss.SSS]` 恒 14 字节 ASCII + 1 个分隔空格, 其后为逐字节期望的正文
        let expected = [
            "[Service   ] 启动成功",
            "[Config    ] [WARN ] 配置缺失: key",
            "[Network   ] [ERROR] Connection failed",
            "[App       ] App started",
            "[App       ] [WARN ] 单参警告",
            "[App       ] [ERROR] 软失败: quiet",
            "[EventBus  ] PUBLISH: FMManager -> FMHandle[MISSING he_162]: fmChanged",
            "[EventBus  ] SUBSCRIBE: MainForm -> Global: uiChanged",
            "[Deep      ] [TRACE] trace msg",
            "[Dbg       ] [DEBUG] debug msg",
            "[FMLoader  ] [ERROR] FM加载异常(he_162): boom",
        ];
        for (got, want) in lines.iter().zip(expected) {
            assert_eq!(&got[15..], want);
        }
        // A5/A6 依赖的行级标记
        assert!(lines.iter().all(|l| e2e_timestamp_prefix_ok(l)));
        assert!(!e2e_warn_err_marked(lines[0]), "INFO 行无级别标记");
        assert!(e2e_warn_err_marked(lines[1]) && e2e_warn_err_marked(lines[2]));

        // printStackTrace 通道 (stderr): 仅 TRACE 段一次, INFO 段软失败不打
        assert_eq!(stderr.matches("java.io.IOException: boom").count(), 1);
        assert_eq!(stderr.matches("java.io.IOException: quiet").count(), 0);
    }
}
