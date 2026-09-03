# *** 使用中文思考 ***
# 如果子agent出现了没有读写权限的情况, 及时停止子agent
# 代码里的注释要简洁精炼
# java->rust已经迁移完了, 不需要再和java对齐了
# 不要跑e2e测试, 不要跑冒烟测试
# 不要补充或新增更多测试了
# rust版本还没发布, 正在重构, 不用担心兼容性问题, 可以随便改架构. 
# java侧的代码已经迁移至rust了, PORT 宪法里要求的1:1已经不用再遵循了. 现在要考虑的是如何改进架构, 抛弃来自java的代码坏味道, 进行现代化改造


# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VoidMei is a Java Swing telemetry overlay for War Thunder. It reads real-time flight data from the game's local HTTP API (port 8111) and displays HUD overlays with flight metrics, warnings, and aircraft performance data.

**Project Statistics:** ~159 Java files, ~33,500 lines of code

## 写代码时, 关键的地方和问题修复一定要添加和补充中文注释

## Build Commands

**统一构建入口**: `python script/build.py`（Python 3.8+ 标准库实现，Windows cmd/PowerShell 直接执行，Linux/CI 行为一致；CI 复用同一脚本）。**版本号由 `VOIDMEI_VERSION` 环境变量注入**（CI 从 git tag 提取，本地缺省 `dev`），发版无需改代码。

**Java 8 Required:** VoidMei strictly requires Java 8 (1.8.x). The Windows EXE enforces `maxVersion: 1.8.999` to prevent running on Java 9+, which has incompatible module changes.

```bash
# 编译 src/ → bin/
python script/build.py compile

# 本地运行 (bin/ 缺失时自动编译; classpath 直跑免打 jar, 版本号 dev)
python script/build.py run

# 运行单元测试 (全部或指定套件)
python script/build.py test              # all / atmosphere / piston / visibility / voicepack /
                                         # fmstore / fmpaths / fmhandle
python script/build.py test spitfire     # 真机 FM 验证 (项目内 data/ 的 blkx, 无 data 自动跳过):
                                         # spitfire / tempest / fuzz-blkx (blkx 变异 fuzz)

# 打 jar (MANIFEST 注入版本号) / 打 exe (launch4j, 版本资源注入)
python script/build.py jar
python script/build.py exe

# 组装完整分发包 → dist/VoidMei_v*.zip (含裁剪版 data, 剔除用户数据)
python script/build.py dist

# 组装 Rust 版分发包 → dist/VoidMei_Rust_*.zip (同形态, 解压即用免 JRE; 版本号注入同 VOIDMEI_VERSION)
python script/build.py rustdist

# 游戏版本更新后: 解包并裁剪 FM 数据 (更新项目内 ./data, 产出 data zip + manifest)
# 游戏目录自动探测 (注册表 > Steam 库 > 常见路径, 缓存 .wt_game_dir), 也可 WT_GAME_DIR 显式指定
python script/build.py fmdata

# JSON 版 FM 数据 (Rust 端数据源, 与 blkx 同名并存 data/, 产出 VoidMei_RustData zip)
python script/build.py fmdatajson

# 清理构建产物
python script/build.py clean

# Mock server for testing (simulates War Thunder API)
#   serve --scenario <name>  按 script/mock_scenarios/ 场景供数 (s1~s6);
#   控制通道 /_mock/state (请求计数, 验证应用未假死) /_mock/scenario/<name> /_mock/shutdown
python3 script/mock_8111.py serve --port 8111 --scenario s5_missing_fm

# FM 端到端回归 (起 mock + 应用跑 N 秒 + 日志断言 A1~A6, 见 script/e2e_fm.sh)
python script/build.py test e2e    # 套件化: s2 正常/s5 缺失/s6 畸形 三场景各 30 秒; 8111 被占自动跳过
bash script/e2e_fm.sh --scenario s5_missing_fm --duration 120   # 单场景长跑 (不进 test all: 慢/动工作区)

# 本地运行 (repo 即工作区, data/fonts/voice 都在项目根)
java -jar VoidMei.jar
```

