# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VoidMei is a Java Swing telemetry overlay for War Thunder. It reads real-time flight data from the game's local HTTP API (port 8111) and displays HUD overlays with flight metrics, warnings, and aircraft performance data.

**Project Statistics:** ~155 Java files, ~34,000 lines of code

## Build Commands

不要运行./script/build.sh, 但是可以用javac确认编译通过

```bash
# Compile (requires JDK 1.8+)
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

# Mock server for testing (simulates War Thunder API)
python3 script/mock_8111.py

# Run unit tests
./script/test.sh              # Run all tests
./script/test.sh atmosphere   # Run AtmosphereModel tests only
./script/test.sh piston       # Run PistonPowerModel tests only
```

**Unit tests** available for utility classes in `test/`. Integration testing is manual via the running application or mock server.

## Architecture

### Core Packages (`src/`)

- **`prog/`** - Application kernel (11 files + 7 subpackages)
  - `Application.java` - Entry point, global config, fonts, logging
  - `Service.java` - Background HTTP polling thread (~10Hz), data calculation (~55KB, largest file)
  - `Controller.java` - Lifecycle manager, overlay coordination (~24KB)
  - `OverlayManager.java` - Manages overlay window visibility (synchronized for thread safety)
  - `OverlayContext.java` - Context object for overlay rendering
  - `ControllerState.java` - State machine for controller lifecycle
  - `ActivationStrategy.java` - Conditional overlay activation logic
  - `event/` - Event buses (`UIStateBus`, `FlightDataBus`, `FlightDataEvent`, `EventPayload`, `FlightDataListener`)
  - `config/` - Configuration system (`ConfigurationService`, `ConfigLoader`, `SExpParser`, `HUDSettings`, `OverlaySettings`)
  - `audio/` - Voice warning system (`VoiceWarning`, `VoiceResourceManager`)
  - `util/` - Utilities (`HttpHelper`, `Logger`, `CalcHelper`, `StringHelper`, `FileUtils`, `FormulaEvaluator`, `PhysicsConstants`, `Interpolation`, `AtmosphereModel`, `PistonPowerModel`)
  - `hotkey/` - Global keyboard hooks (`HotkeyManager`)
  - `i18n/` - Internationalization (`Lang`)
  - `model/` - Data models (`InfoList`)

- **`parser/`** - Data ingestion (9 files)
  - `State.java`, `Indicators.java` - Game telemetry JSON parsers
  - `Blkx.java` - Flight model file (.blk) parser
  - `FlightAnalyzer.java` - Derived metrics calculation
  - `FlightModelParser.java` - War Thunder flight model parsing
  - `FlightLog.java` - Flight data logging
  - `HudMsg.java`, `MapInfo.java`, `MapObj.java` - Additional data structures

- **`ui/`** - User interface (4 root files + 9 subpackages)
  - `MainForm.java` - Settings/configuration window
  - `StatusBar.java` - Status bar component
  - `UIBaseElements.java` - Base UI element definitions
  - `WebLafSettings.java` - WebLaF theme configuration
  - `overlay/` - Real-time HUD overlays:
    - `MiniHUDOverlay.java` - Primary HUD (~28KB, component-based architecture)
    - `AttitudeOverlay.java` - Artificial horizon
    - `EngineControlOverlay.java` - Engine gauges
    - `FlightInfoOverlay.java` - Flight data display
    - `ControlSurfacesOverlay.java` - Control surface indicators
    - `GearFlapsOverlay.java` - Landing gear/flaps status
    - `PowerInfoOverlay.java` - Engine power metrics
    - `FMUnpackedDataOverlay.java` - Flight model debug display
    - `BaseOverlay.java` - Standard overlay base class
    - `DrawFrame.java`, `DrawFrameSimpl.java` - Rendering interfaces
  - `overlay/logic/` - Pure calculation logic (`HUDCalculator`)
  - `overlay/model/` - HUD data models (`HUDData`)
  - `layout/` - Dynamic UI generation from `ui_layout.cfg` (`UIBuilder`, `ModernHUDLayoutEngine`, `HUDLayoutNode`)
  - `layout/renderer/` - Config panel type renderers (15 types):
    - `SwitchRowRenderer`, `SwitchInvRowRenderer` - Boolean toggles
    - `SliderRowRenderer` - Numeric sliders
    - `ComboRowRenderer` - Dropdown selectors
    - `ColorRowRenderer` - Color pickers
    - `TextRowRenderer` - Text inputs
    - `ButtonRowRenderer` - Action buttons
    - `HotkeyRowRenderer` - Keyboard shortcut binding
    - `DataRowRenderer` - Read-only data display
    - `FileListRowRenderer`, `FMListRowRenderer` - File/FM list selectors
    - `VoiceRowRenderer`, `VoiceGlobalRenderer` - Voice warning configuration
  - `component/` - Reusable HUD widgets:
    - `LinearGauge`, `LabeledLinearGauge` - Bar gauges
    - `CompassGauge` - Heading indicator
    - `CrosshairGauge` - Aiming reticle
    - `AttitudeIndicatorGauge` - Artificial horizon
    - `SpeedRatioBar`, `FlapAngleBar` - Specialized bars
    - `TextGauge` - Numeric readouts
    - `WarningOverlay` - Warning display
    - `row/` - HUD row components (`HUDRow`, `HUDTextRow`, `HUDAkbRow`, `HUDEnergyRow`, `HUDFlapsRow`, `HUDManeuverRow`)
  - `base/` - Base overlay classes (`DraggableOverlay`, `FieldOverlay`)
  - `renderer/` - Rendering implementations (`OverlayRenderer`, `LinearGaugeRenderer`, `BOSStyleRenderer`, `TextOnlyRenderer`)
  - `model/` - UI data models (`FieldManager`, `FlightDataProvider`, `ServiceDataAdapter`, `GaugeField`, `FieldDefinition`)
  - `replica/` - UI template/replica system (`ReplicaBuilder`, `ReplicaPanel`, `PinkStyle`)
  - `util/` - UI utilities (`FastNumberFormatter`, `GraphicsUtil`, `NotificationService`, `ReflectBinder`)
  - `window/comparison/` - Aircraft comparison window (`ComparisonFrame`, `ComparisonTable`, `CompactComparisonWindow`, logic/, model/)

