//! OverlayHost: 多窗口 overlay 管理 (Java OverlayManager + AlwaysOnTopCoordinator 合并语义)
//!
//! PORT: Java 两个类的职责合并 —
//! - OverlayManager (OverlayManager.java): LinkedHashMap 注册表 + per-entry open/close/
//!   refreshPreview 生命周期; Rust 侧 `entries: Vec<OverlayEntry>` 保插入序 (对应
//!   LinkedHashMap, HashMap 每次迭代随机故不可用, PORTING §2.5)。
//! - AlwaysOnTopCoordinator (AlwaysOnTopCoordinator.java): pendingDialogs 计数 + 窗口
//!   注册表; LIFETIMES §1.1 裁决 "Rust 若 overlay 归管理器独占拥有, Weak 注册表整体
//!   不需要 — Drop 即注销, 僵尸窗口防护由所有权天然保证", 故只保留计数 + DialogHooks 钩子。
//!
//! 并发纪律 (重构波3 裁决): 本 host 恒留 win32 单线程 (全方法 &mut self 且整体
//! !Send, 不存在第二条访问路径; 原 Java synchronized→Mutex 的形式保真已摘除 —
//! 槽位 `Option<OverlaySlot>` 直存)。窗口操作 (系统调用) 与 render 闭包 (第三方
//! 代码) 不在持有任何锁的上下文执行的历史约束随摘锁自动满足。
//!
//! 线程模型 (PORT 差异, 有意为之): Java 为每个 needsThread overlay 起一条 doit/sleep
//! 轮询线程 (LIFETIMES §3.1 #13/#14, 游戏模式同时 5-8 条冗余 sleep 线程), 那是 Swing/EDT
//! 模型所迫; Win32 消息队列本就以线程为单位 (一个线程天然泵全部 HWND), 故合并为
//! 单线程泵全部窗口消息 + 脏检查渲染 — 即 LIFETIMES "per-overlay 线程全部废除, 迁移
//! 事件驱动" 裁决的落地。
//!
//! 防御外移记录: Java refreshAllPreviews 的 PREVIEW 状态防御检查
//! (OverlayManager.java:117-121, 防 stale callback 建预览) 与 previewGeneration 世代号
//! (Controller.java:42) 未落本层 — 生命周期状态机归上层 Controller, 该防御随
//! Controller 批次移植时补齐, 此处仅记录防漏带。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::platform::{self, OverlayEvent, OverlayWindow, WindowConfig};
use crate::render::canvas::PixCanvas;

/// 内容渲染闭包: 每帧把 overlay 内容画进画布 (对应 Java overlay 子类的 paintComponent)
pub type RenderFn = Box<dyn FnMut(&mut PixCanvas)>;

/// WYSIWYG reinitializer 闭包 (Java 各 overlay `reinitConfig()` 的执行面):
/// 重建 state/字体/几何, 返回 Some((w,h)) = 新窗口尺寸 (Java setBounds 副作用,
/// host 走 resize_entry 落窗口), None = 尺寸不变或重建失败 (闭包内自行留痕)。
/// PORT: 闭包内部读线程局部 [`crate::platform::reinit::ReinitParams`] 仓取最新参数
/// (配置 !Send, 值随 UiCommand 进 win32 线程 — 五色直送同款模式)
pub type ReinitFn = Box<dyn FnMut() -> Option<(i32, i32)>>;

/// 窗口工厂 (依赖注入点: 生产用 platform::create, 测试注入 mock 做事件分流/销毁序模拟)
pub type WindowFactory = Box<dyn Fn(WindowConfig) -> Result<Box<dyn OverlayWindow>, String>>;

/// dialog 协调钩子 — PORT: Java AlwaysOnTopCoordinator.dialogWillShow/dialogDidDismiss 的
/// suspendAll/restoreAll 语义 (计数归零恢复置顶 + 清 popover)。
/// POC 无对话框阶段: 全部窗口恒 TOPMOST (win.rs 创建即 WS_EX_TOPMOST), 空实现合法;
/// 引入设置主窗后由其实现 (遍历各窗口调 `OverlayWindow::set_topmost`, 等价 Java 的
/// setAlwaysOnTopOnEDT — 主循环单线程内调用, 无需 EDT 转发)。
pub trait DialogHooks {
    /// 对话框将显示 (Java dialogWillShow: pendingDialogs++ → 清 popover → suspendAll)
    fn suspend_overlays(&mut self) {}
    /// 对话框已关闭且计数归零 (Java dialogDidDismiss → restoreAll)
    fn restore_overlays(&mut self) {}
}

/// 位置存档后端 — Java overlay 持 OverlaySettings(loadPosition/saveWindowPosition)
/// 直读写 GroupConfig.x/y; Rust 配置树 !Send 不能进 win32 线程, host 经此 trait
/// 解耦: 组装层注入"快照 + 回传"实现 (vm-app ChannelPositionStore), 测试注入 mock。
/// 坐标恒归一化 (0..1) — 与 host 内存档同量纲; id→配置 section 的映射归组装层。
pub trait PositionStore {
    /// 读初始位置 (Java loadPosition: gc.x/y; None = 无组配置 → host 居中兜底)
    fn load(&mut self, id: &str) -> Option<(f64, f64)>;
    /// 写拖拽/销毁存档 (Java saveCurrentPosition → saveWindowPosition + 落盘)
    fn store(&mut self, id: &str, x: f64, y: f64);
}

