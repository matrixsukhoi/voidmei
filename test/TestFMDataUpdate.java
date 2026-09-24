import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

import com.sun.net.httpserver.HttpServer;

import prog.fm.FMDataManifest;
import prog.fm.FMDataPaths;
import prog.fm.FMDataUpdater;
import prog.fm.FMLoader;
import prog.fm.FMManager;
import prog.i18n.Lang;
import prog.util.ExceptionHelper;
import prog.util.FileUtils;

/**
 * FM 数据在线更新白盒测试 —— manifest 解析 / 版本比较 / sha256 / unzip 防护 /
 * 目录替换 / 负缓存生效链 / 本地 HttpServer 整链。
 *
 * 纯文件与本地回环网络, 不碰外网与真机 data/。
 * 运行方式: python script/build.py test fmupdate
 */
public class TestFMDataUpdate {

	private static int passed = 0;
	private static int failed = 0;

	public static void main(String[] args) throws Exception {
		System.out.println("=== FMDataUpdate 测试 ===\n");

		// runUpdateCycle 内的文案格式化需要非 null 的 Lang 字段 (CI 白盒无 i18n 初始化)
		Lang.fmDataUpdateToast = "FM 数据已更新到 %s, 已自动生效";
		Lang.fmDataCheckUpToDate = "FM 数据已是最新 (%s)";
		Lang.fmDataCheckFailed = "FM 数据更新失败";
		Lang.fmDataDownloadProgress = "正在下载 FM 数据更新 %d%%";
		Lang.fmDataInstallStage = "正在安装 FM 数据更新...";
		Lang.fmDataActionBackground = "转后台下载";
		Lang.fmDataActionDisable = "禁止自动更新";
		Lang.fmDataAutoDisabled = "已关闭自动更新";
		// 防外部环境影响: 更新周期里 off 属性会整体跳过
		System.clearProperty("voidmei.fmdata.update");

		try {
			testManifestParse();
			testCompareVersion();
			testSha256();
			testUnzip();
			testReplaceDataDir();
			testEffectiveChain();
			testDateFormat();
			testFullCycle();
		} finally {
			FMDataPaths.setDataRoot("./data");
		}

		System.out.println("\n=== 测试结果 ===");
		System.out.println("通过: " + passed);
		System.out.println("失败: " + failed);

		// 显式退出: 整链用例可能启动 EDT (进度通知), 非 daemon 线程会挂住 JVM
		System.exit(failed > 0 ? 1 : 0);
	}

	private static void check(String name, boolean cond) {
		if (cond) {
			passed++;
			System.out.println("[PASS] " + name);
		} else {
			failed++;
			System.out.println("[FAIL] " + name);
		}
	}

	private static File tempDir(String prefix) throws IOException {
		return Files.createTempDirectory(prefix).toFile();
	}

	private static void writeFile(File f, String content) throws IOException {
		f.getParentFile().mkdirs();
		Files.write(f.toPath(), content.getBytes(StandardCharsets.UTF_8));
	}

	private static String readFile(File f) throws IOException {
		return new String(Files.readAllBytes(f.toPath()), StandardCharsets.UTF_8).trim();
	}

	// ---- 1. manifest 解析 ----

