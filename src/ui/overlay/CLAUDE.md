# Overlay Development Guide

## File Overview

| File | Lines | Purpose |
|------|-------|---------|
| `MiniHUDOverlay.java` | ~700 | Primary HUD with component-based architecture |
| `DrawFrame.java` | ~770 | **@Deprecated** Legacy FM curve visualization |
| `DrawFrameSimpl.java` | ~740 | **@Deprecated** Simplified DrawFrame variant |
| `BaseOverlay.java` | ~290 | Standard list-based overlay base class |
| `AttitudeOverlay.java` | ~430 | Artificial horizon display |
| `EngineControlOverlay.java` | ~610 | Engine gauges and throttle |
| `ControlSurfacesOverlay.java` | ~340 | Control surface indicators |
| `GearFlapsOverlay.java` | ~290 | Landing gear and flaps status |
| `FlightInfoOverlay.java` | ~140 | Flight data display |
| `PowerInfoOverlay.java` | ~170 | Engine power metrics |
| `FMUnpackedDataOverlay.java` | ~120 | Flight model debug display |
| `MinimalHUDContext.java` | ~190 | Immutable config snapshot for MiniHUD |
| `SituationAware.java` | ~190 | Situation awareness logic |
| `ZebraListRenderer.java` | ~60 | Alternating row renderer |
| `OverlayRenderer.java` | ~30 | Renderer interface |

## Inheritance Hierarchy

```
DraggableOverlay (ui/base/) - WebLaF dialog with drag support
    ↑
BaseOverlay - Standard list-based overlay with pluggable renderer
    ↑
FlightInfoOverlay, PowerInfoOverlay, GearFlapsOverlay, FMUnpackedDataOverlay

DraggableOverlay
    ↑
MiniHUDOverlay - Custom HUD with component-based rendering (NOT BaseOverlay)

DraggableOverlay
    ↑
AttitudeOverlay, EngineControlOverlay, ControlSurfacesOverlay - Custom rendering
```

## Standard Overlay Lifecycle

### Game Mode Initialization

```java
overlay.init(Controller controller, Service service, OverlaySettings settings)
```
- Called when entering game mode (after aircraft is detected)
- Register with `FlightDataBus` for real-time data
- Start background update thread
- Apply production styling (no preview border)

### Preview Mode Initialization

```java
overlay.initPreview(Controller controller, OverlaySettings settings)
```
- Called when user opens Settings UI with preview enabled
- Use mock/sample data for display
- Enable drag repositioning
- Apply preview visual styling (colored border)
- **No Service available** - must use static demo data

### Configuration Reload

```java
overlay.reinitConfig()
```
- Called when relevant config keys change (via WYSIWYG)
- Re-read settings and update appearance
- Do NOT recreate entire overlay - update in place
- Called on EDT thread

### Cleanup

```java
overlay.dispose()
```
- **CRITICAL**: Unregister from `FlightDataBus`
- Stop background threads
- Release resources
- Called when exiting game mode or closing preview

## FlightDataListener Subscription Pattern

```java
public class MyOverlay extends BaseOverlay implements FlightDataListener {

    @Override
    public void init(Controller c, Service s, OverlaySettings settings) {
        super.init(settings, () -> generateDataLines());

        // Subscribe to real-time data
        FlightDataBus.getInstance().register(this);
    }

    @Override
    public void onFlightData(FlightDataEvent event) {
        // IMPORTANT: Dispatch UI updates to EDT
        SwingUtilities.invokeLater(() -> {
            // Access type-safe payload for low-frequency fields
            EventPayload payload = event.getPayload();
            this.mapGrid = payload.mapGrid;
            this.fatalWarn = payload.fatalWarn;
            updateDisplay();
        });
    }

    @Override
    public void dispose() {
        // CRITICAL: Unregister to prevent zombie listeners
        FlightDataBus.getInstance().unregister(this);
        super.dispose();
    }
}
```

## Registering with OverlayManager

In `Controller.registerGameModeOverlays()`:

