//! HUD 编辑会话 (R6 真窗即画布): MainForm "编辑 HUD" 按钮进入, 桌面真实
//! overlay 直接可编辑 — 点选/拖动/resize/框选, 吸附参考线画进渲染帧本身。
//!
//! 架构 (两段式, 避借用冲突):
//! - **EditBridge 闭包** (host 持有): on_press 用缓存矩形立即裁决 (编辑接管?),
//!   on_event 入队, on_paint 用缓存矩形画装饰 — 全部只碰 `Rc<RefCell<EditSession>>`;
//! - **edit_pump** (渲染线程主循环每轮调用): 消费事件队列, 改 PageOverlay
//!   节点/PageDoc (真渲染面 — 直改即所见, 0 重建 0 落盘), 刷新缓存矩形,
//!   即时 render_tick (~100Hz 跟手)。
//!
//! 数据流: 编辑仓 docs = 会话期页文档唯一真相; on_reinit_overlays 的外部
//! params.pages 被编辑仓覆写 (写权接管); 提交 = EditCommitted → 主线程
//! save_page 落盘 → 既有 CONFIG_CHANGED 链重建; 丢弃 = 全量 ReinitOverlays。
//! (阶段 C: 原 80ms 节流 web 镜像推送链已退役 — 编辑期 UI 全原生同栈,
//! 唯一对外事件 = HUD_EDIT_SESSION 起止键, 供 MainForm 隐退/恢复;
//! 拒绝回报 = 日志留痕 (C2 镜像推送退役, 无事件面))

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use kernel::config::json_model::{ComponentDoc, PageDoc};

use overlay::layout::hud_layout_node::HUDLayoutNodeExt;
use overlay::platform::host::{EditMouse, OverlayHost};
use overlay::render::canvas::PixCanvas;
use overlay::render::font::LoadedFont;
use overlay::render::primitives;
use overlay::widgets::PageHandle;
use overlay::widgets::list_container::LIST_CONTAINER_TYPE;

/// UIStateBus 编辑事件键 (bridge.rs 订阅 → 前端 emit)。
/// 镜像推送三键 (doc/selection/error) 已随 web 编辑面板退役 (阶段 C);
/// begin 失败回报走 lib.rs 的 EditRejected 字面量发布, 不占常量
pub const HUD_EDIT_SESSION: &str = "HUD_EDIT_SESSION"; // data: "begin" | "end"

/// 右键菜单条目动作 (对象级操作; 经 apply_command 入会话命令面 = 单一撤销栈)
#[derive(Clone)]
pub(crate) enum MenuAction {
    /// 项: enabled 翻转 (软隐藏 — 保留配置, 建树期跳过)
    ToggleEnabled { id: String },
    /// 项: 兄弟段内前移/后移 (容器内 = 排列序; 自由区 = z 序)
    MoveOrder { id: String, delta: i32 },
    /// 项: 删除 (右键目标在选中集内 = 整集删)
    Delete { ids: Vec<String> },
    /// 容器: 排列切换 (columns > 0 时附带列数)
    SetArrange {
        container: String,
        arrange: String,
        columns: i64,
    },
}

/// 右键菜单 (对象级操作统一入口; 画进渲染帧 — 与吸附装饰同形态)。
/// rect = 窗口视图系 (命中与绘制同源)
pub(crate) struct EditMenu {
    pub origin: (i32, i32),
    pub items: Vec<(String, MenuAction, (i32, i32, i32, i32))>,
    pub hover: Option<usize>,
}

/// 菜单几何: 行高 / 水平留白 / 标签前缀槽 (✓ 当前项)
const MENU_ROW_H: i32 = 24;
const MENU_PAD_X: i32 = 14;

/// 拖放插入会话 (组件库 webview 内按下发起; 全局鼠标轮询驱动)
#[derive(Clone)]
pub(crate) struct DragInsert {
    pub type_name: String,
    pub display_zh: String,
    pub props: serde_json::Value,
}

/// 拖放落点反馈 (画布系; 轮询期算出, 绘制/结算共用)
#[derive(Clone, PartialEq)]
pub(crate) enum DropHint {
    /// 容器内插入线: (容器 id, 子项插入序, 线段 x0..x1 @y)
    InsertLine {
        container: String,
        index: usize,
        x0: i32,
        y: i32,
        x1: i32,
    },
    /// 自由区落点幽灵 (矩形)
    Ghost { x: i32, y: i32, w: i32, h: i32 },
}

/// 编辑装饰粉色 (全站主题; edit_chrome 同栈共用)
pub(crate) const ACCENT: [u8; 4] = [255, 105, 180, 235];
/// 网格吸附步长 (行高倍)
const SNAP: f64 = 0.1;
/// 对齐吸附阈值 (物理 px)
const ALIGN_TOL: i32 = 6;
/// 手柄绘制/命中边 (物理 px)
const HANDLE_HIT: i32 = 10;
/// resize 最小尺寸 (物理 px)
const RESIZE_MIN: i32 = 8;

/// resize 手柄方位 (组件矩形 8 向)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandleKind {
    N,
    S,
    E,
    W,
    Ne,
    Nw,
    Se,
    Sw,
}

/// 活动手势
pub enum EditGesture {
    Move {
        ids: Vec<String>,
        start_root: (i32, i32),
        start_units: Vec<(String, [f64; 2])>,
        guide_x: Option<i32>,
        guide_y: Option<i32>,
    },
    Resize {
        id: String,
        handle: HandleKind,
        start_root: (i32, i32),
        /// 手势起点组件画布系矩形 (含起点尺寸)
        start_rect: (i32, i32, i32, i32),
        /// 手势起点组件 pos 单位 (origin 位移换算基准)
        start_unit: [f64; 2],
        last_apply: Instant,
    },
    Marquee {
        start_canvas: (i32, i32),
        cur_canvas: (i32, i32),
    },
}

/// 编辑会话状态 (RenderSession.edit 持有)
pub(crate) struct EditSession {
    /// 装饰只画该页 (点其它页窗口 = 切换目标页)
    pub target_page: String,
    /// 编辑仓 (会话期页文档唯一真相)
    pub docs: Vec<PageDoc>,
    /// 选中组件 id (多选)
    pub selection: Vec<String>,
    pub hover: Option<String>,
    pub gesture: Option<EditGesture>,
    /// 进入时强制开窗的页 (退出按激活探测收回)
    pub forced_open: Vec<String>,
    pub snapping: bool,
    pub show_guides: bool,
    pub marquee_mode: bool,
    /// ---- 渲染面缓存 (EditBridge 闭包/装饰绘制用, edit_pump 每轮刷新) ----
    /// 目标页组件矩形 (画布系, doc 逆序 = z 顶层优先)
    pub hit_rects: Vec<(String, i32, i32, i32, i32)>,
    /// 窗口视图 ← 画布视图平移 (sizing.offset)
    pub canvas_off: (i32, i32),
    /// 事件队列 (on_event 入队, edit_pump 消费)
    pub pending: VecDeque<(String, EditMouse)>,
    /// 装饰面脏标记 (doc/selection 变化置位 → 编辑泵保险渲染一帧后清除。
    /// 原语义 = 80ms 节流推 web 镜像, 阶段 C 推送退役后转为纯渲染脏标记 —
    /// Select 类无重装配路径的真窗高亮刷新仍依赖它)
    pub doc_dirty: bool,
    /// ---- 全局单一撤销栈 (试驾场: 手势+命令全部入栈) ----
    /// 撤销栈 (每项 = 变更前的编辑仓快照; 手势整段 = 一项)
    pub undo_stack: Vec<Vec<PageDoc>>,
    /// 重做栈 (undo 时寄存, 新变更清空)
    pub redo_stack: Vec<Vec<PageDoc>>,
    /// 手势起点快照 (Move/Resize 起手捕获, Release 提交 — 整段拖拽 = 一项)
    pub gesture_snapshot: Option<Vec<PageDoc>>,
    /// ---- 右键菜单 (对象级操作统一入口) ----
    pub menu: Option<EditMenu>,
    /// 菜单字体 (begin 时从目标页捕获; 菜单文本渲染/度量共用)
    pub menu_font: Option<Rc<LoadedFont>>,
    /// 菜单动作执行后置位 → 主循环 rebuild_edit_target (pump 无重装配面)
    pub need_rebuild: bool,
    /// ---- 拖放插入 (组件库 → 真窗; 全局鼠标轮询结算) ----
    pub drag_insert: Option<DragInsert>,
    pub drop_hint: Option<DropHint>,
    /// 场景拨杆位 (试驾场流动数据形态: normal/jet/gear/nodata)。
    /// 场景 UI 已随组件面板简化撤下 — 恒 "normal" (feed_pages_sim 照读)
    pub sim_scenario: String,
}

