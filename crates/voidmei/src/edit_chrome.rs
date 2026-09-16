//! 试驾场组件面板 (与真窗同栈: skia 自绘 + 渲染线程单泵, 同调色板)。
//! 唯一编辑窗口 = 头行会话动作 (撤销/重做/放弃二次确认/完成) +
//! 分类折叠组件列表; 条目按下发起拖放 (edit_pump ⓪ 步全局鼠标轮询接管,
//! 真窗插入线/幽灵反馈照旧), 原地松手 = 点击添加 (主容器尾插)。
//! 旧侧栏四区 (页面tab/大纲/属性表单/搜索) 与悬浮条已随简化撤下
//! (git 可找回), 见 doc/试驾场原生化方案.md。

use std::rc::Rc;
use std::time::Instant;

use overlay::platform::cursor;
use overlay::platform::{create, OverlayEvent, OverlayWindow, WindowConfig, WindowKind};
use overlay::render::canvas::PixCanvas;
use overlay::render::font::LoadedFont;
use overlay::render::primitives;
use overlay::widgets::catalog::catalog_groups;
use overlay::widgets::list_container::LIST_CONTAINER_TYPE;
use overlay::widgets::widget_registry;

use crate::edit_session::{
    apply_command, build_insert_command, DragInsert, DropHint, EditCommand, EditSession, ACCENT,
};

// ---------------------------------------------------------------------
// 几何常量 (物理 px) / 配色
// ---------------------------------------------------------------------

const PANEL_W: i32 = 200; // 面板宽
const PANEL_X: i32 = 8; // 屏幕左边距
const HEADER_H: i32 = 36; // 头行高 (撤销/重做 | 放弃/完成)
const GROUP_H: i32 = 28; // 分类头行高
const ITEM_H: i32 = 28; // 条目行高
const ITEM_INDENT: i32 = 20; // 条目文字左缩进
const PAD_BOTTOM: i32 = 8; // 列表底边距
const CLICK_SLOP: i32 = 5; // 点击 vs 拖拽位移阈

// alpha 255 = 不透明真值 (Opaque BitBlt 忽略 alpha)
const BG: [u8; 4] = [28, 28, 32, 255];
const BG_RAISED: [u8; 4] = [38, 38, 44, 255];
const DIVIDER: [u8; 4] = [64, 64, 72, 255];
const TEXT: [u8; 4] = [232, 232, 232, 255];
const TEXT_DIM: [u8; 4] = [126, 126, 134, 255];
const DANGER: [u8; 4] = [225, 72, 72, 255];
/// 悬停高亮 (ACCENT 同色低 alpha)
const HOVER: [u8; 4] = [255, 105, 180, 40];

/// 命中面 (布局/绘制/命中共源; 全窗口局部系)
#[derive(Clone, PartialEq)]
enum ChromeHit {
    Undo,
    Redo,
    Discard,
    Finish,
    /// 分类头 (catalog_groups 序 idx — 折叠/展开)
    GroupHeader(usize),
    /// 组件条目 (注册表 idx — 会话期稳定)
    LibItem(usize),
}

/// 主循环收口的会话级动作 (chrome 无 RenderSession 面)
pub(crate) enum ChromeAction {
    /// 完成(true)/放弃(false) 退出编辑会话
    EndSession(bool),
}

