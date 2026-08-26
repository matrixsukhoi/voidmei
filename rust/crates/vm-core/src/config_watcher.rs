//! ConfigWatcherService 的 Rust 移植 (src/prog/config/ConfigWatcherService.java)
//!
//! PORT: javax.swing.Timer → std 线程 + sleep 轮询 (任务判例: Timer 仅作调度器,
//! 保持 B 类落 vm-core, 不引 notify crate — 文件时间戳比较直接用 std::fs)。
//! 复用 exception_helper::sleep_quietly 的停机标志分片睡眠 (§2.13 判例)。
//!
//! PORT: Java 版回调在 EDT 上串行执行; Rust 版回调在本 watcher 线程执行,
//! 需要碰 UI 的调用方自行转 UI 线程 (bus.rs "订阅者自行转 EDT" 同一约定,
//! LIFETIMES §5.3 跨线程交汇点核对表)。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Java Runnable 的回调替身。Mutex 提供跨线程可变调用权 (bus.rs 监听器先例:
/// FnMut 对齐 Java Runnable 可反复改写捕获状态的语义)。
type Callback = Arc<Mutex<Box<dyn FnMut() + Send>>>;

/// check/ignore_next 共享的监视状态。
/// PORT: Java 字段无任何锁, 靠 Swing Timer 回调固定在 EDT 上的事实串行化;
/// Rust 专用线程 + 调用方线程并存, 需 Mutex 保护。锁纪律 (LIFETIMES: 锁内
/// 回调外部代码是死锁高危): 本锁内只更新时间戳/标志, on_reload 回调一律在
/// 锁外执行 — 防 onReload 回头调 ignore_next() 重入死锁 (§2.8 不可重入)。
struct WatcherState {
    last_mod_time: i64,
    ignore_next: bool,
}

/// javax.swing.Timer 实例的 Rust 替身: 只剩停机标志 (线程本体已 detach)。
/// PORT: `timer.stop()` → 置位标志, 睡眠中的线程在一个分片 (≤10ms) 内退出;
/// 在途的一轮 check 会跑完 (含回调)。Java 从 EDT 调 stop() 时 EDT 单线程,
/// 既无在途 actionPerformed、已排队事件也被取消 (零额外触发); Rust 无 EDT,
/// 对齐的是从非 EDT 线程调 stop() 的语义 (在途一轮跑完) — 故 stop 后窄窗内
/// 可能补发最后一轮回调, 已声明偏差; 不加回调前复查 (半措施既达不成 EDT
/// 语义又违背本契约)。不 join — Java stop() 立即返回, 不等待在途回调。
struct Timer {
    stop: Arc<AtomicBool>,
}

impl Timer {
    /// javax.swing.Timer.stop(): 取消后续触发
    fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/**
 * Service to monitor a configuration file for changes.
 * Decouples file system watching from the UI layer.
 */
pub struct ConfigWatcherService {
    file_path: String,
    /// PORT: Java `Runnable onReload` 可为 null (check() 内判空) → Option
    on_reload: Option<Callback>,
    state: Arc<Mutex<WatcherState>>,
    /// PORT: Java `Timer timer` 可为 null (未 start/已 stop) → Option
    timer: Option<Timer>,
}

impl ConfigWatcherService {
    /// PORT: Java 构造器 `ConfigWatcherService(String filePath, Runnable onReload)`
    /// 参数 `int intervalMs` 在 start; 此处 filePath: &str (String 的借用形态)。
    pub fn new<F: FnMut() + Send + 'static>(
        file_path: &str,
        on_reload: Option<F>,
    ) -> ConfigWatcherService {
        let on_reload = on_reload
            .map(|f| Arc::new(Mutex::new(Box::new(f) as Box<dyn FnMut() + Send>)) as Callback);
        let file = Path::new(file_path); // File file = new File(filePath);
        let last_mod_time = if file.exists() {
            file_last_modified_millis(file)
        } else {
            0
        };
        ConfigWatcherService {
            file_path: file_path.to_string(),
            on_reload,
            state: Arc::new(Mutex::new(WatcherState {
                last_mod_time,
                ignore_next: false, // Java 显式初始化 ignoreNext = false
            })),
            timer: None,
        }
    }

