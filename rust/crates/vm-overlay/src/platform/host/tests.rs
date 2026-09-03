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
        self.log
            .borrow_mut()
            .push(format!("{}:present:{}", self.label, buf.len()));
        Ok(())
    }
    fn set_position(&mut self, x: i32, y: i32) {
        self.pos = (x, y);
        self.log
            .borrow_mut()
            .push(format!("{}:set_position:{},{}", self.label, x, y));
    }
    fn position(&self) -> (i32, i32) {
        self.log
            .borrow_mut()
            .push(format!("{}:position", self.label));
        self.pos
    }
    fn set_click_through(&mut self, _on: bool) {}
    fn set_topmost(&mut self, on: bool) {
        self.log
            .borrow_mut()
            .push(format!("{}:set_topmost:{}", self.label, on));
    }
    fn set_visible(&mut self, visible: bool) {
        self.log
            .borrow_mut()
            .push(format!("{}:set_visible:{}", self.label, visible));
    }
    fn set_size(&mut self, w: i32, h: i32) {
        self.log
            .borrow_mut()
            .push(format!("{}:set_size:{},{}", self.label, w, h));
    }
    fn poll_event(&mut self) -> Option<OverlayEvent> {
        self.events.borrow_mut().pop_front()
    }
    fn screen_size(&self) -> (i32, i32) {
        self.log
            .borrow_mut()
            .push(format!("{}:screen_size", self.label));
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
    // 测试通道嵌套 (共享日志 + 每窗口事件队列), 类型别名反而绕
    #[allow(clippy::type_complexity)]
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
            log.borrow_mut().push(format!(
                "win{}:create:click_through={}",
                n, cfg.click_through
            ));
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
        reinit: None,
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
    assert!(host.entries[1].is_interested_in(None));
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
    // Java Math.round = floor(x+0.5) 复刻
    let rx = (nx * 1920.0 + 0.5).floor() as i32;
    let ry = (ny * 1080.0 + 0.5).floor() as i32;
    assert!(mock
        .log()
        .contains(&format!("win1:set_position:{},{}", rx, ry)));
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

// ===== close 销毁序 =====

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
    for (id, color) in [
        ("a", [255u8, 0, 0, 255]),
        ("b", [0, 255, 0, 255]),
        ("c", [0, 0, 255, 255]),
    ] {
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
            "win0:position",
            "win0:screen_size",
            "win0:drop",
            "win1:position",
            "win1:screen_size",
            "win1:drop",
            "win2:position",
            "win2:screen_size",
            "win2:drop",
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

/// 僵尸实例 (Java run 自动退场后 instance 僵留): open 跳过 / refreshPreview 只跑
/// reinitializer 不重建 / 策略失活的 close 清僵尸后再激活可重建 (DrawFrameSimpl
/// 收腿退场是生产置位点, 本测试锁 host 通用语义)
#[test]
fn zombie_entry_blocks_rematerialize_until_cleared() {
    let mock = MockHandle::new();
    let enabled = Rc::new(Cell::new(true));
    let probe = {
        let enabled = Rc::clone(&enabled);
        move |_: &str| enabled.get()
    };
    let mut host = OverlayHost::with_factory(mock.factory());
    host.with_activation(Box::new(probe));
    host.register(spec("a", 40, 30, [255, 0, 0, 255]));
    host.open_all().unwrap();
    assert!(host.is_active("a"));
    // run 退场序 (draw_frame_simpl pump): close 销毁窗口 → 置僵尸
    host.close("a");
    host.set_entry_zombie("a", true);
    assert!(!host.is_active("a"));
    let creates_before = mock.count(":create:");
    // openAll 跳过 (Java :294-299 "already active")
    host.open_all().unwrap();
    assert!(!host.is_active("a"));
    // refreshPreview 只跑 reinitializer, 不建窗 (Java :332-336)
    host.refresh_preview().unwrap();
    assert!(!host.is_active("a"));
    assert_eq!(
        mock.count(":create:"),
        creates_before,
        "僵尸期零 materialize"
    );
    // 策略失活: instance != null 即 close → 僵尸清除 (Java :337-340 + :370)
    enabled.set(false);
    host.refresh_preview().unwrap();
    // 策略再开: 无僵尸 → 重建 preview 实例
    enabled.set(true);
    host.refresh_preview().unwrap();
    assert!(host.is_active("a"), "close 清僵尸后再激活重建");
    assert_eq!(mock.count(":create:"), creates_before + 1);
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

// ===== WYSIWYG reinit + resize (Java reinitConfig → setBounds 的 host 面) =====

/// resize_entry: entry 宽高/画布尺寸更新 + 活跃窗口收到 set_size + 新画布
/// present 缓冲长度 = 新 w*h*4
#[test]
fn resize_entry_updates_canvas_and_window() {
    let mock = MockHandle::new();
    let mut host = OverlayHost::with_factory(mock.factory());
    host.register(spec("a", 40, 30, [255, 0, 0, 255]));
    host.refresh_preview().unwrap();
    host.render_tick().unwrap();
    mock.log.borrow_mut().clear();
    host.resize_entry("a", 60, 50).unwrap();
    assert_eq!(host.entries[0].width, 60);
    assert_eq!(host.entries[0].height, 50);
    let cv = host.entries[0].canvas.as_ref().unwrap();
    assert_eq!((cv.width(), cv.height()), (60, 50));
    // 窗口收到 set_size (Java setBounds 的底层动作)
    assert!(mock.log().contains(&"win0:set_size:60,50".to_string()));
    // 下一帧 present 缓冲 = 60*50*4 (画布与窗口尺寸同步)
    host.render_tick().unwrap();
    assert!(mock.log().iter().any(|l| l.contains("win0:present:12000")));
    // 未注册 id 报错
    assert!(host.resize_entry("nope", 10, 10).is_err());
}

// ===== per-entry 可见性 (Java Window.setVisible 的单条目面 — P5 组装契约 (b)) =====

/// set_entry_visible: 只动目标条目窗口 + 幂等守卫 (重复同值零调用, Issue #54
/// DWM 全量合成防抖 = Java isVisible() 守卫); 未注册/未开条目静默无操作
#[test]
fn set_entry_visible_targets_entry_and_is_idempotent() {
    let mock = MockHandle::new();
    let mut host = OverlayHost::with_factory(mock.factory());
    host.register(spec("a", 40, 30, [255, 0, 0, 255]));
    host.register(spec("b", 40, 30, [0, 255, 0, 255]));
    host.open_all().unwrap();
    mock.log.borrow_mut().clear();
    // 隐藏 a: 仅 win0 收 set_visible(false), b 不受影响
    host.set_entry_visible("a", false);
    assert_eq!(mock.log(), vec!["win0:set_visible:false"]);
    // 幂等: 同值再调零系统调用
    host.set_entry_visible("a", false);
    assert_eq!(mock.log(), vec!["win0:set_visible:false"]);
    // 拉起 a: 记录翻转后再次生效
    host.set_entry_visible("a", true);
    assert_eq!(
        mock.log(),
        vec!["win0:set_visible:false", "win0:set_visible:true"]
    );
    // 未注册 id / 未开条目: 静默无操作 (Java instance==null 无窗口可设)
    host.set_entry_visible("nope", false);
    host.close("b");
    host.set_entry_visible("b", true);
    assert_eq!(mock.count(":set_visible:"), 2);
}

/// 全局 hide/show_all 与 per-entry 记录互通: hide_all 后同值 per-entry 调用
/// 幂等跳过, 反向值仍生效 (Java FocusMonitor 隐藏 vs BaseOverlay.run 拉起的
/// "互不感知" 形态 — 保真不打架为闭环, 行为与 Swing isVisible 同源)
#[test]
fn global_hide_show_syncs_slot_visibility_record() {
    let mock = MockHandle::new();
    let mut host = OverlayHost::with_factory(mock.factory());
    host.register(spec("a", 40, 30, [255, 0, 0, 255]));
    host.open("a").unwrap();
    host.hide_all_overlays(); // 窗口隐藏 + 槽位记录同步 false
    mock.log.borrow_mut().clear();
    host.set_entry_visible("a", false); // 已隐藏: 幂等跳过
    assert!(mock.log().is_empty());
    host.set_entry_visible("a", true); // 反向: 生效 (对位 run() 的按需拉起)
    assert_eq!(mock.log(), vec!["win0:set_visible:true"]);
    // show_all 恢复记录: 此后 hide 幂等路径重新闭合
    host.show_all_overlays();
    mock.log.borrow_mut().clear();
    host.set_entry_visible("a", true);
    assert!(mock.log().is_empty(), "show_all 后同值幂等");
}

/// reinit 闭包返回新尺寸: refresh_preview 不重建窗口, 原位 resize (Java
/// reinitializer + setBounds); 无闭包条目仅清指纹
#[test]
fn refresh_preview_runs_reinit_closure_and_resizes() {
    let mock = MockHandle::new();
    let calls = Rc::new(Cell::new(0));
    // 首轮 (冷激活) 尺寸不变, 次轮 (模拟配置变更后) 返回新尺寸
    let grown = Rc::new(Cell::new(false));
    let (calls_c, grown_c) = (Rc::clone(&calls), Rc::clone(&grown));
    let mut host = OverlayHost::with_factory(mock.factory());
    host.register(OverlaySpec {
        id: "a".into(),
        config_key: "a".into(),
        width: 40,
        height: 30,
        render: Box::new(|_c: &mut PixCanvas| {}),
        reinit: Some(Box::new(move || {
            calls_c.set(calls_c.get() + 1);
            if grown_c.get() {
                Some((80, 60))
            } else {
                None
            }
        })),
    });
    host.register(spec("b", 40, 30, [0, 255, 0, 255])); // 无 reinit 对照
    host.refresh_preview().unwrap();
    assert_eq!(calls.get(), 1, "冷激活也跑 reinit (尺寸未变分支)");
    assert_eq!(mock.count(":create:"), 2);
    mock.log.borrow_mut().clear();
    grown.set(true); // 模拟 fontadd 配置变更 → 新 preferred_size
    host.refresh_preview().unwrap(); // 已开: 原位 reinit + resize
    assert_eq!(calls.get(), 2);
    assert_eq!(
        mock.count(":create:"),
        0,
        "已开实例不重建窗口 (Java 实例保留)"
    );
    assert!(mock.log().contains(&"win0:set_size:80,60".to_string()));
    // 无 reinit 闭包的 b: 无 set_size 调用
    assert!(!mock.log().iter().any(|l| l.starts_with("win1:set_size")));
}

/// reinit_active_overlays: 活跃条目跑 reinit 闭包, 未开条目不动
#[test]
fn reinit_active_overlays_runs_closures_for_active_only() {
    let mock = MockHandle::new();
    let calls = Rc::new(Cell::new(0));
    let calls_c = Rc::clone(&calls);
    let mut host = OverlayHost::with_factory(mock.factory());
    host.register(OverlaySpec {
        id: "a".into(),
        config_key: "a".into(),
        width: 40,
        height: 30,
        render: Box::new(|_c: &mut PixCanvas| {}),
        reinit: Some(Box::new(move || {
            calls_c.set(calls_c.get() + 1);
            None // 尺寸不变分支: 只清指纹
        })),
    });
    host.register(spec("b", 40, 30, [0, 255, 0, 255]));
    host.reinit_active_overlays();
    assert_eq!(
        calls.get(),
        0,
        "全未开: 不跑闭包 (Java reinitActiveOverlays 只动在场实例)"
    );
    assert!(host.open("a").unwrap());
    host.reinit_active_overlays();
    assert_eq!(calls.get(), 1, "活跃条目跑闭包");
    assert!(
        !mock.log().iter().any(|l| l.contains("set_size")),
        "None = 不动尺寸"
    );
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
    mock.push(
        0,
        OverlayEvent::MousePress {
            root_x: 860,
            root_y: 480,
        },
    );
    mock.push(
        0,
        OverlayEvent::MouseMove {
            root_x: 900,
            root_y: 520,
            left_down: true,
        },
    );
    mock.push(0, OverlayEvent::MouseRelease);
    host.pump_events();
    assert!(mock
        .log()
        .contains(&"win0:set_position:850,480".to_string())); // 900-50, 520-40
    let (nx, ny) = host.saved_position("a").unwrap();
    assert!((nx - 850.0 / 1920.0).abs() < 1e-9);
    assert!((ny - 480.0 / 1080.0).abs() < 1e-9);
    assert!(host.is_active("a")); // 拖拽不销毁
}

/// 条目固定初始位置 (P6: Java DrawFrameSimpl 每次 init/initPreview 的
/// setBounds(0, screenH-500, 900, 500) 字面量 — 存档键 thrustdFSX/Y 只写不读):
/// materialize 优先于居中 (不查 screen_size); 拖拽/销毁存档照常写入但
/// re-materialize 恒回固定几何
#[test]
fn fixed_pos_overrides_saved_and_center_on_materialize() {
    let mock = MockHandle::new();
    let mut host = OverlayHost::with_factory(mock.factory());
    host.register(spec("thrustdFS", 900, 500, [255, 0, 0, 255]));
    assert!(host.set_entry_fixed_pos("thrustdFS", 0, 580));
    assert!(
        !host.set_entry_fixed_pos("missing", 0, 0),
        "未注册条目报 false"
    );
    host.open("thrustdFS").unwrap();
    assert!(
        mock.log().contains(&"win0:set_position:0,580".to_string()),
        "固定几何优先"
    );
    assert!(
        !mock.log().iter().any(|l| l.contains("screen_size")),
        "不走居中/归一化换算路径"
    );
    // preview 形态拖拽存档 (Java initPreview setupDragListeners — 游戏模式不可拖)
    host.close("thrustdFS");
    host.refresh_preview().unwrap(); // win1 回固定几何
    mock.push(
        1,
        OverlayEvent::MousePress {
            root_x: 500,
            root_y: 700,
        },
    );
    mock.push(
        1,
        OverlayEvent::MouseMove {
            root_x: 900,
            root_y: 900,
            left_down: true,
        },
    );
    mock.push(1, OverlayEvent::MouseRelease);
    host.pump_events();
    assert!(
        host.saved_position("thrustdFS").is_some(),
        "拖拽存档照常写入"
    );
    // 销毁重开: 恒回固定几何 (Java 工厂每实例 setBounds 的等价面)
    host.close("thrustdFS");
    host.refresh_preview().unwrap(); // win2
    let pos_logs: Vec<String> = mock
        .log()
        .iter()
        .filter(|l| l.contains("set_position"))
        .cloned()
        .collect();
    assert_eq!(
        pos_logs.last().unwrap(),
        "win2:set_position:0,580",
        "re-materialize 回固定几何 (实测 {pos_logs:?})"
    );
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
    mock.push(
        0,
        OverlayEvent::MousePress {
            root_x: 860,
            root_y: 480,
        },
    );
    mock.push(
        0,
        OverlayEvent::MouseMove {
            root_x: 900,
            root_y: 520,
            left_down: true,
        },
    );
    mock.push(0, OverlayEvent::MouseRelease);
    mock.push(1, OverlayEvent::Close);
    let closed = host.pump_events();
    assert_eq!(closed, vec!["b".to_string()]);
    // a 只被拖动未销毁, 位置存档只属于 a; b 走销毁链
    assert!(host.is_active("a"));
    assert!(!host.is_active("b"));
    assert!(mock
        .log()
        .contains(&"win0:set_position:850,480".to_string()));
    assert!(!mock
        .log()
        .contains(&"win1:set_position:850,480".to_string()));
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
        reinit: None,
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
    assert!(mock
        .log()
        .iter()
        .all(|l| !l.contains("present") || l.ends_with(":4800")));
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
        reinit: None,
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

// ---- PositionStore 后端 (配置位置桥) ----

/// 记录型后端句柄 (Rc 共享, host 持实现、测试持句柄读记录)
#[derive(Clone, Default)]
struct StoreHandle {
    loads: Rc<RefCell<Vec<String>>>,
    saves: Rc<RefCell<Vec<(String, f64, f64)>>>,
}

struct RecordingStore {
    h: StoreHandle,
    map: HashMap<String, (f64, f64)>,
}

impl PositionStore for RecordingStore {
    fn load(&mut self, id: &str) -> Option<(f64, f64)> {
        self.h.loads.borrow_mut().push(id.to_string());
        self.map.get(id).copied()
    }
    fn store(&mut self, id: &str, x: f64, y: f64) {
        self.h.saves.borrow_mut().push((id.to_string(), x, y));
        self.map.insert(id.to_string(), (x, y));
    }
}

fn store_with(map: &[(&str, f64, f64)]) -> (Box<RecordingStore>, StoreHandle) {
    let h = StoreHandle::default();
    let mut s = RecordingStore {
        h: h.clone(),
        map: HashMap::new(),
    };
    for (id, x, y) in map {
        s.map.insert(id.to_string(), (*x, *y));
    }
    (Box::new(s), h)
}

/// 后端位置作为初始位置 (Java loadPosition: gc.x/y × 屏幕; round(0.25*1920)=480)
#[test]
fn open_uses_position_store_when_no_memory_archive() {
    let mock = MockHandle::new();
    let mut host = OverlayHost::with_factory(mock.factory());
    let (store, _h) = store_with(&[("a", 0.25, 0.5)]);
    host.with_position_store(store);
    host.register(spec("a", 300, 200, [255, 0, 0, 255]));
    host.refresh_preview().unwrap();
    // round(0.25*1920)=480, round(0.5*1080)=540 — 不再居中 (810,440)
    assert!(mock
        .log()
        .contains(&"win0:set_position:480,540".to_string()));
    assert_eq!(host.saved_position("a"), Some((0.25, 0.5))); // 后端命中填内存档
}

/// 会话内存档优先于后端 (同会话拖拽后, 后端快照是旧值; 且不再触发 load)
#[test]
fn memory_archive_takes_priority_over_store() {
    let mock = MockHandle::new();
    let mut host = OverlayHost::with_factory(mock.factory());
    let (store, h) = store_with(&[("a", 0.9, 0.9)]);
    host.with_position_store(store);
    host.register(spec("a", 300, 200, [255, 0, 0, 255]));
    // 预置内存档 (模拟本会话已拖拽; 子模块直摸私有字段): round(0.1*1920)=192
    host.saved_positions.insert("a".to_string(), (0.1, 0.2));
    host.refresh_preview().unwrap();
    assert!(mock
        .log()
        .contains(&"win0:set_position:192,216".to_string()));
    assert!(h.loads.borrow().is_empty(), "内存档命中不应查后端");
}

/// 拖拽松手双写: 内存档 + 后端 (Java mouseReleased → saveWindowPosition 落盘)
#[test]
fn drag_release_persists_to_store() {
    let mock = MockHandle::new();
    let mut host = OverlayHost::with_factory(mock.factory());
    let (store, h) = store_with(&[]);
    host.with_position_store(store);
    host.register(spec("a", 300, 200, [255, 0, 0, 255]));
    host.refresh_preview().unwrap(); // 居中 (810,440)
    mock.push(
        0,
        OverlayEvent::MousePress {
            root_x: 860,
            root_y: 480,
        },
    );
    mock.push(
        0,
        OverlayEvent::MouseMove {
            root_x: 900,
            root_y: 520,
            left_down: true,
        },
    );
    mock.push(0, OverlayEvent::MouseRelease);
    host.pump_events();
    let saves = h.saves.borrow();
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].0, "a");
    assert!((saves[0].1 - 850.0 / 1920.0).abs() < 1e-9);
    assert!((saves[0].2 - 480.0 / 1080.0).abs() < 1e-9);
}

/// 销毁链双写: close 存档也进后端 (Java close → saveCurrentPosition)
#[test]
fn close_persists_to_store() {
    let mock = MockHandle::new();
    let mut host = OverlayHost::with_factory(mock.factory());
    let (store, h) = store_with(&[]);
    host.with_position_store(store);
    host.register(spec("a", 300, 200, [255, 0, 0, 255]));
    host.refresh_preview().unwrap(); // 居中 (810,440)
    host.close("a");
    let saves = h.saves.borrow();
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].0, "a");
    assert!((saves[0].1 - 810.0 / 1920.0).abs() < 1e-9);
    assert!((saves[0].2 - 440.0 / 1080.0).abs() < 1e-9);
}
