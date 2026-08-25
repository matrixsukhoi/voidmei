import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import parser.Blkx;
import prog.fm.FMDataPaths;
import prog.fm.FMLoader;
import prog.fm.FMHandle;
import prog.fm.FMStatus;
import prog.i18n.Lang;
import prog.util.Logger;

/**
 * blkx 文本变异 Fuzz 测试 (P6) —— FM 物理文件解析管线的防御性验收
 *
 * 种子取项目内 data/ 的真机 blkx (默认 fm/bf-109e-4.blkx —— 中等体积且带
 * PASSPORT.ALT/IAS 曲线数组, 能覆盖 getAllplotdata 的 parseDouble/split 路径;
 * 注意 spitfire_f24.blkx 无 PASSPORT 块, 用它当种子会让腿1的管线阶段空转),
 * 对其施加字节级/行级/结构级/语义级四类变异, 每个变异体走完整生产管线:
 *
 *   腿1 (每个变异体): new Blkx(临时文件, name) —— 构造器含 getload() (P1 已加
 *       固: 构造器不得抛异常, 失败置 valid=false) → valid 时接 getAllplotdata() +
 *       finalizeLoading() (与 FMLoader.load 的 5/6 两步完全一致)
 *
 *   腿2 (抽样 30 个变异体): FMLoader.load(planeName) —— 写临时 data 目录结构
 *       (FMDataPaths.setDataRoot 注入), 断言返回句柄契约:
 *       status ∈ {READY, MISSING, CORRUPT} 且 READY ⇒ blkx != null、
 *       isMissingLike ⇒ blkx == null (P2 回归的直接扩展)
 *
 * 验收标准 (每个变异体):
 *   ① 任何 Throwable 逃逸即失败 (OutOfMemoryError 单独标记并提示)
 *   ② 单变异体限时 5s (变异集合有限 + 固定种子顺序执行, 超时视为疑似死循环)
 *   ③ valid 布尔与对象状态自洽 (valid=false 时不访问解析字段)
 *
 * 固定种子 (默认 20260825) 保证可复现; --seed/--iterations 可覆盖。
 *
 * 运行方式: python script/build.py test fuzz-blkx
 *   (build.py 会传 --central <data/.../bf-109e-4.blkx> --fm <data/.../fm/bf-109e-4.blkx>;
 *    data/ 缺失时 build.py 的 run_fm_test 机制自动跳过整套)
 */
public class FMParserFuzzer {

	/** 默认迭代数 (变异体个数) */
	private static final int DEFAULT_ITERATIONS = 200;
	/** 默认随机种子 —— 固定值保证变异序列可复现 */
	private static final long DEFAULT_SEED = 20260825L;
	/** 腿2 抽样走 FMLoader 的变异体个数 */
	private static final int LOADER_SAMPLES = 30;
	/** 单变异体耗时上限 (ms), 超过判失败 (疑似死循环) */
	private static final long PER_CASE_LIMIT_MS = 5000;

	/** 字节级字符替换池: ASCII 全谱, 含换行/引号/花括号/等号等结构性字符 */
	private static final String ASCII_POOL = "abcXYZ019 \t\n\r\"'{}[]<>=:,;.+-*/\\#$%&()!?|~^`_";

	/** 语义级: 数值字面量的替换池 (NaN / 上下溢 / 500 位长数字 / 负零) */
	private static final String[] NUM_REPLACEMENTS = {
		"NaN", "1e999", "-1e999", repeat('9', 500), "-0"
	};

	/** 数值字面量定位 (整数/小数/科学计数) */
	private static final Pattern RE_NUM = Pattern.compile("\\d+\\.?\\d*(?:[eE][-+]?\\d+)?");
	/** 带引号字符串定位 (单行内, 长度上界防误吃大段文本) */
	private static final Pattern RE_QUOTED = Pattern.compile("\"([^\"\\n\\r]{1,60})\"");

