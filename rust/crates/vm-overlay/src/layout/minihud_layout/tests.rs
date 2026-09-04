use super::*;
use crate::layout::hud_layout_node::{Dimension, HUDLayoutNode};
use crate::layout::Anchor;
use std::cell::Cell;
use std::rc::Rc;

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
    HUDLayoutNode::new(
        id,
        VisComp {
            w,
            h,
            visible: true,
        },
    )
}

fn vis(w: i32, h: i32) -> VisComp {
    VisComp {
        w,
        h,
        visible: true,
    }
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

/// Java String.hashCode 基线 (JLS 31 多项式; 值为 Java 8 实测, §6)
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

/// drawDebug 颜色 (Java 8 实测 基线): hash 低 24 位拆 RGB;
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
    let n = HUDLayoutNode::new(
        "d",
        DynComp {
            size: Rc::clone(&size),
        },
    );
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
    assert_eq!(
        root.get_pixel_rect(),
        Rectangle::with_bounds(42, 70, 50, 10)
    );
    assert_eq!(
        child.get_pixel_rect(),
        Rectangle::with_bounds(42, 82, 60, 10)
    );
}

/// getContentBounds: 只统计可见节点; 空集 (含未布局) 返回 1x1 兜底。
#[test]
fn get_content_bounds_visible_only_and_empty_fallback() {
    let mut e = ModernHUDLayoutEngine::new(300, 200);
    let a = HUDLayoutNode::new("a", vis(20, 20));
    let b = node("b", 50, 10).set_relative_position(2.1, 3.5);
    let hidden = HUDLayoutNode::new(
        "h",
        VisComp {
            w: 999,
            h: 999,
            visible: false,
        },
    );
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
        AutoSizingPlan {
            new_width: 112,
            new_height: 100,
            offset_x: 10,
            offset_y: 10
        }
    );
    let mut seen = Vec::new();
    e.render(|n, x, y, dbg| {
        if dbg.is_none() {
            seen.push((n.borrow().id.clone(), x, y));
        }
    });
    // b: (42,70) + offset(10,10) = (52,80)
    assert_eq!(
        seen,
        vec![("a".to_string(), 10, 10), ("b".to_string(), 52, 80)]
    );
}

/// render: 隐藏节点跳过; debug 开启时每节点本体之后紧跟调试框回调
/// (Java component.draw → drawDebug 的逐节点 z 序), 框色 = id hash 派生。
#[test]
fn render_skips_hidden_and_emits_debug_frame() {
    let mut e = ModernHUDLayoutEngine::new(100, 100);
    let a = node("a", 30, 10).set_relative_position(1.0, 2.0); // (20,40)
    let hidden = HUDLayoutNode::new(
        "x",
        VisComp {
            w: 5,
            h: 5,
            visible: false,
        },
    );
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

/// cfg 快照 基线: 逐项对照 ui_layout.cfg (panel "MiniHUD" L45-94)。
/// 波12 起常量表仅测试消费, 自 minihud_layout.rs 移入本文件 (生产代码不持
/// ui_layout.cfg 的第二份手工快照 — 配置真值走 ReinitParams/cfg 树)。
///
/// :type (布局消费的子集; info 行不含 target 不入表)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MiniHudItemType {
    /// switch (UI ON = value true)
    Switch,
    /// switch-inv (UI ON = value false)
    SwitchInv,
    /// slider (:min/:max/:unit)
    Slider,
    /// combo (:source 列表)
    Combo,
}