	private static void testManifestParse() {
		System.out.println("-- manifest 解析 --");
		// 照 build.py json.dumps(indent=2) 的实际产物格式
		String normal = "{\n  \"wt_version\": \"2.59.0.28\",\n  \"prev_wt_version\": \"2.59.0.11\",\n"
				+ "  \"date\": \"20260923\",\n  \"blkx_count\": 3800,\n  \"file_count\": 3801,\n"
				+ "  \"total_bytes\": 135102085,\n  \"diff\": {\n    \"added\": 0\n  },\n"
				+ "  \"zip\": \"VoidMei_data_2.59.0.28.zip\",\n"
				+ "  \"sha256\": \"f8d256d2088c3ee8f1b9e77ace601dbfe1dd8bf2747e90961d9b5e655cd39211\"\n}";
		FMDataManifest m = FMDataManifest.parse(normal);
		check("正常解析 wt_version", m != null && "2.59.0.28".equals(m.wtVersion));
		check("正常解析 zip", m != null && "VoidMei_data_2.59.0.28.zip".equals(m.zipName));
		check("正常解析 sha256", m != null && m.sha256.startsWith("f8d256d2"));
		check("正常解析 date", m != null && "20260923".equals(m.date));

		check("缺 wt_version 返 null",
				FMDataManifest.parse(normal.replace("wt_version", "wt_ver")) == null);
		check("缺 zip 返 null",
				FMDataManifest.parse(normal.replace("\"zip\"", "\"zipx\"")) == null);
		check("缺 sha256 返 null",
				FMDataManifest.parse(normal.replace("sha256", "sha2xx")) == null);

		// prev_wt_version 首次为 null: 非必需字段值非字符串不影响
		String nullPrev = normal.replace("\"2.59.0.11\"", "null");
		check("prev_wt_version null 不影响解析", FMDataManifest.parse(nullPrev) != null);

		check("空串返 null", FMDataManifest.parse("") == null);
		check("null 返 null", FMDataManifest.parse(null) == null);

		// 键乱序 (zip 在最前)
		String reordered = "{\n  \"zip\": \"z.zip\",\n  \"sha256\": \"ab\",\n  \"wt_version\": \"1.0.0.0\"\n}";
		FMDataManifest m2 = FMDataManifest.parse(reordered);
		check("键乱序解析", m2 != null && "z.zip".equals(m2.zipName) && m2.date == null);
	}

	// ---- 2. 版本比较 ----

	private static void testCompareVersion() {
		System.out.println("-- 版本比较 --");
		// 字符串序会判错的反例: '28' < '7'
		check("28 > 7", FMDataManifest.compareVersion("2.59.0.28", "2.59.0.7") > 0);
		check("7 < 28", FMDataManifest.compareVersion("2.59.0.7", "2.59.0.28") < 0);
		check("相等", FMDataManifest.compareVersion("2.59.0.28", "2.59.0.28") == 0);
		check("主版本优先", FMDataManifest.compareVersion("2.60.0.0", "2.59.9.99") > 0);
		// 段数不齐: 缺段补 0 → 2.59 = 2.59.0.0 < 2.59.0.1
		check("段数不齐补零", FMDataManifest.compareVersion("2.59", "2.59.0.1") < 0);
		// 非数字段退化字符串比较, 不抛
		boolean threw = false;
		try {
			FMDataManifest.compareVersion("2.59.0.x", "2.59.0.28");
		} catch (Exception e) {
			threw = true;
		}
		check("非数字段不抛", !threw);
		check("null 视为最小", FMDataManifest.compareVersion(null, "1.0.0.0") < 0);
	}

	// ---- 3. sha256 ----

	private static void testSha256() throws IOException {
		System.out.println("-- sha256 --");
		File f = tempDir("vmtest_sha");
		File target = new File(f, "blob.bin");
		writeFile(target, "voidmei fmdata sha256 test vector");
		// 期望值由 python hashlib 预生成
		check("固定内容 sha256 对拍",
				"4a93390ebbf195575ae42470a8bfc6d46f53eaf7bb6384a928135b1ee4944aa0"
						.equals(FileUtils.sha256Hex(target)));
		check("不存在文件返 null", FileUtils.sha256Hex(new File(f, "nope")) == null);
	}

	// ---- 4. unzip 防护 ----

