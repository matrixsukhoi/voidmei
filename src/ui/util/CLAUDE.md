# UI 工具（ui.util）

可复用 UI 工具类，静态方法、零分配、线程安全。每个都为收敛 3+ 处重复代码而生。

| 类 | 职责 / 关键方法 |
|----|----------------|
| `OverlayStyleHelper` | `applyTransparentStyle(WebFrame)`（游戏态全透明，替代 setFrameOpaque）、`applyPreviewStyle`（预览态样式）、`loadFontConfig(OverlaySettings)`（字体名/数字字体/字号偏移带缺省） |
| `GraphicsUtil` | `configureOverlayRendering(Graphics2D)`（标准 AA/性能 hints，替代每处 4 行重复）、`createPreciseStroke`（CAP_BUTT 精确端点）、`createRoundedStroke`（CAP_ROUND 装饰线） |
| `SliderHelper` | 只读展示滑条：`configureVerticalProgress`（起落架/襟翼竖条，渐变）、`configureAttitudeSlider`（舵面横条，-100~100）、`removeAllListeners`（去交互） |
| `FastNumberFormatter` | 零分配数字格式化到 `char[]` 缓冲（int/带小数），HUD 行/表盘热路径用 |
| `DialogService` | 对话框显示（`showDialog`/`showColorChooser`），与 AlwaysOnTopCoordinator 协调保证对话框浮于游戏 overlay 之上 |
| `NotificationService` / `NotificationFactory` | toast 通知系统 |
| `ReflectBinder` | 反射绑定（`resolveString`/`resolveInt` 建 supplier，:unit-source 动态绑定用） |
| `UIConstants` | 集中魔数：DPI 基准（`BASE_SCREEN_HEIGHT`=1440、`BASE_FONT_SIZE`）、时间延迟（`DELAY_SHORT/MEDIUM/LONG_MS`=100/500/1000）、尺寸倍率、AoA/AoS 显示上限。静态导入使用 |

新增工具的门槛：模式在 3+ 文件重复才建类；静态方法+清晰参数名；迁移存量代码。