/// :value / :default 字面量 (cfg 两列恒同值)
#[derive(Debug, Clone, Copy, PartialEq)]
enum CfgDefault {
    Bool(bool),
    Int(i32),
    Str(&'static str),
}

/// panel "MiniHUD" 单条 (item ...) 定义快照
#[derive(Debug, Clone, Copy)]
struct MiniHudCfgItem {
    item_type: MiniHudItemType,
    /// :target 配置键 (cfg 字符串键原样)
    target: &'static str,
    default: CfgDefault,
    /// slider :min
    min: Option<i32>,
    /// slider :max
    max: Option<i32>,
    /// slider :unit
    unit: Option<&'static str>,
}

/// MiniHUD panel 段 28 条 item 逐行快照 (ui_layout.cfg L45-94, 顺序一致;
/// group 归属见行间注释, label 断言未消费故不入表)。
/// 只保留 type/target/value/min/max/unit (布局消费的子集);
/// combo 的 :source 与 :desc 等设置面板字段未入表 — vm-ui 生成完整设置面板时
/// 须从 ui_layout.cfg 另出全量表, 勿复用本表 (避免单一来源分裂)。
const MINIHUD_PANEL_ITEMS: &[MiniHudCfgItem] = &[
    // (group "基本设定")
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "crosshairSwitch",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    // (group "hud面板设置")
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "drawHUDtext",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "displayCrosshair",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    // (group "hud数据设置")
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "enableFlapAngleBar",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showSpeedBar",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showAttitudeGauge",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "attitudeIndicatorInertialMode",
        default: CfgDefault::Bool(false),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "hudMach",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "alwaysShowRadarAltitude",
        default: CfgDefault::Bool(false),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Slider,
        target: "miniHUDaoaWarningRatio",
        default: CfgDefault::Int(20),
        min: Some(0),
        max: Some(100),
        unit: Some("%"),
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Slider,
        target: "miniHUDaoaBarWarningRatio",
        default: CfgDefault::Int(25),
        min: Some(0),
        max: Some(100),
        unit: Some("%"),
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDSpeed",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDAoA",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDAltitude",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDEnergy",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDFlaps",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDAirbrake",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDGear",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDSep",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDGLoad",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Switch,
        target: "showHUDManeuverBar",
        default: CfgDefault::Bool(true),
        min: None,
        max: None,
        unit: None,
    },
    // (group "hud文字标签设置")
    MiniHudCfgItem {
        item_type: MiniHudItemType::SwitchInv,
        target: "disableHUDSpeedLabel",
        default: CfgDefault::Bool(false),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::SwitchInv,
        target: "disableHUDHeightLabel",
        default: CfgDefault::Bool(false),
        min: None,
        max: None,
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::SwitchInv,
        target: "disableHUDSEPLabel",
        default: CfgDefault::Bool(false),
        min: None,
        max: None,
        unit: None,
    },
    // (group "hud准星设置")
    MiniHudCfgItem {
        item_type: MiniHudItemType::Combo,
        target: "crosshairName",
        default: CfgDefault::Str("软件渲染准星"),
        min: None,
        max: None,
        unit: None,
    },
    // (group "外观设置")
    MiniHudCfgItem {
        item_type: MiniHudItemType::Slider,
        target: "crosshairScale",
        default: CfgDefault::Int(113),
        min: Some(0),
        max: Some(200),
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Slider,
        target: "fontSize",
        default: CfgDefault::Int(0),
        min: Some(-10),
        max: Some(10),
        unit: None,
    },
    MiniHudCfgItem {
        item_type: MiniHudItemType::Combo,
        target: "MonoNumFont",
        default: CfgDefault::Str("Sarasa Mono SC"),
        min: None,
        max: None,
        unit: None,
    },
];

/// enableLayoutDebug 不在 MiniHUD panel 段 (位于「杂项→调试」组), 布局引擎
/// 开关的单列快照。
const ENABLE_LAYOUT_DEBUG_ITEM: MiniHudCfgItem = MiniHudCfgItem {
    item_type: MiniHudItemType::Switch,
    target: "enableLayoutDebug",
    default: CfgDefault::Bool(false),
    min: None,
    max: None,
    unit: None,
};

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
    assert_eq!(
        find("attitudeIndicatorInertialMode").default,
        CfgDefault::Bool(false)
    );
    assert_eq!(
        find("alwaysShowRadarAltitude").default,
        CfgDefault::Bool(false)
    );
    let aoa = find("miniHUDaoaWarningRatio");
    assert_eq!(aoa.default, CfgDefault::Int(20));
    assert_eq!(
        (aoa.min, aoa.max, aoa.unit),
        (Some(0), Some(100), Some("%"))
    );
    let aoa_bar = find("miniHUDaoaBarWarningRatio");
    assert_eq!(aoa_bar.default, CfgDefault::Int(25));
    assert_eq!(
        (aoa_bar.min, aoa_bar.max, aoa_bar.unit),
        (Some(0), Some(100), Some("%"))
    );
    assert_eq!(
        find("disableHUDSpeedLabel").item_type,
        MiniHudItemType::SwitchInv
    );
    assert_eq!(
        find("disableHUDHeightLabel").item_type,
        MiniHudItemType::SwitchInv
    );
    assert_eq!(
        find("disableHUDSEPLabel").item_type,
        MiniHudItemType::SwitchInv
    );
    let scale = find("crosshairScale");
    assert_eq!(scale.default, CfgDefault::Int(113));
    assert_eq!((scale.min, scale.max), (Some(0), Some(200)));
    let font = find("fontSize");
    assert_eq!(font.default, CfgDefault::Int(0));
    assert_eq!((font.min, font.max), (Some(-10), Some(10)));
    assert_eq!(
        find("crosshairName").default,
        CfgDefault::Str("软件渲染准星")
    );
    assert_eq!(
        find("MonoNumFont").default,
        CfgDefault::Str("Sarasa Mono SC")
    );
    // target 在 panel 段内唯一
    let mut targets: Vec<&str> = MINIHUD_PANEL_ITEMS.iter().map(|i| i.target).collect();
    let n = targets.len();
    targets.sort_unstable();
    targets.dedup();
    assert_eq!(targets.len(), n);
    // 布局调试开关 (panel 外「杂项→调试」组单列快照)
    assert_eq!(ENABLE_LAYOUT_DEBUG_ITEM.target, "enableLayoutDebug");
    assert_eq!(ENABLE_LAYOUT_DEBUG_ITEM.default, CfgDefault::Bool(false));
}