	private static int passed = 0;
	private static int failed = 0;
	private static int fuzzCases = 0;
	/** 逃逸异常计数 (构造器阶段 / 管线阶段分开记, 便于定位) */
	private static int ctorExceptions = 0;
	private static int pipelineExceptions = 0;
	private static int validTrue = 0;
	private static int validFalse = 0;

	// 变异策略名 (下标即策略编号, 输出统计用)
	private static final String[] STRATEGY_NAMES = {
		"truncate", "charReplace", "chunkPaste", "deleteLine", "shuffleLines",
		"commentLine", "stripIndent", "dropBrace", "killEquals", "injectNest",
		"numberMutate", "unquote", "jsonInject"
	};
	private static final int[] strategyCount = new int[STRATEGY_NAMES.length];

	public static void main(String[] args) throws Exception {
		System.out.println("=== blkx 文本变异 Fuzz 测试 ===\n");

		String centralPath = null;
		String fmPath = null;
		int iterations = DEFAULT_ITERATIONS;
		long seed = DEFAULT_SEED;
		for (int i = 0; i < args.length - 1; i++) {
			if ("--central".equals(args[i])) {
				centralPath = args[i + 1];
			} else if ("--fm".equals(args[i])) {
				fmPath = args[i + 1];
			} else if ("--iterations".equals(args[i])) {
				iterations = Integer.parseInt(args[i + 1]);
			} else if ("--seed".equals(args[i])) {
				seed = Long.parseLong(args[i + 1]);
			}
		}
		if (fmPath == null) {
			System.out.println("Usage: java FMParserFuzzer --fm <path> [--central <path>]"
					+ " [--iterations 200] [--seed 20260825]");
			System.exit(1);
		}

		// Blkx 构造器依赖 Lang.noblkx 等字符串 (fmdata 初始值), 先初始化语言
		Lang.initLang();
		// 压掉每个变异体的 "Parsed FM file ..." INFO 噪音 (200 行), 只留 WARN 及以上;
		// Blx 的 JSON 误喂/解析失败提示是 WARN, 仍可见
		Logger.setMinLevel(Logger.Level.WARN);

		Path fmFile = new File(fmPath).toPath();
		if (!Files.isRegularFile(fmFile)) {
			System.out.println("SKIP: 种子文件不存在: " + fmPath);
			System.exit(0);
		}
		// 读文件用平台默认字符集 —— 与 Blx 构造器内 FileReader 一致,
		// 保证"读出->变异->写回->Blkx 再读"对非 ASCII 字节往返一致
		String seedText = new String(Files.readAllBytes(fmFile));
		String seedName = fmFile.getFileName().toString();
		seedName = seedName.substring(0, seedName.lastIndexOf('.'));
		System.out.println("种子: " + fmPath + " (" + seedText.length() + " chars)"
				+ " | 迭代 " + iterations + " | 种子值 " + seed + "\n");

		long startMs = System.currentTimeMillis();

		// ---- 基线自检: 原始种子必须能正常解析 (否则数据有问题, 本套件直接判失败) ----
		if (!baselineCheck(seedText, seedName)) {
			System.out.println("\n=== 测试结果 ===");
			System.out.println("失败: " + failed + " (基线未通过)");
			System.exit(1);
		}

		// ---- 生成全部变异体 (单个 Random 顺序驱动, 固定种子下完全可复现) ----
		Random rnd = new Random(seed);
		List<String> mutants = new ArrayList<String>();
		List<Integer> kinds = new ArrayList<Integer>();
		for (int i = 0; i < iterations; i++) {
			int kind = rnd.nextInt(STRATEGY_NAMES.length);
			strategyCount[kind]++;
			mutants.add(mutate(seedText, kind, rnd));
			kinds.add(kind);
		}

		// ---- 腿1: 每个变异体直接走 Blkx 全管线 ----
		System.out.println("-- 腿1: Blkx 全管线 (构造器 + getAllplotdata + finalizeLoading) x"
				+ mutants.size() + " --");
		Path tmpBlkx = Files.createTempFile("voidmei_fuzz_", ".blkx");
		try {
			for (int i = 0; i < mutants.size(); i++) {
				runDirectPipeline(mutants.get(i), kinds.get(i), i, tmpBlkx, seedName);
			}
		} finally {
			Files.deleteIfExists(tmpBlkx);
		}
		System.out.println("  完成: valid=true " + validTrue + " 个, valid=false " + validFalse
				+ " 个, 逃逸异常 " + (ctorExceptions + pipelineExceptions) + " 个");

		// ---- 腿2: 抽样变异体走 FMLoader.load (P2 句柄契约回归) ----
		if (centralPath != null && new File(centralPath).isFile()) {
			System.out.println("\n-- 腿2: FMLoader.load 句柄契约 x" + LOADER_SAMPLES + " --");
			runLoaderLeg(centralPath, mutants);
		} else {
			System.out.println("\n-- 腿2 跳过: 未提供有效的 --central (FMLoader 契约测试需要中央文件) --");
		}

		long elapsed = System.currentTimeMillis() - startMs;

		// ---- 汇总 ----
		System.out.println("\n-- 变异策略分布 --");
		for (int k = 0; k < STRATEGY_NAMES.length; k++) {
			System.out.printf("  %-13s %d%n", STRATEGY_NAMES[k], strategyCount[k]);
		}
		System.out.printf("%n共 %d 个变异体, 总耗时 %d ms%n", fuzzCases, elapsed);
		System.out.println("\n=== 测试结果 ===");
		System.out.println("通过: " + passed);
		System.out.println("失败: " + failed);

		Logger.setMinLevel(Logger.Level.INFO);
		if (failed > 0) {
			System.exit(1);
		}
	}

