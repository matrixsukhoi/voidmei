import parser.Blkx;
import prog.i18n.Lang;

import java.io.File;
import java.io.FilenameFilter;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

/**
 * 全量真机 FM 边界普查（检视反馈新增）—— 遍历 data/ 下全部物理 FM 文件，
 * 断言真实数据的结构极值与解析器防御护栏之间留有余量：
 *
 * 1. 全部文件解析零异常（Blkx 构造器承诺"任何输入不抛"，真机数据是最大样本集）
 * 2. 引擎数极值 < 解析护栏（Blkx.getload 引用 State.maxEngNum，截断防御未被真机数据触发）
 * 3. 发动机负载档位极值 <= 数组容量（Load 档位截断防御未被触发；
 *    maxEngLoad==9 满档时额外探测 Load10，区分"恰好 10 档"与"10+ 档被截断"）
 * 4. 引擎数极值 <= State.maxEngNum（遥测侧 throttles/power 等数组按引擎索引，
 *    超过即静默丢引擎数据——硬断言，数据越界即提醒上调常量）
 * 5. invalid 文件必须均为空文件（空文件判 invalid 是正确行为，非回归）
 *
 * 同时输出结构统计摘要（引擎数/档位分布），作为 fmdata 数据档案。
 *
 * 自动化扫描（反射, 零手工枚举——新增字段自动纳入）:
 * 6. 全部 public 数值字段非 NaN/非 Infinity（NaN 流入 UI 是真实 bug 源,
 *    如 initEngineLoad 除零历史; 检视反馈要求不依赖手工列举 key）
 * 7. 全部 public 数组字段结构合法: 二维数组必须矩形（锯齿 = 解析不完整）
 *
 * 已知"计数字段→容量上限"配对表（KNOWN_LIMITS, 数据驱动, 新边界加一行即可）:
 *    engineNum→State.maxEngNum, maxEngLoad→Application.maxEngLoad,
 *    altThrNum/velThrNum→30(altitudeThr/velocityThr 数组), modeEngineNum→10,
 *    FlapsDestructionNum→6(FlapsDestructionIndSpeed[6][2])
 *
 * Run with: python script/build.py test fm-all   (data/ 缺失时由 build.py 跳过)
 */
public class TestFMAllBoundaries {

	/** 解析器引擎数截断护栏 = 遥测数组容量（Blkx.getload 引用同一常量, 单一来源） */
	private static final int ENGINE_NUM_GUARD = parser.State.maxEngNum;
	/** 遥测侧每引擎数组长度（State.maxEngNum） */
	private static final int TELEMETRY_ENGINE_CAP = parser.State.maxEngNum;

	/** 已知"计数字段 → 容量上限"配对表: 计数值超过上限即静默截断/越界, 直接红灯 */
	private static final Map<String, Integer> KNOWN_LIMITS = new LinkedHashMap<String, Integer>();
	static {
		KNOWN_LIMITS.put("engineNum", parser.State.maxEngNum);
		KNOWN_LIMITS.put("maxEngLoad", prog.Application.maxEngLoad);
		KNOWN_LIMITS.put("altThrNum", 30);        // altitudeThr = new double[30]
		KNOWN_LIMITS.put("velThrNum", 30);        // velocityThr = new double[30]
		KNOWN_LIMITS.put("FlapsDestructionNum", 6); // FlapsDestructionIndSpeed[6][2]
		// 注: modeEngineNum 是 private 且分配 [10] 与循环 <10 硬性同长, 无数据驱动越界可能,
		// 反射不可达故不列入; 若改为 public 可加 "modeEngineNum"→10
	}

	private static int passed = 0;
	private static int failed = 0;

	private static void assertTrue(boolean cond, String msg) {
		if (cond) {
			passed++;
			System.out.println("  PASS: " + msg);
		} else {
			failed++;
			System.out.println("  FAIL: " + msg);
		}
	}