// ===========================================================================
// PageDoc 数据驱动建树 (build_page_layout; 原 build_mihud_layout 常量拓扑族
// 的接替 — 组件经注册表工厂创建, 几何以锚点关系式钉 doc 拓扑值)
// ===========================================================================

use crate::overlays::minihud::MinimalHudContext;
use crate::widgets::{build_page_layout, BuiltPageLayout, FactoryCtx, PageBuildInputs};
use vm_core::config::config_api::HudSettingsSnapshot;
use vm_core::config::json_model::{ComponentDoc, PageDoc};

fn page_font_path() -> std::path::PathBuf {
    std::path::Path::new("../../../fonts").join("sarasa-mono-sc-bold.ttf")
}

/// 出厂页 (minihud-default)
fn factory_page() -> PageDoc {
    vm_core::config::json_store::factory()
        .pages
        .iter()
        .find(|d| d.id == "minihud-default")
        .cloned()
        .expect("出厂页 minihud-default 应存在")
}

/// 建树 settings 快照 (crosshairScale=113, 行开关全开 — cfg :default 同源;
/// 建树几何只消费 crossScale 派生量, 开关族不影响 build)
fn page_snap() -> HudSettingsSnapshot {
    HudSettingsSnapshot {
        num_font: "Sarasa Mono SC".into(),
        crosshair_scale: 113,
        crosshair_name: "软件渲染准星".into(),
        display_crosshair: true,
        draw_hud_text: true,
        show_attitude_gauge: true,
        enable_flap_angle_bar: true,
        show_speed_bar: true,
        draw_hud_mach: true,
        show_hud_speed: true,
        show_hud_aoa: true,
        show_hud_altitude: true,
        show_hud_energy: true,
        show_hud_flaps: true,
        show_hud_airbrake: true,
        show_hud_gear: true,
        show_hud_sep: true,
        show_hud_g_load: true,
        show_hud_maneuver_bar: true,
        ..Default::default()
    }
}

/// 建树助手 (lh=24, 画布显式传 — 对齐原 build 测试的 300x200/600x200 口径)
fn build_page(
    doc: &PageDoc,
    visible_src: &dyn Fn(&str) -> Option<bool>,
    canvas_w: i32,
    canvas_h: i32,
) -> BuiltPageLayout {
    let ctx = MinimalHudContext::create(&page_snap(), 1.0, &page_font_path()).unwrap();
    let fonts = Rc::new(ctx.fonts.clone());
    let fctx = FactoryCtx {
        minihud_ctx: Some(&ctx),
        fonts,
        rows: &EMPTY_TEST_ROWS,
        engine_disables: None,
        lang: None,
        fonts_dir: None,
        fields_cfg: None,
        gauge_cfg: None,
    };
    let inputs = PageBuildInputs {
        doc,
        fctx: &fctx,
        visible_src,
        canvas_w,
        canvas_h,
        line_height: 24.0,
        debug: false,
    };
    build_page_layout(&inputs)
}

