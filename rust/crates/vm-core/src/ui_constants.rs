//! UIConstants 的 Rust 移植 (src/ui/util/UIConstants.java)
//!
//! UI 相关常量定义
//! 集中管理魔法数字，便于维护和一致性
//!
//! 设计原则：
//! - 所有数值与原代码中的硬编码值完全一致
//! - 常量命名清晰表达含义
//! - 分组组织便于查找
//!
//! PORT: Java final class + 私有构造 `private UIConstants() { // 禁止实例化 }`
//! → Rust 模块本身不可实例化, 天然满足"禁止实例化"语义。
//! Java `static final int` → `pub const i32`, `static final long` → `pub const i64`。

// ===== DPI 缩放基准 =====

/// 参考屏幕高度（1440p 为基准） - 用于计算缩放因子
pub const BASE_SCREEN_HEIGHT: i32 = 1440;

/// 基础字体大小（像素）
pub const BASE_FONT_SIZE: i32 = 16;

// ===== BaseOverlay 尺寸 =====

/// 宽度乘数（相对于字体大小）
pub const WIDTH_MULTIPLIER: i32 = 36;

/// 高度乘数（相对于字体大小）
pub const HEIGHT_MULTIPLIER: i32 = 72;

// ===== AttitudeOverlay =====

/// 最大攻角显示范围（度）
pub const MAX_AOA: i32 = 30;

/// 最大侧滑角显示范围（度）
pub const MAX_AOS: i32 = 15;

/// 姿态仪基础宽度（像素）
pub const ATTITUDE_BASE_WIDTH: i32 = 100;

/// 姿态仪基础高度（像素）
pub const ATTITUDE_BASE_HEIGHT: i32 = 200;

/// 姿态仪刷新间隔（毫秒）
pub const ATTITUDE_REFRESH_MS: i64 = 40;

// ===== EngineControlOverlay =====

/// 引擎仪表基础字体大小
pub const ENGINE_BASE_FONT_SIZE: i32 = 24;

/// 引擎仪表宽度乘数
pub const ENGINE_WIDTH_MULTIPLIER: i32 = 8;

/// 引擎仪表阴影宽度
pub const ENGINE_SHADE_WIDTH: i32 = 10;

/// 引擎仪表默认刷新间隔（毫秒）
pub const ENGINE_DEFAULT_REFRESH_MS: i64 = 100;

// ===== 时间常量 =====

/// 短延迟（UI 响应，毫秒）
pub const DELAY_SHORT_MS: i64 = 100;

/// 中等延迟（网络重试，毫秒）
pub const DELAY_MEDIUM_MS: i64 = 500;

/// 长延迟（初始化等待，毫秒）
pub const DELAY_LONG_MS: i64 = 1000;

// ===== 颜色相关 =====

/// 默认 Alpha 值（完全不透明）
pub const DEFAULT_ALPHA: i32 = 255;

/// 半透明 Alpha 值
pub const SEMI_TRANSPARENT_ALPHA: i32 = 128;

// ===== 边距和间距 =====

/// 小间距（像素）
pub const SPACING_SMALL: i32 = 5;

/// 中等间距（像素）
pub const SPACING_MEDIUM: i32 = 10;

/// 大间距（像素）
pub const SPACING_LARGE: i32 = 20;

#[cfg(test)]
mod tests {
    use super::*;

    /// 21 个常量逐项与 UIConstants.java 数值核对。
    /// 字面量带 i32/i64 后缀: 若类型映射写错 (int→i64 / long→i32) 编译期即报错,
    /// 对齐 Java 的 int/long 区分 (ATTITUDE_REFRESH_MS 等 5 个 long 常量)。
    #[test]
    fn constants_match_java_values() {
        // ===== DPI 缩放基准 =====
        assert_eq!(BASE_SCREEN_HEIGHT, 1440i32);
        assert_eq!(BASE_FONT_SIZE, 16i32);
        // ===== BaseOverlay 尺寸 =====
        assert_eq!(WIDTH_MULTIPLIER, 36i32);
        assert_eq!(HEIGHT_MULTIPLIER, 72i32);
        // ===== AttitudeOverlay =====
        assert_eq!(MAX_AOA, 30i32);
        assert_eq!(MAX_AOS, 15i32);
        assert_eq!(ATTITUDE_BASE_WIDTH, 100i32);
        assert_eq!(ATTITUDE_BASE_HEIGHT, 200i32);
        assert_eq!(ATTITUDE_REFRESH_MS, 40i64);
        // ===== EngineControlOverlay =====
        assert_eq!(ENGINE_BASE_FONT_SIZE, 24i32);
        assert_eq!(ENGINE_WIDTH_MULTIPLIER, 8i32);
        assert_eq!(ENGINE_SHADE_WIDTH, 10i32);
        assert_eq!(ENGINE_DEFAULT_REFRESH_MS, 100i64);
        // ===== 时间常量 =====
        assert_eq!(DELAY_SHORT_MS, 100i64);
        assert_eq!(DELAY_MEDIUM_MS, 500i64);
        assert_eq!(DELAY_LONG_MS, 1000i64);
        // ===== 颜色相关 =====
        assert_eq!(DEFAULT_ALPHA, 255i32);
        assert_eq!(SEMI_TRANSPARENT_ALPHA, 128i32);
        // ===== 边距和间距 =====
        assert_eq!(SPACING_SMALL, 5i32);
        assert_eq!(SPACING_MEDIUM, 10i32);
        assert_eq!(SPACING_LARGE, 20i32);
    }
}
