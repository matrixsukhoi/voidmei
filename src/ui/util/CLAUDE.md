# UI Utility Classes Development Guide

## Overview

The `ui.util` package provides **reusable utility classes** for UI components, overlays, and rendering. These utilities consolidate common patterns to reduce code duplication and ensure consistency.

## File Overview

| File | Responsibility |
|------|----------------|
| `OverlayStyleHelper.java` | Window transparency and preview styling for overlays |
| `GraphicsUtil.java` | Graphics2D rendering utilities (strokes, rendering hints) |
| `SliderHelper.java` | Read-only WebSlider configuration for display gauges |
| `FastNumberFormatter.java` | Zero-allocation number formatting for HUD displays |
| `DialogService.java` | Dialog display with z-order coordination |
| `NotificationService.java` | Toast notification system |
| `NotificationFactory.java` | Notification creation helpers |
| `ReflectBinder.java` | Reflection-based UI binding utilities |

---

## OverlayStyleHelper

Consolidates window styling patterns used across 6+ overlay files.

```java
import ui.util.OverlayStyleHelper;

// Apply transparent window style (game mode)
OverlayStyleHelper.applyTransparentStyle(this);

// Apply preview mode styling (settings UI)
OverlayStyleHelper.applyPreviewStyle(this);

// Load font configuration with defaults fallback
OverlayStyleHelper.FontConfig fonts = OverlayStyleHelper.loadFontConfig(overlaySettings);
Font labelFont = new Font(fonts.fontName, Font.BOLD, 12 + fonts.fontSizeAdd);
```

**Available Methods:**

| Method | Description |
|--------|-------------|
| `applyTransparentStyle(WebFrame)` | Make window fully transparent (game mode) |
| `applyPreviewStyle(WebFrame)` | Apply colored background for WYSIWYG preview |
| `loadFontConfig(OverlaySettings)` | Load font name, numFont, and sizeAdd with defaults |

**Replaces:** The 5-line `setFrameOpaque()` pattern in AttitudeOverlay, ControlSurfacesOverlay, MiniHUDOverlay, GearFlapsOverlay, DrawFrame, DrawFrameSimpl.

---

## GraphicsUtil

Provides optimized Graphics2D configuration utilities.

```java
import ui.util.GraphicsUtil;

// Configure standard overlay rendering hints (replaces 4-line pattern)
public void paintComponent(Graphics g) {
    Graphics2D g2d = (Graphics2D) g;
    GraphicsUtil.configureOverlayRendering(g2d);
    // ... drawing code
}

// Create precise strokes (no cap extension)
Stroke stroke = GraphicsUtil.createPreciseStroke(2.0f);

// Create rounded strokes (for decorative lines)
Stroke rounded = GraphicsUtil.createRoundedStroke(3.0f);
```

**Available Methods:**

| Method | Description |
|--------|-------------|
| `configureOverlayRendering(Graphics2D)` | Apply standard AA and performance hints |
| `createPreciseStroke(float)` | Stroke with CAP_BUTT (exact endpoints) |
| `createPreciseStroke(float, int)` | Precise stroke with custom join style |
| `createRoundedStroke(float)` | Stroke with CAP_ROUND (smooth endpoints) |

**Consolidates:** The 4-line rendering hint pattern repeated 30+ times:
```java
// Before (repeated in every overlay)
g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION, RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);

// After
GraphicsUtil.configureOverlayRendering(g2d);
```

---

## SliderHelper

Configures WebSlider components as read-only visual indicators.

```java
import ui.util.SliderHelper;

// Vertical progress bar (gear/flaps display)
SliderHelper.configureVerticalProgress(slider, 0, 100, progressTopColor, progressBottomColor);

// Horizontal attitude slider (control surface display)
SliderHelper.configureAttitudeSlider(slider, -100, 100, Color.white);

// Just disable interaction on any slider
SliderHelper.removeAllListeners(slider);
```

**Available Methods:**

| Method | Description |
|--------|-------------|
| `configureVerticalProgress(WebSlider, min, max, topColor, bottomColor)` | Vertical progress bar with gradient |
| `configureAttitudeSlider(WebSlider, min, max, thumbColor)` | Horizontal slider with visible thumb |
| `removeAllListeners(WebSlider)` | Remove all mouse interaction |

**Replaces:** The ~25-line `initslider()` methods in GearFlapsOverlay, AttitudeOverlay, ControlSurfacesOverlay.

**Usage Notes:**
- These sliders display values but don't accept user input
- Mouse listeners are removed to prevent accidental changes
- Used for flaps percentage, control surface positions, etc.

---

## FastNumberFormatter

Zero-allocation number formatting for high-frequency HUD updates.

```java
import ui.util.FastNumberFormatter;

// Format to reusable char buffer (zero GC)
char[] buffer = new char[8];
int length = FastNumberFormatter.format(1234, buffer, 0);
// buffer now contains "1234", length = 4

// Format with decimal places
length = FastNumberFormatter.format(123.456, buffer, 2);
// buffer now contains "123.46", length = 6
```

**Design Notes:**
- Avoids String allocation in paintComponent hot paths
- Used by HUD rows and gauge components
- Critical for 60+ FPS rendering without GC pauses

---

## DialogService

Manages dialog display with overlay z-order coordination.

```java
import ui.util.DialogService;

// Show dialog (automatically suspends overlay alwaysOnTop)
DialogService.showDialog(parentFrame, dialogContent);

// Show color chooser dialog
Color selected = DialogService.showColorChooser(parentFrame, initialColor);
```

**Integration:** Works with `AlwaysOnTopCoordinator` to ensure dialogs appear above game overlays.

---

## Design Principles

1. **Static Methods**: All utilities are stateless with static methods
2. **Zero Allocation**: Avoid object creation in rendering hot paths
3. **DRY Consolidation**: Each utility replaces 3+ instances of duplicate code
4. **Fail Safe**: Invalid inputs return sensible defaults
5. **Thread Safe**: Safe to call from any thread (no shared mutable state)

## Adding New Utilities

When creating UI utilities:

1. Identify patterns repeated 3+ times across files
2. Create static methods with clear parameter names
3. Document which files are consolidated
4. Update this file with usage examples
5. Migrate existing code to use the new utility
