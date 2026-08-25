package prog.fm;

import java.io.File;

/**
 * FM 数据路径的唯一来源（P2 重构）。
 *
 * <p>此前 "./data/aces/gamedata/flightmodels/..." 字符串散落在 Controller.loadFMData、
 * Blkx.getVersion 等多处硬编码；本类集中管理，并为白盒测试提供 {@link #setDataRoot}
 * 注入点（测试可指向临时目录，不依赖真机 data/）。
 *
 * <p><b>扩展名统一小写 ".blkx"</b>：旧代码拼 ".Blkx"（大写 B），仅在 Windows
 * 大小写不敏感的文件系统上碰巧可用；fmdata 解包产物（wt_ext_cli --blk_extension blkx）
 * 与 build.py 均为小写，Linux/CI 下大写拼法会直接找不到文件。这里统一为小写。
 */
public final class FMDataPaths {

	/** FM 数据根目录；volatile 供测试运行时注入临时目录 */
	private static volatile String dataRoot = "./data";

	private FMDataPaths() {
	}

	/** FM 数据根目录（默认 "./data"，与程序工作区约定一致） */
	public static String getDataRoot() {
		return dataRoot;
	}

	/**
	 * 注入数据根目录（白盒测试用）。传相对/绝对路径均可，
	 * 后续所有路径拼装以最新值为准。
	 */
	public static void setDataRoot(String root) {
		dataRoot = root;
	}

	/** flightmodels 目录：&lt;root&gt;/aces/gamedata/flightmodels */
	public static File fmDir() {
		return new File(dataRoot, "aces/gamedata/flightmodels");
	}

	/**
	 * 中央文件（机型入口文件）路径：
	 * &lt;root&gt;/aces/gamedata/flightmodels/&lt;name 小写&gt;.blkx。
	 * 机型名做小写规范化（大小写不敏感匹配游戏侧命名）。
	 */
	public static File centralFile(String planeName) {
		return new File(fmDir(), planeName.toLowerCase() + ".blkx");
	}

	/**
	 * 物理 FM 文件路径。{@code fmFileWithX} 为中央文件 fmFile 字段解析出的相对路径
	 * 再补 "x"（形如 "fm/spitfire_f24.blkx"）——与 FMLoader 的调用约定一致。
	 */
	public static File physicalFile(String fmFileWithX) {
		return new File(fmDir(), fmFileWithX);
	}

	/** FM 数据版本文件：&lt;root&gt;/aces/version（Blkx.getVersion 展示用） */
	public static File versionFile() {
		return new File(dataRoot, "aces/version");
	}
}
