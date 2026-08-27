# Overlay Java → Rust 迁移文档 (FlightInfoOverlay POC)

> 状态: M0-M4 全部完成 (2026-08-26)。
> POC 代码: `rust/`; Java 端对拍基线: `src/ui/debug/OverlayPngExport.java`;
> 一键对拍: `python script/build.py test rustcmp`。

## 1. 背景与目标

VoidMei 主体为 Java 8 Swing, 计划迁移 Rust。overlay 是技术难点 (透明窗口/穿透/低占用/跨平台),
先以 FlightInfoOverlay 做最小复现验证技术栈, 并建立像素级比对方法论。

POC 验收维度: 与 Java 版离屏导出 PNG 比对 (尽力而为+人工审)、mock_8111 数据端到端、
交互特性清单 (拖拽/穿透/置顶)、CPU/GPU 占用。

**非目标**: 其它 overlay、FM 链路、语音/热键、MainForm、macOS、原生 Wayland
(War Thunder Linux 版跑 X11/XWayland)。

## 2. Java 现状架构速览

```
FlightInfoOverlay extends FieldOverlay extends DraggableOverlay (WebLaF WebFrame)
  数据: Service 线程 50ms 轮询 127.0.0.1:8111 (/state + /indicators)
        → FlightDataBus 事件 → FieldOverlay.onFlightData (50ms EDT 节流)
        → FastNumberFormatter 零 GC 写 char[32] buffer
  渲染: paintComponent → BOSStyleRenderer.render(g2d, fields, ctx, offset)
        → 每字段一个 TextGauge (数值右对齐 + 标签 + 单位, 各带 +1,+1 阴影)
  窗口: WebLaF 装饰窗 per-pixel 透明 (AWT/ULW 路径), always-on-top 由
        AlwaysOnTopCoordinator 管理; 游戏模式 setFocusable(false) + 空白光标
        (注意: Java 版无真鼠标穿透, 点击仍命中窗口但不响应)
```

关键源文件: `FieldOverlay.java`(事件/布局), `BOSStyleRenderer.java`(网格),
`TextGauge.java`(三段式文本), `RenderContext.java`(字号/尺寸公式),
`FastNumberFormatter.java`(格式化), `Service.java`(派生量)。

## 3. 技术选型与被否方案