    /// Starts monitoring at the specified interval.
    ///
    /// PORT: `int intervalMs` → u64 — 负值在 Java 属编程错误, Rust 在类型层面
    /// 排除该值域 (exception_helper sleep_quietly 毫秒参数同一判例)。
    /// PORT: `interval_ms == 0` 退化为无节流热循环 (sleep(0) 立即返回 + 全速
    /// mtime 轮询); Java Timer(0) 在 EDT 上同样连续触发, 属调用方病态用法,
    /// 保真不复健。
    pub fn start(&mut self, interval_ms: u64) {
        if let Some(timer) = self.timer.take() {
            timer.stop();
        }
        let stop = Arc::new(AtomicBool::new(false));
        // 线程闭包只持 Arc 克隆/String 副本, self 可自由移动或销毁 (Drop→stop)
        let state = Arc::clone(&self.state);
        let on_reload = self.on_reload.clone();
        let file_path = self.file_path.clone();
        let stop_flag = Arc::clone(&stop);
        std::thread::spawn(move || {
            loop {
                // PORT: javax.swing.Timer 默认 initialDelay == delay →
                // 首轮 check 在一个完整周期后 (先睡后查), 之后每周期重复 (setRepeats 默认 true)
                crate::exception_helper::sleep_quietly(&stop_flag, interval_ms);
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                // PORT: Swing Timer 的监听器异常被 EDT 捕获打印、Timer 继续运行;
                // catch_unwind 复刻"单次回调异常不杀死定时器" (§6 catch_unwind 契约的
                // 同族语义, 打印走 logger ERROR 通道)。
                // PORT: 默认 panic hook 会先向 stderr 打一条 "thread ... panicked",
                // 随后才走到下方 logger ERROR — 比 Java 单次输出多一条观测噪音,
                // 控制流等价; 不换全局 hook (进程级状态, 会误伤其他线程的 panic 报告)
                if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    check(&file_path, &state, &on_reload);
                })) {
                    let msg = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "未知 panic 载荷".to_string());
                    crate::logger::error_default(&format!("ConfigWatcher 回调异常: {}", msg));
                }
            }
        });
        self.timer = Some(Timer { stop });
    }

    /// Stops monitoring.
    pub fn stop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.stop();
        }
    }

    /**
     * Signals the Service to ignore the very next file modification event.
     * Useful when the application itself writes to the file.
     */
    pub fn ignore_next(&self) {
        // PORT: 锁中毒恢复 — Java 无锁中毒概念 (直接字段赋值), 中毒时取回内部
        // 值继续, 保证 ignore 请求不因既往 panic 而丢失
        let mut st = lock_state(&self.state);
        st.ignore_next = true;
    }
}

impl Drop for ConfigWatcherService {
    /// PORT: Java 无此语义 (未 stop 的 Timer 会一直挂在 EDT 事件队列上继续触发,
    /// 属泄漏源); Rust 侧 Drop 即停 — bus.rs Subscription RAII 先例, 显式优于原状。
    fn drop(&mut self) {
        self.stop();
    }
}

/// Java `private void check()`: 定时器回调体 (watcher 线程执行)。
/// 私有方法对齐 — 线程闭包与同模块测试直接调用。
fn check(file_path: &str, state: &Mutex<WatcherState>, on_reload: &Option<Callback>) {
    let path = Path::new(file_path); // File file = new File(filePath);
    if !path.exists() {
        return;
    }

    let current_mod_time = file_last_modified_millis(path);

    let mut st = lock_state(state);
    if st.ignore_next {
        st.ignore_next = false;
        st.last_mod_time = current_mod_time;
        return; // 锁随 return 释放
    }

    if current_mod_time > st.last_mod_time {
        st.last_mod_time = current_mod_time;
        // PORT: 锁内只改时间戳, 回调移到锁外 (顺序对齐 Java: 先 lastModTime 赋值
        // 后 onReload.run(); 锁外回调防 onReload→ignore_next() 重入死锁)
        drop(st);
        if let Some(callback) = on_reload {
            // PORT: 锁中毒恢复 — EDT 每轮照常回调监听器, 不因既往异常永久失效
            let mut f = callback.lock().unwrap_or_else(|e| e.into_inner());
            f();
        }
    }
}

/// 状态锁获取 (中毒恢复, 见 check 内注释)
fn lock_state(state: &Mutex<WatcherState>) -> std::sync::MutexGuard<'_, WatcherState> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

/// java.io.File.lastModified() 语义复刻: 返回 epoch 毫秒;
/// 文件不存在/读取失败/平台不支持时返回 0 (Java 对应路径返回 0L)。
fn file_last_modified_millis(path: &Path) -> i64 {
    match std::fs::metadata(path) {
        Ok(meta) => match meta.modified() {
            Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
                // PORT: u128→i64 as 转换 (§3 SystemTime→millis 库映射)。Rust as 饱和
                // vs Java 截断仅在毫秒域溢出 (年份 ~2.9 亿) 时有差异, 现实文件时间不可能触界
                Ok(d) => d.as_millis() as i64,
                // 时钟早于 1970: lastModified 不产生负值语义, 归 0 对齐失败路径
                Err(_) => 0,
            },
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
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
}
