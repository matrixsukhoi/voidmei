//! 对应 Java: `src/ui/layout/HUDLayoutNode.java` (一比一翻译, B 类)。
//!
//! 映射裁决:
//! - Java 引用语义 (节点被 engine map / 父 children / 局部变量共享引用) →
//!   `Rc<RefCell<HUDLayoutNode<T>>>` 共享句柄 (Java 本类无线程同步, 布局图仅在
//!   EDT 触碰 → Rc 而非 Arc, 见 LIFETIMES.md §5 "UI 单写者")。
//! - 父子双向边: children 强持 Rc (父拥有子), parent 用 `Weak` 反向回指 ——
//!   Java 的 GC 环 (parent↔child) 在 Rust 显式断环; 正常图中父节点必被
//!   engine map 或自身祖先的 children 强持, Weak 升级不会失败。
//!   PORT(审查 B1/B2 备案): (a) 升级失败时 get_parent() 告警并返回 None,
//!   engine 以 get_parent()==None 判根 (ModernHUDLayoutEngine.java:102) —
//!   engine 移植必须让 nodes map 强持全部节点, 否则节点被误判为 ROOT;
//!   (b) 重复 setParent 可构造 A↔B children 强引用环, Rc 永不回收 (Java GC
//!   可收环 + engine visit() 环检测仅日志跳过, ModernHUDLayoutEngine.java:111-114)
//!   — engine 移植 visit() 环检测分支时须同时负责断环。
//! - Java 实例方法 → `HUDLayoutNodeExt` 扩展 trait 挂在句柄上
//!   (`self: &Rc<RefCell<Self>>` 非稳定接收者, trait shim 保持
//!   `node.set_parent(..)` 调用形态与链式写法)。
//! - HUDComponent (C 类, 未移植) → 泛型负载 `T` 保持 + `HasPreferredSize`
//!   最小 seam (solve 只依赖接口的 getPreferredSize 契约)。
//! - java.awt.Dimension / java.awt.Rectangle → 本模块最小移植 (i32 字段),
//!   后续 ModernHUDLayoutEngine (C 类) 移植时复用。

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::layout::Anchor;

/// A node in the Modern HUD Layout graph.
/// Wraps a HUDComponent and defines its dependency-based positioning.
// PORT: Java 字段 `HUDComponent component` → 泛型 `T` (C 类接口未移植,
// 泛型负载保持); public final 字段保持 pub。
pub struct HUDLayoutNode<T> {
    // Identity
    pub id: String,
    pub component: T,

    // Topology
    // PORT: Java `HUDLayoutNode parent` (可空引用) → Option<Weak<..>>
    // (Weak 指向句柄 Rc 的内部类型 RefCell<HUDLayoutNode<T>>);
    // children 对应 `List<HUDLayoutNode>` (强引用表)。
    parent: Option<Weak<RefCell<HUDLayoutNode<T>>>>,
    children: Vec<SharedNode<T>>,

    // Unit-based Layout Specs (Scaling Invariant)
    unit_x: f64, // Relative to parent anchor
    unit_y: f64,
    parent_anchor: Anchor,
    self_anchor: Anchor,

    // Flags
    // PORT(审查 B4): 该标志在 Java 即 write-only 死状态 (全库无读者,
    // getContentBounds 不检查它), 保真保留 — 勿据注释名推断其已生效。
    ignore_bounds: bool, // If true, doesn't affect parent's bound calculation (Overlay)

    // Runtime State (Calculated)
    pixel_rect: Rectangle,
    dirty: bool,
}

/// HUDLayoutNode 的共享句柄 (Java 引用语义的 Rust 对应物)。
pub type SharedNode<T> = Rc<RefCell<HUDLayoutNode<T>>>;

/// java.awt.Dimension 的最小移植 (int width/height)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dimension {
    pub width: i32,
    pub height: i32,
}

impl Dimension {
    /// 对应 `new Dimension(width, height)`
    pub fn new(width: i32, height: i32) -> Self {
        Dimension { width, height }
    }
}

/// java.awt.Rectangle 的最小移植 (int x/y/width/height)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rectangle {
    /// 对应 `new Rectangle()` (全 0)
    pub fn new() -> Self {
        Rectangle::default()
    }

    /// 对应 `new Rectangle(x, y, width, height)`
    pub fn with_bounds(x: i32, y: i32, width: i32, height: i32) -> Self {
        Rectangle { x, y, width, height }
    }

    /// 对应 `setBounds(x, y, width, height)` (原地改写)
    pub fn set_bounds(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }
}

/// HUDComponent 接口 (C 类) 中布局所需的尺寸契约。
/// PORT: Java solve() 经 `component.getPreferredSize()` 取 java.awt.Dimension;
/// C 类 HUDComponent 重设计落地前以此最小 seam 解耦。
///
/// # 实现约束 (审查 B3)
/// `preferred_size()` 在 `solve()` 持有节点 `RefCell` 可变借用期间被调用 —
/// 实现不得回指节点图 (get_parent/get_children/set_parent 等句柄方法),
/// 否则 BorrowMutError panic (Java 无锁, 无此故障模式)。
pub trait HasPreferredSize {
    fn preferred_size(&self) -> Dimension;
}

