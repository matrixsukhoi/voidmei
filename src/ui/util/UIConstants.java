package ui.util;

/**
 * UI 相关常量定义
 * 集中管理魔法数字，便于维护和一致性
 *
 * 设计原则：
 * - 所有数值与原代码中的硬编码值完全一致
 * - 常量命名清晰表达含义
 * - 分组组织便于查找
 */
public final class UIConstants {

    private UIConstants() {
        // 禁止实例化
    }

    // ===== DPI 缩放基准 =====

    /** 参考屏幕高度（1440p 为基准） - 用于计算缩放因子 */
    public static final int BASE_SCREEN_HEIGHT = 1440;

    /** 基础字体大小（像素） */
    public static final int BASE_FONT_SIZE = 16;

    // ===== BaseOverlay 尺寸 =====

    /** 宽度乘数（相对于字体大小） */
    public static final int WIDTH_MULTIPLIER = 36;

    /** 高度乘数（相对于字体大小） */
    public static final int HEIGHT_MULTIPLIER = 72;

    // ===== AttitudeOverlay =====

    /** 最大攻角显示范围（度） */
    public static final int MAX_AOA = 30;

    /** 最大侧滑角显示范围（度） */
    public static final int MAX_AOS = 15;

    /** 姿态仪基础宽度（像素） */
    public static final int ATTITUDE_BASE_WIDTH = 100;

    /** 姿态仪基础高度（像素） */
    public static final int ATTITUDE_BASE_HEIGHT = 200;

    /** 姿态仪刷新间隔（毫秒） */
    public static final long ATTITUDE_REFRESH_MS = 40;

    // ===== EngineControlOverlay =====

    /** 引擎仪表基础字体大小 */
    public static final int ENGINE_BASE_FONT_SIZE = 24;

    /** 引擎仪表宽度乘数 */
    public static final int ENGINE_WIDTH_MULTIPLIER = 8;

    /** 引擎仪表阴影宽度 */
    public static final int ENGINE_SHADE_WIDTH = 10;

    /** 引擎仪表默认刷新间隔（毫秒） */
    public static final long ENGINE_DEFAULT_REFRESH_MS = 100;

    // ===== 时间常量 =====

    /** 短延迟（UI 响应，毫秒） */
    public static final long DELAY_SHORT_MS = 100;

    /** 中等延迟（网络重试，毫秒） */
    public static final long DELAY_MEDIUM_MS = 500;

    /** 长延迟（初始化等待，毫秒） */
    public static final long DELAY_LONG_MS = 1000;

    // ===== 颜色相关 =====

    /** 默认 Alpha 值（完全不透明） */
    public static final int DEFAULT_ALPHA = 255;

    /** 半透明 Alpha 值 */
    public static final int SEMI_TRANSPARENT_ALPHA = 128;

    // ===== 边距和间距 =====

    /** 小间距（像素） */
    public static final int SPACING_SMALL = 5;

    /** 中等间距（像素） */
    public static final int SPACING_MEDIUM = 10;

    /** 大间距（像素） */
    public static final int SPACING_LARGE = 20;
}
