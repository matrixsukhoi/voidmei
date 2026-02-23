# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VoidMei is a Java Swing telemetry overlay for War Thunder. It reads real-time flight data from the game's local HTTP API (port 8111) and displays HUD overlays with flight metrics, warnings, and aircraft performance data.

**Project Statistics:** ~162 Java files, ~34,000 lines of code

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
```

**Unit tests** available for utility classes in `test/`. Integration testing is manual via the running application or mock server.

### Windows Launch Scripts

VoidMei provides multiple ways to launch on Windows:

| File | Purpose |
|------|---------|
| `VoidMei.exe` | Launch4j-wrapped executable with Java 8 enforcement |
| `VoidMei.bat` | Intelligent batch script that finds Java 8 from registry |
| `script/build.cmd` | Simple Windows build script |

**VoidMei.bat** searches the Windows Registry for Java 8 installations in this order:

1. Oracle JRE 1.8 (`HKLM\SOFTWARE\JavaSoft\Java Runtime Environment\1.8`)
2. Oracle JDK 1.8 (`HKLM\SOFTWARE\JavaSoft\JDK\1.8`)
3. Eclipse Temurin JRE 8 (`HKLM\SOFTWARE\Eclipse Adoptium\JRE\8`)
4. Eclipse Temurin JDK 8 (`HKLM\SOFTWARE\Eclipse Adoptium\JDK\8`)
5. Azul Zulu 8 (`HKLM\SOFTWARE\Azul Systems\Zulu\zulu-8`)
6. Amazon Corretto 8 (`HKLM\SOFTWARE\Amazon.com\Corretto\8`)
7. Microsoft OpenJDK 8 (`HKLM\SOFTWARE\Microsoft\JDK\8`)
8. Fallback to `%JAVA_HOME%` (with warning)
9. Fallback to `PATH` (with warning)

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
  - `util/` - Utilities (`HttpHelper`, `Logger`, `CalcHelper`, `StringHelper`, `FileUtils`, `FormulaEvaluator`, `PhysicsConstants`, `Interpolation`, `AtmosphereModel`, `PistonPowerModel`, `ColorHelper`, `GPUCompatibilityHelper`, `DPIHelper`, `FocusDetector`)
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
  - `layout/renderer/` - Config panel type renderers (16 types):
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
  - `util/` - UI utilities (`FastNumberFormatter`, `GraphicsUtil`, `SliderHelper`, `OverlayStyleHelper`, `NotificationService`, `ReflectBinder`)
  - `window/comparison/` - Aircraft comparison window (`ComparisonFrame`, `ComparisonTable`, `CompactComparisonWindow`, logic/, model/)

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

When users rapidly click the system tray icon, each click creates a new `Controller` instance with associated overlays. Without protection, overlays can stack up, leading to multiple duplicate windows.

**Problem solved:** Fast double-click on tray icon → two `Controller` instances created simultaneously → duplicate overlay windows.

**Solution:** Use `AtomicBoolean` with compare-and-set (CAS) operation to ensure only one click is processed at a time:

```java
// In Application.java
// 防止快速重复点击任务栏图标导致多次创建Controller
private static final AtomicBoolean trayClickProcessing = new AtomicBoolean(false);

