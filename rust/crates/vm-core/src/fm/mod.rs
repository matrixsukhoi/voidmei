//! prog.fm 数据侧 (A 类): FM 加载状态机六态、不可变加载结果句柄、FM 数据路径、
//! 加载器与 FM 管理器 (单一真相源, LIFETIMES ★1 多写者裁决落地于 fm_manager)。
//! PORT: 类型顶层 re-export 镜像 Java `import prog.fm.FMStatus` 的扁平引用
//! (event/mod.rs 同款先例); Java `FMHandle.UNRESOLVED` 是关联常量, 经
//! `FMHandle::UNRESOLVED` 访问, 无需 (也无法) 顶层 re-export。
//! Java `FMDataPaths.xxx`/`FMLoader.xxx` 静态方法 → `fm_data_paths::xxx()`/
//! `fm_loader::xxx()` 自由函数访问。
//! `FmChangedBus` (审查 A 提示): 顶层 re-export 供后续 C 类订阅方
//! (Controller/AttitudeOverlay/DrawFrameSimpl/FMUnpackedDataOverlay 波次) 直接
//! `crate::fm::FmChangedBus` 引用, 免写全路径。

pub mod fm_data_paths;
pub mod fm_loader;
pub mod fm_manager;
pub mod handle;
pub mod status;
// 重构波2 吸收: FM 数据 (fmdata) + 功率模型族原顶层平铺, 归入本域
pub mod fmdata;
pub mod fm_power_extractor;
pub mod piston_power_model;
pub mod power_curve_helper;

pub use fm_manager::{FMManager, FmChangedBus};
pub use handle::FMHandle;
pub use status::FMStatus;

// TestFMStore.java 的 FMManager 白盒用例移植 (集成测试文件, 仅测试构建加载);
// 依赖 fm_manager 模块 (W3 批次并行落地)
#[cfg(test)]
mod store_tests;

/// DATA_ROOT 相关测试的共享串行守卫 (serial_test 式, 禁新增依赖) ——
/// fm_data_paths.rs 测试注释自设纪律 ("后续触碰/依赖 DATA_ROOT 的测试加共享
/// static Mutex 串行守卫") 的锁本体落点。
/// 挂锁方: fm_loader::tests (铺 crate 相对根 + LOAD_COUNT)、fm_manager::tests
/// 三个用例 (铺 crate 相对根)、fm::store_tests (**set_data_root(绝对临时根)**
/// 全程持锁, Drop 逆序保证锁释放前 DATA_ROOT 已还原 "./data" —— 审查 A B1
/// 修复: 相对铺根随进程级 CWD 漂移被 config_manager sandbox 翻转击穿,
/// 绝对根直译回 Java TestFMStore 原文方案)。
/// fm_data_paths::java_main_sequence 已接入 test_guard 串行锁 (批五 QA 修复,
/// B1 残余项闭环——全库 DATA_ROOT 翻转源均已挂锁)。
/// 未来新增翻转源 (blkx/realtests 腿2 计划的临时根注入) 必须挂本锁 + 注入
/// **绝对**路径根, 而非再铺相对根。
/// ⚠ 仅进程内互斥 (审查 B W4 实证): 多个 cargo test 进程共享同一 CWD 的相对根
/// 夹具 (./data/testroot/otherroot) 时, 外部进程的 setup/cleanup 仍可互相删除
/// 文件造成假失败 —— 流水线纪律: 同一 workspace 禁止多 agent 并发跑 vm-core
/// 测试 (或夹具改每进程唯一的绝对临时根, store_tests create_temp_root 先例)。
#[cfg(test)]
pub(crate) mod test_guard {
    use std::sync::{Mutex, MutexGuard};

    static DATA_ROOT_LOCK: Mutex<()> = Mutex::new(());

    /// 获取 DATA_ROOT 测试串行锁。守卫不保护任何数据, 锁中毒无不变量可破,
    /// 中毒复取即可 (避免一次测试失败连锁炸掉后续守卫测试)。
    pub(crate) fn data_root() -> MutexGuard<'static, ()> {
        DATA_ROOT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
}
