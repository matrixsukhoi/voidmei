//! 列表容器排列黑盒场景: 排列策略纯函数 / 引擎容器子树求解 / 隐藏塌缩补位 /
//! 塌缩传播 / 嵌套容器 / props 解析。零像素零字体 — 只测布局计算面。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;

use overlay::layout::hud_layout_node::{
    Dimension, HasPreferredSize, HUDLayoutNode, HUDLayoutNodeExt, SharedNode,
};
use overlay::layout::list_arrange::{parse_list_props, ListArrange, ListMode};
use overlay::layout::minihud_layout::{
    HasVisibility, ModernHUDLayoutEngine,
};

// ---------------------------------------------------------------------
// 测试组件 (布局契约最小实现, 同 layout_calc.rs 形态)
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

/// 容器节点 (Column 缺省排列)
fn container(id: &str, arrange: ListArrange) -> SharedNode<VisComp> {
    let n = node(id, 0, 0); // 壳 preferred 0×0 (引擎 measure 接管)
    n.set_arrange(Some(arrange));
    n
}

fn rects(e: &ModernHUDLayoutEngine<VisComp>, ids: &[&str]) -> String {
    ids.iter()
        .map(|id| {
            let n = e.get_node(id).unwrap();
            let r = n.get_pixel_rect();
            format!("{id} x{} y{} w{} h{}", r.x, r.y, r.width, r.height)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_ids(e: &ModernHUDLayoutEngine<VisComp>) -> Vec<String> {
    let mut ids = Vec::new();
    e.render(|n, _, _, dbg| {
        if dbg.is_none() {
            ids.push(n.borrow().id.clone());
        }
    });
    ids
}

fn d(w: i32, h: i32) -> Dimension {
    Dimension::new(w, h)
}

/// 纯排列策略表: 四模式 × gap 换算 (line_height 10, gap 0.5 → 5px)
#[test]
fn list_arrange_四模式排列表() {
    let lh = 10.0;
    let sizes = [d(100, 10), d(80, 20), d(60, 10)];
    let column = ListArrange {
        mode: ListMode::Column,
        gap: 0.5,
    };
    let a = column.arrange(&sizes, lh);
    let actual = format!(
        "col: {:?} {}x{}",
        a.offsets, a.size.width, a.size.height
    );

    let columns = ListArrange {
        mode: ListMode::Columns(2),
        gap: 0.5,
    };
    let five = [d(10, 10); 5];
    let a2 = columns.arrange(&five, lh);
    let actual2 = format!(
        "cols2: {:?} {}x{}",
        a2.offsets, a2.size.width, a2.size.height
    );

    // 预算 25px (2.5 单位): 每列容 2 项 (10+5+10) → 三列
    let wrap = ListArrange {
        mode: ListMode::Wrap {
            max_height_units: 2.5,
        },
        gap: 0.5,
    };
    let a3 = wrap.arrange(&five, lh);
    let actual3 = format!(
        "wrap: {:?} {}x{}",
        a3.offsets, a3.size.width, a3.size.height
    );

    let grid = ListArrange {
        mode: ListMode::Grid {
            rows: 3,
            cols: 2,
        },
        gap: 0.5,
    };
    let a4 = grid.arrange(&five, lh);
    let actual4 = format!(
        "grid: {:?} {}x{}",
        a4.offsets, a4.size.width, a4.size.height
    );
    expect![[r#"
        col: [(0, 0), (0, 15), (0, 40)] 100x50
        cols2: [(0, 0), (0, 15), (0, 30), (15, 0), (15, 15)] 25x40
        wrap: [(0, 0), (0, 15), (15, 0), (15, 15), (30, 0)] 40x20
        grid: [(0, 0), (15, 0), (0, 15), (15, 15), (0, 30)] 25x40"#]]
    .assert_eq(&format!("{actual}\n{actual2}\n{actual3}\n{actual4}"));
}

/// 隐藏项 (高 0) 塌缩补位: 不占位不计间距, 内容随收缩
#[test]
fn list_arrange_隐藏项塌缩补位() {
    let a = ListArrange {
        mode: ListMode::Column,
        gap: 0.5,
    };
    let sizes = [d(100, 10), d(80, 0), d(60, 10)];
    let r = a.arrange(&sizes, 10.0);
    expect![[r#"
        offsets [(0, 0), (0, 15), (0, 15)]
        size 100x25"#]]
    .assert_eq(&format!(
        "offsets {:?}\nsize {}x{}",
        r.offsets, r.size.width, r.size.height
    ));
}

/// 引擎求解: 容器子树接管 (子项 pos/anchor 忽略, 顺序语义堆叠)
#[test]
fn engine_容器子树求解() {
    let mut e = ModernHUDLayoutEngine::<VisComp>::new(200, 200);
    let block = container(
        "block",
        ListArrange {
            mode: ListMode::Column,
            gap: 0.0,
        },
    );
    let a = node("a", 50, 20);
    let b = node("b", 40, 20);
    let c = node("c", 30, 10);
    a.set_parent(Some(&block));
    b.set_parent(Some(&block));
    c.set_parent(Some(&block));
    // 子项 pos/anchor 故意设非零 — 容器语义下应被忽略
    b.set_relative_position(9.0, 9.0);
    c.set_anchors(overlay::layout::anchor::Anchor::BottomRight, overlay::layout::anchor::Anchor::BottomRight);
    e.add_node(block);
    e.add_node(a);
    e.add_node(b);
    e.add_node(c);
    e.do_layout();

    expect![[r#"
        block x0 y0 w50 h50
        a x0 y0 w50 h20
        b x0 y20 w40 h20
        c x0 y40 w30 h10"#]]
    .assert_eq(&rects(&e, &["block", "a", "b", "c"]));

    // 渲染序 = 前序 (容器先, 子项按挂载序)
    expect![[r#"
        block
        a
        b
        c"#]]
    .assert_eq(&render_ids(&e).join("\n"));
}

/// 引擎隐藏塌缩: 中项不可见 → 0×0 补位, 包围盒随收缩
#[test]
fn engine_容器隐藏塌缩与补位() {
    let mut e = ModernHUDLayoutEngine::<VisComp>::new(200, 200);
    let block = container(
        "block",
        ListArrange {
            mode: ListMode::Column,
            gap: 0.0,
        },
    );
    let a = node("a", 50, 20);
    let b = node("b", 40, 20);
    let c = node("c", 30, 10);
    a.set_parent(Some(&block));
    b.set_parent(Some(&block));
    c.set_parent(Some(&block));
    e.add_node(block.clone());
    e.add_node(a);
    e.add_node(b.clone());
    e.add_node(c);
    b.borrow_mut().component.visible = false;
    e.do_layout();

    expect![[r#"
        block x0 y0 w50 h30
        a x0 y0 w50 h20
        c x0 y20 w30 h10"#]]
    .assert_eq(&rects(&e, &["block", "a", "c"]));

    let bounds = e.get_content_bounds();
    expect!["content (0,0) 50x30"].assert_eq(&format!(
        "content ({},{}) {}x{}",
        bounds.x, bounds.y, bounds.width, bounds.height
    ));
}

/// 容器自身不可见 → 整块塌缩: 后代从渲染序与包围盒剔除
#[test]
fn engine_容器不可见整块塌缩() {
    let mut e = ModernHUDLayoutEngine::<VisComp>::new(200, 200);
    let block = container(
        "block",
        ListArrange {
            mode: ListMode::Column,
            gap: 0.0,
        },
    );
    let a = node("a", 50, 20);
    let b = node("b", 40, 20);
    a.set_parent(Some(&block));
    b.set_parent(Some(&block));
    e.add_node(block.clone());
    e.add_node(a);
    e.add_node(b);
    block.borrow_mut().component.visible = false;
    e.do_layout();

    // 子项 cell 可见但被塌缩传播剔除 — 渲染序全空
    expect![""].assert_eq(&render_ids(&e).join(","));
    let bounds = e.get_content_bounds();
    expect!["fallback 1x1"].assert_eq(&format!(
        "fallback {}x{}",
        bounds.width, bounds.height
    ));
}

/// 嵌套容器: 外层单列含内层两列容器 + 叶子; 内层尺寸参与外层排列
#[test]
fn engine_嵌套容器() {
    let mut e = ModernHUDLayoutEngine::<VisComp>::new(200, 200);
    let outer = container(
        "outer",
        ListArrange {
            mode: ListMode::Column,
            gap: 0.0,
        },
    );
    let inner = container(
        "inner",
        ListArrange {
            mode: ListMode::Columns(2),
            gap: 0.0,
        },
    );
    let p = node("p", 10, 10);
    let q = node("q", 20, 20);
    let r = node("r", 15, 10);
    let leaf = node("leaf", 60, 10);
    p.set_parent(Some(&inner));
    q.set_parent(Some(&inner));
    r.set_parent(Some(&inner));
    inner.set_parent(Some(&outer));
    leaf.set_parent(Some(&outer));
    e.add_node(outer);
    e.add_node(inner);
    e.add_node(p);
    e.add_node(q);
    e.add_node(r);
    e.add_node(leaf);
    e.do_layout();

    // inner 内容 = 两列 [p,q | r] → 35x30; outer = [inner, leaf] → 60x40
    expect![[r#"
        outer x0 y0 w60 h40
        inner x0 y0 w35 h30
        p x0 y0 w10 h10
        q x0 y10 w20 h20
        r x20 y0 w15 h10
        leaf x0 y30 w60 h10"#]]
    .assert_eq(&rects(&e, &["outer", "inner", "p", "q", "r", "leaf"]));
}

/// props 解析: 模式键/参数键/缺省宽容退化
#[test]
fn parse_props_排列解析() {
    let show = |v: &str| {
        let p: serde_json::Value = serde_json::from_str(v).unwrap();
        let a = parse_list_props(&p);
        format!("{:?} gap{}", a.mode, a.gap)
    };
    expect![[r#"
        Column gap0
        Columns(3) gap0
        Grid { rows: 2, cols: 4 } gap0
        Wrap { max_height_units: 8.5 } gap0.25
        Column gap0
        Columns(8) gap0"#]]
    .assert_eq(
        &[
            show(r#"{}"#),
            show(r#"{"arrange":"columns","columns":3}"#),
            show(r#"{"arrange":"grid","gridCols":4}"#),
            show(r#"{"arrange":"wrap","wrapHeight":8.5,"gap":0.25}"#),
            show(r#"{"arrange":"weird"}"#),
            show(r#"{"arrange":"columns","columns":99}"#),
        ]
        .join("\n"),
    );
}
