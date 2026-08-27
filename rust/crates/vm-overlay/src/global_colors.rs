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

use vm_core::configuration_service::GlobalColors;

static GLOBAL: OnceLock<RwLock<GlobalColors>> = OnceLock::new();

/// 注入运行时色 (win32 线程启动快照 / WYSIWYG 色变命令)
pub fn set(c: GlobalColors) {
    *global().write().expect("global_colors 锁中毒") = c;
}

/// 还原 Java 静态初始值 (测试清理用)
pub fn reset_default() {
    set(GlobalColors::JAVA_DEFAULT);
}

/// 当前色快照 (Copy 直取, 渲染路径每帧调用 — 读锁 ~20ns 可忽略)
pub fn colors() -> GlobalColors {
    *global().read().expect("global_colors 锁中毒")
}

fn global() -> &'static RwLock<GlobalColors> {
    GLOBAL.get_or_init(|| RwLock::new(GlobalColors::JAVA_DEFAULT))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// set/colors/reset 往返 + 默认值 = Java 静态初始值
    #[test]
    fn set_colors_and_reset_default() {
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
}
