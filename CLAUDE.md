# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VoidMei is a Java Swing telemetry overlay for War Thunder. It reads real-time flight data from the game's local HTTP API (port 8111) and displays HUD overlays with flight metrics, warnings, and aircraft performance data.

**Project Statistics:** ~159 Java files, ~33,500 lines of code

## 写代码时, 关键的地方和问题修复一定要添加和补充中文注释

## Build Commands

不要运行./script/build.sh, 但是可以用javac确认编译通过

**Java 8 Required:** VoidMei strictly requires Java 8 (1.8.x). The Windows EXE enforces `maxVersion: 1.8.999` to prevent running on Java 9+, which has incompatible module changes.

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

# Mock server for testing (simulates War Thunder API)
python3 script/mock_8111.py

# Run unit tests
./script/test.sh              # Run all tests
./script/test.sh atmosphere   # Run AtmosphereModel tests only
./script/test.sh piston       # Run PistonPowerModel tests only
./script/test.sh visibility   # Run VisibilityExpressionEvaluator tests only
```

**Unit tests** available for utility classes in `test/`. Integration testing is manual via the running application or mock server.

### Windows Launch Scripts

VoidMei provides multiple ways to launch on Windows:

| File | Purpose |
|------|---------|
| `VoidMei.exe` | Launch4j-wrapped executable with Java 8 enforcement |
| `VoidMei.bat` | Intelligent batch script that finds Java 8 from registry |
| `script/build.cmd` | Simple Windows build script |

**VoidMei.bat** searches Windows Registry for Java 8 (Oracle, Temurin, Zulu, Corretto, Microsoft), then falls back to `%JAVA_HOME%` or `PATH`.

**voidmeil4j.xml** (Launch4j configuration):
- `minVersion: 1.8.0`, `maxVersion: 1.8.999` - Strictly enforces Java 8
- `jreVersionErr` message guides users to download Eclipse Temurin 8
- JVM flags: `-Dsun.java2d.uiScale=1 -Xms64m -Xmx320m`

## Architecture

### Core Packages (`src/`)

- **`prog/`** - Application kernel (14 files + 7 subpackages)
  - `Launcher.java` - Bootstrap entry point, sets GPU compat JVM properties before AWT loads
  - `Application.java` - Main application initialization, global config, fonts, logging
  - `Service.java` - Background HTTP polling thread (~10Hz), data calculation (~55KB, largest file)
  - `Controller.java` - Lifecycle manager, overlay coordination (~24KB)
  - `OverlayManager.java` - Manages overlay window visibility (synchronized for thread safety)
  - `OverlayContext.java` - Context object for overlay rendering
  - `ControllerState.java` - State machine for controller lifecycle
  - `ActivationStrategy.java` - Conditional overlay activation logic
  - `AlwaysOnTopCoordinator.java` - Singleton z-order manager for overlay/dialog coordination
  - `FocusMonitor.java` - Game window focus tracking for auto-hide overlay feature
  - `event/` - Event buses (`UIStateBus`, `FlightDataBus`, `FlightDataEvent`, `EventPayload`, `FlightDataListener`)
  - `config/` - Configuration system (`ConfigurationService`, `ConfigLoader`, `SExpParser`, `HUDSettings`, `OverlaySettings`)
  - `audio/` - Voice warning system (`VoiceWarning`, `VoiceResourceManager`)
  - `util/` - Utilities (`HttpHelper`, `Logger`, `CalcHelper`, `StringHelper`, `FileUtils`, `FormulaEvaluator`, `PhysicsConstants`, `Interpolation`, `AtmosphereModel`, `PistonPowerModel`, `ColorHelper`, `GPUCompatibilityHelper`, `DPIHelper`, `FocusDetector`, `ExceptionHelper`)
  - `hotkey/` - Global keyboard hooks (`HotkeyManager`)
  - `i18n/` - Internationalization (`Lang`)
  - `model/` - Data models (`InfoList`)

- **`parser/`** - Data ingestion (8 files)
  - `State.java`, `Indicators.java` - Game telemetry JSON parsers
  - `Blkx.java` - Flight model file (.blk) parser
  - `FlightAnalyzer.java` - Derived metrics calculation
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
  - `layout/renderer/` - Config panel type renderers (17 types):
    - `SwitchRowRenderer`, `SwitchInvRowRenderer` - Boolean toggles
    - `SliderRowRenderer` - Numeric sliders
    - `ComboRowRenderer` - Dropdown selectors
    - `ColorRowRenderer` - Color pickers (hex/decimal input with graphical picker)
    - `ColorPickerPopup` - HSB palette popup with alpha slider and hex input
    - `TextRowRenderer` - Text inputs
    - `ButtonRowRenderer` - Action buttons
    - `HotkeyRowRenderer` - Keyboard shortcut binding
    - `DataRowRenderer` - Read-only data display
    - `FileListRowRenderer`, `FMListRowRenderer` - File/FM list selectors
    - `VoiceRowRenderer`, `VoiceGlobalRenderer` - Voice warning configuration
    - `RendererConfigHelper` - Unified config read/write helper for renderers
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
  - `util/` - UI utilities (`FastNumberFormatter`, `GraphicsUtil`, `SliderHelper`, `OverlayStyleHelper`, `NotificationService`, `ReflectBinder`, `UIConstants`, `VisibilityExpressionEvaluator`)
  - `window/comparison/` - Aircraft comparison window (`CompactComparisonWindow`, logic/, model/)

### Data Flow

```
War Thunder HTTP API (127.0.0.1:8111)
    ↓ HTTP GET (~10Hz polling)
