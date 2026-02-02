# Overlay Development Guide

## Inheritance Hierarchy

```
DraggableOverlay (WebLaF dialog with drag support)
    ↑
BaseOverlay (Standard list-based overlay with renderer)
    ↑
FlightInfoOverlay, PowerInfoOverlay, GearFlapsOverlay, ...

DraggableOverlay
    ↑
MiniHUDOverlay (Custom HUD with component-based rendering)
```

## Standard Overlay Lifecycle

### Game Mode Initialization

```java
overlay.init(Controller, Service, OverlaySettings)
```
- Called when entering game mode (after aircraft is detected)
- Register with `FlightDataBus` for real-time data
- Start background update thread

### Preview Mode Initialization

```java
overlay.initPreview(Controller, OverlaySettings)
```
- Called when user opens Settings UI with preview enabled
- Use mock/sample data for display
- Enable drag repositioning
- Apply preview visual styling (colored border)

### Configuration Reload

```java
overlay.reinitConfig()
```
- Called when relevant config keys change (via WYSIWYG)
- Re-read settings and update appearance
- Do NOT recreate entire overlay - update in place

### Cleanup

```java
overlay.dispose()
```
- **CRITICAL**: Unregister from `FlightDataBus`
- Stop background threads
- Release resources

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
            this.cachedData = event.getData();
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
```

## MiniHUD Special Architecture

MiniHUD uses a **component-based architecture** instead of the standard list renderer:

```
MiniHUDOverlay
    ├─ MinimalHUDContext (immutable config snapshot)
    ├─ HUDCalculator (pure computation, no UI)
    ├─ ModernHUDLayoutEngine (topological relative layout)
    └─ HUDComponent[] (pluggable visual components)
```

**Key differences from BaseOverlay:**
- No `dataPanel` or `ZebraListRenderer`
- Custom `paintComponent()` drives all rendering
- Layout computed via DAG-based dependency resolution
- Components are stateless - receive data via `onDataUpdate(HUDData)`

For detailed MiniHUD development, see: [`doc/minihud贡献者开发手册.md`](../../doc/minihud贡献者开发手册.md)

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
        lines.add("Speed: " + service.sState.TAS);
        lines.add("Altitude: " + service.sState.H);
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

## Threading Rules

1. **Never block EDT** - Long operations go in background threads
2. **Always dispatch UI updates** - Use `SwingUtilities.invokeLater()`
3. **Synchronized access** - `OverlayManager` methods are synchronized to prevent race conditions
4. **Background thread for data polling** - `BaseOverlay.run()` handles this automatically

## Common Pitfalls

- **Zombie Listeners**: Forgetting to unregister from `FlightDataBus` in `dispose()` causes memory leaks and crashes
- **EDT Violations**: Modifying Swing components from background threads causes random UI corruption
- **Interest Mismatch**: Forgetting to add config keys to `.withInterest()` breaks WYSIWYG preview
- **Preview Without Data**: Preview mode has no `Service` - use mock data
