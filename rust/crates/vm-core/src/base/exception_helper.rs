//! ExceptionHelper 的 Rust 移植 (src/prog/util/ExceptionHelper.java)
//!
//! 统一异常处理工具类
//! 替代散落在代码中的空 catch 块和 printStackTrace()
//!
//! 设计原则：
//! - 保持与原有空 catch 块的行为一致（不改变控制流）
//! - 提供可选的日志记录功能，增加可观测性
//! - 线程安全，可在任意线程中使用
//!
//! PORT: Java 类仅含 static 方法 → Rust 模块自由函数 (string_helper 先例)。
//! PORT: §2.13 — Java Thread.interrupt()/InterruptedException 在 Rust 无直接
//! 对应, 统一映射为 AtomicBool 停机标志 (PORTING.md §2.13 裁决):
//! - "恢复线程中断状态" ≡ 提前返回后**不清除**标志 (上游可观察到停机请求);
//! - Java `long millis` → u64: 负值在 Java 触发未捕获的 IllegalArgumentException
//!   (编程错误), Rust 侧在类型层面直接排除该值域。
//!
//! PORT: 停机标志生命周期契约 (跨文件, 按 PORTING §6 只标注上报):
//! - Java 中断位是**每线程**状态 (S.interrupt() 只影响一个线程; 托盘重建
//!   ctr.stop() → new Controller() 会启动全新线程, 中断状态干净)。Rust 侧
//!   stop 标志必须按线程/按 Controller 世代分配 (Arc<AtomicBool> 随线程
//!   move), **严禁**接到进程级全局停机 token (LIFETIMES §7 草案的
//!   App.shutdown): 若新一代复用已置位的全局标志, 新 Service 的
//!   sleep_quietly 全部立即返回, 轮询循环退化为热自旋。
//! - 同一线程的 ignore(stop) 与 sleep_quietly(stop) 必须贯穿**同一个**
//!   AtomicBool, 才能复刻 Java 中断位对后续阻塞调用的可见性
//!   (HttpHelper 的 ignore 调用点 ↔ Service 轮询循环的 sleep 点)。

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// 停机标志轮询粒度: sleep_quietly 分片睡眠的片长, 即停机响应延迟上界。
/// PORT: Java Thread.sleep 由 OS 直接唤醒, 无轮询; 此值为 §2.13 标志位的
/// 配套机制 (10ms 对齐 UI 层轮询周期量级, 不参与行为断言)。
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// 记录异常但不中断流程（用于非关键操作）
/// 控制流不变，只是增加可观测性
///
/// PORT: Java 参数 `Exception e` → `&dyn std::error::Error` (对齐 logger.rs
/// error_with_throwable 的签名约定, 不加多余 `'static` bound — Display/Debug/
/// source 均不需要); `e.getMessage()` → `err.to_string()` (Display)。
/// Java getMessage() 可能返回 null 被拼成 "null", Rust Display 必有值 —
/// 此差异只影响日志文案, 不影响控制流语义。
pub fn log_and_continue(err: &dyn std::error::Error, context: &str) {
    crate::base::logger::warn_default(&format!("{}: {}", context, err));
    // PORT: 枚举 ordinal 序与 value 序严格同构 (TRACE(-1) < DEBUG(0) < INFO(1)
    // < WARN(2) < ERROR(3)), compareTo <= 0 即 TRACE/DEBUG 两级,
    // 用 value() 比较等价复刻
    if crate::base::logger::get_level().value() <= crate::base::logger::Level::Debug.value() {
        // PORT: printStackTrace() → stderr 打印错误链。Rust 错误无 Java 式
        // 抛出点栈回溯; 首行用 Debug repr (错误类型的 Debug 可复刻 Java
        // "类全名: 消息" 首行形态, 与 logger.rs error_with_throwable 的 `{:?}`
        // 约定统一, 同属 e2e A2 RE_EXC_FIRST 匹配域), 包装原因对齐
        // printStackTrace 的 "Caused by: ..." 行
        eprintln!("{err:?}");
        let mut src = err.source();
        while let Some(s) = src {
            eprintln!("Caused by: {s:?}");
            src = s.source();
        }
    }
}

/// 静默忽略 InterruptedException
/// 恢复线程中断状态，这是正确的处理方式
///
/// PORT: §2.13 — Rust 无线程中断位。Java `Thread.currentThread().interrupt()`
/// 的可观测效果是中断位对上游可见; 此处以置位停机标志复刻 (幂等: 若标志
/// 已因睡眠提前返回而置位, 再次 store(true) 无副作用)。
pub fn ignore(stop: &AtomicBool) {
    stop.store(true, Ordering::SeqCst);
}

