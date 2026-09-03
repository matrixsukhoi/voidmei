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

/// 历史基线: 两条 printf 格式串的逐字节期望 (各 level 全覆盖)
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
        assert_eq!(
            e2e_warn_err_marked(&line),
            marked,
            "WARN/ERROR 标记: {line}"
        );
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

// ===== Application.setDebugLog/setErrLog 重定向 (debugLog 开关面) =====

/// 重定向静态与级别静态同级共享进程, 触碰重定向的测试用此锁串行
static REDIRECT_LOCK: Mutex<()> = Mutex::new(());

/// Drop 清空重定向 (panic 也不污染后续测试的 stdout 捕获)
struct RedirectGuard;
impl Drop for RedirectGuard {
    fn drop(&mut self) {
        clear_redirects_for_test();
    }
}

/// setDebugLog 重定向后 Logger 输出改落文件 (格式与控制台同源 format_line);
/// setErrLog 同语义 (printStackTrace 面)。
/// 并行测试的日志与本测试共用进程级重定向静态 — 断言按唯一标记过滤行
/// (他测试的 INFO 行可能交错落进同一文件, 不参与计数); 级别面持 LEVEL_LOCK 串行。
#[test]
fn set_debug_log_redirects_output_to_file() {
    let (_lvl, _restore) = lock_level();
    let _g = REDIRECT_LOCK.lock().expect("测试重定向锁中毒");
    let _restore_redirect = RedirectGuard;
    let dir = env::temp_dir().join(format!("vm_logger_redirect_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out_path = dir.join("output.log");
    let err_path = dir.join("error.log");

    set_debug_log(out_path.to_str().unwrap());
    set_err_log(err_path.to_str().unwrap());
    assert_eq!(redirects_active_for_test(), (true, true));

    info("Update", "Latest remote version: 1.590");
    error_default_with_throwable("软失败", &TestIoError("boom".to_string()));
    // 默认 INFO 级下堆栈不打 (printStackTrace 闸门 = DEBUG), 先降级再触发
    set_min_level(Level::Trace);
    error_default_with_throwable("带堆栈", &TestIoError("trace-boom".to_string()));
    set_min_level(Level::Info);

    let out = std::fs::read_to_string(&out_path).unwrap();
    let lines: Vec<&str> = out
        .lines()
        .filter(|l| {
            l.contains("Latest remote version")
                || l.contains("软失败: boom")
                || l.contains("带堆栈: trace-boom")
        })
        .collect();
    assert_eq!(lines.len(), 3, "三条本测试日志行: {out:?}");
    assert!(
        e2e_timestamp_prefix_ok(lines[0]),
        "时间戳前缀失形: {}",
        lines[0]
    );
    assert_eq!(&lines[0][15..], "[Update    ] Latest remote version: 1.590");
    assert!(lines[1].contains("[App       ] [ERROR] 软失败: boom"));
    assert!(lines[2].contains("[App       ] [ERROR] 带堆栈: trace-boom"));
    let err = std::fs::read_to_string(&err_path).unwrap();
    assert_eq!(
        err.matches("java.io.IOException: trace-boom").count(),
        1,
        "DEBUG 级堆栈走 error.log: {err:?}"
    );
    assert_eq!(
        err.matches("java.io.IOException: boom").count(),
        0,
        "INFO 级软失败不打堆栈: {err:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 建文件失败 (父目录缺失): logAndContinue 语义 — 不 panic, 维持控制台输出
#[test]
fn set_debug_log_create_failure_keeps_console() {
    let _g = REDIRECT_LOCK.lock().expect("测试重定向锁中毒");
    let _restore = RedirectGuard;
    let bad = env::temp_dir()
        .join(format!("vm_logger_missing_{}", std::process::id()))
        .join("output.log"); // 父目录不存在 → File::create NotFound
    set_debug_log(bad.to_str().unwrap());
    assert!(!redirects_active_for_test().0, "失败后不应激活重定向");
    // 失败告警本身走控制台 warn (logAndContinue), 后续日志不受影响
    info("Service", "still on console");
    assert!(!redirects_active_for_test().0);
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
    error_with_throwable(
        "FMLoader",
        "FM加载异常(he_162)",
        &TestIoError("boom".to_string()),
    );
    // -- ERROR 级: event 静音 (INFO 闸门) --
    set_min_level(Level::Error);
    event("PUBLISH", "muted", Some("X"), Some("Y"));
    set_min_level(Level::Info); // 恢复默认
}

/// e2e 端到端格式钉子: 拉起子进程跑真实 stdout, 逐行断言
/// "[HH:mm:ss.SSS] [Component ] ([LEVEL] ) message" 逐字节形态与级别过滤。
#[test]
fn stdout_format_e2e_pin() {
    // 子进程测试名按 module_path! 拼接 (随目录移动自适应); --exact 需不带
    // crate 名前缀的路径 (曾硬编码旧路径致拉起空跑)
    let child_test = format!(
        "{}::child_emit_all_levels",
        module_path!()
            .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
            .unwrap_or(module_path!())
    );
    let out = Command::new(env::current_exe().expect("定位测试二进制失败"))
        .args(["--exact", child_test.as_str(), "--nocapture"])
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