// In tray icon mouse listener
icon.addMouseListener(new MouseAdapter() {
    @Override
    public void mouseClicked(MouseEvent e) {
        if (e.getButton() == MouseEvent.BUTTON1) {
            // 使用CAS操作防止快速重复点击导致多次创建Controller
            // compareAndSet: 如果当前值为false则设为true并返回true，否则返回false
            if (!trayClickProcessing.compareAndSet(false, true)) {
                prog.util.Logger.info("Application", "Ignoring duplicate tray click");
                return;
            }
            try {
                ctr.stop();
                ctr = new Controller();
            } finally {
                // 无论成功或异常都重置标志，允许下一次点击
                trayClickProcessing.set(false);
            }
        }
    }
});
```

**Controller.stop() 5-phase cleanup order:**

```java
public void stop() {
    // 1. 先关闭所有overlay（必须在dispose MainForm之前）
    if (State == ControllerState.PREVIEW) {
        previewGeneration.incrementAndGet();  // 使所有pending回调失效
        if (S != null) {
            closepad();  // 游戏模式：完整清理
        } else {
            overlayManager.closeAll();  // 预览模式：只需关闭overlay
        }
    }

    // 2. 取消事件订阅（防止重启时重复处理）
    UIStateBus.getInstance().unsubscribe(UIStateEvents.CONFIG_CHANGED, configChangedHandler);
    UIStateBus.getInstance().unsubscribe(UIStateEvents.UI_READY, uiReadyHandler);

    // 3. 清理MainForm
    if (M != null) { M.dispose(); M = null; }

    // 4. 清理Service线程
    S = null;
    if (S1 != null) { S1.interrupt(); S1 = null; }

    // 5. 保存配置
    configService.saveConfig();
}
```

**Key behaviors:**
- `compareAndSet(false, true)` - Atomic check-and-set prevents race conditions
- `finally` block ensures flag reset even if exception occurs
- `stop()` cleans up overlays BEFORE creating new Controller
- Generation counter invalidates stale async callbacks

### Preview Generation Counter (Stale Callback Detection)

The `Controller` uses a generation counter pattern to prevent race conditions when users quickly switch from Preview to Game mode before async preview creation completes.

**Problem solved:** On slow machines, `Preview()` spawns a background thread that schedules overlay creation on EDT. If users click "Start" before this completes, the stale EDT callback would create preview overlays *after* game mode has started.

```java
// In Controller.java
private final AtomicLong previewGeneration = new AtomicLong(0);

public void Preview() {
    State = ControllerState.PREVIEW;
    final long generation = previewGeneration.get();  // Capture
    new Thread(() -> refreshPreviews(generation)).start();
}

public void endPreview() {
    previewGeneration.incrementAndGet();  // Invalidate pending callbacks
    overlayManager.closeAll();
    State = ControllerState.INIT;
}

public void refreshPreviews(long generation) {
    // ... background work ...
    SwingUtilities.invokeLater(() -> {
        // Check for stale callback
        if (State != ControllerState.PREVIEW || previewGeneration.get() != generation) {
            Logger.info("Controller", "Skipping stale preview refresh");
            return;
        }
        overlayManager.refreshAllPreviews();
    });
}
```

**Key behaviors:**
- `previewGeneration.get()` - Captures current generation when starting async operation
- `previewGeneration.incrementAndGet()` - Called in `endPreview()` to invalidate all pending callbacks
- Defense-in-depth: `OverlayManager.refreshAllPreviews()` also checks `State != PREVIEW`

### Preview Mode FM Fallback

当用户在预览模式下打开设置界面时，VoidMei会尝试从游戏API获取当前飞机型号。如果解析失败（例如用户不在游戏中），会回退到配置中保存的`selectedFM0`飞机。

**Problem solved:** 用户打开设置界面时，如果无法连接游戏或FM文件解析失败，预览窗口将显示空白或错误。

**Solution:** 在`Controller.getBlkx()`中添加预览模式专用的回退逻辑：

```java
// In Controller.java
public synchronized Blkx getBlkx() {
    // ... 正常的FM加载逻辑 ...
    loadFMData(identifiedFMName);

    // 预览模式回退：解析失败时加载 selectedFM0 配置的默认飞机
    // 仅在预览模式下执行，避免影响游戏模式的正常行为
    if ((Blkx == null || !Blkx.valid) && State == ControllerState.PREVIEW) {
        String fallbackPlane = getConfig("selectedFM0");
        if (fallbackPlane != null && !fallbackPlane.isEmpty()
                && !fallbackPlane.equalsIgnoreCase(identifiedFMName)) {
            prog.util.Logger.info("Controller",
                "FM解析失败，回退到selectedFM0: " + fallbackPlane);
            loadFMData(fallbackPlane);
        }
    }

    return Blkx;
}
```

**Key behaviors:**
- Only triggers in `PREVIEW` state (settings UI), not during game mode
- Checks if `selectedFM0` differs from the failed aircraft to avoid infinite loop
- Logs the fallback for debugging purposes
- User can set their preferred default aircraft via the FM selection dropdown

### Overlay Z-Order (AlwaysOnTopCoordinator)

Use `AlwaysOnTopCoordinator` to manage `alwaysOnTop` state for all overlay windows. This coordinator solves the timing problem where dialogs triggered before overlays exist would be covered by later-created overlays.

```java
import prog.AlwaysOnTopCoordinator;

