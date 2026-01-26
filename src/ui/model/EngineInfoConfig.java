package ui.model;

import java.util.ArrayList;
import java.util.List;

import prog.config.ConfigProvider;

/**
 * Configuration for EngineInfo overlay.
 * Replaces hardcoded defaults in EngineInfo.
 */
public class EngineInfoConfig {

    private List<FieldDefinition> fieldDefinitions = new ArrayList<>();

    public String title = "EngineInfo";

    // Style configuration
    public boolean showEdge = false;
    public int columnNum = 2; // Default 2 columns for EngineInfo

    // Font configuration keys
    public String numFontKey = "GlobalNumFont";
    public String labelFontKey = "fontName"; // Was 'engineInfoFont' in old config, mapped to 'fontName' in ui_layout
    public String fontAddKey = "fontSize"; // Was 'engineInfoFontadd', mapped to 'fontSize'
    public String columnKey = "columns"; // Was 'engineInfoColumn', mapped to 'columns'

    // Position keys
    public String posXKey = "engineInfoX";
    public String posYKey = "engineInfoY";

    // Edge style key
    // ui_layout.cfg doesn't seem to have a specific edge key for EngineInfo in the
    // default layout provided?
    // Checking ui_layout.cfg... [引擎信息] section has: Font, FontSize, Columns.
    // FlightInfo had 'flightInfoEdge'.
    // EngineInfo seems to lack a specific edge switch in default layout, but we can
    // define one for future or consistency.
    // Let's assume standard key or default false.
    public String edgeKey = "engineInfoEdge";

    // Layout Config
    public prog.config.ConfigLoader.GroupConfig groupConfig;

    public List<FieldDefinition> getFieldDefinitions() {
        return fieldDefinitions;
    }

    public void addFieldDefinition(String key, String label, String unit, String configKey, boolean hideWhenNA,
            boolean hideWhenZero, String exampleValue) {
        fieldDefinitions.add(new FieldDefinition(key, label, unit, configKey, hideWhenNA, hideWhenZero, exampleValue));
    }

    public void addFieldDefinition(String key, String label, String unit, String configKey, boolean hideWhenNA,
            String exampleValue) {
        addFieldDefinition(key, label, unit, configKey, hideWhenNA, false, exampleValue);
    }