/// Java Application.previewColor = (0,0,0,10): 预览模式极淡黑底 (同 window.rs, 便于看清范围)
const PREVIEW_BG: [u8; 4] = [0x00, 0x00, 0x00, 0x0A];

/// 注册用描述 (Java register/registerWithPreview 的参数打包; 注册不建实例)
pub struct OverlaySpec {
    /// 唯一实例键 (Java entries LinkedHashMap 的 key; 亦为位置存档键)。
    /// PORT: Java 三个 register 重载均以 configKey 作 LinkedHashMap 键 — 同 configKey
    /// 后注册者整体替换前者 (只余一个 entry); Rust 以 id 为键, 同 config_key 不同 id
    /// 的两个 spec 可并存双窗。接 Controller 批次时保持 id==config_key 或显式核对此分叉
    pub id: String,
    /// 激活策略引用的配置键 (Java ActivationStrategy.config(configKey))
    pub config_key: String,
    pub width: i32,
    pub height: i32,
    pub render: RenderFn,
    /// WYSIWYG reinitializer (Java reinitConfig; None = 该 overlay 无 reinit 面,
    /// 仅清像素指纹强制重绘)
    pub reinit: Option<ReinitFn>,
}

/// 窗口槽位: Java OverlayEntry 的 instance/thread 字段 (Rust 无轮询线程, thread 无对应物;
/// LIFETIMES §4.2: 反射置 doit 停线程 → Rust 无线程可停, Drop 即完整销毁链)
struct OverlaySlot {
    window: Box<dyn OverlayWindow>,
    /// 拖拽状态机: 按下时 (root - win_pos) 偏移 (Java DraggableOverlay.dragStartX/Y)
    drag: Option<(i32, i32)>,
    /// 上帧像素指纹 (脏检查: 对应 Java repaint 抑制 / window.rs last_frame)
    last_frame: Option<Vec<u8>>,
    /// 窗口当前可见态 (Java isVisible() 的替身记录): Win32 无查询面, 以本记录做
    /// 幂等守卫 (Issue #54 — 重复 setVisible(true) 触发 DWM 全量合成致 DX12 卡顿)。
    /// 全局 hide/show_all 与单条目 set_entry_visible 均经此记录, 二者互不感知
    /// (Java 同形态: FocusMonitor 隐藏后 BaseOverlay.run 的可见分支会再拉起, 保真)
    visible: bool,
}

/// 注册表条目 (Java OverlayManager.OverlayEntry)
pub struct OverlayEntry {
    pub id: String,
    pub config_key: String,
    /// 实例创建模式: true=preview 可拖拽非穿透 (Java initPreview), false=live 穿透 (Java init)
    pub preview: bool,
    width: i32,
    height: i32,
    render: RenderFn,
    reinit: Option<ReinitFn>,
    /// 僵尸实例标志 (Java run() 自动退场形态: 窗口已 dispose 但 entry.instance
    /// 僵留非 null — DrawFrameSimpl 的 displayFmKey==0 收腿退场是唯一置位点)。
    /// 语义对位 OverlayManager.java: open(:294-299) 跳过 / refreshPreview
    /// (:332-336) 只跑 reinitializer 不重建窗口 / close(:370 instance=null) 清除
    zombie: bool,
    /// 窗口槽位 (Java entry 的 instance 字段; monitor 无对应物 — 见模块头并发纪律)
    slot: Option<OverlaySlot>,
    /// 复用画布 (Java overlay 后备缓冲; 首次 open 时创建)
    canvas: Option<PixCanvas>,
    /// 感兴趣的配置键前缀 (Java interestedPrefixes, 默认含自身 config_key)
    interested_prefixes: Vec<String>,
    /// 固定初始位置 (像素) — Java overlay init 的 setBounds 字面量形态
    /// (DrawFrameSimpl 每次 init/initPreview 硬编码 (0, screenH-500)): materialize
    /// 时优先于存档/居中, 且每次重新 materialize 都重 applying = Java 工厂每实例
    /// setBounds 的等价面 (位置存档键 thrustdFSX/Y 只写不读, 不参与定位)
    fixed_pos: Option<(i32, i32)>,
}

impl OverlayEntry {
    /// Java OverlayEntry.isInterestedIn: key==null 恒真; 等于自身键或命中前缀
    fn is_interested_in(&self, changed: Option<&str>) -> bool {
        let Some(k) = changed else { return true };
        if k == self.config_key {
            return true;
        }
        self.interested_prefixes.iter().any(|p| k.starts_with(p.as_str()))
    }
}

/// Java AlwaysOnTopCoordinator 的 GLOBAL_CONFIG_KEYS / GLOBAL_CONFIG_PREFIXES 原样搬移
const GLOBAL_CONFIG_KEYS: [&str; 5] =
    ["AAEnable", "simpleFont", "Interval", "voiceVolume", "ui_layout.cfg"];
const GLOBAL_CONFIG_PREFIXES: [&str; 2] = ["Global", "font"];

