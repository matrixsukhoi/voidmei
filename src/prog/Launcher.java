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
     * 网络代理引导：只给 HttpURLConnection 设置 http(s).proxyHost/Port（GitHub 更新检查与
     * FM 数据下载走这条路），代理作用域到此为止。
     *
     * <p><b>绝不设置 java.net.useSystemProxies=true</b>（1.586 线上回归根因）：该属性会让
     * JDK 8 的裸 Socket（SocksSocketImpl.connect）也查 ProxySelector；Windows 系统代理是
     * host:port 简写时，JDK 把系统代理当 SOCKS 返回，连 127.0.0.1:8111 都被塞进代理隧道
     * （系统代理分支不检查默认 nonProxyHosts；ProxyOverride 只做前缀匹配，"127.*" 匹配不到
     * "127.0.0.1"）。属性模式只被 HttpURLConnection 读取，裸 Socket 查的是 socksProxyHost，
     * 我们不设它 → 8111 永远直连（8111 的 Socket 另显式带 Proxy.NO_PROXY）。
     *
     * <p>代理来源优先级：显式 -D > HTTPS_PROXY/HTTP_PROXY 环境变量（仅终端启动可见）>
     * Windows 系统代理注册表（{@link prog.util.SystemProxyResolver}，Clash/v2rayN 写在这里）。
     */
    private static void applyProxySettings() {
        mapEnvProxy("HTTPS_PROXY", "https.proxyHost", "https.proxyPort");
        mapEnvProxy("https_proxy", "https.proxyHost", "https.proxyPort");
        mapEnvProxy("HTTP_PROXY", "http.proxyHost", "http.proxyPort");
        mapEnvProxy("http_proxy", "http.proxyHost", "http.proxyPort");

        // 环境变量没给出时, 再读系统代理; 已显式设置(-D/环境变量)的不覆盖
        String hp = prog.util.SystemProxyResolver.resolveWindowsProxy();
        if (hp != null) {
            applyProxyValue(hp, "Windows 系统代理", "https.proxyHost", "https.proxyPort");
            applyProxyValue(hp, "Windows 系统代理", "http.proxyHost", "http.proxyPort");
        }
    }

    private static void mapEnvProxy(String envKey, String hostProp, String portProp) {
        String v = System.getenv(envKey);
        if (v != null && !v.isEmpty())
            applyProxyValue(v, envKey, hostProp, portProp);
    }

    /**
     * 解析 "http://user@host:port" 形式的代理值写入标准属性。
     * 属性已存在（显式 -D 优先）或格式非法时静默跳过（直连）。
     */
    private static void applyProxyValue(String v, String source, String hostProp, String portProp) {
        if (System.getProperty(hostProp) != null)
            return;
        String s = v.trim();
        int slash = s.indexOf("://");
        if (slash >= 0)
            s = s.substring(slash + 3);
        int at = s.lastIndexOf('@'); // credentials unsupported by properties — strip
        if (at >= 0)
            s = s.substring(at + 1);
        int colon = s.lastIndexOf(':');
        if (colon <= 0 || colon == s.length() - 1)
            return;
        try {
            int port = Integer.parseInt(s.substring(colon + 1).trim());
            String host = s.substring(0, colon).trim();
            System.setProperty(hostProp, host);
            System.setProperty(portProp, String.valueOf(port));
            System.out.println("[Proxy] " + source + " -> " + host + ":" + port);
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
