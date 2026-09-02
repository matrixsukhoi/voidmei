//! FM 域: 加载状态机六态 (status)、不可变加载结果句柄 (handle)、FM 数据路径
//! (data_paths)、加载器 (loader) 与管理器单一真相源 (manager, LIFETIMES ★1
//! 多写者裁决落地)、FM 数据装载 (data)、功率模型族 (piston_model/power_extractor/
//! power_curve)。
//! 波8 域内语义分层: 剥 fm_ 前缀 (域已表达)、fmdata→data (消解与 data_paths 的
//! 拼写分裂)、power_curve_helper→power_curve。
//! PORT: 类型顶层 re-export 镜像 Java `import prog.fm.FMStatus` 的扁平引用
//! (event/mod.rs 同款先例); Java `FMHandle.UNRESOLVED` 是关联常量, 经
//! `FMHandle::UNRESOLVED` 访问, 无需 (也无法) 顶层 re-export。
//! Java `FMDataPaths.xxx`/`FMLoader.xxx` 静态方法 → `data_paths::xxx()`/
//! `loader::xxx()` 自由函数访问。
//! `FmChangedBus` (审查 A 提示): 顶层 re-export 供 C 类订阅方
//! (Controller/AttitudeOverlay/DrawFrameSimpl/FMUnpackedDataOverlay 波次) 直接
//! `crate::fm::FmChangedBus` 引用, 免写全路径。

// ---- 加载栈 ----
pub mod data_paths;
pub mod handle;
pub mod loader;
pub mod manager;
pub mod status;

// ---- FM 数据 (原 fmdata, 波2 自顶层平铺吸收, 波8 更名) ----
pub mod data;

// ---- 功率模型族 ----
pub mod piston_model;
pub mod power_curve;
pub mod power_extractor;

pub use handle::FMHandle;
pub use manager::{FMManager, FmChangedBus};
pub use status::FMStatus;

// TestFMStore.java 的 FMManager 白盒用例移植 (集成测试文件, 仅测试构建加载);
// 依赖 manager 模块 (W3 批次并行落地)
#[cfg(test)]
mod store_tests;

// DATA_ROOT 测试串行守卫 (仅测试构建; mod.rs 回归纯声明, 波8 自本文件抽出)
#[cfg(test)]
pub(crate) mod test_support;
