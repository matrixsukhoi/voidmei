//! minihud_layout: MiniHUD 拓扑相对布局引擎 (C 类语义复刻)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`ModernHUDLayoutEngine`] | src/ui/layout/ModernHUDLayoutEngine.java | DAG 依赖解析: DFS 前序拓扑排序 (根=无父节点, 父先子后) + 惰性 dirty 布局 + 可见节点包围盒/自动尺寸 |
//!
//! W2 组件化: 原 MINIHUD_NODE_SPECS 常量拓扑 + build_mihud_layout 建树族退役 —
//! 组件树生成数据化为 PageDoc 驱动, 见 [`crate::widgets::page_layout`]
//! (出厂页 factory_default.json 的 minihud-default, 逐值复刻原常量表)。
//!
//! ui_layout.cfg panel "MiniHUD" 段的常量表快照 (原 MINIHUD_PANEL_ITEMS 族)
//! 仅测试消费, 已随波12 移入 tests.rs — 生产布局不持 cfg 第二份手工快照。
//!
//! 锚点公式 (doc/minihud贡献者开发手册.md §3.2):
//! **Self.Point(SelfAnchor) = Parent.Point(ParentAnchor) + Offset(Unit × LineHeight)**
//! —— 求解体 = [`crate::layout::hud_layout_node`] 的 `solve()` (已移植), 本引擎是它的
//! 驱动方 (拓扑排序 → 逐节点按父矩形求解)。
//!
//! 映射裁决:
//! - 节点图 = `SharedNode<T>` (crate::layout::hud_layout_node, Rc+RefCell 共享句柄);
//!   engine 的 nodes 容器**强持全部节点** (hud_layout_node.rs PORT 备案: 否则
//!   Weak 父升级失败会让节点被误判为 ROOT)。
//! - Java `HashMap<String,HUDLayoutNode>` → `Vec<(String, SharedNode<T>)>` (线性
//!   查找, MiniHUD 节点 <15)。HashMap 迭代序 = String hash 桶序,
//!   逐 id 复刻不现实; roots 遍历序决定跨根渲染序 → 以插入序 (addNode 调用序)
//!   近似, 保稳定可测。对拍帧实证 (审查 A 实测): 双根 (row0 链 与 crosshair) 的
//!   preferred 矩形**相交** (crosshair x282-508 vs speedBar x339-426/y92-246),
//!   但本帧实际像素不重叠 (speedBar 右缘 x438 < crosshair 图样左缘 x451) 且双端
//!   渲染序一致 (crosshair 最后) → 当前无视觉差; 组件尺寸若变化, 跨根覆盖序将
//!   依赖本 Vec 迭代序, 届时须复核 (勿再以「互不重叠」为前提)。
//!   同 id 覆盖时位置不变 (HashMap.put 语义)。
//! - `render(Graphics2D)` → 回调形式: Java 逐节点 `component.draw(g, x, y)` 与
//!   debug 线框 `drawDebug` 两次绘制合并为一次闭包调用 (第 4 参 `None`=组件本体 /
//!   `Some(color)`=调试框, 与 Java 逐节点先本体外后框的 z 序一致); 画布由宿主携带。
//! - `applyAutoSizing(window, padding)` → [`ModernHUDLayoutEngine::apply_auto_sizing`]
//!   返回 [`AutoSizingPlan`]: `window.setSize` 是 AWT 副作用, 由宿主执行;
//!   `setRenderOffset` 副作用保留在引擎内 (Java 同款)。

use std::collections::HashSet;

use crate::layout::hud_layout_node::{
    HUDLayoutNodeExt, HasPreferredSize, Rectangle, SharedNode,
};

/// HUDComponent 接口的布局引擎侧最小 seam (Java src/ui/component/HUDComponent.java
/// 接口两方法: getPreferredSize 已入 vm-core `HasPreferredSize`; isVisible 由本模块
/// 补充 —— 不越文件改 vm-core)。render/getContentBounds 的可见性门控依赖它。
pub trait HasVisibility {
    /// Java `boolean isVisible()` (AbstractHUDComponent.visible 字段)
    fn is_visible(&self) -> bool;
}

// ---------------------------------------------------------------------------
// ModernHUDLayoutEngine (src/ui/layout/ModernHUDLayoutEngine.java 一比一)
// ---------------------------------------------------------------------------

/// A modern, unit-based layout engine for HUDs.
/// Features:
/// 1. LineHeight based scaling (DPI independent).
/// 2. Topological dependency resolution.
/// 3. Anchor-based positioning.
// Java `Map<String, HUDLayoutNode> nodes` → Vec 对 (id, 共享句柄)。
pub struct ModernHUDLayoutEngine<T> {
    /// ID -> Node (HashMap → Vec, 见模块头映射裁决; 同 id put 覆盖位置不变)
    nodes: Vec<(String, SharedNode<T>)>,
    line_height: f64,
    canvas_width: i32,
    canvas_height: i32,
    canvas_rect: Rectangle,