	// ==================== 基线 ====================

	/** 原始种子必须 valid (真机数据本应可解析; 失败说明种子选择或环境有误) */
	private static boolean baselineCheck(String seedText, String seedName) throws Exception {
		Path tmp = Files.createTempFile("voidmei_fuzz_base_", ".blkx");
		try {
			Files.write(tmp, seedText.getBytes());
			Blkx b = new Blkx(tmp.toString(), seedName);
			if (!b.valid) {
				System.out.println("  [失败] 基线: 原始种子解析后 valid=false (不应发生)");
				failed++;
				return false;
			}
			b.getAllplotdata();
			b.finalizeLoading();
			System.out.println("  [通过] 基线: 原始种子全管线解析成功");
			passed++;
			return true;
		} finally {
			Files.deleteIfExists(tmp);
		}
	}

	// ==================== 腿1: Blkx 直连全管线 ====================

	/**
	 * 单变异体执行: 写临时文件 → new Blkx (含 getload) → valid 时接
	 * getAllplotdata + finalizeLoading (与 FMLoader.load 第 5/6 步一致)。
	 * 断言① 逃逸 Throwable 即失败; ② 单体 5s 限时; ③ valid=false 不触碰解析字段。
	 */
	private static void runDirectPipeline(String mutant, int kind, int index, Path tmpBlkx,
			String seedName) {
		fuzzCases++;
		try {
			Files.write(tmpBlkx, mutant.getBytes());
		} catch (Exception e) {
			System.out.println("  [失败] #" + index + " 写临时文件异常: " + e);
			failed++;
			return;
		}

		long t0 = System.nanoTime();
		try {
			// 构造器即 P1 加固边界: getload 包 try, 任何解析失败都应置 valid=false 而非抛出
			Blkx b = new Blkx(tmpBlkx.toString(), seedName);
			if (b.valid) {
				validTrue++;
				// 断言③: valid=true 时对象状态自洽 —— 原始文本已读入
				if (b.data == null) {
					System.out.println("  [失败] #" + index + " (" + STRATEGY_NAMES[kind]
							+ ") valid=true 但 data 为 null");
					failed++;
					return;
				}
				// 与生产管线一致的后两步 (FMLoader.load 第 6 步)
				b.getAllplotdata();
				b.finalizeLoading();
			} else {
				// 断言③: valid=false 时刻意不访问任何解析字段 (只看布尔, 对象应安全废弃)
				validFalse++;
			}
		} catch (OutOfMemoryError oome) {
			// OOM 单独标记: 变异可能诱发巨量分配, 提示而非与普通异常混报
			ctorExceptions++;
			System.out.println("  [失败] #" + index + " (" + STRATEGY_NAMES[kind]
					+ ") 逃逸 OutOfMemoryError: " + oome
					+ " —— 检查变异策略是否诱发无界分配");
			failed++;
			return;
		} catch (Throwable t) {
			// 构造器阶段与管线阶段分开计数 (堆栈第一帧在 Blkx 构造器内 = 构造器逃逸)
			boolean inCtor = false;
			for (StackTraceElement st : t.getStackTrace()) {
				if (st.getClassName().equals("parser.Blkx") && st.getMethodName().equals("<init>")) {
					inCtor = true;
					break;
				}
			}
			if (inCtor)
				ctorExceptions++;
			else
				pipelineExceptions++;
			System.out.println("  [失败] #" + index + " (" + STRATEGY_NAMES[kind]
					+ ") 逃逸异常[" + (inCtor ? "构造器" : "管线") + "]: " + t);
			StackTraceElement[] st = t.getStackTrace();
			if (st.length > 0)
				System.out.println("         at " + st[0]);
			dumpMutant(mutant, index);
			failed++;
			return;
		}
		long ms = (System.nanoTime() - t0) / 1_000_000L;
		// 断言②: 单体限时 (变异集合有限 + 顺序执行可复现, 超时即疑似死循环)
		if (ms > PER_CASE_LIMIT_MS) {
			System.out.println("  [失败] #" + index + " (" + STRATEGY_NAMES[kind]
					+ ") 单文件耗时 " + ms + " ms 超过 " + PER_CASE_LIMIT_MS + " ms 上限");
			dumpMutant(mutant, index);
			failed++;
		}
	}