/// 出厂页全树: 11 组件 cell/节点全建, DFS 前序 = factory components 挂载序;
/// 逐节点锚点关系式手算 (单位偏移 ×24 截断 + 锚点对齐 — 组件尺寸字体相关
/// 不入字面量表, 关系式即 doc 拓扑值的 oracle)。
#[test]
fn page_layout_full_tree_topology_and_geometry() {
    let doc = factory_page();
    let built = build_page(&doc, &|_| Some(true), 600, 200);
    assert_eq!(built.cells.len(), 11);
    // DFS 前序: row 链 (row0→flap→row1..row4) 先于 row2 右挂件
    // (attitude/compass), row4 子 (speedBar/throttle) 先于 attitude/compass
    // (挂载序), crosshair 根最后
    assert_eq!(
        render_ids(&built.engine),
        [
            "row0", "flap", "row1", "row2", "row3", "row4", "speedBar", "throttle",
            "attitude", "compass", "crosshair"
        ]
    );
    let rect = |id: &str| built.engine.get_node(id).unwrap().get_pixel_rect();
    // row0 根: TopLeft/TopLeft 于 canvas(0,0) + (2.1,3.5)*24 = (50.4,84)→(50,84)
    assert_eq!((rect("row0").x, rect("row0").y), (50, 84));
    // flap: BottomLeft 挂 row0 TopLeft + (0,-0.1)*24=-2.4→-2 → 底贴 row0 顶上 2px
    assert_eq!(rect("flap").x, rect("row0").x);
    assert_eq!(rect("flap").y + rect("flap").height, rect("row0").y - 2);
    // row 链: TopLeft 挂前一行 BottomLeft + (0,0.1)*24=2.4→2
    for (prev, next) in [("row0", "row1"), ("row1", "row2"), ("row2", "row3"), ("row3", "row4")] {
        assert_eq!(
            rect(next).y - (rect(prev).y + rect(prev).height),
            2,
            "{next} 链间距 0.1×24 截断"
        );
    }
    // attitude/compass: TopRight 挂 row2 BottomRight + (0,0.5)/ (0,0.1) ×24
    let row2_bottom = rect("row2").y + rect("row2").height;
    let row2_right = rect("row2").x + rect("row2").width;
    assert_eq!(rect("attitude").y - row2_bottom, 12); // 0.5*24
    assert_eq!(rect("attitude").x + rect("attitude").width, row2_right);
    assert_eq!(rect("compass").y - row2_bottom, 2); // 0.1*24=2.4→2
    assert_eq!(rect("compass").x + rect("compass").width, row2_right);
    // speedBar/throttle: BottomRight 挂 row4 BottomLeft + (-0.3,0)*24=-7.2→-7
    let row4_bottom = rect("row4").y + rect("row4").height;
    for id in ["speedBar", "throttle"] {
        assert_eq!(rect(id).y + rect(id).height, row4_bottom, "{id} 底贴 row4 底");
        assert_eq!(rect(id).x + rect(id).width, rect("row4").x - 7, "{id} 左让 7px");
    }
    // crosshair 独立根: MiddleRight 自/父锚 → 右缘贴画布, 垂直居中
    assert!(built.engine.get_node("crosshair").unwrap().get_parent().is_none());
    assert!(built.engine.get_node("row0").unwrap().get_parent().is_none());
    let ch = rect("crosshair");
    assert_eq!(ch.x + ch.width, 600);
    assert_eq!(ch.y + ch.height / 2, 100);
    // padding 45 (doc.padding) 进自动尺寸: 窗口 = 包围盒 + 2×45, 偏移推到 45
    let bounds = built.engine.get_content_bounds();
    let plan = built.sizing.unwrap();
    assert_eq!(plan.new_width, bounds.width + 90);
    assert_eq!(plan.new_height, bounds.height + 90);
    assert_eq!((plan.offset_x, plan.offset_y), (45 - bounds.x, 45 - bounds.y));
}

