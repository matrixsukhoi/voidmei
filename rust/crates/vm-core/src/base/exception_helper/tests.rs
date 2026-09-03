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

// (波21 清场: ignore/close_quietly/sleep_quietly_strict/log_and_continue
//  四个死函数及对应用例已删, 仅存 sleep_* 与 panic 载荷的活用例)