impl EditSession {
    pub fn new(docs: Vec<PageDoc>, forced_open: Vec<String>) -> Self {
        let target_page = docs
            .first()
            .map(|d| d.id.clone())
            .unwrap_or_default();
        EditSession {
            target_page,
            docs,
            selection: Vec::new(),
            hover: None,
            gesture: None,
            forced_open,
            snapping: true,
            show_guides: true,
            marquee_mode: false,
            hit_rects: Vec::new(),
            canvas_off: (0, 0),
            pending: VecDeque::new(),
            // begin 后首帧装饰保险渲染 (编辑窗全开后的选中/装饰初始态)
            doc_dirty: true,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            gesture_snapshot: None,
            menu: None,
            menu_font: None,
            need_rebuild: false,
            drag_insert: None,
            drop_hint: None,
            sim_scenario: "normal".to_string(),
        }
    }

    pub fn target_doc(&self) -> Option<&PageDoc> {
        self.docs.iter().find(|d| d.id == self.target_page)
    }

    pub fn target_doc_mut(&mut self) -> Option<&mut PageDoc> {
        self.docs.iter_mut().find(|d| d.id == self.target_page)
    }
}

// =====================================================================
// 命中测试 (缓存矩形; on_press 立即裁决路径)
// =====================================================================

#[derive(Debug, PartialEq)]
enum Hit {
    /// resize 手柄 (单选组件的 8 向)
    Handle(HandleKind),
    /// 组件本体
    Component(String),
    /// 空白 (host 整窗拖拽)
    Blank,
}

/// 手柄中心 (窗口系; rect = 画布系 + off 平移)
fn handle_centers(r: (i32, i32, i32, i32), off: (i32, i32)) -> [(HandleKind, i32, i32); 8] {
    let (x, y, w, h) = (r.0 + off.0, r.1 + off.1, r.2, r.3);
    let (cx, cy) = (x + w / 2, y + h / 2);
    [
        (HandleKind::N, cx, y),
        (HandleKind::S, cx, y + h),
        (HandleKind::E, x + w, cy),
        (HandleKind::W, x, cy),
        (HandleKind::Ne, x + w, y),
        (HandleKind::Nw, x, y),
        (HandleKind::Se, x + w, y + h),
        (HandleKind::Sw, x, y + h),
    ]
}

fn hit_test(s: &EditSession, win_local: (i32, i32)) -> Hit {
    let (lx, ly) = win_local;
    // ① 手柄 (仅单选)
    if s.selection.len() == 1 {
        if let Some((_, x, y, w, h)) = s
            .hit_rects
            .iter()
            .find(|(id, ..)| id == &s.selection[0])
        {
            let r = (*x, *y, *w, *h);
            for (kind, hx, hy) in handle_centers(r, s.canvas_off) {
                if (lx - hx).abs() <= HANDLE_HIT / 2 && (ly - hy).abs() <= HANDLE_HIT / 2 {
                    return Hit::Handle(kind);
                }
            }
        }
    }
    // ② 组件本体 (缓存序 = doc 逆序, z 顶层优先), 2px 容差
    for (id, x, y, w, h) in &s.hit_rects {
        let (x, y) = (x + s.canvas_off.0, y + s.canvas_off.1);
        if lx >= x - 2 && lx <= x + w + 2 && ly >= y - 2 && ly <= y + h + 2 {
            return Hit::Component(id.clone());
        }
    }
    Hit::Blank
}

/// 屏幕 → 窗口局部
#[inline]
fn win_local(root: (i32, i32), win_pos: (i32, i32)) -> (i32, i32) {
    (root.0 - win_pos.0, root.1 - win_pos.1)
}

// =====================================================================
// on_press 裁决 (EditBridge 闭包内同步调用 — 只碰 session 缓存)
// =====================================================================

/// 返回 true = 编辑层接管 (host 不启整窗拖拽)
/// 专用编排器页 (MiniHUD: 组件树由 minihud_overlay_spec 独占管理, 不走
/// PageOverlay 通用面 → 编辑层无命中/无装饰/无重装配支持, 排除出可编辑集)
pub(crate) fn is_editable_page(id: &str) -> bool {
    id != "minihud-default"
}

pub(crate) fn press_decision(s: &mut EditSession, entry_id: &str, root: (i32, i32), win_pos: (i32, i32)) -> bool {
    // 点非目标页窗口 = 切目标页 (装饰转移; 编辑仓不变);
    // 专用编排器页不可编辑 (不切 — 点击只作为普通窗口交互)
    if !is_editable_page(entry_id) {
        return false;
    }
    if entry_id != s.target_page {
        if s.docs.iter().any(|d| d.id == entry_id) {
            s.target_page = entry_id.to_string();
            s.selection.clear();
            s.hover = None;
            s.menu = None; // 跨页点击 = 菜单随之关闭
            s.doc_dirty = true; // 装饰转移 → 保险渲染一帧
        }
        return false; // 切页那一击不启动编辑手势 (也不拖窗, 自然落点)
    }
    // 菜单模态: 菜单在场时点击全归菜单 (执行/关闭在 Release 结算), 不启手势不拖窗
    if s.menu.is_some() {
        return true;
    }
    let local = win_local(root, win_pos);
    match hit_test(s, local) {
        Hit::Handle(kind) => {
            if let Some((_, x, y, w, h)) = s
                .hit_rects
                .iter()
                .find(|(id, ..)| s.selection.first() == Some(id))
            {
                // 起点 pos 单位 (doc 直查; origin 位移换算基准)
                let start_unit = s
                    .target_doc()
                    .and_then(|d| d.components.iter().find(|c| c.id == s.selection[0]))
                    .map(|c| c.pos)
                    .unwrap_or([0.0, 0.0]);
                s.gesture = Some(EditGesture::Resize {
                    id: s.selection[0].clone(),
                    handle: kind,
                    start_root: root,
                    start_rect: (*x, *y, *w, *h),
                    start_unit,
                    last_apply: Instant::now(),
                });
                s.gesture_snapshot = Some(s.docs.clone()); // 撤销栈: 手势起手快照
                return true;
            }
            false
        }
        Hit::Component(id) => {
            // 未选中 → 选它 (单选); 已在选中集 → 保持 (可能多选拖动)
            if !s.selection.contains(&id) {
                s.selection = vec![id.clone()];
                s.doc_dirty = true;
            }
            let start_units: Vec<(String, [f64; 2])> = {
                let Some(doc) = s.target_doc() else { return false };
                s.selection
                    .iter()
                    .filter_map(|sid| {
                        doc.components
                            .iter()
                            .find(|c| &c.id == sid)
                            .map(|c| (sid.clone(), c.pos))
                    })
                    .collect()
            };
            s.gesture = Some(EditGesture::Move {
                ids: s.selection.clone(),
                start_root: root,
                start_units,
                guide_x: None,
                guide_y: None,
            });
            s.gesture_snapshot = Some(s.docs.clone()); // 撤销栈: 手势起手快照
            true
        }
        Hit::Blank => {
            if s.marquee_mode {
                let canvas = (local.0 - s.canvas_off.0, local.1 - s.canvas_off.1);
                s.gesture = Some(EditGesture::Marquee {
                    start_canvas: canvas,
                    cur_canvas: canvas,
                });
                true
            } else {
                // 空白点击 = 清选 + host 拖整窗 (位置持久化走 PageDoc.pos 链)
                s.selection.clear();
                s.doc_dirty = true;
                false
            }
        }
    }
}

