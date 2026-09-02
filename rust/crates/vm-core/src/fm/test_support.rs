//! DATA_ROOT 相关测试的共享串行守卫 (serial_test 式, 禁新增依赖) ——
//! fm 数据路径测试注释自设纪律 ("后续触碰/依赖 DATA_ROOT 的测试加共享
//! static Mutex 串行守卫") 的锁本体落点 (波8 自 fm/mod.rs 抽出, mod.rs 回归纯声明)。
//! 挂锁方: loader::tests (铺 crate 相对根 + LOAD_COUNT)、manager::tests
//! 三个用例 (铺 crate 相对根)、fm::store_tests (**set_data_root(绝对临时根)**
//! 全程持锁, Drop 逆序保证锁释放前 DATA_ROOT 已还原 "./data" —— 审查 A B1
//! 修复: 相对铺根随进程级 CWD 漂移被 config_manager sandbox 翻转击穿,
//! 绝对根直译回 Java TestFMStore 原文方案)。
//! data_paths::java_main_sequence 已接入本守卫串行锁 (批五 QA 修复,
//! B1 残余项闭环——全库 DATA_ROOT 翻转源均已挂锁)。
//! 未来新增翻转源 (data/realtests 腿2 计划的临时根注入) 必须挂本锁 + 注入
//! **绝对**路径根, 而非再铺相对根。
//! ⚠ 仅进程内互斥 (审查 B W4 实证): 多个 cargo test 进程共享同一 CWD 的相对根
//! 夹具 (./data/testroot/otherroot) 时, 外部进程的 setup/cleanup 仍可互相删除
//! 文件造成假失败 —— 流水线纪律: 同一 workspace 禁止多 agent 并发跑 vm-core
//! 测试 (或夹具改每进程唯一的绝对临时根, store_tests create_temp_root 先例)。

use std::sync::{Mutex, MutexGuard};

static DATA_ROOT_LOCK: Mutex<()> = Mutex::new(());

/// 获取 DATA_ROOT 测试串行锁。守卫不保护任何数据, 锁中毒无不变量可破,
/// 中毒复取即可 (避免一次测试失败连锁炸掉后续守卫测试)。
pub(crate) fn data_root() -> MutexGuard<'static, ()> {
    DATA_ROOT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
