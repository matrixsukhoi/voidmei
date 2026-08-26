//! minihud_layout: MiniHUD 拓扑相对布局引擎 (C 类语义复刻)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`ModernHUDLayoutEngine`] | src/ui/layout/ModernHUDLayoutEngine.java | DAG 依赖解析: DFS 前序拓扑排序 (根=无父节点, 父先子后) + 惰性 dirty 布局 + 可见节点包围盒/自动尺寸 |
//! | [`MINIHUD_PANEL_ITEMS`] | ui_layout.cfg (panel "MiniHUD" L45-94) | cfg 常量表快照 (同 vm-core fields.rs 先例): 组件树生成的配置键/默认值单一来源 |
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
//!   逐 id 复刻不现实; roots 遍历序决定跨根渲染序, MiniHUD 两根 (row0 链 与
//!   crosshair) 互不重叠无视觉差 → 以插入序 (addNode 调用序) 近似, 保稳定可测。
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
/// 解耦配置源, 缺省值 = 上方 cfg 快照的 :default。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiniHudLayoutConfig {
    /// HUDSettings.isDisplayCrosshair() (target "displayCrosshair", 默认 true)
    pub display_crosshair: bool,
    /// hudSettings.getBool("enableLayoutDebug", false) (默认 false)
    pub enable_layout_debug: bool,
}

impl Default for MiniHudLayoutConfig {
    /// cfg :default 快照值
    fn default() -> Self {
        MiniHudLayoutConfig {
            display_crosshair: true,
            enable_layout_debug: false,
        }
    }
}

impl MiniHudLayoutConfig {
    /// 从任意 bool 配置源构造 (键 = cfg :target 原样)。
    /// 缺键/无法解析 → cfg :default (Java getBool(key, default) 同款兜底)。
    pub fn from_bool_source(src: impl Fn(&str) -> Option<bool>) -> Self {
        MiniHudLayoutConfig {
            display_crosshair: src("displayCrosshair").unwrap_or(true),
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
pub struct BuiltMiniHudLayout<T> {
    pub engine: ModernHUDLayoutEngine<T>,
    pub sizing: AutoSizingPlan,
}

/// cfg 驱动组件树生成 (Java MiniHUDOverlay.initModernLayout L652-763 的树构建部分)。
///
/// 语义保真分支:
/// - `layoutWidth = showCrosshair ? base_width*2 : base_width` (L654-655);
/// - row 链按 `rows.len()` 截断 (Java 循环 `i=1..hudRows.size()`);
/// - attitude/compass 仅当 "row2" 已建 (Java `if (row2 != null)`);
/// - speedBar/throttle 无条件建, 父 "row4" 缺席时 setParent(None) → 根;
/// - crosshair 仅 `display_crosshair` 且组件 Some (Java 只查 cfg — 组件恒非 null);
/// - 尾部 doLayout + applyAutoSizing(LAYOUT_PADDING) + logTopology (L757-762)。
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
    let mut rows: Vec<Option<T>> = parts.rows.into_iter().map(Some).collect();
    if rows.is_empty() {
        engine.do_layout();
        let sizing = engine.apply_auto_sizing(LAYOUT_PADDING);
        engine.log_topology();
        return BuiltMiniHudLayout { engine, sizing };
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

    BuiltMiniHudLayout { engine, sizing }
}

#[cfg(test)]
mod tests {
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
        // cfg 驱动读取: 缺键走 :default; override 生效
        assert_eq!(
            MiniHudLayoutConfig::from_bool_source(|_| None),
            MiniHudLayoutConfig::default()
        );
        let cfg = MiniHudLayoutConfig::from_bool_source(|k| (k == "displayCrosshair").then_some(false));
        assert!(!cfg.display_crosshair);
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
            AutoSizingPlan { new_width: 687, new_height: 220, offset_x: 42, offset_y: -17 }
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
            AutoSizingPlan { new_width: 177, new_height: 220, offset_x: 42, offset_y: -17 }
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
            AutoSizingPlan { new_width: 227, new_height: 228, offset_x: 92, offset_y: -17 }
        );
    }

    /// 空 rows 守卫 (Java components.isEmpty() return): 空引擎 + 1x1 兜底尺寸。
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
        assert_eq!(built.sizing, AutoSizingPlan { new_width: 91, new_height: 91, offset_x: 45, offset_y: 45 });
    }
}