### Data Flow

```
War Thunder HTTP API (127.0.0.1:8111)
    ↓ HTTP GET (~10Hz polling)
Service.java (background thread)
    ↓ Parse JSON (State.java, Indicators.java)
FlightDataBus (event publisher)
    ↓ FlightDataEvent
Overlay components (FlightDataListener subscribers)
    ↓ SwingUtilities.invokeLater()
Swing/WebLaF UI (EDT thread)
```

### Key Configuration Files

- `ui_layout.cfg` - Dynamic UI layout (custom S-expression DSL, ~25KB)
- `lang/cur.properties` - UI localization (Chinese, ~12KB)
- `MANIFEST.MF` - JAR entry point: `prog.Application`

### Dependencies (`dep/`)

- `weblaf-complete-1.29.jar` (5.6 MB) - WebLaF modern Swing UI framework
- `jnativehook-2.2.2.jar` (673 KB) - Global keyboard/mouse hooks

### Directory Structure

```
voidmei/
├── src/                    # Source code (144 Java files)
│   ├── prog/               # Application kernel
│   ├── parser/             # Data parsing layer
│   └── ui/                 # User interface
├── test/                   # Unit tests (TestAtmosphereModel, TestPistonPowerModel)
├── dep/                    # JAR dependencies
├── doc/                    # Chinese development guides
├── script/                 # Build scripts, mock server & test runner
├── lang/                   # Localization resources
├── image/                  # Image assets
├── fonts/                  # Custom fonts
├── bin/                    # Compiled classes (output)
├── ui_layout.cfg           # UI configuration
├── MANIFEST.MF             # JAR manifest
├── VoidMei.jar             # Built application
└── VoidMei.exe             # Windows executable
```

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

### Physics Constants

Use `prog.util.PhysicsConstants` for physical constants to ensure consistency across the codebase:

```java
import static prog.util.PhysicsConstants.g;

// Use in physics calculations
double energy = velocity * velocity / (2 * g);
double turnRadius = speedv * speedv / (g * loadFactor);
```

Available constants:
- `G` / `g` - Gravitational acceleration (9.80 m/s²)

**Never hardcode** values like `9.78f` or `9.80` directly in code.

### Interpolation Utilities

Use `prog.util.Interpolation` for all interpolation operations to ensure consistency and avoid code duplication:

```java
import static prog.util.Interpolation.lerp;
import static prog.util.Interpolation.interpSweepLevel;

// Linear interpolation between two points
double y = lerp(x, x0, y0, x1, y1);

// 1D table lookup with boundary clamping
double result = Interpolation.interp1d(x, xArray, yArray);

// 2D bilinear interpolation (e.g., thrust tables)
double thrust = Interpolation.interp2d(altitude, velocity, altitudes, velocities, thrustTable);

// Zero-allocation sweep interpolation for variable-geometry wings
double vne = interpSweepLevel(vwing, sweepLevels,
    level -> level.vne,           // value extractor
    level -> level.sweep,         // sweep extractor
    defaultVne);
```