	private static void testUnzip() throws IOException {
		System.out.println("-- unzip --");
		File tmp = tempDir("vmtest_unzip");
		File zip = new File(tmp, "normal.zip");
		try (ZipOutputStream zos = new ZipOutputStream(new FileOutputStream(zip))) {
			zos.putNextEntry(new ZipEntry("data/aces/version"));
			zos.write("1.0.0.0".getBytes(StandardCharsets.UTF_8));
			zos.closeEntry();
			zos.putNextEntry(new ZipEntry("data/aces/gamedata/flightmodels/test.blkx"));
			zos.write("dummy".getBytes(StandardCharsets.UTF_8));
			zos.closeEntry();
		}
		File out = new File(tmp, "out");
		FileUtils.unzip(zip, out);
		check("保留顶层 data/ 结构", new File(out, "data/aces/version").isFile());
		check("文件内容正确",
				"1.0.0.0".equals(readFile(new File(out, "data/aces/version"))));

		// zip-slip: 条目路径逃逸目标目录必须被拒
		File evil = new File(tmp, "evil.zip");
		try (ZipOutputStream zos = new ZipOutputStream(new FileOutputStream(evil))) {
			zos.putNextEntry(new ZipEntry("../evil.txt"));
			zos.write("x".getBytes(StandardCharsets.UTF_8));
			zos.closeEntry();
		}
		boolean threw = false;
		try {
			FileUtils.unzip(evil, out);
		} catch (IOException e) {
			threw = true;
		}
		check("zip-slip 条目被拒", threw);
		check("zip-slip 未逃逸", !new File(tmp, "evil.txt").exists());

		// 单条目超限 (实测最大 blkx ≈0.27MB, 上限 32MB): 写真实超限内容
		File bomb = new File(tmp, "bomb.zip");
		try (ZipOutputStream zos = new ZipOutputStream(new FileOutputStream(bomb))) {
			zos.putNextEntry(new ZipEntry("data/big.bin"));
			byte[] chunk = new byte[65536];
			for (long w = 0; w <= FileUtils.UNZIP_MAX_ENTRY_BYTES; w += chunk.length)
				zos.write(chunk, 0, (int) Math.min(chunk.length, FileUtils.UNZIP_MAX_ENTRY_BYTES + 1 - w));
			zos.closeEntry();
		}
		threw = false;
		try {
			FileUtils.unzip(bomb, new File(tmp, "bomb_out"));
		} catch (IOException e) {
			threw = true;
		}
		check("超大条目被拒", threw);
	}

	// ---- 5. 目录替换 ----

	private static void testReplaceDataDir() throws Exception {
		System.out.println("-- 目录替换 --");
		File tmp = tempDir("vmtest_replace");
		File data = new File(tmp, "data");
		File newData = new File(tmp, "staging/unpacked/data");
		File backup = new File(tmp, "data_old");
		writeFile(new File(data, "aces/version"), "1.0.0.0");
		writeFile(new File(newData, "aces/version"), "2.0.0.0");

		check("替换成功", FMDataUpdater.replaceDataDir(data, newData, backup));
		check("新内容就位", "2.0.0.0".equals(readFile(new File(data, "aces/version"))));
		check("备份已删", !backup.exists());
		check("旧目录已移交", !newData.exists());

		// 占用场景: 持开旧 data 内文件, rename 可能失败 (Windows 目录语义)
		// 宽断言: 重试后成功 或 失败且旧数据原封不动 —— 两种都是可接受结果
		File data2 = new File(tmp, "d2");
		File newData2 = new File(tmp, "staging2/unpacked/data");
		writeFile(new File(data2, "aces/version"), "1.0.0.0");
		writeFile(new File(newData2, "aces/version"), "3.0.0.0");
		try (InputStream hold = new FileInputStream(new File(data2, "aces/version"))) {
			hold.read(); // 确保句柄真实打开
			boolean r = FMDataUpdater.replaceDataDir(data2, newData2, new File(tmp, "d2_old"));
			if (r) {
				check("占用时重试后成功或安全失败", "3.0.0.0".equals(readFile(new File(data2, "aces/version"))));
			} else {
				check("占用时重试后成功或安全失败",
						"1.0.0.0".equals(readFile(new File(data2, "aces/version"))));
			}
		}
	}

