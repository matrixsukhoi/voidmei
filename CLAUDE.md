# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VoidMei is a Java Swing telemetry overlay for War Thunder. It reads real-time flight data from the game's local HTTP API (port 8111) and displays HUD overlays with flight metrics, warnings, and aircraft performance data.

## Build Commands

```bash
# Compile (requires JDK 1.8)
mkdir -p bin
find src -name "*.java" > sources.txt
javac -encoding UTF-8 -d bin -classpath 'dep/*' @sources.txt

# Package JAR
jar -cvfm VoidMei.jar MANIFEST.MF -C ./bin .

# Run
java -jar VoidMei.jar

# Package Windows EXE (requires launch4j)
launch4j ./script/voidmeil4j.xml

# Full build script
./script/build.sh
```

**No automated tests** - testing is manual via the running application.

## Architecture

### Core Packages (`src/`)

- **`prog/`** - Application kernel
  - `Application.java` - Entry point, global config, fonts, logging
  - `Service.java` - Background HTTP polling thread (~10Hz), data calculation
  - `Controller.java` - Lifecycle manager, overlay coordination
  - `OverlayManager.java` - Manages overlay window visibility
  - `event/` - Event buses (`UIStateBus`, `FlightDataBus`)
  - `config/` - Configuration system (`ConfigurationService`)

- **`parser/`** - Data ingestion
  - `State.java`, `Indicators.java` - Game telemetry JSON parsers
  - `Blkx.java` - Flight model file parser
  - `FlightAnalyzer.java` - Derived metrics calculation

- **`ui/`** - User interface
  - `MainForm.java` - Settings/configuration window
  - `overlay/` - Real-time HUD overlays (`MiniHUDOverlay`, `EngineControlOverlay`, `FlightInfoOverlay`, `AttitudeOverlay`)
  - `overlay/logic/` - Pure calculation logic (`HUDCalculator`)
  - `layout/` - Dynamic UI generation from `ui_layout.cfg`
  - `layout/renderer/` - Config panel type renderers
  - `component/` - Reusable widgets (`LinearGauge`, `CompassGauge`)

### Data Flow

```
War Thunder (127.0.0.1:8111)
    ↓ HTTP GET
Service.java (background thread)
    ↓
FlightDataBus (events)
    ↓
Overlay renderers → Swing/WebLaF UI
```

### Key Configuration Files

- `ui_layout.cfg` - Dynamic UI layout (custom S-expression syntax)
- `lang/cur.properties` - UI localization (Chinese)
- `MANIFEST.MF` - JAR entry point: `prog.Application`

### Dependencies (`dep/`)

- `weblaf-complete-1.29.jar` - WebLaF UI framework
- `jnativehook-2.2.2.jar` - Global keyboard hooks

## Development Guidelines

### Threading

- **EDT Rule**: Swing components must be updated via `SwingUtilities.invokeLater`
- **OverlayManager**: Methods (`open`, `close`, `refreshPreview`) must be `synchronized` to prevent race conditions from rapid config change events
- Event subscribers may receive events on background threads - dispatch UI updates to EDT

### Performance

- Use dirty checking in UI components (store `lastValue`, only repaint when changed)
- Avoid object allocation in `paintComponent` or high-frequency loops
- `HUDCalculator` prepares raw data; components handle formatting in `onDataUpdate`

### Engine Type Filtering

`Service.java` forces metrics to `0.0` based on engine type for `hide-when-zero` logic:
- Jets: `ManifoldPressure`, `WaterTemp` → 0
- Props: `Thrust` → 0

### Config Renderers

Implement `RowRenderer` pattern: construct a `WebPanel` and bind to `ConfigService`.

### Module Dependency Graph

```
Application (Entry Point)
    ↓
Controller (Lifecycle Coordinator)
    ├→ Service (HTTP Data Polling) → FlightDataBus (Event Publisher)
    ├→ OverlayManager → [各种Overlay] → HUDComponent
    ├→ ConfigurationService → HUDSettings / OverlaySettings
    └→ MainForm (Settings UI)
```

### Common Feature Addition Paths

When adding a new configuration toggle, follow this typical modification path:

1. **`ui_layout.cfg`** - Add `(item ...)` definition with type, target, default value
2. **`HUDSettings.java`** or `OverlaySettings.java` - Add getter interface method
3. **`ConfigurationService.java`** - Implement the getter method
4. **Target Overlay** (e.g., `MiniHUDOverlay.java`) - Use config to control visibility/behavior
5. **`Controller.java`** - Add config key to `.withInterest()` for WYSIWYG preview refresh

Example from `showSpeedBar` toggle:
```java
// Controller.java - Register interest for live preview
.withInterest("displayCrosshair", "drawHUD", ..., "showSpeedBar");

// MiniHUDOverlay.java - Use in updateComponents()
boolean showSpeed = hudSettings.showSpeedBar();
speedRatioBar.setVisible(textVisible && showSpeed);
throttleBar.setVisible(textVisible && !showSpeed);
```

### Sub-Module Documentation

Detailed development guides for complex subsystems:

| Module | Documentation |
|--------|---------------|
| Config System | [`src/prog/config/CLAUDE.md`](src/prog/config/CLAUDE.md) |
| Overlay Development | [`src/ui/overlay/CLAUDE.md`](src/ui/overlay/CLAUDE.md) |
| HUD Components | [`src/ui/component/CLAUDE.md`](src/ui/component/CLAUDE.md) |
| MiniHUD Architecture | [`doc/minihud贡献者开发手册.md`](doc/minihud贡献者开发手册.md) |