Available methods:
- `lerp(x, x0, y0, x1, y1)` - Linear interpolation between two points
- `slope(x0, y0, x1, y1)` - Calculate slope (dy/dx) between two points
- `interp1d(x, xs, ys)` - 1D table interpolation with boundary clamping
- `interp1d(x, xs, ys, extrapolate)` - 1D interpolation with optional extrapolation
- `interp2d(x, y, xs, ys, zz)` - 2D bilinear interpolation for table lookup
- `interpSweepLevel(vwing, levels, valueExtractor, sweepExtractor, default)` - Zero-allocation sweep interpolation

**Never duplicate** interpolation logic - use these utilities instead.

### Atmosphere Model

Use `prog.util.AtmosphereModel` for ISA standard atmosphere calculations:

```java
import static prog.util.AtmosphereModel.*;

// Pressure at altitude (relative, sea level = 1.0)
double p = pressure(5000);  // → 0.533

// Air density
double rho = density(p, 15.0, 5000);  // sea level temp 15°C

// Airspeed conversions
double tas = iasToTas(400, rho);  // IAS → TAS
double ias = tasToIas(500, rho);  // TAS → IAS

// RAM effect equivalent altitude (for supercharger calculations)
double effectiveAlt = ramEffectAltitude(5000, 15.0, 500, true, 0.9);
```

### Piston Power Model

Use `prog.util.PistonPowerModel` for piston engine power curve calculations:

```java
import static prog.util.PistonPowerModel.*;

// Create compressor stage parameters (from FM data)
CompressorStageParams stage = new CompressorStageParams(7000, 2000, 1800);
stage.wepCritAlt = 6000;
stage.wepPowerMult = 1.15;

// Calculate power at altitude/speed
double power = powerAtAltitudeAdvanced(stage, 5000, true, 400, true, 15.0);

// For multi-stage superchargers
CompressorStageParams[] stages = {stage1, stage2};
double optPower = optimalPowerAdvanced(stages, 5000, false, 0, false, 15.0);
```

> **Note:** `PistonPowerModel` is a calculation engine only. `CompressorStageParams` must be populated from FM file data externally. See [`src/prog/util/CLAUDE.md`](src/prog/util/CLAUDE.md) for details.

### Config Renderers

Implement `RowRenderer` pattern: construct a `WebPanel` and bind to `ConfigService`.

```java
public class MyRowRenderer implements RowRenderer {
    @Override
    public WebPanel render(RowConfig config, ConfigProvider provider) {
        WebPanel panel = new WebPanel();
        // Build UI and bind to provider.getConfig() / provider.setConfig()
        return panel;
    }
}
```

Register in `RowRendererRegistry.java` with type key (e.g., `"switch"`, `"slider"`).

### Module Dependency Graph

```
Application (Entry Point)
    ↓
Controller (Lifecycle Coordinator)
    ├→ Service (HTTP Data Polling) → FlightDataBus (Event Publisher)
    │       ↑
    │   State/Indicators (JSON Parsers)
    │
    ├→ OverlayManager (synchronized)
    │       ├→ MiniHUDOverlay → HUDComponent[] → HUDCalculator
    │       ├→ AttitudeOverlay, FlightInfoOverlay, ...
    │       └→ BaseOverlay → ZebraListRenderer
    │
    ├→ ConfigurationService
    │       ├→ HUDSettings (interface)
    │       ├→ OverlaySettings (interface)
    │       └→ ConfigLoader → SExpParser → ui_layout.cfg
    │
    ├→ MainForm (Settings UI)
    │       └→ UIBuilder → RowRenderer[]
    │
    └→ VoiceWarning (Audio alerts)
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

### MiniHUD Component Architecture

MiniHUD uses a **component-based architecture** distinct from `BaseOverlay`:

```
MiniHUDOverlay
    ├─ MinimalHUDContext (immutable config snapshot)
    ├─ HUDCalculator (pure computation, no UI)
    ├─ ModernHUDLayoutEngine (DAG-based relative positioning)
    └─ HUDComponent[] (pluggable visual components)
```

**Key differences from BaseOverlay:**
- No `dataPanel` or `ZebraListRenderer`
- Custom `paintComponent()` drives all rendering
- Layout computed via topological sort of anchor dependencies
- Components are stateless - receive data via `onDataUpdate(HUDData)`

### Overlay Registration

In `Controller.registerGameModeOverlays()`:

```java
overlayManager.registerWithPreview(
    "configSwitchKey",           // Config key that enables this overlay
    () -> new MyOverlay(),       // Factory: creates new instance
    overlay -> overlay.init(this, S, settings),   // Game mode init
    overlay -> overlay.initPreview(this, settings), // Preview mode init
    overlay -> overlay.reinitConfig(),  // Config reload (WYSIWYG)
    true                         // previewEnabled
)
.withInterest("key1", "key2");   // Config keys that trigger reinitConfig
```

### Activation Strategies

For overlays with complex visibility conditions:

```java
// Game mode only
ActivationStrategy.config("enableFeature")
    .and(ActivationStrategy.gameModeOnly())

