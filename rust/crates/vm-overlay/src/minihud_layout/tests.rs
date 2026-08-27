use super::*;
use std::cell::Cell;
use std::rc::Rc;
use vm_core::hud_layout_node::Dimension;

/// 测试组件: 固定尺寸 + 可见开关 (对齐 HUDComponent 两方法契约)
struct VisComp {
    w: i32,
    h: i32,
    visible: bool,
}

impl HasPreferredSize for VisComp {
    fn preferred_size(&self) -> Dimension {
        Dimension::new(self.w, self.h)
    }
}

impl HasVisibility for VisComp {
    fn is_visible(&self) -> bool {
        self.visible
    }
}

/// 可变尺寸组件 (验证 doLayout 无条件重算)
struct DynComp {
    size: Rc<Cell<(i32, i32)>>,
}

impl HasPreferredSize for DynComp {
    fn preferred_size(&self) -> Dimension {
        let (w, h) = self.size.get();
        Dimension::new(w, h)
    }
}

impl HasVisibility for DynComp {
    fn is_visible(&self) -> bool {
        true
    }
}

fn node(id: &str, w: i32, h: i32) -> SharedNode<VisComp> {
    HUDLayoutNode::new(id, VisComp { w, h, visible: true })
}

fn vis(w: i32, h: i32) -> VisComp {
    VisComp { w, h, visible: true }
}

fn render_ids<T: HasPreferredSize + HasVisibility>(e: &ModernHUDLayoutEngine<T>) -> Vec<String> {
    let mut ids = Vec::new();
    e.render(|n, _, _, dbg| {
        if dbg.is_none() {
            ids.push(n.borrow().id.clone());
        }
    });
    ids
}

/// Java String.hashCode oracle (JLS 31 多项式; 值为 Java 8 实测, §6)
#[test]
fn java_string_hashcode_matches_java_oracle() {
    assert_eq!(java_string_hashcode(""), 0);
    assert_eq!(java_string_hashcode("a"), 97);
    // "abc" = 31*(31*97+98)+99 = 96354 (Java 8 实测)
    assert_eq!(java_string_hashcode("abc"), 96354);
    // "row0" = 31*(31*(31*114+111)+119)+48 = 3506582 (Java 8 实测)
    assert_eq!(java_string_hashcode("row0"), 3506582);
    // "row1" = 31*h("row")+49 = 3506583 (共享前缀 "row", Java 8 实测)
    assert_eq!(java_string_hashcode("row1"), 3506583);
    // "speedBar" 溢出回绕为负 (Java int 静默回绕, §2.2 → wrapping_mul)
    assert_eq!(java_string_hashcode("speedBar"), -2131211700);
}

/// drawDebug 颜色 (Java 8 实测 oracle): hash 低 24 位拆 RGB;
/// sum<380 提亮 +100, >=380 原样。speedBar 覆盖负 hash 分支。
#[test]
fn debug_frame_color_brightens_only_dark_ids() {
    // row0: (53,129,150) sum=332 → +100
    assert_eq!(debug_frame_color("row0"), [153, 229, 250, 255]);
    // row1: (53,129,151) sum=333 → +100
    assert_eq!(debug_frame_color("row1"), [153, 229, 251, 255]);
    // attitude: (249,136,255) sum>=380 → 原样
    assert_eq!(debug_frame_color("attitude"), [249, 136, 255, 255]);
    // speedBar: 负 hash 回绕后 (248,74,76) → 原样
    assert_eq!(debug_frame_color("speedBar"), [248, 74, 76, 255]);
}

/// setCanvasOrigin 平移画布 → 根节点 (锚 canvasRect) 坐标随动。
#[test]
fn set_canvas_origin_shifts_root_coordinates() {
    let mut e = ModernHUDLayoutEngine::new(100, 100);
    let n = node("r", 10, 10).set_relative_position(1.0, 0.0);
    e.add_node(n.clone());
    e.do_layout();
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(20, 0, 10, 10));
    e.set_canvas_origin(5, 7);
    e.do_layout();
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(25, 7, 10, 10));
}