**Unit tests** available for utility classes in `test/`. Integration testing is manual via the running application or mock server.

**Rust 全量迁移 (已完成 + 六波架构重构 + 波7~11 组织结构 + 波12~19 坏味道清扫 + 波20 8111 链现代化)**: `rust/` 是
Java 版的全量迁移产物 (cargo workspace 六 crate)。2026-09 架构:
vm-core 11 域分组 (base/config/game_api/fm/formula/derived/audio/ui_support/
platform/lang/activation, 根 shim 已退役 — 全库唯一路径 `vm_core::<域>::<模块>`；game_api 域
= 8111 客户端 ureq + serde 解析, 波20 由 telemetry 更名并退役手写 HTTP/子串扫描);
vm-overlay 五域 (platform/render/overlays/layout/ui_model, 根 re-export 壳亦退役);
vm-app lib 标准入口 (win32.rs 已更名 render_thread.rs);
跨线程数据读面 = vm-data `FrameStore` 不可变帧快照 (零锁); UIStateBus 统一路由总线
(嵌套 publish 安全); HTTP 单线程阻塞客户端 (ureq)。
坏味道清扫登记: `doc/rust坏味道登记与重构方案.md`。
构建/运行/对拍/e2e 见 `rust/README.md`;
迁移设计档案: `build/migration/` (PORTING 宪法/CLASSIFY/LIFETIMES/DECISIONS/PROGRESS),
迁移文档: `doc/overlay_java_to_rust_migration.md` (§11 执行记录+人工验收清单)。
Java 端离屏导出器: `java -classpath "bin;dep/*" ui.debug.OverlayPngExport`。
Rust e2e: `bash script/rust_e2e.sh`(Java 对拍链已随 Java 版退役删除)。

### Windows Launch Scripts

VoidMei provides multiple ways to launch on Windows:

| File | Purpose |
|------|---------|
| `VoidMei.exe` | Launch4j-wrapped executable with Java 8 enforcement |
| `VoidMei.bat` | Intelligent batch script that finds Java 8 from registry |

**VoidMei.bat** searches Windows Registry for Java 8 (Oracle, Temurin, Zulu, Corretto, Microsoft), then falls back to `%JAVA_HOME%` or `PATH`.

**voidmeil4j.xml** (Launch4j 配置模板，版本号为 `@VERSION@`/`@VERSION4@` 占位符，由 `build.py exe` 注入):
- `minVersion: 1.8.0`, `maxVersion: 1.8.999` - Strictly enforces Java 8
- `jreVersionErr` message guides users to download Eclipse Temurin 8
- JVM flags: `-Dsun.java2d.uiScale=1 -Xms64m -Xmx320m`

### 目录职责（单一来源架构）

| 路径 | 角色 |
|------|------|
| 项目根 `~/projects/voidmei/` | **唯一源 + 本地运行工作区**（git 跟踪源码/资源 + gitignore 的本地 `data/` `fonts/` 中未入库字体 + 运行时生成物） |
| `dist/` | 构建产物（zip/jar/exe，gitignore） |
| GitHub `v*` Release | 唯一分发渠道（CI 自动构建） |
| GitHub `data` prerelease | fmdata 云端存储层（CI 组包用，`--prerelease` 保证不进 `/releases/latest`，`checkUpdate()` 永远看不到） |

**资源管理（按"丢了怎么恢复"分类）**: 源码+自有资产(image/lang/dep/fonts/voice 中已入库部分) 进 git；`data/` 是派生数据不进 git（wt_ext_cli 从游戏客户端再生成）；运行时数据(records/ config/ ui_layout.user.cfg) gitignore。`fonts/DIN Pro 400.otf` 为商业字体，gitignore 排除、不分发。