	/** 失败现场留存: 把出问题的变异体写到 build/ 下供人工复现 (build/ 已 gitignore) */
	private static void dumpMutant(String mutant, int index) {
		try {
			File buildDir = new File("build");
			if (!buildDir.isDirectory())
				return; // 无 build 目录 (如 CI 精简环境) 时静默跳过, 不影响测试结果
			Path out = buildDir.toPath().resolve("fuzz_fail_" + index + ".blkx");
			Files.write(out, mutant.getBytes());
			System.out.println("         变异体已留存: " + out.toAbsolutePath());
		} catch (Exception ignore) {
			// 留存失败不影响断言结果
		}
	}

	// ==================== 腿2: FMLoader 句柄契约 ====================

	/**
	 * 抽样 LOADER_SAMPLES 个变异体作物理文件, 中央文件用真机原件,
	 * 通过 FMDataPaths.setDataRoot 注入临时 data 根后走 FMLoader.load(name)。
	 * 断言: 句柄非 null; status ∈ {READY, MISSING, CORRUPT};
	 * READY ⇒ blkx != null; isMissingLike ⇒ blkx == null。
	 */
	private static void runLoaderLeg(String centralPath, List<String> mutants) throws Exception {
		File central = new File(centralPath);
		String plane = central.getName();
		plane = plane.substring(0, plane.lastIndexOf('.')); // 中央文件名即机型名 (下划线)

		Path tmpRoot = Files.createTempDirectory("voidmei_fuzzloader_");
		try {
			Path fmDir = tmpRoot.resolve("aces").resolve("gamedata").resolve("flightmodels");
			Path fmSub = fmDir.resolve("fm");
			Files.createDirectories(fmSub);
			// 中央文件: 真机原件原样拷入 (腿2 只变异物理文件, 走最重的解析路径)
			Files.copy(central.toPath(), fmDir.resolve(plane + ".blkx"));

			// 中央文件里记录的物理文件名 (fmfile 字段) 决定 FMLoader 找哪个文件;
			// 简化处理: 直接看中央文件内容, 取不到就回退 fm/<机型>.blkx 约定
			String centralText = new String(Files.readAllBytes(central.toPath()));
			String physRel = extractFmFile(centralText);
			if (physRel == null)
				physRel = "fm/" + plane + ".blkx";
			Path physTarget = fmDir.resolve(physRel + "x"); // FMLoader 拼 fmfile + "x"
			Files.createDirectories(physTarget.getParent());

			FMDataPaths.setDataRoot(tmpRoot.toString());
			System.out.println("  机型: " + plane + " | 物理文件: " + physRel + "x");

			int step = Math.max(1, mutants.size() / LOADER_SAMPLES);
			int ready = 0, corrupt = 0, missing = 0;
			int sampleCount = 0;
			for (int i = 0; i < mutants.size() && sampleCount < LOADER_SAMPLES; i += step) {
				sampleCount++;
				Files.write(physTarget, mutants.get(i).getBytes());
				// FMLoader.load 内部 catch(Throwable) → CORRUPT, 契约上永不抛出/永不返回 null
				FMHandle h = FMLoader.load(plane);
				boolean ok = h != null
						&& (h.status == FMStatus.READY || h.status == FMStatus.MISSING
								|| h.status == FMStatus.CORRUPT)
						&& ((h.status == FMStatus.READY) == (h.blkx != null));
				if (ok && h.isMissingLike()) {
					ok = h.blkx == null; // missing-like 句柄必须不带 blkx
				}
				if (ok) {
					if (h.status == FMStatus.READY)
						ready++;
					else if (h.status == FMStatus.CORRUPT)
						corrupt++;
					else
						missing++;
				} else {
					System.out.println("  [失败] 样本#" + i + " 句柄契约违反: " + h
							+ " (blkx=" + (h != null && h.blkx != null ? "非null" : "null") + ")");
					failed++;
				}
			}
			passed++;
			System.out.println("  完成: " + sampleCount + " 个样本 → READY " + ready
					+ " / CORRUPT " + corrupt + " / MISSING " + missing);
		} finally {
			// 还原全局数据根并清理临时目录 (无论成败)
			FMDataPaths.setDataRoot("./data");
			rmtree(tmpRoot.toFile());
		}
	}