// In overlay window initialization (DrawFrame, DrawFrameSimpl, etc.)
AlwaysOnTopCoordinator.getInstance().registerOverlay(this);

// In overlay dispose()
AlwaysOnTopCoordinator.getInstance().unregisterOverlay(this);

// Before showing any dialog (DialogService, JColorChooser, etc.)
AlwaysOnTopCoordinator.getInstance().dialogWillShow();
try {
    dialog.setVisible(true);  // Blocks until dialog closes
} finally {
    AlwaysOnTopCoordinator.getInstance().dialogDidDismiss();
}
```

**Key behaviors:**
- `registerOverlay()` - Adds window to tracking; only sets `alwaysOnTop(true)` if no pending dialogs
- `dialogWillShow()` - Suspends all overlays' `alwaysOnTop` to let dialog appear on top
- `dialogDidDismiss()` - Restores `alwaysOnTop` when dialog count reaches zero
- Uses `WeakReference` to avoid memory leaks from disposed windows
- Thread-safe via `AtomicInteger` (dialog count) and `CopyOnWriteArrayList` (overlay list)

#### Preventing Overlay Focus Theft (焦点抢占问题修复)

某些窗口管理器下，alwaysOnTop窗口在显示或刷新时可能抢夺焦点，导致用户操作被中断。`DrawFrameSimpl`中实现了两个关键的防护模式：

**问题1：焦点属性设置顺序错误**

如果在`registerOverlay()`之后才设置`setFocusable(false)`，注册时的`setAlwaysOnTop(true)`可能已经触发了焦点事件。

```java
// 必须在 registerOverlay 之前设置焦点属性，否则 setAlwaysOnTop(true) 可能触发焦点事件
setFocusable(false);
setFocusableWindowState(false);  // 取消窗口焦点

AlwaysOnTopCoordinator.getInstance().registerOverlay(this);
```

**问题2：周期性`setVisible(true)`触发焦点抢占**

在`run()`循环中，重复调用`setVisible(true)`可能导致窗口管理器激活窗口。

```java
// In DrawFrameSimpl.run()
while (doit) {
    boolean shouldShow = isPreview || visible;
    if (shouldShow) {
        // 只在窗口不可见时才调用 setVisible(true)，避免周期性触发焦点抢占
        // 某些窗口管理器下，对 alwaysOnTop 窗口重复调用 setVisible(true) 会触发窗口激活事件
        if (!this.isVisible()) {
            this.setVisible(true);
        }
        this.getContentPane().repaint();
    } else {
        this.setVisible(false);
    }
    // ...
}
```

**关键模式总结：**

| 问题 | 解决方案 |
|------|----------|
| 焦点抢占 (注册时) | `setFocusable(false)` 必须在 `registerOverlay()` 之前 |
| 焦点抢占 (刷新时) | 检查 `!isVisible()` 后才调用 `setVisible(true)` |

### Focus Monitor (游戏失焦自动隐藏)

当用户Alt+Tab切换到其他应用时，VoidMei可以自动隐藏所有overlay窗口，避免遮挡其他应用；当War Thunder重新获得焦点时立即恢复显示。

**架构：**

```
Service.run() (后台线程, ~10Hz轮询)
    ↓ 每次轮询调用
