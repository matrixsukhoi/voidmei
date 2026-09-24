package prog.fm;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;

import java.util.concurrent.atomic.AtomicBoolean;

import prog.Application;
import prog.Controller;
import prog.i18n.Lang;
import prog.util.FileUtils;
import prog.util.HttpHelper;
import prog.util.Logger;
import ui.util.NotificationService;
import ui.util.Toast;

/**
 * FM 数据在线自动更新：检查→下载→校验→解压→替换→生效 一条链（后台线程执行）。
 *
 * <p>云端真相源是本仓库 data prerelease 的 release 直链（manifest 是 zip 文件名的
 * 单一真相源，与 CI 语义一致；prerelease 附件公开匿名可下，且零 api.github.com 配额）。
 * 更新条件：云端 wt_version &gt; 本地 data/aces/version（四段语义比较，防降级）。
 *
 * <p>进度 UI：auto 与 manual 共用 {@link ProgressToast}（右下角、不抢焦点），
 * 下载显示百分比，校验/解压/替换为不确定进度；finally 必销毁。
 * 失败策略：仅 Logger 静默 + 温和 toast，绝不弹错误对话框；下次启动再试。
 *
 * <p>系统属性（开发/e2e 用）：{@code -Dvoidmei.fmdata.update=force} 无视版本相等
 * 强制整链；{@code =off} 硬关（e2e 防真连网）。
 */
public final class FMDataUpdater {

	private static final FMDataUpdater INSTANCE = new FMDataUpdater();

	public static FMDataUpdater getInstance() {
		return INSTANCE;
	}

	private FMDataUpdater() {
	}

	private static final String COMP = "FMDataUpdater";

	/** release 直链前缀（owner/repository 与 checkUpdate 同源） */
	public static final String CLOUD_BASE = "https://github.com/" + Application.owner + "/"
			+ Application.repository + "/releases/download/data/";

	private static final int MANIFEST_CONNECT_MS = 10_000;
	private static final int MANIFEST_READ_MS = 15_000;
	/** zip 下载 read 超时按块计时：慢链路 100KB/s 也能下完 18.6MB */
	private static final int DL_CONNECT_MS = 10_000;
	private static final int DL_READ_MS = 60_000;
	/** data→data_old rename 重试：撞上 FMLoader 瞬时打开文件的窗口 */
	private static final int RENAME_ATTEMPTS = 5;
	private static final long RENAME_DELAY_MS = 500;

	/** 防重入：启动检查与手动按钮可能并发，同时至多一个更新周期 */
	private final AtomicBoolean running = new AtomicBoolean(false);
	/** 用户取消标志（进度通知上"禁止自动更新"按钮置位; 下载回调检查并中止） */
	private volatile boolean cancelled = false;
	/** 进行中收到手动请求时排队：当前周期结束补跑一次 manual 周期（带结果反馈） */
	private volatile boolean pendingManual = false;
	/** 排队请求的完成回调（按钮保持"正在检查..."直到补跑结束）；单槽，重复点击覆盖 */
	private volatile Runnable pendingOnDone = null;

	/** 是否有更新周期进行中（含排队补跑段）。设置面板渲染按钮初始状态用 */
	public boolean isRunning() {
		return running.get();
	}

	/**
	 * 异步执行一次更新周期（提交 Application.threadPool，任意线程可调）。
	 *
	 * @param manual true=手动按钮入口（跳过 autoUpdateFmData 开关，反馈"已最新"）
	 * @param onDone 周期结束回调（无论成败；供按钮恢复禁用态），可为 null。
	 *               进行中被吞的手动请求：onDone 挂到补跑周期结束才执行——按钮全程保持反馈
	 */
	public void checkAndApplyAsync(final boolean manual, final Runnable onDone) {
		if (running.getAndSet(true)) {
			if (manual) {
				// 手动请求排队到周期尾补跑（曾直接忽略——启动 auto 周期被吞的第一次点击
				// 无任何反馈，用户以为按钮失灵）。补跑在 running=true 期间执行，不产生并发周期
				pendingManual = true;
				pendingOnDone = onDone;
			} else if (onDone != null) {
				onDone.run();
			}
			return;
		}
		Application.threadPool.submit(() -> {
			try {
				runUpdateCycle(CLOUD_BASE, manual);
				// 补跑排队的手动请求（手动点击落在自动周期内被排队的场景）
				while (pendingManual) {
					pendingManual = false;
					Runnable cb = pendingOnDone;
					pendingOnDone = null;
					runUpdateCycle(CLOUD_BASE, true);
					if (cb != null)
						cb.run();
				}
			} finally {
				running.set(false);
				if (onDone != null)
					onDone.run();
			}
		});
	}

