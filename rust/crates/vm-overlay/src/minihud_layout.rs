//! minihud_layout: MiniHUD 拓扑相对布局引擎 (C 类语义复刻)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`ModernHUDLayoutEngine`] | src/ui/layout/ModernHUDLayoutEngine.java | DAG 依赖解析: DFS 前序拓扑排序 (根=无父节点, 父先子后) + 惰性 dirty 布局 + 可见节点包围盒/自动尺寸 |
//! | [`MINIHUD_PANEL_ITEMS`] | ui_layout.cfg (panel "MiniHUD" L45-94) | cfg 常量表快照 (同 vm-core fields.rs 先例, 布局消费子集): 组件树生成的配置键/默认值单一来源 |
//! | [`MINIHUD_NODE_SPECS`] + [`build_mihud_layout`] | src/ui/overlay/MiniHUDOverlay.java initModernLayout (L652-763) | cfg 驱动组件树生成: Java 硬编码拓扑转常量 spec 表, build 按配置 (displayCrosshair) 与行数动态建树 |
//!
//! 锚点公式 (doc/minihud贡献者开发手册.md §3.2):
//! **Self.Point(SelfAnchor) = Parent.Point(ParentAnchor) + Offset(Unit × LineHeight)**
//! —— 求解体 = [`vm_core::hud_layout_node`] 的 `solve()` (已移植), 本引擎是它的
//! 驱动方 (拓扑排序 → 逐节点按父矩形求解)。
//!
//! 映射裁决:
//! - 节点图 = `SharedNode<T>` (vm_core::hud_layout_node, Rc+RefCell 共享句柄);
//!   engine 的 nodes 容器**强持全部节点** (hud_layout_node.rs PORT 备案: 否则
//!   Weak 父升级失败会让节点被误判为 ROOT)。
//! - Java `HashMap<String,HUDLayoutNode>` → `Vec<(String, SharedNode<T>)>` (线性
//!   查找, MiniHUD 节点 <15)。PORT(§2.5): HashMap 迭代序 = String hash 桶序,
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