focusMonitor.tick()
    ↓ FocusMonitor内部节流 (200ms一次)
FocusDetector.isWarThunderFocused()
    ↓ 状态变化时
AlwaysOnTopCoordinator.hideAllOverlays() / showAllOverlays()
    ↓
SwingUtilities.invokeLater(() -> setVisible())
```

**关键文件：**

| 文件 | 作用 |
|------|------|
| `prog/util/FocusDetector.java` | 跨平台前台窗口检测（Windows/Linux/macOS） |
| `prog/FocusMonitor.java` | 焦点监控辅助类，封装节流和状态追踪 |
| `prog/AlwaysOnTopCoordinator.java` | 提供`hideAllOverlays()`/`showAllOverlays()`方法 |

**平台检测机制：**

| 平台 | 检测方法 | 进程名匹配 |
|------|----------|------------|
| Windows | PowerShell + user32.dll GetForegroundWindow | `aces` |
| Linux | xdotool getactivewindow | `war thunder` |
| macOS | AppleScript System Events | `war thunder` / `aces` |

**使用方式：**

```java
// Controller.openpad() - 启用焦点监控
String autoHideStr = getconfig("autoHideOnFocusLoss");
if (autoHideStr != null && Boolean.parseBoolean(autoHideStr)) {
    S.getFocusMonitor().setEnabled(true);
}

// Controller.closepad() - 禁用焦点监控（会自动恢复被隐藏的overlay）
S.getFocusMonitor().setEnabled(false);
```

**设计要点：**
- **无新线程**：复用Service已有的~10Hz轮询循环
- **200ms节流**：FocusMonitor内部控制检测频率，避免进程检测开销
- **安全降级**：检测失败时返回`true`（假设有焦点），不会误隐藏overlay
- **最小侵入**：Service.java仅增加5行代码，Controller仅增加9行

### GPU Compatibility Mode

VoidMei can conflict with War Thunder's GPU on some systems due to Java2D hardware acceleration. The GPU compatibility mode disables hardware acceleration, forcing software rendering.

**Architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│  MANIFEST.MF → Main-Class: prog.Launcher                    │
└─────────────────────────────────────────────────────────────┘
          ↓
┌─────────────────────────────────────────────────────────────┐
│                    Launcher.java                             │
│  • NO java.awt.* imports (critical!)                        │
│  • Reads gpu_compat.properties                              │
│  • Sets sun.java2d.* system properties                      │
│  • Calls Application.main(args)                             │
└─────────────────────────────────────────────────────────────┘
          ↓
┌─────────────────────────────────────────────────────────────┐
│                   Application.java                           │
│  • AWT classes load with properties already set             │
│  • Java2D uses software rendering (if enabled)              │
└─────────────────────────────────────────────────────────────┘
```

**Why a separate Launcher class?**

JVM system properties like `sun.java2d.d3d` must be set **before** any AWT class is loaded. Once `java.awt.Toolkit` is instantiated, the Java2D rendering pipeline is locked. Since `Application.java` imports AWT at the top of the file, we need `Launcher.java` (with zero AWT imports) to set properties first.

**Key files:**

| File | Purpose |
|------|---------|
| `prog/Launcher.java` | Bootstrap class, reads `gpu_compat.properties`, sets JVM properties |
| `prog/util/GPUCompatibilityHelper.java` | Runtime helper: save/load settings, check active mode |
| `gpu_compat.properties` | External config file (created when user enables the feature) |

**JVM properties set (when enabled):**

| OS | Properties |
|----|------------|
| Windows | `sun.java2d.d3d=false`, `sun.java2d.noddraw=true` |
| Linux | `sun.java2d.opengl=false`, `sun.java2d.xrender=false` |
| macOS | `apple.awt.graphics.UseQuartz=false` |
| All | `sun.java2d.pmoffscreen=false` |

**Usage in code:**