/// setLineHeight 的 0.001 阈值: |Δ|<=0.001 不接受 (Java L52)。
/// unitX=1000 放大截断差异: lh=20 → 20000; 20.0005 被拒仍 20000;
/// 20.002 接受后 1000*20.002=20002。
#[test]
fn set_line_height_epsilon_threshold() {
    let mut e = ModernHUDLayoutEngine::new(40000, 100); // 缺省 lineHeight=20.0
    let n = node("r", 10, 10).set_relative_position(1000.0, 0.0);
    e.add_node(n.clone());
    e.do_layout();
    assert_eq!(n.get_pixel_rect().x, 20000);
    e.set_line_height(20.0005); // |Δ|=0.0005 <= 0.001 → 不更新
    e.do_layout();
    assert_eq!(n.get_pixel_rect().x, 20000);
    e.set_line_height(20.002); // |Δ|=0.002 > 0.001 → 更新
    e.do_layout();
    assert_eq!(n.get_pixel_rect().x, 20002);
}

/// 拓扑序: 前序 DFS, 父先于子, 同层按挂载 (set_parent) 顺序。
/// c2 挂在 root 之后于 c1 → DFS 序 root, c1, g, c2。
#[test]
fn topological_order_parents_before_children() {
    let mut e = ModernHUDLayoutEngine::new(100, 100);
    let root = node("root", 10, 10);
    let c1 = node("c1", 10, 10).set_parent(Some(&root));
    let c2 = node("c2", 10, 10).set_parent(Some(&root));
    let g = node("g", 10, 10).set_parent(Some(&c1));
    // add 顺序与树结构无关 (Java HashMap 语义), 唯一根 = root
    e.add_node(c2.clone());
    e.add_node(root.clone());
    e.add_node(g.clone());
    e.add_node(c1.clone());
    e.do_layout();
    let order = render_ids(&e);
    // root.children 挂载序 [c1, c2] → root, c1, (c1 的子) g, c2
    assert_eq!(order, ["root", "c1", "g", "c2"]);
    let idx = |name: &str| order.iter().position(|s| s == name).unwrap();
    assert!(idx("root") < idx("c1"));
    assert!(idx("c1") < idx("g"));
    assert!(idx("root") < idx("c2"));
}

/// 环检测分支: 日志 + 跳过 + 终止 (不死递归)。
/// set_parent API 下环成员互为父 (无根, resolveTopology 不可达 — 与 Java
/// setParent 一致性同款), 故直接驱动 visit_node 验证该分支语义。
#[test]
fn cycle_detection_terminates_and_skips() {
    let a = node("a", 10, 10);
    let b = node("b", 10, 10);
    a.set_parent(Some(&b)); // b.children=[a]
    b.set_parent(Some(&a)); // a.children=[b] — a,b 互环
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    let mut out = Vec::new();
    ModernHUDLayoutEngine::<VisComp>::visit_node(&a, &mut visited, &mut stack, &mut out);
    assert_eq!(out.len(), 2); // a, b 各一次; 二次抵达 a 走环分支返回
    assert!(Rc::ptr_eq(&out[0], &a));
    assert!(Rc::ptr_eq(&out[1], &b));
    // 清理: 无 engine 持有的手工环须显式断开 (Drop 清扫不覆盖本测试的图)
    a.set_parent(None);
    b.set_parent(None);
}

/// 备案 b 的断环履约 (审查 B2): 对抗性 set_parent 构环 (a↔b) + add_node 后
/// drop engine — Drop 清扫逐节点摘父边, 引用计数归一, 无 Rc 环泄漏
/// (Java GC 可收环的 Rust 对应物)。
#[test]
fn drop_sweeps_adversarial_cycle_edges() {
    let a = node("a", 10, 10);
    let b = node("b", 10, 10);
    a.set_parent(Some(&b)); // b.children=[a]
    b.set_parent(Some(&a)); // a.children=[b] — a↔b children 强环
    let mut e = ModernHUDLayoutEngine::<VisComp>::new(100, 100);
    e.add_node(a.clone());
    e.add_node(b.clone());
    // 本地句柄 + nodes map + 对方 children 各持一份强引用
    assert_eq!(Rc::strong_count(&a), 3);
    assert_eq!(Rc::strong_count(&b), 3);
    drop(e); // Drop 清扫: a/b 各自从对方 children 摘除, map 随字段释放
    assert_eq!(Rc::strong_count(&a), 1);
    assert_eq!(Rc::strong_count(&b), 1);
}

