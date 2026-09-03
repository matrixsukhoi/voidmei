//! 跨线程命令/事件 (D8: 全部经 channel/bus) — UiCommand/TrayCommand/MainEvent/
//! SupervisorOutcome/DebounceMsg 五枚举。重构波2 自 app_shell.rs 拆出。

use vm_core::config::configuration_service::GlobalColors;

/// UI→shell 命令。**按变体有唯一属主** (见各变体注释); 主线程侧经
/// [`crate::AppShell::dispatch`] 路由, 渲染线程侧变体由发送方经
/// [`crate::AppShell::send_ui`] 受控直达 (E9a: 通道发送端私有,
/// 禁外部绕过 dispatch 裸发)。
#[derive(Debug, Clone, PartialEq)]
pub enum UiCommand {
    /// MainForm.confirm "开始游戏" — **主线程属主**
    /// (MainForm 侧 vm-ui W2 接线调 `AppShell::dispatch`)。
    StartGame,
    /// MainForm 底部"结束游戏"按钮 (保存配置 + System.exit(0)) —
    /// **主线程属主** (退出经 exit_requested, 见 dispatch 处理注)
    EndGame,
    /// Java OverlayManager.openAll (Controller.openpad) — 渲染线程属主
    OpenAllOverlays,
    /// Java OverlayManager.closeAll (closepad/endPreview/stop 步1) — 渲染线程属主
    CloseAllOverlays,
    /// WYSIWYG 刷新 (Java refreshPreviews(changedKey)/refreshAllPreviews) — 渲染线程属主。
    /// `generation` = 发送时 previewGeneration 快照, 渲染线程消费侧做防过期守卫
    /// (D8 修正★2: Java 在 ConfigDebounce 线程直碰 UI 组件, Rust 改在本线程刷新)。
    /// `changed_key`: None = 全量刷新 (refreshAllPreviews / ACTION_RESET_COMPLETED)。
    RefreshPreviews {
        changed_key: Option<String>,
        generation: u64,
    },
    /// Java OverlayManager.reinitActiveOverlays (非 PREVIEW 态配置变更) — 渲染线程属主
    ReinitActiveOverlays,
    /// WYSIWYG reinit 参数直送 (PORT 新增命令, 五色直送同款模式): 主线程
    /// CONFIG_CHANGED 时即时读配置重建 [`vm_overlay::platform::reinit::ReinitParams`]
    /// (纯值 Send), 先于 RefreshPreviews/ReinitActiveOverlays 入队 — 渲染线程存入
    /// 线程局部参数仓供各 spec 工厂 reinit 闭包读取 (配置 !Send, 值随命令进线程)
    /// — 渲染线程属主
    /// Box: 参数包 ~272B 远大于其余变体, 装箱拉平枚举尺寸 (clippy large_enum_variant)
    ReinitOverlays {
        params: Box<vm_overlay::platform::reinit::ReinitParams>,
    },
    /// 游戏失焦隐藏全部 overlay (Java FocusMonitor → hideAllOverlays;
    /// 不销毁实例) — 渲染线程属主
    HideAllOverlays,
    /// 游戏复焦恢复 (Java showAllOverlays) — 渲染线程属主
    ShowAllOverlays,
    /// AA 开关更新 (cfg AAEnable — Java 同开同关 graph/text 两 hint, Rust 仓单值;
    /// 直读 cfg 即时值, 配置 !Send) — 渲染线程属主
    SetAa(bool),
    /// 全局五色更新 (Java: 改色 → CONFIG_CHANGED(font 前缀全局键) → 刷新;
    /// Rust 配置 !Send, 色值随命令直送渲染线程的 global_colors 仓) — 渲染线程属主
    SetGlobalColors(GlobalColors),
    /// 渲染线程退出 (host 停泵 + 托盘 NIM_DELETE)
    Shutdown,
}

/// 托盘动作 (渲染线程 AppTrayHandler → 主线程监督循环)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// 左键/"设置" (Java 托盘: ctr.stop(); ctr = new Controller())
    Activate,
    /// 菜单"开始" — PORT(多出能力, 非 Java 菜单项): Java 托盘菜单仅 about/close,
    /// 无"开始"项; Rust tray.rs 提供独立 start 入口,
    /// handler 语义 = Controller.start() 的服务启动部分 (保真)。多出面的回收
    /// 归 tray.rs 波次, 本侧仅忠实转发。
    Start,
    /// 菜单"关于" (Java about 菜单项 → NotificationService.showAbout×3;
    /// 纯展示动作, 不重建核 — 组装层转发前端 About Modal)
    About,
    /// 菜单"退出" (Java close 菜单项 → System.exit(0) 的归属方)
    Exit,
}

/// 主线程监督循环消费的事件 (Controller 订阅闭包只转发不处理 — 配置 !Send,
/// 实际处理落在主线程 [`crate::AppShell::handle_main_event`])
#[derive(Debug, Clone)]
pub enum MainEvent {
    /// UIStateBus CONFIG_CHANGED 载荷 (Java configChangedHandler 输入)
    ConfigChanged(String),
    /// UIStateBus UI_READY (MainForm 首显 → Controller.Preview)
    UiReady,
    /// FM_CHANGED 载荷摘要 (Java fmChangedHandler: toast + 防抖全量刷新)。
    /// name=Some 即 missing/corrupt (toast 面); name=None 为纯刷新调度信号
    FmChanged { name: Option<String>, corrupt: bool },
    /// 托盘动作
    Tray(TrayCommand),
    /// overlay 位置存档 (渲染线程拖拽松手/销毁链 → 主线程落盘)。
    /// section = 配置组标题 (Java OverlaySettings 按 sectionName 查 GroupConfig),
    /// 坐标归一化 (Java saveWindowPosition 的 gc.x/y 量纲)
    PositionSaved { section: String, x: f64, y: f64 },
}

/// 分相监督循环 ([`crate::AppShell::run_supervisor_phase`]) 的退出形态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorOutcome {
    /// 进程退出请求 (EndGame / 托盘 Exit / 监督通道关闭)
    Exit,
    /// 托盘 Activate 已重建核, 请求弹设置窗 (主循环回相 A)
    MainFormRequested,
}

/// ConfigDebouncer 输入 (Java 两个 handler 共用 configDebouncer 的两种任务载荷)
#[derive(Debug, Clone, PartialEq)]
pub enum DebounceMsg {
    /// CONFIG_CHANGED 的配置键 (任务体: refreshPreviews(key))
    ConfigKey(String),
    /// FM_CHANGED (任务体: refreshAllPreviews)
    FmChanged,
}