	/**
	 * 同步执行一次更新周期。baseUrl 参数化供白盒测试（本地 HttpServer 整链回归）。
	 */
	public void runUpdateCycle(String baseUrl, boolean manual) {
		if ("off".equals(System.getProperty("voidmei.fmdata.update")))
			return;
		if (!manual && !autoUpdateEnabled())
			return;
		cancelled = false; // 上个周期的取消标志不能拖进本周期 (否则手动检查秒中止)

		File dataDir = new File(FMDataPaths.getDataRoot());
		File staging = new File(dataDir.getParentFile(), "data_staging");
		File backup = new File(dataDir.getParentFile(), "data_old");
		// 数组包装: lambda 回调需捕获, 而句柄在 try 中段才创建
		final Toast.Progress[] toast = { null };
		// 下载进度日志: 上次打印时刻 (按时间间隔打, 网速快自然少打/慢自然多打; 数组供回调捕获)
		final long[] lastLogMs = { 0 };
		final long cycleStart = System.currentTimeMillis();
		final long[] downloadStart = { 0 };
		// 进度窗出现过才发失败 toast：manifest 拉不到（最常见=断网）完全静默，
		// 下载中途断掉才告知（用户看着进度条消失无解释更困惑）
		boolean downloadStarted = false;
		try {
			cleanResidue(staging, backup);

			// 1. 检查：拉 manifest 比版本
			Logger.info(COMP, "开始检查 FM 数据更新: " + baseUrl + "data_manifest.json");
			String json = new HttpHelper().sendGetURL(baseUrl + "data_manifest.json",
					MANIFEST_CONNECT_MS, MANIFEST_READ_MS);
			FMDataManifest m = FMDataManifest.parse(json);
			if (m == null) {
				Logger.warn(COMP, "manifest 缺失或解析失败，跳过本次检查");
				return;
			}
			String local = readLocalVersion();
			boolean force = "force".equals(System.getProperty("voidmei.fmdata.update"));
			if (!force && local != null && FMDataManifest.compareVersion(m.wtVersion, local) <= 0) {
				Logger.info(COMP, "FM 数据已是最新: " + m.wtVersion + " (本地 " + local + ")");
				if (manual) {
					// 带上发布日期与距今天数, 用户直接看出数据多旧
					String dd = displayDate(m.date);
					String rel = relativeDays(m.date);
					String extra = dd != null ? ", " + dd + " 发布" + (rel != null ? ", " + rel : "") : "";
					NotificationService.showBottomRight(
							String.format(Lang.fmDataCheckUpToDate, m.wtVersion + extra), 4000);
				}
				return;
			}
			Logger.info(COMP, "FM 数据有更新: " + (local == null ? "(缺失)" : local) + " -> " + m.wtVersion);

			// 2. 下载（进度窗出现; 附"转后台/禁止自动更新"动作）
			downloadStarted = true;
			downloadStart[0] = System.currentTimeMillis();
			staging.mkdirs();
			Logger.info(COMP, "开始下载 FM 数据包: " + m.zipName);
			toast[0] = Toast.showProgress(String.format(Lang.fmDataDownloadProgress, 0),
					new Toast.Action(Lang.fmDataActionBackground, () -> toast[0].dismiss()),
					new Toast.Action(Lang.fmDataActionDisable, () -> cancelAndDisableAuto()));
			File zipFile = new File(staging, m.zipName);
			new HttpHelper().downloadToFile(baseUrl + m.zipName, zipFile, DL_CONNECT_MS, DL_READ_MS,
					(done, total) -> {
						// 用户点了"禁止自动更新": 从下载循环内部中止 (异常经 downloadToFile 删半成品后上抛)
						if (cancelled)
							throw new IllegalStateException("用户取消");
						toast[0].update(
								String.format(Lang.fmDataDownloadProgress, total > 0 ? (int) (done * 100 / total) : 0),
								done, total);
						// 进度日志: 按时间间隔打 (5s 一条, 自适应网速——快网两三条, 慢网持续有心跳)
						long now = System.currentTimeMillis();
						if (now - lastLogMs[0] >= 5_000) {
							lastLogMs[0] = now;
							int pct = total > 0 ? (int) (done * 100 / total) : -1;
							Logger.info(COMP, String.format("下载进度 %d%% (%.1f/%.1f MB)", pct,
									done / 1048576.0, total / 1048576.0));
						}
					});
			Logger.info(COMP, String.format("下载完成 (%.1f MB)", zipFile.length() / 1048576.0));

			// 3. 校验 sha256（防传输损坏/半截文件）
			toast[0].stage(Lang.fmDataInstallStage);
			String sha = FileUtils.sha256Hex(zipFile);
			if (sha == null || !sha.equalsIgnoreCase(m.sha256)) {
				Logger.warn(COMP, "sha256 校验失败: local=" + sha + " remote=" + m.sha256);
				notifyFailed();
				return;
			}
			Logger.info(COMP, "sha256 校验通过");

			// 4. 解压（zip 顶层即 data/；zip-slip/炸弹防护在 FileUtils.unzip）
			File unpacked = new File(staging, "unpacked");
			FileUtils.unzip(zipFile, unpacked);
			File newData = new File(unpacked, "data");
			if (!newData.isDirectory()) {
				Logger.warn(COMP, "zip 内容异常: 顶层缺 data/ 目录");
				notifyFailed();
				return;
			}

			// 5. 替换（rename 交换，失败保旧 data）
			if (!replaceDataDir(dataDir, newData, backup)) {
				notifyFailed();
				return;
			}

			// 6. 生效：清负缓存强制重载当前目标，FM_CHANGED 自动广播，无需重启
			FMManager.getInstance().dataUpdated();
			// date 兜底落盘: 打包日期只在 manifest 里 (旧版 zip 内无 date 文件),
			// 写进 data/aces/date 供设置面板离线显示
			if (m.date != null) {
				writeTextFile(new File(dataDir, "aces/date"), m.date);
			}
			Logger.info(COMP, "FM 数据更新完成: " + m.wtVersion + String.format(
					" (下载 %.1fs, 总耗时 %.1fs)",
					(System.currentTimeMillis() - downloadStart[0]) / 1000.0,
					(System.currentTimeMillis() - cycleStart) / 1000.0));
			NotificationService.showBottomRight(
					String.format(Lang.fmDataUpdateToast, m.wtVersion), 6000);
		} catch (Exception e) {
			if (cancelled) {
				// 用户主动取消: 不是失败, 不发失败 toast (取消确认已在按钮回调里发过)
				Logger.info(COMP, "FM 数据更新已取消 (用户关闭自动更新)");
			} else {
				Logger.warn(COMP, "FM 数据更新失败: " + e);
				// manual 必须有反馈 (点了按钮毫无反应是坏体验); auto 仅在下载已开始后才
				// 提示——启动断网是常态, 每次弹失败 toast 是骚扰
				if (downloadStarted || manual)
					notifyFailed();
			}
		} finally {
			if (toast[0] != null)
				toast[0].close();
			// staging 只含可再生产物，删除零风险；backup 已在 replaceDataDir 内处理
			FileUtils.deleteRecursively(staging);
		}
	}

