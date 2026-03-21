package prog.util;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.util.Properties;

/**
 * Runtime helper for GPU compatibility mode settings.
 *
 * This class provides methods to:
 * 1. Save GPU compatibility settings to an external properties file
 * 2. Read the current saved setting
 * 3. Check if software rendering is currently active
 *
 * The settings are stored in a separate file (gpu_compat.properties) because:
 * - They must be read BEFORE AWT initialization (by Launcher.java)
 * - The main config (ui_layout.cfg) requires AWT to parse properly
 *
 * @see prog.Launcher for the bootstrap class that reads these settings
 */
public class GPUCompatibilityHelper {
    private static final String COMPAT_FILE = "gpu_compat.properties";
    private static final String KEY_ENABLED = "softwareRenderingEnabled";

    /**
     * Saves the GPU compatibility mode setting to the external properties file.
     * Called when the user toggles the setting in the UI.
     *
     * @param enabled true to enable software rendering on next startup
     */
    public static void saveSettings(boolean enabled) {
        Properties props = new Properties();
        props.setProperty(KEY_ENABLED, String.valueOf(enabled));
        try (FileOutputStream fos = new FileOutputStream(COMPAT_FILE)) {
            props.store(fos, "VoidMei GPU Compatibility Settings\n"
                    + "Set softwareRenderingEnabled=true to disable GPU acceleration.\n"
                    + "Requires application restart to take effect.");
        } catch (Exception e) {
            Logger.error("GPUCompat", "Failed to save settings: " + e.getMessage());
        }
    }

    /**
     * Reads the current saved setting from the properties file.
     * This reflects what will be applied on next startup.
     *
     * @return true if software rendering is enabled in the saved config
     */
    public static boolean isEnabled() {
        Properties props = new Properties();
        File file = new File(COMPAT_FILE);
        if (file.exists()) {
            try (FileInputStream fis = new FileInputStream(file)) {
                props.load(fis);
                return Boolean.parseBoolean(props.getProperty(KEY_ENABLED, "false"));
            } catch (Exception e) {
                Logger.warn("GPUCompat", "Failed to read settings: " + e.getMessage());
            }
        }
        return false;
    }

    /**
     * Checks if software rendering is currently active in this JVM session.
     * This detects whether the Launcher successfully applied the GPU compat settings.
     *
     * @return true if Java2D hardware acceleration is disabled
     */
    public static boolean isSoftwareRenderingActive() {
        // On Windows, check if Direct3D is disabled
        String d3d = System.getProperty("sun.java2d.d3d");
        if ("false".equals(d3d)) {
            return true;
        }

        // On Linux, check if OpenGL pipeline is disabled
        String opengl = System.getProperty("sun.java2d.opengl");
        if ("false".equals(opengl)) {
            return true;
        }

        // On macOS, check if Quartz acceleration is disabled
        String quartz = System.getProperty("apple.awt.graphics.UseQuartz");
        if ("false".equals(quartz)) {
            return true;
        }

        return false;
    }

    /**
     * Returns a human-readable description of the current rendering mode.
     * Useful for debugging and status display.
     *
     * @return Description of the current Java2D rendering pipeline
     */
    public static String getRenderingModeDescription() {
        if (isSoftwareRenderingActive()) {
            return "Software Rendering (GPU acceleration disabled)";
        }

        String os = System.getProperty("os.name", "").toLowerCase();
        if (os.contains("win")) {
            String d3d = System.getProperty("sun.java2d.d3d");
            if ("true".equals(d3d) || d3d == null) {
                return "Direct3D Hardware Acceleration";
            }
            return "GDI Software Rendering";
        } else if (os.contains("linux")) {
            String opengl = System.getProperty("sun.java2d.opengl");
            if ("true".equals(opengl)) {
                return "OpenGL Hardware Acceleration";
            }
            String xrender = System.getProperty("sun.java2d.xrender");
            if ("true".equals(xrender) || xrender == null) {
                return "XRender Hardware Acceleration";
            }
            return "X11 Software Rendering";
        } else if (os.contains("mac")) {
            return "Quartz Rendering";
        }
        return "Unknown Rendering Mode";
    }
}