    // Sorted list of nodes for rendering/layout
    sorted_nodes: Vec<SharedNode<T>>,
    dirty: bool,

    debug: bool,
    render_offset_x: i32,
    render_offset_y: i32,
}

impl<T> ModernHUDLayoutEngine<T> {
    /// Java 构造器 `ModernHUDLayoutEngine(int width, int height)` (缺省 lineHeight=20.0)
    pub fn new(width: i32, height: i32) -> Self {
        let mut engine = ModernHUDLayoutEngine {
            nodes: Vec::new(),
            line_height: 20.0,
            canvas_width: 0,
            canvas_height: 0,
            canvas_rect: Rectangle::new(),
            sorted_nodes: Vec::new(),
            dirty: true,
            debug: false,
            render_offset_x: 0,
            render_offset_y: 0,
        };
        engine.set_canvas_size(width, height);
        engine
    }

    /// Java `setCanvasSize(int, int)` — Keep current Origin
    pub fn set_canvas_size(&mut self, width: i32, height: i32) {
        self.canvas_width = width;
        self.canvas_height = height;
        // Keep current Origin
        self.canvas_rect.width = width;
        self.canvas_rect.height = height;
        self.dirty = true;
    }

    /// Java `setCanvasOrigin(int, int)`
    pub fn set_canvas_origin(&mut self, x: i32, y: i32) {
        self.canvas_rect.x = x;
        self.canvas_rect.y = y;
        self.dirty = true;
    }

    /// Java `setLineHeight(double)` — |Δ|>0.001 才接受
    pub fn set_line_height(&mut self, line_height: f64) {
        if (self.line_height - line_height).abs() > 0.001 {
            self.line_height = line_height;
            self.dirty = true;
        }
    }

    /// Java `addNode(HUDLayoutNode)` (HashMap.put: 同 id 覆盖 value, 位置不变)
    pub fn add_node(&mut self, node: SharedNode<T>) {
        let id = node.borrow().id.clone();
        match self.nodes.iter().position(|(k, _)| *k == id) {
            Some(i) => self.nodes[i].1 = node,
            None => self.nodes.push((id, node)),
        }
        self.dirty = true;
    }

