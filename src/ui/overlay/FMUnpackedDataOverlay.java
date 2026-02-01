package ui.overlay;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.function.Consumer;

import ui.layout.renderer.FMListRowRenderer;

import prog.Application;
import prog.Controller;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;
import prog.util.Logger;
import parser.Blkx;

import prog.config.OverlaySettings;

/**
 * Overlay for displaying FM Data description (formerly UsefulData).
 * Extends BaseOverlay as required by user.
 *
 * Visibility is self-managed:
 * - Preview mode: always visible
 * - Game mode: initially hidden, toggle via FM_OVERLAY_TOGGLE event
 */
public class FMUnpackedDataOverlay extends BaseOverlay {

    private static final long serialVersionUID = 1L;
    private FMListRowRenderer fmSelector0;
    private Controller controller;
    private List<String> cachedData = Collections.singletonList("FM Data Preview\n[No Data Loaded]");

    // Self-managed visibility state
    private boolean visible = true;
    private Consumer<Object> toggleHandler;
    private Consumer<Object> fmLoadedHandler;

    public void init(Controller c, OverlaySettings settings) {
        this.controller = c;
        this.settings = settings;
        this.isPreview = false;

        // Game mode: initially hidden
        this.visible = false;

        Logger.info("FMUnpackedDataOverlay", "Initializing for game mode (visible=" + visible + ")");

        // Subscribe to toggle event
        toggleHandler = data -> {
            this.visible = !this.visible;
            Logger.debug("FMUnpackedDataOverlay", "Toggled visible: " + visible);
        };
        UIStateBus.getInstance().subscribe(UIStateEvents.FM_OVERLAY_TOGGLE, toggleHandler);

        // Initialize BaseOverlay with settings and data supplier
        super.init(settings, () -> cachedData);

        // Set header matcher for FM data styling
        setHeaderMatcher(line -> line.contains("fm器件") || line.contains("FM文件"));

        // Subscribe to FM Loaded event
        fmLoadedHandler = this::onFMDataLoaded;
        UIStateBus.getInstance().subscribe(UIStateEvents.FM_DATA_LOADED, fmLoadedHandler);

        // Initial load
        reloadFMData();
    }

    public void initPreview(Controller c, OverlaySettings settings) {
        this.controller = c;
        this.settings = settings;
        this.isPreview = true;

        // Preview mode: always visible, no toggle subscription
        this.visible = true;

        Logger.info("FMUnpackedDataOverlay", "Initializing for preview mode (visible=" + visible + ")");

        super.initPreview(settings, () -> cachedData);
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
        if (toggleHandler != null) {
            UIStateBus.getInstance().unsubscribe(UIStateEvents.FM_OVERLAY_TOGGLE, toggleHandler);
            toggleHandler = null;
        }
        if (fmLoadedHandler != null) {
            UIStateBus.getInstance().unsubscribe(UIStateEvents.FM_DATA_LOADED, fmLoadedHandler);
            fmLoadedHandler = null;
        }
        super.dispose();
    }

    // --- BaseOverlay Overrides ---
    @Override
    protected boolean isVisibleNow() {
        return visible;
    }

    public void reinitConfig() {
        // BaseOverlay setup
        setupFont();
        // Maybe reload data
        reloadFMData();
    }
}