```java
overlayManager.registerWithPreview(
    "configSwitchKey",           // Config key that enables this overlay
    () -> new MyOverlay(),       // Factory: creates new instance
    overlay -> ((MyOverlay) overlay).init(this, S, configService.getOverlaySettings("SectionName")),
    overlay -> ((MyOverlay) overlay).initPreview(this, configService.getOverlaySettings("SectionName")),
    overlay -> ((MyOverlay) overlay).reinitConfig(),  // Optional: config reload
    true                         // previewEnabled: show in settings UI
)
.withInterest("key1", "key2");   // Config keys that trigger reinitConfig
```

### Interest Keys for WYSIWYG

The `.withInterest(keys...)` method specifies which config changes should trigger `reinitConfig()`:

```java
// MiniHUD example: refresh preview when any of these change
.withInterest("displayCrosshair", "drawHUD", "disableHUD", "crosshair",
              "miniHUD", "enableLayoutDebug", "enableFlapAngleBar",
              "hudMach", "showSpeedBar");
```

## Activation Strategies

For overlays with complex visibility rules:

```java
// Game mode only (no preview)
overlayManager.registerWithStrategy("enableVoiceWarn",
    () -> new VoiceWarning(),
    overlay -> ((VoiceWarning) overlay).init(this, S),
    null,  // No preview initializer
    null,  // No reinit
    true,
    ActivationStrategy.config("enableVoiceWarn")
        .and(ActivationStrategy.gameModeOnly())
);

// Conditional on engine type
ActivationStrategy.config("enableFMPrint")
    .and(ActivationStrategy.jetOnly())

ActivationStrategy.config("showManifold")
    .and(ActivationStrategy.propOnly())
```

## BaseOverlay Architecture

Standard overlays use a **composition-based rendering** approach:

```java
public class BaseOverlay extends DraggableOverlay {
    protected WebPanel dataPanel;
    protected OverlayRenderer renderer;  // Pluggable: ZebraListRenderer, etc.
    protected Supplier<List<String>> dataSupplier;

    public void init(OverlaySettings settings, Supplier<List<String>> supplier) {
        this.settings = settings;
        this.dataSupplier = supplier;
        this.renderer = new ZebraListRenderer();  // Default
        // Build UI...
    }

    // Background thread calls dataSupplier and updates display
    public void run() {
        while (doit) {
            List<String> data = dataSupplier.get();
            SwingUtilities.invokeLater(() -> renderer.render(dataPanel, data));
            Thread.sleep(100);
        }
    }
}
```

### Renderer Interface

```java
public interface OverlayRenderer {
    void render(WebPanel panel, List<String> lines);
    void setHeaderMatcher(Predicate<String> matcher);
}
```

## MiniHUD Special Architecture

MiniHUD uses a **component-based architecture** instead of the standard list renderer:

```
MiniHUDOverlay
    ├─ MinimalHUDContext (immutable config snapshot)
    │      - Pre-calculated dimensions, fonts, strokes
    │      - Created once per reinitConfig()
    │
    ├─ HUDCalculator (pure computation, no UI)
    │      - Calculates HUDData from Service state
    │      - No side effects, stateless
    │
    ├─ ModernHUDLayoutEngine (topological relative layout)
    │      - DAG-based anchor dependency resolution
    │      - Computes absolute positions from relative anchors
    │
    └─ HUDComponent[] (pluggable visual components)
           - CrosshairGauge, CompassGauge, TextGauge, etc.
           - Receive data via onDataUpdate(HUDData)
           - draw(Graphics2D, x, y) for rendering
```

### MinimalHUDContext

Immutable context object holding pre-calculated metrics:

```java
public class MinimalHUDContext {
    // Layout Dimensions
    public final int width, height;
    public final int hudFontSize, hudFontSizeSmall;
    public final int windowX, windowY;

    // Component Metrics
    public final int crossScale, crossX, crossY;
    public final int compassDiameter, compassRadius;
    public final double aoaLength;

    // Styling
    public final int lineWidth, barWidth;
    public final Font drawFont, drawFontSmall;
    public final BasicStroke strokeThick, strokeThin;

    // Resources
    public final Image crosshairImageScaled;

    // Factory method
    public static MinimalHUDContext create(HUDSettings settings);
}
```