	/** 从中央文件文本提取 fmFile 字段的相对路径 (形如 fm/xxx.blk); 解析失败返回 null */
	private static String extractFmFile(String centralText) {
		try {
			Matcher m = Pattern.compile("fmFile:t\\s*=\\s*\"([^\"]+)\"").matcher(centralText);
			if (!m.find())
				return null;
			String v = m.group(1);
			if (v.startsWith("/"))
				v = v.substring(1);
			return v;
		} catch (Exception e) {
			return null;
		}
	}

	// ==================== 变异原语 (四类 13 种) ====================

	private static String mutate(String s, int kind, Random rnd) {
		switch (kind) {
		case 0:
			return truncate(s, rnd); // 字节级: 头/中/尾截断
		case 1:
			return charReplace(s, rnd); // 字节级: 随机字符替换 (ASCII 全谱)
		case 2:
			return chunkPaste(s, rnd); // 字节级: 段落复制粘贴
		case 3:
			return deleteLines(s, rnd); // 行级: 随机删行
		case 4:
			return shuffleLines(s, rnd); // 行级: 行乱序
		case 5:
			return commentLines(s, rnd); // 行级: 前插 // 注释化
		case 6:
			return stripIndent(s, rnd); // 行级: 缩进清空
		case 7:
			return dropBrace(s, rnd); // 结构级: 删一个 { 或 } (括号失配)
		case 8:
			return killEquals(s, rnd); // 结构级: 某个 = 换成空格
		case 9:
			return injectNest(s, rnd); // 结构级: 注入额外嵌套 "{\n" 块
		case 10:
			return numberMutate(s, rnd); // 语义级: 数值字面量换 NaN/1e999/长数字等
		case 11:
			return unquote(s, rnd); // 语义级: 去掉某个字符串的引号
		case 12:
			return jsonInject(s, rnd); // 语义级: 注入 JSON 片段替换随机区间
		default:
			return charReplace(s, rnd);
		}
	}

