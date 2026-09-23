package prog.util;

import prog.Application;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.net.HttpURLConnection;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.net.SocketAddress;
import java.net.URL;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.Future;

/**
 * 8111 API 请求门面 (issue #71 重构)。
 *
 * <p>轮询路径 (state/indicators/map_obj/map_info) 走 {@link OneShotHttp}:
 * 每请求一条连接, 读完等服务器 FIN 再关闭 (被动方, 本地 TIME_WAIT 恒为零)。
 * state 走线程池、indicators 在调用者线程——两请求保持并行 (延迟优化);
 * map_obj (500ms) 与 map_info (复活瞬间) 均在 Service 线程串行调用。
 *
 * <p>契约 (Service 状态机依赖, 勿改): 任何网络失败 ⇒ strState/strIndic/strMapObj/strMapInfo
 * 置空串 (非 null 不抛出), Service 据此走"等待连接"分支并翻转 8111/9222 端口。
 */
public class HttpHelper {
	// ---- 共享结果字段 (Service 直接读取; 失败恒为空串) ----
	public String strState = nstring;
	public String strIndic = nstring;
	public String strMapObj = nstring;
	public String strMapInfo = nstring;

	private static final String nstring = "";

	// ---- fmCmd (fmTesting 门控的调试路径, 极低频, 一次性写命令) ----

	/**
	 * 一轮主轮询: state (线程池并行) + indicators (当前线程)。
	 * Future.get() 有界——OneShotHttp 全部 IO 带超时, 修掉旧版 completableFuture
	 * 永不重置导致的串行失效与池任务异常时的挂死窗口。
	 */
	public void getReqResult(SocketAddress req_addr) {
		try {
			Future<String> fst = Application.threadPool.submit(() -> OneShotHttp.get("/state", req_addr));
			String rIndic = OneShotHttp.get("/indicators", req_addr);
			String rState;
			try {
				rState = fst.get();
			} catch (ExecutionException e) {
				rState = null;
			}
			// 一损俱损: 任一失败两字段都置空。Service 端以 strState/strIndic 均
			// 非空为有效判据 (AND), 故与旧版"各自成败"在此消费点行为等价
			if (rState == null || rIndic == null) {
				strState = nstring;
				strIndic = nstring;
			} else {
				strState = rState;
				strIndic = rIndic;
			}
		} catch (InterruptedException e) {
			// 中断异常，恢复中断状态
			ExceptionHelper.ignore(e);
			strState = nstring;
			strIndic = nstring;
		}
	}

	public void getReqMapObjResult(SocketAddress req_addr) {
		String r = OneShotHttp.get("/map_obj.json", req_addr);
		strMapObj = (r != null) ? r : nstring;
	}

	public void getReqMapInfoResult(SocketAddress req_addr) {
		String r = OneShotHttp.get("/map_info.json", req_addr);
		strMapInfo = (r != null) ? r : nstring;
	}

	/**
	 * 获取当前 8111 端口的实时机型信息
	 *
	 * @return 机型名称，如果获取失败或无效则返回 null
	 */
	public String getLiveAircraftType() {
		try {
			SocketAddress dest = new InetSocketAddress("127.0.0.1", 8111);
			String indicatorsJson = OneShotHttp.get("/indicators", dest);
			if (indicatorsJson == null)
				return null; // 失败前置判断, 不靠下游 NPE 兜底

			parser.Indicators indicatorsParser = new parser.Indicators();
			indicatorsParser.init();
			indicatorsParser.update(indicatorsJson);

			if (indicatorsParser.valid != null && indicatorsParser.valid.equals("true") && indicatorsParser.type != null
					&& !indicatorsParser.type.isEmpty()
					&& !indicatorsParser.type.equals("No Cockpit")) {
				return indicatorsParser.type.toLowerCase().trim();
			}
		} catch (Exception e) {
			// 忽略错误，返回 null
		}
		return null;
	}

	// ---- 以下为低频/调试路径, 与轮询无关 ----

	/** fmCmd 一次性写命令: 发完读到服务器关闭即止 (调试路径, 保持旧请求串格式) */
	private static void sendFmCmd(String requestLine, SocketAddress dest) throws IOException {
		Socket socket = new Socket();
		try {
			socket.connect(dest, 500);
			BufferedWriter w = new BufferedWriter(new OutputStreamWriter(socket.getOutputStream()));
			w.write(requestLine);
			w.flush();
			socket.getInputStream().read(); // 读至服务器关闭 (调试服务器答完即关)
		} finally {
			ExceptionHelper.closeQuietly(socket);
		}
	}

	public void fmCmdSetAlt(int alt, SocketAddress dest) throws IOException {
		String req = "GET /editor/fm_commands?cmd=setAlt&value=" + alt + " HTTP/1.1\n" + "Host: " + "127.0.0.1"
				+ "\n" + "Cache-Control:no-cache\n" + Application.httpHeader + "\n";
		sendFmCmd(req, dest);
	}

	public void fmCmdSetSpd(double spd, SocketAddress dest) throws IOException {
		String req = "GET /editor/fm_commands?cmd=setVelocity&value=" + String.format("%.0f", spd)
				+ " HTTP/1.1\n" + "Host: " + "127.0.0.1" + "\n" + "Cache-Control:no-cache\n"
				+ Application.httpHeader + "\n";
		sendFmCmd(req, dest);
	}

	/** 更新检查等外部 URL (低频, 走 HttpURLConnection) */
	public String sendGetURL(String url) throws Exception {

		URL obj = new URL(url);
		HttpURLConnection con = (HttpURLConnection) obj.openConnection();

		con.setRequestMethod("GET");

		int responseCode = con.getResponseCode();

		BufferedReader in = new BufferedReader(
				new InputStreamReader(con.getInputStream()));
		String inputLine;
		StringBuffer response = new StringBuffer();

		while ((inputLine = in.readLine()) != null) {
			response.append(inputLine);
		}
		in.close();

		String result = response.toString();
		if (url.contains("api.github.com")) {
			prog.util.Logger.info("Update", "Latest version info fetched successfully (HTTP " + responseCode + ")");
		}
		return result;
	}
}