/// 全局配置判定: None 恒真; 全局键集合或前缀命中。
/// 全库唯一真相 (voice_setup 的 voice_warn_refresh_reaches 在此基础上追加 enableVoiceWarn)。
pub fn is_global_config(key: Option<&str>) -> bool {
    let Some(k) = key else { return true };
    GLOBAL_CONFIG_KEYS.contains(&k) || GLOBAL_CONFIG_PREFIXES.iter().any(|p| k.starts_with(p))
}

/// 槽位可见性落地: 幂等守卫 (与记录态相同即跳过, Issue #54 DWM 全量合成防抖),
/// 命中才调系统 set_visible 并同步记录 (Java isVisible()/setVisible 对)
fn set_slot_visible(sl: &mut OverlaySlot, visible: bool) {
    if sl.visible != visible {
        sl.visible = visible;
        sl.window.set_visible(visible);
    }
}

struct NoopDialogHooks;

impl DialogHooks for NoopDialogHooks {}

/// 多窗口 overlay 宿主 (Java OverlayManager + AlwaysOnTopCoordinator 合并)
pub struct OverlayHost {
    /// 注册表: 插入序 (LinkedHashMap 语义), 同键重注册原位替换
    entries: Vec<OverlayEntry>,
    factory: WindowFactory,
    /// 激活探测: config_key → 是否启用 (Java ActivationStrategy.config(key).shouldActivate(ctx))
    activation: Box<dyn Fn(&str) -> bool>,
    dialog_hooks: Box<dyn DialogHooks>,
    /// Java AtomicInteger pendingDialogs
    pending_dialogs: i32,
    /// 会话内存档 (归一化屏幕坐标; 同会话拖拽/销毁存档, 优先于 position_store —
    /// Java 同进程内 gc 内存值即最新)。跨进程持久化经 [`PositionStore`] 后端
    saved_positions: HashMap<String, (f64, f64)>,
    /// 位置存档后端 (Java OverlaySettings 的 GroupConfig.x/y; None = 纯内存档)
    position_store: Option<Box<dyn PositionStore>>,
    /// Java volatile boolean overlaysHidden (AlwaysOnTopCoordinator.java:38) —
    /// 游戏失焦隐藏标志。Java 需 volatile 因 FocusMonitor 在 Service 线程调用;
    /// Rust 侧 host 为单线程独占 (&mut self), 服务线程经消息送主循环调用, 普通 bool 即可
    overlays_hidden: bool,
    /// 停机标志 (Java doit volatile 族; LIFETIMES §3.2 → AtomicBool)。
    /// Arc + stop_handle() 跨线程可达: host 本体 !Send, 上层 Controller/Service 线程
    /// 持句柄请求退出 (Java doit 从别的线程置 false 的语义)
    stop: Arc<AtomicBool>,
}

impl Default for OverlayHost {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayHost {
    /// 生产构造: 真实 Win32/X11 窗口工厂, 激活探测默认全启用 (未接配置层的 POC 行为)
    pub fn new() -> Self {
        Self::with_factory(Box::new(|cfg| Ok(Box::new(platform::create(cfg)?))))
    }

