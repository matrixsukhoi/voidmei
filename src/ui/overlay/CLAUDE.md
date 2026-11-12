# Overlay 开发（ui.overlay）

## 文件与继承层级

```
DraggableOverlay (ui/base/, WebLaF 可拖拽窗)
  ├─ BaseOverlay (列表型: dataPanel + 可插拔 OverlayRenderer/ZebraListRenderer, 后台线程轮 dataSupplier)
  │    └─ FlightInfoOverlay / PowerInfoOverlay / GearFlapsOverlay / FMUnpackedDataOverlay
  ├─ MiniHUDOverlay (组件化自绘, 非 BaseOverlay)
  ├─ CrosshairOverlay (独立准星, 纯静态绘制, 无 FlightDataBus 订阅)
  └─ AttitudeOverlay / EngineControlOverlay / ControlSurfacesOverlay (自绘)
```

另有：`DrawFrame`/`DrawFrameSimpl`（FM 曲线可视化）、`MinimalHUDContext`（MiniHUD 不可变配置快照）、`logic/HUDCalculator`（纯计算）、`model/HUDData`、`SituationAware`。

## 生命周期契约

| 方法 | 时机 | 要点 |
|------|------|------|
| `init(Controller, Service, OverlaySettings)` | 进游戏模式 | 注册 FlightDataBus，产线样式 |
| `initPreview(Controller, OverlaySettings)` | 设置界面预览 | **无 Service**，用静态演示数据；可拖拽；预览样式 |
| `reinitConfig()` | interest 键变化（WYSIWYG） | EDT 上原地更新外观，**不重建** overlay |
| `dispose()` | 退出/关闭预览 | **必须**注销 `FlightDataBus.unregister` **和** `AlwaysOnTopCoordinator.unregisterOverlay`（防僵尸监听/僵尸窗口） |

## 注册（Controller.registerGameModeOverlays）

`overlayManager.registerWithPreview(配置开关键, 工厂, 游戏态init, 预览init, reinitConfig, previewEnabled).withInterest("key1", "key2")`——interest 列表内的配置键变化才触发 `reinitConfig()`，新配置键记得加。复杂可见性条件用 `ActivationStrategy`（`config("key").and(gameModeOnly()/jetOnly()/propOnly())`）。

## 线程与数据访问

- UI 更新一律 `SwingUtilities.invokeLater`；长操作（HTTP/IO）放后台；脏检查后才 repaint
- **数据访问渠道**：高频数值走 `TelemetrySource`（`getAoA()`/`getIAS()` 等）；低频标志走 `event.getPayload()`（isJet/fatalWarn/mapGrid）；FM 配置直接读 `Blkx`（经 `FMManager.getInstance().current()`）
- 配置经 `c.getConfigProvider()`，位置保存经 `OverlaySettings.saveWindowPosition()`；禁止把 config 强转 Controller

## MiniHUD 特有架构

组件化而非列表渲染：`MinimalHUDContext`（不可变快照，reinitConfig 时重建）+ `HUDCalculator`（Service 线程预计算 HUDData）+ 自由画布场景 + 原子 `HUDComponent[]`。HUDData 在 Service 线程预计算后才发布事件，EDT 只消费（降延迟 40-60ms）。组件细节见 `../component/CLAUDE.md`。

**自由画布（v1.587+）**：MiniHUD 的内容/位置由用户布局决定，旧 `showHUD*` 开关已删除。所有组件保存画布绝对坐标，没有父子/锚点；窗口跟随内容包围盒，保存画布原点的屏幕锚点，内容增删或字体变化时其他组件不漂。**不要改回保存窗口左上角**：内容包围盒一变，整个 HUD 会跟着漂。仅设置窗预览态可编辑：**默认只读（只显示「编辑」按钮）**，点「编辑」后才能多选、拖动、缩放、右键增删/改属性；编辑态显示边界框、图层、撤销/重做，游戏态只读。布局存在 MiniHUD 面板的隐藏配置项里，随配置合并/导入/出厂一起走；老配置升级时一次性换算，窗口位置/字体等偏好不动。所有显示方式（表速/马赫、标签、高度来源、随体/离体）都是组件属性，无全局开关；襟翼条与数据区独立。组件扩展与原子类型约定见 `../component/CLAUDE.md`。

准星已拆为独立 `CrosshairOverlay`（配置面板「准星」）。它的面板 x/y 表示**窗口中心**（不是左上角），改大小时围绕中心缩放；`saveCurrentPosition()` 也存中心点。`CrosshairGauge` 矢量臂自然外接 `2×width`，开窗按此留 padding。

## 性能

paint/draw 零分配（Color/Font 缓存）；模板宽度防抖；脏检查。窗口样式/滑条/绘图配置统一用 `ui.util` 的 Helper（见 `../util/CLAUDE.md`）。

## DPI

字体/尺寸乘 `Application.dpiScale`（在 `reinitConfig()` 里做，预览同步生效）；屏幕定位用 `Application.logicalWidth/Height`（逻辑像素），不用 `Toolkit.getScreenSize()`（物理像素）；MiniHUD 经 `MinimalHUDContext.create()` 自动级联缩放；`BaseOverlay` 的 scaleFactor 已含 DPI。100%/200% 两档都要验证。

## DrawFrame / DrawFrameSimpl

特殊初始化：`DrawFrame.init(Controller, FlightAnalyzer)`（爬升记录后多曲线）；`DrawFrameSimpl.init(Controller)`（透明推力曲线 overlay）。需要 Controller 引用取 FM/Service；配置写经 `xc.getConfigProvider().setConfig()`。同样必须在 dispose 里注销 AlwaysOnTopCoordinator。

## 常见坑

| 症状 | 原因 |
|------|------|
| 关闭后泄漏/崩溃 | dispose 未注销 FlightDataBus |
| Alt+Tab 后销毁窗口复活 | dispose 未注销 AlwaysOnTopCoordinator |
| UI 随机损坏/冻结 | EDT 违规（未 invokeLater / EDT 阻塞操作） |
| WYSIWYG 不刷新 | 配置键没进 `.withInterest()` |
| 预览 NPE | initPreview 没有 Service，未用演示数据 |
| 卡顿掉帧 | paint 分配对象 / 未脏检查 |
| 高分屏过大过小 | 未乘 dpiScale / 用了物理像素 |