// =====================================================================
// 右键菜单 (对象级操作统一入口 — 降心智负担主战场)
// =====================================================================

/// 建菜单: 右键目标分型 (容器 → 排列族; 项 → 显隐/排序/删除族)。
/// 布局同源: 开菜单时算好条目 rect (窗口视图系), 绘制与命中共用
fn build_menu(s: &EditSession, target: &str, at: (i32, i32)) -> Option<EditMenu> {
    let doc = s.target_doc()?;
    let comp = doc.components.iter().find(|c| c.id == target)?;
    let font = s.menu_font.as_ref()?;
    let mk = |label: String, action: MenuAction| (label, action);

    let entries: Vec<(String, MenuAction)> = if comp.r#type == LIST_CONTAINER_TYPE {
        // 块菜单: 排列是选择不是搭建 (单列/两列/三列/自动换行; 当前项打勾)
        let cur = comp
            .props
            .get("arrange")
            .and_then(|v| v.as_str())
            .unwrap_or("column");
        let mark = |key: &str| if cur == key { "✓ " } else { "　" };
        let arrange_of = |arrange: &str, columns: i64| MenuAction::SetArrange {
            container: target.to_string(),
            arrange: arrange.to_string(),
            columns,
        };
        vec![
            mk(format!("{}排列 · 单列", mark("column")), arrange_of("column", 0)),
            mk(format!("{}排列 · 两列", mark("columns")), arrange_of("columns", 2)),
            mk(format!("{}排列 · 三列", mark("columns")), arrange_of("columns", 3)),
            mk(format!("{}排列 · 自动换行", mark("wrap")), arrange_of("wrap", 0)),
        ]
    } else {
        // 项菜单: 上移/下移 (容器内 = 排列序) + 隐藏/显示 + 删除 (选中集整删)
        let delete_ids = if s.selection.len() > 1 && s.selection.contains(&target.to_string()) {
            s.selection.clone()
        } else {
            vec![target.to_string()]
        };
        let hidden = !comp.enabled;
        vec![
            mk("　上移".into(), MenuAction::MoveOrder { id: target.to_string(), delta: -1 }),
            mk("　下移".into(), MenuAction::MoveOrder { id: target.to_string(), delta: 1 }),
            mk(
                if hidden { "　显示".into() } else { "　隐藏".into() },
                MenuAction::ToggleEnabled { id: target.to_string() },
            ),
            mk("　删除".into(), MenuAction::Delete { ids: delete_ids }),
        ]
    };

    // 布局: 宽 = 最宽标签 + 双侧留白; 行 rect 自上而下
    let text_w = entries
        .iter()
        .map(|(label, _)| font.measure(label))
        .max()
        .unwrap_or(60);
    let menu_w = text_w + MENU_PAD_X * 2;
    let items = entries
        .into_iter()
        .enumerate()
        .map(|(i, (label, action))| {
            let rect = (
                at.0,
                at.1 + i as i32 * MENU_ROW_H,
                menu_w,
                MENU_ROW_H,
            );
            (label, action, rect)
        })
        .collect();
    Some(EditMenu {
        origin: at,
        items,
        hover: None,
    })
}

/// 菜单动作执行: 翻译为 EditCommand 走 apply_command (入会话单一撤销栈),
/// 置位 need_rebuild (pump 无重装配面, 主循环收口)
pub(crate) fn execute_menu_action(s: &mut EditSession, action: &MenuAction) {
    let cmd = match action {
        MenuAction::ToggleEnabled { id } => {
            let Some(mut comp) = s
                .target_doc()
                .and_then(|d| d.components.iter().find(|c| c.id == *id).cloned())
            else {
                return;
            };
            comp.enabled = !comp.enabled; // 软隐藏: 保留配置, 建树期跳过
            EditCommand::UpdateComponent {
                comp,
                old_id: Some(id.clone()),
            }
        }
        MenuAction::MoveOrder { id, delta } => {
            // 兄弟段内换位 (容器子项 = 排列序; 自由区 = z 序)。
            // to = 交换目标的 doc 序 (remove+insert 语义下双向同式)
            let Some(doc) = s.target_doc() else { return };
            let Some(pos) = doc.components.iter().position(|c| c.id == *id) else {
                return;
            };
            let parent = doc.components[pos].parent.clone();
            let sibs: Vec<usize> = doc
                .components
                .iter()
                .enumerate()
                .filter(|(_, c)| c.parent == parent)
                .map(|(i, _)| i)
                .collect();
            let Some(k) = sibs.iter().position(|&i| i == pos) else {
                return;
            };
            let tk = (k as i32 + delta).clamp(0, sibs.len() as i32 - 1) as usize;
            if tk == k {
                return; // 已在段端 (无实变不入撤销栈)
            }
            EditCommand::ReorderComponent {
                id: id.clone(),
                to: sibs[tk],
            }
        }
        MenuAction::Delete { ids } => EditCommand::RemoveComponents { ids: ids.clone() },
        MenuAction::SetArrange {
            container,
            arrange,
            columns,
        } => {
            let Some(mut comp) = s
                .target_doc()
                .and_then(|d| d.components.iter().find(|c| c.id == *container).cloned())
            else {
                return;
            };
            comp.props["arrange"] = serde_json::json!(arrange);
            if *columns > 0 {
                comp.props["columns"] = serde_json::json!(columns);
            }
            EditCommand::UpdateComponent {
                comp,
                old_id: Some(container.clone()),
            }
        }
    };
    let _ = apply_command(s, &cmd);
    s.need_rebuild = true;
}

// =====================================================================
// 手势推进 (edit_pump 内 — 可访问 host/handles/页面实例)
// =====================================================================

