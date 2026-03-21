package prog.util;

import java.awt.GraphicsConfiguration;
import java.awt.GraphicsDevice;
import java.awt.GraphicsEnvironment;
import java.awt.Toolkit;
import java.awt.geom.AffineTransform;

/**
 * DPI scaling utility for high-DPI display support.
 *
 * On Windows with 200% scaling, Toolkit.getScreenSize() returns physical pixels,
 * but Swing operates in logical pixels. This helper detects the actual DPI scale
 * and provides logical screen dimensions.
 *
 * Usage:
 * - Call DPIHelper.init() early in application startup (before creating windows)
 * - Use getLogicalScreenWidth/Height() for window positioning
 * - Use scale(baseValue) to convert design-time pixels to scaled pixels
 *
 * Example:
 *   DPIHelper.init();
 *   int windowWidth = DPIHelper.scale(800);  // 800 at 100%, 1600 at 200%
 *   int screenW = DPIHelper.getLogicalScreenWidth();  // Logical, not physical
 */
public final class DPIHelper {

    // DPI scale factors (1.0 = 100%, 2.0 = 200%)
    private static double scaleX = 1.0;
    private static double scaleY = 1.0;

    // Logical screen dimensions (what Swing sees)
    private static int logicalScreenWidth;
    private static int logicalScreenHeight;

    // Physical screen dimensions (actual monitor pixels)
    private static int physicalScreenWidth;
    private static int physicalScreenHeight;

    private static boolean initialized = false;

    private DPIHelper() {
        // Utility class - no instantiation
    }

    /**
     * Initialize DPI detection. Call once at application startup,
     * after AWT is loaded but before creating windows.
     *
     * This method is idempotent - safe to call multiple times.
     */
    public static synchronized void init() {
        if (initialized) {
            return;
        }

        try {
            // Get physical screen size from Toolkit
            java.awt.Dimension screenSize = Toolkit.getDefaultToolkit().getScreenSize();
            physicalScreenWidth = screenSize.width;
            physicalScreenHeight = screenSize.height;

            // Detect DPI scale using GraphicsConfiguration
            GraphicsEnvironment ge = GraphicsEnvironment.getLocalGraphicsEnvironment();
            GraphicsDevice gd = ge.getDefaultScreenDevice();
            GraphicsConfiguration gc = gd.getDefaultConfiguration();

            AffineTransform transform = gc.getDefaultTransform();
            scaleX = transform.getScaleX();
            scaleY = transform.getScaleY();

            // Calculate logical screen dimensions
            // Physical pixels / scale factor = logical pixels
            if (scaleX > 0) {
                logicalScreenWidth = (int) Math.round(physicalScreenWidth / scaleX);
            } else {
                logicalScreenWidth = physicalScreenWidth;
            }

            if (scaleY > 0) {
                logicalScreenHeight = (int) Math.round(physicalScreenHeight / scaleY);
            } else {
                logicalScreenHeight = physicalScreenHeight;
            }

            initialized = true;

            // Log DPI detection results
            Logger.info("DPIHelper", String.format(
                "DPI Detection: Scale=%.2fx%.2f, Physical=%dx%d, Logical=%dx%d",
                scaleX, scaleY,
                physicalScreenWidth, physicalScreenHeight,
                logicalScreenWidth, logicalScreenHeight
            ));

        } catch (Exception e) {
            // Fallback to 100% scaling if detection fails
            Logger.warn("DPIHelper", "DPI detection failed, using defaults: " + e.getMessage());
            scaleX = 1.0;
            scaleY = 1.0;

            java.awt.Dimension screenSize = Toolkit.getDefaultToolkit().getScreenSize();
            physicalScreenWidth = screenSize.width;
            physicalScreenHeight = screenSize.height;
            logicalScreenWidth = physicalScreenWidth;
            logicalScreenHeight = physicalScreenHeight;

            initialized = true;
        }
    }

    /**
     * Returns the horizontal DPI scale factor.
     * 1.0 = 100%, 1.5 = 150%, 2.0 = 200%
     */
    public static double getScaleX() {
        ensureInitialized();
        return scaleX;
    }

    /**
     * Returns the vertical DPI scale factor.
     * Usually equals getScaleX() on most systems.
     */
    public static double getScaleY() {
        ensureInitialized();
        return scaleY;
    }

    /**
     * Returns the primary DPI scale factor (horizontal).
     * Convenience method - use this for general scaling.
     */
    public static double getScale() {
        ensureInitialized();
        return scaleX;
    }

    /**
     * Returns the logical screen width in pixels.
     * This is what Swing sees and should be used for window positioning.
     */
    public static int getLogicalScreenWidth() {
        ensureInitialized();
        return logicalScreenWidth;
    }

    /**
     * Returns the logical screen height in pixels.
     * This is what Swing sees and should be used for window positioning.
     */
    public static int getLogicalScreenHeight() {
        ensureInitialized();
        return logicalScreenHeight;
    }

    /**
     * Returns the physical screen width in actual monitor pixels.
     */
    public static int getPhysicalScreenWidth() {
        ensureInitialized();
        return physicalScreenWidth;
    }

    /**
     * Returns the physical screen height in actual monitor pixels.
     */
    public static int getPhysicalScreenHeight() {
        ensureInitialized();
        return physicalScreenHeight;
    }

    /**
     * Scales a base value by the DPI scale factor.
     * Use this to convert design-time (100% DPI) values to scaled values.
     *
     * @param baseValue The value at 100% DPI
     * @return The scaled value for current DPI
     */
    public static int scale(int baseValue) {
        ensureInitialized();
        return (int) Math.round(baseValue * scaleX);
    }

    /**
     * Scales a base value by the DPI scale factor (double version).
     *
     * @param baseValue The value at 100% DPI
     * @return The scaled value for current DPI
     */
    public static double scale(double baseValue) {
        ensureInitialized();
        return baseValue * scaleX;
    }

    /**
     * Inverse scale - converts a scaled value back to base value.
     * Use this when reading positions from screen coordinates.
     *
     * @param scaledValue The value at current DPI
     * @return The value at 100% DPI
     */
    public static int unscale(int scaledValue) {
        ensureInitialized();
        if (scaleX == 0) return scaledValue;
        return (int) Math.round(scaledValue / scaleX);
    }

    /**
     * Returns true if the system is using high-DPI scaling (> 100%).
     */
    public static boolean isHighDPI() {
        ensureInitialized();
        return scaleX > 1.01 || scaleY > 1.01;
    }

    /**
     * Returns true if DPIHelper has been initialized.
     */
    public static boolean isInitialized() {
        return initialized;
    }

    private static void ensureInitialized() {
        if (!initialized) {
            init();
        }
    }
}