	public static void main(String[] args) {
		// Lang 先行: Blkx.getload() 输出用 Lang 格式串, 不初始化会 NPE (惯例同其它 FM 测试)
		prog.i18n.Lang.initLang();

		File fmDir = new File("data/aces/gamedata/flightmodels/fm");
		if (!fmDir.isDirectory()) {
			System.out.println("SKIP: 项目内 data/ 不存在 (先运行 python script/build.py fmdata)");
			return;
		}

		File[] files = fmDir.listFiles(new FilenameFilter() {
			public boolean accept(File dir, String name) {
				return name.toLowerCase().endsWith(".blkx");
			}
		});
		if (files == null || files.length == 0) {
			System.out.println("SKIP: fm/ 目录为空");
			return;
		}

		System.out.println("=== FM All-Data Boundary Scan ===");
		System.out.println("扫描目录: " + fmDir.getPath() + " (" + files.length + " 个文件)\n");

		int parseExceptions = 0;
		int invalidCount = 0;
		int jetCount = 0;
		int maxEngineNum = 0;
		int maxLoadStages = 0;
		List<String> invalidFiles = new ArrayList<String>();
		List<String> emptyInvalidFiles = new ArrayList<String>();
		List<String> engineOverTelemetryCap = new ArrayList<String>();
		// 引擎数/档位分布（TreeMap 按档排序, 便于输出数据档案）
		TreeMap<Integer, Integer> engineDist = new TreeMap<Integer, Integer>();
		TreeMap<Integer, Integer> loadDist = new TreeMap<Integer, Integer>();
		// KNOWN_LIMITS 各计数字段的全量极值 (档案输出)
		Map<String, Integer> counterMax = new LinkedHashMap<String, Integer>();

		for (File f : files) {
			String name = f.getName();
			Blkx b;
			try {
				// 生产路径等价构造: doLoad=true 全量解析 (DrawFrame 等直读场景同款)
				b = new Blkx(f.getPath(), name);
			} catch (Throwable t) {
				// 构造器加固后承诺不抛; 真机数据出现异常即护栏失效
				parseExceptions++;
				System.out.println("  EXCEPTION: " + name + " -> " + t);
				continue;
			}

			if (!b.valid) {
				invalidCount++;
				invalidFiles.add(name);
				// 空文件判 invalid 是预期行为 (构造器防御), 单独归类
				if (f.length() == 0)
					emptyInvalidFiles.add(name);
				continue;
			}

			if (b.isJet)
				jetCount++;

			// 引擎数统计与遥测侧容量检查
			if (b.engineNum > maxEngineNum)
				maxEngineNum = b.engineNum;
			Integer c = engineDist.get(b.engineNum);
			engineDist.put(b.engineNum, c == null ? 1 : c + 1);
			if (b.engineNum > TELEMETRY_ENGINE_CAP)
				engineOverTelemetryCap.add(name + " (" + b.engineNum + " 引擎)");

			// 档位统计: maxEngLoad 是最后有效档索引, 有效档数 = maxEngLoad + 1
			int stages = b.maxEngLoad + 1;
			if (stages > maxLoadStages)
				maxLoadStages = stages;
			c = loadDist.get(stages);
			loadDist.put(stages, c == null ? 1 : c + 1);

			// ---- 自动化扫描 (反射, 零手工枚举) ----
			scanNumericFields(b, name);
			scanArrayFields(b, name);
			for (Map.Entry<String, Integer> lim : KNOWN_LIMITS.entrySet()) {
				try {
					Field fl = Blkx.class.getField(lim.getKey());
					int v = fl.getInt(b);
					Integer cur = counterMax.get(lim.getKey());
					if (cur == null || v > cur)
						counterMax.put(lim.getKey(), v);
					if (v > lim.getValue()) {
						failed++;
						System.out.println("  FAIL: " + name + " 的 " + lim.getKey()
								+ "=" + v + " 超出容量上限 " + lim.getValue() + " (静默截断/越界风险)");
					}
				} catch (NoSuchFieldException e) {
					failed++;
					System.out.println("  FAIL: KNOWN_LIMITS 引用了不存在的字段: " + lim.getKey());
				} catch (IllegalAccessException e) {
					// public 字段不应发生; 发生即测试自身缺陷, 判失败暴露
					failed++;
					System.out.println("  FAIL: KNOWN_LIMITS 读取 " + lim.getKey() + " 失败: " + e);
				}
			}

			// 满档 (索引 9, 即 10 档全填满) 时探测 Load10:
			// 区分"恰好 10 档"与"11+ 档被数组容量截断"——截断发生即护栏余量为零
			if (b.maxEngLoad == 9 && b.getdouble("Load10.WaterTemperature") != 0) {
				failed++;
				System.out.println("  FAIL: " + name + " 的 Load 档位被数组容量截断 (Load10+ 存在但未解析)");
			}
		}

		// ---- 结构统计摘要 (数据档案) ----
		System.out.println("---- 结构统计 ----");
		System.out.println("有效解析: " + (files.length - invalidCount - parseExceptions)
				+ " / " + files.length + "  (喷气 " + jetCount + ")");
		StringBuilder sb = new StringBuilder("引擎数分布: ");
		for (java.util.Map.Entry<Integer, Integer> e : engineDist.entrySet())
			sb.append(e.getKey()).append("发x").append(e.getValue()).append("  ");
		System.out.println(sb.toString().trim());
		sb = new StringBuilder("档位数分布: ");
		for (java.util.Map.Entry<Integer, Integer> e : loadDist.entrySet())
			sb.append(e.getKey()).append("档x").append(e.getValue()).append("  ");
		System.out.println(sb.toString().trim());
		System.out.println("引擎数极值: " + maxEngineNum + " (解析护栏 " + ENGINE_NUM_GUARD
				+ ", 遥测数组容量 " + TELEMETRY_ENGINE_CAP + ")");
		System.out.println("档位数极值: " + maxLoadStages + " (engLoad 数组容量 10)");
		StringBuilder cb = new StringBuilder("计数字段极值: ");
		for (Map.Entry<String, Integer> e : counterMax.entrySet())
			cb.append(e.getKey()).append("=").append(e.getValue())
					.append("/").append(KNOWN_LIMITS.get(e.getKey())).append("  ");
		System.out.println(cb.toString().trim());
		if (invalidCount > 0) {
			System.out.println("invalid 文件 (" + invalidCount + "): " + invalidFiles);
		}
		System.out.println();

		// ---- 边界断言 ----
		assertTrue(parseExceptions == 0,
				"全部 " + files.length + " 个真机 FM 解析零异常 (实际异常 " + parseExceptions + ")");
		assertTrue(maxEngineNum < ENGINE_NUM_GUARD,
				"引擎数极值 " + maxEngineNum + " 未触解析护栏 " + ENGINE_NUM_GUARD);
		assertTrue(maxLoadStages <= 10,
				"档位数极值 " + maxLoadStages + " 未超出 engLoad 数组容量 10");
		assertTrue(emptyInvalidFiles.size() == invalidCount,
				"invalid 文件均为空文件 (空文件判 invalid 为预期行为; 非空 invalid "
						+ (invalidCount - emptyInvalidFiles.size()) + " 个)");
		// 遥测侧容量: 超限即 State 侧按引擎数组静默截断 (第 9+ 引擎数据丢弃)。
		// 检视反馈: 不做假通过——硬断言。历史包袱已修: State.maxEngNum 8->16
		// (2026-08 普查极值 b_66b 14 引擎块), 此后数据越界即红灯, 提醒同步上调
		// State.maxEngNum 与 Blkx 解析护栏 (两者为同一常量)
		assertTrue(engineOverTelemetryCap.isEmpty(),
				"引擎数极值 " + maxEngineNum + " 在遥测数组容量 " + TELEMETRY_ENGINE_CAP + " 内"
						+ (engineOverTelemetryCap.isEmpty() ? "" : "; 超限机型: " + engineOverTelemetryCap));

		System.out.println();
		System.out.println("TestFMAllBoundaries: " + passed + " passed, " + failed + " failed");
		System.exit(failed == 0 ? 0 : 1);
	}