    /// 注入窗口工厂 (测试: mock 窗口做事件分流/销毁序模拟)
    pub fn with_factory(factory: WindowFactory) -> Self {
        OverlayHost {
            entries: Vec::new(),
            factory,
            activation: Box::new(|_| true),
            dialog_hooks: Box::new(NoopDialogHooks),
            pending_dialogs: 0,
            saved_positions: HashMap::new(),
            position_store: None,
            overlays_hidden: false,
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 注入激活探测 (Java ActivationStrategy 读 OverlayContext 配置)。
    /// PORT: Java ActivationStrategy.config(key) = Boolean.parseBoolean(getConfig(key)),
    /// 配置缺失 (null) 解析为 false — 未配置的 overlay 默认**不激活**; POC 未接配置层,
    /// 默认探测 `|_| true` 全启用, 接配置层时必须恢复"缺省 false"语义, 否则 open_all
    /// 会打开全部未配置 overlay。
    /// 签名只收 config_key: Java 复合策略 (Controller.java:717-723 enableVoiceWarn =
    /// config+gameModeOnly, 746-752 thrustdFS = config+jetOnly) 由闭包捕获模式/发动机
    /// 类型等外部状态等价实现 — 接口不给 ctx, Controller 批次移植时照此办理
    pub fn with_activation(&mut self, probe: Box<dyn Fn(&str) -> bool>) -> &mut Self {
        self.activation = probe;
        self
    }

    /// 注入 dialog 协调钩子 (PORT: Java AlwaysOnTopCoordinator 挂起/恢复; 空实现合法)
    pub fn with_dialog_hooks(&mut self, hooks: Box<dyn DialogHooks>) -> &mut Self {
        self.dialog_hooks = hooks;
        self
    }

    /// 注入位置存档后端 (Java overlay 的 OverlaySettings 位置面; 缺省纯内存档)
    pub fn with_position_store(&mut self, store: Box<dyn PositionStore>) -> &mut Self {
        self.position_store = Some(store);
        self
    }

    /// 注册 overlay (Java register: entries.put, 不建实例)
    /// 同键重复注册 = LinkedHashMap.put 语义: 原位替换, 保留插入序
    pub fn register(&mut self, spec: OverlaySpec) -> &mut Self {
        // Java 构造器: 默认 interest 自身 key
        let interested_prefixes = vec![spec.config_key.clone()];
        let entry = OverlayEntry {
            interested_prefixes,
            id: spec.id,
            config_key: spec.config_key,
            preview: false,
            width: spec.width,
            height: spec.height,
            render: spec.render,
            reinit: spec.reinit,
            zombie: false,
            slot: None,
            canvas: None,
            fixed_pos: None,
        };
        match self.entries.iter().position(|e| e.id == entry.id) {
            Some(idx) => {
                // PORT: Java LinkedHashMap.put 替换后旧实例被孤儿化 (窗口漏在屏幕上,
                // Java 的 bug); Rust 所有权下旧窗口随条目 Drop 销毁, 但静默 Drop 不走
                // close 销毁链会丢位置存档 — 故先按 close 链收尾 (存位置 + drop),
                // 同键位置存档跨替换保留, 新条目 materialize 时恢复
                self.close(&entry.id);
                self.entries[idx] = entry; // 原位替换
            }
            None => self.entries.push(entry),
        }
        self
    }

    /// 给最后注册的条目加兴趣前缀 (Java withInterest: 影响 refreshPreviews(changedKey) 过滤)
    pub fn with_interest(&mut self, prefixes: &[&str]) -> &mut Self {
        if let Some(entry) = self.entries.last_mut() {
            for p in prefixes {
                entry.interested_prefixes.push(p.to_string());
            }
        }
        self
    }

    /// 条目固定初始位置 (P6 组装契约: Java overlay init 的 setBounds 字面量 —
    /// DrawFrameSimpl 每次 init/initPreview 硬编码 (0, screenH-500), 位置存档键
    /// thrustdFSX/Y 只写不读)。materialize 时优先于存档/居中生效, 每次
    /// re-materialize 重 applying。返回 false = 未注册条目。
    pub fn set_entry_fixed_pos(&mut self, id: &str, x: i32, y: i32) -> bool {
        match self.entries.iter_mut().find(|e| e.id == id) {
            Some(entry) => {
                entry.fixed_pos = Some((x, y));
                true
            }
            None => false,
        }
    }

    /// 条目僵尸化开关 (Java run() 自动退场: 窗口 dispose 后 instance 僵留)。
    /// 置位后 open 跳过 / refreshPreview 不重建窗口 (只跑 reinitializer),
    /// 直至 close 清除 (closeAll 或策略失活路径)。返回 false = 未注册条目。
    pub fn set_entry_zombie(&mut self, id: &str, on: bool) -> bool {
        match self.entries.iter_mut().find(|e| e.id == id) {
            Some(entry) => {
                entry.zombie = on;
                true
            }
            None => false,
        }
    }

    /// 打开单个 overlay — 游戏模式 (Java OverlayEntry.open: 已开则跳过并记日志)
    /// 实例 = live 穿透窗口 (click_through=true), preview 标志置 false
    pub fn open(&mut self, id: &str) -> Result<bool, String> {
        let idx = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| format!("未注册的 overlay: {}", id))?;
        self.open_idx(idx)
    }

    fn open_idx(&mut self, idx: usize) -> Result<bool, String> {
        // 槽位占用检查 (单线程独占)
        if self.entries[idx].slot.is_some() || self.entries[idx].zombie {
            // 退场后 instance 僵留) 同跳过, 不重建死窗口 (OverlayManager.java:294-299)
            return Ok(false);
        }
        // 建窗口 (工厂可能慢/失败)
        self.materialize(idx, false)?;
        Ok(true)
    }

    /// 打开全部 (Java openAll: 按 entries 序, shouldActivate(ctx) 为真才 open)
    pub fn open_all(&mut self) -> Result<(), String> {
        let plan: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| (self.activation)(&e.config_key))
            .map(|(i, _)| i)
            .collect();
        for idx in plan {
            self.open_idx(idx)?;
        }
        Ok(())
    }

    /// 关闭单个 overlay — PORT: Java OverlayEntry.close() 销毁序
    /// Java: saveCurrentPosition() → 反射 doit=false → Window.dispose() → thread.interrupt()
    ///       → instance=null
    /// Rust: 槽位摘取期间完成存位置/销毁链 (单线程独占, 无锁 — LIFETIMES §3.3-1);
    ///       doit/interrupt 无对应物 (无轮询线程), drop(window) = dispose 注销链
    pub fn close(&mut self, id: &str) -> bool {
        let Some(idx) = self.entries.iter().position(|e| e.id == id) else {
            return false;
        };
        // ① 摘槽位 (take 走 ownership; 槽位 None = Java instance=null)
        let taken = self.entries[idx].slot.take();
        // 僵尸清除 (Java close 末尾 instance=null): 无窗口的死实例同样要清标志,
        // 否则 closeAll 后的会话重开会被 open/refresh 的僵尸守卫误拦
        self.entries[idx].zombie = false;
        let Some(slot) = taken else {
            return false; // 未开: Java close() 首行 instance==null 直接 return
        };
        // ② 存位置 (Java saveCurrentPosition: 归一化屏幕坐标)
        let (wx, wy) = slot.window.position();
        let (sw, sh) = slot.window.screen_size();
        if sw > 0 && sh > 0 {
            let n = (wx as f64 / sw as f64, wy as f64 / sh as f64);
            self.saved_positions.insert(id.to_string(), n);
            if let Some(store) = self.position_store.as_mut() {
                store.store(id, n.0, n.1);
            }
        }
        // ③ 销毁窗口 (drop = DestroyWindow; Java Window.dispose → 子类 dispose →
        //    unregisterOverlay 注销链由 Drop 天然完成, 顺序见 Drop 实现体)
        drop(slot);
        true
    }

