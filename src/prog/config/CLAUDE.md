# Config System Development Guide

## Architecture Overview

The configuration system follows a **three-layer architecture**:

```
ui_layout.cfg (S-expression DSL)
        ↓ ConfigLoader.loadConfig()
GroupConfig / RowConfig (In-Memory Model)
        ↓ ConfigurationService
Settings Interfaces (HUDSettings, OverlaySettings)
```

## File Overview

| File | Lines | Responsibility |
|------|-------|----------------|
| `ConfigurationService.java` | ~400 | Unified config entry point, persistence, implements Settings interfaces |
| `ConfigLoader.java` | ~350 | Parses `ui_layout.cfg` S-expression syntax into GroupConfig/RowConfig tree |
| `ConfigManager.java` | ~200 | Handles first-run setup, user config path, factory reset, import/export |
| `SExpParser.java` | ~150 | Low-level S-expression tokenizer and parser |
| `HUDSettings.java` | ~50 | MiniHUD-specific config interface |
| `OverlaySettings.java` | ~60 | Generic overlay config interface |
| `ConfigProvider.java` | ~20 | Base interface for `getConfig(key)` / `setConfig(key, value)` |
| `Config.java` | ~50 | Legacy property-based config (deprecated) |
| `ConfigWatcherService.java` | ~80 | File watcher for hot-reload (optional) |

## Core Class Responsibilities

### ConfigurationService

Main entry point implementing both `ConfigProvider` and generating `OverlaySettings` instances:

```java
public class ConfigurationService implements ConfigProvider {
    private List<GroupConfig> layoutConfigs;

    public void initConfig();                    // Load from ConfigManager
    public void saveLayoutConfig();              // Persist to user config
    public boolean importConfig(String path);   // Import external config
    public boolean resetToFactory();            // Reset to defaults

    // Settings factory methods
    public HUDSettings getHudSettings();
    public OverlaySettings getOverlaySettings(String groupTitle);
}
```

### ConfigLoader Data Model

```java
public class GroupConfig {
    public String title;           // Group display name
    public double x, y;            // Window position (0.0-1.0 ratio)
    public int alpha;              // Transparency (0-255)
    public int hotkey;             // Toggle hotkey (NativeKeyEvent code)
    public boolean visible;        // Initial visibility
    public String fontName;        // Font family
    public int fontSize;           // Font size adjustment (-6 to +20)
    public int columns;            // Data display columns
    public int panelColumns;       // Settings panel columns
    public String switchKey;       // Config key for visibility toggle
    public List<RowConfig> rows;   // Child items
}

public class RowConfig {
    public String label;           // Display label
    public String type;            // DATA, HEADER, SLIDER, COMBO, SWITCH, SWITCH_INV, BUTTON, COLOR, HOTKEY
    public String formula;         // Reflection path (e.g., "S.rpm")
    public Object value;           // Current value (Boolean, Integer, String)
    public Object defaultValue;    // Factory default for reset
    public String desc;            // Tooltip description
    public String descImg;         // Help image path
    public int minVal, maxVal;     // For SLIDER type
    public boolean hideWhenZero;   // Hide if value is zero
    public int precision;          // Decimal places
    public String fgColor;         // Foreground color
    public String unitSource;      // Dynamic unit method (e.g., "getManifoldPressureDisplayUnit")
    public String precisionSource; // Dynamic precision method (e.g., "getManifoldPressureDisplayPrecision")
    public List<RowConfig> children; // Nested items
}
```

### Settings Interfaces

```java
public interface OverlaySettings {
    int getWindowX(int width);
    int getWindowY(int height);
    void saveWindowPosition(double x, double y);
    String getFontName();
    String getNumFontName();
    int getFontSizeAdd();
    boolean getBool(String key, boolean def);
    int getInt(String key, int def);
    String getString(String key, String def);
    GroupConfig getGroupConfig();
}

public interface HUDSettings extends OverlaySettings {
    String getNumFont();
    int getCrosshairScale();
    String getCrosshairName();
    boolean isDisplayCrosshair();
    boolean useTextureCrosshair();
    boolean drawHUDText();
    boolean drawHUDAttitude();
    double getAoAWarningRatio();
    double getAoABarWarningRatio();
    boolean enableFlapAngleBar();
    boolean showSpeedBar();
    boolean drawHudMach();
    boolean isSpeedLabelDisabled();
    boolean isAltitudeLabelDisabled();
    boolean isSEPLabelDisabled();
    boolean isAoADisabled();
}
```

