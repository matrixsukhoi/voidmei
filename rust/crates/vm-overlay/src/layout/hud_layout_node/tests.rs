use super::*;
use std::cell::Cell;

/// 测试组件: 固定尺寸 (对齐 HUDComponent.getPreferredSize 契约)
struct FixedComp {
    w: i32,
    h: i32,
}

impl HasPreferredSize for FixedComp {
    fn preferred_size(&self) -> Dimension {
        Dimension::new(self.w, self.h)
    }
}

/// 测试组件: 尺寸可变 (验证 solve 每次重读 getPreferredSize)
struct SharedComp {
    size: Rc<Cell<(i32, i32)>>,
}

impl HasPreferredSize for SharedComp {
    fn preferred_size(&self) -> Dimension {
        let (w, h) = self.size.get();
        Dimension::new(w, h)
    }
}

fn fixed(w: i32, h: i32) -> FixedComp {
    FixedComp { w, h }
}

/// Java 字段缺省值 (隐式初始化) 的直译核对 (§2.10)。
#[test]
fn initial_defaults_match_java() {
    let n = HUDLayoutNode::new("gauge", fixed(30, 10));
    assert!(n.get_parent().is_none());
    assert!(n.get_children().is_empty());
    {
        let g = n.borrow();
        assert_eq!(g.id, "gauge");
        assert_eq!((g.unit_x, g.unit_y), (0.0, 0.0));
        assert_eq!(g.parent_anchor, Anchor::TopLeft);
        assert_eq!(g.self_anchor, Anchor::TopLeft);
        assert_eq!(g.pixel_rect, Rectangle::new()); // new Rectangle() = 全 0
        assert!(g.dirty);
    }
}

/// setParent 双向拓扑维护: 挂载/改挂/摘除, `return this` 返回同一节点。
#[test]
fn set_parent_maintains_bidirectional_topology() {
    let root_a = HUDLayoutNode::new("a", fixed(10, 10));
    let root_b = HUDLayoutNode::new("b", fixed(10, 10));
    let child = HUDLayoutNode::new("c", fixed(10, 10));

    // 挂到 root_a
    let ret = child.set_parent(Some(&root_a));
    assert!(Rc::ptr_eq(&ret, &child)); // return this
    assert!(Rc::ptr_eq(&child.get_parent().unwrap(), &root_a));
    assert_eq!(root_a.get_children().len(), 1);
    assert!(Rc::ptr_eq(&root_a.get_children()[0], &child));

    // 改挂 root_b: 旧父 children 移除, 新父追加
    child.set_parent(Some(&root_b));
    assert_eq!(root_a.get_children().len(), 0);
    assert_eq!(root_b.get_children().len(), 1);
    assert!(Rc::ptr_eq(&child.get_parent().unwrap(), &root_b));

    // 摘除 (setParent(null))
    child.set_parent(None);
    assert!(child.get_parent().is_none());
    assert!(root_b.get_children().is_empty());
}

/// Java List.remove(Object) 只删首个匹配 (同一性比较);
/// 畸形图中 children 存重复时不得全删 (retain 语义是错的)。
#[test]
fn set_parent_removes_only_first_occurrence() {
    let root = HUDLayoutNode::new("root", fixed(10, 10));
    let child = HUDLayoutNode::new("c", fixed(10, 10));
    child.set_parent(Some(&root));
    // 人为制造重复项 (Java 侧畸形使用同样可造出)
    root.borrow_mut().children.push(child.clone());
    assert_eq!(root.get_children().len(), 2);

    child.set_parent(None);
    assert_eq!(root.get_children().len(), 1);
    assert!(Rc::ptr_eq(&root.get_children()[0], &child));
}

/// 链式 setter 状态落地, 且每环返回同一节点。
#[test]
fn fluent_setters_return_self_and_apply() {
    let n = HUDLayoutNode::new("x", fixed(5, 5));
    let ret = n
        .set_relative_position(1.5, -2.0)
        .set_anchors(Anchor::BottomRight, Anchor::Center);
    assert!(Rc::ptr_eq(&ret, &n));
    let g = n.borrow();
    assert_eq!((g.unit_x, g.unit_y), (1.5, -2.0));
    assert_eq!(g.parent_anchor, Anchor::BottomRight);
    assert_eq!(g.self_anchor, Anchor::Center);
}