    /// 关闭全部 (Java closeAll: 按 entries 序逐个 close)
    pub fn close_all(&mut self) {
        let ids: Vec<String> = self.entries.iter().map(|e| e.id.clone()).collect();
        for id in ids {
            self.close(&id);
        }
    }

    /// preview 模式生命周期 (Java OverlayEntry.refreshPreview):
    /// 应开未开 → 建 preview 窗口 (可拖拽); 已开应开 → reinit (标脏强制重绘);
    /// 已开不应开 → close (Java "Closing overlay (inactive strategy)")
    pub fn refresh_preview(&mut self) -> Result<(), String> {
        let all: Vec<usize> = (0..self.entries.len()).collect();
        for idx in all {
            self.refresh_preview_idx(idx)?;
        }
        Ok(())
    }

    /// 按变更配置键刷新 (Java refreshPreviews(changedKey): 全局键或条目感兴趣才刷新)
    pub fn refresh_preview_key(&mut self, changed_key: Option<&str>) -> Result<(), String> {
        let targets: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| is_global_config(changed_key) || e.is_interested_in(changed_key))
            .map(|(i, _)| i)
            .collect();
        for idx in targets {
            self.refresh_preview_idx(idx)?;
        }
        Ok(())
    }

    fn refresh_preview_idx(&mut self, idx: usize) -> Result<(), String> {
        let should_open = (self.activation)(&self.entries[idx].config_key);
        let active = self.entries[idx].slot.is_some();
        let zombie = self.entries[idx].zombie;
        if should_open {
            // WYSIWYG reinitializer (Java refreshPreview → reinitializer)。
            // PORT(冷激活补口): Java 工厂创建实例时读即时配置; Rust spec 工厂是
            // 启动期一次性快照 — 未开实例先跑 reinit 把 state/尺寸刷到最新参数,
            // 再 materialize 建窗 (否则首次激活冻结在旧配置尺寸)
            self.reinit_idx(idx)?;
            // 僵尸实例 (Java instance != null): 只跑 reinitializer, 已 dispose 的
            // 死窗口不重建 (OverlayManager.java:332-336 — 中途换配置不复活退场窗)
            if !active && !zombie {
                self.materialize(idx, true)?;
            }
        } else if active || zombie {
            // 策略再开才会建新实例; OverlayManager.java:337-340)
            let id = self.entries[idx].id.clone();
            self.close(&id);
        }
        Ok(())
    }

    /// 单条目 reinitializer (Java reinitConfig 的组装面): 跑工厂闭包重建 state,
    /// 返回新尺寸则 resize (setBounds 副作用); 无论尺寸是否变化均清指纹强制重绘
    /// (Java reinitConfig 末尾 repaint)
    fn reinit_idx(&mut self, idx: usize) -> Result<(), String> {
        // 闭包是任意第三方代码 (槽位摘取期外执行, 无状态借用交叉)
        let new_size = match self.entries[idx].reinit.as_mut() {
            Some(f) => f(),
            None => None,
        };
        match new_size {
            Some((w, h)) => self.resize_idx(idx, w, h)?,
            None => {
                if let Some(slot) = self.entries[idx].slot.as_mut() {
                    slot.last_frame = None;
                }
            }
        }
        Ok(())
    }

