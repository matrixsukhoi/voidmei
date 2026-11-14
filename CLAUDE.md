# *** 使用中文思考 ***
# 做完需求后和修复问题后, 要将对项目的理解和容易踩坑的地方更新到CLAUDE.md和相关文档里. 主要是记录high level的东西.
# *** fmdata在data下, 写需求时会依赖data下的各种飞机的数据; 对于做需求来说, 需求影响到的每一个飞机, 要列出一个列表确认. 每一个需求的测试也要遍历data的所有涉及的飞机 ***
# 如果子agent出现了没有读写权限的情况, 及时停止子agent
# 不要让测试假通过
# 代码里的注释要简洁精炼
# 写代码时, 关键的地方和问题修复一定要添加和补充中文注释
# 发布版本和发布tag时要更新版本更新文档, 不要忘了加入"FM文件更新到"
# 测试手册.md 由 agent 维护, 审美与规则见正文"测试手册"节; 每次做完特性或修完 bug 必须补充/更新场景并做纯审美审查, 真机验证后在测试版本列登记版本号
# commit 和 push 都需要用户的明确指令, 不能自行发起或扩大动作 (例如让你 commit 时不要自行切分支)
# 所有改动直推 master, 不走 PR、不切分支. commit要保持简洁. push前要尽量按照功能合并, 不要零零散散
# 你不用考虑什么时候打tag发版. 听用户的明确指令就行.
# 打tag发版时, 先去github的release里看上一个版本的版本号是什么. 版本号一定要严格根据github的release递增, 递增基准是已发布的 Latest,不是 tag/draft. 打tag和更新更新日志时不要立刻commit&push, 一步一步等用户指令
# 本地验证时可以写临时测试用例/脚本测试, 但是是否要保留并git提交最终一定要问用户! 很多测试用例是没有用的, 听用户的判断
# **测试用例不擅自新增**: agent 缺少用户手里的场景上下文, 场景理解不透时写出的测试牛头不对马嘴、分不清等价类 (2026-09-25 哨兵值场景测试返工: 同一数据形态在不同局被标成两个矛盾的场景, 五个资产只留空壳帧一个)。要加测试时等用户明确指示怎么加
# 历史上发现的容易踩坑的地方:
 - 直升机有的有可释放起落架, 有的是固定起落架
 - 战雷 8111 状态转换窗口的退化帧形态 (2026-09-25 真机抓包, f4u-4b): 选机/载入/RTB 期 state 发单键 `{"valid": false}` 空壳帧; 载入后有一段幽灵帧 (state 是上一局冻结的全量真数据, indicators 只剩 3 键 type=dummy_plane, 靠 DUMMY_PLANE 检查挡住); 空中出生从按出击到出生 ~17 秒是 27 键占位帧 (缺 AoA/AoS/Ny/水温, IAS=0/RPM=600, indicators 温度=-273.15 绝对零度); RTB 后 Ny/AoA/AoS 可单独缺失十几秒; J3 后 ~11 秒仍是真实值 (死亡回放, 油门残留/RPM 衰减), 哨兵守卫拦不住, 由死亡检测收口。防线: State/Indicators isSingleKeyFrame 丢弃空壳帧、playerLive 类判定用 >0、VoiceWarning 各告警消费原始字段前加哨兵守卫、formatDataAsStrings 哨兵显示 "-"。**已拍板不做**: 出生等待占位帧的整帧丢弃——"哪个机型在什么时机缺哪些键"是战雷侧实现行为, 无契约可依 (机型间也无等价类可言, 先例: P-63 无 RPM throttle 键), 抽样归纳不出可靠判据, 收益小不投入。新增消费 8111 原始字段的判断必须防哨兵值

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
                                #   (atmosphere/piston/visibility/voicepack/fmstore/fmpaths/fmhandle/real8111/e2e)
