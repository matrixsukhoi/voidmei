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

    boolean drawHUDAttitude();

    double getAoAWarningRatio();

    double getAoABarWarningRatio();

    boolean enableFlapAngleBar();

    boolean drawHudMach();

    boolean isSpeedLabelDisabled();

    boolean isAltitudeLabelDisabled();

    boolean isSEPLabelDisabled();

    boolean isAoADisabled();
}
