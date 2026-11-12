package prog.util;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.Locale;
import java.util.concurrent.TimeUnit;

/**
 * Windows 系统代理解析（只读注册表，不启用 JVM 全局代理）。
 *
 * <p><b>为什么不直接用 {@code java.net.useSystemProxies=true}</b>：JDK 8 的裸 Socket
 * （SocksSocketImpl.connect）每个连接都会查 ProxySelector；当 Windows 系统代理是
 * {@code host:port} 简写形式时，JDK 原生实现把它当 SOCKS 代理返回——连 127.0.0.1:8111
 * 都被塞进代理隧道（默认 nonProxyHosts 在该分支不生效，ProxyOverride 又只做前缀比较，
 * "127.*" 匹配不到 "127.0.0.1"）。1.586"开代理后读不到 8111"即此因。
 *
 * <p>本类只把系统代理导出成 {@code host:port} 给 Launcher 设置 http(s).proxyHost——
 * 属性模式只影响 HttpURLConnection（GitHub 更新检查/FM 数据下载），裸 Socket 不查这两个属性。
 *
 * <p>数据源：HKCU\...\Internet Settings 的 ProxyEnable/ProxyServer（Clash/v2rayN 的
 * "系统代理"写在这里）。ProxyServer 支持两种形式：{@code host:port} 与
 * {@code http=h:p;https=h:p;socks=h:p}（优先 https，其次 http；只有 socks= 视为无代理，
 * http(s).proxyHost 走不了纯 SOCKS）。非 Windows 返回 null（靠环境变量/-D）。
 */
public final class SystemProxyResolver {

	private static final String REG_KEY = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings";
	/** reg 查询超时：异常环境不拖慢启动 */
	private static final long REG_TIMEOUT_MS = 2000;

	private SystemProxyResolver() {
	}

	/** 返回系统 HTTP 代理 "host:port"；未启用/不可用返回 null。启动早期调用一次，失败静默直连。 */
	public static String resolveWindowsProxy() {
		if (!System.getProperty("os.name", "").toLowerCase(Locale.ROOT).contains("win"))
			return null;
		try {
			Process p = new ProcessBuilder("reg", "query", REG_KEY).start();
			if (!p.waitFor(REG_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
				p.destroy();
				return null;
			}
			if (p.exitValue() != 0)
				return null;
			String enable = null, server = null;
			// reg 输出含本地化文本, 按字节串读避免系统编码问题; 需要的字段名/值均为 ASCII
			try (BufferedReader r = new BufferedReader(new InputStreamReader(p.getInputStream(), "ISO-8859-1"))) {
				String line;
				while ((line = r.readLine()) != null) {
					String[] f = line.trim().split("\\s+");
					if (f.length < 3)
						continue;
					if ("ProxyEnable".equalsIgnoreCase(f[0]))
						enable = f[f.length - 1];
					else if ("ProxyServer".equalsIgnoreCase(f[0]))
						server = f[f.length - 1];
				}
			}
			if (!isEnabled(enable) || server == null)
				return null;
			return parseServer(server);
		} catch (Exception e) {
			return null; // 读注册表失败 → 直连 (不影响 8111, GitHub 更新只是跳过本轮)
		}
	}

	/** ProxyEnable 为 0x1 才启用；缺失/0x0/无法解析 → false */
	private static boolean isEnabled(String v) {
		if (v == null)
			return false;
		try {
			return Integer.decode(v) == 1;
		} catch (NumberFormatException e) {
			return false;
		}
	}

	/**
	 * ProxyServer → "host:port"。纯 {@code host:port} 视为所有协议共用；
	 * 协议列表形式优先 https（GitHub 是 HTTPS），其次 http；只有 socks= 返回 null。
	 */
	static String parseServer(String server) {
		String s = server.trim();
		if (s.isEmpty())
			return null;
		if (s.indexOf('=') < 0)
			return validHostPort(s) ? s : null;
		String https = protoEntry(s, "https");
		if (https != null)
			return https;
		return protoEntry(s, "http");
	}

	private static String protoEntry(String list, String proto) {
		String prefix = proto + "=";
		for (String part : list.split(";")) {
			String p = part.trim();
			if (p.regionMatches(true, 0, prefix, 0, prefix.length())) {
				String hp = p.substring(prefix.length()).trim();
				return validHostPort(hp) ? hp : null;
			}
		}
		return null;
	}

	private static boolean validHostPort(String hp) {
		int c = hp.lastIndexOf(':');
		if (c <= 0 || c == hp.length() - 1)
			return false;
		try {
			int port = Integer.parseInt(hp.substring(c + 1).trim());
			return port > 0 && port < 65536;
		} catch (NumberFormatException e) {
			return false;
		}
	}
}