**Key differences from BaseOverlay:**
- No `dataPanel` or `ZebraListRenderer`
- Custom `paintComponent()` drives all rendering
- Layout computed via DAG-based dependency resolution
- Components are stateless - receive data via `onDataUpdate(HUDData)`

For detailed MiniHUD component development, see: [`../component/CLAUDE.md`](../component/CLAUDE.md)

## Creating a New Simple Overlay

1. Extend `BaseOverlay`
2. Implement data supplier function
3. Register in `Controller.registerGameModeOverlays()`

```java
public class MyInfoOverlay extends BaseOverlay {

    private Service service;

    public void init(Controller c, Service s, OverlaySettings settings) {
        this.service = s;
        super.init(settings, this::generateLines);
    }

    private List<String> generateLines() {
        List<String> lines = new ArrayList<>();
        lines.add("Speed: " + String.format("%.0f", service.sState.TAS) + " km/h");
        lines.add("Altitude: " + String.format("%.0f", service.sState.H) + " m");
        return lines;
    }

    public void initPreview(Controller c, OverlaySettings settings) {
        super.initPreview(settings, () -> Arrays.asList(
            "Speed: 450 km/h",
            "Altitude: 5000 m"
        ));
    }
}
```

## Creating a Custom Rendering Overlay

For overlays needing custom graphics (gauges, dials, etc.):

```java
public class MyGaugeOverlay extends DraggableOverlay {

    private double currentValue = 0;
    private Font cachedFont;  // Pre-allocate for zero-GC

    @Override
    protected void paintComponent(Graphics g) {
        super.paintComponent(g);
        Graphics2D g2d = (Graphics2D) g;

        // Enable anti-aliasing
        g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING,
                             RenderingHints.VALUE_ANTIALIAS_ON);

        // Custom drawing...
        g2d.setFont(cachedFont);
        g2d.drawArc(...);
    }

    @Override
    public void onFlightData(FlightDataEvent event) {
        // Use TelemetrySource for high-frequency numerics (preferred)
        // or event.getPayload() for low-frequency logic flags
        double newValue = telemetrySource.getThrottle();
        if (Math.abs(newValue - currentValue) > 0.01) {
            currentValue = newValue;
            SwingUtilities.invokeLater(this::repaint);
        }
    }
}
```

## Threading Rules

1. **Never block EDT** - Long operations go in background threads
2. **Always dispatch UI updates** - Use `SwingUtilities.invokeLater()`
3. **Synchronized access** - `OverlayManager` methods are synchronized to prevent race conditions
4. **Background thread for data polling** - `BaseOverlay.run()` handles this automatically
5. **Dirty checking** - Only repaint when data actually changes

```java
// Good: Dirty checking pattern
private double lastValue = Double.NaN;

@Override
public void onFlightData(FlightDataEvent event) {
    double newValue = telemetrySource.getIAS();  // Use TelemetrySource for numerics
    if (Math.abs(newValue - lastValue) < 0.001) {
        return;  // Skip redundant update
    }
    lastValue = newValue;
    SwingUtilities.invokeLater(this::updateDisplay);
}
```

## Common Pitfalls

| Issue | Symptom | Solution |
|-------|---------|----------|
| Zombie Listeners | Memory leaks, crashes after closing | Always unregister from `FlightDataBus` in `dispose()` |
| EDT Violations | Random UI corruption, freezes | Use `SwingUtilities.invokeLater()` for all UI updates |
| Interest Mismatch | WYSIWYG preview doesn't update | Add config keys to `.withInterest()` |
| Preview Without Data | NullPointerException in preview | Check `isPreview` flag, use mock data |
| GC Pressure | Frame drops, stuttering | Cache Color/Font objects, avoid allocations in paint |
| Blocking EDT | UI freezes during data fetch | Move HTTP/IO to background threads |

## Performance Optimization

### Zero-Allocation Rendering

```java
// BAD: Creates garbage every frame
public void paintComponent(Graphics g) {
    Color c = new Color(255, 0, 0);  // GC pressure!
    g.setColor(c);
}

// GOOD: Reuse cached objects
private static final Color WARNING_COLOR = new Color(255, 0, 0);

public void paintComponent(Graphics g) {
    g.setColor(WARNING_COLOR);
}
```