/// doLayout 的 Java 怪癖: dirty 清零后仍无条件重算 (组件尺寸变化无需置脏)。
#[test]
fn do_layout_recalculates_when_clean() {
    let size = Rc::new(Cell::new((10, 10)));
    let n = HUDLayoutNode::new("d", DynComp { size: Rc::clone(&size) });
    let mut e = ModernHUDLayoutEngine::new(100, 100);
    e.add_node(n.clone());
    e.do_layout();
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(0, 0, 10, 10));
    size.set((40, 20)); // 组件尺寸变化, 未触碰引擎
    e.do_layout();
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(0, 0, 40, 20));
}

/// 引擎驱动的锚点公式手算对拍: 根 (2.1,3.5)*lh=20 → (42,70);
/// 子 BottomLeft 锚 + (0,0.1)*20=2 → (42, 70+10+2=82)。
#[test]
fn solve_chain_via_engine_matches_manual_math() {
    let mut e = ModernHUDLayoutEngine::new(300, 200);
    let root = node("root", 50, 10).set_relative_position(2.1, 3.5);
    let child = node("child", 60, 10)
        .set_parent(Some(&root))
        .set_relative_position(0.0, 0.1)
        .set_anchors(Anchor::BottomLeft, Anchor::TopLeft);
    e.add_node(root.clone());
    e.add_node(child.clone());
    e.do_layout();
    assert_eq!(root.get_pixel_rect(), Rectangle::with_bounds(42, 70, 50, 10));
    assert_eq!(child.get_pixel_rect(), Rectangle::with_bounds(42, 82, 60, 10));
}

/// getContentBounds: 只统计可见节点; 空集 (含未布局) 返回 1x1 兜底。
#[test]
fn get_content_bounds_visible_only_and_empty_fallback() {
    let mut e = ModernHUDLayoutEngine::new(300, 200);
    let a = HUDLayoutNode::new("a", vis(20, 20));
    let b = node("b", 50, 10).set_relative_position(2.1, 3.5);
    let hidden = HUDLayoutNode::new("h", VisComp { w: 999, h: 999, visible: false });
    e.add_node(a);
    e.add_node(b);
    e.add_node(hidden);
    // doLayout 前 sortedNodes 为空 (Java 同: 引擎持有空表)
    assert_eq!(e.get_content_bounds(), Rectangle::with_bounds(0, 0, 1, 1));
    e.do_layout();
    // 可见: a (0,0,20,20) + b (42,70,50,10) → (0,0,92,80); 隐藏 h 不计
    assert_eq!(e.get_content_bounds(), Rectangle::with_bounds(0, 0, 92, 80));
}

/// applyAutoSizing 数学: 新窗口 = 内容 + 2*padding; 偏移把内容左上推到
/// padding 处; setRenderOffset 副作用立即作用于 render。
#[test]
fn auto_sizing_plan_and_render_offset() {
    let mut e = ModernHUDLayoutEngine::new(300, 200);
    e.add_node(HUDLayoutNode::new("a", vis(20, 20)));
    e.add_node(node("b", 50, 10).set_relative_position(2.1, 3.5));
    let plan = e.apply_auto_sizing(10);
    assert_eq!(
        plan,
        AutoSizingPlan { new_width: 112, new_height: 100, offset_x: 10, offset_y: 10 }
    );
    let mut seen = Vec::new();
    e.render(|n, x, y, dbg| {
        if dbg.is_none() {
            seen.push((n.borrow().id.clone(), x, y));
        }
    });
    // b: (42,70) + offset(10,10) = (52,80)
    assert_eq!(seen, vec![("a".to_string(), 10, 10), ("b".to_string(), 52, 80)]);
}

/// render: 隐藏节点跳过; debug 开启时每节点本体之后紧跟调试框回调
/// (Java component.draw → drawDebug 的逐节点 z 序), 框色 = id hash 派生。
#[test]
fn render_skips_hidden_and_emits_debug_frame() {
    let mut e = ModernHUDLayoutEngine::new(100, 100);
    let a = node("a", 30, 10).set_relative_position(1.0, 2.0); // (20,40)
    let hidden = HUDLayoutNode::new("x", VisComp { w: 5, h: 5, visible: false });
    e.add_node(a.clone());
    e.add_node(hidden);
    e.set_render_offset(5, 7);
    e.set_debug(true);
    e.do_layout();
    let mut calls = Vec::new();
    e.render(|n, x, y, d| calls.push((n.borrow().id.clone(), x, y, d.is_some())));
    assert_eq!(calls.len(), 2); // a 两次, x 零次
    assert_eq!(calls[0], ("a".to_string(), 25, 47, false)); // 本体 (20+5, 40+7)
    assert_eq!(calls[1], ("a".to_string(), 25, 47, true)); // 调试框
    e.render(|n, _x, _y, d| {
        if let Some(c) = d {
            assert_eq!(n.borrow().id, "a");
            assert_eq!(c, debug_frame_color("a"));
        }
    });
}

