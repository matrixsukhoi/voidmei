use std::sync::{Mutex, MutexGuard};

use super::*;

/// 全局仓测试串行锁 (先例: vm-data DATA_ROOT_TEST_LOCK) — set 与断言之间存在
/// 并发写窗口, 两测试并行时 reset_default() 会把对方 set 的值写回默认导致
/// 互踩 (num 实测被写回 JAVA_DEFAULT); 持锁覆盖 set→assert→reset 全程
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// set/colors/reset 往返 + 默认值 = Java 静态初始值
#[test]
fn set_colors_and_reset_default() {
    let _g = lock();
    assert_eq!(colors(), GlobalColors::JAVA_DEFAULT);
    let custom = GlobalColors {
        num: [255, 255, 255, 255],
        ..GlobalColors::JAVA_DEFAULT
    };
    set(custom);
    assert_eq!(colors(), custom);
    reset_default();
    assert_eq!(colors(), GlobalColors::JAVA_DEFAULT);
}

/// AA 仓往返 (set_aa/aa/reset)
#[test]
fn set_aa_and_reset() {
    let _g = lock();
    assert!(aa(), "默认 true (旧渲染路径取值)");
    set_aa(false);
    assert!(!aa());
    reset_default();
    assert!(aa());
}
