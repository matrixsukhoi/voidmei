package ui.overlay;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;

import prog.Application;
import prog.Controller;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;
import parser.Blkx;

/**
 * Overlay for displaying FM Data description (formerly UsefulData).
 * Extends BaseOverlay as required by user.
 */
public class FMDataOverlay extends BaseOverlay implements BaseOverlay.ConfigBridge {

    private static final long serialVersionUID = 1L;
    private Controller controller;
    private List<String> cachedData = Collections.singletonList("FM Data Preview\n[No Data Loaded]");

    public FMDataOverlay() {
        super();
        // BaseOverlay uses ZebraListRenderer by default
    }

    public void init(Controller c) {
        this.controller = c;

        // Initialize BaseOverlay with a supplier that returns our cached data
        super.init(this, "UsefulDataX", "UsefulDataY", () -> {
            // If we want purely event driven, we return cachedData.
            // If we want to ensure freshness (polling), we can fetch here too.
            // But let's stick to event-driven update of cache + polling for display.
            return cachedData;
        });

        // Set header matcher for FM data styling
        setHeaderMatcher(line -> line.contains("fm器件") || line.contains("FM文件"));

        // Subscribe to FM Loaded event
        UIStateBus.getInstance().subscribe(UIStateEvents.FM_DATA_LOADED, this::onFMDataLoaded);

        // Initial load
        reloadFMData();
    }

    // Preview init signature to match expected usage?
    // Controller calls: ((FMDataOverlay) overlay).initPreview(this);
    public void initPreview(Controller c) {
        this.controller = c;
        this.isPreview = true; // BaseOverlay field

        super.initPreview(this, "UsefulDataX", "UsefulDataY", () -> cachedData);
        setHeaderMatcher(line -> line.contains("fm器件") || line.contains("FM文件"));

        reloadFMData();
    }

    private void onFMDataLoaded(Object planeName) {
        reloadFMData();
    }

    private synchronized void reloadFMData() {
        if (controller == null)
            return;
        Blkx b = controller.getBlkx();
        String text = (b != null && b.fmdata != null) ? b.fmdata : "FM Data Preview\n[No Data Loaded]";
        this.cachedData = Arrays.asList(text.split("\n"));
    }

    @Override
    public void dispose() {
        UIStateBus.getInstance().unsubscribe(UIStateEvents.FM_DATA_LOADED, this::onFMDataLoaded);
        super.dispose();
    }

    // --- ConfigBridge Implementation ---

    @Override
    public String getConfig(String key) {
        if (controller != null) {
            return controller.getconfig(key);
        }
        return null;
    }

    @Override
    public void setConfig(String key, String value) {
        if (controller != null) {
            controller.setconfig(key, value);
        }
    }

    // --- BaseOverlay Overrides ---

    @Override
    protected boolean shouldExit() {
        if (isPreview)
            return false;

        if (Application.displayFmCtrl) {
            return false;
        }
        // Null safety
        if (controller == null || controller.S == null || controller.S.sState == null) {
            return false;
        }
        // Logic from original UsefulData: Exit if taking off?
        // "if (this.xc.S.sState.gear != 100 || (this.xc.S.speedv > 10 &&
        // this.xc.S.sState.throttle > 0))"
        // This implies the overlay is meant to show ONLY on ground/prep?
        // Restoring exactly as authorized.
        if (controller.S.sState.gear != 100 || (controller.S.speedv > 10 && controller.S.sState.throttle > 0)) {
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
        // Use Application.displayFm flag validation
        return Application.displayFm;
    }

    public void reinitConfig() {
        // BaseOverlay setup
        setupFont();
        // Maybe reload data
        reloadFMData();
    }
}
