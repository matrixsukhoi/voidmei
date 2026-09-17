# *** 使用中文思考 ***
# 做完需求后和修复问题后, 要将对项目的理解和容易踩坑的地方更新到CLAUDE.md和相关文档里. 主要是记录high level的东西.
# *** fmdata在data下, 写需求时会依赖data下的各种飞机的数据; 对于做需求来说, 需求影响到的每一个飞机, 要列出一个列表确认. 每一个需求的测试也要遍历data的所有涉及的飞机 ***
# 如果子agent出现了没有读写权限的情况, 及时停止子agent
# 不要让测试假通过
# 代码里的注释要简洁精炼
# 写代码时, 关键的地方和问题修复一定要添加和补充中文注释
# 发布版本前要更新版本更新文档, 不要忘了加入fmdata的版本变化
# 历史上发现的容易踩坑的地方:
 - 直升机有的有可释放起落架, 有的是固定起落架

# CLAUDE.md

VoidMei 的开发指引。原则：只记录代码里看不出来的知识（命令、流程、契约、坑），不复述代码本身。

## 项目概览

Java 8 Swing 遥测悬浮窗（War Thunder HUD overlay）。轮询游戏本地 HTTP API（127.0.0.1:8111，~10Hz）解析实时飞行数据，结合离线拆包的 FM（.blkx）文件计算派生指标，以半透明 overlay 呈现。约 159 个 Java 文件。

**严格 Java 8**（1.8.x）：EXE 由 launch4j 强制 `maxVersion: 1.8.999`；JVM flags `-Dsun.java2d.uiScale=1 -Xms64m -Xmx320m`（见 voidmeil4j.xml）。

## 构建命令

统一入口 `python script/build.py`（Python 3.8+ 标准库，Windows/Linux/CI 行为一致）。版本号由 `VOIDMEI_VERSION` 环境变量注入（CI 从 git tag 提取，本地缺省 `dev`）。

```bash
python script/build.py compile   # 编译 src/ → bin/
python script/build.py run       # 本地运行 (bin/ 缺失自动编译)
python script/build.py test      # 全部单元测试; test <套件> 跑指定套件
                                #   (atmosphere/piston/visibility/voicepack/fmstore/fmpaths/fmhandle/e2e)
python script/build.py test spitfire  # 真机 FM 验证 (项目内 data/ 的 blkx, 无 data 自动跳过)
                                #   spitfire / tempest / fuzz-blkx (blkx 变异 fuzz)
python script/build.py jar       # 打 jar (MANIFEST 注入版本号)
python script/build.py exe       # 打 exe (launch4j)
python script/build.py dist      # 组装分发包 → dist/VoidMei_v*.zip (含裁剪版 data)
python script/build.py fmdata    # 游戏版本更新后: 解包并裁剪 FM 数据 (游戏目录自动探测, WT_GAME_DIR 可显式指定)
python script/build.py clean     # 清理构建产物

# mock server (模拟 8111 API, 场景 s1~s6 见 script/mock_scenarios/)
python3 script/mock_8111.py serve --port 8111 --scenario s5_missing_fm

# FM 端到端回归 (起 mock + 应用跑 N 秒 + 日志断言 A1~A6)
python script/build.py test e2e          # 三场景各 30 秒; 8111 被占自动跳过
bash script/e2e_fm.sh --scenario s5_missing_fm --duration 120   # 单场景长跑 (慢, 不进 test all)
```

## 目录职责（单一来源架构）

| 路径 | 角色 |
|------|------|
| 项目根 | 唯一源 + 本地运行工作区（git 跟踪源码/资源 + gitignore 的本地 data/、运行时生成物） |
| `dist/` | 构建产物（gitignore） |
| GitHub `v*` Release | 唯一分发渠道（CI 自动构建） |
| GitHub `data` prerelease | fmdata 云端存储层（CI 组包用，`--prerelease` 保证 `checkUpdate()` 看不到） |