/// displayCrosshair=false (visibleWhen 求值 false): crosshair 节点与 cell
/// 都不建 (W2 建树门控); 求值源缺键 (None) 同样不建 — Java getBool 字面
/// 兜底 false 语义 (整树缺失时关准星)。
#[test]
fn page_layout_without_crosshair() {
    let doc = factory_page();
    let built = build_page(&doc, &|k| (k == "displayCrosshair").then_some(false), 300, 200);
    assert_eq!(built.cells.len(), 10);
    assert!(!built.cells.contains_key("crosshair"));
    assert!(built.engine.get_node("crosshair").is_none());
    // 行链几何不受裁剪影响
    let row0 = built.engine.get_node("row0").unwrap().get_pixel_rect();
    assert_eq!((row0.x, row0.y), (50, 84));
    // padding 语义同全树
    let bounds = built.engine.get_content_bounds();
    let plan = built.sizing.unwrap();
    assert_eq!(plan.new_width, bounds.width + 90);
    assert_eq!(plan.offset_x, 45 - bounds.x);

    // 求值源缺键 → unwrap_or(false) 不建 (原 MiniHudLayoutConfig::from_bool_source
    // 两层缺省的字面兜底分支)
    let built2 = build_page(&doc, &|_| None, 300, 200);
    assert_eq!(built2.cells.len(), 10);
    assert!(built2.engine.get_node("crosshair").is_none());
}

/// 父组件缺席 (用户编辑删父) → 子组件退化根, 不无故消失 (W2 宽容裁决;
/// 原 build 的 speedBar/throttle setParent(null) 语义泛化到全部组件)。
#[test]
fn page_layout_missing_parent_degrades_to_root() {
    let mut doc = factory_page();
    doc.components.retain(|c| c.id != "row4"); // 删父, speedBar/throttle 悬空
    let built = build_page(&doc, &|_| Some(true), 300, 200);
    // 悬空子仍在 (cells/节点), 退化根
    for id in ["speedBar", "throttle"] {
        assert!(built.cells.contains_key(id), "{id} 不应因删父消失");
        assert!(
            built.engine.get_node(id).unwrap().get_parent().is_none(),
            "{id} 父缺席应退化根"
        );
    }
    // 根锚: BottomLeft 于 canvas (0,200) + (-0.3*24=-7.2→-7, 0),
    // 自锚 BottomRight → 底贴画布底, 右缘 = -7
    for id in ["speedBar", "throttle"] {
        let r = built.engine.get_node(id).unwrap().get_pixel_rect();
        assert_eq!(r.y + r.height, 200, "{id} 底贴画布底");
        assert_eq!(r.x + r.width, -7, "{id} 右缘 = 0-7");
    }
    // row 链不受影响 (row3 的父 row2 在场)
    assert!(built.engine.get_node("row3").unwrap().get_parent().is_some());
}

/// 硬开关 enabled=false 与未注册类型: 组件不建 (warn 跳过, 出厂页不可达分支)。
#[test]
fn page_layout_skips_disabled_and_unknown_types() {
    let doc = PageDoc {
        id: "t".into(),
        padding: 45,
        components: vec![
            ComponentDoc {
                id: "off".into(),
                r#type: "core.minihud.row0".into(),
                enabled: false, // 硬开关关 → 不建
                ..Default::default()
            },
            ComponentDoc {
                id: "on".into(),
                r#type: "core.minihud.row1".into(),
                enabled: true,
                ..Default::default()
            },
            ComponentDoc {
                id: "bogus".into(),
                r#type: "core.nope.ghost".into(), // 未注册 → 工厂失败跳过
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let built = build_page(&doc, &|_| None, 300, 200);
    assert_eq!(built.cells.len(), 1);
    assert!(built.cells.contains_key("on"));
    for id in ["off", "bogus"] {
        assert!(!built.cells.contains_key(id), "{id} 应跳过");
        assert!(built.engine.get_node(id).is_none());
    }
}

/// 空 components: 空引擎, 不自动尺寸 (sizing=None, 窗口/renderOffset 保持
/// 宿主原状) — 原 Java components.isEmpty() 裸 return 分支。
#[test]
fn page_layout_empty_components_no_sizing() {
    let doc = PageDoc::default();
    let built = build_page(&doc, &|_| Some(true), 300, 200);
    assert!(built.cells.is_empty());
    assert!(built.engine.get_node("row0").is_none());
    assert!(built.sizing.is_none());
}

static EMPTY_TEST_ROWS: std::sync::LazyLock<std::collections::HashMap<String, std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>>>> =
    std::sync::LazyLock::new(std::collections::HashMap::new);
