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
//! 锁纪律 (LIFETIMES §3.3-1 根治裁决): Java OverlayEntry.close() 在 synchronized 锁内做
//! saveCurrentPosition (写配置) + Window.dispose() (级联回调 Bus 注销拿别的锁), 靠 Java
//! monitor 可重入侥幸不死锁; Rust Mutex 不可重入 — **锁内只摘/放槽位, 存位置与销毁链
//! 一律锁外执行** (拿走 ownership 后自然无竞态)。
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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::platform::{self, OverlayEvent, OverlayWindow, WindowConfig};
use crate::render2d::PixCanvas;

/// 内容渲染闭包: 每帧把 overlay 内容画进画布 (对应 Java overlay 子类的 paintComponent)
pub type RenderFn = Box<dyn FnMut(&mut PixCanvas)>;

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
}

/// 窗口槽位: Java OverlayEntry 的 instance/thread 字段 (Rust 无轮询线程, thread 无对应物;
/// LIFETIMES §4.2: 反射置 doit 停线程 → Rust 无线程可停, Drop 即完整销毁链)
struct OverlaySlot {
    window: Box<dyn OverlayWindow>,
    /// 拖拽状态机: 按下时 (root - win_pos) 偏移 (Java DraggableOverlay.dragStartX/Y)
    drag: Option<(i32, i32)>,
    /// 上帧像素指纹 (脏检查: 对应 Java repaint 抑制 / window.rs last_frame)
    last_frame: Option<Vec<u8>>,
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
    /// 窗口槽位 — PORT: Java entry 的 instance 字段 + synchronized(entry) monitor 合并;
    /// 只允许锁内摘/放槽位, 销毁链锁外 (见模块头锁纪律)
    slot: Mutex<Option<OverlaySlot>>,
    /// 复用画布 (Java overlay 后备缓冲; 首次 open 时创建)
    canvas: Option<PixCanvas>,
    /// 感兴趣的配置键前缀 (Java interestedPrefixes, 默认含自身 config_key)
    interested_prefixes: Vec<String>,
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

/// Java OverlayManager.isGlobalConfig: null 恒真; 全局键集合或前缀命中
fn is_global_config(key: Option<&str>) -> bool {
    let Some(k) = key else { return true };
    GLOBAL_CONFIG_KEYS.contains(&k) || GLOBAL_CONFIG_PREFIXES.iter().any(|p| k.starts_with(p))
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
    /// 位置存档 (归一化屏幕坐标; Java saveCurrentPosition → OverlaySettings 的 POC 内存版,
    /// 持久化归配置层接入后接管)
    saved_positions: HashMap<String, (f64, f64)>,
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
            slot: Mutex::new(None),
            canvas: None,
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
        // 锁内: 只查槽位
        if self.entries[idx].slot.lock().unwrap().is_some() {
            // Java: "Skipping open for {key}: already active"
            return Ok(false);
        }
        // 锁外: 建窗口 (工厂可能慢/失败, 不占锁)
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
    ///       → instance=null (全在 synchronized 锁内)
    /// Rust: 锁内只摘槽位, 存位置/销毁链锁外 (LIFETIMES §3.3-1 根治);
    ///       doit/interrupt 无对应物 (无轮询线程), drop(window) = dispose 注销链
    pub fn close(&mut self, id: &str) -> bool {
        let Some(idx) = self.entries.iter().position(|e| e.id == id) else {
            return false;
        };
        // ① 锁内: 摘槽位 (take 走 ownership; 槽位 None = Java instance=null)
        let taken = self.entries[idx].slot.lock().unwrap().take();
        let Some(slot) = taken else {
            return false; // 未开: Java close() 首行 instance==null 直接 return
        };
        // ② 锁外: 存位置 (Java saveCurrentPosition: 归一化屏幕坐标)
        let (wx, wy) = slot.window.position();
        let (sw, sh) = slot.window.screen_size();
        if sw > 0 && sh > 0 {
            self.saved_positions
                .insert(id.to_string(), (wx as f64 / sw as f64, wy as f64 / sh as f64));
        }
        // ③ 锁外: 销毁窗口 (drop = DestroyWindow; Java Window.dispose → 子类 dispose →
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
        let active = self.entries[idx].slot.lock().unwrap().is_some();
        if should_open {
            if !active {
                self.materialize(idx, true)?;
            } else {
                // reinitializer 等价: 清指纹强制重绘 (WYSIWYG)
                if let Some(slot) = self.entries[idx].slot.lock().unwrap().as_mut() {
                    slot.last_frame = None;
                }
            }
        } else if active {
            let id = self.entries[idx].id.clone();
            self.close(&id);
        }
        Ok(())
    }

    /// 重初始化全部已开实例 (Java reinitActiveOverlays; reinit = 标脏强制重绘)
    pub fn reinit_active_overlays(&mut self) {
        for entry in &mut self.entries {
            if let Some(slot) = entry.slot.lock().unwrap().as_mut() {
                slot.last_frame = None;
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
        // 初始位置: 会话存档 (归一化 → 物理像素, Java loadPosition) → 屏幕居中
        match self.saved_positions.get(&id).copied() {
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
        // Java registerOverlay: 有挂起对话框则暂缓置顶 (AlwaysOnTopCoordinator.java:68-73)
        // — 防对话框期间新建的 overlay 盖住对话框 (该协调器存在意义正是修这个时序 bug)
        if self.pending_dialogs > 0 {
            window.set_topmost(false);
        }
        self.entries[idx].preview = preview;
        // 锁内: 只放槽位 (并发已开则丢弃新建, 补偿锁外建窗的窗口期)。
        // 丢弃路径的窗口销毁在锁外执行: drop → DestroyWindow → WNDPROC → 拿
        // EVENT_QUEUES 锁, 若持槽位锁 drop 会形成 slot→EVENT_QUEUES 嵌套锁 (锁纪律)
        {
            let mut slot = self.entries[idx].slot.lock().unwrap();
            if slot.is_some() {
                drop(slot); // 先释放槽位锁
                drop(window); // 再销毁新建窗口 (锁外销毁链)
                return Ok(());
            }
            *slot = Some(OverlaySlot { window, drag: None, last_frame: None });
        }
        Ok(())
    }

    /// 是否激活 (Java isActive: entry != null && instance != null)
    pub fn is_active(&self, id: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.id == id && e.slot.lock().unwrap().is_some())
    }

    /// 全部活跃 id, 按注册序 (Java getActiveOverlays)
    pub fn active_ids(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.slot.lock().unwrap().is_some())
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
            // Java: pendingDialogs.compareAndSet(count, 0) 下溢复位
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
            return; // Java: "overlay 已处于隐藏状态，跳过"
        }
        self.overlays_hidden = true;
        self.for_each_active_window(|w| w.set_visible(false));
    }

    /// 显示所有被隐藏的 overlay 窗口（游戏重新获得焦点时恢复显示）。
    /// 亦为 FocusMonitor.setEnabled(false) 的恢复路径 (FocusMonitor.java:43-47)
    pub fn show_all_overlays(&mut self) {
        if !self.overlays_hidden {
            return; // Java: "overlay 已处于显示状态，跳过"
        }
        self.overlays_hidden = false;
        self.for_each_active_window(|w| w.set_visible(true));
    }

    /// 检查 overlay 是否因游戏失焦而被隐藏 (Java isOverlaysHidden)
    pub fn is_overlays_hidden(&self) -> bool {
        self.overlays_hidden
    }

    /// 遍历活跃窗口执行动作 — 锁内摘/放槽位, 窗口操作 (系统调用) 锁外 (模块头锁纪律)
    fn for_each_active_window(&mut self, mut f: impl FnMut(&mut dyn OverlayWindow)) {
        for i in 0..self.entries.len() {
            let taken = self.entries[i].slot.lock().unwrap().take();
            let Some(mut sl) = taken else { continue };
            f(sl.window.as_mut());
            self.entries[i].slot.lock().unwrap().replace(sl);
        }
    }

    /// 一轮消息泵: 逐窗口取事件 → 拖拽状态机; Close 事件走 close 销毁链 (锁外)。
    /// 返回本轮因 Close 事件被关闭的 id (Java 无对应返回, 测试/上层生命周期用)。
    /// 锁纪律: poll_event (→DispatchMessageW→WNDPROC 回调) 与 set_position/position/
    /// screen_size (系统调用) 均为外部代码, 不得持槽位锁执行 — 锁内只摘/放槽位,
    /// 事件处理整体锁外; 拖拽落点保存推迟到循环尾写入 (统一"外部副作用锁外"纪律)。
    pub fn pump_events(&mut self) -> Vec<String> {
        let mut closed: Vec<String> = Vec::new();
        let mut position_saves: Vec<(String, f64, f64)> = Vec::new();
        for i in 0..self.entries.len() {
            // ① 锁内: 摘槽位
            let taken = self.entries[i].slot.lock().unwrap().take();
            let Some(mut sl) = taken else { continue };
            let mut close_req = false;
            let mut save: Option<(f64, f64)> = None;
            // ② 锁外: 排空事件 + 拖拽状态机
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
            // ③ 锁内: 放回槽位
            self.entries[i].slot.lock().unwrap().replace(sl);
            if let Some((nx, ny)) = save {
                position_saves.push((self.entries[i].id.clone(), nx, ny));
            }
            if close_req {
                closed.push(self.entries[i].id.clone());
            }
        }
        for (id, nx, ny) in position_saves {
            self.saved_positions.insert(id, (nx, ny));
        }
        for id in &closed {
            self.close(id);
        }
        closed
    }

    /// 一帧渲染: 清底 (preview 铺极淡黑底, PORT: Java applyPreviewStyle) → render 闭包 →
    /// 与上帧逐字节比较 → 变化才 present (脏检查, Java repaint 抑制 / 零无谓提交)。
    /// 锁纪律: render 闭包是任意第三方代码 (一旦经捕获引用回环 host 即死锁, panic 则
    /// Mutex 毒化级联 panic), present 是系统调用 — 二者均不得持槽位锁执行, 锁内只摘/放
    pub fn render_tick(&mut self) -> Result<(), String> {
        for i in 0..self.entries.len() {
            if self.entries[i].canvas.is_none() {
                continue; // materialize 保证 canvas 先于窗口存在, 此处防御性跳过
            }
            // ① 锁内: 摘槽位
            let taken = self.entries[i].slot.lock().unwrap().take();
            let Some(mut sl) = taken else { continue };
            // ② 锁外: 渲染 + 脏检查 + present (entry 的 canvas/render 字段与 slot 无关)
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
            // ③ 锁内: 放回槽位 (present 失败也放回 — 槽位状态不因渲染失败丢失,
            // 销毁与否由上层决定, 保真 Java 实例不因 paint 异常消失)
            self.entries[i].slot.lock().unwrap().replace(sl);
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
            .filter(|e| e.slot.lock().unwrap().is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::rc::Rc;

    // ===== mock 窗口: 每窗口独立事件通道 (模拟 hwnd 分流) + 全局带标签调用日志 =====

    struct MockWindow {
        label: String,
        log: Rc<RefCell<Vec<String>>>,
        events: Rc<RefCell<VecDeque<OverlayEvent>>>,
        pos: (i32, i32),
        screen: (i32, i32),
        /// present 失败注入 (render_tick 失败路径测试)
        fail: Rc<Cell<bool>>,
    }

    impl OverlayWindow for MockWindow {
        fn present(&mut self, buf: &[u8]) -> Result<(), String> {
            if self.fail.get() {
                return Err("注入的 present 失败".into());
            }
            self.log.borrow_mut().push(format!("{}:present:{}", self.label, buf.len()));
            Ok(())
        }
        fn set_position(&mut self, x: i32, y: i32) {
            self.pos = (x, y);
            self.log.borrow_mut().push(format!("{}:set_position:{},{}", self.label, x, y));
        }
        fn position(&self) -> (i32, i32) {
            self.log.borrow_mut().push(format!("{}:position", self.label));
            self.pos
        }
        fn set_click_through(&mut self, _on: bool) {}
        fn set_topmost(&mut self, on: bool) {
            self.log.borrow_mut().push(format!("{}:set_topmost:{}", self.label, on));
        }
        fn set_visible(&mut self, visible: bool) {
            self.log.borrow_mut().push(format!("{}:set_visible:{}", self.label, visible));
        }
        fn poll_event(&mut self) -> Option<OverlayEvent> {
            self.events.borrow_mut().pop_front()
        }
        fn screen_size(&self) -> (i32, i32) {
            self.log.borrow_mut().push(format!("{}:screen_size", self.label));
            self.screen
        }
    }

    impl Drop for MockWindow {
        fn drop(&mut self) {
            self.log.borrow_mut().push(format!("{}:drop", self.label));
        }
    }

    /// mock 工厂句柄: 按创建序给窗口编号 (win0/win1/...), 各窗口独立事件通道
    struct MockHandle {
        log: Rc<RefCell<Vec<String>>>,
        channels: Rc<RefCell<Vec<Rc<RefCell<VecDeque<OverlayEvent>>>>>>,
        counter: Rc<Cell<usize>>,
        fail: Rc<Cell<bool>>,
    }

    impl MockHandle {
        fn new() -> Self {
            MockHandle {
                log: Rc::new(RefCell::new(Vec::new())),
                channels: Rc::new(RefCell::new(Vec::new())),
                counter: Rc::new(Cell::new(0)),
                fail: Rc::new(Cell::new(false)),
            }
        }

        fn factory(&self) -> WindowFactory {
            let log = Rc::clone(&self.log);
            let channels = Rc::clone(&self.channels);
            let counter = Rc::clone(&self.counter);
            let fail = Rc::clone(&self.fail);
            Box::new(move |cfg| {
                let n = counter.get();
                counter.set(n + 1);
                let events = Rc::new(RefCell::new(VecDeque::new()));
                channels.borrow_mut().push(Rc::clone(&events));
                log.borrow_mut()
                    .push(format!("win{}:create:click_through={}", n, cfg.click_through));
                Ok(Box::new(MockWindow {
                    label: format!("win{}", n),
                    log: Rc::clone(&log),
                    events,
                    pos: (60, 100), // WindowConfig 占位初值
                    screen: (1920, 1080),
                    fail: Rc::clone(&fail),
                }))
            })
        }

        /// 往第 n 个创建的窗口的事件通道塞事件 (模拟该 hwnd 的 WNDPROC 投递)
        fn push(&self, n: usize, ev: OverlayEvent) {
            self.channels.borrow()[n].borrow_mut().push_back(ev);
        }

        fn log(&self) -> Vec<String> {
            self.log.borrow().clone()
        }

        fn count(&self, pat: &str) -> usize {
            self.log().iter().filter(|l| l.contains(pat)).count()
        }
    }

    fn spec(id: &str, w: i32, h: i32, color: [u8; 4]) -> OverlaySpec {
        OverlaySpec {
            id: id.to_string(),
            config_key: id.to_string(),
            width: w,
            height: h,
            render: Box::new(move |c: &mut PixCanvas| {
                c.fill_rect(0, 0, 10, 10, color);
            }),
        }
    }

    // ===== 注册表语义 =====

    /// 同键重注册 = LinkedHashMap.put 原位替换, 保插入序; 注册不建实例
    #[test]
    fn register_replaces_same_id_in_place() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.register(spec("b", 40, 30, [0, 255, 0, 255]));
        host.register(spec("c", 40, 30, [0, 0, 255, 255]));
        host.register(spec("b", 60, 30, [255, 255, 0, 255])); // 重注册 b
        let ids: Vec<String> = host.entries.iter().map(|e| e.id.clone()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]); // 序不变
        assert_eq!(host.entries[1].width, 60); // 内容已替换
        assert!(host.active_ids().is_empty()); // 注册不建实例 (Java entries.put)
        assert!(mock.log().is_empty()); // 未触碰窗口工厂
    }

    /// with_interest 只作用于最后注册项 (Java withInterest)
    #[test]
    fn with_interest_targets_last_entry() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.register(spec("b", 40, 30, [0, 255, 0, 255]))
            .with_interest(&["shared"]);
        assert!(!host.entries[0].is_interested_in(Some("shared_x")));
        assert!(host.entries[1].is_interested_in(Some("shared_x")));
        assert!(host.entries[1].is_interested_in(Some("b"))); // 默认自身键
        assert!(host.entries[1].is_interested_in(None)); // Java: null 恒真
    }

    /// 重注册活跃 overlay: 先走 close 销毁链 (存位置 + drop) 再原位替换 —
    /// Java LinkedHashMap.put 替换会孤儿化旧实例 (窗口漏屏, Java 的 bug),
    /// Rust 所有权下必须显式收尾且不得丢位置存档
    #[test]
    fn reregister_active_overlay_runs_close_chain() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 300, 200, [255, 0, 0, 255]));
        host.open("a").unwrap(); // 居中于 (810, 440)
        mock.log.borrow_mut().clear();
        host.register(spec("a", 60, 30, [0, 255, 0, 255])); // 重注册 (窗口活跃)
        // 旧实例完整销毁链: position → screen_size (归一化) → drop
        assert_eq!(
            mock.log(),
            vec!["win0:position", "win0:screen_size", "win0:drop"]
        );
        assert!(host.saved_position("a").is_some()); // 位置已存档 (非静默 Drop)
        assert!(!host.is_active("a")); // 替换后回到未开状态
        assert_eq!(host.entries[0].width, 60); // 新条目内容生效
        // 位置存档跨替换保留: 重开 (win1) 从存档恢复而非居中
        assert!(host.open("a").unwrap());
        let (nx, ny) = host.saved_position("a").unwrap();
        // PORT: Java Math.round = floor(x+0.5) 复刻 (PORTING §2.3)
        let rx = (nx * 1920.0 + 0.5).floor() as i32;
        let ry = (ny * 1080.0 + 0.5).floor() as i32;
        assert!(mock.log().contains(&format!("win1:set_position:{},{}", rx, ry)));
    }

    // ===== open / open_all =====

    /// open_all 按激活探测过滤, live 模式穿透建窗
    #[test]
    fn open_all_respects_activation_probe() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.with_activation(Box::new(|key| key != "b"));
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.register(spec("b", 40, 30, [0, 255, 0, 255]));
        host.register(spec("c", 40, 30, [0, 0, 255, 255]));
        host.open_all().unwrap();
        assert_eq!(host.active_ids(), vec!["a", "c"]);
        // live 模式: click_through=true, 恰好两窗 (未激活的 b 不触碰工厂)
        assert_eq!(mock.count(":create:"), 2);
        assert_eq!(mock.count("create:click_through=true"), 2);
    }

    /// 已开重复 open 跳过 (Java "Skipping open: already active")
    #[test]
    fn open_skips_when_already_active() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        assert!(host.open("a").unwrap());
        assert!(!host.open("a").unwrap()); // 第二次跳过
        assert_eq!(mock.count(":create:"), 1);
        assert!(host.open("nope").is_err()); // 未注册的 id 报错
    }

    // ===== close 销毁序 (LIFETIMES §3.3-1: 锁内摘槽, 锁外销毁链) =====

    /// close 顺序: 读位置 → 存归一化位置 → 销毁窗口; 槽位清空
    #[test]
    fn close_saves_position_then_destroys() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 300, 200, [255, 0, 0, 255]));
        host.open("a").unwrap();
        mock.log.borrow_mut().clear();
        assert!(host.close("a"));
        // 销毁链恰好三步 (无 present/set_position 夹在中间):
        // ① position ② screen_size (归一化用) ③ drop (= Java dispose 注销链)
        assert_eq!(
            mock.log(),
            vec!["win0:position", "win0:screen_size", "win0:drop"]
        );
        // 居中位置 (1920-300)/2, (1080-200)/2 的归一化存档
        let (nx, ny) = host.saved_position("a").unwrap();
        assert!((nx - 810.0 / 1920.0).abs() < 1e-9);
        assert!((ny - 440.0 / 1080.0).abs() < 1e-9);
        assert!(!host.is_active("a"));
    }

    /// 未开/不存在的 close 为无操作
    #[test]
    fn close_inactive_is_noop() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        assert!(!host.close("a")); // 未开
        assert!(!host.close("nope")); // 未注册
        assert_eq!(mock.count(":drop"), 0);
    }

    /// close_all 按注册序逐窗口走完整销毁链
    #[test]
    fn close_all_destroys_in_registration_order() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        for (id, color) in
            [("a", [255u8, 0, 0, 255]), ("b", [0, 255, 0, 255]), ("c", [0, 0, 255, 255])]
        {
            host.register(spec(id, 40, 30, color));
        }
        host.open_all().unwrap();
        mock.log.borrow_mut().clear();
        host.close_all();
        assert!(host.active_ids().is_empty());
        // 每窗口三步销毁链, 且窗口顺序 = 注册序 win0(a)→win1(b)→win2(c)
        assert_eq!(
            mock.log(),
            vec![
                "win0:position", "win0:screen_size", "win0:drop",
                "win1:position", "win1:screen_size", "win1:drop",
                "win2:position", "win2:screen_size", "win2:drop",
            ]
        );
    }

    // ===== preview 生命周期 =====

    /// refresh_preview: 应开建 preview 窗 (非穿透), 策略失活关闭, 再激活重建
    #[test]
    fn refresh_preview_lifecycle() {
        let mock = MockHandle::new();
        let enabled = Rc::new(Cell::new(true));
        let probe = {
            let enabled = Rc::clone(&enabled);
            move |_: &str| enabled.get()
        };
        let mut host = OverlayHost::with_factory(mock.factory());
        host.with_activation(Box::new(probe));
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.refresh_preview().unwrap(); // 建
        assert!(host.is_active("a"));
        assert_eq!(mock.log()[0], "win0:create:click_through=false"); // preview 可拖拽
        assert!(host.entries[0].preview);
        host.refresh_preview().unwrap(); // reinit: 不重建, 只标脏
        assert_eq!(mock.count(":create:"), 1);
        enabled.set(false);
        host.refresh_preview().unwrap(); // 策略失活 → close (Java "inactive strategy")
        assert!(!host.is_active("a"));
        assert_eq!(mock.count(":drop"), 1);
        enabled.set(true);
        host.refresh_preview().unwrap(); // 再建
        assert!(host.is_active("a"));
        assert_eq!(mock.count(":create:"), 2);
    }

    /// reinit 标脏: 同内容下 present 恰好发生在指纹失效后
    #[test]
    fn refresh_preview_reinit_forces_repaint() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.refresh_preview().unwrap();
        host.render_tick().unwrap(); // 首帧 present
        host.render_tick().unwrap(); // 静态内容: 脏检查抑制
        assert_eq!(mock.count(":present:"), 1);
        host.refresh_preview().unwrap(); // reinit 清指纹
        host.render_tick().unwrap();
        assert_eq!(mock.count(":present:"), 2);
    }

    /// refresh_preview_key: 自身键/兴趣前缀/全局键过滤 (Java refreshPreviews(changedKey))
    #[test]
    fn refresh_preview_key_filtering() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.register(spec("b", 40, 30, [0, 255, 0, 255]))
            .with_interest(&["shared"]);
        host.register(spec("c", 40, 30, [0, 0, 255, 255]));
        // 只动 a: 仅 a 建
        host.refresh_preview_key(Some("a")).unwrap();
        assert_eq!(host.active_ids(), vec!["a"]);
        // shared_x 只命中 b 的兴趣前缀
        host.refresh_preview_key(Some("shared_font")).unwrap();
        assert_eq!(host.active_ids(), vec!["a", "b"]);
        // 全局键 (Java GLOBAL_CONFIG_KEYS/PREFIXES): 全部刷新
        host.refresh_preview_key(Some("fontName")).unwrap();
        assert_eq!(host.active_ids(), vec!["a", "b", "c"]);
        // 前缀匹配是 Java 有意语义 (isInterestedIn startsWith, 派生键如
        // engineInfoSwitch 命中 engineInfo): "bx" 命中 b 的默认前缀 "b", 不命中 a/c
        host.close_all();
        host.refresh_preview_key(Some("bx")).unwrap();
        assert_eq!(host.active_ids(), vec!["b"]);
        // 全局键集合成员原样生效
        host.close_all();
        host.refresh_preview_key(Some("AAEnable")).unwrap();
        assert_eq!(host.active_ids(), vec!["a", "b", "c"]);
        // changed_key=None = 全量刷新 (Java key==null 恒真)
        host.close_all();
        host.refresh_preview_key(None).unwrap();
        assert_eq!(host.active_ids(), vec!["a", "b", "c"]);
    }

    // ===== 拖拽 + 事件分流 (模拟) =====

    /// preview 拖拽: press 记偏移 → move 挪窗 → release 存归一化位置
    #[test]
    fn drag_moves_window_and_saves_position() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 300, 200, [255, 0, 0, 255]));
        host.refresh_preview().unwrap();
        // 窗口创建后居中于 (810, 440)
        mock.push(0, OverlayEvent::MousePress { root_x: 860, root_y: 480 });
        mock.push(0, OverlayEvent::MouseMove { root_x: 900, root_y: 520, left_down: true });
        mock.push(0, OverlayEvent::MouseRelease);
        host.pump_events();
        assert!(mock.log().contains(&"win0:set_position:850,480".to_string())); // 900-50, 520-40
        let (nx, ny) = host.saved_position("a").unwrap();
        assert!((nx - 850.0 / 1920.0).abs() < 1e-9);
        assert!((ny - 480.0 / 1080.0).abs() < 1e-9);
        assert!(host.is_active("a")); // 拖拽不销毁
    }

    /// Close 事件 → 走 close 销毁链 (存位置 + drop), pump 返回被关闭 id
    #[test]
    fn close_event_triggers_close_chain() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.refresh_preview().unwrap();
        mock.push(0, OverlayEvent::Close);
        let closed = host.pump_events();
        assert_eq!(closed, vec!["a".to_string()]);
        assert!(!host.is_active("a"));
        assert_eq!(mock.count(":drop"), 1);
        assert!(host.saved_position("a").is_some()); // 销毁前位置已存
    }

    /// 多窗口事件分流: 各窗口只消费自己通道的事件, 互不串扰
    #[test]
    fn multi_window_events_routed_independently() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 300, 200, [255, 0, 0, 255]));
        host.register(spec("b", 300, 200, [0, 255, 0, 255]));
        host.refresh_preview().unwrap(); // win0=a, win1=b, 均居中 (810, 440)
        // 各自通道互不串扰: a 拖到 (900,520) 偏移处, b 收到 Close
        mock.push(0, OverlayEvent::MousePress { root_x: 860, root_y: 480 });
        mock.push(0, OverlayEvent::MouseMove { root_x: 900, root_y: 520, left_down: true });
        mock.push(0, OverlayEvent::MouseRelease);
        mock.push(1, OverlayEvent::Close);
        let closed = host.pump_events();
        assert_eq!(closed, vec!["b".to_string()]);
        // a 只被拖动未销毁, 位置存档只属于 a; b 走销毁链
        assert!(host.is_active("a"));
        assert!(!host.is_active("b"));
        assert!(mock.log().contains(&"win0:set_position:850,480".to_string()));
        assert!(!mock.log().contains(&"win1:set_position:850,480".to_string()));
        assert_eq!(mock.count(":drop"), 1);
        assert!(host.saved_position("a").is_some());
        assert!(host.saved_position("b").is_some()); // b 的居中位置在销毁前存档
    }

    /// 脏检查渲染: 内容变化才 present, 多窗口各自独立指纹
    #[test]
    fn render_tick_dirty_check_per_window() {
        let mock = MockHandle::new();
        let tick = Rc::new(Cell::new(0u32));
        let tick_b = Rc::clone(&tick);
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        // b 的内容随 tick 变化 (模拟 live 数据驱动)
        host.register(OverlaySpec {
            id: "b".into(),
            config_key: "b".into(),
            width: 40,
            height: 30,
            render: Box::new(move |c: &mut PixCanvas| {
                c.fill_rect(0, 0, 10, 10, [tick_b.get() as u8, 0, 0, 255]);
            }),
        });
        host.refresh_preview().unwrap();
        host.render_tick().unwrap(); // 双首帧: a + b 各 present 一次
        host.render_tick().unwrap(); // b 的 render 副作用 tick 未变 → 双双抑制
        assert_eq!(mock.count(":present:"), 2);
        tick.set(5);
        host.render_tick().unwrap(); // 只有 b 变化
        assert_eq!(mock.count("win0:present:"), 1);
        assert_eq!(mock.count("win1:present:"), 2);
        // present 的缓冲尺寸 = w*h*4 (预乘 BGRA)
        assert!(mock.log().iter().all(|l| !l.contains("present") || l.ends_with(":4800")));
    }

    /// render_tick present 失败: 槽位必须放回 (实例不因渲染失败丢失, 销毁归上层决定),
    /// 恢复后同帧不再重复 present (last_frame 已在失败前存档)
    #[test]
    fn render_tick_present_failure_keeps_slot() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        let tick = Rc::new(Cell::new(0u8));
        let t = Rc::clone(&tick);
        host.register(OverlaySpec {
            id: "a".into(),
            config_key: "a".into(),
            width: 40,
            height: 30,
            render: Box::new(move |c: &mut PixCanvas| {
                c.fill_rect(0, 0, 10, 10, [t.get(), 0, 0, 255]);
            }),
        });
        host.open("a").unwrap();
        host.render_tick().unwrap(); // 首帧 present 成功
        tick.set(1); // 内容变化 → 触发 present
        mock.fail.set(true); // 注入失败
        assert!(host.render_tick().is_err());
        assert!(host.is_active("a")); // 槽位已放回, 实例存活
        // 失败帧恢复: 同内容不再重试 (指纹已存), 新内容正常提交
        mock.fail.set(false);
        host.render_tick().unwrap();
        assert_eq!(mock.count(":present:"), 1); // 仅首帧
        tick.set(2);
        host.render_tick().unwrap();
        assert_eq!(mock.count(":present:"), 2);
    }

    // ===== dialog 协调 (AlwaysOnTopCoordinator 合并语义) =====

    struct CountingHooks {
        suspends: Rc<Cell<u32>>,
        restores: Rc<Cell<u32>>,
    }

    impl DialogHooks for CountingHooks {
        fn suspend_overlays(&mut self) {
            self.suspends.set(self.suspends.get() + 1);
        }
        fn restore_overlays(&mut self) {
            self.restores.set(self.restores.get() + 1);
        }
    }

    /// 计数 + 钩子触发语义: 每次 will_show 挂起; 归零 (含下溢复位) 恢复
    /// (Java pendingDialogs.compareAndSet(count, 0) + suspendAll/restoreAll)
    #[test]
    fn dialog_counting_hooks_and_underflow_reset() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        let suspends = Rc::new(Cell::new(0u32));
        let restores = Rc::new(Cell::new(0u32));
        host.with_dialog_hooks(Box::new(CountingHooks {
            suspends: Rc::clone(&suspends),
            restores: Rc::clone(&restores),
        }));
        host.dialog_will_show();
        host.dialog_will_show();
        assert_eq!(host.pending_dialog_count(), 2);
        assert_eq!(suspends.get(), 2);
        assert_eq!(restores.get(), 0);
        host.dialog_did_dismiss();
        assert_eq!(host.pending_dialog_count(), 1); // 未归零不恢复
        assert_eq!(restores.get(), 0);
        host.dialog_did_dismiss();
        assert_eq!(host.pending_dialog_count(), 0); // 归零恢复
        assert_eq!(restores.get(), 1);
        host.dialog_did_dismiss(); // 下溢: 复位 0 且再次恢复 (Java 行为)
        assert_eq!(host.pending_dialog_count(), 0);
        assert_eq!(restores.get(), 2);
    }

    /// 默认空钩子 (POC: 全窗口恒 TOPMOST, 不受 dialog 计数影响) 不 panic
    #[test]
    fn dialog_noop_hooks_default() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.dialog_will_show();
        assert_eq!(host.pending_dialog_count(), 1);
        host.dialog_did_dismiss();
        assert_eq!(host.pending_dialog_count(), 0);
    }

    /// Java registerOverlay (AlwaysOnTopCoordinator.java:59-74): 有挂起对话框时
    /// 新建 overlay 暂缓置顶 — 防对话框期间建的 overlay 盖住对话框
    #[test]
    fn materialize_defers_topmost_when_dialog_pending() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.dialog_will_show(); // pendingDialogs=1 (钩子为空实现, 只看置顶动作)
        host.open("a").unwrap();
        assert!(mock.log().contains(&"win0:set_topmost:false".to_string()));
        // 归零后新建的窗口恢复恒置顶 (不再有降级调用)
        host.dialog_did_dismiss();
        host.close("a");
        host.register(spec("b", 40, 30, [0, 255, 0, 255]));
        host.open("b").unwrap();
        assert!(!mock.log().contains(&"win1:set_topmost:false".to_string()));
    }

    // ===== 游戏失焦隐藏/显示 (FocusMonitor 面的窗口动作, Java L197-233) =====

    /// hide/show_all_overlays: overlaysHidden 幂等标志 (重复调用跳过) + 不销毁实例;
    /// FocusMonitor.setEnabled(false) 的恢复路径 = show_all_overlays
    #[test]
    fn hide_show_all_overlays_idempotent_flag() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.register(spec("b", 40, 30, [0, 255, 0, 255]));
        host.open_all().unwrap();
        mock.log.borrow_mut().clear();
        assert!(!host.is_overlays_hidden());
        // 游戏失焦: 全部活跃窗口隐藏 (Java "hideAllOverlays 调用")
        host.hide_all_overlays();
        assert!(host.is_overlays_hidden());
        assert_eq!(mock.count(":set_visible:false"), 2);
        // 不销毁实例 (Java: "隐藏所有已注册的overlay窗口（不销毁实例）")
        assert!(host.is_active("a") && host.is_active("b"));
        mock.log.borrow_mut().clear();
        host.hide_all_overlays(); // 幂等: 已隐藏跳过
        assert!(mock.log().is_empty());
        // 游戏获焦: 恢复显示
        host.show_all_overlays();
        assert!(!host.is_overlays_hidden());
        assert_eq!(mock.count(":set_visible:true"), 2);
        mock.log.borrow_mut().clear();
        host.show_all_overlays(); // 幂等: 已显示跳过
        assert!(mock.log().is_empty());
        // 未开窗口不在遍历范围 (只动活跃槽位, 等价 Java Weak 注册表 + isDisplayable 守卫)
        host.register(spec("c", 40, 30, [0, 0, 255, 255])); // 注册未开
        mock.log.borrow_mut().clear();
        host.hide_all_overlays();
        host.show_all_overlays();
        assert_eq!(mock.count(":set_visible:"), 4); // 仅 a/b 各隐藏+显示, c 不触碰
        assert!(!mock.log().iter().any(|l| l.starts_with("win2:")));
    }

    // ===== run 主循环 =====

    /// run: 全部窗口关闭后退出 (Close → close → active 0 → 循环条件失效)
    #[test]
    fn run_exits_when_all_windows_closed() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.refresh_preview().unwrap();
        mock.push(0, OverlayEvent::Close);
        host.run().unwrap(); // 首轮 pump 即关 a, active=0 退出
        assert!(!host.is_active("a"));
    }

    /// request_stop 停机标志生效 (Java doit=false 停机语义)
    #[test]
    fn run_respects_stop_flag() {
        let mock = MockHandle::new();
        let mut host = OverlayHost::with_factory(mock.factory());
        host.register(spec("a", 40, 30, [255, 0, 0, 255]));
        host.refresh_preview().unwrap();
        host.request_stop();
        host.run().unwrap(); // 立即退出, 窗口保持打开 (后续由上层决定)
        assert!(host.is_active("a"));
    }

    /// stop_handle 跨线程停机句柄 (host 本体 !Send; Java doit 从别的线程置 false 的
    /// 语义 — Controller/Service 线程持句柄请求退出, 主循环消费)
    #[test]
    fn stop_handle_signals_stop_from_outside() {
        let mock = MockHandle::new();
        let host = OverlayHost::with_factory(mock.factory());
        let handle = host.stop_handle();
        assert!(!host.is_stop_requested());
        handle.store(true, Ordering::Release); // 模拟其他线程请求停机
        assert!(host.is_stop_requested());
        host.request_stop(); // 本体直调同效
        assert!(host.is_stop_requested());
    }
}

