use super::*;
use std::sync::Arc;

// ---- sleep_quietly: 正常路径 (睡满) ----

#[test]
fn sleep_quietly_sleeps_full_duration_when_no_stop() {
    let stop = AtomicBool::new(false);
    let t0 = Instant::now();
    sleep_quietly(&stop, 120);
    let elapsed = t0.elapsed();
    assert!(elapsed.as_millis() >= 120, "实际 {:?} < 120ms", elapsed);
    // 上界只是防挂死哨兵, 不承载断言语义 (睡满语义由下界钉住)。
    // 12 片 × Windows 默认计时器粒度 ~15.6ms 最坏 ~187ms, 再留重载
    // 机器调度抖动余量 → 600ms
    assert!(elapsed.as_millis() < 600, "实际 {:?} 过长", elapsed);
    assert!(!stop.load(Ordering::SeqCst), "正常睡满不应置位标志");
}

#[test]
fn sleep_quietly_zero_millis_returns_immediately() {
    // 对齐 Java Thread.sleep(0): 立即返回
    let stop = AtomicBool::new(false);
    let t0 = Instant::now();
    sleep_quietly(&stop, 0);
    assert!(t0.elapsed().as_millis() < 50);
}

// ---- sleep_quietly: 停机路径 (§2.13 中断映射) ----

#[test]
fn sleep_quietly_returns_immediately_when_flag_pre_set() {
    let stop = AtomicBool::new(true);
    let t0 = Instant::now();
    sleep_quietly(&stop, 60_000);
    assert!(t0.elapsed().as_millis() < 500, "预置标志应立即返回");
    // 恢复中断状态语义: 提前返回后标志保持置位
    assert!(stop.load(Ordering::SeqCst));
}

#[test]
fn sleep_quietly_returns_early_when_flag_set_midway() {
    let stop = Arc::new(AtomicBool::new(false));
    let setter = {
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(40));
            stop.store(true, Ordering::SeqCst);
        })
    };
    let t0 = Instant::now();
    sleep_quietly(&stop, 60_000);
    let elapsed = t0.elapsed();
    assert!(
        elapsed.as_millis() >= 35,
        "不应在标志置位前返回, 实际 {:?}",
        elapsed
    );
    // 响应延迟上界 = 一个轮询片 (10ms) + 调度误差, 放宽到 2s 防重载机器抖动
    assert!(
        elapsed.as_millis() < 2_000,
        "置位后应及时返回, 实际 {:?}",
        elapsed
    );
    // 标志保持置位 = 恢复中断状态 (上游可观察)
    assert!(stop.load(Ordering::SeqCst));
    setter.join().unwrap();
}

// ---- sleep_while_run: 运行极性辅助 (备案收口: other_service/flight_log 专用) ----

#[test]
fn sleep_while_run_sleeps_full_duration_when_running() {
    // 极性回归: is_run/logon 这类 true=运行 标志在睡眠期间保持 true → 睡满
    // (接反 sleep_quietly 会立即返回 → 热自旋, 本测试即钉住该缺陷)
    let run = AtomicBool::new(true);
    let t0 = Instant::now();
    sleep_while_run(&run, 120);
    let elapsed = t0.elapsed();
    assert!(elapsed.as_millis() >= 120, "实际 {:?} < 120ms", elapsed);
    // 上界哨兵同 sleep_quietly_sleeps_full_duration (Windows 计时器粒度 + 抖动余量)
    assert!(elapsed.as_millis() < 600, "实际 {:?} 过长", elapsed);
    assert!(run.load(Ordering::SeqCst));
}

#[test]
fn sleep_while_run_returns_immediately_when_run_pre_clear() {
    // 进入时运行标志已 false → 立即返回 (循环即刻退出, 等价中断位已置位)
    let run = AtomicBool::new(false);
    let t0 = Instant::now();
    sleep_while_run(&run, 60_000);
    assert!(t0.elapsed().as_millis() < 500, "预清标志应立即返回");
}

#[test]
fn sleep_while_run_returns_early_when_run_cleared_midway() {
    let run = Arc::new(AtomicBool::new(true));
    let clearer = {
        let run = Arc::clone(&run);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(40));
            run.store(false, Ordering::SeqCst);
        })
    };
    let t0 = Instant::now();
    sleep_while_run(&run, 60_000);
    let elapsed = t0.elapsed();
    assert!(
        elapsed.as_millis() >= 35,
        "不应在标志清零前返回, 实际 {:?}",
        elapsed
    );
    // 响应延迟上界 = 一个轮询片 (10ms) + 调度误差 (防重载机器抖动)
    assert!(
        elapsed.as_millis() < 2_000,
        "清零后应及时返回, 实际 {:?}",
        elapsed
    );
    clearer.join().unwrap();
}

// ---- sleep_quietly_strict: 不可中断版 ----

#[test]
fn sleep_quietly_strict_sleeps_full_duration() {
    let t0 = Instant::now();
    sleep_quietly_strict(60);
    assert!(t0.elapsed().as_millis() >= 60);
}

// ---- ignore: 恢复中断状态 → 置位停机标志 ----

#[test]
fn ignore_sets_stop_flag() {
    let stop = AtomicBool::new(false);
    ignore(&stop);
    assert!(stop.load(Ordering::SeqCst));
}

// ---- close_quietly: Drop 语义 ----