	// ---- 6. 生效链 (负缓存 + 强制重载) ----

	private static void testEffectiveChain() throws Exception {
		System.out.println("-- 生效链 --");
		File tmp = tempDir("vmtest_effect");
		FMDataPaths.setDataRoot(tmp.toString()); // 空 data → 一切 identify 落 MISSING
		FMManager mgr = FMManager.getInstance();
		mgr.reset();
		FMLoader.resetLoadCount();

		mgr.identify("nonexistent_plane");
		check("首次 identify 触发加载", awaitLoadCount(1, 5000));
		check("落 MISSING", mgr.current().isMissingLike());

		// 负缓存: 同名 identify 不再触盘
		mgr.identify("other_plane");
		check("切走触发加载", awaitLoadCount(2, 5000));
		mgr.identify("nonexistent_plane");
		check("负缓存拦截同名重载", awaitLoadCount(2, 5000) && FMLoader.getLoadCount() == 2);

		// dataUpdated(): 清负缓存 + 强制重载当前目标 (load3; 若负缓存未清,
		// 重载结果仍会发布, 此断言由 loadCount 增加唯一区分)
		mgr.dataUpdated();
		check("dataUpdated 强制重载", awaitLoadCount(3, 5000));

		// 重载后新数据仍无此机型 → MISSING 结果重新进负缓存, 切走再切回不再触盘
		mgr.identify("other_plane");
		awaitLoadCount(4, 5000);
		mgr.identify("nonexistent_plane");
		ExceptionHelper.sleepQuietly(500);
		check("仍缺失时重新入负缓存不触盘", FMLoader.getLoadCount() == 4);

		mgr.reset();
	}

	private static boolean awaitLoadCount(long expected, long timeoutMs) {
		long end = System.currentTimeMillis() + timeoutMs;
		while (System.currentTimeMillis() < end) {
			if (FMLoader.getLoadCount() >= expected)
				return true;
			ExceptionHelper.sleepQuietly(20);
		}
		return FMLoader.getLoadCount() >= expected;
	}

	// ---- 7. 日期格式化 ----

	private static void testDateFormat() {
		System.out.println("-- 日期格式化 --");
		check("yyyymmdd 转显示格式", "2026-09-23".equals(FMDataUpdater.displayDate("20260923")));
		check("displayDate 空/非 8 位返 null",
				FMDataUpdater.displayDate(null) == null && FMDataUpdater.displayDate("2026092") == null);
		// 相对天数用动态日期构造, 写死明天跑就 FAIL
		java.time.format.DateTimeFormatter basic = java.time.format.DateTimeFormatter.BASIC_ISO_DATE;
		String today = java.time.LocalDate.now().format(basic);
		String yesterday = java.time.LocalDate.now().minusDays(1).format(basic);
		check("relativeDays 今天", "今天".equals(FMDataUpdater.relativeDays(today)));
		check("relativeDays 1 天前", "1 天前".equals(FMDataUpdater.relativeDays(yesterday)));
		check("relativeDays 非法日期返 null", FMDataUpdater.relativeDays("20261399") == null);
		check("relativeDays 空/非 8 位返 null",
				FMDataUpdater.relativeDays(null) == null && FMDataUpdater.relativeDays("x") == null);
	}

	// ---- 8. 本地 HttpServer 整链 ----

