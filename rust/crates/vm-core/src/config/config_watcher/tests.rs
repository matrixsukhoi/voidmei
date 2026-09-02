use super::*;
use std::sync::atomic::AtomicU32;
use std::time::{Duration, Instant};

/// 生成唯一临时文件路径 (原子计数 + 纳秒时间戳, 防并行测试互踩)
fn temp_file(tag: &str) -> std::path::PathBuf {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "vm_core_config_watcher_{}_{}_{}.tmp",
        tag, n, nanos
    ))
}

/// 反复写入直到观察到严格大于 floor 的 mtime。
/// 兜底粗粒度文件系统 (FAT 2s / ext3 1s); NTFS/ext4 首次写入即满足。
fn write_until_newer(path: &Path, floor: i64) -> i64 {
    for _ in 0..250 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::fs::write(path, format!("tick-{}", nanos)).unwrap();
        let m = file_last_modified_millis(path);
        if m > floor {
            return m;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("写入 250 次仍未观察到 mtime 前进 (文件系统粒度过粗?)");
}

/// 轮询等待条件成立 (10ms 粒度), 超时 panic — 不允许假通过
fn wait_until(timeout_ms: u64, cond: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while !cond() {
        if Instant::now() >= deadline {
            panic!("等待条件超时 ({}ms)", timeout_ms);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// 计数回调 + 读取句柄
fn counter() -> (Arc<AtomicU32>, impl FnMut() + Send + 'static) {
    let hits = Arc::new(AtomicU32::new(0));
    let h = Arc::clone(&hits);
    (hits, move || {
        h.fetch_add(1, Ordering::SeqCst);
    })
}

// ---- 构造边界: 存在文件捕获当前 mtime / 缺失文件归 0 ----

#[test]
fn new_existing_file_captures_mtime() {
    let path = temp_file("ctor_exists");
    std::fs::write(&path, "init").unwrap();
    let svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(|| {}));
    let expected = file_last_modified_millis(&path);
    assert!(expected > 0, "刚写入的文件 mtime 应大于 0");
    let st = lock_state(&svc.state);
    assert_eq!(st.last_mod_time, expected);
    assert!(!st.ignore_next, "ignoreNext 显式初始化为 false");
    drop(st);
    std::fs::remove_file(&path).ok();
}

#[test]
fn new_missing_file_last_mod_time_is_zero() {
    let path = temp_file("ctor_missing"); // 不创建文件
    let svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(|| {}));
    assert_eq!(lock_state(&svc.state).last_mod_time, 0);
}

// ---- check 边界 (Java 私有方法, 同模块直测) ----

#[test]
fn check_missing_file_returns_early() {
    let path = temp_file("check_missing"); // 不创建文件
    let (hits, cb) = counter();
    let svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    check(&svc.file_path, &svc.state, &svc.on_reload);
    assert_eq!(hits.load(Ordering::SeqCst), 0, "文件缺失不得触发");
    assert_eq!(
        lock_state(&svc.state).last_mod_time,
        0,
        "文件缺失不得改写基线"
    );
}

#[test]
fn check_fires_once_for_newer_mtime_and_not_again_for_same() {
    let path = temp_file("check_newer");
    let t0 = write_until_newer(&path, 0);
    let (hits, cb) = counter();
    let svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    let m1 = write_until_newer(&path, t0);
    check(&svc.file_path, &svc.state, &svc.on_reload);
    assert_eq!(hits.load(Ordering::SeqCst), 1, "更新的 mtime 应触发一次");
    assert_eq!(lock_state(&svc.state).last_mod_time, m1, "触发后基线前移");
    // 同一 mtime 再查: 不得重复触发
    check(&svc.file_path, &svc.state, &svc.on_reload);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    std::fs::remove_file(&path).ok();
}

#[test]
fn check_equal_mtime_does_not_fire() {
    // 相等分支: 直接钉死 last_mod_time 构造 (两次真实写入无法可靠产生相等 mtime)
    let path = temp_file("check_equal");
    let m = write_until_newer(&path, 0);
    let svc =
        ConfigWatcherService::new(path.to_str().unwrap(), Some(|| panic!("相等 mtime 不应触发")));
    lock_state(&svc.state).last_mod_time = m;
    check(&svc.file_path, &svc.state, &svc.on_reload);
    assert_eq!(lock_state(&svc.state).last_mod_time, m);
    std::fs::remove_file(&path).ok();
}

#[test]
fn check_older_mtime_does_not_fire_and_keeps_baseline() {
    let path = temp_file("check_older");
    let m = write_until_newer(&path, 0);
    let svc =
        ConfigWatcherService::new(path.to_str().unwrap(), Some(|| panic!("回退 mtime 不应触发")));
    // 人为把基线抬到未来: current(m) > last(m+10000) 为假 → 严格大于方向钉子
    lock_state(&svc.state).last_mod_time = m + 10_000;
    check(&svc.file_path, &svc.state, &svc.on_reload);
    assert_eq!(
        lock_state(&svc.state).last_mod_time,
        m + 10_000,
        "未触发时不得改写基线"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn ignore_next_suppresses_exactly_one_event_and_absorbs_mtime() {
    let path = temp_file("ignore_next");
    let t0 = write_until_newer(&path, 0);
    let (hits, cb) = counter();
    let svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    let m1 = write_until_newer(&path, t0);
    svc.ignore_next();
    check(&svc.file_path, &svc.state, &svc.on_reload);
    assert_eq!(hits.load(Ordering::SeqCst), 0, "被 ignore 的那一轮不触发回调");
    // Java ignore 分支同时吸收 mtime: lastModTime = currentModTime
    assert_eq!(
        lock_state(&svc.state).last_mod_time,
        m1,
        "ignore 分支应吸收当前 mtime 作基线"
    );
    // 下一次修改恢复正常触发 (只压制一轮)
    let m2 = write_until_newer(&path, m1);
    check(&svc.file_path, &svc.state, &svc.on_reload);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(lock_state(&svc.state).last_mod_time, m2);
    std::fs::remove_file(&path).ok();
}

#[test]
fn check_without_callback_updates_mtime_without_panic() {
    // Java onReload == null 的路径: check() 内判空跳过
    let path = temp_file("null_cb");
    let t0 = write_until_newer(&path, 0);
    let svc = ConfigWatcherService::new(path.to_str().unwrap(), None::<fn()>);
    let m1 = write_until_newer(&path, t0);
    check(&svc.file_path, &svc.state, &svc.on_reload);
    assert_eq!(lock_state(&svc.state).last_mod_time, m1, "无回调仍应前移基线");
    std::fs::remove_file(&path).ok();
}

// ---- start/stop/Drop: 定时器线程行为 (真实线程) ----

#[test]
fn start_fires_reload_on_file_change() {
    let path = temp_file("start_fire");
    let t0 = write_until_newer(&path, 0);
    let (hits, cb) = counter();
    let mut svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    svc.start(25);
    let _m1 = write_until_newer(&path, t0);
    wait_until(10_000, || hits.load(Ordering::SeqCst) >= 1);
    // 同一次修改只触发一次: 静置多个完整周期再确认
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(hits.load(Ordering::SeqCst), 1, "同一修改不得重复触发");
    std::fs::remove_file(&path).ok();
}

#[test]
fn stop_halts_monitoring() {
    let path = temp_file("stop_halt");
    let t0 = write_until_newer(&path, 0);
    let (hits, cb) = counter();
    let mut svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    svc.start(25);
    let m1 = write_until_newer(&path, t0);
    wait_until(10_000, || hits.load(Ordering::SeqCst) >= 1);
    svc.stop();
    // 等待在途 check 收尾 (Java stop 后在途 actionPerformed 也会跑完) + 线程退出;
    // 60ms 保证 stop 前启动的在途 check 的 mtime 读取先于下一次写入
    std::thread::sleep(Duration::from_millis(60));
    let _m2 = write_until_newer(&path, m1);
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(hits.load(Ordering::SeqCst), 1, "stop 后不得再触发");
    std::fs::remove_file(&path).ok();
}

#[test]
fn start_after_stop_resumes_monitoring() {
    let path = temp_file("restart");
    let t0 = write_until_newer(&path, 0);
    let (hits, cb) = counter();
    let mut svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    svc.start(25);
    let m1 = write_until_newer(&path, t0);
    wait_until(10_000, || hits.load(Ordering::SeqCst) >= 1);
    svc.stop();
    std::thread::sleep(Duration::from_millis(60));
    svc.start(25); // 重启 = 全新 Timer (Java: 旧 stop + new Timer, initialDelay 重置)
    let _m2 = write_until_newer(&path, m1);
    wait_until(10_000, || hits.load(Ordering::SeqCst) >= 2);
    std::fs::remove_file(&path).ok();
}

#[test]
fn double_start_replaces_previous_timer() {
    // Java start(): 先 timer.stop() 再 new Timer → 旧定时器不再触发
    let path = temp_file("double_start");
    let t0 = write_until_newer(&path, 0);
    let (hits, cb) = counter();
    let mut svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    svc.start(25);
    svc.start(25); // 立即二次 start: 旧线程在首轮睡眠中即被停机
    let _m1 = write_until_newer(&path, t0);
    wait_until(10_000, || hits.load(Ordering::SeqCst) >= 1);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(hits.load(Ordering::SeqCst), 1, "旧定时器被替换, 不得双重触发");
    std::fs::remove_file(&path).ok();
}

#[test]
fn stop_without_start_is_noop() {
    let path = temp_file("stop_noop");
    write_until_newer(&path, 0);
    let mut svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(|| {}));
    svc.stop(); // timer == null 分支
    svc.stop(); // 幂等
    std::fs::remove_file(&path).ok();
}

#[test]
fn drop_stops_timer() {
    // Drop → stop (Rust 侧 RAII 收尾, bus.rs Subscription 先例)
    let path = temp_file("drop_stop");
    let t0 = write_until_newer(&path, 0);
    let (hits, cb) = counter();
    let mut svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    svc.start(20);
    drop(svc);
    std::thread::sleep(Duration::from_millis(60)); // 线程退出收尾
    let _m1 = write_until_newer(&path, t0);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(hits.load(Ordering::SeqCst), 0, "Drop 后不得再触发");
    std::fs::remove_file(&path).ok();
}

#[test]
fn panicking_callback_does_not_kill_timer() {
    // Swing 语义: 监听器异常被 EDT 捕获打印、Timer 继续运行;
    // catch_unwind + 回调锁中毒恢复复刻之 — 第二轮修改仍应到达回调
    let path = temp_file("panic_cb");
    let t0 = write_until_newer(&path, 0);
    let (hits, cb) = counter_with_panic();
    let mut svc = ConfigWatcherService::new(path.to_str().unwrap(), Some(cb));
    svc.start(25);
    let _m1 = write_until_newer(&path, t0);
    wait_until(10_000, || hits.load(Ordering::SeqCst) >= 1);
    let m1 = file_last_modified_millis(&path);
    let _m2 = write_until_newer(&path, m1);
    // 走到这里说明第一轮 panic 后定时器仍活着
    wait_until(10_000, || hits.load(Ordering::SeqCst) >= 2);
    std::fs::remove_file(&path).ok();
}

/// 计数后 panic 的回调 (panicking_callback_does_not_kill_timer 专用)
fn counter_with_panic() -> (Arc<AtomicU32>, impl FnMut() + Send + 'static) {
    let hits = Arc::new(AtomicU32::new(0));
    let h = Arc::clone(&hits);
    (
        hits,
        move || {
            h.fetch_add(1, Ordering::SeqCst);
            panic!("回调模拟异常");
        },
    )
}