### Dirty Checking

```java
private String lastText = "";

public void updateText(String newText) {
    if (newText.equals(lastText)) {
        return;  // Skip redundant repaint
    }
    lastText = newText;
    repaint();
}
```

### Template Width for Stability

```java
// BAD: Width changes with content
label.setText("ALT " + currentAlt);

// GOOD: Width locked to template
label.setTemplate("ALT 88888");  // Maximum expected width
label.setText("ALT " + currentAlt);
```

## Using TelemetrySource in Overlays

Overlays should access flight data through the `TelemetrySource` interface rather than directly accessing `Service` internal fields. This eliminates Feature Envy and provides a clean abstraction.

### Pattern

```java
public class MyOverlay extends DraggableOverlay implements FlightDataListener {
    private Service xs;
    private ui.model.TelemetrySource telemetrySource;

    public void init(Controller c, Service s, OverlaySettings settings) {
        this.xs = s;
        // Cast Service to TelemetrySource interface
        if (s instanceof ui.model.TelemetrySource) {
            this.telemetrySource = (ui.model.TelemetrySource) s;
        }
        // ... rest of init
    }

    public void drawTick() {
        if (telemetrySource == null) return;

        // GOOD: Use TelemetrySource interface
        double aoa = telemetrySource.getAoA();
        double pitch = telemetrySource.getAviahorizonPitch();

        // BAD: Direct field access (Feature Envy)
        // double aoa = xs.sState.AoA;
        // double pitch = xs.sIndic.aviahorizon_pitch;
    }
}
```

### When to Use TelemetrySource vs Direct Access

| Data Type | Access Method | Example |
|-----------|---------------|---------|
| Flight telemetry | `TelemetrySource` | `getAoA()`, `getIAS()`, `getAviahorizonPitch()` |
| FM configuration | Direct `Blkx` access | `xc.getBlkx().NoFlapsWing.AoACritHigh` |
| Event flags | `FlightDataEvent.getPayload()` | `payload.isJet`, `payload.fatalWarn` |

## UI Utility Classes

### OverlayStyleHelper

Use `ui.util.OverlayStyleHelper` for common window styling operations:

```java
import ui.util.OverlayStyleHelper;

// Apply transparent style (game mode) - replaces setFrameOpaque()
OverlayStyleHelper.applyTransparentStyle(this);

// Apply preview style (settings UI)
OverlayStyleHelper.applyPreviewStyle(this);

// Load font configuration with defaults
OverlayStyleHelper.FontConfig fonts = OverlayStyleHelper.loadFontConfig(overlaySettings);
Font labelFont = new Font(fonts.fontName, Font.BOLD, 12 + fonts.fontSizeAdd);
```

This helper consolidates duplicate styling code previously scattered across 6+ overlay files.

### SliderHelper

Use `ui.util.SliderHelper` for read-only display sliders:

```java
import ui.util.SliderHelper;

// Vertical progress bar (flaps/gear display)
SliderHelper.configureVerticalProgress(slider, 0, 100, topColor, bottomColor);

// Horizontal attitude slider (control surfaces)
SliderHelper.configureAttitudeSlider(slider, -100, 100, Application.colorNum);
```

This helper consolidates the ~25-line `initslider()` pattern in GearFlapsOverlay, AttitudeOverlay, and ControlSurfacesOverlay.

### GraphicsUtil

Use `ui.util.GraphicsUtil` for standard rendering configuration:

```java
import ui.util.GraphicsUtil;

public void paintComponent(Graphics g) {
    Graphics2D g2d = (Graphics2D) g;
    GraphicsUtil.configureOverlayRendering(g2d);  // Replaces 4-line hint pattern
    // ... drawing code
}
```

For full documentation, see: [`../util/CLAUDE.md`](../util/CLAUDE.md)

## Deprecated Classes

### DrawFrame / DrawFrameSimpl

These legacy FM curve visualization classes are marked `@Deprecated`:

- Do not extend or use as templates for new overlays
- They use non-standard initialization patterns
- New overlays should extend `DraggableOverlay` and follow the standard lifecycle

See `MiniHUDOverlay` for a modern event-driven implementation pattern.