**资源管理（按"丢了怎么恢复"分类）**：源码+自有资产进 git；`data/` 是派生数据不进 git（wt_ext_cli 从游戏客户端再生成）；运行时数据（records/ config/ ui_layout.user.cfg）gitignore。`fonts/DIN Pro 400.otf` 为商业字体，gitignore 排除、不分发。

## 发版流程

**版本号单一来源 = git tag**（规范 `1.590`，纯数字三段、**无 v 前缀**——沿用仓库历史惯例，也是 scoop autoupdate 模板 `download/$version/` 的依赖；fmdata 更新也占正常版本号如 `1.591`）。**不要用四段号** `1.590.1`——`checkUpdate()` 的正则会截断成 `1.590`，用户收不到更新提示。更新日志.txt 里的版本行**带 v**（`v1.590`，面向用户显示格式，`release_notes.py` 内部做转换）。

**更新记录唯一来源 = `更新日志.txt`**（人手写，git 跟踪；CI 对它只读不改）。CHANGELOG.md 已废除。

**日常发版（全自动）**：
1. `更新日志.txt` 顶部（TODO 注释块之后）插入新版本块：`____分隔线 / v1.590 / 一行一条改动`——只写用户可感知的改动
2. 代码改动**一律走 PR**（单人也不例外）：分支提交 → `gh pr create`（正文 `Fixes #N` 关联 issue）→ rebase-merge 进 master
3. 在 merge 后的 master commit 上 `git tag 1.590 && git push origin 1.590` ← 触发 CI。**tag 是发版运维动作不走 PR**（业界惯例：PR 管代码变更，tag 管发布；版本号由 tag 驱动，必须打在已 merge 的 commit 上直推）
4. CI（release.yml）：checkout tag 的 commit → 从 `data` prerelease 拉 FM 数据 → `build.py dist` → 从更新日志提取该版本条目作 Release body → 创建 **draft** Release
5. 测试同学验证 draft 附件 → 人工点 "Publish release" 转正

**游戏版本更新后（fmdata，纯运维）**：
```bash
python script/build.py fmdata
python script/build.py fmdata-upload
# fmdata-upload: 以 dist/data_manifest.json 为真相源选包上传到 data prerelease,
#                复查线上资产确认成功后自动删除旧版本 zip (防多版本并存时 CI 选错包)
# 然后更新 更新日志.txt (走 PR) → 在 master 上打新 tag (如 1.591), 由人拍板
```

**灰度测试**：push 正式 tag → CI 建 draft（公众不可见，`checkUpdate()` 不弹）→ 测试同学下载验证 → Publish 转正。不通过即删 draft + 删 tag 重来。测试与发布共用同一份产物。**版本号不用 `-rc`/`-test` 后缀**——发布状态由 draft/published 表达。

**纯构建核验**：Actions 页手动触发 `release` workflow 填 version + 勾 dry-run → 只产出 artifact，不创建 Release。

**原则**：发版永远是显式动作（人工 Publish；tag 触发的只是构建）；旧版本 Release 一经发布不再改动。

## 架构

### 包一览（src/）

- **`prog/`** 内核：`Launcher`（GPU 兼容 JVM 属性，须在 AWT 加载前设置）→ `Application` → `Controller`（生命周期/overlay 协调）→ `Service`（HTTP 轮询+计算后台线程，最大文件）；`OverlayManager`（overlay 可见性，方法须 synchronized）；`AlwaysOnTopCoordinator`（单例 z-order/对话框协调，WeakReference）；`FocusMonitor`（游戏失焦自动隐藏，复用 Service 轮询）；`fm/`（FM 单一真相源：`FMManager` 单例 identify/负缓存/FM_CHANGED 广播；`FMLoader` 项目内唯一 `new Blkx` 点，全程 catch(Throwable)→READY/MISSING/CORRUPT；`FMHandle` 不可变句柄；`FMDataPaths` 路径唯一来源）；`config/`、`audio/`、`util/`、`hotkey/`、`i18n/`、`model/`、`event/`（`UIStateBus`/`FlightDataBus`）
- **`parser/`** 数据摄取：`State`/`Indicators`（8111 JSON）、`Blkx`（FM .blk 解析）、`FlightAnalyzer`、`FlightLog` 等
- **`ui/`** 界面：`MainForm`（设置窗）、`overlay/`（各 HUD overlay + `logic/HUDCalculator` + `model/HUDData`）、`layout/`（ui_layout.cfg 动态生成设置面板 + 17 种 renderer）、`component/`（HUD 部件）、`base/`（`DraggableOverlay`/`FieldOverlay`）、`model/`（`TelemetrySource` 等）、`replica/`、`util/`、`window/comparison/`（飞机对比）