Service.java (background thread)
    ↓ Parse JSON (State.java, Indicators.java)
    ↓ Pre-compute HUDData (reduces EDT latency)
FlightDataBus (event publisher)
    ↓ FlightDataEvent (carries pre-computed HUDData)
Overlay components (FlightDataListener subscribers)
    ↓ SwingUtilities.invokeLater()
Swing/WebLaF UI (EDT thread)
```

**Performance Note:** HUDData is pre-computed on the Service thread before publishing, reducing EDT latency by ~40-60ms. Overlays access this via `event.getHudData()` instead of computing on the EDT.

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

### Tray Icon Click Race Prevention

Use `AtomicBoolean` with CAS in `Application.java` to prevent duplicate `Controller` creation from rapid tray icon clicks:

```java
private static final AtomicBoolean trayClickProcessing = new AtomicBoolean(false);
// In mouseClicked: if (!trayClickProcessing.compareAndSet(false, true)) return;
// try { ctr.stop(); ctr = new Controller(); } finally { trayClickProcessing.set(false); }
```

**Controller.stop() cleanup order:** 1) Close overlays + invalidate generation counter, 2) Unsubscribe events, 3) Dispose MainForm, 4) Stop Service thread, 5) Save config.

### Preview Generation Counter (Stale Callback Detection)

`Controller` uses `AtomicLong previewGeneration` to prevent stale EDT callbacks from creating preview overlays after switching to game mode. Pattern: capture generation at async start, check before execution, increment on `endPreview()` to invalidate pending callbacks.

### Preview Mode FM Fallback

在`Controller.getBlkx()`中，如果预览模式下FM解析失败，会回退到`selectedFM0`配置的默认飞机。仅在`PREVIEW`状态触发，避免影响游戏模式。

### Overlay Z-Order (AlwaysOnTopCoordinator)

Use `AlwaysOnTopCoordinator` singleton to manage overlay z-order and dialog coordination:

```java
// Overlay init: AlwaysOnTopCoordinator.getInstance().registerOverlay(this);
// Overlay dispose: AlwaysOnTopCoordinator.getInstance().unregisterOverlay(this);
// Before dialog: dialogWillShow(); try { dialog.setVisible(true); } finally { dialogDidDismiss(); }
```

**Key behaviors:** `registerOverlay()` tracks windows with `WeakReference`; `dialogWillShow()`/`dialogDidDismiss()` suspend/restore `alwaysOnTop` for dialogs. Thread-safe via `AtomicInteger` + `CopyOnWriteArrayList`.

**焦点抢占防护：** 1) `setFocusable(false)` 必须在 `registerOverlay()` 之前; 2) 循环中检查 `!isVisible()` 后才调用 `setVisible(true)`。

### Focus Monitor (游戏失焦自动隐藏)

`FocusMonitor` (200ms节流) + `FocusDetector` (跨平台检测) 实现Alt+Tab时自动隐藏overlay。复用Service的~10Hz轮询，无新线程。

| 平台 | 检测方法 | 进程名 |
|------|----------|--------|
| Windows | PowerShell GetForegroundWindow | `aces` |
| Linux | xdotool | `war thunder` |
| macOS | AppleScript | `war thunder`/`aces` |

启用：`S.getFocusMonitor().setEnabled(true)`；禁用时自动恢复overlay。

### GPU Compatibility Mode

Disables Java2D hardware acceleration to prevent GPU conflicts. `Launcher.java` (no AWT imports) sets `sun.java2d.*` properties **before** any AWT class loads, then calls `Application.main()`.

| OS | Properties disabled |
|----|---------------------|
| Windows | `d3d`, `noddraw` |
| Linux | `opengl`, `xrender` |
| macOS | `UseQuartz` |

**Usage:** `GPUCompatibilityHelper.saveSettings(true/false)` saves to `gpu_compat.properties`; `isEnabled()` reads setting; `isSoftwareRenderingActive()` checks runtime state. The `gpuCompatibilityMode` config key in `SwitchRowRenderer` has special handling (reads from Helper, not ui_layout.cfg).

### DPI Scaling (High-DPI Display Support)

`DPIHelper` detects display scaling via `GraphicsConfiguration.getDefaultTransform()` and exposes `Application.dpiScale` (1.0=100%, 2.0=200%), `logicalWidth`, `logicalHeight`.

**Usage:**
```java
double scale = Application.dpiScale;
int fontSize = (int) Math.round((24 + fontadd) * scale);
// Or: int scaled = DPIHelper.scale(24);
```

**JVM flag** `-Dsun.java2d.uiScale=1` (in voidmeil4j.xml) disables Java's auto-scaling for crisp font rendering. At 100% scaling, all calculations match pre-DPI code.

### Performance

- **HUDData Pre-computation**: `Service.java` pre-computes `HUDData` on the background thread before publishing `FlightDataEvent`. This offloads ~40-60ms of calculation from the EDT.
- **Dirty Checking**: UI components should store `lastValue` and only repaint when data changes
- **Zero Allocation**: Avoid object allocation in `paintComponent` or high-frequency loops
- `HUDCalculator` prepares raw data on Service thread; components consume via `event.getHudData()`

```java
// MiniHUDOverlay: Use pre-computed HUDData from event
@Override
public void onFlightData(FlightDataEvent event) {
    HUDData data = event.getHudData();  // Pre-computed on Service thread
    if (data == null) return;
    SwingUtilities.invokeLater(() -> updateComponents(data));
}
```

### Engine Type Filtering

`Service.java` forces metrics to `0.0` based on engine type for `hide-when-zero` logic:
- Jets: `ManifoldPressure`, `WaterTemp` → 0
- Props: `Thrust` → 0

### Service.java Field Naming Conventions

`Service.java` uses clean camelCase naming for public fields. **Do not use Hungarian notation** (e.g., `iCount`, `sName`, `bFlag`).

#### API Objects (kept as-is for clarity)
| Field | Description |
|-------|-------------|
| `sState` | State object from `/state` API endpoint |
| `sIndic` | Indicators object from `/indicators` API endpoint |

#### Numeric Fields (use plain camelCase)
| Field | Type | Description |
|-------|------|-------------|
| `totalHp` | `int` | Total horsepower |
| `totalHpEff` | `int` | Effective horsepower |
| `totalThrust` | `int` | Total thrust (kgf) |
| `totalFuel` | `double` | Total fuel (kg) |
| `totalFuelPrev` | `double` | Previous fuel reading (for delta calculation) |
| `fuelDelta` | `double` | Fuel consumption rate |
| `checkAlt` | `int` | Altitude check counter |
| `prevEnergyJKg` | `double` | Previous specific energy |
| `compassDelta` | `double` | Compass heading delta |

#### String Display Fields (use `Str` suffix)
| Field | Type | Description |
|-------|------|-------------|
| `totalHpStr` | `String` | Formatted total HP for display |
| `totalHpEffStr` | `String` | Formatted effective HP for display |
| `totalThrustStr` | `String` | Formatted thrust for display |
| `totalFuelStr` | `String` | Formatted fuel for display |
| `fueltimeStr` | `String` | Formatted fuel time for display |
| `statusText` | `String` | Status text |
| `timeText` | `String` | Elapsed time text |

#### Boolean Fields (use verb prefixes like `is`, `has`, `use`)
| Field | Type | Description |
|-------|------|-------------|
| `useMegaHp` | `boolean` | Whether to display HP in MHp units |
| `lowAccFuel` | `boolean` | Low fuel accuracy warning flag |

**Naming Rules:**
1. **Numeric values**: Use plain camelCase (`totalHp`, not `iTotalHp`)
2. **String representations**: Add `Str` suffix (`totalHpStr`, not `sTotalHp`)
3. **Booleans**: Use verb prefixes (`useMegaHp`, not `bUnitMHp`)
4. **Previous/Delta values**: Use `Prev` or `Delta` suffix (`totalFuelPrev`, `fuelDelta`)

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

Use `prog.util.Interpolation` for all interpolation: `lerp(x, x0, y0, x1, y1)`, `interp1d(x, xs, ys)`, `interp2d(x, y, xs, ys, zz)`, `interpSweepLevel(...)`. Never duplicate interpolation logic.

### Atmosphere Model

Use `prog.util.AtmosphereModel` for ISA calculations: `pressure(alt)`, `density(p, tempSL, alt)`, `iasToTas(ias, rho)`, `tasToIas(tas, rho)`, `ramEffectAltitude(...)`.

### Piston Power Model

Use `prog.util.PistonPowerModel` for piston engine power curves: `powerAtAltitudeAdvanced(stage, alt, wep, speed, ramEffect, tempSL)`, `optimalPowerAdvanced(stages[], ...)`. `CompressorStageParams` must be populated from FM data. See [`src/prog/util/CLAUDE.md`](src/prog/util/CLAUDE.md).

### Color Utilities

Use `prog.util.ColorHelper`: `parseColor(str, default)` accepts hex (`#RRGGBBAA`) or decimal (`R, G, B, A`); `toHexString(color, withAlpha)` for display; `toDecimalString(color)` for config storage (backward compatible).

