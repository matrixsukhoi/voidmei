package prog.config;

import java.awt.Color;

/**
 * Interface for reading HUD-specific configurations.
 * Decouples the UI layer from underlying key names and parsing logic.
 *
 * MiniHUD 的显示内容/位置/格式全部由 miniHudLayout 文档描述；
 * 这里只保留全局外观（尺寸/字体/告警阈值）与布局文档存取。
 */
public interface HUDSettings extends OverlaySettings {
    String getNumFont();

    @Override
    int getWindowX(int canvasWidth);

    @Override
    int getWindowY(int canvasHeight);

    @Override
    void saveWindowPosition(double x, double y);

    /**
     * MiniHUD 整体缩放（历史配置键仍为 crosshairScale）。
     */
    int getMiniHUDScale();

    double getAoAWarningRatio();

    double getAoABarWarningRatio();

    boolean isGPUCompatibilityMode();

    // === MiniHUD 自定义布局 ===

    /** 总内容开关（迁移时复制旧 drawHUDtext）。 */
    boolean isMiniHudContentVisible();

    /** 布局文档紧凑串；空串表示使用出厂默认布局。 */
    String getMiniHudLayout();

    /** 保存布局文档（内部 setConfig + saveLayoutConfig）。 */
    void saveMiniHudLayout(String encoded);

    /** 数据字段目录（从 ui_layout.cfg 的 data 行汇总）。 */
    MiniHudFieldCatalog getMiniHudFieldCatalog();

    // === MiniHUD 右侧机场面板 ===

    /** 是否绘制指向机场的箭头。 */
    boolean isMiniHudAirfieldArrow();

    /** true=显示全部机场；false=只显示最近的一个。 */
    boolean isMiniHudAirfieldShowAll();

    /** 是否在距离后显示机场所在的地图网格（如 C4）。 */
    boolean isMiniHudAirfieldShowGrid();

    /** 友军机场显示颜色。 */
    Color getMiniHudAirfieldFriendlyColor();

    /** 敌军机场显示颜色。 */
    Color getMiniHudAirfieldEnemyColor();

    /** 距离小于该值的机场闪烁告警；0 表示关闭。 */
    double getMiniHudAirfieldWarnKm();
}