    /// Java `getNode(String)`
    pub fn get_node(&self, id: &str) -> Option<SharedNode<T>> {
        self.nodes
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, n)| n.clone())
    }

    /// 删除节点 (R6 编辑面: 删组件; Java 无对应 — 编辑器新增)。
    /// 子节点的父引用由调用方悬空处理 (set_parent(None))
    pub fn remove_node(&mut self, id: &str) -> Option<SharedNode<T>> {
        let pos = self.nodes.iter().position(|(k, _)| k == id)?;
        let (_, node) = self.nodes.remove(pos);
        self.dirty = true;
        Some(node)
    }

    /// 节点重排 (R6 编辑面: z 序 = nodes 序; to 越界钳到端点)
    pub fn reorder_node(&mut self, id: &str, to: usize) -> bool {
        let Some(pos) = self.nodes.iter().position(|(k, _)| k == id) else {
            return false;
        };
        let entry = self.nodes.remove(pos);
        let to = to.min(self.nodes.len());
        self.nodes.insert(to, entry);
        self.dirty = true;
        true
    }

    /// Java `clear()`
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.sorted_nodes.clear();
        self.dirty = true;
    }

    /// Java `setDebug(boolean)` (置脏: 手册 §2.2 脏标记来源之一)
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
        self.dirty = true;
    }

    /// Java `setRenderOffset(int, int)`
    pub fn set_render_offset(&mut self, x: i32, y: i32) {
        self.render_offset_x = x;
        self.render_offset_y = y;
    }

    /// Java `resolveTopology()`: Sort nodes based on dependency (DFS).
    /// Root nodes (no parent) come first.
    fn resolve_topology(&mut self) {
        self.sorted_nodes.clear();
        let mut visited: HashSet<String> = HashSet::new();
        let mut recursion_stack: HashSet<String> = HashSet::new();

        for (_, node) in &self.nodes {
            if node.get_parent().is_none() {
                Self::visit_node(
                    node,
                    &mut visited,
                    &mut recursion_stack,
                    &mut self.sorted_nodes,
                );
            }
        }
    }

    /// Java `visit(HUDLayoutNode, Set, Set)` — 前序 DFS: 节点先于其子进入
    /// sortedNodes (渲染序 = 子永远覆盖在父之上, 手册 §4.2)。
    fn visit_node(
        node: &SharedNode<T>,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        out: &mut Vec<SharedNode<T>>,
    ) {
        let id = node.borrow().id.clone();
        if visited.contains(&id) {
            return;
        }
        if stack.contains(&id) {
            vm_core::base::logger::info(
                "ModernLayout",
                &format!("Cycle detected in layout dependency: {id}"),
            );
            return;
        }

        stack.insert(id.clone());

        // Dependency: Parent must be layout BEFORE Child.
        // Wait, 'visit' logic for sorting?
        // If 'parent' is dependency, we should visit parent first.
        // My loop starts from Roots (parent==null).
        // Then I should traverse children.
        // Roots are calculated first relative to Canvas.
        // Children are calculated relative to Parent.

        out.push(node.clone());

        for child in node.get_children() {
            Self::visit_node(&child, visited, stack, out);
        }

        stack.remove(&id);
        visited.insert(id);
    }

    /// Java `calculateCoordinates()`: 锚点公式的驱动循环 ——
    /// Self.Point = Parent.Point(ParentAnchor) + Offset (根节点父矩形 = canvasRect)
    fn calculate_coordinates(&mut self)
    where
        T: HasPreferredSize,
    {
        for node in &self.sorted_nodes {
            let ref_rect = match node.get_parent() {
                None => self.canvas_rect,
                Some(p) => p.get_pixel_rect(),
            };
            node.solve(self.line_height, &ref_rect);
        }
    }

    /// Java `render(Graphics2D)` → 回调形式 (见模块头映射裁决)。
    /// 每个可见节点依次回调: `None` = component.draw(g, x+offX, y+offY);
    /// debug 开启时紧随一次 `Some(调试框色)` = drawDebug 的 1px 线框
    /// (rect 同为 x+offX, y+offY, w, h — 颜色由 id hash 生成, 见 [`debug_frame_color`])。
    pub fn render(&self, mut draw: impl FnMut(&SharedNode<T>, i32, i32, Option<[u8; 4]>))
    where
        T: HasVisibility,
    {
        // ... (existing render logic)
        for node in &self.sorted_nodes {
            if !node.borrow().component.is_visible() {
                continue;
            }

            let r = node.get_pixel_rect();
            // Apply Render Offset to shift logical layout into physical window space
            let x = r.x + self.render_offset_x;
            let y = r.y + self.render_offset_y;
            draw(node, x, y, None);

            if self.debug {
                let color = debug_frame_color(&node.borrow().id);
                draw(node, x, y, Some(color));
            }
        }
    }

    /// Java `logTopology()`
    pub fn log_topology(&self) {
        vm_core::base::logger::info("ModernLayout", "Topology Order: ");
        for node in &self.sorted_nodes {
            let parent = match node.get_parent() {
                None => "ROOT".to_string(),
                Some(p) => p.borrow().id.clone(),
            };
            vm_core::base::logger::info(
                "ModernLayout",
                &format!(" -> {} (Parent: {})", node.borrow().id, parent),
            );
        }
    }

    /// Calculate the bounding rectangle of all VISIBLE components.
    /// Used for dynamic window resizing.
    /// (Java javadoc 原文重复两遍, 保留一份)
    pub fn get_content_bounds(&self) -> Rectangle
    where
        T: HasVisibility,
    {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut has_content = false;

        for node in &self.sorted_nodes {
            // Java `node.component != null && ...` 的 null 检查 — Rust T 非可空
            if node.borrow().component.is_visible() {
                has_content = true;
                let r = node.get_pixel_rect();
                let right = r.x + r.width;
                let bottom = r.y + r.height;

                if r.x < min_x {
                    min_x = r.x;
                }
                if r.y < min_y {
                    min_y = r.y;
                }
                if right > max_x {
                    max_x = right;
                }
                if bottom > max_y {
                    max_y = bottom;
                }
            }
        }

        // Return at least 1x1 to avoid invisible windows if empty
        if !has_content {
            return Rectangle::with_bounds(0, 0, 1, 1);
        }

        // Return full bounding box relative to current (0,0)
        // Width/Height must be positive dimensions
        Rectangle::with_bounds(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Java `applyAutoSizing(Component window, int padding)` 的计算部分:
    /// 返回新窗口尺寸与内容居中偏移; `window.setSize(newWidth, newHeight)` 的 AWT
    /// 副作用由宿主执行, `setRenderOffset(offsetX, offsetY)` 副作用在此保留。
    pub fn apply_auto_sizing(&mut self, padding: i32) -> AutoSizingPlan
    where
        T: HasPreferredSize + HasVisibility,
    {
        // 1. Ensure topology is resolved
        self.do_layout();

        // 2. Get actual content bounds
        let content_bounds = self.get_content_bounds();

        // 3. Calculate Render Offset
        // Goal: Shift minX/minY to the padding position
        let offset_x = padding - content_bounds.x;
        let offset_y = padding - content_bounds.y;

        // 4. Calculate New Window Size
        // Width = Content Width + Left Padding + Right Padding
        let new_width = content_bounds.width + (padding * 2);
        let new_height = content_bounds.height + (padding * 2);

        // 5. Apply changes
        // (Java: window.setSize(newWidth, newHeight); — 宿主职责)
        self.set_render_offset(offset_x, offset_y);

        vm_core::base::logger::info(
            "ModernLayout",
            &format!(
                "Auto-sized window: Content[{},{} {}x{}] -> Window[{}x{}] Offset[{},{}]",
                content_bounds.x,
                content_bounds.y,
                content_bounds.width,
                content_bounds.height,
                new_width,
                new_height,
                offset_x,
                offset_y
            ),
        );

        AutoSizingPlan {
            new_width,
            new_height,
            offset_x,
            offset_y,
            content_x: content_bounds.x,
            content_y: content_bounds.y,
            content_w: content_bounds.width,
            content_h: content_bounds.height,
        }
    }

    /// Perform layout calculation if needed.
    /// (Java `doLayout()` — dirty 分支之外**无条件**再调一次 calculateCoordinates,
    /// 组件尺寸变化无需手动置脏即可生效; 原注释保留)
    pub fn do_layout(&mut self)
    where
        T: HasPreferredSize,
    {
        if self.dirty {
            self.resolve_topology();
            self.calculate_coordinates();
            self.dirty = false;
        }

        // Always re-calculate if components changed size?
        // Ideally, we check if any component size changed.
        // For performance, we assume size changes trigger layouts externally or we
        // check hash?
        // Modern engine: check basic dirty flag or forced update.
        // In simple mode: always recalculate positions is cheap if node count < 100.
        self.calculate_coordinates();
    }
}

/// PORT(hud_layout_node.rs 备案 b): 重复 setParent 可构造 children 强引用环
/// (Rc 永不回收; Java GC 可收环)。[`ModernHUDLayoutEngine::visit_node`] 的环
/// 检测分支保真 Java 只日志+跳过 (ModernHUDLayoutEngine),
/// 断环职责由本引擎在 **drop 时**履行: 逐节点摘除父边 (同时从父 children 移除),
/// engine 强持的引用环随 nodes map 一起释放。产品路径 (widgets::page_layout)
/// 不可能构环, 此清扫仅覆盖对抗性直接 set_parent 的场景。
impl<T> Drop for ModernHUDLayoutEngine<T> {
    fn drop(&mut self) {
        for (_, node) in &self.nodes {
            node.set_parent(None);
        }
    }
}

/// [`ModernHUDLayoutEngine::apply_auto_sizing`] 的返回计划
/// (Java applyAutoSizing 对 window 的两步副作用拆分)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoSizingPlan {
    pub new_width: i32,
    pub new_height: i32,
    pub offset_x: i32,
    pub offset_y: i32,
    /// 内容包围盒 (画布系) — 编辑器画布坐标系的基准面: solve 快照用
    /// (窗口视图 = 画布视图 + offset 平移; offset = padding − content 原点)
    pub content_x: i32,
    pub content_y: i32,
    pub content_w: i32,
    pub content_h: i32,
}

