package prog.fm;

import java.io.File;

import prog.util.FMPowerExtractor;
import prog.util.Logger;
import prog.util.PistonPowerModel;

/**
 * 纯静态的 FM 加载器（P2 重构）—— 项目内未来唯一 new Blkx 的地方。
 *
 * <p>逻辑自旧 Controller.loadFMData 原样迁移：中央文件只读文本 → 提取燃油改装修正
 * 与 fmFile 字段 → 解析物理 FM 文件 → 全量解析（getAllplotdata + finalizeLoading）→
 * 按发动机类型提取增压器参数或峰值推力 → 产出不可变 {@link FMHandle}。
 *
 * <p>与旧实现的差异（均为死循环重构的关键）：
 * <ul>
 *   <li><b>全程 try{...}catch(Throwable)</b>：旧代码只 try 了物理文件构造，getAllplotdata/
 *       finalizeLoading 在 try 之外（P1 核验发现的第二条循环路径——那里抛异常会直接炸出
 *       loadFMData，失败状态记录不上，调用方下一轮又重试）。现在任何 Throwable（含 OOM，
 *       记日志后）一律收敛为 CORRUPT 句柄，进入 {@link FMManager} 负缓存，永不再试。</li>
 *   <li><b>不再 System.gc()</b>：loader 是低频后台线程，显式 gc 只是"建议"且在旧架构的
 *       每秒多次重载风暴下反而放大停顿；大 FM 结构的回收交给 JVM 自行决策。</li>
 *   <li><b>不持有任何状态</b>：失败记录（旧 failedFMName）由 FMManager 的负缓存承担，
 *       本类无副作用、可任意重入。</li>
 * </ul>
 */
public final class FMLoader {

	/** 白盒测试计数器：FMLoader.load 真正执行（进入加载流程）的次数 */
	private static volatile long loadCount = 0;

	private FMLoader() {
	}

	/** 白盒测试用：读取 load 执行计数 */
	public static long getLoadCount() {
		return loadCount;
	}

	/** 白盒测试用：清零计数 */
	public static void resetLoadCount() {
		loadCount = 0;
	}

	/**
	 * 加载指定机型的 FM 数据。任何一步失败都返回 MISSING/CORRUPT 句柄，绝不抛出、
	 * 绝不返回 null。
	 *
	 * @param planeName 机型名（任意大小写/空白，内部规范化）
	 * @return 加载结果句柄；name 为空时返回 UNRESOLVED
	 */
	public static FMHandle load(String planeName) {
		// 空名直接 UNRESOLVED（与 FMManager.identify 的空值守卫双保险）
		if (planeName == null || planeName.isEmpty()) {
			return FMHandle.UNRESOLVED;
		}
		final String name = planeName.toLowerCase().trim();
		loadCount++;

		// 全程兜底：见类 javadoc——任何异常（含 getAllplotdata/finalizeLoading 阶段）
		// 都收敛为 CORRUPT，交给 FMManager 负缓存，杜绝重试风暴
		try {
			// 1. 中央文件不存在 → 确认机型不在库 → MISSING
			File central = FMDataPaths.centralFile(name);
			if (!central.exists()) {
				return FMHandle.missing(name);
			}

			// 2. 只读解析中央文件（doLoad=false，不触发全量 FM 解析）
			parser.Blkx lookupBlkx = new parser.Blkx(central.getPath(), name + ".blk", false);

			// 3. 提取燃油改装修正（中央文件专属信息，物理文件里没有）
			parser.Blkx.FuelModification fuelMod = null;
			String fmfile = null;
			if (lookupBlkx.valid && lookupBlkx.data != null) {
				fuelMod = parser.Blkx.extractFuelModifications(lookupBlkx.data);
				if (fuelMod.type != parser.Blkx.FuelModification.FuelType.NONE) {
					Logger.info("FMLoader", "Fuel modification detected: " + fuelMod.type
							+ " (HP bonus=" + fuelMod.sovietOctaneHpBonus + ")");
				}

				// 4. 从中央文件取物理 FM 文件相对路径（fmFile:t = "fm/xxx.blk"）
				fmfile = lookupBlkx.getlastone("fmfile");
				if (fmfile != null) {
					// 剥首尾引号并去前导 '/'
					fmfile = fmfile.substring(fmfile.indexOf("\"") + 1, fmfile.length() - 1);
					if (fmfile.charAt(0) == '/')
						fmfile = fmfile.substring(1);
				}
			}
			if (fmfile == null) {
				// 中央文件里没写 fmFile → 按目录约定回退
				fmfile = "fm/" + name + ".blk";
			}
			if (-1 == fmfile.indexOf(".blk")) {
				fmfile += ".blk";
			}

			// 5. 全量解析物理 FM 文件（物理文件 = fmfile + "x"，即 .blkx）
			parser.Blkx blkx = new parser.Blkx(FMDataPaths.physicalFile(fmfile + "x").getPath(), fmfile);
			if (!blkx.valid) {
				// 中央文件在库但物理文件缺失/解析失败 → CORRUPT（数据不完整）
				Logger.warn("FMLoader", "FM文件不存在或解析失败: " + name);
				return FMHandle.corrupt(name);
			}

			// 6. plot 数据解析同样可能抛异常，必须留在 try 内（第二条循环路径）
			blkx.getAllplotdata();
			blkx.finalizeLoading();

			// 7. 按发动机类型提取派生数据（与旧 loadFMData 一致）
			if (FMPowerExtractor.isPistonEngine(blkx)) {
				PistonPowerModel.CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx, fuelMod);
				// 多发飞机乘引擎数（与喷气推力计算口径一致）
				double peakWep = PistonPowerModel.peakWepPower(stages) * blkx.engineNum;
				return FMHandle.ready(name, blkx, peakWep, 0, stages);
			} else {
				// 喷气机固定取加力峰值推力
				return FMHandle.ready(name, blkx, 0, blkx.peakThrust(true), null);
			}
		} catch (Throwable t) {
			// OOM 也一并捕获（记 ERROR 便于排查）：不允许异常炸穿 loader 线程导致任务队列停摆，
			// 统一收敛为 CORRUPT 句柄进负缓存
			Logger.error("FMLoader", "FM加载异常(" + name + "): " + t, t);
			return FMHandle.corrupt(name);
		}
	}
}