## Adding a New Configuration Item

### Step 1: Define in `ui_layout.cfg`

```lisp
(group "MiniHUD"
  (item "速度条开关"
        :type switch
        :target "showSpeedBar"
        :value true
        :default true
        :desc "显示速度比例条"))
```

**Item Types:**

| Type | Value Type | UI Widget | Extra Properties |
|------|------------|-----------|------------------|
| `switch` | Boolean | Toggle switch | - |
| `switch_inv` | Boolean | Toggle (inverted) | UI ON = value false |
| `slider` | Integer | Slider + Spinner | `:min`, `:max`, `:unit` |
| `combo` | String | Dropdown | `:options ("A" "B" "C")` |
| `color` | String | Color picker | Hex: `#RRGGBBAA` or Decimal: `R,G,B,A` |
| `font` | String | Font selector | - |
| `hotkey` | Integer | Key binding | NativeKeyEvent code |
| `button` | - | Action button | `:fgColor`, callback |
| `data` | - | Read-only display | `:formula`, `:format`, `:unit-source`, `:precision-source` |

### Slider Unit Display

`:unit` attribute displays a unit label after the Spinner:

```lisp
(item "刷新间隔" :type slider :target "refreshMs" :min 10 :max 300 :unit "ms" :value 80)
```

Layout: `[Label] ... [Slider] [Spinner] [Unit]`

**Note:** `:format` is deprecated for slider types; use `:unit` instead.

### Color Format

| Format | Example | Notes |
|--------|---------|-------|
| Hex | `"#FF5500AA"` | Preferred for ui_layout.cfg defaults |
| Decimal | `"255, 85, 0, 170"` | Legacy format, backward compatible |

**Behavior:** Display in hex (text field), store in decimal (config file), auto-save on focus lost.

### Step 2: Add Interface Method

In `HUDSettings.java` or `OverlaySettings.java`:

```java
public interface HUDSettings extends OverlaySettings {
    // ... existing methods ...

    boolean showSpeedBar();
}
```

### Step 3: Implement in ConfigurationService

In the inner class `HUDSettingsImpl`:

```java
@Override
public boolean showSpeedBar() {
    return getBool("showSpeedBar", true);  // key matches :target
}
```

### Step 4: Use in Overlay/Component

```java
// MiniHUDOverlay.java
boolean showSpeed = hudSettings.showSpeedBar();
speedRatioBar.setVisible(showSpeed);
```

### Step 5: Enable WYSIWYG Preview

In `Controller.java`, add the config key to the overlay's interest list:

```java
overlayManager.registerWithPreview("crosshairSwitch", ...)
    .withInterest("displayCrosshair", "showSpeedBar", ...);
```

## WYSIWYG Preview Mechanism

When a user changes a config value in the Settings UI:

```
User toggles switch in MainForm
        ↓
ConfigurationService.setConfig(key, value)
        ↓
UIStateBus.publish(CONFIG_CHANGED, key)
        ↓
Controller receives event, checks State == PREVIEW
        ↓
OverlayManager.refreshPreviews(key)
        ↓
Matches overlay's interest list? → reinitConfig()
        ↓
Overlay re-reads settings and updates UI
```

## S-Expression Syntax Reference

```lisp
;; Panel definition (top-level container for settings UI)
(panel "PanelTitle"
  :cols 2                         ; Settings panel columns

  ;; Group definition (collapsible section)
  (group "GroupTitle"
    :switch "groupSwitchKey"      ; Optional: visibility toggle key
    :x 0.5 :y 0.3                 ; Optional: window position (0.0-1.0 ratio)
    :fontSize 2                   ; Optional: font size offset
    :alpha 180                    ; Optional: transparency (0-255)
    :hotkey "P"                   ; Optional: toggle hotkey

    ;; Items inside group
    (item "Label Text"
          :type switch|slider|combo|color|font|hotkey|button|data
          :target "configKey"
          :value <default-value>
          :default <factory-default>
          :desc "Tooltip description"
          :descImg "path/to/help.png"

          ;; Type-specific options
          :min 0 :max 100         ; For slider
          :unit "ms"              ; For slider (unit label)
          :options ("A" "B")      ; For combo
          :fgColor "255,100,100"  ; For button
          :formula "S.TAS"        ; For data
          :format "%.1f"          ; For data
          :unit "km/h"            ; For data (static unit)
          :unit-source "getDisplayUnit"    ; For data (dynamic unit from TelemetrySource method)
          :precision-source "getDisplayPrecision" ; For data (dynamic precision)
          :hideWhenZero true      ; For data
    )

    ;; Nested group (subsection)
    (group "Subsection"
      (item ...))
  )
)
```