#[test]
fn close_quietly_none_is_noop() {
    close_quietly(None::<std::fs::File>);
}

#[test]
fn close_quietly_some_drops_resource() {
    // Drop 打点资源验证 close 语义 (所有权交出 → Drop 触发 = close 执行)
    struct Res(Arc<AtomicBool>);
    impl Drop for Res {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let closed = Arc::new(AtomicBool::new(false));
    let res = Res(Arc::clone(&closed));
    assert!(!closed.load(Ordering::SeqCst), "关闭前不应触发 Drop");
    close_quietly(Some(res));
    assert!(
        closed.load(Ordering::SeqCst),
        "close_quietly 应触发 Drop (= close)"
    );
}

// ---- log_and_continue: 接 crate::base::logger, 控制流不变 ----

#[test]
fn log_and_continue_does_not_panic() {
    // 冒烟: WARN 记录 + 控制流不中断 (不 panic 即通过)
    #[derive(Debug)]
    struct FakeErr;
    impl std::fmt::Display for FakeErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "boom")
        }
    }
    impl std::error::Error for FakeErr {}
    log_and_continue(&FakeErr, "测试上下文");
}

// ---- log_and_continue: DEBUG 闸门分支 (子进程隔离, 见下) ----

/// 测试用 Throwable 替身: Display = getMessage(); Debug 刻意复刻 Java
/// printStackTrace 首行形态 ("类全名: 消息"), 对齐 e2e A2 的 RE_EXC_FIRST 域
/// (与 logger.rs 测试的 TestIoError 同约定)
#[derive(Clone)]
struct TestIoError(String);
impl std::fmt::Display for TestIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::fmt::Debug for TestIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "java.io.IOException: {}", self.0)
    }
}
impl std::error::Error for TestIoError {}

/// 带包装原因的替身: source() 链 → printStackTrace 的 "Caused by:" 行
struct ChainedErr(TestIoError);
impl std::fmt::Display for ChainedErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("chained")
    }
}
impl std::fmt::Debug for ChainedErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("java.io.IOException: chained")
    }
}
impl std::error::Error for ChainedErr {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// 子进程样本发射器 (logger.rs child_emit_all_levels 先例)。
/// CURRENT_LEVEL 是进程级静态 (D5 豁免), 而 logger.rs 的 LEVEL_LOCK
/// 测试锁未导出 — 正常套件内并行运行时若触碰级别会与 logger::tests
/// 的级别断言竞争。故仅在以 `--exact 本测试名` 单独拉起时执行设级
/// 与发射; 套件内直跑时为空操作 (真正的断言在父测试 log_and_continue_debug_gate,
/// 不存在假通过)。
#[test]
fn child_log_and_continue_debug_channel() {
    let args: Vec<String> = std::env::args().collect();
    let solo = args.iter().any(|a| a == "--exact")
        && args
            .iter()
            .any(|a| a.contains("child_log_and_continue_debug_channel"));
    if !solo {
        return;
    }
    // DEBUG 级: 闸门开 → stderr 首行 (Debug repr) + Caused by 链
    crate::base::logger::set_min_level(crate::base::logger::Level::Debug);
    log_and_continue(&ChainedErr(TestIoError("root".to_string())), "闸门上下文");
    // INFO 级 (默认): 闸门关 → 仅 WARN 行, stderr 无 printStackTrace 通道输出
    crate::base::logger::set_min_level(crate::base::logger::Level::Info);
    log_and_continue(&TestIoError("quiet".to_string()), "闸门上下文");
}

/// DEBUG 闸门分支钉子: 拉起子进程跑真实 stderr, 断言 (1) DEBUG 级输出
/// Debug 首行形态 (RE_EXC_FIRST 可匹配域) + Caused by 链; (2) INFO 级
/// (默认) 不输出该通道; (3) WARN 行两级都放行 (不受闸门影响)。
#[test]
fn log_and_continue_debug_gate() {
    // 子进程测试名按 module_path! 拼接 (随目录移动自适应); --exact 需不带
    // crate 名前缀的路径 (曾硬编码旧路径致拉起空跑)
    let child_test = format!(
        "{}::child_log_and_continue_debug_channel",
        module_path!()
            .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
            .unwrap_or(module_path!())
    );
    let out = std::process::Command::new(std::env::current_exe().expect("定位测试二进制失败"))
        .args(["--exact", child_test.as_str(), "--nocapture"])
        .output()
        .expect("拉起子进程失败");
    assert!(out.status.success(), "子进程测试失败: {out:?}");
    let stdout = String::from_utf8(out.stdout).expect("stdout 非 UTF-8");
    let stderr = String::from_utf8(out.stderr).expect("stderr 非 UTF-8");

    // DEBUG 级一次: 首行 + Caused by 链各一
    assert_eq!(stderr.matches("java.io.IOException: chained").count(), 1);
    assert_eq!(
        stderr
            .matches("Caused by: java.io.IOException: root")
            .count(),
        1
    );
    // INFO 级 (默认) 一次: 闸门关, printStackTrace 通道静默
    assert_eq!(stderr.matches("java.io.IOException: quiet").count(), 0);
    // WARN 行两次都在 (闸门只管 stderr 通道, WARN 由级别过滤放行)
    assert_eq!(stdout.matches("闸门上下文: chained").count(), 1);
    assert_eq!(stdout.matches("闸门上下文: quiet").count(), 1);
}
