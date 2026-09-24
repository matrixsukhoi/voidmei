package prog;

import java.io.File;
import java.io.FileInputStream;
import java.util.Properties;

/**
 * VoidMei application launcher / bootstrap class.
 *
 * This class sets GPU compatibility mode JVM properties BEFORE loading any AWT/Swing classes.
 *
 * CRITICAL: This class must NOT import any java.awt.* or javax.swing.* classes!
 * The JVM system properties for Java2D rendering (sun.java2d.d3d, sun.java2d.opengl, etc.)
 * must be set before any AWT class is loaded, or they will have no effect.
 *
 * The Application class imports AWT classes at the top of the file, so by the time
 * Application.main() runs, AWT is already initialized. This Launcher class runs first,
 * sets the properties, and then delegates to Application.main().
 *
 * @see prog.util.GPUCompatibilityHelper for runtime config persistence
 */
public class Launcher {
    private static final String COMPAT_FILE = "gpu_compat.properties";
    private static final String KEY_ENABLED = "softwareRenderingEnabled";

    public static void main(String[] args) {
        // Step 1: Apply GPU compatibility settings BEFORE any AWT classes load
        applyGPUCompatibilitySettings();

        // Step 2: Network proxy bootstrap — must run before the first HTTP connection
        // (DefaultProxySelector reads these properties when first used)
        applyProxySettings();

        // Step 3: Now delegate to the real Application entry point
        // This will load AWT classes, but the system properties are already set
        Application.main(args);
    }

    /**
     * Network proxy bootstrap (JVM ignores environment proxies — a long-standing Java pitfall:
     * HttpURLConnection does not read HTTPS_PROXY/HTTP_PROXY, and does not read OS-level
     * proxy settings unless told to):
     *
     * 1. java.net.useSystemProxies=true — follow OS-level proxy settings (the "system proxy"
     *    toggled by Clash/v2rayN etc. on Windows); zero-config auto for most users behind GFW.
     *    Local 8111 polling is unaffected: DefaultProxySelector's default nonProxyHosts
     *    excludes localhost/127.*.
     * 2. Env var mapping — parse HTTPS_PROXY/HTTP_PROXY into the standard properties
     *    https.proxyHost/Port. Only helps users launching from a terminal (GUI launch
     *    inherits no shell env); explicit -D flags always take precedence.
     */
    private static void applyProxySettings() {
        System.setProperty("java.net.useSystemProxies", "true");
        mapEnvProxy("HTTPS_PROXY", "https.proxyHost", "https.proxyPort");
        mapEnvProxy("https_proxy", "https.proxyHost", "https.proxyPort");
        mapEnvProxy("HTTP_PROXY", "http.proxyHost", "http.proxyPort");
        mapEnvProxy("http_proxy", "http.proxyHost", "http.proxyPort");
    }

    /**
     * Parse "http://user@host:port" style env value into host/port standard properties.
     * Skips silently (direct connection) on missing env, existing explicit property,
     * or malformed value.
     */
    private static void mapEnvProxy(String envKey, String hostProp, String portProp) {
        String v = System.getenv(envKey);
        if (v == null || v.isEmpty() || System.getProperty(hostProp) != null)
            return;
        String s = v;
        int slash = s.indexOf("://");
        if (slash >= 0)
            s = s.substring(slash + 3);
        int at = s.lastIndexOf('@'); // credentials unsupported by properties — strip
        if (at >= 0)
            s = s.substring(at + 1);
        int colon = s.indexOf(':');
        if (colon <= 0 || colon == s.length() - 1)
            return;
        try {
            int port = Integer.parseInt(s.substring(colon + 1).trim());
            System.setProperty(hostProp, s.substring(0, colon).trim());
            System.setProperty(portProp, String.valueOf(port));
            System.out.println("[Proxy] " + envKey + " -> " + s.substring(0, colon).trim() + ":" + port);
        } catch (NumberFormatException e) {
            // malformed value — keep direct connection
        }
    }

    /**
     * Reads GPU compatibility settings from the external properties file
     * and applies the appropriate JVM system properties.
     *
     * These properties MUST be set before any AWT class is loaded:
     * - sun.java2d.d3d=false (Windows: disable Direct3D)
     * - sun.java2d.noddraw=true (Windows: disable DirectDraw)
     * - sun.java2d.opengl=false (Linux: disable OpenGL pipeline)
     * - sun.java2d.xrender=false (Linux: disable XRender extension)
     * - apple.awt.graphics.UseQuartz=false (macOS: disable Quartz)
     * - sun.java2d.pmoffscreen=false (All: disable offscreen acceleration)
     */
    private static void applyGPUCompatibilitySettings() {
        Properties props = new Properties();
        File file = new File(COMPAT_FILE);

        if (file.exists()) {
            try (FileInputStream fis = new FileInputStream(file)) {
                props.load(fis);
            } catch (Exception e) {
                // Silent fallback to defaults - cannot use Logger here
                // as it might trigger AWT indirectly through some code paths
            }
        }

        boolean enabled = Boolean.parseBoolean(props.getProperty(KEY_ENABLED, "false"));

        if (enabled) {
            String os = System.getProperty("os.name", "").toLowerCase();

            if (os.contains("win")) {
                // Windows: Disable Direct3D and DirectDraw pipelines
                // Forces Java2D to use GDI software rendering
                System.setProperty("sun.java2d.d3d", "false");
                System.setProperty("sun.java2d.noddraw", "true");
            } else if (os.contains("linux")) {
                // Linux: Disable OpenGL and XRender pipelines
                // Falls back to X11 software rendering
                System.setProperty("sun.java2d.opengl", "false");
                System.setProperty("sun.java2d.xrender", "false");
            } else if (os.contains("mac")) {
                // macOS: Disable Quartz accelerated rendering
                System.setProperty("apple.awt.graphics.UseQuartz", "false");
            }

            // Universal: Disable offscreen image acceleration
            // This prevents GPU memory allocation for offscreen buffers
            System.setProperty("sun.java2d.pmoffscreen", "false");

            // Use System.out directly - Logger may trigger AWT initialization
            System.out.println("[GPU Compat] Software rendering mode enabled");
        }
    }
}