/// addNode 同 id 覆盖 (HashMap.put): 值替换、位置不变。
#[test]
fn add_node_duplicate_id_replaces_keeps_order() {
    let mut e = ModernHUDLayoutEngine::new(100, 100);
    let n1 = node("x", 1, 1);
    let n2 = node("x", 2, 2);
    e.add_node(n1);
    e.add_node(node("y", 3, 3));
    e.do_layout();
    assert_eq!(render_ids(&e), ["x", "y"]);
    e.add_node(n2);
    e.do_layout();
    assert_eq!(e.get_node("x").unwrap().borrow().component.w, 2);
    assert_eq!(render_ids(&e), ["x", "y"]); // 覆盖不改变遍历位置
}

/// clear(): 节点表/排序表清空, bounds 回 1x1 兜底。
#[test]
fn clear_resets_engine() {
    let mut e = ModernHUDLayoutEngine::new(100, 100);
    e.add_node(node("a", 1, 1));
    e.do_layout();
    e.clear();
    assert!(e.get_node("a").is_none());
    let mut calls = 0;
    e.render(|_, _, _, _| calls += 1);
    assert_eq!(calls, 0);
    assert_eq!(e.get_content_bounds(), Rectangle::with_bounds(0, 0, 1, 1));
}

/// cfg 快照 oracle: 逐项对照 ui_layout.cfg (panel "MiniHUD" L45-94)。
#[test]
fn cfg_snapshot_matches_ui_layout_panel() {
    assert_eq!(MINIHUD_PANEL_ITEMS.len(), 28);
    let find = |t: &str| MINIHUD_PANEL_ITEMS.iter().find(|i| i.target == t).unwrap();
    assert_eq!(find("crosshairSwitch").default, CfgDefault::Bool(true));
    assert_eq!(find("drawHUDtext").default, CfgDefault::Bool(true));
    assert_eq!(find("displayCrosshair").default, CfgDefault::Bool(true));
    assert_eq!(find("enableFlapAngleBar").default, CfgDefault::Bool(true));
    assert_eq!(find("showSpeedBar").default, CfgDefault::Bool(true));
    assert_eq!(find("showAttitudeGauge").default, CfgDefault::Bool(true));
    assert_eq!(find("attitudeIndicatorInertialMode").default, CfgDefault::Bool(false));
    assert_eq!(find("alwaysShowRadarAltitude").default, CfgDefault::Bool(false));
    let aoa = find("miniHUDaoaWarningRatio");
    assert_eq!(aoa.default, CfgDefault::Int(20));
    assert_eq!((aoa.min, aoa.max, aoa.unit), (Some(0), Some(100), Some("%")));
    let aoa_bar = find("miniHUDaoaBarWarningRatio");
    assert_eq!(aoa_bar.default, CfgDefault::Int(25));
    assert_eq!((aoa_bar.min, aoa_bar.max, aoa_bar.unit), (Some(0), Some(100), Some("%")));
    assert_eq!(find("disableHUDSpeedLabel").item_type, MiniHudItemType::SwitchInv);
    assert_eq!(find("disableHUDHeightLabel").item_type, MiniHudItemType::SwitchInv);
    assert_eq!(find("disableHUDSEPLabel").item_type, MiniHudItemType::SwitchInv);
    let scale = find("crosshairScale");
    assert_eq!(scale.default, CfgDefault::Int(113));
    assert_eq!((scale.min, scale.max), (Some(0), Some(200)));
    let font = find("fontSize");
    assert_eq!(font.default, CfgDefault::Int(0));
    assert_eq!((font.min, font.max), (Some(-10), Some(10)));
    assert_eq!(find("crosshairName").default, CfgDefault::Str("软件渲染准星"));
    assert_eq!(find("MonoNumFont").default, CfgDefault::Str("Sarasa Mono SC"));
    // target 在 panel 段内唯一
    let mut targets: Vec<&str> = MINIHUD_PANEL_ITEMS.iter().map(|i| i.target).collect();
    let n = targets.len();
    targets.sort_unstable();
    targets.dedup();
    assert_eq!(targets.len(), n);
    // 布局调试开关 (panel 外「杂项→调试」组单列快照)
    assert_eq!(ENABLE_LAYOUT_DEBUG_ITEM.target, "enableLayoutDebug");
    assert_eq!(ENABLE_LAYOUT_DEBUG_ITEM.default, CfgDefault::Bool(false));
    // cfg 驱动读取: 缺键 = Java getBool 字面兜底 (displayCrosshair **false**,
    // 非 cfg :default true — 整树缺失时 Java 关准星; 两层缺省见 from_bool_source)
    let cfg = MiniHudLayoutConfig::from_bool_source(|_| None);
    assert!(!cfg.display_crosshair);
    assert!(!cfg.enable_layout_debug);
    // override 生效; 未覆盖键走字面兜底
    let cfg = MiniHudLayoutConfig::from_bool_source(|k| (k == "displayCrosshair").then_some(true));
    assert!(cfg.display_crosshair);
    assert!(!cfg.enable_layout_debug);
}

