//! Overlay 应用层: 事件循环 + 拖拽状态机 + 渲染呈现
//! 对应 Java DraggableOverlay (拖拽/位置) + FieldOverlay (渲染) 的运行时行为

use crate::font::Canvas;
use crate::platform::{self, OverlayEvent, OverlayWindow, WindowConfig};
use crate::render::{FieldText, FontTriple, RenderColors};
use vm_core::layout::RenderCtx;

pub enum OverlayMode {
    /// 预览: 静态 preview-value, 灰底可拖拽 (同 Java applyPreviewStyle)
    Preview,
    /// 游戏: 鼠标穿透 + 置顶 (数据接入在 M3)
    Live,
}

/// Java Application.previewColor = (0,0,0,10): 预览模式极淡黑底, 方便看清 overlay 范围
const PREVIEW_BG: [u8; 4] = [0x00, 0x00, 0x00, 0x0A];

/// 直通 RGBA 画布 → 预乘 BGRA (UpdateLayeredWindow 的格式)
pub fn to_premul_bgra(canvas: &Canvas) -> Vec<u8> {
    let n = canvas.buf.len() / 4;
    let mut out = vec![0u8; canvas.buf.len()];
    for i in 0..n {
        let (r, g, b, a) = (
            canvas.buf[i * 4] as u32,
            canvas.buf[i * 4 + 1] as u32,
            canvas.buf[i * 4 + 2] as u32,
            canvas.buf[i * 4 + 3] as u32,
        );
        out[i * 4] = (b * a / 255) as u8;
        out[i * 4 + 1] = (g * a / 255) as u8;
        out[i * 4 + 2] = (r * a / 255) as u8;
        out[i * 4 + 3] = a as u8;
    }
    out
}

struct DragState {
    /// 按下时 (root - win_pos) 偏移, 对应 Java DraggableOverlay.dragStartX/Y
    off_x: i32,
    off_y: i32,
}