```java
import prog.util.GPUCompatibilityHelper;

// Save setting (called when user toggles in UI)
GPUCompatibilityHelper.saveSettings(true);

// Read saved setting
boolean enabled = GPUCompatibilityHelper.isEnabled();

// Check if software rendering is currently active
boolean active = GPUCompatibilityHelper.isSoftwareRenderingActive();
```

**Special handling in SwitchRowRenderer:**

The `gpuCompatibilityMode` config key has special handling:
1. Initial value reads from `GPUCompatibilityHelper.isEnabled()` (not ui_layout.cfg)
2. On toggle, saves to `gpu_compat.properties` via `GPUCompatibilityHelper.saveSettings()`
3. Shows a dialog prompting user to restart VoidMei

### DPI Scaling (High-DPI Display Support)

VoidMei supports high-DPI displays (e.g., Windows 200% scaling) using `DPIHelper` to detect and apply proper scaling.

**Problem solved:** On high-DPI displays, `Toolkit.getScreenSize()` returns physical pixels, but Swing operates in logical pixels. Without DPI awareness:
- MainForm extends beyond the visible screen area
- Overlay fonts appear blurry (due to Windows bitmap scaling)
- UI elements are improperly sized

**Architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│                   Application Startup                        │
│  Application.getScreenSize()                                │
│      ↓                                                       │
│  DPIHelper.init()                                           │
│      • GraphicsConfiguration.getDefaultTransform()          │
│      • Calculate scaleX/scaleY (1.0=100%, 2.0=200%)        │
│      • Calculate logical screen dimensions                  │
│      ↓                                                       │
│  Application.dpiScale, logicalWidth, logicalHeight          │
└─────────────────────────────────────────────────────────────┘
          ↓
┌─────────────────────────────────────────────────────────────┐
│                    UI Components                              │
│  MainForm         → Uses logicalWidth/Height for positioning │
│  BaseOverlay      → scaleFactor includes dpiScale           │
│  MinimalHUDContext→ crossScale, fonts scaled by dpiScale    │
│  Other Overlays   → fontSize scaled by dpiScale             │
└─────────────────────────────────────────────────────────────┘
```

**Key files:**

| File | Purpose |
|------|---------|
| `prog/util/DPIHelper.java` | DPI detection and scaling utilities |
| `prog/Application.java` | Global `dpiScale`, `logicalWidth`, `logicalHeight` fields |
| `script/voidmeil4j.xml` | JVM flag `-Dsun.java2d.uiScale=1` |

**Usage in code:**

```java
import prog.util.DPIHelper;
import prog.Application;

// Access pre-computed values (recommended)
double scale = Application.dpiScale;           // 1.0 at 100%, 2.0 at 200%
int screenW = Application.logicalWidth;        // Logical screen width
int screenH = Application.logicalHeight;       // Logical screen height

// Direct API access
DPIHelper.init();                              // Called once in Application.getScreenSize()
int scaled = DPIHelper.scale(24);              // Scale a base value: 24→48 at 200%
boolean hiDpi = DPIHelper.isHighDPI();         // True if scale > 1.0
```

**Scaling pattern for overlays:**

```java
// In overlay reinitConfig() or font setup
double dpiScale = Application.dpiScale;
int fontSize = (int) Math.round((24 + fontadd) * dpiScale);
Font font = new Font(fontName, Font.BOLD, fontSize);
```

**JVM flag `-Dsun.java2d.uiScale=1`:**

This flag (set in `voidmeil4j.xml` for Windows EXE) disables Java's automatic UI scaling, giving our code full control over DPI handling. This ensures:
- Fonts render at native resolution (crisp, not blurry)
- Consistent behavior across different JVM versions
- Predictable scaling calculations

**Backward compatibility:**

At 100% scaling, `dpiScale = 1.0` and `logicalWidth = physicalWidth`, so all calculations produce identical results to the pre-DPI-aware code.

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

### Color Utilities

Use `prog.util.ColorHelper` for parsing and formatting colors in both hex and decimal formats:

```java
import static prog.util.ColorHelper.*;