	/**
	 * 反射扫描全部 public 数值字段 (double/float 及其包装/数组): 断言非 NaN 非 Infinity。
	 * 零手工枚举——新增字段自动纳入。NaN 流入 UI 是真实 bug 源 (如历史除零缺陷)。
	 */
	private static void scanNumericFields(Blkx b, String file) {
		for (Field f : Blkx.class.getFields()) {
			if (Modifier.isStatic(f.getModifiers()))
				continue;
			Class<?> type = f.getType();
			try {
				if (type == double.class) {
					double v = f.getDouble(b);
					if (Double.isNaN(v) || Double.isInfinite(v))
						reportBadNumber(file, f.getName(), String.valueOf(v));
				} else if (type == float.class) {
					float v = f.getFloat(b);
					if (Float.isNaN(v) || Float.isInfinite(v))
						reportBadNumber(file, f.getName(), String.valueOf(v) + "f");
				} else if (type == Double.class || type == Float.class) {
					Object o = f.get(b);
					if (o instanceof Double) {
						double v = (Double) o;
						if (Double.isNaN(v) || Double.isInfinite(v))
							reportBadNumber(file, f.getName(), String.valueOf(v));
					} else if (o instanceof Float) {
						float v = (Float) o;
						if (Float.isNaN(v) || Float.isInfinite(v))
							reportBadNumber(file, f.getName(), String.valueOf(v) + "f");
					}
				} else if (type == double[].class) {
					double[] arr = (double[]) f.get(b);
					if (arr == null)
						continue;
					for (int i = 0; i < arr.length; i++)
						if (Double.isNaN(arr[i]) || Double.isInfinite(arr[i]))
							reportBadNumber(file, f.getName() + "[" + i + "]", String.valueOf(arr[i]));
				} else if (type == float[].class) {
					float[] arr = (float[]) f.get(b);
					if (arr == null)
						continue;
					for (int i = 0; i < arr.length; i++)
						if (Float.isNaN(arr[i]) || Float.isInfinite(arr[i]))
							reportBadNumber(file, f.getName() + "[" + i + "]", String.valueOf(arr[i]) + "f");
				}
			} catch (IllegalAccessException e) {
				// public 字段不应发生
			}
		}
	}