### Exception Handling

Use `prog.util.ExceptionHelper` for consistent exception handling:

```java
// Replace verbose try-catch Thread.sleep with:
ExceptionHelper.sleepQuietly(100);  // Silently handles InterruptedException

// Log exceptions without disrupting control flow:
catch (Exception e) {
    ExceptionHelper.logAndContinue(e, "文件操作");  // Logs at WARN level
}

// Safely close resources in finally blocks:
finally {
    ExceptionHelper.closeQuietly(stream);
}
```

**Avoid:** Empty catch blocks with `// TODO Auto-generated catch block` comments.

### Logger Levels

Use `prog.util.Logger` with appropriate levels:

```java
Logger.trace("详细跟踪信息");           // TRACE: Only for deep debugging
Logger.debug("调试信息");              // DEBUG: Development debugging
Logger.info("Service", "启动成功");    // INFO: Normal operation (default level)
Logger.warn("配置缺失: " + key);       // WARN: Non-fatal issues
Logger.error("操作失败", exception);   // ERROR: Fatal issues with stack trace
```

### UI Utility Helpers

| Helper | Usage |
|--------|-------|
| `OverlayStyleHelper` | `applyTransparentStyle(window)`, `applyPreviewStyle(window)`, `loadFontConfig(settings)` |
| `SliderHelper` | `configureVerticalProgress(...)`, `configureAttitudeSlider(...)`, `removeAllListeners(slider)` |
| `GraphicsUtil` | `configureOverlayRendering(g2d)`, `createPreciseStroke(width)` |
| `UIConstants` | DPI scaling constants (`BASE_SCREEN_HEIGHT`, `BASE_FONT_SIZE`), time delays (`DELAY_SHORT_MS`, etc.) |