/// 吸附 (画布 px 域): 网格 round + 对齐参考线 (选中集 bbox 特征线 vs 静止组件)
fn snap_move(
    s: &EditSession,
    raw_dx: i32,
    raw_dy: i32,
    line_height: f64,
) -> (i32, i32, Option<i32>, Option<i32>) {
    let mut dx = raw_dx;
    let mut dy = raw_dy;
    let mut guide_x = None;
    let mut guide_y = None;
    // 移动集当前矩形 (缓存 + 平移)
    let moving: Vec<(i32, i32, i32, i32)> = s
        .hit_rects
        .iter()
        .filter(|(id, ..)| s.selection.contains(id))
        .map(|(_, x, y, w, h)| (x + raw_dx, y + raw_dy, *w, *h))
        .collect();
    // 网格: 首个移动组件原点对齐 0.1 行高
    if let Some((x, _, _, _)) = moving.first() {
        if let Some((_, ox, _, _, _)) = s
            .hit_rects
            .iter()
            .find(|(id, ..)| s.selection.first() == Some(id))
        {
            let grid = (line_height * SNAP).round().max(1.0) as i32;
            let gx = (*x / grid) * grid - *ox;
            if (gx - raw_dx).abs() <= ALIGN_TOL {
                dx = gx;
            }
        }
    }
    if let Some((_, y, _, _)) = moving.first() {
        if let Some((_, _, oy, _, _)) = s
            .hit_rects
            .iter()
            .find(|(id, ..)| s.selection.first() == Some(id))
        {
            let grid = (line_height * SNAP).round().max(1.0) as i32;
            let gy = (*y / grid) * grid - *oy;
            if (gy - raw_dy).abs() <= ALIGN_TOL {
                dy = gy;
            }
        }
    }
    // 对齐线: 移动集 bbox 6 特征线 × 静止组件同特征线 (x 轴)
    let bbox = |rs: &[(i32, i32, i32, i32)]| {
        let mut b = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (x, y, w, h) in rs {
            b.0 = b.0.min(*x);
            b.1 = b.1.min(*y);
            b.2 = b.2.max(x + w);
            b.3 = b.3.max(y + h);
        }
        b
    };
    let statics: Vec<(i32, i32, i32, i32)> = s
        .hit_rects
        .iter()
        .filter(|(id, ..)| !s.selection.contains(id))
        .map(|(_, x, y, w, h)| (*x, *y, *w, *h))
        .collect();
    if !moving.is_empty() {
        let mb = bbox(&moving);
        let marks_x = [mb.0, (mb.0 + mb.2) / 2, mb.2];
        let marks_y = [mb.1, (mb.1 + mb.3) / 2, mb.3];
        let mut best_x: Option<(i32, i32)> = None; // (差, 线位)
        let mut best_y: Option<(i32, i32)> = None;
        for (sx, sy, sw, sh) in &statics {
            let (sx, sw) = (*sx, *sw);
            for t in [sx, sx + sw / 2, sx + sw] {
                for m in marks_x {
                    let d = t - m;
                    if d.abs() <= ALIGN_TOL && best_x.map(|b| d.abs() < b.0.abs()).unwrap_or(true) {
                        best_x = Some((d, t));
                    }
                }
            }
            let (sy, sh) = (*sy, *sh);
            for t in [sy, sy + sh / 2, sy + sh] {
                for m in marks_y {
                    let d = t - m;
                    if d.abs() <= ALIGN_TOL && best_y.map(|b| d.abs() < b.0.abs()).unwrap_or(true) {
                        best_y = Some((d, t));
                    }
                }
            }
        }
        if let Some((d, t)) = best_x {
            dx += d;
            guide_x = Some(t);
        }
        if let Some((d, t)) = best_y {
            dy += d;
            guide_y = Some(t);
        }
    }
    (dx, dy, guide_x, guide_y)
}

/// resize: 手柄方位 × 拖动量 → 新矩形 (画布系)
fn resize_rect(
    start: (i32, i32, i32, i32),
    handle: HandleKind,
    d: (i32, i32),
) -> (i32, i32, i32, i32) {
    let (mut x, mut y, mut w, mut h) = start;
    let (dx, dy) = d;
    use HandleKind::*;
    if matches!(handle, E | Ne | Se) {
        w = (w + dx).max(RESIZE_MIN);
    }
    if matches!(handle, W | Nw | Sw) {
        let nw = (w - dx).max(RESIZE_MIN);
        x += w - nw;
        w = nw;
    }
    if matches!(handle, S | Se | Sw) {
        h = (h + dy).max(RESIZE_MIN);
    }
    if matches!(handle, N | Ne | Nw) {
        let nh = (h - dy).max(RESIZE_MIN);
        y += h - nh;
        h = nh;
    }
    (x, y, w, h)
}

/// 编辑泵 (渲染线程主循环每轮): 消费事件队列 → 推进手势 (直改页面实例)
/// → 刷新缓存 → 即时渲染。host/handles 从 RenderSession 传入。
/// (阶段 C: web 镜像推送步已随编辑面板退役)
pub(crate) fn edit_pump(
    s: &mut EditSession,
    host: &mut OverlayHost,
    pages: &[(String, PageHandle)],
) {
    // ⓪ 拖放插入推进 (组件库拖出: 全局鼠标轮询 → 落点反馈 → 释放结算)
    let mut need_render = advance_drag_insert(s, host);
    // ① 取事件 (entry_id 限目标页 — 其它页事件只用于 hover 判定, 简化丢弃)
    let mut events: Vec<(String, EditMouse)> = Vec::new();
    while let Some((id, ev)) = s.pending.pop_front() {
        events.push((id, ev));
    }
    // ② 手势推进
    for (entry_id, ev) in events {
        if entry_id != s.target_page {
            continue;
        }
        let Some(page) = pages.iter().find(|(id, _)| id == &entry_id).map(|(_, h)| h.clone())
        else {
            continue;
        };
        match ev {
            EditMouse::Move { x, y } => {
                let win_pos = entry_window_pos(host, &entry_id).unwrap_or((0, 0));
                let local = (x - win_pos.0, y - win_pos.1);
                // 菜单模态: Move 只驱动条目悬停 (手势不推进)
                if let Some(menu) = s.menu.as_mut() {
                    let h = menu.items.iter().position(|(_, _, (rx, ry, rw, rh))| {
                        local.0 >= *rx
                            && local.0 <= rx + rw
                            && local.1 >= *ry
                            && local.1 <= ry + rh
                    });
                    if h != menu.hover {
                        menu.hover = h;
                        need_render = true;
                    }
                    continue;
                }
                // hover 更新 (无手势时) / 手势推进
                match s.gesture.is_some() {
                    false => {
                        let h = match hit_test(s, local) {
                            Hit::Component(id) => Some(id),
                            _ => None,
                        };
                        if h != s.hover {
                            s.hover = h;
                            need_render = true;
                        }
                    }
                    true => {
                        // 手势推进 (Move 用屏幕差分; Marquee 需画布系 — 传 local)
                        advance_gesture(s, &page, (x, y), local, host, &mut need_render);
                    }
                }
            }
            EditMouse::Release => {
                // 菜单结算: 悬停条目执行动作, 空白点击 = 关闭 (press_decision 已接管)
                if let Some(menu) = s.menu.take() {
                    if let Some(i) = menu.hover {
                        let (_, action, _) = &menu.items[i];
                        let action = action.clone();
                        execute_menu_action(s, &action);
                        need_render = true;
                    }
                    s.doc_dirty = true;
                }
                if let Some(g) = s.gesture.take() {
                    match g {
                        EditGesture::Marquee { start_canvas, cur_canvas } => {
                            // 框选完成: 与缓存矩形求交 → 选中集 (位移 < 3px 视为
                            // 空白点击 = 清选, 已在 press_decision 处理)
                            let sel = {
                                let (x0, y0) = (
                                    start_canvas.0.min(cur_canvas.0),
                                    start_canvas.1.min(cur_canvas.1),
                                );
                                let (x1, y1) = (
                                    start_canvas.0.max(cur_canvas.0),
                                    start_canvas.1.max(cur_canvas.1),
                                );
                                s.hit_rects
                                    .iter()
                                    .filter(|(_, x, y, w, h)| {
                                        *x < x1 && x + w > x0 && *y < y1 && y + h > y0
                                    })
                                    .map(|(id, ..)| id.clone())
                                    .collect::<Vec<_>>()
                            };
                            s.selection = sel;
                        }
                        _ => {
                            // Move/Resize 结束: doc 已在推进中同步;
                            // 起手快照有实变 → 入撤销栈 (整段拖拽 = 一项)
                            if let Some(snap) = s.gesture_snapshot.take() {
                                if snap != s.docs {
                                    s.undo_stack.push(snap);
                                    s.redo_stack.clear();
                                }
                            }
                        }
                    }
                    s.doc_dirty = true;
                    need_render = true;
                }
            }
            EditMouse::RightPress { x, y } => {
                // 对象级菜单: 命中组件 → 分型建菜单 (容器=排列族/项=显隐排序删除族);
                // 空白 = 关菜单 + 清选 (原语义)
                let win_pos = entry_window_pos(host, &entry_id).unwrap_or((0, 0));
                let local = (x - win_pos.0, y - win_pos.1);
                if s.menu.is_some() {
                    s.menu = None; // 菜单已开 → 再右键 = 关闭
                } else {
                    let target = match hit_test(s, local) {
                        Hit::Component(id) => Some(id),
                        _ => None,
                    };
                    match target {
                        Some(id) => {
                            if let Some(m) = build_menu(s, &id, local) {
                                // 右键目标入选 (单选 — 菜单操作对象可视化)
                                s.selection = vec![id];
                                s.menu = Some(m);
                            }
                        }
                        None => {
                            s.selection.clear();
                        }
                    }
                }
                s.doc_dirty = true;
                need_render = true;
            }
            EditMouse::DoubleClick { .. } => {
                // 双击暂无语义 — 兜底清选
                s.selection.clear();
                s.doc_dirty = true;
                need_render = true;
            }
        }
    }
    // ③ 刷新缓存 (hit_rects / canvas_off)
    refresh_cache(s, pages);
    // ④ 即时渲染 (~100Hz 跟手) — 仅在有渲染面活动时 (事件/手势/装饰脏);
    // 静止时不额外 tick (主循环 50ms 常规节拍足够, 省下每 10ms 全页重画)。
    // doc_dirty = 装饰面脏 (doc/selection 变化, 含 edit_chrome 的 Select 类
    // 无重装配路径), 渲染一帧后清除 — 原 80ms 节流 web 镜像推送已退役
    if need_render || s.doc_dirty || s.gesture.is_some() {
        let _ = host.render_tick();
        s.doc_dirty = false;
    }
}