	private void notifyFailed() {
		NotificationService.showBottomRight(Lang.fmDataCheckFailed, 6000);
	}

	/**
	 * 进度通知上"禁止自动更新"按钮：中止本次下载 + 关掉 autoUpdateFmData 开关
	 * （设置面板开关经 CONFIG_CHANGED 同步）+ 确认提示。EDT 调用。
	 */
	public void cancelAndDisableAuto() {
		cancelled = true;
		Controller c = Application.ctr;
		if (c != null && c.configService != null) {
			c.configService.setConfig("autoUpdateFmData", "false");
		}
		NotificationService.showBottomRight(Lang.fmDataAutoDisabled, 5000);
	}

	/** 本地 FM 数据版本（data/aces/version）；缺失/空返回 null（按"需要更新"处理）。供设置面板版本信息行复用 */
	public static String readLocalVersion() {
		return readTrimmed(FMDataPaths.versionFile());
	}

	/** 本地 FM 数据打包日期（data/aces/date，yyyymmdd）；缺失返回 null */
	public static String readLocalDate() {
		return readTrimmed(FMDataPaths.dateFile());
	}

	/** yyyymmdd → "2026-09-23"；空/长度非 8 返回 null */
	public static String displayDate(String d) {
		if (d == null || d.length() != 8)
			return null;
		return d.substring(0, 4) + "-" + d.substring(4, 6) + "-" + d.substring(6, 8);
	}