    public static EngineInfoConfig createDefault(ConfigProvider config,
            prog.config.ConfigLoader.GroupConfig groupConfig) {
        EngineInfoConfig cfg = new EngineInfoConfig();
        cfg.groupConfig = groupConfig;

        // NOTE: EngineInfo in ui_layout.cfg uses "fontName", "fontSize", "columns" as
        // keys
        // because they are generic within the [Engine Info] group.
        // FieldOverlay's reinitConfig uses these keys to fetch from the passed
        // ConfigProvider (which is usually the GroupConfig wrapper).
        // However, FieldOverlay expects unique keys if fetching from Global config.
        // BUT, Reference to `FlightFactory` usage implies `ConfigProvider` is the
        // GroupConfig.
        // Let's verify `FlightInfo` usage. `FlightInfo` uses specific keys like
        // `flightInfoFontC`.
        // EngineInfo's `ui_layout.cfg` section uses generic keys.
        // If `ConfigProvider` passed to `init` is the `GroupConfig` (wrapped), then
        // `getConfig("fontName")` works.
        // In `Controller`, `registerWithPreview` passes `new EngineInfo()`.
        // `EngineInfo.init` receives `Controller` which IS a `ConfigProvider`.
        // BUT `Controller.getConfig` fetches global properties.
        // `EngineInfo` currently does: `xc.getconfig("engineInfoFont")` which implies
        // global keys were used in legacy.
        // BUT `ConfigLoader` loads `ui_layout.cfg`.
        // If we use `FieldOverlay`, we need to ensure keys match.

        // Refactoring Decision:
        // Use the Keys defined in `ui_layout.cfg` [Engine Info] section if possible?
        // Actually, `FieldOverlay` uses `renderContext =
        // RenderContext.fromConfig(...)`.
        // If we want to use the generic keys defined in `ui_layout.cfg` (like
        // `Font=...`),
        // we might need to update how `RenderContext` works or map them.
        // `EngineInfo` old code: `xc.getconfig("engineInfoFont")`.
        // ConfigurationService DOES NOT seem to have flattened keys for dynamic groups
        // unless `bind` is used?
        // Wait, `Controller`'s `dynamicConfigs` list contains `GroupConfig` objects
        // with `rows`.

        // Let's look at `FlightInfoConfig` again.
        // `cfg.labelFontKey = "flightInfoFontC";`
        // `ui_layout.cfg`: `字体 || COMBO:flightInfoFontC:_FONTS_ || %s || Sarasa Mono
        // SC`
        // So `FlightInfo` uses explicit global keys.

        // `EngineInfo` section in `ui_layout.cfg`:
        // `字体 || COMBO:fontName:_FONTS_ || %s || Sarasa Mono SC`
        // It uses `fontName`.
        // So `cfg.labelFontKey = "fontName";` is correct.

        if (config != null) {
            // EngineInfo doesn't seem to have an edge switch config in the layout provided,
            // default false.
            // But we can check "engineInfoEdge" just in case legacy exists.
            if ("true".equals(config.getConfig("engineInfoEdge"))) {
                cfg.showEdge = true;
            }

            String colStr = config.getConfig("columns"); // Key in ui_layout [EngineInfo] section
            if (colStr != null && !colStr.isEmpty()) {
                try {
                    cfg.columnNum = Integer.parseInt(colStr);
                } catch (NumberFormatException e) {
                    cfg.columnNum = 2;
                }
            }
        }

        // Add fields matching Service.java updates
        // label comes from Lang class
        // configKey is for visibility check (e.g. "HorsePower")
        // Note: FieldOverlay uses the configKey to check "disable"+Key or just
        // Key=false?
        // FlightInfo: `disableFlightInfoIAS`.
        // EngineInfo legacy: `disableEngineInfoHorsePower`.
        // But `ui_layout.cfg` has: `功率 || S.sTotalHp || %s Hp || true`.
        // It uses `S.sTotalHp` as the config Key for visibility in `ConfigLoader`? No,
        // RowConfig has `visible`.
        // `FieldOverlay` uses `FieldDefinition.configKey`.
        // `FieldManager` checks `config.getConfig(def.configKey)`.

        // CRITICAL DIFFERENCE:
        // `FlightInfo` in `ui_layout.cfg`: `示空速_IAS || SWITCH_INV:disableFlightInfoIAS
        // || %s || true`
        // The visibility is controlled by a SWITCH that sets `disableFlightInfoIAS`.

        // `EngineInfo` in `ui_layout.cfg`: `功率 || S.sTotalHp || %s Hp || true`
        // Result: It is a DATA row. `visible` is set directly in the row.
        // It DOES NOT use a separate switch key like `disableEngineInfo...`.
        // The old `EngineInfo` check `isFieldEnabledFromConfig` looked up the row by
        // label.

        // If we convert to `FieldOverlay`, `FieldManager` expects a config key to check
        // visibility.
        // If we provide a config key that doesn't exist (because it's just a row
        // visibility), it might fail?
        // `DefaultFieldManager` checks:
        // `String val = config.getConfig(configKey);`
        // If `ui_layout.cfg` rows are loaded, they are in `GroupConfig.rows`.
        // `Controller` (ConfigProvider) might not expose these row visibilities as flat
        // keys!

        // `FlightInfo` works because it has explicit switches defined in
        // `ui_layout.cfg`.
        // `EngineInfo` DOES NOT have switches for individual items in `ui_layout.cfg`.
        // It simply lists them.

        // To make `FieldOverlay` work for `EngineInfo`, we might need to:
        // 1. Update `ui_layout.cfg` to add switches? (User didnt ask for file change)
        // 2. Or, make `EngineInfoConfig` or `FieldManager` smarter to lookup row
        // visibility?

        // The User asked to refactor `EngineInfo` to "Reference flightinfo".
        // `FlightInfo` works because of `disableFlightInfo...` switches.
        // `EngineInfo` legacy: checked `disableEngineInfoHorsePower` (legacy) OR row
        // visibility.

        // If we stick to `FieldOverlay`, `FieldManager` uses `configProvider`.
        // We can create a `RowVisibilityConfigProvider` adapter?

        // Let's define the config keys as "engineInfoSwitch" (the Group switch) or
        // similar? No.
        // We map `HorsePower` -> `disableEngineInfoHorsePower`?
        // Does `ui_layout.cfg` have these switches?
        // [Engine Info] section:
        // `功率 || S.sTotalHp || %s Hp || true`
        // No explicit switch.

        // Wait! `EngineInfo` legacy code:
        // `isFieldEnabledFromConfig` iterates `GroupConfig.rows` and finds matching
        // label.

        // Strategy:
        // `EngineInfo` (new) will extend `FieldOverlay`.
        // We override `initFields` or use a custom `FieldManager` that knows how to
        // check RowConfig visibility!

        // Or, better: `EngineInfoConfig` can populate `FieldDefinition` with `visible`
        // defaults based on `GroupConfig`!
        // But `GroupConfig` is dynamic (loaded from file).

        // Let's pass the `GroupConfig` to `EngineInfoConfig.createDefault`?
        if (groupConfig != null) {
            cfg.populateFromGroup(groupConfig.rows);
        }

        // Actually, `FlightInfo` setup:
        // `overlayManager.registerWithPreview(..., () -> new FlightInfo(), ...)`
        // `FlightInfo.init(..., FlightInfoConfig.createDefault(this))`

        // If we want `FieldOverlay` to verify visibility against `ui_layout.cfg` rows
        // dynamically:
        // `FieldOverlay.onFlightData` -> `fieldManager.updateField`.
        // It checks `field.visible`.
        // `FieldManager` initialization sets `field.visible`.
        // We need updates when config changes. `reinitConfig` calls `initFields`.

        // `DefaultFieldManager.addField` logic:
        // `boolean isVisible = !hideWhenNA;` (initial)
        // Then `FieldManager` doesn't strictly check configKey for visibility
        // constantly?
        // `DefaultFieldManager` constructor takes `ConfigProvider`.
        // It has `updateVisibility`?

        // Let's check `DefaultFieldManager`.

        return cfg;
    }

    private void populateFromGroup(List<prog.config.ConfigLoader.RowConfig> rows) {
        if (rows == null)
            return;
        for (prog.config.ConfigLoader.RowConfig row : rows) {
            if ("DATA".equals(row.type) && row.property != null && !row.property.isEmpty()) {
                // Use targetName if provided, otherwise fallback to label
                String displayLabel = (row.targetName != null && !row.targetName.isEmpty()) ? row.targetName
                        : row.label;
                String defVal = row.previewValue != null ? row.previewValue : "0";
                addFieldDefinition(row.property, displayLabel, row.unit, row.property, true, row.hideWhenZero, defVal);
            }
            if (row.children != null) {
                populateFromGroup(row.children);
            }
        }
    }
}