/// 条目窗口位置 (host 直查; 窗口未开 = None → 屏幕系退化)
fn entry_window_pos(host: &OverlayHost, id: &str) -> Option<(i32, i32)> {
    host.entry_position(id)
}

// =====================================================================
// 拖放插入 (组件库 webview → 真窗; 全局鼠标轮询 — 编辑泵 ⓪ 步)
// =====================================================================

/// 拖放推进: 左键按住 → 算落点反馈; 释放 → 结算落组件。
/// 返回是否需要即时渲染 (反馈变化/结算)
fn advance_drag_insert(s: &mut EditSession, host: &OverlayHost) -> bool {
    let Some(drag) = s.drag_insert.clone() else {
        return false;
    };
    if overlay::platform::cursor::left_down() {
        let hint = compute_drop_hint(s, host, overlay::platform::cursor::cursor_pos());
        if hint != s.drop_hint {
            s.drop_hint = hint;
            return true;
        }
        return false;
    }
    // 释放结算: 有效落点 → 构造组件入编辑仓 (单一撤销栈); 悬在窗口外 = 取消
    let mut need = false;
    if let Some(hint) = s.drop_hint.take() {
        if let Some(cmd) = build_insert_command(s, &drag, &hint) {
            let _ = apply_command(s, &cmd);
            s.need_rebuild = true;
            need = true;
        }
    }
    s.drag_insert = None;
    need
}

/// 落点反馈计算: 屏幕坐标 → 命中编辑页 → 分型 (容器子项=插入线 /
/// 容器本体=尾插线 / 自由区=幽灵)。命中他页 = 目标页切换
fn compute_drop_hint(s: &mut EditSession, host: &OverlayHost, pos: (i32, i32)) -> Option<DropHint> {
    // 命中窗口 (可编辑页; 悬在工具条/面板/桌面 = 无落点)
    let mut hit_entry: Option<(String, (i32, i32), (i32, i32))> = None;
    for d in &s.docs {
        if !is_editable_page(&d.id) {
            continue;
        }
        if let (Some(wp), Some(sz)) = (host.entry_position(&d.id), host.entry_size(&d.id)) {
            if pos.0 >= wp.0 && pos.0 <= wp.0 + sz.0 && pos.1 >= wp.1 && pos.1 <= wp.1 + sz.1 {
                hit_entry = Some((d.id.clone(), wp, sz));
                break;
            }
        }
    }
    let (entry, win, _size) = hit_entry?;
    if entry != s.target_page {
        s.target_page = entry;
        s.selection.clear();
        s.hover = None;
        s.menu = None;
        s.doc_dirty = true;
        s.hit_rects.clear(); // 换页后旧矩形失效 (refresh_cache 在泵尾补)
        return None;
    }
    let local = (pos.0 - win.0, pos.1 - win.1);
    let canvas = (local.0 - s.canvas_off.0, local.1 - s.canvas_off.1);
    // 组件直命中 (跳过 resize 手柄 — 拖放期间手柄语义无关; hit_rects 画布系,
    // doc 逆序 = z 顶层优先), 2px 容差
    let hit = s.hit_rects.iter().find(|(_, x, y, w, h)| {
        canvas.0 >= x - 2 && canvas.0 <= x + w + 2 && canvas.1 >= y - 2 && canvas.1 <= y + h + 2
    });
    let Some(doc) = s.target_doc() else { return None };
    let ghost = || {
        let snap = 10i32; // 幽灵吸附 10px 网格 (视觉稳定)
        Some(DropHint::Ghost {
            x: (canvas.0 / snap) * snap,
            y: (canvas.1 / snap) * snap,
            w: 140,
            h: 30,
        })
    };
    let Some((id, x, y, w, h)) = hit else {
        return ghost(); // 空白 = 自由放置
    };
    let comp = doc.components.iter().find(|c| c.id == *id)?;
    let is_container = |c: &ComponentDoc| c.r#type == LIST_CONTAINER_TYPE;
    if is_container(comp) {
        // 容器本体 → 尾插 (线 = 容器底缘)
        let index = doc
            .components
            .iter()
            .filter(|c| c.parent.as_deref() == Some(id.as_str()))
            .count();
        return Some(DropHint::InsertLine {
            container: id.clone(),
            index,
            x0: *x,
            y: y + h,
            x1: x + w,
        });
    }
    if let Some(parent) = &comp.parent {
        let in_container = doc
            .components
            .iter()
            .any(|c| c.id == *parent && is_container(c));
        if in_container {
            // 容器子项 → 上半前插 / 下半后插 (线 = 项顶/项底)
            let sibs: Vec<&str> = doc
                .components
                .iter()
                .filter(|c| c.parent.as_deref() == Some(parent.as_str()))
                .map(|c| c.id.as_str())
                .collect();
            let k = sibs.iter().position(|&sid| sid == id.as_str())?;
            let upper = canvas.1 < y + h / 2;
            let index = if upper { k } else { k + 1 };
            let line_y = if upper { *y } else { y + h };
            return Some(DropHint::InsertLine {
                container: parent.clone(),
                index,
                x0: *x,
                y: line_y,
                x1: x + w,
            });
        }
    }
    ghost() // 自由区组件/根链项 = 自由放置
}

