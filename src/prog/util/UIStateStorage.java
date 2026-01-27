package prog.util;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.Properties;

/**
 * Utility to save UI state (like last active tab) in the user's home directory.
 * Follows XDG config spec on Linux and AppData on Windows.
 */
public class UIStateStorage {
    private static final String APP_NAME = "voidmei";
    private static final String STATE_FILE = "ui_state.properties";
    private static final String KEY_LAST_TAB = "lastActiveMainFormTab";

    private static String getConfigDir() {
        String os = System.getProperty("os.name").toLowerCase();
        String userHome = System.getProperty("user.home");

        if (os.contains("win")) {
            String appData = System.getenv("APPDATA");
            if (appData != null) {
                return appData + File.separator + APP_NAME;
            }
            return userHome + File.separator + "." + APP_NAME;
        } else if (os.contains("linux")) {
            String xdgConfig = System.getenv("XDG_CONFIG_HOME");
            if (xdgConfig != null && !xdgConfig.isEmpty()) {
                return xdgConfig + File.separator + APP_NAME;
            }
            return userHome + File.separator + ".config" + File.separator + APP_NAME;
        } else {
            // macOS or others
            return userHome + File.separator + "." + APP_NAME;
        }
    }

    private static File getConfigFile() {
        File dir = new File(getConfigDir());
        if (!dir.exists()) {
            dir.mkdirs();
        }
        return new File(dir, STATE_FILE);
    }

    public static int loadLastTab() {
        Properties props = new Properties();
        File file = getConfigFile();
        if (file.exists()) {
            try (FileInputStream in = new FileInputStream(file)) {
                props.load(in);
                String val = props.getProperty(KEY_LAST_TAB);
                if (val != null) {
                    return Integer.parseInt(val);
                }
            } catch (Exception e) {
                Logger.info("UIStateStorage", "Failed to load UI state: " + e.getMessage());
            }
        }
        return 0; // Default
    }

    public static void saveLastTab(int index) {
        Properties props = new Properties();
        File file = getConfigFile();
        Logger.info("UIStateStorage", "saveLastTab, config file: " + file.getAbsolutePath());

        // Load existing to avoid overwriting other potential future keys
        if (file.exists()) {
            try (FileInputStream in = new FileInputStream(file)) {
                props.load(in);
            } catch (IOException e) {
                // Ignore load error
            }
        }

        props.setProperty(KEY_LAST_TAB, String.valueOf(index));

        try (FileOutputStream out = new FileOutputStream(file)) {
            props.store(out, "UI State for VoidMei");
        } catch (IOException e) {
            Logger.info("UIStateStorage", "Failed to save UI state: " + e.getMessage());
        }
    }

    private static final String KEY_WINDOW_X = "mainFormX";
    private static final String KEY_WINDOW_Y = "mainFormY";

    // ... existing code ...

    public static java.awt.Point loadWindowPosition() {
        Properties props = new Properties();
        File file = getConfigFile();
        if (file.exists()) {
            try (FileInputStream in = new FileInputStream(file)) {
                props.load(in);
                String xVal = props.getProperty(KEY_WINDOW_X);
                String yVal = props.getProperty(KEY_WINDOW_Y);
                if (xVal != null && yVal != null) {
                    return new java.awt.Point(Integer.parseInt(xVal), Integer.parseInt(yVal));
                }
            } catch (Exception e) {
                Logger.info("UIStateStorage", "Failed to load window position: " + e.getMessage());
            }
        }
        return null;
    }

    public static void saveWindowPosition(int x, int y) {
        Properties props = new Properties();
        File file = getConfigFile();

        // Load existing to preserve other keys
        if (file.exists()) {
            try (FileInputStream in = new FileInputStream(file)) {
                props.load(in);
            } catch (IOException e) {
                // Ignore
            }
        }

        props.setProperty(KEY_WINDOW_X, String.valueOf(x));
        props.setProperty(KEY_WINDOW_Y, String.valueOf(y));

        try (FileOutputStream out = new FileOutputStream(file)) {
            props.store(out, "UI State for VoidMei");
        } catch (IOException e) {
            Logger.info("UIStateStorage", "Failed to save window position: " + e.getMessage());
        }
    }
}
