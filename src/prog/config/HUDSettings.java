package prog.config;

/**
 * Interface for reading HUD-specific configurations.
 * Decouples the UI layer from underlying key names and parsing logic.
 */
public interface HUDSettings extends OverlaySettings {
    String getNumFont();

    @Override
    int getWindowX(int canvasWidth);

    @Override
    int getWindowY(int canvasHeight);

    @Override
    void saveWindowPosition(double x, double y);

    int getCrosshairScale();

    String getCrosshairName();

    boolean isDisplayCrosshair();

    boolean useTextureCrosshair();

    boolean drawHUDText();

    boolean showAttitudeGauge();

    double getAoAWarningRatio();

    double getAoABarWarningRatio();

    boolean enableFlapAngleBar();

    boolean showSpeedBar();

    boolean drawHudMach();

    boolean isSpeedLabelDisabled();

    boolean isAltitudeLabelDisabled();

    boolean isSEPLabelDisabled();

    // 组件级独立显示开关 — 每个视觉元素可独立控制
    boolean showHUDSpeed();         // Row 0: 速度文字
    boolean showHUDAoA();           // Row 0: AoA bar + α文字
    boolean showHUDAltitude();      // Row 1: 高度文字
    boolean showHUDEnergy();        // Row 1: 能量读数
    boolean showHUDMechanization(); // Row 2: 襟翼/起落架文字 (旧，保留向后兼容)
    boolean showHUDFlaps();         // Row 2: 襟翼/可变翼
    boolean showHUDAirbrake();      // Row 2: 减速板 BRK
    boolean showHUDGear();          // Row 2: 起落架 GEA
    boolean showHUDSep();           // Row 3: 爬升率文字
    boolean showHUDGLoad();         // Row 4: G-force 文字
    boolean showHUDManeuverBar();   // Row 4: 机动条

    boolean isAttitudeIndicatorInertialMode();

    boolean isGPUCompatibilityMode();

    boolean alwaysShowRadarAltitude();
}