impl<T> HUDLayoutNode<T> {
    /// Java 构造器 `new HUDLayoutNode(id, component)`;
    /// 直接返回共享句柄 (Java 引用即共享)。
    /// 字段缺省值对齐 Java 隐式初始化 (§2.10): unit 0.0 / 锚 TOP_LEFT /
    /// ignoreBounds false / pixelRect 全 0 / dirty true。
    pub fn new(id: impl Into<String>, component: T) -> SharedNode<T> {
        Rc::new(RefCell::new(HUDLayoutNode {
            id: id.into(),
            component,
            parent: None,
            children: Vec::new(),
            unit_x: 0.0,
            unit_y: 0.0,
            parent_anchor: Anchor::TopLeft,
            self_anchor: Anchor::TopLeft,
            ignore_bounds: false,
            pixel_rect: Rectangle::new(),
            dirty: true,
        }))
    }

    /// Java `private int getAnchorX(Rectangle rect, Anchor anchor)`
    fn get_anchor_x(rect: &Rectangle, anchor: Anchor) -> i32 {
        if anchor.is_left() {
            return rect.x;
        }
        if anchor.is_right() {
            return rect.x + rect.width;
        }
        rect.x + rect.width / 2
    }

    /// Java `private int getAnchorY(Rectangle rect, Anchor anchor)`
    fn get_anchor_y(rect: &Rectangle, anchor: Anchor) -> i32 {
        if anchor.is_top() {
            return rect.y;
        }
        if anchor.is_bottom() {
            return rect.y + rect.height;
        }
        rect.y + rect.height / 2
    }
}

/// Java HUDLayoutNode 实例方法面 → 句柄上的扩展 trait。
/// PORT: Rust 稳定接收者不含 `self: &Rc<RefCell<Self>>`, 借 trait impl
/// `Rc<RefCell<HUDLayoutNode<T>>>` 保持 `node.set_parent(..)` 方法调用形态。
pub trait HUDLayoutNodeExt<T> {
    /// Java `setParent(HUDLayoutNode)` (可空参数 → Option; `return this` → 返回句柄克隆)
    fn set_parent(&self, parent: Option<&SharedNode<T>>) -> SharedNode<T>;

    /// Java `setRelativePosition(double, double)` (`return this`)
    fn set_relative_position(&self, unit_x: f64, unit_y: f64) -> SharedNode<T>;

    /// Java `setAnchors(Anchor, Anchor)` (`return this`)
    fn set_anchors(&self, parent_anchor: Anchor, self_anchor: Anchor) -> SharedNode<T>;

    /// Java `setIgnoreBounds(boolean)` (`return this`)
    fn set_ignore_bounds(&self, ignore: bool) -> SharedNode<T>;

    /// Java `getParent()` (null → None)
    fn get_parent(&self) -> Option<SharedNode<T>>;

    /// Java `getChildren()`
    /// PORT: Java 返回 live 可变 List; Rust 返回 Rc 快照 Vec
    /// (全库消费点只遍历, 遍历期改图需重取)。
    fn get_children(&self) -> Vec<SharedNode<T>>;

    /// Java `getPixelRect()`
    /// PORT: Java 返回 live Rectangle 引用; 全库消费点 (ModernHUDLayoutEngine
    /// 4 处: L138/163/187/217 — 审查勘误, 翻译者报告误记 3 处) 均只读
    /// → 返回 Copy 快照。
    fn get_pixel_rect(&self) -> Rectangle;

    /// Java `solve(double lineHeight, Rectangle parentRect)`
    fn solve(&self, line_height: f64, parent_rect: &Rectangle)
    where
        T: HasPreferredSize;
}

impl<T> HUDLayoutNodeExt<T> for SharedNode<T> {
    fn set_parent(&self, parent: Option<&SharedNode<T>>) -> SharedNode<T> {
        // Java: if (this.parent != null) { this.parent.children.remove(this); }
        let old = self.borrow_mut().parent.take().and_then(|w| w.upgrade());
        if let Some(old_parent) = old {
            // List.remove(Object) 按同一性 (默认 equals) 删首个匹配
            // → Rc::ptr_eq 找首个, 与 retain(全删) 语义不同。
            let mut pc = old_parent.borrow_mut();
            if let Some(idx) = pc.children.iter().position(|c| Rc::ptr_eq(c, self)) {
                pc.children.remove(idx);
            }
        }
        // Java: this.parent = parent;
        self.borrow_mut().parent = parent.map(Rc::downgrade);
        // Java: if (parent != null) { parent.children.add(this); }
        if let Some(p) = parent {
            p.borrow_mut().children.push(self.clone());
        }
        self.clone() // Java: return this
    }

