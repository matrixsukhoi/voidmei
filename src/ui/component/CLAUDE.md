# HUD 组件（ui.component）

MiniHUD 的可插拔视觉部件：`HUDComponent` 接口（`getId`/`getPreferredSize`/`draw(Graphics2D,x,y)`/`onDataUpdate(HUDData)`），`AbstractHUDComponent` 提供可见性管理。现有：`LinearGauge`/`LabeledLinearGauge`（条形表）、`SpeedRatioBar`、`FlapAngleBar`、`CompassGauge`（含离体/随体模式与北极三角）、`CrosshairGauge`（贴图或软件绘制）、`AttitudeIndicatorGauge`、`TextGauge`、`WarningOverlay`、`row/`（HUDRow/HUDTextRow/HUDAkbRow/HUDEnergyRow/HUDFlapsRow/HUDManeuverRow）。

## 硬性契约

- **单位化尺寸**：**禁止硬编码像素**（4K/低分屏必坏）。一切尺寸 = `ctx.hudFontSize` × 浮点倍数（1 Unit ≈ 1 行高）；`getPreferredSize()` 按字体度量计算
- **draw() 零分配**：60+ FPS 热路径，禁止 `new Color/Font`、`String.format`——Color/Font 缓存为字段或 static final，数字用复用 `char[]` 缓冲（参考 `LinearGauge.valueBuffer`）；`draw()` 必须是无副作用的纯读
- **脏检查**：`onDataUpdate` 里比较新值与 `lastValue`，变化小于阈值直接跳过
- **模板宽度防抖动**：数值宽度随内容变会跳（99→100），用 `setTemplate("ALT 88888")` 锁定最大宽度
- 数据一律经 `onDataUpdate(HUDData)` 传入，组件无状态不自己取数

## 布局系统

`ModernHUDLayoutEngine`：组件以 anchor（九宫格）挂到父节点，偏移以 Unit×行高 计，父链构成 **DAG 经拓扑排序解析绝对坐标——循环依赖会崩引擎**。

## 新增组件四步

1. 实现 `HUDComponent`（或继承 `AbstractHUDComponent`），遵守上述契约
2. `MiniHUDOverlay.initComponentsLayout()` 实例化
3. `HUDLayoutNode` 挂进布局 DAG
4. `MiniHUDOverlay.updateComponents()` 里 `onDataUpdate(data)` 接线

## 常见坑

| 症状 | 原因 |
|------|------|
| 堆增长/OOM | draw() 里分配对象——Color/Font 未缓存 |
| 布局跳动 | 未用模板宽度 |
| 布局引擎栈溢出 | 父链成环 |
| NPE | ctx 未判空 |

无自动化测试，用 `script/mock_8111.py` 起 mock + 应用预览模式人工验证（不同 crosshair 缩放下渲染正确、数据更新无闪烁、长时间运行无内存增长）。