/// 实时模式: 轮询快照 + 脏检查重绘 (对应 FieldOverlay.onFlightData 50ms 节流)
/// 窗口尺寸固定为全 16 行 (POC 简化: visible-when 变化不重建窗口, 空行透明无碍)
pub fn run_live(
    ctx: RenderCtx,
    fonts: FontTriple,
    colors: &RenderColors,
    aa: bool,
    snapshot: std::sync::Arc<std::sync::Mutex<Option<vm_data::data::derive::FlightValues>>>,
    build_texts: fn(&vm_data::data::derive::FlightValues) -> Vec<(String, String, String)>,
) -> Result<(), String> {
    let max_height = ctx.total_height(16);
    let max_width = ctx.total_width();
    let saved = crate::config::load_pos();
    let cfg = WindowConfig {
        width: max_width,
        height: max_height,
        x: 60,
        y: 100,
        click_through: true, // live 恒穿透
    };
    let mut win = platform::create(cfg)?;
    if let Some(p) = saved {
        let (sw, sh) = win.screen_size();
        win.set_position((p.x * sw as f64).round() as i32, (p.y * sh as f64).round() as i32);
    } else {
        let (sw, sh) = win.screen_size();
        win.set_position((sw - max_width) / 2, (sh - max_height) / 2);
    }

    eprintln!("overlay live: {}x{} @ {:?} (穿透模式, Ctrl+C 退出)", max_width, max_height, win.position());

    // 画布复用: 尺寸固定, 每帧清零重绘
    let mut canvas = crate::font::Canvas::new(max_width, max_height);
    let mut last_frame: Option<String> = None; // 脏检查: 格式化字符串指纹
    let mut last_poll = std::time::Instant::now();
    loop {
        while let Some(_ev) = win.poll_event() {
            // 穿透模式无交互事件 (保留泵消息避免假死)
        }
        if last_poll.elapsed() >= std::time::Duration::from_millis(50) {
            last_poll = std::time::Instant::now();
            if let Some(v) = snapshot.lock().ok().and_then(|s| *s) {
                let owned = build_texts(&v);
                // 指纹: 直接拼接格式化值 (脏检查, 对应 Java repaint 抑制)
                let fp = owned
                    .iter()
                    .map(|(_, _, val)| val.as_str())
                    .collect::<Vec<_>>()
                    .join("|");
                if last_frame.as_deref() != Some(fp.as_str()) {
                    last_frame = Some(fp);
                    let texts: Vec<FieldText> = owned
                        .iter()
                        .map(|(l, u, val)| FieldText { label: l, unit: u, value: val })
                        .collect();
                    crate::render::render_fields_fixed(&mut canvas, &texts, &ctx, &fonts, colors, aa);
                    let buf = to_premul_bgra(&canvas);
                    win.present(&buf)?;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

pub fn run(
    mode: OverlayMode,
    ctx: RenderCtx,
    fonts: FontTriple,
    texts: Vec<FieldText<'static>>,
    colors: &RenderColors,
    aa: bool,
) -> Result<(), String> {
    // 预览模式: 先铺 Java previewColor 灰底再画文本 (WYSIWYG 辅助)
    let mut canvas = crate::font::Canvas::new(ctx.total_width(), ctx.total_height(texts.len() as i32));
    if matches!(mode, OverlayMode::Preview) {
        canvas.fill(PREVIEW_BG);
    }
    crate::render::draw_fields(&mut canvas, &texts, &ctx, &fonts, colors, aa);
    let click_through = matches!(mode, OverlayMode::Live);

    // 初始位置: 已保存的归一化坐标 → 屏幕居中 (屏幕尺寸创建后可知, 先占位)
    let saved = crate::config::load_pos();
    let cfg = WindowConfig {
        width: canvas.width,
        height: canvas.height,
        x: 60,
        y: 100,
        click_through,
    };
    let mut win = platform::create(cfg)?;

    // 应用保存位置 (归一化 → 物理像素, 同 Java loadPosition 语义)
    if let Some(p) = saved {
        let (sw, sh) = win.screen_size();
        win.set_position(
            (p.x * sw as f64).round() as i32,
            (p.y * sh as f64).round() as i32,
        );
    } else {
        let (sw, sh) = win.screen_size();
        win.set_position((sw - canvas.width) / 2, (sh - canvas.height) / 2);
    }

    // 首帧呈现
    let buf = to_premul_bgra(&canvas);
    win.present(&buf)?;
    eprintln!(
        "overlay 运行中: {}x{} @ {:?} ({}模式, Ctrl+C 退出)",
        canvas.width,
        canvas.height,
        win.position(),
        if click_through { "穿透" } else { "预览可拖拽" }
    );

    // 事件循环: 稀疏事件 + 无数据变更时零渲染 (低占用)
    let mut drag: Option<DragState> = None;
    loop {
        while let Some(ev) = win.poll_event() {
            match ev {
                OverlayEvent::Close => return Ok(()),
                OverlayEvent::MousePress { root_x, root_y } => {
                    if !click_through {
                        let (wx, wy) = win.position();
                        drag = Some(DragState {
                            off_x: root_x - wx,
                            off_y: root_y - wy,
                        });
                    }
                }
                OverlayEvent::MouseMove { root_x, root_y, left_down } => {
                    if let Some(d) = drag.as_ref() {
                        if left_down {
                            win.set_position(root_x - d.off_x, root_y - d.off_y);
                        }
                    }
                }
                OverlayEvent::MouseRelease => {
                    if drag.take().is_some() {
                        // 保存归一化位置 (同 Java saveCurrentPosition)
                        let (wx, wy) = win.position();
                        let (sw, sh) = win.screen_size();
                        crate::config::save_pos(
                            wx as f64 / sw as f64,
                            wy as f64 / sh as f64,
                        );
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