/// 全树构建几何 oracle (lh=24, 组件统一 40x20, 画布 base 300x200):
/// displayCrosshair=true → 画布 600 宽; DFS 序 = 挂载序。
#[test]
fn build_full_tree_topology_and_geometry() {
    let parts = MiniHudParts {
        rows: (0..5).map(|_| vis(40, 20)).collect(),
        flap_angle_bar: vis(40, 20),
        speed_ratio_bar: vis(40, 20),
        throttle_bar: vis(40, 20),
        attitude_indicator_gauge: vis(40, 20),
        compass_gauge: vis(40, 20),
        crosshair_gauge: Some(vis(40, 20)),
    };
    let built = build_mihud_layout(&MiniHudLayoutConfig::default(), parts, 300, 200, 24.0);
    assert_eq!(MINIHUD_NODE_SPECS.len(), 11);
    // DFS 前序: row 链循环 (L699-713) 先于右挂件 (L719-731), 故
    // row2.children=[row3, attitude, compass] 挂载序 → row3 子树 (含
    // row4 的 speedBar/throttle) 全部先于 attitude/compass; crosshair 根最后
    assert_eq!(
        render_ids(&built.engine),
        ["row0", "flap", "row1", "row2", "row3", "row4", "speedBar", "throttle", "attitude", "compass", "crosshair"]
    );
    // 逐节点 rect 手算 (单位偏移 ×24 后 (int) 截断, 锚点公式见各注释)
    let expect: [(&str, Rectangle); 11] = [
        ("row0", Rectangle::with_bounds(50, 84, 40, 20)),       // (2.1,3.5)*24 → (50.4,84)→(50,84)
        ("flap", Rectangle::with_bounds(50, 62, 40, 20)),       // row0 顶 (50,84)+(-0.1*24=-2.4→-2), 底锚上移 h=20
        ("row1", Rectangle::with_bounds(50, 106, 40, 20)),      // row0 底 (50,104)+2.4→2
        ("row2", Rectangle::with_bounds(50, 128, 40, 20)),      // row1 底 (50,126)+2
        ("attitude", Rectangle::with_bounds(50, 160, 40, 20)),  // row2 右下 (90,148)+0.5*24=12, TOP_RIGHT: x=90-40
        ("compass", Rectangle::with_bounds(50, 150, 40, 20)),   // (90,148)+2.4→2, TOP_RIGHT
        ("row3", Rectangle::with_bounds(50, 150, 40, 20)),      // row2 底 (50,148)+2
        ("row4", Rectangle::with_bounds(50, 172, 40, 20)),      // row3 底 (50,170)+2
        ("speedBar", Rectangle::with_bounds(3, 172, 40, 20)),   // row4 左下 (50,192)+(-0.3*24=-7.2→-7), BOTTOM_RIGHT: (43-40,192-20)
        ("throttle", Rectangle::with_bounds(3, 172, 40, 20)),   // 同 speedBar (Java 同位互斥可见)
        ("crosshair", Rectangle::with_bounds(560, 90, 40, 20)), // 画布 600x200 MIDDLE_RIGHT (600,100), 自锚减半宽/半高
    ];
    for (id, rect) in expect {
        assert_eq!(
            built.engine.get_node(id).unwrap().get_pixel_rect(),
            rect,
            "{id}"
        );
    }
    // crosshair 为根 (父=None)
    assert!(built.engine.get_node("crosshair").unwrap().get_parent().is_none());
    // 内容包围盒 (3,62)~(600,192) → padding 45 自动尺寸
    assert_eq!(
        built.sizing,
        Some(AutoSizingPlan { new_width: 687, new_height: 220, offset_x: 42, offset_y: -17 })
    );
}