### Release（发版流程）

**版本号单一来源 = git tag**（规范 `1.590`，纯数字三段、**无 v 前缀**——沿用仓库历史惯例（历史 tag 全为 `1.583` 格式），也是 Lutra-Fs/scoop-bucket autoupdate 模板 `download/$version/` 的依赖；fmdata 更新版也占正常版本号，如 `1.591`，更新日志注明 WT 数据版本。**不要用四段号** `1.590.1`——`checkUpdate()` 的正则会截断成 `1.590`，用户收不到更新提示。更新日志.txt 里的版本行**带 v**（`v1.590`，面向用户的显示格式），与 tag 无 v 是两回事，`release_notes.py` 内部做转换）。

**更新记录唯一来源 = `更新日志.txt`**（人手写，git 跟踪；CHANGELOG.md 已废除——无外部开发者，Keep a Changelog 工具链用不上）。CI 对它**只读不改**：tag 的 commit 里就带着最新日志，zip 内外天然一致，不存在任何回写/同步环节。

**日常发版（全自动，代码内容由 tag 锁定）:**
1. 发版前直接在 `更新日志.txt` 顶部（TODO 注释块之后）插入新版本块：`____分隔线 / v1.590 / 一行一条改动`——**只写用户可感知的改动，不写工程实现细节**
2. `git commit`，然后 `git tag 1.590 && git push origin master 1.590` ← 触发 CI 构建 draft Release
3. CI（release.yml）: checkout tag 的 commit（不是 master HEAD）→ 从 `data` prerelease 拉取最新 FM 数据 → `build.py dist`（更新日志.txt 原样进 zip）→ 从 `更新日志.txt` 提取该版本条目块作 Release body（`release_notes.py extract`）→ 创建 **draft** Release
4. 测试同学验证 draft 附件 → 人工点 "Publish release" 转正（见下）

**游戏版本更新后（fmdata 更新，纯运维不触发发版）:**
```bash
python script/build.py fmdata   # 游戏目录自动探测 (或 WT_GAME_DIR=... 显式指定)
gh release upload data dist/VoidMei_data_*.zip dist/data_manifest.json --clobber
# 然后在已测试的 commit 上更新 更新日志.txt (如 "FM文件更新到2.57.2.x") 并打新 tag (如 1.591), 由人拍板
```

**灰度测试（不影响用户）**: push 正式 tag（如 `1.590`）→ CI 以 **draft** 创建 Release（公众不可见，`/releases/latest` 永不返回 draft，`checkUpdate()` 不弹）→ 测试同学（需仓库协作者权限）下载 draft 附件验证 → 通过后在 GitHub 页面点 "Publish release"（或 `gh release edit 1.590 --draft=false`）转正进 latest。测试与发布共用**同一份产物**，无需重新构建；不通过即删 draft + 删 tag 重来。将来若需公开灰度（外部无权限人员），Publish 时可勾选 pre-release。**版本号不使用 `-rc`/`-test` 后缀**——发布状态由 Release 的 draft/published 状态表达，不进版本号。

**纯构建核验**: Actions 页手动触发 `release` workflow 并填写 version + 勾选 dry-run → 只构建产出 artifact（不创建 Release、不改任何远端状态）。

**原则**: 发版永远是显式动作（人工 Publish draft；tag 触发的只是构建）；data 上传不触发任何 workflow；旧版本 Release 一经发布不再改动。

## Architecture

### Core Packages (`src/`)