/// 释放结算: 落点反馈 → InsertComponent 命令 (含 doc 插入位; 单一撤销栈)
pub(crate) fn build_insert_command(
    s: &EditSession,
    drag: &DragInsert,
    hint: &DropHint,
) -> Option<EditCommand> {
    let doc = s.target_doc()?;
    // 页内唯一 id (c1, c2… 找空位)
    let mut n = doc.components.len() + 1;
    while doc.components.iter().any(|c| c.id == format!("c{n}")) {
        n += 1;
    }
    match hint {
        DropHint::InsertLine { container, index, .. } => {
            // 子项插入序 → doc 绝对位 (容器子项区段内)
            let container_idx = doc.components.iter().position(|c| c.id == *container)?;
            let sibs: Vec<usize> = doc
                .components
                .iter()
                .enumerate()
                .filter(|(_, c)| c.parent.as_deref() == Some(container.as_str()))
                .map(|(i, _)| i)
                .collect();
            let at = if sibs.is_empty() {
                container_idx + 1
            } else if *index >= sibs.len() {
                sibs[sibs.len() - 1] + 1
            } else {
                sibs[*index]
            };
            let comp = ComponentDoc {
                id: format!("c{n}"),
                r#type: drag.type_name.clone(),
                pos: [0.0, 0.0], // 容器内 = 顺序语义, pos/anchor 忽略
                anchor: ["TopLeft".into(), "TopLeft".into()],
                parent: Some(container.clone()),
                props: drag.props.clone(),
                ..Default::default()
            };
            Some(EditCommand::InsertComponent { comp, at })
        }
        DropHint::Ghost { x, y, .. } => {
            // 自由放置: 画布 px → pos 单位 (line_height 基), 0.1 网格吸附
            let lh = s
                .menu_font
                .as_ref()
                .map(|f| f.size as f64)
                .unwrap_or(20.0)
                .max(1.0);
            let snap_unit = |v: f64| (v / SNAP).round() * SNAP;
            let comp = ComponentDoc {
                id: format!("c{n}"),
                r#type: drag.type_name.clone(),
                pos: [snap_unit(*x as f64 / lh), snap_unit(*y as f64 / lh)],
                anchor: ["TopLeft".into(), "TopLeft".into()],
                parent: None,
                props: drag.props.clone(),
                ..Default::default()
            };
            Some(EditCommand::InsertComponent {
                comp,
                at: doc.components.len(),
            })
        }
    }
}

/// 手势推进核心 (直改页面节点 — 真渲染面; local = 窗口局部坐标, Marquee 用)
fn advance_gesture(
    s: &mut EditSession,
    page: &PageHandle,
    root: (i32, i32),
    local: (i32, i32),
    host: &mut OverlayHost,
    need_render: &mut bool,
) {
    let lh = page.borrow().line_height();
    let gesture = s.gesture.take();
    match gesture {
        Some(EditGesture::Move {
            ids,
            start_root,
            start_units,
            ..
        }) => {
            let (raw_dx, raw_dy) = (root.0 - start_root.0, root.1 - start_root.1);
            let (dx, dy, guide_x, guide_y) = if s.snapping {
                let r = snap_move(s, raw_dx, raw_dy, lh);
                (r.0, r.1, r.2, r.3)
            } else {
                (raw_dx, raw_dy, None, None)
            };
            {
                let p = page.borrow_mut();
                for (id, [sx, sy]) in &start_units {
                    if let Some(node) = p.layout.engine.get_node(id) {
                        let nx = snap_unit(sx + dx as f64 / lh);
                        let ny = snap_unit(sy + dy as f64 / lh);
                        node.set_relative_position(nx, ny);
                    }
                }
                // doc 同步 (提交时的真相)
                if let Some(doc) = s.target_doc_mut() {
                    for c in doc.components.iter_mut() {
                        if let Some((_, [sx, sy])) = start_units.iter().find(|(i, _)| i == &c.id) {
                            c.pos = [
                                snap_unit(sx + dx as f64 / lh),
                                snap_unit(sy + dy as f64 / lh),
                            ];
                        }
                    }
                }
            }
            // 窗口包围盒跟随 (轻: 每 Move 一次 apply_auto_sizing + 条件 resize)
            follow_sizing(s, page, host, &s.target_page);
            s.gesture = Some(EditGesture::Move {
                ids,
                start_root,
                start_units,
                guide_x,
                guide_y,
            });
            *need_render = true;
        }
        Some(EditGesture::Resize {
            id,
            handle,
            start_root,
            start_rect,
            start_unit,
            last_apply,
        }) => {
            let d = (root.0 - start_root.0, root.1 - start_root.1);
            let (x, y, w, h) = resize_rect(start_rect, handle, d);
            // origin 位移 (N/W 向手柄拖动改变起点) → 新 pos = 起点 + 位移/行高
            let lh = page.borrow().line_height().max(1.0);
            let new_pos = [
                snap_unit(start_unit[0] + (x - start_rect.0) as f64 / lh),
                snap_unit(start_unit[1] + (y - start_rect.1) as f64 / lh),
            ];
            {
                let p = page.borrow_mut();
                if let Some(node) = p.layout.engine.get_node(&id) {
                    node.set_relative_position(new_pos[0], new_pos[1]);
                }
                if let Some(cell) = p.cells.get(&id) {
                    cell.set_size_override(Some((w, h)));
                }
                if let Some(doc) = s.target_doc_mut() {
                    if let Some(c) = doc.components.iter_mut().find(|c| c.id == id) {
                        c.size = Some([w, h]);
                        c.pos = new_pos;
                    }
                }
            }
            // DIB 重建昂贵: 33ms 节流; 松手 (Release) 补终值
            if last_apply.elapsed() >= Duration::from_millis(33) {
                follow_sizing(s, page, host, &s.target_page);
                s.gesture = Some(EditGesture::Resize {
                    id,
                    handle,
                    start_root,
                    start_rect,
                    start_unit,
                    last_apply: Instant::now(),
                });
            } else {
                s.gesture = Some(EditGesture::Resize {
                    id,
                    handle,
                    start_root,
                    start_rect,
                    start_unit,
                    last_apply,
                });
            }
            *need_render = true;
        }
        Some(EditGesture::Marquee { start_canvas, .. }) => {
            // 画布系当前点 (窗口局部 − auto-sizing 偏移; 与 on_press 起点同系)
            s.gesture = Some(EditGesture::Marquee {
                start_canvas,
                cur_canvas: (local.0 - s.canvas_off.0, local.1 - s.canvas_off.1),
            });
            *need_render = true;
        }
        None => {}
    }
}

