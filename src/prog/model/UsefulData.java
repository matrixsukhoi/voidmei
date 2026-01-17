package prog.model;

import java.util.Arrays;

import parser.Blkx;
import prog.Application;
import prog.Controller;
import ui.overlay.BaseOverlay;

public class UsefulData extends BaseOverlay implements BaseOverlay.ConfigBridge {
    /**
     * 
     */
    private static final long serialVersionUID = 285137206980711202L;
    Controller xc;
    Blkx xp;

    public void init(Controller c, Blkx p) {
        this.xc = c;
        this.xp = p;
        super.init(this, "UsefulDataX", "UsefulDataY", () -> {
            parser.Blkx b = xc.getBlkx();
            String currentText = b != null ? b.fmdata : "FM Data Preview\n[No Data Loaded]";
            return Arrays.asList(currentText.split("\n"));
        });
        // Set custom header matcher for FM data
        setHeaderMatcher(line -> line.contains("fm器件") || line.contains("FM文件"));
    }

    public void initPreview(Controller c, Blkx p) {
        this.xc = c;
        this.xp = p;
        super.initPreview(this, "UsefulDataX", "UsefulDataY", () -> {
            parser.Blkx b = xc.getBlkx();
            String currentText = b != null ? b.fmdata : "FM Data Preview\n[No Data Loaded]";
            return Arrays.asList(currentText.split("\n"));
        });
        setHeaderMatcher(line -> line.contains("fm器件") || line.contains("FM文件"));
    }

    public void reinitConfig(Blkx p) {
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
        if (Application.displayFmCtrl) {
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
        if (Application.displayFmCtrl) {
            return 1000;
        }
        return 200;
    }

    @Override
    protected boolean isVisibleNow() {
        return Application.displayFm;
    }
}