- **`prog/`** - Application kernel (14 files + 8 subpackages)
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
  - `fm/` - FM（飞行数据包）单一真相源（issue #55 死循环重构）: `FMManager`（单例，identify/负缓存/FM_CHANGED 广播的唯一入口；非飞机载具如坦克 type 含 `/` 前缀直接短路 NOT_AIRCRAFT，不加载不弹 toast）、`FMLoader`（项目内唯一 `new Blkx` 点，全程 catch(Throwable)→READY/MISSING/CORRUPT 句柄）、`FMHandle`（不可变加载结果句柄）、`FMStatus`（六态状态机）、`FMDataPaths`（FM 数据路径唯一来源 + 测试 setDataRoot 注入）
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
    ↓ FMManager.identify(type) → FM-Loader 线程 → FMLoader.load → FMHandle
    │   (READY/MISSING/CORRUPT; 缺失进负缓存不再重试 → UIStateBus FM_CHANGED)
    ↓ Pre-compute HUDData (reduces EDT latency; FM 派生量经 hasFM() 守卫降级)
FlightDataBus (event publisher)
    ↓ FlightDataEvent (carries pre-computed HUDData)
Overlay components (FlightDataListener subscribers)
    ↓ SwingUtilities.invokeLater()
Swing/WebLaF UI (EDT thread)
```

**Performance Note:** HUDData is pre-computed on the Service thread before publishing, reducing EDT latency by ~40-60ms. Overlays access this via `event.getHudData()` instead of computing on the EDT.

**FM Loading Note:** Service 每轮 `FMManager.getInstance().identify(sIndic.type)`（同目标零成本），
计算取 `FMManager.current()` 快照；无 FM（UNRESOLVED/LOADING/MISSING/CORRUPT）时相关
指标按 0/上次值/MAX_VALUE 降级，UI 端配合 hide-when-zero 隐藏。**EDT 上不允许 new Blkx / FMLoader.load**（R3 规则，详见 `doc/voidmei贡献者开发手册.md` 第 11 章）。

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

预览模式下 FM 识别走 `Controller.detectAndIdentify()`：优先探测 8111 的 live 机型
（`HttpHelper.getLiveAircraftType()`），拿不到再回退 `selectedFM0` 配置的默认飞机，
最终统一交给 `FMManager.identify()`（旧 `Controller.getBlkx()` 桥接已删，P5）。
仅在 `PREVIEW` 状态触发，避免影响游戏模式。

### Overlay Z-Order (AlwaysOnTopCoordinator)

Use `AlwaysOnTopCoordinator` singleton to manage overlay z-order and dialog coordination:

```java
// Overlay init: AlwaysOnTopCoordinator.getInstance().registerOverlay(this);
// Overlay dispose: AlwaysOnTopCoordinator.getInstance().unregisterOverlay(this);
// Before dialog: dialogWillShow(); try { dialog.setVisible(true); } finally { dialogDidDismiss(); }
```

**Key behaviors:** `registerOverlay()` tracks windows with `WeakReference`; `dialogWillShow()`/`dialogDidDismiss()` suspend/restore `alwaysOnTop` for dialogs. Thread-safe via `AtomicInteger` + `CopyOnWriteArrayList`.

**焦点抢占防护：** 1) `setFocusable(false)` 必须在 `registerOverlay()` 之前; 2) 循环中检查 `!isVisible()` 后才调用 `setVisible(true)`。

**僵尸窗口防护：** 所有注册的窗口必须在 `dispose()` 中调用 `unregisterOverlay(this)`，否则 `FocusMonitor` 的 `showAllOverlays()` 会复活已销毁的窗口。`AlwaysOnTopCoordinator` 内部使用 `isDisplayable()` 检查作为全局防护。

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
    │       ├→ FMManager (identify → FM-Loader 线程 → FMLoader → Blkx；负缓存 + FM_CHANGED)
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
    this.controller = c;                   // 生命周期/刷新协作引用 (FM 数据请走 FMManager)
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
| `controller` | `Controller` | 生命周期/刷新协作 (FM 数据统一走 `FMManager.getInstance().current()`) |
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
;; TelemetrySource methods: (isJetEngine), (isPropEngine), (isPistonEngine),
;; (isTurbopropEngine), (hasWep), (isEngineCheckDone)
```
