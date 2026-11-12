package prog.config;

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
}