    fn set_relative_position(&self, unit_x: f64, unit_y: f64) -> SharedNode<T> {
        let mut this = self.borrow_mut();
        this.unit_x = unit_x;
        this.unit_y = unit_y;
        drop(this);
        self.clone() // Java: return this
    }

    fn set_anchors(&self, parent_anchor: Anchor, self_anchor: Anchor) -> SharedNode<T> {
        let mut this = self.borrow_mut();
        this.parent_anchor = parent_anchor;
        this.self_anchor = self_anchor;
        drop(this);
        self.clone() // Java: return this
    }

    fn set_ignore_bounds(&self, ignore: bool) -> SharedNode<T> {
        self.borrow_mut().ignore_bounds = ignore;
        self.clone() // Java: return this
    }

    fn get_parent(&self) -> Option<SharedNode<T>> {
        // PORT: Java 强引用直返 → Weak 升级 (正常图中父必被强持, 升级不失败)
        let weak = self.borrow().parent.clone();
        match &weak {
            Some(w) => w.upgrade().or_else(|| {
                // 审查 B1: 升级失败 = 父节点未被任何强引用持有 (engine 未收进
                // nodes map 等), 节点将被 engine 的 get_parent()==None 根判定
                // 误当 ROOT; Java 强引用下父被 child 保活, 不可能出现 —
                // 告警暴露违约而非静默降级。
                crate::logger::warn(
                    "HUDLayout",
                    &format!(
                        "节点 {} 的父节点已被 drop, get_parent() 退化为 None (调用方将按 ROOT 处理)",
                        self.borrow().id
                    ),
                );
                None
            }),
            None => None,
        }
    }

    fn get_children(&self) -> Vec<SharedNode<T>> {
        self.borrow().children.clone()
    }

    fn get_pixel_rect(&self) -> Rectangle {
        self.borrow().pixel_rect
    }

    fn solve(&self, line_height: f64, parent_rect: &Rectangle)
    where
        T: HasPreferredSize,
    {
        let mut this = self.borrow_mut();
        let size = this.component.preferred_size(); // Assuming component has valid size

        // 1. Determine Target Point (on Parent)
        let mut target_x = HUDLayoutNode::<T>::get_anchor_x(parent_rect, this.parent_anchor);
        let mut target_y = HUDLayoutNode::<T>::get_anchor_y(parent_rect, this.parent_anchor);

        // 2. Add Unit Offset
        // PORT: Java `(int)(unitX * lineHeight)` = JLS 5.1.3 向零截断 + 饱和
        // (NaN→0, 越界→MIN/MAX); Rust `as i32` 同语义, 一致。
        target_x += (this.unit_x * line_height) as i32;
        target_y += (this.unit_y * line_height) as i32;
        // PORT(审查 A3 备案): 本函数其余 i32 加减 Java 静默回绕 / Rust debug 下
        // panic; 坐标像素域不达 i32 边界, §2.2 仅要求 hash/时间差/大数累加类用
        // wrapping — 保持原生运算, 与 Java 同域。

        // 3. Determine Self Origin (Top-Left) based on Self Anchor aligning to Target
        // Point
        // If SelfAnchor is CENTER, then TopLeft = Target - Width/2, Target - Height/2
        let mut self_x = target_x;
        let mut self_y = target_y;

        if this.self_anchor.is_center_horizontal() {
            self_x -= size.width / 2;
        } else if this.self_anchor.is_right() {
            self_x -= size.width;
        }

        if this.self_anchor.is_center_vertical() {
            self_y -= size.height / 2;
        } else if this.self_anchor.is_bottom() {
            self_y -= size.height;
        }

        this.pixel_rect
            .set_bounds(self_x, self_y, size.width, size.height);
        // prog.util.Logger.info("LayoutDebug", String.format("Node %s: Anchor(%.1f,
        // %.1f) -> Target(%d, %d) -> Self(%d, %d) [Size: %dx%d]",
        // id, unitX, unitY, targetX, targetY, selfX, selfY, size.width, size.height));
        this.dirty = false;
    }
}

#[cfg(test)]
mod tests {
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
            assert!(!g.ignore_bounds);
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
            .set_anchors(Anchor::BottomRight, Anchor::Center)
            .set_ignore_bounds(true);
        assert!(Rc::ptr_eq(&ret, &n));
        let g = n.borrow();
        assert_eq!((g.unit_x, g.unit_y), (1.5, -2.0));
        assert_eq!(g.parent_anchor, Anchor::BottomRight);
        assert_eq!(g.self_anchor, Anchor::Center);
        assert!(g.ignore_bounds);
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
            assert_eq!(n.get_pixel_rect(), expect, "{parent_anchor:?}/{self_anchor:?}");
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
        let n = HUDLayoutNode::new("t", fixed(31, 11))
            .set_anchors(Anchor::TopLeft, Anchor::Center);
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
}
