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
    crate::logger::warn_default(&format!("{}: {}", context, err));
    // Java: Logger.getLevel().compareTo(Logger.Level.DEBUG) <= 0
    // PORT: 枚举 ordinal 序与 value 序严格同构 (TRACE(-1) < DEBUG(0) < INFO(1)
    // < WARN(2) < ERROR(3)), compareTo <= 0 即 TRACE/DEBUG 两级,
    // 用 value() 比较等价复刻
    if crate::logger::get_level().value() <= crate::logger::Level::Debug.value() {
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
mod tests {
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
        // Java: 中断位已置位的 Thread.sleep 立即抛 InterruptedException → 此处立即返回
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
        assert!(elapsed.as_millis() >= 35, "不应在标志置位前返回, 实际 {:?}", elapsed);
        // 响应延迟上界 = 一个轮询片 (10ms) + 调度误差, 放宽到 2s 防重载机器抖动
        assert!(elapsed.as_millis() < 2_000, "置位后应及时返回, 实际 {:?}", elapsed);
        // 标志保持置位 = 恢复中断状态 (上游可观察)
        assert!(stop.load(Ordering::SeqCst));
        setter.join().unwrap();
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
        // Java: closeable == null → 什么都不做
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
        assert!(closed.load(Ordering::SeqCst), "close_quietly 应触发 Drop (= close)");
    }

    // ---- log_and_continue: 接 crate::logger, 控制流不变 ----

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
        crate::logger::set_min_level(crate::logger::Level::Debug);
        log_and_continue(&ChainedErr(TestIoError("root".to_string())), "闸门上下文");
        // INFO 级 (默认): 闸门关 → 仅 WARN 行, stderr 无 printStackTrace 通道输出
        crate::logger::set_min_level(crate::logger::Level::Info);
        log_and_continue(&TestIoError("quiet".to_string()), "闸门上下文");
    }

    /// DEBUG 闸门分支钉子: 拉起子进程跑真实 stderr, 断言 (1) DEBUG 级输出
    /// Debug 首行形态 (RE_EXC_FIRST 可匹配域) + Caused by 链; (2) INFO 级
    /// (默认) 不输出该通道; (3) WARN 行两级都放行 (不受闸门影响)。
    #[test]
    fn log_and_continue_debug_gate() {
        let out = std::process::Command::new(std::env::current_exe().expect("定位测试二进制失败"))
            .args([
                "--exact",
                "exception_helper::tests::child_log_and_continue_debug_channel",
                "--nocapture",
            ])
            .output()
            .expect("拉起子进程失败");
        assert!(out.status.success(), "子进程测试失败: {out:?}");
        let stdout = String::from_utf8(out.stdout).expect("stdout 非 UTF-8");
        let stderr = String::from_utf8(out.stderr).expect("stderr 非 UTF-8");

        // DEBUG 级一次: 首行 + Caused by 链各一
        assert_eq!(stderr.matches("java.io.IOException: chained").count(), 1);
        assert_eq!(stderr.matches("Caused by: java.io.IOException: root").count(), 1);
        // INFO 级 (默认) 一次: 闸门关, printStackTrace 通道静默
        assert_eq!(stderr.matches("java.io.IOException: quiet").count(), 0);
        // WARN 行两次都在 (闸门只管 stderr 通道, WARN 由级别过滤放行)
        assert_eq!(stdout.matches("闸门上下文: chained").count(), 1);
        assert_eq!(stdout.matches("闸门上下文: quiet").count(), 1);
    }
}