/// 面板布局产物 (唯一坐标真相: 命中/绘制/窗高共源; 自上而下无缝构造 →
/// 各 rect.y+rect.h ≤ content_h 恒成立, 无第二套坐标)
struct PanelLayout {
    undo: (i32, i32, i32, i32),
    redo: (i32, i32, i32, i32),
    discard: (i32, i32, i32, i32),
    finish: (i32, i32, i32, i32),
    /// 分类区: (分类名, 头行 rect, 展开态, 条目 (注册表 idx, rect))
    groups: Vec<(&'static str, (i32, i32, i32, i32), bool, Vec<(usize, (i32, i32, i32, i32))>)>,
    /// 窗高 = HEADER_H + Σ(GROUP_H + 展开分类条目数×ITEM_H) + PAD_BOTTOM
    content_h: i32,
}

/// 面板字号 (页面主字号 × 0.6, 钳 11..15 — 头行 46 宽按钮 3 字不溢出)
fn panel_font_size(base: i32) -> i32 {
    ((base as f64 * 0.6).round() as i32).clamp(11, 15)
}

/// 组件面板 (RenderSession.chrome; Drop 即窗口销毁链)
pub(crate) struct EditChrome {
    panel: Box<dyn OverlayWindow>,
    cv: PixCanvas,
    font: Rc<LoadedFont>,
    /// 展开分类集 (catalog_groups 序; 默认全折叠)
    expanded: Vec<bool>,
    hover: Option<ChromeHit>,
    /// 按下记录 (起点屏幕坐标 + 条目注册表 idx; 位移 ≤ CLICK_SLOP = 点击添加)
    press: Option<((i32, i32), usize)>,
    discard_armed: bool,
    discard_armed_at: Instant,
    /// 屏高 (窗高变化后的垂直居中基准)
    screen_h: i32,
}

impl EditChrome {
    /// 建窗 (渲染线程直接 create — 与真窗同泵同线程): 左缘 x=8 垂直居中,
    /// 高度随折叠态自适应 (初始全折叠 = 36 + 7×28 + 8)
    pub fn open(font_path: &std::path::Path, base_size: i32) -> Result<Self, String> {
        let font =
            LoadedFont::new_cached(font_path, panel_font_size(base_size))
                .map_err(|e| format!("面板字体加载失败: {e}"))?;
        let expanded = vec![false; catalog_groups().len()];
        let h = layout_h(&expanded);
        // probe: 1x1 探测屏尺寸后即弃
        let probe = create(WindowConfig {
            width: 1,
            height: 1,
            x: 0,
            y: 0,
            click_through: false,
            kind: WindowKind::Layered,
        })?;
        let (_, sh) = probe.screen_size();
        drop(probe);
        // Opaque: 不透明 UI 面板
        let panel: Box<dyn OverlayWindow> = Box::new(create(WindowConfig {
            width: PANEL_W,
            height: h,
            x: PANEL_X,
            y: (sh - h) / 2,
            click_through: false,
            kind: WindowKind::Opaque,
        })?);
        let cv = PixCanvas::new(PANEL_W, h).map_err(|e| format!("面板画布: {e}"))?;
        Ok(EditChrome {
            panel,
            cv,
            font,
            expanded,
            hover: None,
            press: None,
            discard_armed: false,
            discard_armed_at: Instant::now(),
            screen_h: sh,
        })
    }
}

// ---------------------------------------------------------------------
// 布局 (唯一坐标真相)
// ---------------------------------------------------------------------

/// 窗高计算 (布局数学; 与 panel_layout 互证 — debug_assert 同值)
fn layout_h(expanded: &[bool]) -> i32 {
    let mut h = HEADER_H + PAD_BOTTOM;
    for (i, (_, items)) in catalog_groups().iter().enumerate() {
        h += GROUP_H;
        if expanded.get(i).copied().unwrap_or(false) {
            h += items.len() as i32 * ITEM_H;
        }
    }
    h
}

fn panel_layout(c: &EditChrome) -> PanelLayout {
    let mut y = HEADER_H;
    let mut groups = Vec::new();
    for (i, (cat, items)) in catalog_groups().iter().enumerate() {
        let expanded = c.expanded.get(i).copied().unwrap_or(false);
        let hdr = (0, y, PANEL_W, GROUP_H);
        y += GROUP_H;
        let rows: Vec<_> = items
            .iter()
            .filter(|_| expanded)
            .map(|&idx| {
                let r = (0, y, PANEL_W, ITEM_H);
                y += ITEM_H;
                (idx, r)
            })
            .collect();
        groups.push((*cat, hdr, expanded, rows));
    }
    let content_h = y + PAD_BOTTOM;
    // 布局纪律互证: 逐 rect 推进与标量式恒同值 (画布尺寸与窗口尺寸恒同步)
    debug_assert_eq!(content_h, layout_h(&c.expanded));
    PanelLayout {
        undo: (0, 0, 46, HEADER_H),
        redo: (46, 0, 46, HEADER_H),
        discard: (96, 0, 48, HEADER_H),
        finish: (144, 0, 56, HEADER_H),
        groups,
        content_h,
    }
}

/// 内容态变化 (折叠/展开) 后追平窗口: 画布重建 + set_size + 垂直居中重算。
/// 只在折叠切换时调用 — 撤销栈灰显等绘制态变化不动窗口几何
fn sync_panel_geometry(c: &mut EditChrome) {
    let h = layout_h(&c.expanded);
    if h == c.cv.height() as i32 {
        return;
    }
    match PixCanvas::new(PANEL_W, h) {
        Ok(cv) => {
            c.panel.set_size(PANEL_W, h);
            c.panel.set_position(PANEL_X, (c.screen_h - h) / 2);
            c.cv = cv;
        }
        Err(e) => kernel::base::logger::warn("EditChrome", &format!("面板画布重建失败: {e}")),
    }
}

fn in_rect(p: (i32, i32), r: (i32, i32, i32, i32)) -> bool {
    p.0 >= r.0 && p.0 <= r.0 + r.2 && p.1 >= r.1 && p.1 <= r.1 + r.3
}

// ---------------------------------------------------------------------
// 事件泵 (渲染线程主循环; 交互即时重绘)
// ---------------------------------------------------------------------

/// 面板事件消费。头行按钮/分类头按下即动作; 条目按下发起拖放 (原地松手 =
/// 点击添加); 会话级动作经返回值主循环收口
pub(crate) fn chrome_pump(c: &mut EditChrome, es: &mut EditSession) -> Option<ChromeAction> {
    let mut action = None;
    let mut dirty = false;
    while let Some(ev) = c.panel.poll_event() {
        // 折叠切换会改窗 y — 逐事件取位置保局部系一致
        let pos = c.panel.position();
        match ev {
            OverlayEvent::MousePress { root_x, root_y } => {
                let local = (root_x - pos.0, root_y - pos.1);
                match panel_hit(c, local) {
                    // 条目: 记按下 + 立即发起拖放 (同线程直连 → edit_pump ⓪ 步接管)
                    Some(ChromeHit::LibItem(idx)) => {
                        c.press = Some(((root_x, root_y), idx));
                        if let Some(drag) = drag_of(idx) {
                            es.drag_insert = Some(drag);
                        }
                    }
                    // 分类头: 折叠/展开 → 窗高自适应 + 垂直居中重算
                    Some(ChromeHit::GroupHeader(i)) => {
                        if let Some(v) = c.expanded.get_mut(i) {
                            *v = !*v;
                        }
                        sync_panel_geometry(c);
                    }
                    // 头行按钮: 按下即动作 (无拖放语义)
                    Some(h) => action = header_action(c, es, h).or(action),
                    None => {}
                }
                dirty = true;
            }
            OverlayEvent::MouseMove { root_x, root_y, .. } => {
                let local = (root_x - pos.0, root_y - pos.1);
                let h = panel_hit(c, local);
                if h != c.hover {
                    c.hover = h;
                    dirty = true;
                }
            }
            OverlayEvent::MouseRelease => {
                // 位移 ≤ CLICK_SLOP = 原地松手 → 点击添加; 拖出的释放结算归 edit_pump
                if let Some(((sx, sy), idx)) = c.press.take() {
                    let (cx, cy) = cursor::cursor_pos();
                    if (cx - sx).abs() <= CLICK_SLOP && (cy - sy).abs() <= CLICK_SLOP {
                        click_add(es, idx);
                        dirty = true;
                    }
                }
            }
            _ => {}
        }
    }
    // 放弃确认 3s 超时还原
    if c.discard_armed && c.discard_armed_at.elapsed().as_secs() >= 3 {
        c.discard_armed = false;
        dirty = true;
    }

    if dirty {
        render(c, es);
    }
    action
}

/// 头行按钮动作 (撤销/重做/放弃二次确认/完成)
fn header_action(c: &mut EditChrome, es: &mut EditSession, hit: ChromeHit) -> Option<ChromeAction> {
    match hit {
        ChromeHit::Undo | ChromeHit::Redo => {
            let cmd = if hit == ChromeHit::Undo {
                &EditCommand::Undo
            } else {
                &EditCommand::Redo
            };
            if let Err(e) = apply_command(es, cmd) {
                kernel::base::logger::warn("EditChrome", &format!("命令被拒: {e}"));
            }
            es.need_rebuild = true;
            None
        }
        ChromeHit::Discard => {
            if c.discard_armed {
                c.discard_armed = false;
                Some(ChromeAction::EndSession(false))
            } else {
                c.discard_armed = true;
                c.discard_armed_at = Instant::now();
                None
            }
        }
        ChromeHit::Finish => Some(ChromeAction::EndSession(true)),
        // 头行按钮专属入口 — 列表命中不会到这
        _ => None,
    }
}

/// 点击添加: 目标页主容器尾插 (无容器 = 自由区固定落点)
fn click_add(es: &mut EditSession, idx: usize) {
    let Some(drag) = drag_of(idx) else { return };
    let hint = es.target_doc().and_then(|doc| {
        doc.components
            .iter()
            .find(|comp| comp.r#type == LIST_CONTAINER_TYPE)
            .map(|cont| {
                let n = doc
                    .components
                    .iter()
                    .filter(|comp| comp.parent.as_deref() == Some(cont.id.as_str()))
                    .count();
                DropHint::InsertLine {
                    container: cont.id.clone(),
                    index: n,
                    x0: 0,
                    y: 0,
                    x1: 0,
                }
            })
    });
    let hint = hint.unwrap_or(DropHint::Ghost { x: 60, y: 60, w: 140, h: 30 });
    if let Some(cmd) = build_insert_command(es, &drag, &hint) {
        if let Err(e) = apply_command(es, &cmd) {
            kernel::base::logger::warn("EditChrome", &format!("命令被拒: {e}"));
        }
        es.need_rebuild = true;
    }
    es.drag_insert = None; // 按下时置位的拖放会话就此了结
}

/// 注册表 idx → 拖放载荷 (default_props 解析; 非法兜底空对象)
fn drag_of(idx: usize) -> Option<DragInsert> {
    let meta = *widget_registry().get(idx)?;
    let props =
        serde_json::from_str(meta.default_props).unwrap_or_else(|_| serde_json::json!({}));
    Some(DragInsert {
        type_name: meta.type_name.to_string(),
        display_zh: meta.display_zh.to_string(),
        props,
    })
}

// ---------------------------------------------------------------------
// 命中 (与绘制共源 PanelLayout)
// ---------------------------------------------------------------------

fn panel_hit(c: &EditChrome, local: (i32, i32)) -> Option<ChromeHit> {
    let l = panel_layout(c);
    if in_rect(local, l.undo) {
        return Some(ChromeHit::Undo);
    }
    if in_rect(local, l.redo) {
        return Some(ChromeHit::Redo);
    }
    if in_rect(local, l.discard) {
        return Some(ChromeHit::Discard);
    }
    if in_rect(local, l.finish) {
        return Some(ChromeHit::Finish);
    }
    for (i, (_, hdr, _, _)) in l.groups.iter().enumerate() {
        if in_rect(local, *hdr) {
            return Some(ChromeHit::GroupHeader(i));
        }
    }
    for (_, _, _, rows) in &l.groups {
        for (idx, r) in rows {
            if in_rect(local, *r) {
                return Some(ChromeHit::LibItem(*idx));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------
// 绘制 (skia; 坐标全来自 PanelLayout — 无第二套)
// ---------------------------------------------------------------------

pub(crate) fn render(c: &mut EditChrome, es: &EditSession) {
    draw_panel(c, es);
    // Opaque 契约 = 直通 BGRA (BitBlt 忽略 alpha)
    let buf = c.cv.straight_bgra();
    if let Err(e) = c.panel.present(&buf) {
        kernel::base::logger::warn("EditChrome", &format!("面板上屏失败: {e}"));
    }
}

fn draw_panel(c: &mut EditChrome, es: &EditSession) {
    let l = panel_layout(c);
    let hover = c.hover.clone();
    let armed = c.discard_armed;
    let font = Rc::clone(&c.font);
    let cv = &mut c.cv;
    let (w, h) = (PANEL_W, l.content_h);
    cv.fill_rect(0, 0, w, h, BG);

    // 头行四钮 (撤销/重做禁用灰显按栈空; 放弃 armed = 红底「确认?」; 完成 = accent)
    let buttons: [(ChromeHit, (i32, i32, i32, i32), &str, bool); 4] = [
        (ChromeHit::Undo, l.undo, "↩撤销", es.undo_stack.is_empty()),
        (ChromeHit::Redo, l.redo, "↪重做", es.redo_stack.is_empty()),
        (ChromeHit::Discard, l.discard, if armed { "确认?" } else { "放弃" }, false),
        (ChromeHit::Finish, l.finish, "完成 ✓", false),
    ];
    for (hit, r, label, disabled) in buttons {
        let hovered = !disabled && hover.as_ref() == Some(&hit);
        let bg = match (&hit, armed, hovered) {
            (ChromeHit::Discard, true, _) => DANGER,
            (ChromeHit::Finish, _, _) => ACCENT,
            (_, _, true) => [64, 64, 72, 255],
            _ => BG_RAISED,
        };
        cv.fill_rect(r.0, r.1, r.2, r.3, bg);
        let tw = font.measure(label);
        cv.draw_text(
            &font,
            r.0 + (r.2 - tw) / 2,
            baseline(r.1, r.3, &font),
            label,
            if disabled { TEXT_DIM } else { TEXT },
            true,
        );
    }
    cv.fill_rect(0, HEADER_H, w, 1, DIVIDER);

    // 分类列表: 头行 (▸/▾ 箭头 + 分类名, hover 高亮) + 展开条目 (缩进 + display_zh)
    for (i, (cat, hdr, expanded, rows)) in l.groups.iter().enumerate() {
        if hover == Some(ChromeHit::GroupHeader(i)) {
            cv.fill_rect(hdr.0, hdr.1, hdr.2, hdr.3, HOVER);
        }
        let arrow = if *expanded { "▾" } else { "▸" };
        cv.draw_text(&font, 8, baseline(hdr.1, hdr.3, &font), arrow, TEXT_DIM, true);
        let label = truncate_to_width(&font, cat, w - 24 - 8);
        cv.draw_text(&font, 24, baseline(hdr.1, hdr.3, &font), &label, TEXT, true);
        for (idx, r) in rows {
            if hover == Some(ChromeHit::LibItem(*idx)) {
                cv.fill_rect(r.0, r.1, r.2, r.3, HOVER);
            }
            let meta = widget_registry()[*idx];
            let label = truncate_to_width(&font, meta.display_zh, w - ITEM_INDENT - 8);
            cv.draw_text(&font, ITEM_INDENT, baseline(r.1, r.3, &font), &label, TEXT, true);
        }
    }
    primitives::ring1px(cv, 0, 0, w, h, DIVIDER);
}

/// 行内垂直居中基线
fn baseline(y: i32, h: i32, font: &LoadedFont) -> i32 {
    y + (h + font.size) / 2 - 2
}

/// 文本超宽截断 (加省略号; 分类名/条目名显示用)
fn truncate_to_width(font: &LoadedFont, s: &str, max_w: i32) -> String {
    if font.measure(s) <= max_w {
        return s.to_string();
    }
    let mut out = String::new();
    for ch in s.chars() {
        // 预留省略号宽度 (等宽字体 = 单字符宽)
        if font.measure(&out) + font.char_width(ch) > max_w - font.char_width('…') {
            break;
        }
        out.push(ch);
    }
    format!("{out}…")
}
