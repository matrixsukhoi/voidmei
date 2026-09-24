# 工具类（prog.util）

纯函数工具集：无状态、线程安全、热路径零分配、静态导入友好。**各领域禁止自写实现**，统一走这里。

| 类 | 职责 / 关键 API |
|----|----------------|
| `PhysicsConstants` | 物理常数（`g`/`G`、气压公式系数、ISA 海平面值等）。**永不硬编码** `9.78f`/`9.80` 之类的数 |
| `Interpolation` | `lerp` / `interp1d`（带钳位）/ `interp2d`（双线性，推力表用）/ `interpSweepLevel`（变后掠翼，零分配） |
| `AtmosphereModel` | ISA：`pressure(alt)`、`altitudeAtPressure`（反函数）、`density`、`iasToTas`/`tasToIas`、`ramEffectAltitude`（冲压效应等效高度：高速时进气捕获动压，等效于更低高度） |
| `PistonPowerModel` | 活塞功率曲线：`powerAtAltitudeAdvanced`、`optimalPowerAdvanced`（多级增压自动选最优档）、`generatePowerCurveAdvanced`、`peakWepPower`。**纯计算引擎**——`CompressorStageParams` 必须由 `FMPowerExtractor` 从 FM 数据外部填充；飞机验证一律用真 FM 数据，禁硬编码基准参数 |
| `PowerCurveHelper` | 功率曲线形状判定（`hasConstRpm`、`ceilingIsUseful` 等） |
| `FMPowerExtractor` | FM 文件 → `CompressorStageParams[]`（含燃油品质修正） |
| `ColorHelper` | `parseColor(str, default)`（自动识别 hex/十进制，无效输入返默认不抛）/ `toHexString`（显示）/ `toDecimalString`（存储兼容） |
| `GPUCompatibilityHelper` | `saveSettings`/`isEnabled`/`isSoftwareRenderingActive`。与 `Launcher` 配合：**配置必须在 AWT 初始化前可读**，而 ConfigLoader 用了 AWT 类，故独立存 `gpu_compat.properties`（纯 java.io）而非 ui_layout.cfg |
| `DPIHelper` | `init()` 幂等线程安全，探测失败回退 1.0。暴露 `Application.dpiScale`（1.0=100%, 2.0=200%）、`logicalWidth/Height`。**物理 vs 逻辑像素**：200% 下 `Toolkit.getScreenSize()` 返回物理（3840×2160），Swing 工作在逻辑（1920×1080），DPIHelper 弥合二者 |
| `CalcHelper` / `StringHelper` / `FileUtils` | 通用数学 / 字符串格式化 / 文件 IO。FileUtils：`sha256Hex`（失败返 null 不抛）/ `deleteRecursively`（尽力删）/ `renameWithRetry`（Windows 被占重试）/ `unzip`（保目录结构；zip-slip canonical 前缀校验 + 单条目/总量/条目数上限防 zip 炸弹，上限常量以本地 data 实测校准） |
| `OneShotHttp` | 8111 一次性 HTTP GET（issue #71）。**按真机实测设计**：游戏 8111 一条连接只答一个请求、答完立即 FIN（~0.01ms），无 keep-alive 能力——故每请求一条连接，不池化不重试（轮询循环本身就是重试）。**TIME_WAIT 归零的关键**：读完 body 等 FIN 再关闭（被动方，240 秒等待期由先关的游戏承担）。契约：成功返 body（UTF-8）；失败/超时/chunked/头截断返 `null` 不抛；无状态线程安全。Content-Length 精读修掉旧单次 read 截断；读超时 2s 修掉旧无限阻塞。超时常量集中在类头 |
| `HttpHelper` | 8111 API 门面：`getReqResult` 用 `Future.get()` 汇合（state 池线程 + indic 当前线程保并行；有界等待修掉旧版 completableFuture 不重置导致的串行失效与挂死窗口）；契约：失败 ⇒ `strState` 等共享字段置**空串**（非 null 不抛），Service 据此翻转 8111/9222 端口。低频路径 `getLiveAircraftType`/`sendGetURL`（HttpURLConnection）/fmCmd 各自独立。**公网请求必须走带超时的 `sendGetURL(url, connectMs, readMs)`**（无超时版挂死会占死池线程）；`downloadToFile` 流式写盘 + 进度回调，失败删半成品，跨 host 重定向时校验 github 域白名单（直连 host 由调用方信任，白盒测试传本地 HttpServer） |
| `Logger` | TRACE/DEBUG/INFO/WARN/ERROR，默认 INFO；双参形式带组件名 `Logger.info("Service", "...")` |
| `ExceptionHelper` | `sleepQuietly`（恢复中断标志）/ `logAndContinue(e, ctx)`（WARN 级）/ `closeQuietly` / `ignore`。替代散落的空 catch 块 |
| `FormulaEvaluator` | 反射式运行时公式求值 |

## 性能注记

- `peakWepPower` 遍历高度×速度 2D 网格（0–10000m/100m × 0–800 km/h/50 ≈ 1717 次求值，计冲压效应），用于显示/对比，注意调用频度
- 新增物理/数学工具：常量进 `PhysicsConstants`、写成纯静态方法、Javadoc 标清单位、考虑边界与 NaN 防护