/// Java `String.hashCode()` (JLS: h = 31*h + c, UTF-16 码元)。
/// 引擎 id 全为 ASCII → `chars()` 与 UTF-16 码元序列等价;
/// §2.2: hash 溢出 Java 静默回绕 → wrapping_mul/add。
pub fn java_string_hashcode(s: &str) -> i32 {
    let mut h: i32 = 0;
    for c in s.chars() {
        h = h.wrapping_mul(31).wrapping_add(c as i32);
    }
    h
}

/// Java `drawDebug` 的调试框颜色: id hashCode 低 24 位拆 RGB, 暗色提亮 +100
/// (ModernHUDLayoutEngine)。alpha=255 (`new Color(r,g,b)` 不透明)。
pub fn debug_frame_color(id: &str) -> [u8; 4] {
    let hash = java_string_hashcode(id);
    // Ensure high brightness for visibility on dark background
    let mut r_col = (hash & 0xFF0000) >> 16;
    let mut g_col = (hash & 0x00FF00) >> 8;
    let mut b_col = hash & 0x0000FF;
    if r_col + g_col + b_col < 380 {
        r_col = 255.min(r_col + 100);
        g_col = 255.min(g_col + 100);
        b_col = 255.min(b_col + 100);
    }
    [r_col as u8, g_col as u8, b_col as u8, 255]
}
#[cfg(test)]
mod tests;