### Config Renderers

Implement `RowRenderer` interface: `render(RowConfig, ConfigProvider) → WebPanel`. Register in `RowRendererRegistry.java` with type key (e.g., `"switch"`, `"slider"`).

**RendererConfigHelper** provides unified config read/write for renderers:

```java
// Read with priority: PropertyBinder → ConfigurationService → default
int val = RendererConfigHelper.readInt(context, groupConfig, row, defaultVal);
String str = RendererConfigHelper.readString(context, groupConfig, row, defaultStr);
boolean bool = RendererConfigHelper.readBool(context, groupConfig, row, defaultBool);

// Write (syncs to both PropertyBinder and ConfigurationService)
RendererConfigHelper.writeInt(context, groupConfig, property, value);
RendererConfigHelper.writeString(context, groupConfig, property, value);
RendererConfigHelper.writeBool(context, groupConfig, property, value);
```

### Module Dependency Graph

```
Application (Entry Point)
    ↓
Controller (Lifecycle Coordinator)
    ├→ Service (HTTP Data Polling)
    │       ├→ State/Indicators (JSON Parsers)
    │       ├→ HUDCalculator (pre-computes HUDData)
    │       ├→ FlightDataBus (publishes event with HUDData)
    │       └→ FocusMonitor → FocusDetector (game focus detection)
    │
    ├→ OverlayManager (synchronized)
    │       ├→ MiniHUDOverlay → HUDComponent[] (consumes pre-computed HUDData)
    │       ├→ AttitudeOverlay, FlightInfoOverlay, ...
    │       └→ BaseOverlay → ZebraListRenderer
    │
    ├→ AlwaysOnTopCoordinator (Singleton z-order manager)
    │       ├← DrawFrame, DrawFrameSimpl (FM 曲线可视化窗口)
    │       ├← DialogService (dialog lifecycle hooks)
    │       └← FocusMonitor (hide/show on focus change)
    │
    ├→ ConfigurationService (implements ConfigProvider)
    │       ├→ HUDSettings (interface)
    │       ├→ OverlaySettings (interface)
    │       └→ ConfigLoader → SExpParser → ui_layout.cfg
    │
    ├→ MainForm (Settings UI)
    │       └→ UIBuilder → RowRenderer[] → ColorPickerPopup
    │
    └→ VoiceWarning (Audio alerts)
            └→ ConfigProvider (配置访问接口)
```