	/** 字节级-截断: 头部/中部/尾部随机去掉一段 (最多 55%) */
	private static String truncate(String s, Random rnd) {
		int len = s.length();
		if (len < 32)
			return s;
		int cut = Math.max(1, (int) (len * (0.02 + rnd.nextDouble() * 0.53)));
		int mode = rnd.nextInt(3);
		if (mode == 0)
			return s.substring(cut); // 头部截断
		if (mode == 1)
			return s.substring(0, len - cut); // 尾部截断
		int at = rnd.nextInt(len - cut); // 中部截断
		return s.substring(0, at) + s.substring(at + cut);
	}

	/** 字节级-字符替换: 1~8 个随机位置换成 ASCII 池字符 (含换行/引号/花括号/等号) */
	private static String charReplace(String s, Random rnd) {
		int len = s.length();
		if (len < 16)
			return s;
		StringBuilder sb = new StringBuilder(s);
		int n = 1 + rnd.nextInt(8);
		for (int k = 0; k < n; k++) {
			int at = rnd.nextInt(len);
			sb.setCharAt(at, ASCII_POOL.charAt(rnd.nextInt(ASCII_POOL.length())));
		}
		return sb.toString();
	}

	/** 字节级-段落复制粘贴: 取一段 (≤10%) 复制插入到随机位置 */
	private static String chunkPaste(String s, Random rnd) {
		int len = s.length();
		if (len < 64)
			return s;
		int clen = 1 + rnd.nextInt(Math.min(2000, Math.max(2, len / 10)));
		int from = rnd.nextInt(len - clen);
		String chunk = s.substring(from, from + clen);
		int at = rnd.nextInt(len);
		return s.substring(0, at) + chunk + s.substring(at);
	}

	/** 行级-删行: 随机删 1~3 行 */
	private static String deleteLines(String s, Random rnd) {
		String[] lines = s.split("\n", -1);
		if (lines.length < 4)
			return s;
		int n = 1 + rnd.nextInt(3);
		StringBuilder sb = new StringBuilder();
		int first = rnd.nextInt(lines.length);
		for (int i = 0; i < lines.length; i++) {
			if (i >= first && i < first + n)
				continue; // 跳过被删的连续行
			sb.append(lines[i]).append('\n');
		}
		return sb.toString();
	}

	/** 行级-乱序: 随机交换 2~4 对行 */
	private static String shuffleLines(String s, Random rnd) {
		String[] lines = s.split("\n", -1);
		if (lines.length < 4)
			return s;
		int n = 2 + rnd.nextInt(3);
		for (int k = 0; k < n; k++) {
			int i = rnd.nextInt(lines.length);
			int j = rnd.nextInt(lines.length);
			String t = lines[i];
			lines[i] = lines[j];
			lines[j] = t;
		}
		return join(lines);
	}

	/** 行级-注释化: 随机 1~3 个非空行前插 // */
	private static String commentLines(String s, Random rnd) {
		String[] lines = s.split("\n", -1);
		if (lines.length < 4)
			return s;
		int n = 1 + rnd.nextInt(3);
		for (int k = 0; k < n; k++) {
			int i = rnd.nextInt(lines.length);
			if (lines[i].trim().length() > 0)
				lines[i] = "//" + lines[i];
		}
		return join(lines);
	}

	/** 行级-缩进清空: 随机窗口内 ≤30 行去掉行首空白 (破坏花括号缩进结构) */
	private static String stripIndent(String s, Random rnd) {
		String[] lines = s.split("\n", -1);
		if (lines.length < 4)
			return s;
		int w = rnd.nextInt(lines.length);
		int end = Math.min(lines.length, w + 30);
		for (int i = w; i < end; i++) {
			lines[i] = lines[i].replaceFirst("^[ \\t]+", "");
		}
		return join(lines);
	}