#[inline]
fn snap_unit(v: f64) -> f64 {
    (v / SNAP).round() * SNAP
}

/// 窗口包围盒跟随 (拖动/resize 后重算布局 + 条件 resize_entry)
fn follow_sizing(s: &EditSession, page: &PageHandle, host: &mut OverlayHost, entry_id: &str) {
    let padding = s
        .target_doc()
        .map(|d| d.padding)
        .unwrap_or(45);
    let Some((w, h)) = page.borrow_mut().refresh_sizing(padding) else {
        return;
    };
    if let Some(cur) = host.entry_size(entry_id) {
        if cur != (w, h) {
            let _ = host.resize_entry(entry_id, w, h);
        }
    }
}

/// 刷新渲染面缓存 (hit_rects: doc 逆序 z 顶层优先; canvas_off)
fn refresh_cache(s: &mut EditSession, pages: &[(String, PageHandle)]) {
    let Some(page) = pages
        .iter()
        .find(|(id, _)| *id == s.target_page)
        .map(|(_, h)| h.clone())
    else {
        s.hit_rects.clear();
        return;
    };
    let Some(doc) = s.target_doc() else {
        s.hit_rects.clear();
        return;
    };
    let p = page.borrow();
    let off = p
        .sizing()
        .map(|sz| (sz.offset_x, sz.offset_y))
        .unwrap_or((0, 0));
    let mut rects = Vec::with_capacity(doc.components.len());
    for comp in doc.components.iter().rev() {
        if let Some(node) = p.layout.engine.get_node(&comp.id) {
            let r = node.get_pixel_rect();
            rects.push((comp.id.clone(), r.x, r.y, r.width, r.height));
        }
    }
    drop(p);
    s.canvas_off = off;
    s.hit_rects = rects;
}

// =====================================================================
// 装饰绘制 (on_paint 闭包: 只读 session 缓存)
// =====================================================================

pub(crate) fn paint_decorations(s: &EditSession, entry_id: &str, cv: &mut PixCanvas) {
    if entry_id != s.target_page {
        return;
    }
    let off = s.canvas_off;
    let to_win = |x: i32, y: i32, w: i32, h: i32| (x + off.0, y + off.1, w, h);
    // hover 框 (1px 半透明)
    if let Some(h) = &s.hover {
        if !s.selection.contains(h) {
            if let Some((_, x, y, w, hh)) = s.hit_rects.iter().find(|(id, ..)| id == h) {
                let (x, y, w, hh) = to_win(*x, *y, *w, *hh);
                primitives::ring1px(cv, x, y, w, hh, [255, 105, 180, 120]);
            }
        }
    }
    // 参考线 (拖动中)
    if let Some(EditGesture::Move {
        guide_x, guide_y, ..
    }) = &s.gesture
    {
        if s.show_guides {
            let (_, _, ch, _) = ((), (), cv.height(), ());
            if let Some(gx) = guide_x {
                primitives::dash_vline(cv, gx + off.0, 0, ch as i32, ACCENT);
            }
            if let Some(gy) = guide_y {
                primitives::dash_hline(cv, gy + off.1, 0, cv.width() as i32, ACCENT);
            }
        }
    }
    // 选中框 (2px) + 手柄 (单选)
    for (id, x, y, w, h) in &s.hit_rects {
        if s.selection.contains(id) {
            let (x, y, w, h) = to_win(*x, *y, *w, *h);
            primitives::ring2px(cv, x, y, w, h, ACCENT);
            if s.selection.len() == 1 {
                for (_, hx, hy) in handle_centers((x, y, w, h), (0, 0)) {
                    primitives::handle6(cv, hx, hy, ACCENT);
                }
            }
        }
    }
    // 框选 rubber band
    if let Some(EditGesture::Marquee {
        start_canvas,
        cur_canvas,
    }) = &s.gesture
    {
        let (x0, y0) = (start_canvas.0 + off.0, start_canvas.1 + off.1);
        let (x1, y1) = (cur_canvas.0 + off.0, cur_canvas.1 + off.1);
        let (x, y) = (x0.min(x1), y0.min(y1));
        let (w, h) = ((x1 - x0).abs(), (y1 - y0).abs());
        cv.fill_rect(x, y, w, h, [255, 105, 180, 20]);
        primitives::ring1px(cv, x, y, w, h, ACCENT);
    }
    // 右键菜单 (画进渲染帧 — 与装饰同形态; 最顶层)
    if let Some(menu) = &s.menu {
        let Some(font) = &s.menu_font else {
            return;
        };
        let menu_h = menu.items.len() as i32 * MENU_ROW_H;
        let Some(mw) = menu.items.first().map(|(_, _, r)| r.2) else {
            return;
        };
        // 面板底 + 1px 边
        cv.fill_rect(menu.origin.0, menu.origin.1, mw, menu_h, [32, 32, 36, 242]);
        primitives::ring1px(cv, menu.origin.0, menu.origin.1, mw, menu_h, [70, 70, 76, 255]);
        for (i, (label, _, (rx, ry, rw, rh))) in menu.items.iter().enumerate() {
            if menu.hover == Some(i) {
                cv.fill_rect(*rx, *ry, *rw, *rh, [255, 105, 180, 56]);
            }
            let baseline = ry + rh - 7; // 行底内收 (菜单字号 ≈ 行高-14 基线)
            cv.draw_text(font, rx + MENU_PAD_X - 2, baseline, label, [235, 235, 235, 255], true);
        }
    }
    // 拖放落点反馈 (最顶层; 画布系 → 窗口视图 +off)
    if let Some(hint) = &s.drop_hint {
        match hint {
            DropHint::InsertLine { x0, y, x1, .. } => {
                // 插入线: 2px 粗 + 两端小竖须 (容器内插入位)
                let (x0, y, x1) = (x0 + off.0, y + off.1, x1 + off.0);
                cv.fill_rect(x0, y - 1, x1 - x0, 2, ACCENT);
                cv.fill_rect(x0, y - 5, 2, 10, ACCENT);
                cv.fill_rect(x1 - 2, y - 5, 2, 10, ACCENT);
            }
            DropHint::Ghost { x, y, w, h } => {
                let (x, y) = (x + off.0, y + off.1);
                cv.fill_rect(x, y, *w, *h, [255, 105, 180, 40]);
                primitives::ring1px(cv, x, y, *w, *h, ACCENT);
                if let Some(font) = &s.menu_font {
                    if let Some(drag) = &s.drag_insert {
                        cv.draw_text(
                            font,
                            x + 6,
                            y + h - 8,
                            &drag.display_zh,
                            [255, 220, 240, 255],
                            true,
                        );
                    }
                }
            }
        }
    }
}

// =====================================================================
// 命中缓存 (render_thread 重装配后刷新的公开入口)
// =====================================================================

/// 刷新命中缓存 (rebuild_edit_target 后命中矩形/画布偏移追平新装配)
pub(crate) fn refresh_cache_public(s: &mut EditSession, pages: &[(String, PageHandle)]) {
    refresh_cache(s, pages);
}

// =====================================================================
// 命令执行面 (UiCommand 编辑命令的渲染线程处理体)
// =====================================================================