use vm_core::hud_layout_node::{
    HasPreferredSize, HUDLayoutNode, HUDLayoutNodeExt, Rectangle, SharedNode,
};
use vm_core::layout::Anchor;

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
// PORT: Java `Map<String, HUDLayoutNode> nodes` → Vec 对 (id, 共享句柄)。
pub struct ModernHUDLayoutEngine<T> {
    /// ID -> Node (PORT: HashMap → Vec, 见模块头映射裁决; 同 id put 覆盖位置不变)
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
                Self::visit_node(node, &mut visited, &mut recursion_stack, &mut self.sorted_nodes);
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
            vm_core::logger::info(
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
    pub fn render(
        &self,
        mut draw: impl FnMut(&SharedNode<T>, i32, i32, Option<[u8; 4]>),
    ) where
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
        vm_core::logger::info("ModernLayout", "Topology Order: ");
        for node in &self.sorted_nodes {
            let parent = match node.get_parent() {
                None => "ROOT".to_string(),
                Some(p) => p.borrow().id.clone(),
            };
            vm_core::logger::info(
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
            // PORT: Java `node.component != null && ...` 的 null 检查 — Rust T 非可空
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

        vm_core::logger::info(
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
/// 检测分支保真 Java 只日志+跳过 (ModernHUDLayoutEngine.java:111-114),
/// 断环职责由本引擎在 **drop 时**履行: 逐节点摘除父边 (同时从父 children 移除),
/// engine 强持的引用环随 nodes map 一起释放。产品路径 (build_mihud_layout)
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
}

/// Java `String.hashCode()` (JLS: h = 31*h + c, UTF-16 码元)。
/// PORT(§2.1): 引擎 id 全为 ASCII → `chars()` 与 UTF-16 码元序列等价;
/// §2.2: hash 溢出 Java 静默回绕 → wrapping_mul/add。
pub fn java_string_hashcode(s: &str) -> i32 {
    let mut h: i32 = 0;
    for c in s.chars() {
        h = h.wrapping_mul(31).wrapping_add(c as i32);
    }
    h
}

/// Java `drawDebug` 的调试框颜色: id hashCode 低 24 位拆 RGB, 暗色提亮 +100
/// (ModernHUDLayoutEngine.java:176-186)。alpha=255 (`new Color(r,g,b)` 不透明)。
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

// ---------------------------------------------------------------------------
// ui_layout.cfg (panel "MiniHUD" L45-94) 常量表快照 — 同 vm-core fields.rs 先例
// ---------------------------------------------------------------------------

/// :type (布局引擎/设置面板消费的子集; info 行不含 target 不入表)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiniHudItemType {
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
pub enum CfgDefault {
    Bool(bool),
    Int(i32),
    Str(&'static str),
}

/// panel "MiniHUD" 单条 (item ...) 定义快照
#[derive(Debug, Clone, Copy)]
pub struct MiniHudCfgItem {
    /// 所属 (group "...") 名
    pub group: &'static str,
    /// item 标签原文
    pub label: &'static str,
    pub item_type: MiniHudItemType,
    /// :target 配置键 (cfg 字符串键原样, PORTING.md §0.5)
    pub target: &'static str,
    pub default: CfgDefault,
    /// slider :min
    pub min: Option<i32>,
    /// slider :max
    pub max: Option<i32>,
    /// slider :unit
    pub unit: Option<&'static str>,
}

/// MiniHUD panel 段 28 条 item 逐行快照 (ui_layout.cfg L45-94, 顺序一致)。
/// 组件树生成的配置键单一来源: displayCrosshair 控制 crosshair 节点与画布倍宽,
/// enableFlapAngleBar/showSpeedBar/showAttitudeGauge 等控制组件可见性 (宿主消费)。
/// PORT(审查 A3): 本表是**布局消费的子集**快照 — 只保留 label/group/type/
/// target/value/min/max/unit; combo 的 :source ("_CROSSHAIRS_"/"_FONTS_") 与
/// :desc/:desc-img/:column 等设置面板字段未入表。P5 vm-ui 生成完整设置面板时
/// 须从 ui_layout.cfg 另出全量表, 勿复用本表 (避免单一来源分裂)。
pub const MINIHUD_PANEL_ITEMS: &[MiniHudCfgItem] = &[
    // (group "基本设定")
    MiniHudCfgItem { group: "基本设定", label: "启用Overlay", item_type: MiniHudItemType::Switch, target: "crosshairSwitch", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    // (group "hud面板设置")
    MiniHudCfgItem { group: "hud面板设置", label: "显示hud数据", item_type: MiniHudItemType::Switch, target: "drawHUDtext", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud面板设置", label: "显示hud准星", item_type: MiniHudItemType::Switch, target: "displayCrosshair", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    // (group "hud数据设置")
    MiniHudCfgItem { group: "hud数据设置", label: "智能襟翼指示器", item_type: MiniHudItemType::Switch, target: "enableFlapAngleBar", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "油门条/速度条", item_type: MiniHudItemType::Switch, target: "showSpeedBar", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "罗盘/姿态指示", item_type: MiniHudItemType::Switch, target: "showAttitudeGauge", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "离体/随体配置", item_type: MiniHudItemType::Switch, target: "attitudeIndicatorInertialMode", default: CfgDefault::Bool(false), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "表速更换马赫数", item_type: MiniHudItemType::Switch, target: "hudMach", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "始终显示雷达高度", item_type: MiniHudItemType::Switch, target: "alwaysShowRadarAltitude", default: CfgDefault::Bool(false), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "攻角数值告警阈值", item_type: MiniHudItemType::Slider, target: "miniHUDaoaWarningRatio", default: CfgDefault::Int(20), min: Some(0), max: Some(100), unit: Some("%") },
    MiniHudCfgItem { group: "hud数据设置", label: "攻角条告警阈值", item_type: MiniHudItemType::Slider, target: "miniHUDaoaBarWarningRatio", default: CfgDefault::Int(25), min: Some(0), max: Some(100), unit: Some("%") },
    MiniHudCfgItem { group: "hud数据设置", label: "速度读数", item_type: MiniHudItemType::Switch, target: "showHUDSpeed", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "攻角指示", item_type: MiniHudItemType::Switch, target: "showHUDAoA", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "高度读数", item_type: MiniHudItemType::Switch, target: "showHUDAltitude", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "能量读数", item_type: MiniHudItemType::Switch, target: "showHUDEnergy", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "襟翼/可变翼", item_type: MiniHudItemType::Switch, target: "showHUDFlaps", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "减速板", item_type: MiniHudItemType::Switch, target: "showHUDAirbrake", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "起落架", item_type: MiniHudItemType::Switch, target: "showHUDGear", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "爬升率", item_type: MiniHudItemType::Switch, target: "showHUDSep", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "过载读数", item_type: MiniHudItemType::Switch, target: "showHUDGLoad", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud数据设置", label: "机动条", item_type: MiniHudItemType::Switch, target: "showHUDManeuverBar", default: CfgDefault::Bool(true), min: None, max: None, unit: None },
    // (group "hud文字标签设置")
    MiniHudCfgItem { group: "hud文字标签设置", label: "速度读数显示标签", item_type: MiniHudItemType::SwitchInv, target: "disableHUDSpeedLabel", default: CfgDefault::Bool(false), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud文字标签设置", label: "高度读数显示标签", item_type: MiniHudItemType::SwitchInv, target: "disableHUDHeightLabel", default: CfgDefault::Bool(false), min: None, max: None, unit: None },
    MiniHudCfgItem { group: "hud文字标签设置", label: "SEP读数显示标签", item_type: MiniHudItemType::SwitchInv, target: "disableHUDSEPLabel", default: CfgDefault::Bool(false), min: None, max: None, unit: None },
    // (group "hud准星设置")
    MiniHudCfgItem { group: "hud准星设置", label: "选择准星", item_type: MiniHudItemType::Combo, target: "crosshairName", default: CfgDefault::Str("软件渲染准星"), min: None, max: None, unit: None },
    // (group "外观设置")
    MiniHudCfgItem { group: "外观设置", label: "minihud大小", item_type: MiniHudItemType::Slider, target: "crosshairScale", default: CfgDefault::Int(113), min: Some(0), max: Some(200), unit: None },
    MiniHudCfgItem { group: "外观设置", label: "hud读数和指示器字体大小", item_type: MiniHudItemType::Slider, target: "fontSize", default: CfgDefault::Int(0), min: Some(-10), max: Some(10), unit: None },
    MiniHudCfgItem { group: "外观设置", label: "等宽字体", item_type: MiniHudItemType::Combo, target: "MonoNumFont", default: CfgDefault::Str("Sarasa Mono SC"), min: None, max: None, unit: None },
];

/// enableLayoutDebug 不在 MiniHUD panel 段 (位于「杂项→调试」组, ui_layout.cfg:392),
/// 但为 MiniHUDOverlay.initModernLayout:668 消费的布局引擎开关 — 单列快照。
pub const ENABLE_LAYOUT_DEBUG_ITEM: MiniHudCfgItem = MiniHudCfgItem {
    group: "调试",
    label: "显示布局调试",
    item_type: MiniHudItemType::Switch,
    target: "enableLayoutDebug",
    default: CfgDefault::Bool(false),
    min: None,
    max: None,
    unit: None,
};

/// 组件树生成所需的布局开关系集。
/// PORT: Java 侧 = MiniHUDOverlay 直接持 HUDSettings (initModernLayout 读
/// isDisplayCrosshair / getBool("enableLayoutDebug", false)); Rust 以纯值结构
/// 解耦配置源。两层缺省 (审查 B3): [`Default`] = cfg 树健康时的 :default 快照;
/// [`from_bool_source`] 缺键 = Java getBool 的**字面兜底参数** (整树缺失/损坏
/// 分支, 此时 Java 关准星 + 单宽画布)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiniHudLayoutConfig {
    /// HUDSettings.isDisplayCrosshair() = getBool("displayCrosshair", false)
    /// (cfg :default true; Java 字面兜底 false)
    pub display_crosshair: bool,
    /// hudSettings.getBool("enableLayoutDebug", false) (两层缺省均为 false)
    pub enable_layout_debug: bool,
}

impl Default for MiniHudLayoutConfig {
    /// cfg :default 快照值 (displayCrosshair=true: cfg 树健康且用户未设时的值)
    fn default() -> Self {
        MiniHudLayoutConfig {
            display_crosshair: true,
            enable_layout_debug: false,
        }
    }
}

impl MiniHudLayoutConfig {
    /// 从任意 bool 配置源构造 (键 = cfg :target 原样)。
    /// 缺键/无法解析 → Java getBool 的字面兜底参数 (ConfigurationService.java:639
    /// isDisplayCrosshair() = getBool("displayCrosshair", **false**);
    /// MiniHUDOverlay.java:668 getBool("enableLayoutDebug", false))。
    /// cfg 树健康时 row 的 :default (displayCrosshair=true) 由 src 侧生效 —
    /// 与 [`MiniHudLayoutConfig::default`] 是两层不同缺省。
    pub fn from_bool_source(src: impl Fn(&str) -> Option<bool>) -> Self {
        MiniHudLayoutConfig {
            // PORT(审查 B3): Java 字面兜底是 false 而非 cfg :default 的 true
            display_crosshair: src("displayCrosshair").unwrap_or(false),
            enable_layout_debug: src("enableLayoutDebug").unwrap_or(false),
        }
    }
}

// ---------------------------------------------------------------------------
// MiniHUD 组件树拓扑 (MiniHUDOverlay.initModernLayout L679-754 硬编码拓扑的快照)
// ---------------------------------------------------------------------------

/// spec 行对应的组件槽位 (MiniHUDOverlay.initComponentsLayout 的组件清单)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiniHudComp {
    /// hudRows[i] ("row" + i, 链式挂载; i 越界=行不存在 → 整个节点不建)
    Row(usize),
    /// flapAngleBar ("flap")
    FlapBar,
    /// speedRatioBar ("speedBar")
    SpeedRatioBar,
    /// throttleBar ("throttle")
    ThrottleBar,
    /// attitudeIndicatorGauge ("attitude")
    AttitudeGauge,
    /// compassGauge ("compass")
    CompassGauge,
    /// crosshairGauge ("crosshair", displayCrosshair 才建)
    Crosshair,
}

/// 单节点拓扑定义 (锚点 + 单位偏移, 手册 §3 公式的静态参数)
#[derive(Debug, Clone, Copy)]
pub struct MiniHudNodeSpec {
    pub id: &'static str,
    pub component: MiniHudComp,
    /// 父节点 id (None = 根, 挂 canvas)
    pub parent: Option<&'static str>,
    /// 相对偏移 (行高单位, solve 里 × lineHeight 后 (int) 截断)
    pub unit_x: f64,
    pub unit_y: f64,
    pub parent_anchor: Anchor,
    pub self_anchor: Anchor,
}

/// MiniHUD 组件树拓扑常量 (MiniHUDOverlay.java:681-753 逐节点快照, addNode 顺序)。
/// 行间链 (row1..row4) = Java 循环体 (0, 0.1) BOTTOM_LEFT/TOP_LEFT (L699-713);
/// row2 依赖存在才建 attitude/compass (L716); speedBar/throttle 的父 row4 缺席时
/// 退化为根 (L736/743 setParent(row4) 可空); crosshair 仅 displayCrosshair (L749)。
pub const MINIHUD_NODE_SPECS: &[MiniHudNodeSpec] = &[
    // 3. Row 0 (New Anchor for Left Block) — Position: 2.1, 3.5 units
    MiniHudNodeSpec { id: "row0", component: MiniHudComp::Row(0), parent: None, unit_x: 2.1, unit_y: 3.5, parent_anchor: Anchor::TopLeft, self_anchor: Anchor::TopLeft },
    // 4. Flap Bar (Child of Row 0) — Pos: 0, -0.1 (Above Row 0)
    MiniHudNodeSpec { id: "flap", component: MiniHudComp::FlapBar, parent: Some("row0"), unit_x: 0.0, unit_y: -0.1, parent_anchor: Anchor::TopLeft, self_anchor: Anchor::BottomLeft },
    // 5. Rows Chain — Standard Line Spacing: 1.4 units (down from previous row top)
    MiniHudNodeSpec { id: "row1", component: MiniHudComp::Row(1), parent: Some("row0"), unit_x: 0.0, unit_y: 0.1, parent_anchor: Anchor::BottomLeft, self_anchor: Anchor::TopLeft },
    MiniHudNodeSpec { id: "row2", component: MiniHudComp::Row(2), parent: Some("row1"), unit_x: 0.0, unit_y: 0.1, parent_anchor: Anchor::BottomLeft, self_anchor: Anchor::TopLeft },
    MiniHudNodeSpec { id: "row3", component: MiniHudComp::Row(3), parent: Some("row2"), unit_x: 0.0, unit_y: 0.1, parent_anchor: Anchor::BottomLeft, self_anchor: Anchor::TopLeft },
    MiniHudNodeSpec { id: "row4", component: MiniHudComp::Row(4), parent: Some("row3"), unit_x: 0.0, unit_y: 0.1, parent_anchor: Anchor::BottomLeft, self_anchor: Anchor::TopLeft },
    // 6. Right Side Instruments (Attached to Row 2) — Attitude: Pos 0, 0.5
    MiniHudNodeSpec { id: "attitude", component: MiniHudComp::AttitudeGauge, parent: Some("row2"), unit_x: 0.0, unit_y: 0.5, parent_anchor: Anchor::BottomRight, self_anchor: Anchor::TopRight },
    // Compass (Child of Row 2) — Pos: 0, 0.1
    MiniHudNodeSpec { id: "compass", component: MiniHudComp::CompassGauge, parent: Some("row2"), unit_x: 0.0, unit_y: 0.1, parent_anchor: Anchor::BottomRight, self_anchor: Anchor::TopRight },
    // Rate Bar (SpeedRatioBar) — Pos: -0.3, 0
    MiniHudNodeSpec { id: "speedBar", component: MiniHudComp::SpeedRatioBar, parent: Some("row4"), unit_x: -0.3, unit_y: 0.0, parent_anchor: Anchor::BottomLeft, self_anchor: Anchor::BottomRight },
    // Throttle Bar — Pos: -0.3, 0
    MiniHudNodeSpec { id: "throttle", component: MiniHudComp::ThrottleBar, parent: Some("row4"), unit_x: -0.3, unit_y: 0.0, parent_anchor: Anchor::BottomLeft, self_anchor: Anchor::BottomRight },
    // Crosshair (Independent, Center of Attention) — Pos: 0, 0
    MiniHudNodeSpec { id: "crosshair", component: MiniHudComp::Crosshair, parent: None, unit_x: 0.0, unit_y: 0.0, parent_anchor: Anchor::MiddleRight, self_anchor: Anchor::MiddleRight },
];

/// build 输入的组件槽位 (MiniHUDOverlay.initComponentsLayout 组件清单,
/// 字段非 Option 项 = Java 侧恒 new 的组件; crosshair_gauge 为 Option 是因为
/// Rust build 以 None 表达 displayCrosshair=false 时不提供, Java 侧对象恒存在
/// 但节点不建)
pub struct MiniHudParts<T> {
    /// hudRows (MiniHUD 5 行: HUDAkbRow/HUDEnergyRow/HUDMechanization/HUDText/Maneuver)
    pub rows: Vec<T>,
    pub flap_angle_bar: T,
    pub speed_ratio_bar: T,
    pub throttle_bar: T,
    pub attitude_indicator_gauge: T,
    pub compass_gauge: T,
    pub crosshair_gauge: Option<T>,
}

/// Java MiniHUDOverlay.java:765 `private static final int LAYOUT_PADDING = 45`
pub const LAYOUT_PADDING: i32 = 45;

/// [`build_mihud_layout`] 的输出: 布局引擎 + 自动尺寸计划
/// (Java initModernLayout 尾部 applyAutoSizing 的 window.setSize 由宿主执行)。
/// `sizing=None` = Java 空 components 裸 return 分支: 不自动尺寸, 宿主保持
/// 自己的初始窗口尺寸, 引擎 renderOffset 保持缺省 (0,0)。
pub struct BuiltMiniHudLayout<T> {
    pub engine: ModernHUDLayoutEngine<T>,
    pub sizing: Option<AutoSizingPlan>,
}

/// cfg 驱动组件树生成 (Java MiniHUDOverlay.initModernLayout L652-763 的树构建部分)。
///
/// 语义保真分支:
/// - `layoutWidth = showCrosshair ? base_width*2 : base_width` (L654-655);
/// - row 链按 `rows.len()` 截断 (Java 循环 `i=1..hudRows.size()`);
/// - attitude/compass 仅当 "row2" 已建 (Java `if (row2 != null)`);
/// - speedBar/throttle 无条件建, 父 "row4" 缺席时 setParent(None) → 根;
/// - crosshair 仅 `display_crosshair` 且组件 Some (Java 只查 cfg — 组件恒非 null);
/// - 空 rows = Java `components.isEmpty()` 裸 return: **不执行尾部三步** (无
///   Auto-size/Topology 日志), sizing=None (窗口保持宿主初始值, renderOffset
///   保持缺省 (0,0));
/// - 非空才走尾部 doLayout + applyAutoSizing(LAYOUT_PADDING) + logTopology (L757-762)。
pub fn build_mihud_layout<T>(
    cfg: &MiniHudLayoutConfig,
    parts: MiniHudParts<T>,
    base_width: i32,
    canvas_height: i32,
    line_height: f64,
) -> BuiltMiniHudLayout<T>
where
    T: HasPreferredSize + HasVisibility,
{
    // Apply Global Debug Setting
    let show_crosshair = cfg.display_crosshair;
    let layout_width = if show_crosshair { base_width * 2 } else { base_width };
    vm_core::logger::info(
        "MinimalHUD",
        &format!(
            "initModernLayout: showCrosshair={show_crosshair}, layoutWidth={layout_width}"
        ),
    );

    let mut engine = ModernHUDLayoutEngine::new(layout_width, canvas_height);

    // [架构说明] (Java 原注释保留)
    // 这里手动传递配置而不是让 LayoutEngine 直接订阅 EventBus 是为了防止内存泄漏。
    // LayoutEngine 随 MinimalHUD 配置刷新而频繁销毁重建 (Transient Lifecycle)。
    // 如果它直接订阅全局单例 EventBus，旧实例会因无法自动注销而被长期持有，导致
    // "Zombie Listener" 泄漏。因此采用了由持有者 (MinimalHUD) 被动传递状态的设计。
    engine.set_debug(cfg.enable_layout_debug);

    // Use lineHeight from font size for responsive scaling
    engine.set_line_height(line_height);

    // Java: if (components.isEmpty()) return — initComponentsLayout 硬编码添加
    // 组件恒非空; Rust 以 rows (组件清单主体) 为空近似该守卫。
    // PORT: Java 此处裸 return — 尾部 doLayout/applyAutoSizing/logTopology 三步
    // 均不执行 (窗口保持宿主 setBounds 初始值, renderOffset 保持 (0,0), 无对应
    // 日志); sizing=None 由宿主解释为"不自动尺寸" (审查 A1/B1)。
    let mut rows: Vec<Option<T>> = parts.rows.into_iter().map(Some).collect();
    if rows.is_empty() {
        return BuiltMiniHudLayout { engine, sizing: None };
    }
    let mut flap = Some(parts.flap_angle_bar);
    let mut speed = Some(parts.speed_ratio_bar);
    let mut throttle = Some(parts.throttle_bar);
    let mut attitude = Some(parts.attitude_indicator_gauge);
    let mut compass = Some(parts.compass_gauge);
    let mut crosshair = parts.crosshair_gauge;

    vm_core::logger::info(
        "MinimalHUD",
        &format!(
            "initModernLayout: Adding nodes. Components: {}",
            rows.len() + 6
        ),
    );

    for spec in MINIHUD_NODE_SPECS {
        // 组件槽位取件 (row 越界 = 行不存在, 节点不建 — Java 循环上界)
        let component: Option<T> = match spec.component {
            MiniHudComp::Row(i) => rows.get_mut(i).and_then(|slot| slot.take()),
            MiniHudComp::FlapBar => flap.take(),
            MiniHudComp::SpeedRatioBar => speed.take(),
            MiniHudComp::ThrottleBar => throttle.take(),
            MiniHudComp::AttitudeGauge => attitude.take(),
            MiniHudComp::CompassGauge => compass.take(),
            MiniHudComp::Crosshair => {
                if show_crosshair {
                    crosshair.take()
                } else {
                    None // Java: if (hudSettings.isDisplayCrosshair()) 才建节点
                }
            }
        };
        let Some(component) = component else { continue };

        // 父节点解析: attitude/compass 依赖 "row2" 存在 (Java if (row2 != null)),
        // 缺席则整个节点不建; speedBar/throttle 的 "row4" 缺席时退化为根
        // (Java setParent(row4) 传 null); row 链父 = 前一行 (循环 prevRow)。
        let parent = spec.parent.and_then(|pid| engine.get_node(pid));
        if matches!(spec.component, MiniHudComp::AttitudeGauge | MiniHudComp::CompassGauge)
            && spec.parent.is_some()
            && parent.is_none()
        {
            continue; // Java L716: if (row2 != null) 才建右挂件
        }

        let node = HUDLayoutNode::new(spec.id, component);
        node.set_parent(parent.as_ref())
            .set_relative_position(spec.unit_x, spec.unit_y)
            .set_anchors(spec.parent_anchor, spec.self_anchor);
        engine.add_node(node);
    }

    // Force layout calculation to populate sortedNodes for logging
    engine.do_layout();

    // Dynamic Sizing Implementation (Generic)
    let sizing = engine.apply_auto_sizing(LAYOUT_PADDING);

    engine.log_topology();

    BuiltMiniHudLayout { engine, sizing: Some(sizing) }
}

#[cfg(test)]
mod tests;