| 方案 | 结论 | 理由 |
|---|---|---|
| winit + softbuffer | **否** | softbuffer 不支持 alpha 透明 (winit#2960) |
| winit + wgpu/glow (含 egui/eframe) | **否** | 用户群有 GPU 冲突史 (项目 GPUCompatibilityHelper 专门禁硬件加速); overlay 不应引入 GL/Vulkan 上下文 |
| winit + 平台呈现 | 部分 | winit 在"透明呈现"上无增益 (仍需平台代码), 事件循环自写更薄 |
| **平台原生 + 纯 CPU (采用)** | ✅ | Windows: WS_POPUP + WS_EX_LAYERED + UpdateLayeredWindow; Linux: X11 depth-32 visual。与 Java AWT 同路径 (ULW), 零 GPU 依赖 |

Rust 依赖 (5 个): swash (光栅化), ttf-parser (度量), png, serde_json (M3), windows/x11rb (按平台)。
未用 tiny-skia/cosmic-text: 纯文本 blit 不需要矢量基元与 shaping (Sarasa 等宽),
自管理直通 RGBA 画布 + SrcOver 合成 (数学上与 Java2D TYPE_INT_ARGB 等价)。

## 4. 模块映射表 (Java → Rust)

| Java | Rust | 说明 |
|---|---|---|
| `RenderContext` | `rust/src/layout.rs RenderCtx` | 字号/尺寸公式 1:1 |
| `BOSStyleRenderer` + `TextGauge` | `rust/src/render.rs render_fields` | 网格布局 + 三段式文本 + 阴影顺序 |
| `Font`/`FontMetrics` | `rust/src/font.rs LoadedFont` | ttf-parser 度量 + swash 光栅化 |
| `FastNumberFormatter` | `rust/src/format.rs` | 半舍入/负零抑制/NaN→N/A |
| ui_layout.cfg 字段定义 | `rust/src/fields.rs FIELDS` | 16 行常量表 (POC 快照) |
| `Service` 派生量 | `rust/src/data/derive.rs` (M3) | 公式搬运 + SMA |
| `DraggableOverlay` 拖拽/位置 | `rust/src/window.rs DragState` + `config.rs` | 归一化坐标持久化 |
| WebLaF 透明窗 | `rust/src/platform/win.rs` | ULW 预乘 BGRA |
| `AlwaysOnTopCoordinator` | 平台窗口创建即 TOPMOST | 单窗口 POC 无需协调器 |

## 5. 渲染复刻规范 (常量表, 已像素对拍验证)

- `fontSize = 24 + fontAdd`; numFont=BOLD(fontSize), labelFont=BOLD(round(fontSize/2)), unitFont=PLAIN(round(fontSize/2))
- `totalWidth = (fontSize>>1) + int((columnNum+0.5)*5*fontSize)`; 默认参数 192px
- `totalHeight = numHeight + (ceil(visible/columnNum)+1)*numHeight`; 默认 18*numHeight
- numHeight = Java FontMetrics 实测 (Sarasa BOLD @24px = 31: ascent24+descent6+leading1)
  ⚠️ 无法从 ttf 表推算 (Java FontDesignMetrics 取整策略特殊), 由对拍脚本从 java meta 注入
- 渲染起始 (fontSize>>1, fontSize>>1); 列步进 round(5*fontSize), 行步进 numHeight, 每列 columnNum 个字段换行
- TextGauge: `lwidth=(13*fontSize)>>2` 标签区; 数值右对齐 `x+lwidth-valWidth-numPadding`
  (numPadding=max(4,fontSize/4)), **基线** centerY=(y+y+labelSize+unitSize)>>1;
  标签基线 (x+lwidth, y); 单位基线 (x+lwidth, y+labelFontSize)
- 阴影: 每段文本先 (+1,+1) 用 shade 色画一遍再画本体, 顺序 value→label→unit
- 颜色 (#RRGGBBAA, cfg 默认): num/label 白, unit #E89332FF 橙, shade #000000FF 黑
- AA 跟随 AAEnable (默认 true); ⚠️ Java `Color(int,boolean)` 按 AARRGGBB 解读, cfg 是 RRGGBBAA, 必须拆通道
- preview 值是原样字符串不经格式化

## 6. 数据层复刻 (M3 已验证)

轮询模型: 50ms/轮 state+indicators (`Connection: close`, 手写 HTTP/1.1, 读到 EOF);
JSON 宽容提取 (serde_json::Value, 缺失→哨兵 -65535, 顶层非对象→None);
主端口 8111 失败翻 9222 (Java appPortBkp 行为); 失败保留上帧 + 1条/秒 WARN。

派生公式 (源: Service.java, 已单元测试验证):
- `An = g*sqrt(Ny² + 1 - 2Ny·cos(roll)·cos(pitch+AoA))` (roll/pitch 有效时, 否则 g*Ny) [L795]
- `getNy = An/g` [L1901]; `nVy = vario≠哨兵 ? vario : Vy` [L778]
- speedv 链: `tspeedv = speed≠哨兵 ? speed : IAS/3.6`;
  `iastotascoff = SMA(TAS/(tspeedv*3.6)); speedv = tspeedv*iastotascoff` [L840-872]
- `turnRds = SMA((vp+v)²/(4An))`; `turnRate = toDeg(sqrt(An/turnRds))` [L831-833]
- `acceleration = SMA(v-vp)*1000/Δt`; `SEP = SMA((v+vp)(v-vp)*1000/(2Δt*g) + nVy)` [L988-1012]
- `mach = ias/(3.6*sqrt(1.4/1.225*101325*(1-0.0000225577*H)^5.25588))` [L1214]
- radioAltitude 无效→气压高度 [L761]; wsweep 无效→0 (visible-when 隐藏)
- SMA = CalcHelper.SimpleMovingAverage 渐进均值逐行移植, 窗口 = 1000/50 = 20

mock 端到端实测 (s2_preview_live, p51d): IAS=474, vario=-7.3426, compass=164.097,
预热 100 轮后稳态 SEP=nVy / acceleration=0 (SMA 收敛特性与 Java 同构)。

**注意**: turnRate 的 SMA 环形增量收敛慢 (每样本影响 1/20, 数百轮才稳定),
Java 同款算法同样如此——对拍动态帧时需两边同等预热。

不移植项: compass 地图方向回退, radioAlt 英制 checkAlt 检测, FM 链路。

## 7. 平台窗口实现指南 (Windows 已验证)

`rust/src/platform/win.rs` 实测验证 (2026-08-26):
- `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` — 全 API 物理像素, 对齐 Java `-Dsun.java2d.uiScale=1`
- `WS_POPUP|WS_VISIBLE` + ex `WS_EX_LAYERED|WS_EX_TOPMOST|WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE`
  (TOOLWINDOW: 任务栏无图标; NOACTIVATE: 不抢游戏焦点, 对应 Java setFocusable(false))
- **程序化验证**: ex_style=0x080800A8 → TRANSPARENT/TOPMOST/TOOLWINDOW/LAYERED/NOACTIVATE 全部就位
- present: `CreateDIBSection` (32bpp, **负 biHeight = top-down**) → memcpy **预乘 BGRA** →
  `UpdateLayeredWindow(ULW_ALPHA, SourceConstantAlpha=255, AC_SRC_ALPHA)`
- 显示验证: 双截图差分 (overlay 区域 14.1% 像素差异 = 文本内容真实呈现)
- **稳态 CPU 实测 0.0%** (10 秒 <0.1ms; 事件循环 10ms sleep + PeekMessage 空转)
- 穿透切换: GWL_EXSTYLE 置/清 `WS_EX_TRANSPARENT` (Java 版没有真穿透, 这是特性增强)
- 事件: WNDPROC 收 WM_LBUTTONDOWN/MOUSEMOVE/UP (GetCursorPos 屏幕坐标) 入 Mutex 队列,
  主循环 PeekMessage 泵
- 拖拽: press 记 (root - win_pos) 偏移, move 时 SetWindowPos, release 存归一化位置
  (复刻 Java DraggableOverlay.setupDragListeners; 人工验收项)
- **快速拖拽修复** (实测教训): press 时 `SetCapture` / release 时 `ReleaseCapture`——
  否则指针快速滑出客户区后 MOVE/UP 不再发给本窗口, 拖拽中断; 配合 `WM_CAPTURECHANGED`
  兜底结束拖拽 + MouseMove 队列合并 (事件风暴只留最新位置, 防积压迟滞)
- **预览模式辅助** (对齐 Java applyPreviewStyle): 灰底 = `Application.previewColor`
  = (0,0,0,10) 铺满窗口矩形 (Rust: `Canvas::fill` + `draw_fields` 不清零绘制);
  背景拖拽天然支持 (Win32 鼠标按窗口路由, 不像 Swing 按组件); 箭头光标 =
  WNDCLASSW.hCursor = IDC_ARROW (等价 Java setCursor(null))
- windows-rs 0.62 坑位记录: HDC/指针参数多为 `Option<>` 包装; `BLENDFUNCTION`
  在 Gdi 模块; `SetCapture` 在 `UI::Input::KeyboardAndMouse` 模块; `FindWindow` 需
  CharSet.Unicode

### X11 (设计完成, 待接线验证)

- depth-32 TrueColor visual + `CWColormap|CWBorderPixel` (缺一不可, 否则 BadMatch)
- override-redirect=true 确定性置顶 (取舍: 不参与 WM 层叠/不进任务栏)
- present: XPutImage ZPixmap (预乘 ARGB32); 穿透: XShape input region 置空
- 无合成器时 alpha 失效 (检测 `_NET_WM_CM_S?` + WARN; XWayland 必有合成器)
- 接口契约见 `rust/src/platform/mod.rs` (OverlayWindow trait), 实现 `rust/src/platform/x11.rs` 占位

## 8. 比对方法论与实测

- 图像来源: 双方**离屏导出 PNG** (Java `OverlayPngExport` / Rust `--render-png`),
  带 alpha, 不受桌面合成影响; 需桌面环境 (Java Toolkit 字体度量)
- 硬断言 (整数运算, 必须完全相等): 尺寸 + meta (fontSize/numHeight/totalWidth/totalHeight/visible)
- 软审 (尽力而为): max/mean delta + 差异热力图 (compare 子命令, R=RGB 差 G=alpha 差)
- 一键: `python script/build.py test rustcmp` 或 `bash script/rust_compare.sh`

实测 (2026-08, 开发机):

| 组 | 尺寸 | meta | max_delta | mean_delta | diff 像素 |
|---|---|---|---|---|---|
| preview 默认 | 192×558 | PASS | 255 | 54.6 | 11.7% |
| column=3 fontAdd=4 | 504×280 | PASS | 255 | 48.7 | 10.7% |
| --values 动态帧 | 192×496 (visible 14) | PASS | 255 | 50.5 | 10.9% |
| 单字符 '5' | 48×48 | - | 255 | 66.6 | 4.1% |

差异根因分析 (实验记录):
- 阴影贡献 ~2.6% 像素 (透明化实验)
- 剩余为**字形光栅化差异**: swash/zeno vs Java 底层 FreeType 的 hinting/AA 实现不同
- gamma 幂变换扫描 (1.0-3.0) 无显著改善 (mean 66.6→65.7), 排除覆盖率曲线差异
- 字形位置/行分布/布局完全对齐 (逐行剖面验证)
- 进一步逼近路线: freetype-rs 绑定与 Java 同引擎 (Java 8 OpenJDK libfontmanager 用 FreeType)

## 9. 已知差异与遗留项

1. **鼠标穿透是增强**: Java 版只有 setFocusable(false)+空光标, Rust 版 WS_EX_TRANSPARENT 真穿透
2. 字形光栅化 1px 级差异 (见 §8), 用户已接受"尽力而为+人工审"
3. X11 实现为占位 (开发机 Windows 无法运行验证), 接线要点已写入 §7
4. 配置体系独立 (user_pos.json), 未接管 ui_layout.cfg; 字段/颜色为当前默认值快照
5. numHeight 无法公式化 (Java FontDesignMetrics 取整策略特殊), 依赖 java meta 校准闭环
6. flightInfoEdge 玻璃边框 (WebLaF shadeWidth=10) 未复刻 (默认关闭的配置, 优先级低)
7. live 模式窗口尺寸固定 16 行 (visible-when 变化不重建窗口, 空行透明无碍)
8. 人工验收 (2026-08-26 全部通过): 拖拽 (含快速拖拽 SetCapture 修复)、预览灰底、
   背景拖拽、箭头光标; 多显示器位置语义未覆盖

## 10. 正式迁移路线图 (POC 通过后)

1. **工程拆分**: rust/ 升级 cargo workspace — `overlay-platform` (窗口 trait + win/x11),
   `overlay-render` (font/render/layout/format), `overlay-data` (http/json/derive),
   各 overlay 为独立 bin/模块
2. **X11 接线**: 按 §7 要点实现 x11.rs, Linux CI 加 Xvfb 冒烟测试
3. **配置接管**: S-expression 解析器 (复用 Java SExpParser 语义), 直接读写
   ui_layout.cfg/user cfg, 与 Java 版无缝共享用户配置
4. **渲染逼近** (可选): freetype-rs 同引擎光栅化, 消除 §8 的字形差异
5. **overlay 迁移顺序**: FieldOverlay 系 (动力信息/引擎控制, 复用 POC 全部基建) →
   BaseOverlay 系 (斑马纹列表) → MiniHUD 最后 (组件最多, HUDData 预计算模型最复杂)
6. **数据层扩展**: FM 链路 (Blkx 解析/FMManager 负缓存) 独立 crate, 迁移时参照
   `src/parser/CLAUDE.md` 与 FM 六态状态机
7. **打包分发**: 静态链接 musl (Linux) / MSVC (Windows), 替代 launch4j;
   不再需要 JRE, 分发体积从 ~30MB 降到 ~2MB
8. **MainForm**: 最后迁移 (WebLaF 设置界面, 可选 egui/iced 或保留 Web 前端)

---

# 11. 全量迁移执行记录 (2026-08-26 ~ 08-27 终稿)

> 本文档 §1-§10 为 FlightInfoOverlay POC 阶段内容, 全量迁移已完成, 执行档案见
> `build/migration/` (PORTING 宪法 / CLASSIFY 全库地图 / LIFETIMES / DECISIONS D1-D8 /
> PROGRESS)。以下为终态摘要。

## 11.1 迁移总览

- 源: 183 Java 文件 / 41,242 行 (A 纯逻辑 56 / B 轻耦合 31 / C 平台·UI 95+1 不迁移)
- 产物: `rust/` cargo workspace 五 crate (vm-core/vm-data/vm-overlay/vm-ui/vm-app)
- 方法论: 承袭 Bun Zig→Rust (PORTING 宪法/对抗双审/机械保真优先) + vLLM 规范
  (注释逐字保留/expect-test), 三份宪法文档 + 1+2+1 agent 流水线 (批一验证后放量)
- 规模: ~320 agents / 15 个 workflow 批次 / ~32M subagent tokens / 0 流水线失败
  (批十三一次会话重启中断, resume 缓存恢复零重跑)

## 11.2 终态验收数字

| 验收项 | 结果 |
|---|---|
| cargo test (五 crate) | **1,239 passed / 0 failed** (连跑两遍无 flaky) |
| cargo check / clippy | 零 warning (116 个 Java 保真点 #[allow]+PORT 注) |
| e2e 三场景 (s2/s5/menu) | **全部 PASS** (A1~A6 断言, 复用 Java e2e_assert.py——Logger 格式逐字节保真的闭环) |
| mock 冒烟 | Service 收数 + 6 注册 overlay 逐窗 present=140 帧, 三线程干净退出 |
| 像素对拍 (rustcmp) | preview 11.7% / linear 2.4% / attitude 7.0% / compass 16.6% / MiniHUD 6.3% — **全部无结构性偏差** (bbox 一致/剖面平移 0/差异=AA 光栅化) |
| 真机 FM 测试 | Spitfire/Tempest 燃料断言实跑 + blkx fuzz 200/200 变异体 |
| Java 测试移植 | TestAtmosphere/PistonPower/Visibility/VoicePack/FMStore/FMDataPaths/FMHandle/NaWhen 等全量 |
| 真窗验证 | iced 设置窗 + overlay 预览窗同进程共存 (批十四 EnumWindows 实测) |

## 11.3 Java 版缺陷在迁移中根治 (类型级修复)

1. VoiceWarning UIStateBus 订阅泄漏 (LIFETIMES §2 发现) → RAII Subscription
2. OverlayEntry.close() 锁内回调死锁风险 → 锁内摘槽/销毁链锁外
3. PREVIEW 态 WYSIWYG 刷新链非 EDT (现存线程违规) → UiCommand 走 win32 线程
4. DraggableOverlay 轮询线程 interrupt 无效 (doit 唯一退出) → AtomicBool 统一停机
5. 无真鼠标穿透 → WS_EX_TRANSPARENT 真穿透 (增强)

## 11.4 已知差异与遗留

1. 字形光栅化 AA 差异 (zeno vs FreeType): 全部对拍残差 2~17%, 无结构偏差; 进一步
   逼近可换 freetype-rs 同引擎 (未做)
2. X11 平台层占位 (设计完成, Windows 优先实装)
3. D8 降级尾巴 (可选未迁移): replica 3 件 / DrawFrame×2 / comparison UI 壳 (~5k 行)
4. MiniHUD live 喂数的 getload 降级路径 (无 FM 时) — PORT 备案
5. numHeight 类 FontMetrics 校准闭环依赖 Java meta (Windows 实测 31)
6. 注册面: 游戏模式默认启用 6 overlay 逐窗验收; 其余 overlay 注册键随 ui_layout.cfg
   开关动态生效 (e2e 断言默认集)
7. MainForm 字体: 平台系统中文字体单字体覆盖 (Win "Microsoft YaHei UI" /
   Linux "Noto Sans CJK SC" / macOS "PingFang SC", `vm-ui lib.rs
   PLATFORM_CJK_FONT`); Java 为 Segoe UI + 逻辑字体系统回退, 拉丁字形有近似差异。
   背景: iced 0.13 文本默认 Shaping::Basic 无字体回退, 未显式指定 CJK 字体时
   中文全部 tofu (人工验收发现, 已修复并加 fontdb 命中+字形覆盖测试)
8. overlay 位置持久化: Java overlay 持 OverlaySettings 直接读写 GroupConfig.x/y
   (归一化, 按组标题索引); Rust 配置树 !Send 不能进 win32 线程 → PositionStore
   trait 桥 (host.rs) + 组装层 ChannelPositionStore 快照读/回传写
   (MainEvent::PositionSaved → save_group_position 落盘)。人工验收发现预览
   恒居中 (原为 POC 内存版无持久化), 已修复: 启动读 cfg/user.cfg 位置 → 拖拽
   松手/关闭即时落盘 (对齐 Java saveWindowPosition 语义)
9. FlightInfo overlay (flightInfoSwitch) 原走 POC window.rs 专径未进 host
   组装面 (人工验收发现预览缺窗), 已收编: vm-overlay `flight_info.rs` 工厂
   复用 POC 像素对拍过的 fields/layout/render 渲染栈, 经
   `PixCanvas::composite_straight_frame` 整帧 SrcOver 桥入 host 画布体系
   (预览灰底保留); live 数据 = ServiceData.flight_values (Deriver step 整包
   快照, service_loop 写回)。注册面 7/7 窗口条目, mock 冒烟逐窗 present>0
10. 全局五色 (fontNum/fontLabel/fontUnit/fontWarn/fontShade, ui_layout.cfg:379-383,
    键名不带 Color 后缀): Java 经 loadFromConfig 写 Application 五色静态
    (colorNum 族) → 全组件消费; Rust 侧组件曾用编译期常量 (Java **静态初始值**
    直译, 如 colorNum=(27,255,128,240)) 未接 cfg 运行时覆盖 — 用户 cfg 改色后
    Rust 不跟随 (人工验收: 除 FlightInfo 外全部 overlay 颜色不符)。已接通:
    vm-core `GlobalColors`/`global_colors()` + vm-overlay `global_colors` 受控
    全局仓 (OnceLock<RwLock>, 初始 = Java 静态默认 → 现有测试零感知, ~185
    引用点常量改 `colors().x` 访问器) + vm-app 启动快照注入 + WYSIWYG
    `UiCommand::SetGlobalColors` (五键即时读 cfg 直送 win32, 下帧生效不需
    重建窗口)。对拍工具路径保留常量基线 (rustcmp 不受影响)。后续二轮修复:
    `RenderContext.palette` 曾为构造期字段快照 (动力信息/TextGauge 路径仍冻结
    Java 默认荧光绿), 改为 `palette()` 方法每帧读仓 — Java TextGauge.
    drawTextShaded 本就直读 Application 静态, 方法化才是直译
11. **审查轮 1** (模式 A/B 双 agent, 派生自人工验收四问题的病理提炼):
    - **AA 三兄弟** (aaEnable/textAA/graphAA): Java cfg 键 AAEnable 用户可关
      (缺省 false), Rust 生产渲染 7 处钉死 true — 已修: global_aa 仓 (同五色
      模板) + UiCommand::SetAa + 启动快照注入; 多处"生产恒 ON"错误注释一并
      修正 (Application.java:102 只是声明默认, 非运行时不变式)
    - **HudColors 旁路** (palette 同型第三例): feed 传编译期 application_defaults,
      AoA 告警色不跟随五色 — 已修: feed 处改从 global_colors 仓构造
    - **disableEngineInfo* 7 键 never-wired**: Rust cfg_true 恒 false, 键从未
      被读 — 用户关仪表 Rust 恒显全部 7 条, **启动首帧即错**。已修:
      ENGINE_DISABLE_KEYS 表 + OverlayInputs.engine_disables + 工厂查表闭包
      + 实效测试 (全关窗口 195 < 全开 306)
    - **httpPort 四件套**: 用户改"8111端口" (mock 打桩) Java Service 改轮询
      目标, Rust 恒 env 启动值 — 已修: ServiceConfig 构建读
      application_state().request_dest (load_app_check 已写毕的值), 缺省回退 env
    - **备案不修**: voiceVolumn (语音链未装配, 修了是死代码); 字体族
      fontName/GlobalTextFont/GlobalNumFont (需 字体名→文件映射机制, 迁移
      裁决字体随包分发; vm-ui _FONTS_ combo 本就单选占位); 组 :alpha (Java
      setAlpha 无调用点, 两侧一致不消费)
    - **批 2 进行中**: WYSIWYG 三层 (快照刷新/reinit 执行/窗口 resize) — 触发
      链完好但 Java reinitConfig 重建字体/布局/窗口 vs Rust 只清指纹重绘同像素;
      组件 reinit 方法大多已存在无人调, OverlayWindow 无 set_size
12. **审查轮 2 + 批 2/3 修复** (模式 C 备案复查 + 模式 D cfg 键三方核对):
    - **WYSIWYG 三层已通** (批 2, fix agent): platform set_size (Win32 DIB 重建
      + SetWindowPos) / host resize_entry+reinit_idx (OverlaySpec.reinit 闭包,
      返回新尺寸→原位 resize 不重建窗口) / UiCommand::ReinitOverlays (ReinitParams
      线程局部仓 + MiniHUD 快照解冻); 顺带修"冷激活冻结旧配置尺寸"缺口。7 工厂
      全挂 reinit, 13 个实效测试 (字号↑变高/仪表全关变矮/edge 外扩 20px 等)
    - **.gitignore 根因**: 裸 `config/` 把 src/prog/config/ 从 rg/Grep 静默排除
      (disableEngineInfo 漏检同源盲区) → 锚定 `/config/`
    - **factoryReset/resetConfig 按钮接线**: 曾 Message::Ignore 全哑 — 确认模态
      (Java JOptionPane 等价) → 直调 reset_to_factory/reset_all_layout_defaults
      → 整树收敛 + 广播
    - **FocusMonitor 接线** (轮 2-C 最高价值项): autoHideOnFocusLoss 开关曾只打
      误导日志 — 现 FocusMonitor 随 Service 装配 (start 按 cfg 启停, 会话同生
      共死, 对齐 Java openpad setEnabled 语义), 失焦回调经 ChannelFocusBridge
      (UiCommand Hide/ShowAllOverlays + ControllerShared.overlays_hidden 镜像)
      送 win32 执行 host hide/show
    - **cfg 键审计结论**: 108 键中 68 正常/12 断链 (已修 4: importConfig 按钮
      族中的 reset 两键 + UseNumColor 定性 + attitudeEdge 定性)/27 部分 (多数
      随语音/对比窗口子系统装配批)/1 死键 (两侧一致)
    - **备案 (有据)**: openComparison/openPowerCurve/importConfig (窗口/文件
      对话框未迁移); attitudeIndicatorUseNumColor (键被读但写入字段无读者 —
      无可观测行为, 精确定性注); flightInfoEdge/attitudeEdge (WebLaF 装饰层
      专用, 不进 setBounds); MonoNumFont/flightInfoFontC (随包字体裁决补录);
      voice 23 键 (语音未装配); enableLogging → 批 3 接线中
    - **vm-core 8 处失实注释销号** (activation_strategy/fm_handle/row_registry/
      data_field/gauge_field/service_loop/focus_monitor/overlay_context) —
      "POC 未接配置层"式误导源清除
13. **批 3 补: FlightLog 飞行记录接线** (轮 2-C/2-D 确诊"整面板功能死"):
    enableLogging 开关 → FlightLogSlot (Controller.logon+Log 二位一体) 全链 —
    Service 轮询 tick (每轮, 对齐 Service.java:1824-1828) / openpad 建 (机型名
    从 live 快照) / closepad 存 / 换机关旧开新 / s4to_s1 自动保存; 履行
    flight_analyzer.rs 集成合同 (弃快照 trait, Arc<dyn AnalyzerService> 活读 —
    防曲线数据冻结在 init 时刻); enableAltInformation 随链自动复活。
    deferred: CSV 的 String 列 (约 20 列) 待 formatDataAsStrings 波次 (既有
    TODO, 数值列/analyze 链全实时正确); DrawFrame/toast/writeDown 按 D8/死码
    豁免备案

14. **用户验收 4: MiniHUD bar 恒 0 + 白盒测试端口约定** (真机对拍发现):
    (a) **事件 state 丢失** (主根因): vm-app 转发链通道边界只克隆 EventPayload,
    重建 `FlightDataEvent::new(payload, None, None)` 令 hud_calculator 的
    sState/sIndic 整块跳过 — 襟翼/油门/姿态/G/减速板全 0。修复: 喂数侧从 live
    guard 现值重打快照 (snapshot_state/snapshot_indicators 转 pub), 对位 Java
    事件携带共享可变引用、EDT 时刻读最新值的时序语义。(b) **速度比值 bar 仍 0
    (备案)**: speed_limit_ratio/stall_speed 等 5 字段的 Java 写入方法
    (updateSpeedRatio L1185-1231 / updateStallSpeed L1236-1266) 未移植, 且其
    输入 (vne/翼数据) 由 getload 填充 — 移植前算出 inf/NaN 假数据, 归 getload
    L 批次; 襟翼允许角度同期走 125 缺省 (feed 侧 getload 禁令降级)。
    (c) **白盒测试一律 9222** (用户指令): 真机在跑时 8111 上 bind 探测对战雷
    0.0.0.0 通配监听假阴性 → mock 抢绑失败 + Service 误读游戏数据 (IAS 593≠474
    实测)。修复: connect 探测 + 全白盒面 (vm-data e2e / mock-smoke /
    rust_e2e.sh) 切 9222, 应用侧新增 `AppShell::new_with_port` + `--port` CLI。

## 11.5 人工验收清单 (移交用户)

1. `bash script/rust_run.sh` — iced 设置窗 + 全部 overlay 预览共存
2. 设置窗交互: 开关/滑条/下拉/颜色 → WYSIWYG 实时刷新 overlay 预览
3. 托盘: 图标/菜单/点击防重入; 热键: 注册键触发
4. preview 拖拽 (含快速) 与位置持久化; live 穿透/置顶
5. 游戏实机: WT 开局后 overlay 数据跟随 (8111 真数据)