python script/build.py test spitfire  # 真机 FM 验证 (项目内 data/ 的 blkx, 无 data 自动跳过)
                                #   spitfire / tempest / fuzz-blkx (blkx 变异 fuzz)
python script/build.py jar       # 打 jar (MANIFEST 注入版本号)
python script/build.py exe       # 打 exe (launch4j)
python script/build.py dist      # 组装分发包 → dist/VoidMei_v*.zip (含裁剪版 data)
python script/build.py fmdata    # 游戏版本更新后: 解包并裁剪 FM 数据 (游戏目录自动探测, WT_GAME_DIR 可显式指定)
python script/build.py clean     # 清理构建产物

# mock server (模拟 8111 API, 场景 s1~s5 见 script/mock_scenarios/; snapshots/real_empty_shell
#   为真机抓包的空壳单键帧快照, 端点体按战雷单行原文保真存储, real8111 套件用)
python3 script/mock_8111.py serve --port 8111 --scenario s5_missing_fm

# FM 端到端回归 (起 mock + 应用跑 N 秒 + 日志断言 A1~A6)
python script/build.py test e2e          # 两场景 (s2/s5) 各 20 秒; 8111 被占自动跳过
bash script/e2e_fm.sh --scenario s5_missing_fm --duration 120   # 单场景长跑 (慢, 不进 test all)
```

## 目录职责（单一来源架构）

| 路径 | 角色 |
|------|------|
| 项目根 | 唯一源 + 本地运行工作区（git 跟踪源码/资源 + gitignore 的本地 data/、运行时生成物） |
| `dist/` | 构建产物（gitignore） |
| GitHub `v*` Release | 唯一分发渠道（CI 自动构建） |
| GitHub `data` prerelease | fmdata 云端存储层（CI 组包 + **客户端自动更新源**：`FMDataUpdater` 启动静默检查/下载/替换/即时生效；`--prerelease` 保证 `checkUpdate()` 看不到，release 直链匿名可下） |

**资源管理（按"丢了怎么恢复"分类）**：源码+自有资产进 git；`data/` 是派生数据不进 git（wt_ext_cli 从游戏客户端再生成）；运行时数据（records/ config/ ui_layout.user.cfg）gitignore。`fonts/DIN Pro 400.otf` 为商业字体，gitignore 排除、不分发。

## 发版流程

**版本号单一来源 = git tag**（规范 `1.590`，纯数字三段、**无 v 前缀**——沿用仓库历史惯例，也是 scoop autoupdate 模板 `download/$version/` 的依赖；fmdata 更新也占正常版本号如 `1.591`）。**不要用四段号** `1.590.1`——`checkUpdate()` 的正则会截断成 `1.590`，用户收不到更新提示。更新日志.txt 里的版本行**带 v**（`v1.590`，面向用户显示格式，`release_notes.py` 内部做转换）。

**更新记录唯一来源 = `更新日志.txt`**（人手写，git 跟踪；CI 对它只读不改）。CHANGELOG.md 已废除。

**日常发版（全自动）**：
1. `更新日志.txt` 顶部（TODO 注释块之后）插入新版本块：`____分隔线 / v1.590 / 一行一条改动`——只写用户可感知的改动
2. 代码改动直接 commit 到 master 后 push（不走 PR，不切分支）；**攒若干 commit 后一起 push**，按功能合并，不要零零散散
3. 更新日志同样直推 master
4. 在 master 最新 commit 上 `git tag 1.590 && git push origin 1.590` ← 触发 CI。**tag 是发版运维动作直推**（版本号由 tag 驱动，必须打在已 push 的 commit 上）
5. CI（release.yml）：checkout tag 的 commit → 从 `data` prerelease 拉 FM 数据 → `build.py dist` → 从更新日志提取该版本条目作 Release body → 创建 **draft** Release
6. 测试同学验证 draft 附件 → 人工点 "Publish release" 转正

**游戏版本更新后（fmdata，纯运维）**：
```bash
python script/build.py fmdata
python script/build.py fmdata-upload
# fmdata-upload: 以 dist/data_manifest.json 为真相源选包上传到 data prerelease,
#                复查线上资产确认成功后自动删除旧版本 zip (防多版本并存时 CI 选错包)
# 然后更新 更新日志.txt (直推 master) → 在 master 上打新 tag (如 1.591), 由人拍板
# 注意: 客户端 FMDataUpdater 会在用户下次启动时自动拉取新 data (无需发版即生效,
#       但要不要发版/打 tag 仍由人拍板, 流程不变)
# data/aces/version(WT 版本号) + data/aces/date(打包日期) 两个标记文件:
#       build.py fmdata 写入, FMDataUpdater 更新后对 date 兜底补写, 设置面板版本信息行显示
```

**灰度测试**：push 正式 tag → CI 建 draft（公众不可见，`checkUpdate()` 不弹）→ 测试同学下载验证 → Publish 转正。不通过即删 draft + 删 tag 重来。测试与发布共用同一份产物。**版本号不用 `-rc`/`-test` 后缀**——发布状态由 draft/published 表达。

**纯构建核验**：Actions 页手动触发 `release` workflow 填 version + 勾 dry-run → 只产出 artifact，不创建 Release。

**原则**：发版永远是显式动作（人工 Publish；tag 触发的只是构建）；旧版本 Release 一经发布不再改动。

## 测试手册

`测试手册.md` 是全场景手动测试的唯一真相源, 由 agent 维护。审美一句话：**把关键点和操作放到一起, 没用的不要有**。

- **结构**：顶层"预览模式 (主窗与所见即所得)"两列表（操作|测试版本）+ "游戏模式 (真机飞行)"三列表（机型|操作|测试版本），两种模式都按面板名分小节，同名小节两两对应
- **操作列文体**："动作, 看什么; 动作, 听什么"的流水句，真实最短路径。标杆：`试飞并摔死重生, 没问题; 然后重新试飞, 没问题`（一句覆盖冷启动/连接/异常退出/幂等）。"没问题"就够了；只在**不直观的回归点**写具体预期（如"直升机不出现失速红区"）
- **机型列**：具体机型（一般检查用 f4u-4b），前提括号跟在机型后（`mi-24a (带起落架)`、`he51a1 (无罗盘仪表)`），不同前提拆行；禁止"任意机型/—/临时选"占位
- **等价类收敛**：普适特性（所见即所得/拖拽/持久化）和设置生效类（字体/边框/开关）拿飞行信息测一个例子；出过问题的项（字体大小, v1.585）补第二个面板做双份（现 MiniHUD）；后续某面板出新问题再照此补双份，不逐面板列等价类
- **收录门槛**：只收**用户可感知**且有出错空间的场景（复杂算法/长传递链/机型前提分支），纯透传显示不收，纯系统层指标（如 TCP 连接数/TIME_WAIT——用户看不到，回归靠 mock/e2e 白盒覆盖）不收；**测试同学必须能自行触发**（依赖"云端发新数据"之类前置不受控的场景不收——agent 用 mock+改本地数据验证一次即可，不进手册）；预期必须可判定，"合理/正常/随x变化"这类看不出错的措辞不要
- **禁止进手册**：场景编号、数据依赖字段名、实现细节（类名/配置键）、操作列里的版本备注、规则解释与元信息（"双份"）——这些只写在 CLAUDE.md。版本号只出现在测试版本列，且只填真机验证过的
- **写完必须做纯审美审查**（重点，单独一步）：只对照上面的文体标准逐行过——"正常/合理"改"没问题"或删、空话（"试XX功能"）、冗余副词、元信息、占位机型、表格断裂。**这一步不做事实正确性核查**（机型文件是否存在/代码行为/配置项），内容对错是另一回事
- **改完代码更新手册时必须做影响面审视**：除新增/修改的场景外，还要列出本次改动波及的**既有场景**并明确告知用户哪些需要重测——状态流/生命周期类改动（数据无效跳过计算、跳过发事件）要追所有仍在运行的消费者（告警独立循环/事件订阅者/记录器），"生产者停摆"必然带来"消费者状态冻结"风险，必须预判并提示验证（2026-09-25 教训: else 分支停算后增压器告警冻结循环响，手册没提示重测，用户靠耳朵发现回归）
- **待测试标记**：新版本需要测试的场景（新增 + 被改动波及的既有），在测试版本列标 **`待测试`**；用户真机验证后把标记替换为版本号。版本列空 = 未波及或未登记的历史场景，`待测试` = 本次改动后需要过一遍

## 架构

### 包一览（src/）

- **`prog/`** 内核：`Launcher`（GPU 兼容 JVM 属性，须在 AWT 加载前设置）→ `Application` → `Controller`（生命周期/overlay 协调）→ `Service`（HTTP 轮询+计算后台线程，最大文件）；`OverlayManager`（overlay 可见性，方法须 synchronized）；`AlwaysOnTopCoordinator`（单例 z-order/对话框协调，WeakReference）；`FocusMonitor`（游戏失焦自动隐藏，复用 Service 轮询）；`fm/`（FM 单一真相源：`FMManager` 单例 identify/负缓存/FM_CHANGED 广播，`dataUpdated()` 供热替换后清负缓存强制重载；`FMLoader` 项目内唯一 `new Blkx` 点，全程 catch(Throwable)→READY/MISSING/CORRUPT；`FMDataUpdater` FM 数据在线自动更新一条链（检查→下载→sha256→解压→rename 替换→`dataUpdated()` 生效，`-Dvoidmei.fmdata.update=force/off` 开发与 e2e 用）；`FMHandle` 不可变句柄；`FMDataPaths` 路径唯一来源）；`config/`、`audio/`、`util/`、`hotkey/`、`i18n/`、`model/`、`event/`（`UIStateBus`/`FlightDataBus`）
- **`parser/`** 数据摄取：`State`/`Indicators`（8111 JSON）、`Blkx`（FM .blk 解析）、`FlightAnalyzer`、`FlightLog` 等
- **`ui/`** 界面：`MainForm`（设置窗）、`overlay/`（各 HUD overlay：MiniHUD 与准星已拆为独立窗 + `logic/HUDCalculator` + `model/HUDData`）、`minihud/`（MiniHUD 自由画布布局与编辑器）、`layout/`（ui_layout.cfg 动态生成设置面板 + 17 种 renderer）、`component/`（HUD 原子部件）、`base/`（`DraggableOverlay`/`FieldOverlay`）、`model/`（`TelemetrySource` 等）、`replica/`、`util/`、`window/comparison/`（飞机对比）

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
- **阵亡语音告警抑制**：死亡回放期 `playerLive` 可能仍为 true，不能只用它判断；`Service.playerDeadOnMap` 由 map_obj 的 Player 图标判定（无图标/坐标越界），VoiceWarning 每轮检查该标志，阵亡后不跑告警检测、清 `fatalWarn`、并 `stopPlayback()` 停止仍在播放的 clip；重生找到 Player 图标后自动恢复
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
| 失焦自动隐藏 | **默认开启**（`autoHideOnFocusLoss`）。`FocusMonitor`(200ms 节流)+`FocusDetector`（Windows: PowerShell 查 `aces` 进程；Linux: xdotool；macOS: AppleScript）；无新线程。**overlay 的 run 循环和热键 toggle 必须尊重 `AlwaysOnTopCoordinator.isOverlaysHidden()`**：BaseOverlay 每 200ms 会 `setVisible(true)` 复活自己，P 键的 FM 拆包面板因此会在失焦隐藏后又被唤出；游戏态 `isPreview` 例外 |
| 网络代理 | **代理只作用于 HttpURLConnection（GitHub 更新检查/FM 数据下载），绝不全局生效**。`Launcher` 设 `http(s).proxyHost/Port`，来源优先级 `-D` > `HTTPS_PROXY/HTTP_PROXY` 环境变量（仅终端启动可见，GUI 双击无 shell 环境）> Windows 系统代理注册表（`SystemProxyResolver` 读 `HKCU\...\Internet Settings`，Clash/v2rayN 写这里；支持 `host:port` 与协议列表两种 ProxyServer 形式）。**禁止设 `java.net.useSystemProxies=true`**（1.586 线上回归根因）：JDK 8 裸 Socket（`SocksSocketImpl.connect`）每个连接都查 ProxySelector，系统代理为 `host:port` 简写时 JDK 原生实现把它当 SOCKS 返回 → 连 `127.0.0.1:8111` 都进代理隧道；系统代理分支不检查默认 nonProxyHosts，ProxyOverride 又只做字符串前缀匹配（`127.*` 匹配不到 `127.0.0.1`，只有 `localhost` 命中）。8111 的裸 Socket 一律 `new Socket(Proxy.NO_PROXY)`（OneShotHttp/OtherService/fmCmd），本地流量永不经过代理。**不做端口扫描探测**（用户拍板）：只开本地端口没开系统代理的工具属于用户侧开关没开，不代猜 |

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

**新增 HUD 原子组件**：实现 `HUDComponent`，尺寸一律基于 `ctx.hudFontSize`（禁硬编码像素），`draw()` 零分配，在 MiniHUD 组件注册表登记类型；不要再做“复合行”类型。详见 `src/ui/component/CLAUDE.md`。

### MiniHUD 自定义布局

- 布局模型是**自由画布**：所有组件都是原子组件，保存画布绝对坐标（单位 = 基准行高），没有父级/锚点/隐式依赖；拖动谁只改谁。窗口跟随内容包围盒，游戏态紧贴内容，编辑态额外留工作区余量。
- 不存“复合组件”：文本组件只表达一个读数或一段静态文本，条/仪表只画自己的表盘。编辑器里的“行/预设”是一次性创建多个原子组件并成组的宏，不是组件类型；插入后与手摆组件完全等价，可解组、单删、单改。
- 分组是显式的选择/移动集合，不是坐标父级；双击组件可选中同组。窗口位置按画布原点锚定保存（编辑工作区不落盘），内容增删/字体变化时 HUD 不整体漂移。
- 不要再把父子/九宫格/锚点带回来：分组一旦参与坐标换算，就会重新出现“拖一个动一片”。窗口只按可见内容包围盒贴合；编辑态额外留工作区余量。
- 界面分**预览（只读）**和**编辑**：默认只读，点「编辑」才能多选、拖动、缩放、右键增删/改属性；编辑态显示边界框、图层、撤销/重做；游戏内始终只读。拖动中不写盘，松手后保存。
- 所有“怎么显示”都是组件实例属性：表速还是马赫、标签文字、自动/气压/雷达高度、罗盘随体/离体；不存在全局显示方式开关。襟翼条与数据区襟翼文字互相独立。
- 预览态显示值也归组件：`preview` 是组件自己的演示文本，`template` 只负责宽度防抖；预览时文本组件不被全局演示数据覆盖。
- 出厂默认必须与旧版一致：**三个文字标签全关**，右列按 3.5 行高；旧版默认预览是 `α 20`、`0/100`、数据区不显示襟翼文字但显示 `BRK/GEA`。不要把 ALT/SEP 标签带进默认布局；导入旧配置时才按旧开关还原标签偏好。
- 老用户升级时，旧逐项开关会一次性换算成等价布局；窗口位置/字体/大小等偏好不动；旧开关随后消失。旧版三个标签全关时右列会收窄到 3.5 行高，迁移/默认布局必须按标签状态选几何，不能只改 label 文本。未发布的相对父子布局（V1）会提取标签/可见性偏好后重建为自由画布。
- MiniHUD 设置页首个组是「功能列表」：用只读 `help` 行逐项列出画布组件，悬浮复用 `:desc`/`:desc-img` 显示说明与旧版示意图；`help` 行不绑配置、不参与 `withInterest`，一行用 `:column 4` 放多个。新增组件时同步补一条。
- **机场距离面板是普通画布组件**：`MiniHudWidgets` 注册 `airfield` 类型，`AirfieldWidget` 包住 `MiniHudAirfieldPanel`；进入 `miniHudLayout`/`scene`，可选中、拖动、缩放、调图层、撤销与持久化。**不要再为单个面板开 overlay 原生特例**（早期把机场面板放在 `MiniHUDOverlay` 里特殊定位，导致编辑器无法拖动，已改正）。默认布局在内容右侧放一个机场组件；V2 布局加载时由 `MiniHUDOverlay` 一次性补组件并落盘为 V3，V3 文档里用户删除后不会自动重现。机场数据由 `Service` 每 10 个轮询周期从 `/map_obj.json` 解析（`MapObj.parseAirfields` + `resolveAirfields`），归一化坐标 × `/map_info.json` 的 `grid_size` 得米数；**不要用 `cmapmaxsize`（map_max-map_min）代替 `grid_size`**，真机快照两者不相等。只显示跑道 >600m 的机场，blink≠0 或红色系判敌军，按距离升序。`Airfield` 还携带世界方位（箭头方向 = 方位 - 当前航向）和地图网格（与玩家 mapGrid 同一公式）。配置在 MainForm「MiniHUD → 机场面板」组：箭头指向/全部或最近/地图区域/友军颜色/敌军颜色/告警距离（默认 5km）；**只有敌军机场会告警**，告警时文字颜色不变、整行琥珀色方框按 400ms 闪烁。配置键全部进 `withInterest` 实时刷新。阵亡/观战/过渡帧没有 Player 图标或玩家非活动时，`Service` 清空机场列表——地图此时仍会返回整图机场，坐标不可信；阵亡判定统一走公共 `MapObj.isPlayerDead(json[, loc])`（无 Player 图标或坐标越界），不要在调用方重复实现。死亡检测要先于 `formatDataAsStrings()` 清空，否则本帧事件会把旧列表推给 EDT。机场行数/宽度变化时组件首选尺寸变化，EDT 用 `scene.syncPreferredSizes()` 触发重新适配（拖动中冻结）。
- 改渲染/布局后先用离屏快照 + 基线对比确认（工具在本地 `build/`，不进 git），不要只看编译，也不要把效果验证全推给测试同学。

### 子模块文档

| 模块 | 文档 |
|------|------|
| Parser（FM/遥测） | [`src/parser/CLAUDE.md`](src/parser/CLAUDE.md) |
| 配置系统 | [`src/prog/config/CLAUDE.md`](src/prog/config/CLAUDE.md) |
| 工具类 | [`src/prog/util/CLAUDE.md`](src/prog/util/CLAUDE.md) |
| UI 工具 | [`src/ui/util/CLAUDE.md`](src/ui/util/CLAUDE.md) |
| Overlay 开发 | [`src/ui/overlay/CLAUDE.md`](src/ui/overlay/CLAUDE.md) |
| HUD 布局引擎 | [`src/ui/layout/CLAUDE.md`](src/ui/layout/CLAUDE.md) |
| UI 模型 / TelemetrySource | [`src/ui/model/CLAUDE.md`](src/ui/model/CLAUDE.md) |
| HUD 组件 | [`src/ui/component/CLAUDE.md`](src/ui/component/CLAUDE.md) |
| FM 对比规则 | [`src/ui/window/comparison/CLAUDE.md`](src/ui/window/comparison/CLAUDE.md) |