// Parse any format (hex or decimal)
Color c1 = parseColor("#FF5500AA", Color.WHITE);      // Hex with alpha
Color c2 = parseColor("255, 85, 0, 170", Color.WHITE); // Decimal RGBA
Color c3 = parseColor("#FF5500", Color.WHITE);         // Hex without alpha (defaults to 255)

// Format for display (hex)
String hex = toHexString(c1, true);   // "#FF5500AA"
String hexNoAlpha = toHexString(c1, false);  // "#FF5500"

// Format for config storage (decimal, backward compatible)
String dec = toDecimalString(c1);     // "255, 85, 0, 170"

// Detect format
boolean isHex = isHexFormat("#FF5500");  // true
```

**Usage pattern in `ColorRowRenderer`:**
- Display: hex format in text field for user-friendly editing
- Storage: decimal format in config for backward compatibility with existing `ui_layout.cfg` values

### Overlay Style Helper

Use `ui.util.OverlayStyleHelper` for common overlay window styling operations:

```java
import static ui.util.OverlayStyleHelper.*;

// Apply transparent window style (game mode)
applyTransparentStyle(this);

// Apply preview mode styling (settings UI)
applyPreviewStyle(this);

// Load font configuration with defaults fallback
FontConfig fonts = loadFontConfig(overlaySettings);
Font labelFont = new Font(fonts.fontName, Font.BOLD, 12 + fonts.fontSizeAdd);
Font numFont = new Font(fonts.numFontName, Font.BOLD, 24 + fonts.fontSizeAdd);
```

This helper consolidates repeated styling patterns from 6+ overlay files, reducing ~400 lines of duplicate code.

### Slider Helper

Use `ui.util.SliderHelper` for configuring read-only display sliders:

```java
import ui.util.SliderHelper;

// Vertical progress bar (gear/flaps display)
SliderHelper.configureVerticalProgress(slider, 0, 100, topColor, bottomColor);

// Horizontal attitude slider (control surfaces)
SliderHelper.configureAttitudeSlider(slider, -100, 100, thumbColor);

// Just disable interaction on any slider
SliderHelper.removeAllListeners(slider);
```

This helper consolidates the ~25-line `initslider()` pattern in GearFlapsOverlay, AttitudeOverlay, and ControlSurfacesOverlay.

### Graphics Utilities

Use `ui.util.GraphicsUtil` for standard Graphics2D configuration:

```java
import ui.util.GraphicsUtil;

public void paintComponent(Graphics g) {
    Graphics2D g2d = (Graphics2D) g;
    // Apply standard anti-aliasing hints (replaces 4-line pattern)
    GraphicsUtil.configureOverlayRendering(g2d);

    // Use precise strokes (exact endpoints, no cap extension)
    g2d.setStroke(GraphicsUtil.createPreciseStroke(2.0f));

    // ... drawing code
}
```

This helper consolidates the 4-line rendering hint pattern repeated 30+ times across overlay files.

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

**重要**: Controller 不再实现 ConfigProvider 接口。需要访问配置的组件应该：

1. **通过 `Controller.getConfigProvider()`** 获取 `ConfigProvider` 接口
2. **通过 `Controller.getConfigService()`** 获取 `ConfigurationService` 实例

```java
// ✅ 正确：使用 ConfigProvider 接口
public void init(Controller c, Service s, OverlaySettings settings) {
    this.config = c.getConfigProvider();  // 配置访问
    this.controller = c;                   // FM 数据访问 (如 getBlkx())
}

// ❌ 错误：将 config 强转为 Controller
prog.Controller ctrl = (prog.Controller) config;  // ClassCastException!
```

**职责分离原则:**

| 字段 | 类型 | 用途 |
|------|------|------|
| `config` | `ConfigProvider` | 配置读写 (`getConfig`, `setConfig`) |
| `controller` | `Controller` | FM 数据访问 (`getBlkx`, `getCompressorStages`) |
| `overlaySettings` | `OverlaySettings` | 分组配置 (位置、字体等) |

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
```