### 数据流

```
8111 HTTP API (~10Hz) → Service 线程: 解析 JSON → FMManager.identify → FMLoader (后台线程)
  → 预计算 HUDData (降低 EDT 延迟 ~40-60ms) → FlightDataBus 发布 FlightDataEvent
  → 各 overlay (FlightDataListener) → SwingUtilities.invokeLater → EDT 渲染
```

- **HUDData 预计算**：计算在 Service 线程完成，EDT 只消费 `event.getHudData()`；overlay 内 dirty check（值变才重绘）
- **FM 加载**：Service 每轮 `identify(sIndic.type)`（同目标零成本），计算取 `FMManager.current()` 快照；无 FM 时相关指标按 0/上次值/MAX_VALUE 降级，UI 配合 visible-when 隐藏。**EDT 上不允许 new Blkx / FMLoader.load**（R3 规则）
- **引擎类型过滤**：Service 按引擎类型将无关指标置 0（喷气：进气压/水温；活塞：推力），配合隐藏逻辑

### 线程与窗口规则

- **EDT 规则**：Swing 组件更新必须 `SwingUtilities.invokeLater`；事件可能在后台线程回调
- **僵尸窗口防护**：注册进 `AlwaysOnTopCoordinator` 的窗口必须在 `dispose()` 中 `unregisterOverlay(this)`，否则 `FocusMonitor.showAllOverlays()` 会复活已销毁窗口
- **焦点抢占防护**：`setFocusable(false)` 必须在 `registerOverlay()` 之前；对话框用 `dialogWillShow()`/`dialogDidDismiss()` 挂起/恢复 alwaysOnTop
- **托盘点击防重**：`Application` 用 `AtomicBoolean` CAS 防重复创建 Controller；`Controller.stop()` 清理顺序：关 overlay→退订事件→dispose MainForm→停 Service→存配置
- **预览过期回调**：`Controller` 用 `AtomicLong previewGeneration` 防切换游戏模式后旧 EDT 回调复活预览 overlay
- **预览 FM 回退**：预览模式走 `Controller.detectAndIdentify()`——先探测 8111 live 机型，拿不到回退配置的默认飞机，统一交 `FMManager.identify()`；仅 PREVIEW 状态触发

### GPU 兼容 / DPI / 失焦检测

| 机制 | 要点 |
|------|------|
| GPU 兼容 | `Launcher`（无 AWT import）在 AWT 加载**前**设 `sun.java2d.*` 属性关硬件加速；状态存 `gpu_compat.properties`（必须早于 ui_layout.cfg 可读，故独立文件）；`gpuCompatibilityMode` 配置键在 SwitchRowRenderer 有特殊处理 |
| DPI 缩放 | `DPIHelper` 经 `GraphicsConfiguration.getDefaultTransform()` 探测，暴露 `Application.dpiScale`（2.0=200%）与 `logicalWidth/Height`；字体/尺寸计算一律乘 dpiScale，屏幕定位用 logical 尺寸不用 `Toolkit.getScreenSize()`（那是物理像素） |
| 失焦自动隐藏 | `FocusMonitor`(200ms 节流)+`FocusDetector`（Windows: PowerShell 查 `aces` 进程；Linux: xdotool；macOS: AppleScript）；无新线程 |

### 工具类去向（禁止自写）

