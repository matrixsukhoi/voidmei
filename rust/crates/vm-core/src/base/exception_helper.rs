//! 异常/中断处理助手 (原 Java ExceptionHelper 的现代化残留)。
//!
//! 波21 清场: log_and_continue/ignore/sleep_quietly_strict/close_quietly
//! 四个死函数 (波12~20 收编后调用方陆续消失) 已删; 存活面 = §2.13 的
//! 可中断睡眠 + catch_unwind panic 载荷收敛点。
//!
//! PORT: §2.13 — Java Thread.interrupt()/InterruptedException 在 Rust 无直接
//! 对应, 统一映射为 AtomicBool 停机标志:
//! - "恢复线程中断状态" ≡ 提前返回后**不清除**标志 (上游可观察到停机请求)。
//!
//! PORT: 停机标志生命周期契约 (跨文件, 按 PORTING §6 只标注上报):
//! - Java 中断位是**每线程**状态 (S.interrupt() 只影响一个线程; 托盘重建
//!   ctr.stop() → new Controller() 会启动全新线程, 中断状态干净)。Rust 侧
//!   stop 标志必须按线程/按 Controller 世代分配 (Arc<AtomicBool> 随线程
//!   move), **严禁**接到进程级全局停机 token (LIFETIMES §7 草案的
//!   App.shutdown): 若新一代复用已置位的全局标志, 新 Service 的
//!   sleep_quietly 全部立即返回, 轮询循环退化为热自旋。

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// 停机标志轮询粒度: sleep_quietly 分片睡眠的片长, 即停机响应延迟上界。
/// PORT: Java Thread.sleep 由 OS 直接唤醒, 无轮询; 此值为 §2.13 标志位的
/// 配套机制 (10ms 对齐 UI 层轮询周期量级, 不参与行为断言)。
const POLL_INTERVAL: Duration = Duration::from_millis(10);

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
/// PORT: [`sleep_quietly`] 的标志是 **true=停** 语义; VoiceWarning.doit 这类
/// while 循环条件标志是 **true=运行**, 极性相反且 AtomicBool 无反视图可复用。
/// Java 原义 `while(run) { sleepQuietly(N); ... }` — sleep 被 interrupt 打断
/// 提前返回后重查循环条件退出, Rust 对位 = 睡眠中标志翻 false 即提前返回
/// (循环重查退出)。极性接反 (直接传给 sleep_quietly) 会立即返回 → 运行期热自旋。
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

/// catch_unwind panic 载荷 → 文本, Java `e.getMessage()` 的对位物 (全库唯一)。
/// downcast 三分支: `&'static str` (panic!("boom")) / String (panic!("{}x", n)) /
/// 其他 (Java 异常恒有类型名, Rust 非字符串载荷无对应 → "null", 对齐 Java
/// getMessage()==null 时 `"..." + null` 的拼接结果)。
/// 此前 flight_data_bus/ui_state_bus/fm_loader/fm_manager 四处私有同构副本
/// (仅兜底文案不同: "unknown"/"null"/"unknown panic payload") 收敛于此。
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "null".to_string()
    }
}

/// [`panic_message`] 的 Box 形态: catch_unwind 的直接产物就是
/// `Box<dyn Any + Send>`, 免调用方手动解引用。
pub fn panic_message_box(payload: Box<dyn std::any::Any + Send>) -> String {
    panic_message(payload.as_ref())
}

#[cfg(test)]
mod tests;