    /// 改条目尺寸 (Java Window.setSize/setBounds): 更新 entry 宽高 + 重建画布
    /// (present 缓冲与新尺寸一致); 活跃窗口 set_size (系统调用)
    pub fn resize_entry(&mut self, id: &str, w: i32, h: i32) -> Result<(), String> {
        let idx = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| format!("未注册的 overlay: {}", id))?;
        self.resize_idx(idx, w, h)
    }

    fn resize_idx(&mut self, idx: usize, w: i32, h: i32) -> Result<(), String> {
        let same = self.entries[idx].width == w && self.entries[idx].height == h;
        if !same {
            self.entries[idx].width = w;
            self.entries[idx].height = h;
            // 画布重建 (materialize 的 is_none 守卫不再重建 — 尺寸变更须显式换)
            self.entries[idx].canvas = Some(PixCanvas::new(w, h)?);
        }
        // ① 摘槽位
        let taken = self.entries[idx].slot.take();
        let Some(mut sl) = taken else {
            return Ok(()); // 未开: entry 尺寸已更新, 建窗时生效
        };
        // ② 窗口 resize (系统调用)
        if !same {
            sl.window.set_size(w, h);
        }
        // 旧指纹尺寸已失配, 清掉强制下一帧 present (同 Java reinit 后 repaint)
        sl.last_frame = None;
        // ③ 放回
        self.entries[idx].slot = Some(sl);
        Ok(())
    }

    /// 重初始化全部已开实例 (Java reinitActiveOverlays): 跑各条目 reinit 闭包
    /// (重建 state + 尺寸跟随), 无闭包者退化为清指纹强制重绘
    pub fn reinit_active_overlays(&mut self) {
        for i in 0..self.entries.len() {
            if self.entries[i].slot.is_some() {
                // 重建失败 (画布分配) 不中断其余条目 — reinit 非销毁链, 单条降级
                let _ = self.reinit_idx(i);
            }
        }
    }

    /// 建窗口并放入槽位 (open/refreshPreview 共用; 位置: 存档 → 屏幕居中, 同 window.rs)
    fn materialize(&mut self, idx: usize, preview: bool) -> Result<(), String> {
        let id = self.entries[idx].id.clone();
        let (w, h) = (self.entries[idx].width, self.entries[idx].height);
        if self.entries[idx].canvas.is_none() {
            self.entries[idx].canvas = Some(PixCanvas::new(w, h)?);
        }
        let cfg = WindowConfig {
            width: w,
            height: h,
            x: 60, // 占位, 创建后按存档/居中修正 (同 window.rs)
            y: 100,
            click_through: !preview,
        };
        let mut window = (self.factory)(cfg)?;
        // 初始位置优先级: 条目固定几何 (Java setBounds 字面量 — DrawFrameSimpl) →
        // 会话内存档 → 配置后端 (GroupConfig.x/y) → 屏幕居中 (Java gc=null 兜底)。
        // 后端命中填入内存档: 后续同会话不再查, 且拖拽存档双写时内存/后端天然一致
        if let Some((fx, fy)) = self.entries[idx].fixed_pos {
            window.set_position(fx, fy);
            return self.finish_materialize(idx, preview, window);
        }
        let initial = self.saved_positions.get(&id).copied().or_else(|| {
            let pos = self.position_store.as_mut().and_then(|s| s.load(&id));
            if let Some(p) = pos {
                self.saved_positions.insert(id.clone(), p);
            }
            pos
        });
        match initial {
            Some((nx, ny)) => {
                let (sw, sh) = window.screen_size();
                // PORT: Java (int) Math.round = floor(x+0.5) (ConfigurationService.java:433);
                // Rust f64::round 是半偶舍入, 恰为 .5 时窗口位置差 1px (PORTING §2.3)
                window.set_position(
                    (nx * sw as f64 + 0.5).floor() as i32,
                    (ny * sh as f64 + 0.5).floor() as i32,
                );
            }
            None => {
                let (sw, sh) = window.screen_size();
                window.set_position((sw - w) / 2, (sh - h) / 2);
            }
        }
        self.finish_materialize(idx, preview, window)
    }

    /// materialize 尾段 (窗口落槽): 重复 open 的已开条目丢弃新建窗
    /// (drop → DestroyWindow 销毁链)
    fn finish_materialize(
        &mut self,
        idx: usize,
        preview: bool,
        mut window: Box<dyn OverlayWindow>,
    ) -> Result<(), String> {
        // Java registerOverlay: 有挂起对话框则暂缓置顶 (AlwaysOnTopCoordinator.java:68-73)
        // — 防对话框期间新建的 overlay 盖住对话框 (该协调器存在意义正是修这个时序 bug)
        if self.pending_dialogs > 0 {
            window.set_topmost(false);
        }
        self.entries[idx].preview = preview;
        // 已开 (重复 open/并发窗口期) 则丢弃新建 (drop → DestroyWindow 销毁链;
        // 重构波3 摘锁后无嵌套锁面, 保留重复占用检查作顺序防御)
        if self.entries[idx].slot.is_some() {
            drop(window);
            return Ok(());
        }
        // visible=true: 窗口以 WS_VISIBLE 建立 (win.rs create)
        self.entries[idx].slot = Some(OverlaySlot { window, drag: None, last_frame: None, visible: true });
        Ok(())
    }

    /// 是否激活 (Java isActive: entry != null && instance != null)
    pub fn is_active(&self, id: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.id == id && e.slot.is_some())
    }

    /// 全部活跃 id, 按注册序 (Java getActiveOverlays)
    pub fn active_ids(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.slot.is_some())
            .map(|e| e.id.clone())
            .collect()
    }

    /// 位置存档查询 (测试/配置层持久化接管用)
    pub fn saved_position(&self, id: &str) -> Option<(f64, f64)> {
        self.saved_positions.get(id).copied()
    }

    /// PORT: Java AlwaysOnTopCoordinator.dialogWillShow — pendingDialogs++ → suspendAll。
    /// 置顶切换动作归 DialogHooks 实现方; POC 空实现 = 窗口恒 TOPMOST 不变
    pub fn dialog_will_show(&mut self) {
        self.pending_dialogs += 1;
        self.dialog_hooks.suspend_overlays();
    }

    /// Java dialogDidDismiss: 计数递减, 归零 (含下溢复位) → restoreAll
    pub fn dialog_did_dismiss(&mut self) {
        self.pending_dialogs -= 1;
        if self.pending_dialogs <= 0 {
            self.pending_dialogs = 0;
            self.dialog_hooks.restore_overlays();
        }
    }

    /// Java getPendingDialogCount
    pub fn pending_dialog_count(&self) -> i32 {
        self.pending_dialogs
    }

    /// 请求主循环退出 (Java doit=false 停机语义)。
    /// Ordering: Java volatile 顺序一致; Release/Acquire 建立同步保证停止标志的
    /// 可见性先于后续读取 (Relaxed 不建立同步, 弱内存平台停止可能不被及时观测)
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
    }

    pub fn is_stop_requested(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    /// 跨线程停机句柄 — host 本体 !Send (含 Box<dyn Fn>/Box<dyn OverlayWindow>),
    /// 上层 Controller/Service 线程持此 Arc 请求退出 (Java doit 从别的线程置 false
    /// 的语义; 状态仍由主循环 is_stop_requested/run 消费)
    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop)
    }

    // ========== 游戏失焦时隐藏/显示 overlay (Java AlwaysOnTopCoordinator.java:191-253;
    // FocusMonitor 200ms 节流探测器归 Service 轮询批次, 此处只落窗口动作面) ==========

    /// 隐藏所有已注册的 overlay 窗口（不销毁实例）。用于游戏失焦时自动隐藏 HUD。
    /// Java: overlaysHidden 幂等标志 + isDisplayable/isVisible 守卫 — Rust 槽位存在 =
    /// 窗口未销毁 (所有权天然保证 isDisplayable); isVisible 守卫省略: 对已隐藏窗口
    /// 重复 set_visible(false) 幂等无害
    pub fn hide_all_overlays(&mut self) {
        if self.overlays_hidden {
            return;
        }
        self.overlays_hidden = true;
        self.for_each_active_slot(|sl| set_slot_visible(sl, false));
    }

    /// 显示所有被隐藏的 overlay 窗口（游戏重新获得焦点时恢复显示）。
    /// 亦为 FocusMonitor.setEnabled(false) 的恢复路径 (FocusMonitor.java:43-47)
    pub fn show_all_overlays(&mut self) {
        if !self.overlays_hidden {
            return;
        }
        self.overlays_hidden = false;
        self.for_each_active_slot(|sl| set_slot_visible(sl, true));
    }

    /// 检查 overlay 是否因游戏失焦而被隐藏 (Java isOverlaysHidden)
    pub fn is_overlays_hidden(&self) -> bool {
        self.overlays_hidden
    }

    /// 单条目窗口可见性 (Java `Window.setVisible` 的 per-overlay 面 — P5 组装契约
    /// (b) 的 host 扩展): BaseOverlay.run() 的不可见分支 setVisible(false) / 可见
    /// 分支按需拉起, 供列表型 overlay 的自管可见性 (FMUnpackedData 热键开关) 落窗。
    /// 幂等守卫 = 槽位 `visible` 记录 (Java isVisible() 守卫同义, Issue #54 防抖);
    /// 未注册/未开条目静默无操作 (Java instance==null 时无窗口可设)。**不动全局
    /// overlays_hidden 标志** — Java setVisible 亦不经协调器 (见 OverlaySlot 注)
    pub fn set_entry_visible(&mut self, id: &str, visible: bool) {
        let Some(idx) = self.entries.iter().position(|e| e.id == id) else {
            return;
        };
        // 槽位摘取期间改可见态; set_visible (系统调用) 在持槽位所有权下执行
        let taken = self.entries[idx].slot.take();
        let Some(mut sl) = taken else { return };
        set_slot_visible(&mut sl, visible);
        self.entries[idx].slot = Some(sl);
    }

    /// 遍历活跃槽位执行动作 (单线程独占, 见模块头并发纪律)
    fn for_each_active_slot(&mut self, mut f: impl FnMut(&mut OverlaySlot)) {
        for i in 0..self.entries.len() {
            let taken = self.entries[i].slot.take();
            let Some(mut sl) = taken else { continue };
            f(&mut sl);
            self.entries[i].slot = Some(sl);
        }
    }

    /// 一轮消息泵: 逐窗口取事件 → 拖拽状态机; Close 事件走 close 销毁链 (槽位放回后)。
    /// 返回本轮因 Close 事件被关闭的 id (Java 无对应返回, 测试/上层生命周期用)。
    /// poll_event (→DispatchMessageW→WNDPROC 回调) 与 set_position/position/screen_size
    /// (系统调用) 均为外部代码; 事件处理在持槽位所有权下执行, 拖拽落点保存推迟到循环尾。
    pub fn pump_events(&mut self) -> Vec<String> {
        let mut closed: Vec<String> = Vec::new();
        let mut position_saves: Vec<(String, f64, f64)> = Vec::new();
        for i in 0..self.entries.len() {
            // ① 摘槽位
            let taken = self.entries[i].slot.take();
            let Some(mut sl) = taken else { continue };
            let mut close_req = false;
            let mut save: Option<(f64, f64)> = None;
            // ② 排空事件 + 拖拽状态机
            while let Some(ev) = sl.window.poll_event() {
                match ev {
                    OverlayEvent::Close => {
                        close_req = true;
                        break; // 槽位放回后由 close() 走完整销毁链 (存位置 → drop)
                    }
                    OverlayEvent::MousePress { root_x, root_y } => {
                        // 仅 preview 可拖拽 (live 穿透收不到鼠标事件, 双保险)
                        if self.entries[i].preview {
                            let (wx, wy) = sl.window.position();
                            sl.drag = Some((root_x - wx, root_y - wy));
                        }
                    }
                    OverlayEvent::MouseMove { root_x, root_y, left_down } => {
                        if let Some((off_x, off_y)) = sl.drag {
                            if left_down {
                                sl.window.set_position(root_x - off_x, root_y - off_y);
                            }
                        }
                    }
                    OverlayEvent::MouseRelease => {
                        if sl.drag.take().is_some() {
                            // Java DraggableOverlay: 松手 → saveCurrentPosition (归一化)
                            let (wx, wy) = sl.window.position();
                            let (sw, sh) = sl.window.screen_size();
                            if sw > 0 && sh > 0 {
                                save = Some((wx as f64 / sw as f64, wy as f64 / sh as f64));
                            }
                        }
                    }
                }
            }
            // ③ 放回槽位
            self.entries[i].slot = Some(sl);
            if let Some((nx, ny)) = save {
                position_saves.push((self.entries[i].id.clone(), nx, ny));
            }
            if close_req {
                closed.push(self.entries[i].id.clone());
            }
        }
        for (id, nx, ny) in position_saves {
            self.saved_positions.insert(id.clone(), (nx, ny));
            // 拖拽松手即落盘 (Java DraggableOverlay mouseReleased → saveWindowPosition
            // + saveLayoutConfig — 持久化不等销毁)
            if let Some(store) = self.position_store.as_mut() {
                store.store(&id, nx, ny);
            }
        }
        for id in &closed {
            self.close(id);
        }
        closed
    }

    /// 一帧渲染: 清底 (preview 铺极淡黑底, PORT: Java applyPreviewStyle) → render 闭包 →
    /// 与上帧逐字节比较 → 变化才 present (脏检查, Java repaint 抑制 / 零无谓提交)。
    /// render 闭包是任意第三方代码, present 是系统调用 — 在持槽位所有权下执行
    pub fn render_tick(&mut self) -> Result<(), String> {
        for i in 0..self.entries.len() {
            if self.entries[i].canvas.is_none() {
                continue; // materialize 保证 canvas 先于窗口存在, 此处防御性跳过
            }
            // ① 摘槽位
            let taken = self.entries[i].slot.take();
            let Some(mut sl) = taken else { continue };
            // ② 渲染 + 脏检查 + present (entry 的 canvas/render 字段与 slot 无关)
            let mut result = Ok(());
            {
                let entry = &mut self.entries[i];
                let canvas = entry.canvas.as_mut().unwrap();
                // 每帧从透明底重绘 (Java paintComponent 每帧全新后备缓冲; fill_rect 是
                // SrcOver 合成, 不清底会叠残影 → 脏检查指纹失真), preview 再铺极淡黑底
                let (cw, ch) = (canvas.width(), canvas.height());
                canvas.clear(cw, ch);
                if entry.preview {
                    canvas.fill_rect(0, 0, cw, ch, PREVIEW_BG);
                }
                (entry.render)(canvas);
                // 指纹 = 预乘 RGBA 逐字节 (比 window.rs 的字符串指纹更严: 任何像素变化都
                // 重绘)。先零拷贝比较 (整帧克隆 400x300 ≈ 480KB/tick, 未变化时白拷),
                // 命中变化才克隆存档
                let data = canvas.pixmap().data();
                if sl.last_frame.as_deref() != Some(data) {
                    sl.last_frame = Some(data.to_vec());
                    let buf = canvas.to_premul_bgra();
                    if let Err(e) = sl.window.present(&buf) {
                        result = Err(e);
                    }
                }
            }
            // ③ 放回槽位 (present 失败也放回 — 槽位状态不因渲染失败丢失,
            // 销毁与否由上层决定, 保真 Java 实例不因 paint 异常消失)
            self.entries[i].slot = Some(sl);
            result?;
        }
        Ok(())
    }

    /// 主循环: 单线程泵全部窗口消息 + 脏检查渲染 (见模块头线程模型说明)。
    /// 渲染节拍 50ms (Java FieldOverlay.onFlightData 50ms 节流), 事件泵 10ms (window.rs 同款)。
    /// 退出条件: stop 标志或全部 overlay 关闭 (生命周期归上层 Controller, POC 收敛为
    /// "无活跃窗口即退出"; Java 版由托盘/Controller.stop 决定)。
    pub fn run(&mut self) -> Result<(), String> {
        let mut last_render = Instant::now();
        while !self.stop.load(Ordering::Acquire) && self.active_count() > 0 {
            self.pump_events();
            if last_render.elapsed() >= Duration::from_millis(50) {
                last_render = Instant::now();
                self.render_tick()?;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    fn active_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.slot.is_some())
            .count()
    }
}

#[cfg(test)]
mod tests;
