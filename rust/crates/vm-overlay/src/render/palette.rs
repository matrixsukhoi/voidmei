//! 全局五色仓 — Java `Application.colorNum/colorLabel/colorUnit/colorWarning/
//! colorShadeShape` 静态字段 (Application.java:106-111) 的 Rust 对位物。
//!
//! **为什么是全局** (§2.9 禁裸全局的豁免备案): Java 原文即全局静态, 组件
//! (gauge/bars/rows/renderers ~185 引用点) 直接读; 机械保真优先, Rust 以
//! `OnceLock<RwLock<GlobalColors>>` 收敛为受控全局 — 写点仅两处:
//! win32 线程启动快照注入 + UiCommand::SetGlobalColors (WYSIWYG 色变),
//! 读点全在渲染路径 (win32 线程 50ms 节拍内)。
//!
//! 初始值 = Java 静态字段初始值 ([`GlobalColors::JAVA_DEFAULT`]) — cfg
//! (ui_layout.cfg:379-383 fontNum/fontLabel/fontUnit/fontWarn/fontShade) 经
//! loadFromConfig 覆盖为运行时真值; 未注入时 (单测/对拍工具) 行为与旧编译期
//! 常量逐字节一致, 现有测试零感知。
//!
//! 测试纪律: set 后必须还原 (RAII guard 或手动 reset_default), 并行测试共享
//! 本仓, 残留会互踩。

use std::sync::{OnceLock, RwLock};

use vm_core::config::configuration_service::GlobalColors;

static GLOBAL: OnceLock<RwLock<GlobalColors>> = OnceLock::new();

/// Application.aaEnable 的运行时值 (graph/text 两 hint 同开同关 — Java 单配置
/// 键 AAEnable 驱动, ConfigurationService.java:152-165)。cfg 缺省 false,
/// ui_layout.cfg:391 的 :value true 撑默认开机 AA; 用户可关 → Java 全部
/// overlay 变硬边。曾与五色同病: 生产渲染 6 处钉死 true, 审查轮 1-A 修复
static GLOBAL_AA: OnceLock<RwLock<bool>> = OnceLock::new();

/// 注入运行时色 (win32 线程启动快照 / WYSIWYG 色变命令)
pub fn set(c: GlobalColors) {
    *global().write().expect("global_colors 锁中毒") = c;
}

/// 注入 AA 开关 (启动快照 / UiCommand::SetAa; 历史默认 true 仅为多数渲染
/// 路径的 POC 期取值, 真值由启动快照决定)
pub fn set_aa(on: bool) {
    *global_aa().write().expect("global_aa 锁中毒") = on;
}

/// 还原 Java 静态初始值 (测试清理用; AA 还原 true 与旧渲染默认一致)
pub fn reset_default() {
    set(GlobalColors::JAVA_DEFAULT);
    set_aa(true);
}

/// 当前色快照 (Copy 直取, 渲染路径每帧调用 — 读锁 ~20ns 可忽略)
pub fn colors() -> GlobalColors {
    *global().read().expect("global_colors 锁中毒")
}

/// 当前 AA 开关 (每帧直读 — Java setRenderingHint 读运行时静态的同位)
pub fn aa() -> bool {
    *global_aa().read().expect("global_aa 锁中毒")
}

fn global() -> &'static RwLock<GlobalColors> {
    GLOBAL.get_or_init(|| RwLock::new(GlobalColors::JAVA_DEFAULT))
}

fn global_aa() -> &'static RwLock<bool> {
    GLOBAL_AA.get_or_init(|| RwLock::new(true))
}

#[cfg(test)]
mod tests;