| 需求 | 用 |
|------|-----|
| 物理常数 | `PhysicsConstants`（`g`/`G` 等）——**永不硬编码** `9.78f`/`9.80` |
| 插值 | `Interpolation`（`lerp`/`interp1d`/`interp2d`/`interpSweepLevel`） |
| 大气 | `AtmosphereModel`（`pressure`/`density`/`iasToTas`/`tasToIas`/`ramEffectAltitude`） |
| 活塞功率 | `PistonPowerModel`（详见 `src/prog/util/CLAUDE.md`） |
| 颜色 | `ColorHelper`（`parseColor` 收 hex `#RRGGBBAA` 或十进制 `R,G,B,A`；存储用十进制保兼容） |
| 异常 | `ExceptionHelper`（`sleepQuietly`/`logAndContinue`/`closeQuietly`）——禁止空 catch 块 |
| 日志 | `Logger`（TRACE/DEBUG/INFO/WARN/ERROR，默认 INFO） |
| UI 样式/滑条/绘图/常量 | `OverlayStyleHelper`/`SliderHelper`/`GraphicsUtil`/`UIConstants`（详见 `src/ui/util/CLAUDE.md`） |

### ConfigProvider 解耦

**Controller 不实现 ConfigProvider**。overlay 组件职责分离：

| 字段 | 用途 |
|------|------|
| `config`（`ConfigProvider`） | 配置读写，`init()` 时 `c.getConfigProvider()` 获取 |
| `controller` | 生命周期/刷新协作（FM 数据统一走 `FMManager.getInstance().current()`） |
| `overlaySettings` / `hudSettings` | 通过 `init()` 参数传入；位置保存用 `OverlaySettings.saveWindowPosition()` |

禁止运行时从 Controller 取 configService、禁止把 config 强转 Controller（ClassCastException）。

### 常见改动路径

**新增配置开关**：① `ui_layout.cfg` 加 `(item ... :type switch :target "myKey")` → ② `HUDSettings`/`OverlaySettings` 加接口方法 → ③ `ConfigurationService` 实现（`getBool("myKey", def)`）→ ④ 目标 overlay 用配置控制行为 → ⑤ `Controller` 把键加进 `.withInterest()`（WYSIWYG 实时预览）。注意 RowConfig 新字段有强制检查点，见 `src/prog/config/CLAUDE.md`。

**新增 overlay**：继承 `BaseOverlay`（列表型）或 `DraggableOverlay`（自绘），实现 `init()`/`initPreview()`/`reinitConfig()`/`dispose()`（**必须**注销 FlightDataBus 与 AlwaysOnTopCoordinator），在 `Controller.registerGameModeOverlays()` 用 `registerWithPreview(...).withInterest(...)` 注册 + ui_layout.cfg 加开关。

**新增 HUD 组件**：实现 `HUDComponent`，尺寸一律基于 `ctx.hudFontSize`（禁硬编码像素），`draw()` 零分配，在 `MiniHUDOverlay` 实例化/布局/接线。详见 `src/ui/component/CLAUDE.md`。

### 子模块文档

| 模块 | 文档 |
|------|------|
| Parser（FM/遥测） | [`src/parser/CLAUDE.md`](src/parser/CLAUDE.md) |
| 配置系统 | [`src/prog/config/CLAUDE.md`](src/prog/config/CLAUDE.md) |
| 工具类 | [`src/prog/util/CLAUDE.md`](src/prog/util/CLAUDE.md) |
| UI 工具 | [`src/ui/util/CLAUDE.md`](src/ui/util/CLAUDE.md) |
| Overlay 开发 | [`src/ui/overlay/CLAUDE.md`](src/ui/overlay/CLAUDE.md) |
| UI 模型 / TelemetrySource | [`src/ui/model/CLAUDE.md`](src/ui/model/CLAUDE.md) |
| HUD 组件 | [`src/ui/component/CLAUDE.md`](src/ui/component/CLAUDE.md) |
| FM 对比规则 | [`src/ui/window/comparison/CLAUDE.md`](src/ui/window/comparison/CLAUDE.md) |