	/** yyyymmdd → "今天"/"N 天前"（相对当前日期）；空/非法返回 null */
	public static String relativeDays(String d) {
		if (d == null || d.length() != 8)
			return null;
		try {
			long days = java.time.temporal.ChronoUnit.DAYS.between(
					java.time.LocalDate.parse(d, java.time.format.DateTimeFormatter.BASIC_ISO_DATE),
					java.time.LocalDate.now());
			return days <= 0 ? "今天" : days + " 天前";
		} catch (java.time.format.DateTimeParseException e) {
			return null;
		}
	}

	private static String readTrimmed(File f) {
		try {
			String v = new String(Files.readAllBytes(f.toPath()), "UTF-8").trim();
			return v.isEmpty() ? null : v;
		} catch (IOException e) {
			return null;
		}
	}

	private static void writeTextFile(File f, String content) {
		try {
			Files.write(f.toPath(), (content + "\n").getBytes("UTF-8"));
		} catch (IOException e) {
			Logger.warn(COMP, "date 文件写入失败(不影响更新): " + f);
		}
	}

	/**
	 * 开关读取。坑：getConfig 缺键返回 ""，既有先例把空当 false——本键默认开，
	 * 语义相反，必须只有显式 "false" 才视为关。
	 */
	private static boolean autoUpdateEnabled() {
		Controller c = Application.ctr;
		if (c == null || c.configService == null)
			return true; // 冷启动极早期按默认
		return !"false".equals(c.configService.getConfig("autoUpdateFmData"));
	}

	/** 入口清理：上次残留的 staging 与 data_old（data_old 删不掉说明有文件被占，留待下次） */
	private static void cleanResidue(File staging, File backup) {
		if (staging.exists() && !FileUtils.deleteRecursively(staging))
			Logger.warn(COMP, "data_staging 残留未清: " + staging);
		if (backup.exists() && !FileUtils.deleteRecursively(backup))
			Logger.warn(COMP, "data_old 残留未清(文件被占?), 下次启动再清理");
	}

	/**
	 * 原子性替换 data 目录（rename 交换）：
	 * data→data_old（重试）→ 新目录→data（失败回滚）→ 删 data_old。
	 * 返回 false 时旧 data 保证原封不动（或已回滚）。
	 * public 供白盒测试 (TestFMDataUpdate)。
	 */
	public static boolean replaceDataDir(File dataDir, File newData, File backup) {
		if (dataDir.exists()
				&& !FileUtils.renameWithRetry(dataDir, backup, RENAME_ATTEMPTS, RENAME_DELAY_MS)) {
			Logger.warn(COMP, "旧 data 目录 rename 失败(被占用?), 放弃本次替换");
			return false;
		}
		if (!newData.renameTo(dataDir)) {
			Logger.warn(COMP, "新 data 就位失败, 回滚旧数据");
			if (backup.exists())
				backup.renameTo(dataDir); // 回滚尽力而为
			return false;
		}
		if (backup.exists() && !FileUtils.deleteRecursively(backup))
			Logger.warn(COMP, "data_old 残留待下次启动清理");
		return true;
	}
}
