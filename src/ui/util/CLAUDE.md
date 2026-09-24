# UI 工具（ui.util）

可复用 UI 工具类，静态方法、零分配、线程安全。每个都为收敛 3+ 处重复代码而生。

| 类 | 职责 / 关键方法 |
|----|----------------|
| `OverlayStyleHelper` | `applyTransparentStyle(WebFrame)`（游戏态全透明，替代 setFrameOpaque）、`applyPreviewStyle`（预览态样式）、`loadFontConfig(OverlaySettings)`（字体名/数字字体/字号偏移带缺省） |
| `GraphicsUtil` | `configureOverlayRendering(Graphics2D)`（标准 AA/性能 hints，替代每处 4 行重复）、`createPreciseStroke`（CAP_BUTT 精确端点）、`createRoundedStroke`（CAP_ROUND 装饰线） |
| `SliderHelper` | 只读展示滑条：`configureVerticalProgress`（起落架/襟翼竖条，渐变）、`configureAttitudeSlider`（舵面横条，-100~100）、`removeAllListeners`（去交互） |
| `FastNumberFormatter` | 零分配数字格式化到 `char[]` 缓冲（int/带小数），HUD 行/表盘热路径用 |
| `DialogService` | 对话框显示（`showDialog`/`showColorChooser`），与 AlwaysOnTopCoordinator 协调保证对话框浮于游戏 overlay 之上 |
| `Toast` | **项目右下角通知库**（特性通知统一走这里）：`show(text, ms)` 定时通知 / `showProgress(text, Action...)` 返回 `Progress` 句柄（update 确定进度/stage 不定模式/close 销毁/dismiss 收起——任务继续、后续更新静默）。`Action(label, onClick)` 通知上的平面文字按钮（EDT 回调），FM 更新的"转后台下载/禁止自动更新"即此。自绘 per-pixel 透明窗+圆角卡片+假阴影+细自绘进度条（折返式扫描）；多条自动垂直堆叠；任意线程安全；headless 全链 no-op。视觉常量集中类头。**已知边界**：per-pixel 透明在 GPU 兼容模式（软件渲染）+ 部分老驱动可能异常，改视觉时真机验一轮 |
| `NotificationService` / `NotificationFactory` | toast 通知系统。`showBottomRight` 已是 `Toast` 的兼容门面（签名不变）；WebLaF `NotificationManager` 那套（引擎倒计时/about）位置是全局设置，与右下角互不相干 |
| `ReflectBinder` | 反射绑定（`resolveString`/`resolveInt` 建 supplier，:unit-source 动态绑定用） |
| `UIConstants` | 集中魔数：DPI 基准（`BASE_SCREEN_HEIGHT`=1440、`BASE_FONT_SIZE`）、时间延迟（`DELAY_SHORT/MEDIUM/LONG_MS`=100/500/1000）、尺寸倍率、AoA/AoS 显示上限。静态导入使用 |

新增工具的门槛：模式在 3+ 文件重复才建类；静态方法+清晰参数名；迁移存量代码。
