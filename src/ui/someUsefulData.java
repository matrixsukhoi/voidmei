package ui;

import java.util.Arrays;

import parser.blkx;
import prog.app;
import prog.controller;
import ui.overlay.BaseOverlay;

public class someUsefulData extends BaseOverlay implements BaseOverlay.ConfigBridge {
	/**
	 * 
	 */
	private static final long serialVersionUID = 285137206980711202L;
	controller xc;
	blkx xp;

	public void init(controller c, blkx p) {
		this.xc = c;
		this.xp = p;
		super.init(this, "someUsefulDataX", "someUsefulDataY", () -> {
			String currentText = xc.blkx != null ? xc.blkx.fmdata : "FM Data Preview\n[No Data Loaded]";
			return Arrays.asList(currentText.split("\n"));
		});
		// Set custom header matcher for FM data
		setHeaderMatcher(line -> line.contains("fm器件") || line.contains("FM文件"));
	}

	public void initPreview(controller c, blkx p) {
		this.xc = c;
		this.xp = p;
		super.initPreview(this, "someUsefulDataX", "someUsefulDataY", () -> {
			String currentText = xc.blkx != null ? xc.blkx.fmdata : "FM Data Preview\n[No Data Loaded]";
			return Arrays.asList(currentText.split("\n"));
		});
		setHeaderMatcher(line -> line.contains("fm器件") || line.contains("FM文件"));
	}

	public void reinitConfig(blkx p) {
		this.xp = p;
		setupFont();
		lastData = null; // 强制刷新
	}

	// --- BaseOverlay.ConfigBridge Implementation ---

	@Override
	public String getConfig(String key) {
		return xc.getconfig(key);
	}

	@Override
	public void setConfig(String key, String value) {
		xc.setconfig(key, value);
	}

	// --- BaseOverlay Overrides ---

	@Override
	protected boolean shouldExit() {
		if (app.displayFmCtrl) {
			return false;
		}
		// Null safety: xc.S or sState may not be initialized yet
		if (xc == null || xc.S == null || xc.S.sState == null) {
			return false;
		}
		// logic from original run() loop
		if (this.xc.S.sState.gear != 100 || (this.xc.S.speedv > 10 && this.xc.S.sState.throttle > 0)) {
			return true;
		}
		return false;
	}

	@Override
	protected int getRefreshInterval() {
		if (app.displayFmCtrl) {
			return 1000;
		}
		return 200;
	}

	@Override
	protected boolean isVisibleNow() {
		return app.displayFm;
	}
}