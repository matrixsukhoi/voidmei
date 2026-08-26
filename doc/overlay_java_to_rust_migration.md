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
