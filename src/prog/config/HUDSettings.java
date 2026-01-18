package prog.config;

/**
 * Interface for reading HUD-specific configurations.
 * Decouples the UI layer from underlying key names and parsing logic.
 */
public interface HUDSettings {
    String getNumFont();

    int getWindowX(int canvasWidth);

    int getWindowY(int canvasHeight);

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