/// 编辑命令执行 (返回渲染面效果分级; Err = 拒绝并回显)
pub(crate) fn apply_command(
    s: &mut EditSession,
    cmd: &EditCommand,
) -> Result<CommandEffect, String> {
    // 撞名预检 (拒绝路径不入撤销栈 — Err 后栈上不留 no-op 快照, 免多按一次撤销)
    match cmd {
        EditCommand::UpdateComponent { comp, old_id } => {
            let old_id = old_id.clone().unwrap_or_else(|| comp.id.clone());
            if comp.id != old_id
                && s.target_doc()
                    .map(|d| d.components.iter().any(|c| c.id == comp.id))
                    .unwrap_or(false)
            {
                return Err(format!("组件 id「{}」已存在", comp.id));
            }
        }
        EditCommand::InsertComponent { comp, .. } => {
            if s.target_doc()
                .map(|d| d.components.iter().any(|c| c.id == comp.id))
                .unwrap_or(false)
            {
                return Err(format!("组件 id「{}」已存在", comp.id));
            }
        }
        _ => {}
    }
    // 全局单一撤销栈: 变更类命令执行前入栈 (undo/redo 自身不入栈)
    if matches!(
        cmd,
        EditCommand::InsertComponent { .. }
            | EditCommand::UpdateComponent { .. }
            | EditCommand::RemoveComponents { .. }
            | EditCommand::ReorderComponent { .. }
    ) {
        push_undo_snapshot(s);
    }
    match cmd {
        // ---- 撤销/重做 (编辑仓整体快照交换; 指向修复后整页重装配) ----
        EditCommand::Undo => Ok(if let Some(prev) = s.undo_stack.pop() {
            s.redo_stack.push(std::mem::replace(&mut s.docs, prev));
            fix_session_targets(s);
            s.doc_dirty = true;
            CommandEffect::Full
        } else {
            CommandEffect::None
        }),
        EditCommand::Redo => Ok(if let Some(next) = s.redo_stack.pop() {
            s.undo_stack.push(std::mem::replace(&mut s.docs, next));
            fix_session_targets(s);
            s.doc_dirty = true;
            CommandEffect::Full
        } else {
            CommandEffect::None
        }),
        // ---- 拖放 (落点反馈/结算在编辑泵 ⓪ 步; 发起 = 组件面板直连置 drag_insert) ----
        EditCommand::InsertComponent { comp, at } => {
            if let Some(doc) = s.target_doc_mut() {
                // 撞名守卫 (同 id 双拖); at 越界钳尾
                if doc.components.iter().any(|c| c.id == comp.id) {
                    return Err(format!("组件 id「{}」已存在", comp.id));
                }
                let at = (*at).min(doc.components.len());
                doc.components.insert(at, comp.clone());
                s.selection = vec![comp.id.clone()]; // 落地即选中 (接属性编辑)
            }
            s.doc_dirty = true;
            Ok(CommandEffect::Full)
        }
        EditCommand::UpdateComponent { comp, old_id } => {
            // 整组件替换 (props/改名/size/visibleWhen); 改名同步 parent 引用。
            // old_id 显式传入 (改名 = 新 id 替换旧 id 槽位 — 此前按 comp.id
            // 自匹配, 改名会误变"复制+残留")
            let old_id = old_id.clone().unwrap_or_else(|| comp.id.clone());
            if let Some(doc) = s.target_doc_mut() {
                // 撞名拒绝
                if comp.id != old_id && doc.components.iter().any(|c| c.id == comp.id) {
                    return Err(format!("组件 id「{}」已存在", comp.id));
                }
                let pos = doc.components.iter().position(|c| c.id == old_id);
                if let Some(i) = pos {
                    doc.components[i] = comp.clone();
                } else {
                    doc.components.push(comp.clone());
                }
                if comp.id != old_id {
                    for c in doc.components.iter_mut() {
                        if c.parent.as_deref() == Some(old_id.as_str()) {
                            c.parent = Some(comp.id.clone());
                        }
                    }
                    s.selection = s
                        .selection
                        .iter()
                        .map(|x| if x == &old_id { &comp.id } else { x })
                        .cloned()
                        .collect();
                }
            }
            s.doc_dirty = true;
            Ok(CommandEffect::Full)
        }
        EditCommand::RemoveComponents { ids } => {
            if let Some(doc) = s.target_doc_mut() {
                doc.components.retain(|c| !ids.contains(&c.id));
                for c in doc.components.iter_mut() {
                    if let Some(p) = &c.parent {
                        if ids.contains(p) {
                            c.parent = None;
                        }
                    }
                }
            }
            s.selection.retain(|id| !ids.contains(id));
            s.doc_dirty = true;
            Ok(CommandEffect::Full)
        }
        EditCommand::ReorderComponent { id, to } => {
            if let Some(doc) = s.target_doc_mut() {
                let Some(pos) = doc.components.iter().position(|c| c.id == *id) else {
                    return Ok(CommandEffect::Full);
                };
                let c = doc.components.remove(pos);
                let to = (*to).min(doc.components.len());
                doc.components.insert(to, c);
            }
            s.doc_dirty = true;
            Ok(CommandEffect::Full)
        }
    }
}

/// 命令的渲染面效果 (render_thread 据此决定轻/重路径)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEffect {
    /// 无渲染面变化 (不重装配不即时渲染)
    None,
    /// doc 结构变化 (增删改组件/页面 — 整页重装配)
    Full,
}

/// 撤销栈入栈 (变更前快照; 与栈顶同态跳过防冗余, 新变更清空重做栈)
fn push_undo_snapshot(s: &mut EditSession) {
    if s.undo_stack.last() == Some(&s.docs) {
        return;
    }
    s.undo_stack.push(s.docs.clone());
    s.redo_stack.clear();
}

/// undo/redo 恢复后的指向修复: 目标页失效回退首页, 选中集过滤到目标页现存组件
fn fix_session_targets(s: &mut EditSession) {
    if !s.docs.iter().any(|d| d.id == s.target_page) {
        // 回退取首个可编辑页 (docs.first 可能是 minihud 不可编辑页 —
        // target 落它会令编辑失去对象)
        s.target_page = s
            .docs
            .iter()
            .find(|d| is_editable_page(&d.id))
            .map(|d| d.id.clone())
            .unwrap_or_default();
    }
    let alive: Vec<String> = s
        .target_doc()
        .map(|d| d.components.iter().map(|c| c.id.clone()).collect())
        .unwrap_or_default();
    s.selection.retain(|id| alive.contains(id));
    if let Some(h) = s.hover.clone() {
        if !alive.contains(&h) {
            s.hover = None;
        }
    }
}

/// 编辑命令 (渲染线程内部命令面 — edit_chrome/菜单/拖放结算构造,
/// apply_command 消费; 原 serde 面 (web IPC 载荷中转) 已随阶段 C 退役)。
/// 页面管理/场景拨杆/目标页切换/选择/页面属性族命令已随组件面板简化
/// 撤下 (git 可找回), 见 doc/试驾场原生化方案.md
#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    /// 撤销/重做 (会话级全局单一栈 — 手势/命令/拖放统一入栈)
    Undo,
    Redo,
    /// 落组件 (拖放/点击添加; at = doc 绝对插入位)
    InsertComponent { comp: ComponentDoc, at: usize },
    UpdateComponent {
        comp: ComponentDoc,
        /// 原 id (改名场景; 缺省 = comp.id 即原 id)
        old_id: Option<String>,
    },
    RemoveComponents { ids: Vec<String> },
    ReorderComponent { id: String, to: usize },
}

/// Rc 便捷别名 (RenderSession.edit 与 EditBridge 闭包共享)
pub(crate) type EditSessionRef = Rc<RefCell<EditSession>>;