	/** 结构级-括号失配: 随机删除一个 '{' 或 '}' */
	private static String dropBrace(String s, Random rnd) {
		List<Integer> bracePos = new ArrayList<Integer>();
		for (int i = 0; i < s.length(); i++) {
			char c = s.charAt(i);
			if (c == '{' || c == '}')
				bracePos.add(i);
		}
		if (bracePos.isEmpty())
			return s;
		int at = bracePos.get(rnd.nextInt(bracePos.size()));
		return s.substring(0, at) + s.substring(at + 1);
	}

	/** 结构级-赋值破坏: 随机一个 '=' 换成空格 */
	private static String killEquals(String s, Random rnd) {
		List<Integer> eqPos = new ArrayList<Integer>();
		for (int i = 0; i < s.length(); i++) {
			if (s.charAt(i) == '=')
				eqPos.add(i);
		}
		if (eqPos.isEmpty())
			return s;
		int at = eqPos.get(rnd.nextInt(eqPos.size()));
		return s.substring(0, at) + ' ' + s.substring(at + 1);
	}

	/** 结构级-嵌套注入: 在随机位置插入 1~3 个 "{\n" (刻意不配对, 制造额外嵌套) */
	private static String injectNest(String s, Random rnd) {
		StringBuilder sb = new StringBuilder(s);
		int n = 1 + rnd.nextInt(3);
		for (int k = 0; k < n; k++) {
			int at = rnd.nextInt(sb.length() + 1);
			sb.insert(at, "{\n");
		}
		return sb.toString();
	}

	/** 语义级-数值变异: 随机一个数值字面量换成 NaN/1e999/-1e999/500 位长数字/负零 */
	private static String numberMutate(String s, Random rnd) {
		Matcher m = RE_NUM.matcher(s);
		List<int[]> matches = new ArrayList<int[]>();
		while (m.find() && matches.size() < 5000) {
			matches.add(new int[] { m.start(), m.end() });
		}
		if (matches.isEmpty())
			return s;
		int[] pick = matches.get(rnd.nextInt(matches.size()));
		return s.substring(0, pick[0]) + NUM_REPLACEMENTS[rnd.nextInt(NUM_REPLACEMENTS.length)]
				+ s.substring(pick[1]);
	}

	/** 语义级-去引号: 随机一个带引号字符串去掉两侧引号 */
	private static String unquote(String s, Random rnd) {
		Matcher m = RE_QUOTED.matcher(s);
		List<int[]> matches = new ArrayList<int[]>();
		while (m.find() && matches.size() < 5000) {
			matches.add(new int[] { m.start(), m.end() });
		}
		if (matches.isEmpty())
			return s;
		int[] pick = matches.get(rnd.nextInt(matches.size()));
		return s.substring(0, pick[0]) + s.substring(pick[0] + 1, pick[1] - 1) + s.substring(pick[1]);
	}

	/** 语义级-JSON 注入: 随机区间 (≤5%) 整段替换为 JSON 片段 */
	private static String jsonInject(String s, Random rnd) {
		int len = s.length();
		if (len < 40)
			return s;
		int span = 1 + rnd.nextInt(Math.max(2, len / 20));
		int from = rnd.nextInt(len - span);
		String json = rnd.nextBoolean() ? "{\"a\":1}" : "{\"x\":[1,2,3],\"y\":null}";
		return s.substring(0, from) + json + s.substring(from + span);
	}

	// ==================== 工具 ====================

	private static String join(String[] lines) {
		StringBuilder sb = new StringBuilder();
		for (String l : lines)
			sb.append(l).append('\n');
		return sb.toString();
	}

	private static String repeat(char c, int n) {
		StringBuilder sb = new StringBuilder(n);
		for (int i = 0; i < n; i++)
			sb.append(c);
		return sb.toString();
	}

	private static void rmtree(File f) {
		if (f == null || !f.exists())
			return;
		File[] children = f.listFiles();
		if (children != null) {
			for (File c : children)
				rmtree(c);
		}
		f.delete();
	}
}
