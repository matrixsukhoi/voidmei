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

## Core Class Responsibilities

| Class | Responsibility |
|-------|----------------|
| `ConfigurationService` | Unified config entry point, persistence, implements Settings interfaces |
| `ConfigLoader` | Parses `ui_layout.cfg` S-expression syntax into GroupConfig/RowConfig tree |
| `ConfigManager` | Handles first-run setup, user config path, factory reset |
| `HUDSettings` | MiniHUD-specific config interface (crosshair, fonts, toggles) |
| `OverlaySettings` | Generic overlay config interface (position, font, size) |
| `ConfigProvider` | Base interface for `getConfig(key)` / `setConfig(key, value)` |

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
- `switch` - Boolean toggle (true/false)
- `switch_inv` - Inverted boolean (UI shows ON when value is false)
- `slider` - Numeric range with min/max
- `combo` - Dropdown selection
- `color` - RGBA color picker
- `font` - Font family selector

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
;; Group definition
(group "GroupTitle"
  :switch "groupSwitchKey"    ; Optional: visibility toggle key
  :x 0.5 :y 0.3               ; Optional: window position (0.0-1.0 ratio)
  :fontSize 2                 ; Optional: font size offset

  ;; Items inside group
  (item "Label Text"
        :type switch|slider|combo|color|font
        :target "configKey"
        :value <default-value>
        :default <factory-default>
        :desc "Tooltip description"

        ;; Type-specific options
        :min 0 :max 100       ; For slider
        :options ("A" "B")    ; For combo
  )

  ;; Nested group (collapsible section)
  (group "Subsection"
    (item ...))
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
```

## Important Notes

- Config changes are **automatically persisted** when the user exits MainForm or game mode
- Use `configService.saveLayoutConfig()` to force immediate save
- The `SWITCH_INV` type inverts boolean logic (useful for "disable" toggles)
- Config keys are case-sensitive and must match exactly