## Config Value Access Patterns

```java
// Boolean with default
boolean enabled = settings.getBool("enableFeature", true);

// Integer with default
int size = settings.getInt("fontSize", 16);

// String with default
String font = settings.getString("fontName", "Arial");

// Color (returns java.awt.Color)
Color c = configService.getColorConfig("fontNum");

// Check if field is disabled (for hide-when-zero logic)
boolean hidden = configService.isFieldDisabled("thrust");

// Access raw GroupConfig for advanced operations
GroupConfig group = settings.getGroupConfig();
RowConfig row = findRow(group.rows, "targetKey");
```

## Event-Driven Architecture

Config changes propagate via `UIStateBus`:

```java
// Publishing a config change
UIStateBus.getInstance().publish(
    UIStateEvents.CONFIG_CHANGED,
    "sourceId",
    "configKey"
);

// Subscribing to config changes
UIStateBus.getInstance().subscribe(UIStateEvents.CONFIG_CHANGED, key -> {
    if ("myKey".equals(key)) {
        // React to change
    }
});

// Special event keys
UIStateEvents.ACTION_RESET_REQUEST   // Request factory reset
UIStateEvents.ACTION_RESET_COMPLETED // Reset completed notification
```

## Dynamic Unit/Precision Support

For DATA fields that need to change units or precision based on aircraft type (e.g., metric vs imperial), use the `:unit-source` and `:precision-source` keywords.

### Use Case: Manifold Pressure

German/Soviet aircraft display manifold pressure in Ata (metric), while American/British aircraft use psi with inHg reference (imperial). The unit and precision must change dynamically.

### Implementation Steps

**1. Add TelemetrySource methods** in `Service.java`:

```java
@Override
public double getManifoldPressureDisplay() {
    return isImperial() ? getManifoldPressurePounds() : getManifoldPressure();
}

@Override
public String getManifoldPressureDisplayUnit() {
    if (isImperial()) {
        return String.format("P/%.1f''", getManifoldPressureInchHg());
    }
    return "Ata";
}

@Override
public int getManifoldPressureDisplayPrecision() {
    return isImperial() ? 1 : 2;
}
```

**2. Declare interface methods** in `TelemetrySource.java`:

```java
double getManifoldPressureDisplay();
String getManifoldPressureDisplayUnit();
int getManifoldPressureDisplayPrecision();
```

**3. Configure in ui_layout.cfg**:

```lisp
(item "进气压" :type data
      :target "getManifoldPressureDisplay"
      :unit-source "getManifoldPressureDisplayUnit"
      :precision-source "getManifoldPressureDisplayPrecision"
      :unit "Ata"        ; Default for preview mode
      :precision 2       ; Default for preview mode
      :hide-when-zero true)
```

### How It Works

1. `PowerInfoOverlay.bindDynamicFields()` detects `:unit-source`/`:precision-source`
2. Uses `ReflectBinder.resolveString()` and `ReflectBinder.resolveInt()` to create suppliers
3. Calls `FieldManager.bind()` with the dynamic suppliers
4. `FieldOverlay.onFlightData()` invokes suppliers each frame to update unit/precision

### Behavior

| Aircraft | Display Value | Unit String | Precision |
|----------|---------------|-------------|-----------|
| Bf 109 (German) | `1.35` | `Ata` | 2 |
| P-51 (American) | `+5.1` | `P/40.4''` | 1 |

The unit string for imperial mode dynamically includes the current inHg value, updating each frame as throttle changes.

## Important Notes

- Config changes are **automatically persisted** when the user exits MainForm or game mode
- Use `configService.saveLayoutConfig()` to force immediate save
- The `SWITCH_INV` type inverts boolean logic (useful for "disable" toggles)
- Config keys are **case-sensitive** and must match exactly between ui_layout.cfg and code
- `ConfigManager.getUserConfigPath()` returns platform-specific user config location
- First-run copies factory config to user location automatically