// Engine type specific
ActivationStrategy.config("showThrust")
    .and(ActivationStrategy.jetOnly())

ActivationStrategy.config("showManifold")
    .and(ActivationStrategy.propOnly())
```

### Sub-Module Documentation

Detailed development guides for complex subsystems:

| Module | Documentation |
|--------|---------------|
| Parser (FM/Telemetry) | [`src/parser/CLAUDE.md`](src/parser/CLAUDE.md) |
| Config System | [`src/prog/config/CLAUDE.md`](src/prog/config/CLAUDE.md) |
| Utility Classes | [`src/prog/util/CLAUDE.md`](src/prog/util/CLAUDE.md) |
| Overlay Development | [`src/ui/overlay/CLAUDE.md`](src/ui/overlay/CLAUDE.md) |
| UI Model & TelemetrySource | [`src/ui/model/CLAUDE.md`](src/ui/model/CLAUDE.md) |
| HUD Components | [`src/ui/component/CLAUDE.md`](src/ui/component/CLAUDE.md) |
| FM Comparison Rules | [`src/ui/window/comparison/CLAUDE.md`](src/ui/window/comparison/CLAUDE.md) |
| MiniHUD Architecture | [`doc/minihud贡献者开发手册.md`](doc/minihud贡献者开发手册.md) |
| VoidMei Contributor Guide | [`doc/voidmei贡献者开发手册.md`](doc/voidmei贡献者开发手册.md) |
| Algorithm Development | [`doc/物理人也能看懂的voidmei算法开发指导.md`](doc/物理人也能看懂的voidmei算法开发指导.md) |
| Debug Logging | [`doc/打桩调试手册.md`](doc/打桩调试手册.md) |
| Power Curve Debugging | [`doc/功率曲线调试手册.md`](doc/功率曲线调试手册.md) |
| Compressor Gauge Delay | [`doc/compressor_gauge_initial_delay.md`](doc/compressor_gauge_initial_delay.md) |

## Quick Reference

### Adding a New Config Toggle

1. `ui_layout.cfg` → Add `(item ... :type switch :target "myKey")`
2. `HUDSettings.java` → Add `boolean myKey();`
3. `ConfigurationService.java` → Implement `getBool("myKey", defaultValue)`
4. Target Overlay → Use `settings.myKey()` to control behavior
5. `Controller.java` → Add `"myKey"` to `.withInterest()` for WYSIWYG

### Adding a New Overlay

1. Create class extending `BaseOverlay` or `DraggableOverlay`
2. Implement `init()`, `initPreview()`, `reinitConfig()`, `dispose()`
3. Register in `Controller.registerGameModeOverlays()`
4. Add config switch in `ui_layout.cfg`

### Adding a New HUD Component

1. Implement `HUDComponent` interface (or extend `AbstractHUDComponent`)
2. Use `ctx.hudFontSize` for responsive sizing (never hardcode pixels)
3. Cache `Color`/`Font` objects (zero-allocation in `draw()`)
4. Instantiate in `MiniHUDOverlay.initComponentsLayout()`
5. Wire data updates in `MiniHUDOverlay.updateComponents()`

### Common S-Expression Item Types

```lisp
;; Boolean toggle
(item "Label" :type switch :target "key" :value true)

;; Inverted toggle (UI ON = value false)
(item "Disable X" :type switch_inv :target "disableX" :value false)

;; Numeric slider
(item "Size" :type slider :target "size" :min 1 :max 100 :value 50)

;; Dropdown
(item "Style" :type combo :target "style" :options ("A" "B" "C") :value "A")

;; Color picker
(item "Color" :type color :target "colorKey" :value "255,200,100,255")

;; Hotkey binding
(item "Toggle HUD" :type hotkey :target "hudHotkey")

;; Data field with static unit
(item "Speed" :type data :target "getIAS" :unit "Km/h" :precision 0)

;; Data field with dynamic unit/precision (for metric/imperial switching)
(item "进气压" :type data
      :target "getManifoldPressureDisplay"
      :unit-source "getManifoldPressureDisplayUnit"
      :precision-source "getManifoldPressureDisplayPrecision"
      :unit "Ata" :precision 2)  ; defaults for preview mode
```