	private static void testFullCycle() throws Exception {
		System.out.println("-- 整链 (本地 HttpServer) --");
		File tmp = tempDir("vmtest_cycle");
		// 本地旧 data
		writeFile(new File(tmp, "data/aces/version"), "1.0.0.0");

		// 云端包: zip 顶层 data/, 内含新版本号
		byte[] zipBytes = buildZip("9.9.9.9");
		String sha = prog.util.FileUtils.sha256Hex(writeTemp(tmp, "src.zip", zipBytes));

		HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
		// zip 内不带 date 文件 (旧版 zip 形态), manifest 带 date —— 专测 updater 兜底落盘
		final String manifest = "{\n  \"wt_version\": \"9.9.9.9\",\n  \"date\": \"20260923\",\n"
				+ "  \"zip\": \"test.zip\",\n  \"sha256\": \"" + sha + "\"\n}";
		server.createContext("/data_manifest.json", ex -> {
			byte[] b = manifest.getBytes(StandardCharsets.UTF_8);
			ex.getResponseHeaders().set("Content-Type", "application/json");
			ex.sendResponseHeaders(200, b.length);
			try (OutputStream os = ex.getResponseBody()) {
				os.write(b);
			}
		});
		server.createContext("/test.zip", ex -> {
			ex.sendResponseHeaders(200, zipBytes.length);
			try (OutputStream os = ex.getResponseBody()) {
				os.write(zipBytes);
			}
		});
		server.start();
		String base = "http://127.0.0.1:" + server.getAddress().getPort() + "/";

		FMDataPaths.setDataRoot(new File(tmp, "data").toString());
		FMManager.getInstance().reset();
		try {
			FMDataUpdater.getInstance().runUpdateCycle(base, true);
			check("整链更新后版本就位",
					"9.9.9.9".equals(readFile(new File(tmp, "data/aces/version"))));
			// date 兜底: zip 无 date 文件, 由 manifest.date 写入
			check("打包日期兜底落盘",
					"20260923".equals(readFile(new File(tmp, "data/aces/date"))));
			check("readLocalVersion/readLocalDate 语义",
					"9.9.9.9".equals(FMDataUpdater.readLocalVersion())
							&& "20260923".equals(FMDataUpdater.readLocalDate()));
			check("staging 已清理", !new File(tmp, "data_staging").exists());
			check("backup 已清理", !new File(tmp, "data_old").exists());

			// 版本已是最新: 再跑一次不重复下载 (本地 version == 云端 wt_version)
			FMDataUpdater.getInstance().runUpdateCycle(base, true);
			check("版本相同时跳过", "9.9.9.9".equals(readFile(new File(tmp, "data/aces/version"))));

			// 坏 sha: 传输损坏被拒, 旧数据保持
			String badSha = manifest.replace(sha, "deadbeef");
			server.removeContext("/data_manifest.json");
			server.createContext("/data_manifest.json", ex -> {
				byte[] b = badSha.getBytes(StandardCharsets.UTF_8);
				ex.sendResponseHeaders(200, b.length);
				try (OutputStream os = ex.getResponseBody()) {
					os.write(b);
				}
			});
			writeFile(new File(tmp, "data/aces/version"), "8.8.8.8"); // 模拟本地落后
			FMDataUpdater.getInstance().runUpdateCycle(base, true);
			check("坏 sha 被拒且旧数据保持",
					"8.8.8.8".equals(readFile(new File(tmp, "data/aces/version"))));
			check("坏 sha 后 staging 已清理", !new File(tmp, "data_staging").exists());
		} finally {
			server.stop(0);
		}
	}

	private static byte[] buildZip(String versionInside) throws IOException {
		try (java.io.ByteArrayOutputStream bos = new java.io.ByteArrayOutputStream();
				ZipOutputStream zos = new ZipOutputStream(bos)) {
			zos.putNextEntry(new ZipEntry("data/aces/version"));
			zos.write(versionInside.getBytes(StandardCharsets.UTF_8));
			zos.closeEntry();
			zos.putNextEntry(new ZipEntry("data/aces/gamedata/flightmodels/dummy.blkx"));
			zos.write("dummy".getBytes(StandardCharsets.UTF_8));
			zos.closeEntry();
			zos.finish();
			return bos.toByteArray();
		}
	}

	private static File writeTemp(File dir, String name, byte[] bytes) throws IOException {
		File f = new File(dir, name);
		try (FileOutputStream fos = new FileOutputStream(f)) {
			fos.write(bytes);
		}
		return f;
	}
}
