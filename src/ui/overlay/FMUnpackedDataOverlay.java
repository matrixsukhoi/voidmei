package ui.overlay;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Consumer;

import parser.Blkx;
import ui.model.FMDataAdapter;

import prog.Controller;
import prog.config.ConfigProvider;
import prog.config.OverlaySettings;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;
import prog.i18n.Lang;
import prog.util.Logger;

import static prog.util.PhysicsConstants.g;

/**
 * Overlay for displaying FM (Flight Model) unpacked data.
 *
 * <p>Uses BaseOverlay with ZebraListRenderer for consistent visual styling.
 * Each FM data field can be individually enabled/disabled via ui_layout.cfg switches.
 *
 * <p>Visibility behavior:
 * - Preview mode: always visible
 * - Game mode: initially hidden, toggle via FM_OVERLAY_TOGGLE hotkey event
 */
public class FMUnpackedDataOverlay extends BaseOverlay {

    private static final long serialVersionUID = 1L;

    private Controller controller;
    private FMDataAdapter fmDataAdapter;
    protected OverlaySettings overlaySettings;
    protected ConfigProvider config;

    // Self-managed visibility state (game mode toggle)
    private boolean visible = true;
    private Consumer<Object> toggleHandler;
    private Consumer<Object> fmLoadedHandler;

    public FMUnpackedDataOverlay() {
        super();
    }

    /**
     * Initialize for game mode.
     *
     * @param c The controller
     * @param adapter The FM data adapter (wrapping Blkx)
     * @param settings Overlay settings from ui_layout.cfg
     */
    public void init(Controller c, FMDataAdapter adapter, OverlaySettings settings) {
        this.controller = c;
        this.fmDataAdapter = adapter;
        // 使用 getConfigProvider() 获取配置接口，而不是直接使用 Controller
        this.config = c.getConfigProvider();
        // 使用 setter 方法确保位置保存回调正确初始化
        setOverlaySettings(settings);
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

        // Subscribe to FM data reload event
        fmLoadedHandler = data -> reloadFMData();
        UIStateBus.getInstance().subscribe(UIStateEvents.FM_DATA_LOADED, fmLoadedHandler);

        // Set header matcher for styling (FM parts headers start with "------fm器件")
        setHeaderMatcher(line -> line.startsWith("FM文件") || line.startsWith("------fm器件"));

        // Initialize BaseOverlay with dynamic data supplier
        super.init(settings, this::generateLines);

        // Game mode starts hidden - toggle via hotkey
        setVisible(false);
    }

    /**
     * Initialize for preview mode.
     *
     * @param c The controller
     * @param adapter The FM data adapter
     * @param settings Overlay settings
     */
    public void initPreview(Controller c, FMDataAdapter adapter, OverlaySettings settings) {
        this.controller = c;
        this.fmDataAdapter = adapter;
        // 使用 getConfigProvider() 获取配置接口，而不是直接使用 Controller
        this.config = c.getConfigProvider();
        // 使用 setter 方法确保位置保存回调正确初始化
        setOverlaySettings(settings);
        this.isPreview = true;

        // Preview mode: always visible
        this.visible = true;

        Logger.info("FMUnpackedDataOverlay", "Initializing for preview mode (visible=" + visible + ")");

        // Set header matcher for styling (FM parts headers start with "------fm器件")
        setHeaderMatcher(line -> line.startsWith("FM文件") || line.startsWith("------fm器件"));

        // Initialize BaseOverlay with dynamic data supplier
        super.initPreview(settings, this::generateLines);
    }

    /**
     * Called when FM data is reloaded (e.g., aircraft change).
     */
    private void reloadFMData() {
        // Update adapter with latest Blkx if available
        if (controller != null) {
            fmDataAdapter.setBlkx(controller.getBlkx());
        }
        // Data will be refreshed on next run() cycle
    }

    /**
     * Reload configuration settings.
     * Called when config values change for WYSIWYG preview.
     */
    public void reinitConfig() {
        // Refresh adapter data
        if (controller != null && fmDataAdapter != null) {
            fmDataAdapter.setBlkx(controller.getBlkx());
        }
        // Font and display settings are handled by BaseOverlay
        setupFont();
    }

