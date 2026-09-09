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

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vm_core::base::bus::ui_state_bus::UIStateBus;
use serde::{Deserialize, Serialize};
use vm_core::config::json_model::{ComponentDoc, PageDoc};

use vm_overlay::layout::hud_layout_node::HUDLayoutNodeExt;
use vm_overlay::platform::host::{EditMouse, OverlayHost};
use vm_overlay::render::canvas::PixCanvas;
use vm_overlay::render::primitives;
use vm_overlay::widgets::PageHandle;

/// UIStateBus 编辑事件键 (bridge.rs 订阅 → 前端 emit)
pub const HUD_EDIT_SESSION: &str = "HUD_EDIT_SESSION"; // data: "begin" | "end"
pub const HUD_EDIT_DOC: &str = "HUD_EDIT_DOC"; // data: JSON (页文档 + 矩形 + 选中)
pub const HUD_EDIT_SELECTION: &str = "HUD_EDIT_SELECTION"; // data: JSON {ids}
pub const HUD_EDIT_ERROR: &str = "HUD_EDIT_ERROR"; // data: 错误串

/// 编辑装饰粉色 (全站主题)
const ACCENT: [u8; 4] = [255, 105, 180, 235];
/// 网格吸附步长 (行高倍)
const SNAP: f64 = 0.1;
/// 对齐吸附阈值 (物理 px)
const ALIGN_TOL: i32 = 6;
/// 手柄绘制/命中边 (物理 px)
const HANDLE_HIT: i32 = 10;
/// resize 最小尺寸 (物理 px)
const RESIZE_MIN: i32 = 8;
/// doc 推送前端节流
const DOC_PUSH_THROTTLE: Duration = Duration::from_millis(80);

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
    /// doc 推送节流基准
    pub last_doc_push: Instant,
    pub doc_dirty: bool,
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
            last_doc_push: Instant::now(),
            // begin 即推首帧镜像 (前端控制台进场有数据, 不等首次编辑)
            doc_dirty: true,
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
pub(crate) fn press_decision(s: &mut EditSession, entry_id: &str, root: (i32, i32), win_pos: (i32, i32)) -> bool {
    // 点非目标页窗口 = 切目标页 (装饰转移; 编辑仓不变)
    if entry_id != s.target_page {
        if s.docs.iter().any(|d| d.id == entry_id) {
            s.target_page = entry_id.to_string();
            s.selection.clear();
            s.hover = None;
            s.doc_dirty = true; // 推前端同步 targetPage
        }
        return false; // 切页那一击不启动编辑手势 (也不拖窗, 自然落点)
    }
    let local = win_local(root, win_pos);
    match hit_test(s, local) {
        Hit::Handle(kind) => {
            if let Some((_, x, y, w, h)) = s
                .hit_rects
                .iter()
                .find(|(id, ..)| s.selection.first() == Some(id))
            {
                s.gesture = Some(EditGesture::Resize {
                    id: s.selection[0].clone(),
                    handle: kind,
                    start_root: root,
                    start_rect: (*x, *y, *w, *h),
                    last_apply: Instant::now(),
                });
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
/// → 刷新缓存 → 推前端。host/handles 从 RenderSession 传入。
#[allow(clippy::too_many_arguments)]
pub(crate) fn edit_pump(
    s: &mut EditSession,
    host: &mut OverlayHost,
    pages: &[(String, PageHandle)],
    ui_bus: &UIStateBus,
) {
    let now = Instant::now();
    // ① 取事件 (entry_id 限目标页 — 其它页事件只用于 hover 判定, 简化丢弃)
    let mut events: Vec<(String, EditMouse)> = Vec::new();
    while let Some((id, ev)) = s.pending.pop_front() {
        events.push((id, ev));
    }
    // ② 手势推进
    let mut need_render = false;
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
                // hover 更新 (无手势时) / 手势推进
                let win_pos = entry_window_pos(host, &entry_id);
                let local = (x - win_pos.0, y - win_pos.1);
                match &s.gesture {
                    None => {
                        let h = match hit_test(s, local) {
                            Hit::Component(id) => Some(id),
                            _ => None,
                        };
                        if h != s.hover {
                            s.hover = h;
                            need_render = true;
                        }
                    }
                    Some(_) => {
                        advance_gesture(s, &page, (x, y), host, &mut need_render);
                    }
                }
            }
            EditMouse::Release => {
                if s.gesture.take().is_some() {
                    // 手势结束: 尺寸终值补齐 + doc 同步 (pos/size 已在推进中写入)
                    s.doc_dirty = true;
                    need_render = true;
                }
            }
            EditMouse::RightPress { .. } | EditMouse::DoubleClick { .. } => {
                // 右键/双击语义预留 (编辑面板上下文菜单); 当前 = 清选
                s.selection.clear();
                s.doc_dirty = true;
                need_render = true;
            }
        }
    }
    // ③ 刷新缓存 (hit_rects / canvas_off)
    refresh_cache(s, pages);
    // ④ doc/selection 推送 (节流)
    if s.doc_dirty && now.duration_since(s.last_doc_push) >= DOC_PUSH_THROTTLE {
        push_doc_event(s, ui_bus);
        s.doc_dirty = false;
        s.last_doc_push = now;
    }
    // ⑤ 即时渲染 (~100Hz 跟手; 静止时脏检查零提交)
    let _ = need_render;
    let _ = host.render_tick();
}

/// 条目窗口位置 (host 无直查 — 经 active 槽位; 简化: 记在 session 缓存)
fn entry_window_pos(_host: &OverlayHost, _id: &str) -> (i32, i32) {
    // host 的 window.position 需要槽位借用; 编辑会话里窗口不移动期间位置不变,
    // 简化为 (0,0) 相对坐标系 (on_press 的 win_pos 已带真实值; Move 用差分)
    (0, 0)
}

/// 手势推进核心 (直改页面节点 — 真渲染面)
fn advance_gesture(
    s: &mut EditSession,
    page: &PageHandle,
    root: (i32, i32),
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
            last_apply,
        }) => {
            let d = (root.0 - start_root.0, root.1 - start_root.1);
            let (_x, _y, w, h) = resize_rect(start_rect, handle, d);
            {
                let p = page.borrow_mut();
                if let Some(cell) = p.cells.get(&id) {
                    cell.set_size_override(Some((w, h)));
                }
                if let Some(doc) = s.target_doc_mut() {
                    if let Some(c) = doc.components.iter_mut().find(|c| c.id == id) {
                        c.size = Some([w, h]);
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
                    last_apply: Instant::now(),
                });
            } else {
                s.gesture = Some(EditGesture::Resize {
                    id,
                    handle,
                    start_root,
                    start_rect,
                    last_apply,
                });
            }
            *need_render = true;
        }
        Some(EditGesture::Marquee { start_canvas, .. }) => {
            // Move 期间只更新当前点 (画布系换算靠缓存 offset — 窗口静止, 足够)
            // 屏幕系差分 (窗口静止期等价画布系; on_press 起点已换算)
            s.gesture = Some(EditGesture::Marquee {
                start_canvas,
                cur_canvas: root,
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
}

// =====================================================================
// 前端推送
// =====================================================================

fn push_doc_event(s: &EditSession, ui_bus: &UIStateBus) {
    let Some(doc) = s.target_doc() else { return };
    let items: Vec<serde_json::Value> = s
        .hit_rects
        .iter()
        .map(|(id, x, y, w, h)| {
            serde_json::json!({ "id": id, "x": x, "y": y, "w": w, "h": h })
        })
        .collect();
    let payload = serde_json::json!({
        "targetPage": s.target_page,
        "page": doc,
        "items": items,
        "lineHeightPx": doc.padding, // 占位 (前端目前不用; 真值在页面字号)
        "selection": s.selection,
        "errors": [],
    });
    if let Ok(data) = serde_json::to_string(&payload) {
        ui_bus.publish(HUD_EDIT_DOC, Some("EditSession"), Some(&data));
    }
    let sel = serde_json::json!({ "ids": s.selection, "hover": s.hover });
    if let Ok(data) = serde_json::to_string(&sel) {
        ui_bus.publish(HUD_EDIT_SELECTION, Some("EditSession"), Some(&data));
    }
}

/// 刷新命中缓存 (render_thread 侧命令处理后公开入口)
pub(crate) fn refresh_cache_public(s: &mut EditSession, pages: &[(String, PageHandle)]) {
    refresh_cache(s, pages);
}

// =====================================================================
// 命令执行面 (UiCommand 编辑命令的渲染线程处理体)
// =====================================================================

/// 编辑命令是否适用于当前会话 (None = 无会话, 调用方拒绝)
pub(crate) fn apply_command(
    s: &mut EditSession,
    pages: &[(String, PageHandle)],
    cmd: &EditCommand,
) -> Result<(), String> {
    match cmd {
        EditCommand::SetTargetPage { page_id } => {
            if s.docs.iter().any(|d| d.id == *page_id) {
                s.target_page = page_id.clone();
                s.selection.clear();
                s.doc_dirty = true;
            }
            Ok(())
        }
        EditCommand::Select { ids } => {
            s.selection = ids
                .iter()
                .filter(|id| {
                    s.target_doc()
                        .map(|d| d.components.iter().any(|c| &c.id == *id))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            s.doc_dirty = true;
            Ok(())
        }
        EditCommand::Nudge { ids, d_unit } => {
            let d = *d_unit;
            if let Some(doc) = s.target_doc_mut() {
                for c in doc.components.iter_mut() {
                    if ids.contains(&c.id) {
                        c.pos = [snap_unit(c.pos[0] + d[0]), snap_unit(c.pos[1] + d[1])];
                    }
                }
            }
            // 页面实例同步: 命令类修改统一走整页重装配 (render_thread 侧调 rebuild)
            let _ = pages;
            s.doc_dirty = true;
            Ok(())
        }
        EditCommand::UpdateComponent { comp } => {
            // 整组件替换 (props/改名/size/visibleWhen); 改名同步 parent 引用
            let old_id = pages_placeholder_find_old(s, comp);
            if let Some(doc) = s.target_doc_mut() {
                // 撞名拒绝
                if comp.id != old_id && doc.components.iter().any(|c| c.id == comp.id) {
                    return Err(format!("组件 id「{}」已存在", comp.id));
                }
                let pos = doc
                    .components
                    .iter()
                    .position(|c| c.id == old_id);
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
                }
            }
            s.doc_dirty = true;
            Ok(())
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
            Ok(())
        }
        EditCommand::ReorderComponent { id, to } => {
            if let Some(doc) = s.target_doc_mut() {
                let Some(pos) = doc.components.iter().position(|c| c.id == *id) else {
                    return Ok(());
                };
                let c = doc.components.remove(pos);
                let to = (*to).min(doc.components.len());
                doc.components.insert(to, c);
            }
            s.doc_dirty = true;
            Ok(())
        }
        EditCommand::UpdatePage { page } => {
            if let Some(doc) = s.docs.iter_mut().find(|d| d.id == page.id) {
                *doc = page.clone();
                s.doc_dirty = true;
            }
            Ok(())
        }
        EditCommand::UpsertPage { page } => {
            match s.docs.iter_mut().find(|d| d.id == page.id) {
                Some(doc) => *doc = page.clone(),
                None => {
                    s.docs.push(page.clone());
                    // 新页注册由 on_reinit_overlays 抑制路径外的 sync 处理
                }
            }
            s.doc_dirty = true;
            Ok(())
        }
        EditCommand::DeletePage { page_id } => {
            s.docs.retain(|d| d.id != *page_id);
            if s.target_page == *page_id {
                s.target_page = s.docs.first().map(|d| d.id.clone()).unwrap_or_default();
                s.selection.clear();
            }
            s.doc_dirty = true;
            Ok(())
        }
        EditCommand::SetOptions {
            snapping,
            show_guides,
            marquee,
        } => {
            if let Some(v) = snapping {
                s.snapping = *v;
            }
            if let Some(v) = show_guides {
                s.show_guides = *v;
            }
            if let Some(v) = marquee {
                s.marquee_mode = *v;
            }
            Ok(())
        }
    }
}

fn pages_placeholder_find_old(s: &EditSession, comp: &ComponentDoc) -> String {
    // 同位替换 (按槽位); 前端携带原 id 的场景在 dto 层展开, 此处按 id 匹配
    // (改名场景前端先按 old id 查 — 简化: UpdateComponent 携带的就是新态,
    // old_id 由"页内已存在且 props 相同"启发不可靠, 故约定前端改名单独走
    // RenameComponent)
    let _ = s;
    comp.id.clone()
}

/// 编辑命令 (UiCommand 变体的载荷面 — commands.rs 引用;
/// serde: IPC 载荷 (vm-webui 经 serde_json Value 中转, vm-app 侧反序列化))
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum EditCommand {
    SetTargetPage { page_id: String },
    Select { ids: Vec<String> },
    Nudge { ids: Vec<String>, d_unit: [f64; 2] },
    UpdateComponent { comp: ComponentDoc },
    RemoveComponents { ids: Vec<String> },
    ReorderComponent { id: String, to: usize },
    UpdatePage { page: PageDoc },
    UpsertPage { page: PageDoc },
    DeletePage { page_id: String },
    SetOptions {
        snapping: Option<bool>,
        show_guides: Option<bool>,
        marquee: Option<bool>,
    },
}

/// Rc 便捷别名 (RenderSession.edit 与 EditBridge 闭包共享)
pub(crate) type EditSessionRef = Rc<RefCell<EditSession>>;
