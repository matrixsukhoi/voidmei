//! ConfigWatcherService 的 Rust 移植 (src/prog/config/ConfigWatcherService.java)
//!
//! javax.swing.Timer → std 线程 + sleep 轮询 (任务判例: Timer 仅作调度器,
//! 保持 B 类落 vm-core, 不引 notify crate — 文件时间戳比较直接用 std::fs)。
//! 复用 exception_helper::sleep_quietly 的停机标志分片睡眠。
//!
//! Java 版回调在 EDT 上串行执行; Rust 版回调在本 watcher 线程执行,
//! 需要碰 UI 的调用方自行转 UI 线程 (bus.rs "订阅者自行转 UI 线程" 同一约定,
//! 跨线程交汇点核对表)。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Java Runnable 的回调替身。Mutex 提供跨线程可变调用权 (bus.rs 监听器先例:
/// FnMut 对齐 Java Runnable 可反复改写捕获状态的语义)。
type Callback = Arc<Mutex<Box<dyn FnMut() + Send>>>;

/// check/ignore_next 共享的监视状态。
/// Java 字段无任何锁, 靠 Swing Timer 回调固定在 EDT 上的事实串行化;
/// Rust 专用线程 + 调用方线程并存, 需 Mutex 保护。锁纪律 (LIFETIMES: 锁内
/// 回调外部代码是死锁高危): 本锁内只更新时间戳/标志, on_reload 回调一律在
/// 锁外执行 — 防 onReload 回头调 ignore_next() 重入死锁。
struct WatcherState {
    last_mod_time: i64,
    ignore_next: bool,
}

/// javax.swing.Timer 实例的 Rust 替身: 只剩停机标志 (线程本体已 detach)。
/// `timer.stop()` → 置位标志, 睡眠中的线程在一个分片 (≤10ms) 内退出;
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
    /// Java `Runnable onReload` 可为 null (check() 内判空) → Option
    on_reload: Option<Callback>,
    state: Arc<Mutex<WatcherState>>,
    /// Java `Timer timer` 可为 null (未 start/已 stop) → Option
    timer: Option<Timer>,
}

impl ConfigWatcherService {
    /// Java 构造器 `ConfigWatcherService(String filePath, Runnable onReload)`
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
                ignore_next: false,
            })),
            timer: None,
        }
    }

    /// Starts monitoring at the specified interval.
    ///
    /// `int intervalMs` → u64 — 负值在 Java 属编程错误, Rust 在类型层面
    /// 排除该值域 (exception_helper sleep_quietly 毫秒参数同一判例)。
    /// `interval_ms == 0` 退化为无节流热循环 (sleep(0) 立即返回 + 全速
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
                // javax.swing.Timer 默认 initialDelay == delay →
                // 首轮 check 在一个完整周期后 (先睡后查), 之后每周期重复 (setRepeats 默认 true)
                crate::base::exception_helper::sleep_quietly(&stop_flag, interval_ms);
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                // Swing Timer 的监听器异常被 EDT 捕获打印、Timer 继续运行;
                // catch_unwind 复刻"单次回调异常不杀死定时器" (§6 catch_unwind 契约的
                // 同族语义, 打印走 logger ERROR 通道)。
                // 默认 panic hook 会先向 stderr 打一条 "thread ... panicked",
                // 随后才走到下方 logger ERROR — 比 Java 单次输出多一条观测噪音,
                // 控制流等价; 不换全局 hook (进程级状态, 会误伤其他线程的 panic 报告)
                if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    check(&file_path, &state, &on_reload);
                })) {
                    // 波21: 本地 downcast 副本收敛至 panic_message_box (全库唯一)
                    let msg = crate::base::exception_helper::panic_message_box(panic);
                    crate::base::logger::error_default(&format!("ConfigWatcher 回调异常: {}", msg));
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
        // 锁中毒恢复 — Java 无锁中毒概念 (直接字段赋值), 中毒时取回内部
        // 值继续, 保证 ignore 请求不因既往 panic 而丢失
        let mut st = lock_state(&self.state);
        st.ignore_next = true;
    }
}

impl Drop for ConfigWatcherService {
    /// Java 无此语义 (未 stop 的 Timer 会一直挂在 EDT 事件队列上继续触发,
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
        // 锁内只改时间戳, 回调移到锁外 (顺序对齐 Java: 先 lastModTime 赋值
        // 后 onReload.run(); 锁外回调防 onReload→ignore_next() 重入死锁)
        drop(st);
        if let Some(callback) = on_reload {
            // 锁中毒恢复 — EDT 每轮照常回调监听器, 不因既往异常永久失效
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
                // u128→i64 as 转换。Rust as 饱和
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
mod tests;