### ConfigProvider 架构

**重要**: Controller 不再实现 ConfigProvider 接口。Overlay 组件应遵循以下解耦模式：

1. **位置保存** - 使用 `OverlaySettings.saveWindowPosition()`，不通过 Controller
2. **配置读取** - 使用 `ConfigProvider` 接口（通过 `c.getConfigProvider()` 获取）
3. **特殊配置** - 如 `HUDSettings`，应通过 init() 参数直接传入

```java
// ✅ 正确：位置保存使用 OverlaySettings（DraggableOverlay 父类已正确实现）
public void init(Controller c, Service s, OverlaySettings settings) {
    this.config = c.getConfigProvider();  // 配置访问
    this.controller = c;                   // FM 数据访问 (如 getBlkx())
    setOverlaySettings(settings);          // 位置保存通过 OverlaySettings

    // 使用父类方法，不通过 Controller 访问 configService
    this.onPositionSave = this::saveCurrentPosition;
}

// ✅ 正确：HUDSettings 通过 init() 参数传入
public void init(Controller c, Service s, HUDSettings hudSettings) {
    this.hudSettings = hudSettings;  // 直接传入，不从 Controller 获取
}

// ❌ 错误：通过 Controller 访问 configService（违反解耦原则）
this.onPositionSave = () -> c.getConfigService().saveLayoutConfig();
configService = controller.getConfigService();  // 不应在运行时从 Controller 获取

// ❌ 错误：将 config 强转为 Controller
prog.Controller ctrl = (prog.Controller) config;  // ClassCastException!
```

**职责分离原则:**

| 字段 | 类型 | 用途 |
|------|------|------|
| `config` | `ConfigProvider` | 配置读写 (`getConfig`, `setConfig`) |
| `controller` | `Controller` | FM 数据访问 (`getBlkx`, `getCompressorStages`) |
| `overlaySettings` | `OverlaySettings` | 分组配置 (位置、字体等)，通过 init() 传入 |
| `hudSettings` | `HUDSettings` | HUD 专用配置，通过 init() 参数传入 |

**核心原则**: 配置通过 init() 参数传入，不应在运行时从 Controller 获取服务。

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
Service Thread (background)
    └─ HUDCalculator.calculate() → HUDData (pre-computed)
           ↓ FlightDataEvent.setHudData()
MiniHUDOverlay (EDT)
    ├─ MinimalHUDContext (immutable config snapshot)
    ├─ ModernHUDLayoutEngine (DAG-based relative positioning)
    └─ HUDComponent[] (pluggable visual components)
           ↑ onDataUpdate(HUDData) - consumes pre-computed data
```

**Key differences from BaseOverlay:**
- **Pre-computed HUDData**: Calculation happens on Service thread, not EDT
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
| UI Utilities | [`src/ui/util/CLAUDE.md`](src/ui/util/CLAUDE.md) |
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

;; Numeric slider with unit
(item "Size" :type slider :target "size" :min 1 :max 100 :unit "px" :value 50)

;; Dropdown
(item "Style" :type combo :target "style" :options ("A" "B" "C") :value "A")

;; Color picker (hex preferred, decimal also supported)
(item "Color" :type color :target "colorKey" :value "#FFC864FF")

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

;; Data field with visibility expression (engine type aware)
(item "功率" :type data :target "getHorsePower"
      :visible-when (and (not (isJetEngine)) (> value 0))
      :unit "Hp")  ; Show for prop aircraft only when value > 0

;; Visibility expression operators: (not), (and), (or), (> value N), (>= value N),
;; (< value N), (<= value N), (= value N), (!= value N)
;; TelemetrySource methods: (isJetEngine), (isPropEngine), (hasWep), (isEngineCheckDone)
```