/// displayCrosshair=false: 画布不翻倍, crosshair 节点不建 (cfg 驱动分支)。
#[test]
fn build_without_crosshair() {
    let parts = MiniHudParts {
        rows: (0..5).map(|_| vis(40, 20)).collect(),
        flap_angle_bar: vis(40, 20),
        speed_ratio_bar: vis(40, 20),
        throttle_bar: vis(40, 20),
        attitude_indicator_gauge: vis(40, 20),
        compass_gauge: vis(40, 20),
        crosshair_gauge: None,
    };
    let cfg = MiniHudLayoutConfig { display_crosshair: false, ..Default::default() };
    let built = build_mihud_layout(&cfg, parts, 300, 200, 24.0);
    assert!(built.engine.get_node("crosshair").is_none());
    assert_eq!(built.engine.get_node("row0").unwrap().get_pixel_rect(), Rectangle::with_bounds(50, 84, 40, 20));
    // maxX 回落到 90 (attitude/compass/文本列右缘), 包围盒 (3,62,87,130)
    assert_eq!(
        built.sizing,
        Some(AutoSizingPlan { new_width: 177, new_height: 220, offset_x: 42, offset_y: -17 })
    );
}

/// 行数不足的退化拓扑: rows=2 → 无 row2/row4;
/// attitude/compass 不建 (Java if(row2!=null)); speedBar/throttle 的父
/// row4 缺席 → setParent(null) 退化为根 (Java setParent 可空参数)。
#[test]
fn build_short_rows_variant() {
    let parts = MiniHudParts {
        rows: vec![vis(40, 20), vis(40, 20)],
        flap_angle_bar: vis(40, 20),
        speed_ratio_bar: vis(40, 20),
        throttle_bar: vis(40, 20),
        attitude_indicator_gauge: vis(40, 20),
        compass_gauge: vis(40, 20),
        crosshair_gauge: None,
    };
    let cfg = MiniHudLayoutConfig { display_crosshair: false, ..Default::default() };
    let built = build_mihud_layout(&cfg, parts, 300, 200, 24.0);
    assert_eq!(
        render_ids(&built.engine),
        ["row0", "flap", "row1", "speedBar", "throttle"]
    );
    for id in ["row2", "row3", "row4", "attitude", "compass", "crosshair"] {
        assert!(built.engine.get_node(id).is_none(), "{id}");
    }
    // speedBar 根: 父矩形退化为 canvasRect, BottomLeft 锚 (0,200) + (-7.2→-7, 0)
    // → BOTTOM_RIGHT 自锚 (0-7-40, 200-20)
    assert_eq!(
        built.engine.get_node("speedBar").unwrap().get_pixel_rect(),
        Rectangle::with_bounds(-47, 180, 40, 20)
    );
    // 包围盒 (-47,62)~(90,200) → (-47,62,137,138)
    assert_eq!(
        built.sizing,
        Some(AutoSizingPlan { new_width: 227, new_height: 228, offset_x: 92, offset_y: -17 })
    );
}

/// 空 rows 守卫 (Java components.isEmpty() 裸 return): 空引擎, 不自动尺寸
/// (sizing=None, 窗口/renderOffset 保持宿主原状), 无任何节点。
#[test]
fn build_empty_rows_returns_empty_engine() {
    let parts = MiniHudParts {
        rows: Vec::new(),
        flap_angle_bar: vis(10, 10),
        speed_ratio_bar: vis(10, 10),
        throttle_bar: vis(10, 10),
        attitude_indicator_gauge: vis(10, 10),
        compass_gauge: vis(10, 10),
        crosshair_gauge: Some(vis(10, 10)),
    };
    let built = build_mihud_layout(&MiniHudLayoutConfig::default(), parts, 300, 200, 24.0);
    assert!(built.engine.get_node("row0").is_none());
    assert!(built.engine.get_node("flap").is_none());
    assert!(built.sizing.is_none());
}