	private static void reportBadNumber(String file, String field, String value) {
		failed++;
		System.out.println("  FAIL: " + file + " 的 " + field + " = " + value + " (NaN/Infinity 不应流入解析结果)");
	}

	/**
	 * 反射扫描全部 public 数组字段的结构合法性: 二维数组必须矩形 (各行等长),
	 * 锯齿 = 解析不完整/部分填充, 下游按矩形遍历会读到 0 假数据或越界。
	 */
	private static void scanArrayFields(Blkx b, String file) {
		for (Field f : Blkx.class.getFields()) {
			if (Modifier.isStatic(f.getModifiers()))
				continue;
			Class<?> type = f.getType();
			if (!type.isArray() || !type.getComponentType().isArray())
				continue; // 只查二维
			try {
				Object outer = f.get(b);
				if (outer == null)
					continue;
				Object[] rows = (Object[]) outer;
				int expect = -1;
				for (int i = 0; i < rows.length; i++) {
					if (rows[i] == null)
						continue; // 行未分配: 跳过 (条件性数据)
					int len = java.lang.reflect.Array.getLength(rows[i]);
					if (expect == -1)
						expect = len;
					else if (len != expect) {
						failed++;
						System.out.println("  FAIL: " + file + " 的 " + f.getName()
								+ " 非矩形 (行0长 " + expect + ", 行" + i + " 长 " + len + ")");
						return;
					}
				}
			} catch (IllegalAccessException e) {
				// public 字段不应发生
			}
		}
	}
}