    /**
     * Generate display lines based on enabled fields in config.
     * Uses Lang.bXXX format strings to match the original Blkx.fmdata format.
     */
    private List<String> generateLines() {
        List<String> lines = new ArrayList<>();
        Blkx blkx = fmDataAdapter.getBlkx();

        if (blkx == null) {
            lines.add("FM Data Preview");
            lines.add("[No Data Loaded]");
            return lines;
        }

        // ==================== FM Version (always shown) ====================
        String fmVersion = String.format(Lang.bFmVersion, blkx.readFileName, blkx.version);
        addLines(lines, fmVersion);

        // ==================== Weight ====================
        if (isFieldEnabled("showWeight")) {
            String weight = String.format(Lang.bWeight, blkx.emptyweight, blkx.maxfuelweight);
            addLines(lines, weight);
        }

        // ==================== Critical Speed ====================
        if (isFieldEnabled("showCritSpeed")) {
            String critSpeed = String.format(Lang.bCritSpeed, blkx.CriticalSpeed * 3.6, blkx.vne);
            addLines(lines, critSpeed);
        }

        // ==================== G-Load Limits (combined full/half fuel) ====================
        if (isFieldEnabled("showGLoadLimits") && blkx.rawWingCritOverload != null) {
            double fullNeg = 1.2 * (2 * blkx.rawWingCritOverload[0] / (g * blkx.grossweight) + 1);
            double fullPos = 1.2 * (2 * blkx.rawWingCritOverload[1] / (g * blkx.grossweight) - 1);
            double halfNeg = 1.2 * (2 * blkx.rawWingCritOverload[0] / (g * blkx.halfweight) + 1);
            double halfPos = 1.2 * (2 * blkx.rawWingCritOverload[1] / (g * blkx.halfweight) - 1);
            String loadFactor = String.format(Lang.bAllowLoadFactor, fullNeg, fullPos, halfNeg, halfPos);
            addLines(lines, loadFactor);
        }

        // ==================== Flap Speed Limits ====================
        if (isFieldEnabled("showFlapLimits") && blkx.FlapsDestructionIndSpeed != null) {
            for (int i = 0; i < blkx.FlapsDestructionNum; i++) {
                String flapLimit = String.format(Lang.bFlapRestrict, i,
                    blkx.FlapsDestructionIndSpeed[i][0] * 100,
                    blkx.FlapsDestructionIndSpeed[i][1]);
                addLines(lines, flapLimit);
            }
        }

        // ==================== Control Surface Effectiveness (combined) ====================
        if (isFieldEnabled("showControlEffectiveness")) {
            String effSpeed = String.format(Lang.bEffSpeedAndPowerLoss,
                blkx.elavEff, blkx.aileronEff, blkx.rudderEff,
                blkx.elavPowerLoss, blkx.aileronPowerLoss, blkx.rudderPowerLoss);
            addLines(lines, effSpeed);
        }

        // ==================== Nitro (only if present) ====================
        if (isFieldEnabled("showNitro") && blkx.nitro > 0) {
            String nitro = String.format(Lang.bNitro, blkx.nitro, blkx.nitro / (blkx.nitroDecr * 60));
            addLines(lines, nitro);
        }

        // ==================== Heat Recovery ====================
        if (isFieldEnabled("showHeatRecovery")) {
            String heatRecovery = String.format(Lang.bAverageHeatRecovery, blkx.avgEngRecoveryRate);
            addLines(lines, heatRecovery);
        }

        // ==================== Max Lift Load ====================
        if (isFieldEnabled("showMaxLiftLoad")) {
            String maxLiftLoad = String.format(Lang.bMaxLiftLoad350,
                (blkx.NoFlapWLL + 1) / 2, (blkx.FullFlapWLL + 1) / 2);
            addLines(lines, maxLiftLoad);
        }

        // ==================== Inertia ====================
        if (isFieldEnabled("showInertia") && blkx.MomentOfInertia != null && blkx.MomentOfInertia.length >= 3) {
            String inertia = String.format(Lang.bInertia,
                blkx.MomentOfInertia[2], blkx.MomentOfInertia[0], blkx.MomentOfInertia[1]);
            addLines(lines, inertia);
        }

        // ==================== Lift Parameters ====================
        if (isFieldEnabled("showLift")) {
            String lift = String.format(Lang.bLift,
                blkx.AWing, blkx.AFuselage, blkx.NoFlapWLL, blkx.FullFlapWLL,
                blkx.OswaldsEfficiencyNumber, blkx.AspectRatio, blkx.SweptWingAngle);
            addLines(lines, lift);
        }

        // ==================== Drag Parameters ====================
        if (isFieldEnabled("showDrag")) {
            String drag = String.format(Lang.bDrag,
                blkx.CdS, blkx.CdS / (blkx.halfweight / 1000.0),
                blkx.indCdF, blkx.halfweight * blkx.indCdF,
                blkx.RadiatorCd, blkx.OilRadiatorCd);
            addLines(lines, drag);
        }

        // ==================== FM Parts Sections ====================
        if (isFieldEnabled("showNoFlapsWing")) {
            addFmParts(lines, blkx.NoFlapsWing);
        }
        if (isFieldEnabled("showFullFlapsWing")) {
            addFmParts(lines, blkx.FullFlapsWing);
        }
        if (isFieldEnabled("showFuselage")) {
            addFmParts(lines, blkx.Fuselage);
        }
        if (isFieldEnabled("showFin")) {
            addFmParts(lines, blkx.Fin);
        }
        if (isFieldEnabled("showStab")) {
            addFmParts(lines, blkx.Stab);
        }

        // If no fields are enabled or all filtered out, show a placeholder
        if (lines.isEmpty()) {
            lines.add("FM Data Preview");
            lines.add("[No Fields Enabled]");
        }

        return lines;
    }

    /**
     * Add FM parts section (header + 4 data lines).
     */
    private void addFmParts(List<String> lines, Blkx.fm_parts p) {
        if (p == null) return;
        addLines(lines, String.format(Lang.bFmParts, p.name));
        addLines(lines, String.format(Lang.bCdMin, p.CdMin));
        addLines(lines, String.format(Lang.bCl0, p.Cl0));
        addLines(lines, String.format(Lang.bAoACrit, p.AoACritLow, p.AoACritHigh));
        addLines(lines, String.format(Lang.bAoACritCl, p.ClCritLow, p.ClCritHigh));
    }

    /**
     * Add multiline format string output, splitting on newlines.
     * Trims each line and skips empty lines.
     */
    private void addLines(List<String> lines, String formatted) {
        for (String line : formatted.split("\n")) {
            String trimmed = line.trim();
            if (!trimmed.isEmpty()) {
                lines.add(trimmed);
            }
        }
    }

    /**
     * Check if a field is enabled in config.
     * Defaults to true if not configured.
     */
    private boolean isFieldEnabled(String fieldKey) {
        if (config == null) return true;
        String value = config.getConfig(fieldKey);
        if (value == null || value.isEmpty()) {
            return true; // Default to enabled
        }
        return Boolean.parseBoolean(value);
    }

    @Override
    protected boolean isVisibleNow() {
        return visible;
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
}