/// 静默休眠（替代重复的 try-catch Thread.sleep）
/// 如果被中断，恢复中断状态但不抛出异常
///
/// PORT: §2.13 裁决 — "被中断"映射为停机标志置位: 分片睡眠并轮询标志,
/// 置位即提前返回且**不清除标志** (对应恢复中断状态, 不抛出任何异常);
/// 调用前标志已置位时立即返回, 对齐 Java 中断位已置位的 Thread.sleep
/// 立即抛 InterruptedException 的行为。
pub fn sleep_quietly(stop: &AtomicBool, millis: u64) {
    let deadline = Instant::now() + Duration::from_millis(millis);
    while !stop.load(Ordering::SeqCst) {
        let now = Instant::now();
        if now >= deadline {
            return;
        }
        // PORT: 分片: 剩余时间与轮询片取小, 保证不睡过 deadline
        let chunk = std::cmp::min(deadline - now, POLL_INTERVAL);
        thread::sleep(chunk);
    }
    // PORT: 标志置位 → 静默提前返回; 标志保持置位 = 恢复中断状态
}

/// 运行极性辅助: 睡眠至 deadline 或运行标志翻 false (提前返回)。
///
/// PORT: [`sleep_quietly`] 的标志是 **true=停** 语义; OtherService.is_run /
/// FlightLog.logon 这类 while 循环条件标志是 **true=运行**, 极性相反且
/// AtomicBool 无反视图可复用。Java 原义 `while(run) { sleepQuietly(N); ... }`
/// — sleep 被 interrupt 打断提前返回后重查循环条件退出, Rust 对位 = 睡眠中
/// 标志翻 false 即提前返回 (循环重查退出)。极性接反 (直接传给 sleep_quietly)
/// 会立即返回 → 运行期热自旋 (备案收口修复, voice_warning.rs 的私有
/// sleep_while_run 是同语义先行实现, 回收归其批次)。
/// 进入时标志已 false 则立即返回 (等价 Java 中断位已置位时 sleep 立抛)。
pub fn sleep_while_run(run: &AtomicBool, millis: u64) {
    let deadline = Instant::now() + Duration::from_millis(millis);
    while run.load(Ordering::SeqCst) {
        let now = Instant::now();
        if now >= deadline {
            return;
        }
        // 分片: 剩余时间与轮询片取小, 保证不睡过 deadline (sleep_quietly 同款)
        let chunk = std::cmp::min(deadline - now, POLL_INTERVAL);
        thread::sleep(chunk);
    }
    // 标志翻 false → 静默提前返回; 循环重查即退出
}

/// 静默休眠（严格保持原有空 catch 块的行为）
/// 中断时不恢复中断状态，与原代码行为完全一致
///
/// 注意：此方法仅用于需要严格行为一致性的场景
/// 新代码应优先使用 sleepQuietly()
///
/// PORT: §2.13 — Java 版被中断时静默忽略且**不恢复状态**地提前返回;
/// Rust 无中断异常源, catch 分支不可达, 退化为不可中断的纯 sleep
/// (需可中断语义处用 sleep_quietly(flag, millis))。
pub fn sleep_quietly_strict(millis: u64) {
    // 严格保持原行为：静默忽略，不恢复中断状态
    // PORT: (Rust 侧无 InterruptedException 可捕获, 直接睡满整个时长)
    thread::sleep(Duration::from_millis(millis));
}

/// 静默关闭资源（用于 finally 块中的资源清理）
///
/// PORT: Java AutoCloseable.close() → Rust **Drop 语义 + 显式函数双轨**:
/// - 轨一 (Drop 语义): 资源离开作用域即自动关闭 (File/TcpStream 等 Drop
///   即 close), 错误天然不上抛 — 等价于原 catch 的静默忽略;
/// - 轨二 (显式函数): 本函数, 供翻译 Java `finally { closeQuietly(x) }`
///   的调用点; 取得所有权并丢弃 = 触发 close。
///
/// `closeable != null` 判断 → Option 匹配; `catch (Exception)` 静默忽略
/// → Drop 无法返回错误, 语义由类型构造保证。
pub fn close_quietly<T>(closeable: Option<T>) {
    if let Some(closeable) = closeable {
        // 静默忽略关闭异常
        // PORT: (Drop 即 close; 关闭失败在 Drop 内部被吞, 不影响控制流)
        drop(closeable);
    }
}

#[cfg(test)]
mod tests;
