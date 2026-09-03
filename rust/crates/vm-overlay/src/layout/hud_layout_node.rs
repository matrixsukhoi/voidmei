//! HUD 布局图节点 (Java HUDLayoutNode 一比一翻译, B 类)。
//!
//! 映射裁决:
//! - Java 引用语义 (节点被 engine map / 父 children / 局部变量共享引用) →
//!   `Rc<RefCell<HUDLayoutNode<T>>>` 共享句柄 (Java 本类无线程同步, 布局图仅在
//!   渲染线程触碰 → Rc 而非 Arc, 见 LIFETIMES.md §5 "UI 单写者")。
//! - 父子双向边: children 强持 Rc (父拥有子), parent 用 `Weak` 反向回指 ——
//!   Java 的 GC 环 (parent↔child) 在 Rust 显式断环; 正常图中父节点必被
//!   engine map 或自身祖先的 children 强持, Weak 升级不会失败。
//!   PORT(审查 B1/B2 备案): (a) 升级失败时 get_parent() 告警并返回 None,
//!   engine 以 get_parent()==None 判根 (ModernHUDLayoutEngine) —
//!   engine 移植必须让 nodes map 强持全部节点, 否则节点被误判为 ROOT;
//!   (b) 重复 setParent 可构造 A↔B children 强引用环, Rc 永不回收 (Java GC
//!   可收环 + engine visit() 环检测仅日志跳过, 见 ModernHUDLayoutEngine)
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
    /// pixelRect 全 0 / dirty true。
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

    /// Java `getParent()` (null → None)
    fn get_parent(&self) -> Option<SharedNode<T>>;

    /// Java `getChildren()`
    /// PORT: Java 返回 live 可变 List; Rust 返回 Rc 快照 Vec
    /// (全库消费点只遍历, 遍历期改图需重取)。
    fn get_children(&self) -> Vec<SharedNode<T>>;

    /// Java `getPixelRect()`
    /// PORT: Java 返回 live Rectangle 引用; 全库消费点 (ModernHUDLayoutEngine
    /// 4 处 — 审查勘误, 翻译者报告误记 3 处) 均只读
    /// → 返回 Copy 快照。
    fn get_pixel_rect(&self) -> Rectangle;

    /// Java `solve(double lineHeight, Rectangle parentRect)`
    fn solve(&self, line_height: f64, parent_rect: &Rectangle)
    where
        T: HasPreferredSize;
}

impl<T> HUDLayoutNodeExt<T> for SharedNode<T> {
    fn set_parent(&self, parent: Option<&SharedNode<T>>) -> SharedNode<T> {
        let old = self.borrow_mut().parent.take().and_then(|w| w.upgrade());
        if let Some(old_parent) = old {
            // List.remove(Object) 按同一性 (默认 equals) 删首个匹配
            // → Rc::ptr_eq 找首个, 与 retain(全删) 语义不同。
            let mut pc = old_parent.borrow_mut();
            if let Some(idx) = pc.children.iter().position(|c| Rc::ptr_eq(c, self)) {
                pc.children.remove(idx);
            }
        }
        self.borrow_mut().parent = parent.map(Rc::downgrade);
        if let Some(p) = parent {
            p.borrow_mut().children.push(self.clone());
        }
        self.clone()
    }

    fn set_relative_position(&self, unit_x: f64, unit_y: f64) -> SharedNode<T> {
        let mut this = self.borrow_mut();
        this.unit_x = unit_x;
        this.unit_y = unit_y;
        drop(this);
        self.clone()
    }

    fn set_anchors(&self, parent_anchor: Anchor, self_anchor: Anchor) -> SharedNode<T> {
        let mut this = self.borrow_mut();
        this.parent_anchor = parent_anchor;
        this.self_anchor = self_anchor;
        drop(this);
        self.clone()
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
                vm_core::base::logger::warn(
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
        // (Java 此处有 LayoutDebug info 日志, 未复刻)
        this.dirty = false;
    }
}

#[cfg(test)]
mod tests;
