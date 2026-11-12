# HUD 组件（ui.component）

MiniHUD 的可插拔视觉部件：`HUDComponent` 接口（`getId`/`getPreferredSize`/`draw(Graphics2D,x,y)`/`onDataUpdate(HUDData)`），`AbstractHUDComponent` 提供可见性管理。现有：`LinearGauge`/`LabeledLinearGauge`（条形表）、`SpeedRatioBar`、`FlapAngleBar`、`CompassGauge`（含离体/随体模式与北极三角）、`CrosshairGauge`（贴图或软件绘制）、`AttitudeIndicatorGauge`、`TextGauge`、`WarningOverlay`。MiniHUD 的原子组件在 `ui/minihud/widget/`。

**彻底原子化（v1.587+）**：MiniHUD 注册表里只有原子组件：文本（一个读数或一段静态文本）、条/量表（一根条及其刻度）、罗盘、姿态仪、背景板。**禁止**再出现“一个组件内部排多个独立读数/条/表”的复合组件；`label/unit/precision` 是文本组件的格式化属性，不是可独立移动的子组件。编辑器里的“行/预设”是一次性创建多个原子组件并成组的宏，插入后就是普通组件。显示格式由各组件按实例属性自行生成，不再依赖全局开关。

`CrosshairGauge` 现由独立 `CrosshairOverlay` 使用：矢量臂从中心向两侧各延伸 `width`，自然外接 `2×width`（`getPreferredSize()` 的 `width×width` 只是圆）；独立开窗和绘制原点按此计算。

## 硬性契约

- **单位化尺寸**：**禁止硬编码像素**（4K/低分屏必坏）。一切尺寸 = `ctx.hudFontSize` × 浮点倍数（1 Unit ≈ 1 行高）；`getPreferredSize()` 按字体度量计算
- **draw() 零分配**：60+ FPS 热路径，禁止 `new Color/Font`、`String.format`——Color/Font 缓存为字段或 static final，数字用复用 `char[]` 缓冲（参考 `LinearGauge.valueBuffer`）；`draw()` 必须是无副作用的纯读
- **脏检查**：`onDataUpdate` 里比较新值与 `lastValue`，变化小于阈值直接跳过
- **模板宽度防抖动**：数值宽度随内容变会跳（99→100），用 `setTemplate("ALT 88888")` 锁定最大宽度
- **预览值归组件**：`preview` 属性是该组件在设置预览里的演示文本；不要再靠全局演示数据覆盖文本组件，否则自定义/迁移布局会露出错误的模板占位值。出厂默认的演示值是旧版预览值（`α 20`、`0/100`、`BRK/GEA`、无 ALT/SEP 标签）
- 数据一律经 `onDataUpdate(HUDData)` 传入，组件无状态不自己取数

## 布局系统

MiniHUD 是自由画布：场景只按组件绝对坐标（单位 × `ctx.hudFontSize`）算矩形，没有父链、锚点或 DAG。窗口只负责贴合内容包围盒；分组只影响选择/一起移动，不参与坐标换算。

## 新增原子组件四步

1. 实现 `HUDComponent`（或继承 `AbstractHUDComponent`），只画一个语义视觉
2. 在 MiniHUD 组件注册表登记类型（type 永不复用/改名，并声明分类/缩放能力）
3. 按实例属性创建组件并套用样式
4. 需要实时数据的在场景构建/更新里接线

不要再往 Overlay 里加固定组件列表；也不要新增“复合行”类型，行由编辑器预设宏创建多个原子组件。

## 常见坑

| 症状 | 原因 |
|------|------|
| 堆增长/OOM | draw() 里分配对象——Color/Font 未缓存 |
| 布局跳动 | 未用模板宽度 |
| 组件内容被窗口裁掉 | 窗口未按内容包围盒重算，或编辑态没留工作区余量 |
| 小字体文字压到下一行 | 默认按自身字体基线；旧行内小字要与主字体共基线时显式设置 baseline=main |
| 拖动一个组件带动一片 | 误把分组/父级做成坐标换算；分组只能影响选择与一起移动 |
| 隐藏组件挡住可见组件点击 | 命中顺序必须先可见组件、后隐藏占位 |
| 读数/刻度画到组件框外被窗口裁掉 | 这类绘制扩展要计入窗口 padding，不要只改组件矩形 |
| NPE | ctx 未判空 |

无自动化测试，用 `script/mock_8111.py` 起 mock + 应用预览模式人工验证（不同 HUD 缩放下渲染正确、数据更新无闪烁、长时间运行无内存增长）。