/// solve 锚点数学的 Java oracle 对拍。
/// 公共参数: parentRect=(100,200,50,40), lineHeight=20, unit=(0.5,-1.0),
/// size=(30,10) → 偏移 (int)(0.5*20)=10, (int)(-1.0*20)=-20。
#[test]
fn solve_anchor_math_matches_java() {
    let cases: [(Anchor, Anchor, f64, f64, Rectangle); 5] = [
        // (parentAnchor, selfAnchor, unitX, unitY, 期望 rect)
        // TopLeft+Center: target=(100,200)+off=(110,180);
        //   中心锚: x=110-30/2=95, y=180-10/2=175
        (
            Anchor::TopLeft,
            Anchor::Center,
            0.5,
            -1.0,
            Rectangle::with_bounds(95, 175, 30, 10),
        ),
        // BottomRight+TopLeft: target=(150,240)+off=(160,220); 无自锚修正
        (
            Anchor::BottomRight,
            Anchor::TopLeft,
            0.5,
            -1.0,
            Rectangle::with_bounds(160, 220, 30, 10),
        ),
        // TopRight+BottomLeft: target=(150,200)+off=(160,180);
        //   底锚: y=180-10=170, 左锚: x 不变
        (
            Anchor::TopRight,
            Anchor::BottomLeft,
            0.5,
            -1.0,
            Rectangle::with_bounds(160, 170, 30, 10),
        ),
        // Center+TopCenter: target=(100+50/2, 200+40/2)=(125,220)+off=(135,200);
        //   水平居中: x=135-15=120, 顶锚: y 不变
        (
            Anchor::Center,
            Anchor::TopCenter,
            0.5,
            -1.0,
            Rectangle::with_bounds(120, 200, 30, 10),
        ),
        // MiddleRight+MiddleLeft, unit=(0,0): target=(150,220);
        //   垂直居中: y=220-10/2=215, 左锚: x 不变
        (
            Anchor::MiddleRight,
            Anchor::MiddleLeft,
            0.0,
            0.0,
            Rectangle::with_bounds(150, 215, 30, 10),
        ),
    ];
    for (parent_anchor, self_anchor, ux, uy, expect) in cases {
        let n = HUDLayoutNode::new("t", fixed(30, 10))
            .set_relative_position(ux, uy)
            .set_anchors(parent_anchor, self_anchor);
        n.solve(20.0, &Rectangle::with_bounds(100, 200, 50, 40));
        assert_eq!(
            n.get_pixel_rect(),
            expect,
            "{parent_anchor:?}/{self_anchor:?}"
        );
        assert!(!n.borrow().dirty); // solve 尾部置 false
    }
}

/// §2.4 oracle: Java `(int)` double 强转 = 向零截断 (非 floor)。
/// 0.04*20=0.8 → 0; -0.04*20=-0.8 → 0 (floor 会给 -1, 是错的)。
/// 0.09*20=1.8 → 1; -0.09*20=-1.8 → -1。
#[test]
fn solve_truncates_unit_offset_toward_zero() {
    let n = HUDLayoutNode::new("t", fixed(5, 5))
        .set_relative_position(0.04, -0.04)
        .set_anchors(Anchor::TopLeft, Anchor::TopLeft);
    n.solve(20.0, &Rectangle::with_bounds(10, 10, 100, 100));
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(10, 10, 5, 5));

    n.set_relative_position(0.09, -0.09);
    n.solve(20.0, &Rectangle::with_bounds(10, 10, 100, 100));
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(11, 9, 5, 5));
}

/// 奇数尺寸的整数除法: Java `31/2`=15, `11/2`=5 (向零截断)。
/// 零尺寸父矩形 + Center 自锚 → 原点减半宽/半高 (负坐标)。
#[test]
fn solve_odd_size_integer_division() {
    let n = HUDLayoutNode::new("t", fixed(31, 11)).set_anchors(Anchor::TopLeft, Anchor::Center);
    n.solve(20.0, &Rectangle::with_bounds(0, 0, 0, 0));
    // selfX = 0 - 31/2 = -15; selfY = 0 - 11/2 = -5
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(-15, -5, 31, 11));
}

/// solve 每次调用重读 component.getPreferredSize() (尺寸变化 → rect 更新)。
#[test]
fn solve_rereads_preferred_size_each_call() {
    let size = Rc::new(Cell::new((10, 10)));
    let n = HUDLayoutNode::new(
        "t",
        SharedComp {
            size: Rc::clone(&size),
        },
    )
    .set_anchors(Anchor::TopLeft, Anchor::Center);
    n.solve(20.0, &Rectangle::with_bounds(0, 0, 0, 0));
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(-5, -5, 10, 10));

    size.set((40, 20));
    n.solve(20.0, &Rectangle::with_bounds(0, 0, 0, 0));
    assert_eq!(n.get_pixel_rect(), Rectangle::with_bounds(-20, -10, 40, 20));
}

/// 审查 A1/B1: parent 仅 Weak — 父节点唯一强引用 drop 后 get_parent()
/// 返回 None (Java 强引用会保活父节点, 已声明的映射差异); 升级失败走
/// WARN 兜底不 panic。engine 移植须以 nodes map 强持全部节点规避此态。
#[test]
fn get_parent_returns_none_when_parent_dropped() {
    let child;
    {
        let parent = HUDLayoutNode::new("p", fixed(10, 10));
        child = HUDLayoutNode::new("c", fixed(10, 10)).set_parent(Some(&parent));
        assert!(child.get_parent().is_some()); // 父存活: 正常升级
    } // parent 唯一强引用随作用域结束 drop
    assert!(child.get_parent().is_none()); // 违约态: WARN + None, 不 panic
}
