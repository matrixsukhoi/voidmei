package prog.util;

import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.net.SocketAddress;
import java.nio.charset.StandardCharsets;

/**
 * 8111 遥测一次性 HTTP GET (issue #71: 消除每请求短连接的 TIME_WAIT 风暴)。
 *
 * <p>按真机实测行为设计 (2026-09): 游戏 8111 服务器一条连接只答一个请求,
 * 答完立即 FIN (延迟 ~0.01ms), 不支持也不需要 keep-alive 复用。因此本类
 * 每请求一条连接, 不做任何连接记忆/池化/重试——轮询循环本身就是重试。
 *
 * <p>TIME_WAIT 归零的关键: 读完 body 后等服务器的 FIN 再关闭 (被动方不产生
 * TIME_WAIT, TCP 的 240 秒等待期由先关的另一方承担——真机上是游戏进程)。
 *
 * <p>契约: 成功返回 body (UTF-8, 可能为空串); 失败/超时/协议不支持返回 null,
 * 不抛异常。无状态, 线程安全。
 */
public class OneShotHttp {
	/** 连接超时: 回环 RST 即刻返回, 此值只兜防火墙/半开类挂起 */
	private static final int CONNECT_TIMEOUT_MS = 500;
	/** 读超时: 20Hz 轮询下 40 周期宽限; 真机慢响应误翻转频繁则调大此值 */
	private static final int SO_TIMEOUT_MS = 2000;
	private static final int MAX_HEADER_BYTES = 8192;
	/** 响应头总字节预算: 拦截"每行都短于读超时的无限滴灌头"式挂死, 保住 get() 有界 */
	private static final int MAX_HEADER_TOTAL_BYTES = 64 * 1024;
	private static final int MAX_BODY_BYTES = 2 * 1024 * 1024;

	/**
	 * 发一次 GET 并读完整响应。每调用一条新连接, 用完即弃。
	 *
	 * @param path 请求路径 (如 "/state")
	 * @param dest 目标地址
	 * @return body 字符串; 失败/超时/不支持 (chunked) 返回 null。
	 *         不校验状态码——404 等 body 原样交解析层 (与旧行为一致, 解析层 valid 判空兜底)
	 */
	public static String get(String path, SocketAddress dest) {
		Socket s = null;
		try {
			s = new Socket();
			s.connect(dest, CONNECT_TIMEOUT_MS); // 拒绝 (游戏未开/断连) → 外层 catch → null → 调用方翻转端口
			s.setSoTimeout(SO_TIMEOUT_MS);
			s.setTcpNoDelay(true); // 请求小包, 消除 Nagle 不确定性
			writeRequest(s.getOutputStream(), path, dest);
			InputStream in = new BufferedInputStream(s.getInputStream(), 8192);

			String statusLine = readHeaderLine(in);
			if (statusLine == null || !statusLine.startsWith("HTTP/"))
				return null; // 连上但无有效响应 (含 accept 即关的断连场景)

			long contentLength = -1;
			boolean chunked = false;
			boolean headerComplete = false;
			int headerBytes = 0; // 头总预算: 单行上限拦不住"每行短于超时的无限滴灌"式挂死
			String h;
			while ((h = readHeaderLine(in)) != null) {
				if (h.isEmpty()) {
					headerComplete = true;
					break;
				}
				headerBytes += h.length() + 2;
				if (headerBytes > MAX_HEADER_TOTAL_BYTES)
					return null;
				String lower = h.toLowerCase();
				if (lower.startsWith("content-length:")) {
					contentLength = Long.parseLong(lower.substring("content-length:".length()).trim());
				} else if (lower.startsWith("transfer-encoding:") && lower.contains("chunked")) {
					chunked = true;
				}
			}
			if (!headerComplete || chunked)
				return null; // 头截断/超长或 chunked: 遥测服务器固定 Content-Length, 视为不支持

			byte[] body;
			if (contentLength >= 0) {
				if (contentLength > MAX_BODY_BYTES)
					return null;
				body = new byte[(int) contentLength];
				new DataInputStream(in).readFully(body); // 半包自愈: 内部循环读到满
				// 等 FIN: 服务器答完即关 (实测 ~0.01ms 到达), 我们随后关闭 = 被动方,
				// TIME_WAIT 不落本地。body 已完整, FIN 探测的任何结局 (超时/RST/EOF)
				// 都不影响结果——全部吞掉。
				try {
					in.read();
				} catch (IOException ignored) {
				}
			} else {
				body = readToEof(in); // 无 CL: 读到 EOF 即 body 边界
				if (body == null)
					return null; // 无 CL 超限: 失败, 不许截断 body 当成功
			}
			return new String(body, StandardCharsets.UTF_8);
		} catch (IOException | RuntimeException ex) {
			return null; // RuntimeException 含垃圾 Content-Length 的 NumberFormatException
		} finally {
			ExceptionHelper.closeQuietly(s);
		}
	}

	private static void writeRequest(OutputStream out, String path, SocketAddress dest) throws IOException {
		String host = (dest instanceof InetSocketAddress)
				? ((InetSocketAddress) dest).getHostString() : "127.0.0.1";
		int port = (dest instanceof InetSocketAddress)
				? ((InetSocketAddress) dest).getPort() : 8111;
		// 规范 CRLF; 一次一连接, 明确告知服务器不复用 (旧客户端发 LF 也被容忍, CRLF 是安全超集)
		StringBuilder req = new StringBuilder(160);
		req.append("GET ").append(path).append(" HTTP/1.1\r\n");
		req.append("Host: ").append(host).append(":").append(port).append("\r\n");
		req.append("Cache-Control: no-cache\r\n");
		req.append("Connection: close\r\n");
		req.append("\r\n");
		out.write(req.toString().getBytes(StandardCharsets.US_ASCII));
		out.flush();
	}

	/** 逐字节读一行头 (至 \n, 去 \r); 超过 8KB 或 EOF 返回 null */
	private static String readHeaderLine(InputStream in) throws IOException {
		StringBuilder sb = new StringBuilder(64);
		int c;
		while ((c = in.read()) != -1) {
			if (c == '\n') {
				int len = sb.length();
				if (len > 0 && sb.charAt(len - 1) == '\r')
					sb.setLength(len - 1);
				return sb.toString();
			}
			sb.append((char) c);
			if (sb.length() > MAX_HEADER_BYTES)
				return null;
		}
		return null; // EOF
	}

	/** 无 Content-Length 时读到 EOF 收 body (连接已被服务器关闭); 超限返回 null = 失败 */
	private static byte[] readToEof(InputStream in) throws IOException {
		ByteArrayOutputStream bos = new ByteArrayOutputStream(4096);
		byte[] buf = new byte[4096];
		int n;
		while ((n = in.read(buf)) != -1) {
			bos.write(buf, 0, n);
			if (bos.size() > MAX_BODY_BYTES)
				return null; // 超限: 失败——截断的半截 JSON 喂朴素解析层比失败更糟
		}
		return bos.toByteArray();
	}
}
