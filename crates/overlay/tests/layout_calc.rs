//! 布局域黑盒场景 (overlay::layout): RenderCtx 派生几何 / Anchor 方位 /
//! HUD 布局节点树协商 / ModernHUDLayoutEngine 拓扑求解 / DPI 常量表。
//! 零像素零字体 — 只测纯布局计算面 (渲染质量归人工验收)。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;

use overlay::layout::anchor::Anchor;
use overlay::layout::hud_layout_node::{
    Dimension, HasPreferredSize, HUDLayoutNode, HUDLayoutNodeExt, Rectangle,
};
use overlay::layout::minihud_layout::{
    debug_frame_color, java_string_hashcode, HasVisibility, ModernHUDLayoutEngine,
};
use overlay::layout::ui_constants as uc;
use overlay::layout::RenderCtx;

// ---------------------------------------------------------------------
// 测试组件 (布局契约的最小实现 — HasPreferredSize/HasVisibility 两 seam)
// ---------------------------------------------------------------------

/// 固定尺寸 + 可见开关
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

/// 可变尺寸组件 (验证 doLayout 无条件重算 — Java 怪癖保真)
struct DynComp {
    size: std::rc::Rc<std::cell::Cell<(i32, i32)>>,
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

/// 固定尺寸节点速记
fn node(id: &str, w: i32, h: i32) -> overlay::layout::hud_layout_node::SharedNode<VisComp> {
    HUDLayoutNode::new(
        id,
        VisComp {
            w,
            h,
            visible: true,
        },
    )
}

/// 渲染回调收集 id (debug=None 的本体回调; 不触碰像素)
fn render_ids<T: HasPreferredSize + HasVisibility>(
    e: &ModernHUDLayoutEngine<T>,
) -> Vec<String> {
    let mut ids = Vec::new();
    e.render(|n, _, _, dbg| {
        if dbg.is_none() {
            ids.push(n.borrow().id.clone());
        }
    });
    ids
}

/// Anchor 九方位谓词表 (is_left/right/top/bottom/center_h/center_v)
#[test]
fn anchor_方位谓词表() {
    let all = [
        Anchor::TopLeft,
        Anchor::TopCenter,
        Anchor::TopRight,
        Anchor::MiddleLeft,
        Anchor::Center,
        Anchor::MiddleRight,
        Anchor::BottomLeft,
        Anchor::BottomCenter,
        Anchor::BottomRight,
    ];
    // 行首勿带差异化前导空格 (expect-test 公共前缀剥离会吃掉)
    let actual = all
        .iter()
        .map(|a| {
            format!(
                "{:?} L{} R{} T{} B{} CH{} CV{}",
                a,
                a.is_left(),
                a.is_right(),
                a.is_top(),
                a.is_bottom(),
                a.is_center_horizontal(),
                a.is_center_vertical(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        TopLeft Ltrue Rfalse Ttrue Bfalse CHfalse CVfalse
        TopCenter Lfalse Rfalse Ttrue Bfalse CHtrue CVfalse
        TopRight Lfalse Rtrue Ttrue Bfalse CHfalse CVfalse
        MiddleLeft Ltrue Rfalse Tfalse Bfalse CHfalse CVtrue
        Center Lfalse Rfalse Tfalse Bfalse CHtrue CVtrue
        MiddleRight Lfalse Rtrue Tfalse Bfalse CHfalse CVtrue
        BottomLeft Ltrue Rfalse Tfalse Btrue CHfalse CVfalse
        BottomCenter Lfalse Rfalse Tfalse Btrue CHtrue CVfalse
        BottomRight Lfalse Rtrue Tfalse Btrue CHfalse CVfalse"#]]
    .assert_eq(&actual);
}

/// RenderCtx 派生几何: Java RenderContext 的字号/步进/基线公式批量钉死
#[test]
fn render_ctx_派生几何表() {
    let rows: Vec<(i32, i32, i32)> = vec![
        (0, 2, 28),    // 缺省字号 24, 两列
        (4, 3, 30),    // +4 档
        (-8, 1, 20),   // 负档 (小字号)
    ];
    let actual = rows
        .iter()
        .map(|&(add, col, nh)| {
            let c = RenderCtx::new(add, col, nh);
            format!(
                "fs{} lfs{} tw{} th3/th4/th0 {}|{}|{} adv({}, {}) lw{} pad{} base(10){}/{}",
                c.font_size,
                c.label_font_size,
                c.total_width(),
                c.total_height(3),
                c.total_height(4),
                c.total_height(0),
                c.advance_x(),
                c.advance_y(),
                c.lwidth(),
                c.num_padding(),
                c.value_baseline(10),
                c.unit_baseline(10),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        fs24 lfs12 tw312 th3/th4/th0 112|112|56 adv(120, 28) lw78 pad6 base(10)22/22
        fs28 lfs14 tw504 th3/th4/th0 90|120|60 adv(140, 30) lw91 pad7 base(10)24/24
        fs16 lfs8 tw128 th3/th4/th0 100|120|40 adv(80, 20) lw52 pad4 base(10)18/18"#]]
    .assert_eq(&actual);
}

/// 节点求解: Self.Point(SelfAnchor) = Parent.Point(ParentAnchor) + Unit×LineHeight
#[test]
fn hud_node_锚点求解表() {
    // (父锚, 自锚, unitX, unitY) → 根节点相对 canvas (0,0,400,300) 的 pixelRect
    let cases: Vec<(Anchor, Anchor, f64, f64)> = vec![
        (Anchor::TopLeft, Anchor::TopLeft, 0.0, 0.0),
        (Anchor::TopLeft, Anchor::TopLeft, 1.5, -2.0),
        (Anchor::Center, Anchor::Center, 0.0, 0.0),
        (Anchor::BottomRight, Anchor::BottomRight, 0.0, 0.0),
        (Anchor::TopCenter, Anchor::BottomCenter, 0.0, 0.0),
        (Anchor::MiddleLeft, Anchor::TopRight, 2.0, 1.0),
    ];
    let actual = cases
        .iter()
        .map(|&(pa, sa, ux, uy)| {
            let n = node("t", 40, 20)
                .set_anchors(pa, sa)
                .set_relative_position(ux, uy);
            n.solve(20.0, &Rectangle::with_bounds(0, 0, 400, 300));
            let r = n.get_pixel_rect();
            format!("{pa:?}/{sa:?} u({ux},{uy}) -> x{} y{} w{} h{}", r.x, r.y, r.width, r.height)
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        TopLeft/TopLeft u(0,0) -> x0 y0 w40 h20
        TopLeft/TopLeft u(1.5,-2) -> x30 y-40 w40 h20
        Center/Center u(0,0) -> x180 y140 w40 h20
        BottomRight/BottomRight u(0,0) -> x360 y280 w40 h20
        TopCenter/BottomCenter u(0,0) -> x180 y-20 w40 h20
        MiddleLeft/TopRight u(2,1) -> x0 y170 w40 h20"#]]
    .assert_eq(&actual);
}

/// 父子协商: set_parent 双向建边 + 重挂摘旧 + 父矩形驱动子坐标
#[test]
fn hud_node_父子拓扑协商() {
    let root = node("root", 100, 100);
    let child = node("child", 20, 10);

    // 双向建边: 父 children 收编 + 子可回溯同一父句柄
    child.set_parent(Some(&root));
    assert_eq!(root.get_children().len(), 1);
    assert!(std::rc::Rc::ptr_eq(&child.get_parent().unwrap(), &root));
    // 无父节点 = 根
    assert!(root.get_parent().is_none());

    // 父矩形求解 → 子相对父锚定位
    root.solve(20.0, &Rectangle::with_bounds(0, 0, 500, 400));
    child
        .set_anchors(Anchor::TopRight, Anchor::TopLeft)
        .set_relative_position(0.0, 0.0);
    child.solve(20.0, &root.get_pixel_rect());
    let r = child.get_pixel_rect();
    expect!["x100 y0 w20 h10"].assert_eq(&format!("x{} y{} w{} h{}", r.x, r.y, r.width, r.height));

    // 重挂: 旧父 children 摘除 (List.remove 同一性语义)
    let root2 = node("root2", 50, 50);
    child.set_parent(Some(&root2));
    assert_eq!(root.get_children().len(), 0, "重挂后旧父应摘除");
    assert_eq!(root2.get_children().len(), 1);
    // 摘父: None 清空
    child.set_parent(None);
    assert!(child.get_parent().is_none());
    assert_eq!(root2.get_children().len(), 0);
}

/// 拓扑序: 前序 DFS 父先子后, 同层按挂载序; add 顺序与树结构无关
#[test]
fn engine_拓扑序_父先子后() {
    let mut e = ModernHUDLayoutEngine::new(100, 100);
    let root = node("root", 10, 10);
    let c1 = node("c1", 10, 10).set_parent(Some(&root));
    let c2 = node("c2", 10, 10).set_parent(Some(&root));
    let g = node("g", 10, 10).set_parent(Some(&c1));
    // add 顺序打乱 (Java HashMap 语义, 唯一根 = root)
    e.add_node(c2.clone());
    e.add_node(root.clone());
    e.add_node(g.clone());
    e.add_node(c1.clone());
    e.do_layout();

    // root.children 挂载序 [c1, c2] → root, c1, (c1 的子) g, c2
    let order = render_ids(&e);
    expect![[r#"
        root
        c1
        g
        c2"#]]
    .assert_eq(&order.join("\n"));

    // 不可见节点从渲染序剔除 (布局门控 — 组件可见性在负载上)
    c2.borrow_mut().component.visible = false;
    let order2 = render_ids(&e);
    expect![[r#"
        root
        c1
        g"#]]
    .assert_eq(&order2.join("\n"));
}

/// 画布平移 (根节点随 canvasRect) + lineHeight 的 0.001 阈值 + 无条件重算
#[test]
fn engine_画布平移与line_height阈值() {
    let mut e = ModernHUDLayoutEngine::<VisComp>::new(100, 100);
    let n = node("r", 10, 10).set_relative_position(1000.0, 0.0);
    e.add_node(n.clone());
    e.do_layout();
    expect!["x20000"].assert_eq(&format!("x{}", n.get_pixel_rect().x));

    // |Δ|<=0.001 不接受 (Java L52)
    e.set_line_height(20.0005);
    e.do_layout();
    expect!["x20000"].assert_eq(&format!("x{}", n.get_pixel_rect().x));
    // |Δ|>0.001 接受
    e.set_line_height(20.002);
    e.do_layout();
    expect!["x20002"].assert_eq(&format!("x{}", n.get_pixel_rect().x));

    // 画布原点平移 → 根节点 (锚 canvasRect) 坐标随动
    e.set_canvas_origin(5, 7);
    e.do_layout();
    expect!["x20007 y7"].assert_eq(&format!(
        "x{} y{}",
        n.get_pixel_rect().x, n.get_pixel_rect().y
    ));
}

/// doLayout 的 Java 怪癖: dirty 清零后仍无条件重算 (组件尺寸变化免置脏)
#[test]
fn engine_干净态也重算() {
    let size = std::rc::Rc::new(std::cell::Cell::new((10, 10)));
    let n = HUDLayoutNode::new("dyn", DynComp { size: size.clone() });
    let mut e = ModernHUDLayoutEngine::new(200, 200);
    e.add_node(n.clone());
    e.do_layout();
    expect!["w10 h10"].assert_eq(&format!(
        "w{} h{}",
        n.get_pixel_rect().width, n.get_pixel_rect().height
    ));

    // 不置脏, 直接改组件尺寸 → 下次 do_layout 仍生效
    size.set((33, 22));
    e.do_layout();
    expect!["w33 h22"].assert_eq(&format!(
        "w{} h{}",
        n.get_pixel_rect().width, n.get_pixel_rect().height
    ));
}

/// 自动尺寸计划: 可见节点包围盒 + padding 双侧留白 + 居中偏移
#[test]
fn engine_自动尺寸计划() {
    let mut e = ModernHUDLayoutEngine::<VisComp>::new(400, 300);
    let a = node("a", 50, 40).set_relative_position(1.0, 1.0); // (20,20)
    let b = node("b", 30, 20).set_relative_position(5.0, 8.0); // (100,160)
    e.add_node(a);
    e.add_node(b);
    // 不可见节点不参与包围盒
    let c = node("c", 10, 10).set_relative_position(0.0, 0.0);
    c.borrow_mut().component.visible = false;
    e.add_node(c);

    let plan = e.apply_auto_sizing(45);
    // 内容包围盒 (20,20)-(130,180) → 110x160; 窗口 = 内容 + 2×padding; 偏移 = padding − 内容原点
    expect![[r#"
        window 200x250
        offset (25, 25)
        content (20, 20) 110x160"#]]
    .assert_eq(&format!(
        "window {}x{}\noffset ({}, {})\ncontent ({}, {}) {}x{}",
        plan.new_width,
        plan.new_height,
        plan.offset_x,
        plan.offset_y,
        plan.content_x,
        plan.content_y,
        plan.content_w,
        plan.content_h
    ));
}

/// 全部不可见 → 包围盒退化为 1x1 (Java 防 0 尺寸窗口)
#[test]
fn engine_空内容包围盒退化() {
    let mut e = ModernHUDLayoutEngine::<VisComp>::new(100, 100);
    let a = node("a", 50, 40);
    a.borrow_mut().component.visible = false;
    e.add_node(a);
    e.do_layout();
    let b = e.get_content_bounds();
    expect!["0 0 1 1"].assert_eq(&format!("{} {} {} {}", b.x, b.y, b.width, b.height));
}

/// ui_constants 常量表 (UIConstants.java 的 Rust 移植快照)
#[test]
fn ui_constants_常量表() {
    let actual = format!(
        "BASE_SCREEN_HEIGHT {}\nBASE_FONT_SIZE {}\nWIDTH_MULTIPLIER {}\nHEIGHT_MULTIPLIER {}\nMAX_AOA {}\nMAX_AOS {}\nATTITUDE {}x{} {}ms\nENGINE {} {}x{}\nDELAY {}/{}/{}\nALPHA {}/{}\nSPACING {}/{}/{}",
        uc::BASE_SCREEN_HEIGHT,
        uc::BASE_FONT_SIZE,
        uc::WIDTH_MULTIPLIER,
        uc::HEIGHT_MULTIPLIER,
        uc::MAX_AOA,
        uc::MAX_AOS,
        uc::ATTITUDE_BASE_WIDTH,
        uc::ATTITUDE_BASE_HEIGHT,
        uc::ATTITUDE_REFRESH_MS,
        uc::ENGINE_BASE_FONT_SIZE,
        uc::ENGINE_WIDTH_MULTIPLIER,
        uc::ENGINE_SHADE_WIDTH,
        uc::DELAY_SHORT_MS,
        uc::DELAY_MEDIUM_MS,
        uc::DELAY_LONG_MS,
        uc::DEFAULT_ALPHA,
        uc::SEMI_TRANSPARENT_ALPHA,
        uc::SPACING_SMALL,
        uc::SPACING_MEDIUM,
        uc::SPACING_LARGE,
    );
    expect![[r#"
        BASE_SCREEN_HEIGHT 1440
        BASE_FONT_SIZE 16
        WIDTH_MULTIPLIER 36
        HEIGHT_MULTIPLIER 72
        MAX_AOA 30
        MAX_AOS 15
        ATTITUDE 100x200 40ms
        ENGINE 24 8x10
        DELAY 100/500/1000
        ALPHA 255/128
        SPACING 5/10/20"#]]
    .assert_eq(&actual);
}

/// Java String.hashCode 基线 (Java 8 实测值) + 调试框色拆色
#[test]
fn hashcode与调试框色() {
    let hashes = ["", "a", "abc", "row0", "row1", "speedBar"]
        .iter()
        .map(|s| format!("{s} = {}", java_string_hashcode(s)))
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
         = 0
        a = 97
        abc = 96354
        row0 = 3506582
        row1 = 3506583
        speedBar = -2131211700"#]]
    .assert_eq(&hashes);

    // drawDebug 框色: hash 低 24 位拆 RGB, 暗色提亮 +100
    let colors = ["row0", "attitude", "speedBar"]
        .iter()
        .map(|s| format!("{s} = {:?}", debug_frame_color(s)))
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        row0 = [153, 229, 250, 255]
        attitude = [249, 136, 255, 255]
        speedBar = [248, 74, 76, 255]"#]]
    .assert_eq(&colors);
}
