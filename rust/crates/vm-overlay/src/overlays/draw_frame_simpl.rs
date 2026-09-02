//! draw_frame_simpl: 简化版 FM 曲线可视化窗口 (DrawFrameSimpl.java 全量翻译, P6 收口)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`DrawFrameSimpl`] | ui/overlay/DrawFrameSimpl.java | 推力-真空速曲线透明 overlay: FM 句柄缓存直绘 (panel paintComponent), 自管可见性 (preview 恒显 / 游戏模式热键切换), run() 循环 = 1000ms 节流 + displayFmKey==0 收腿 10s 自动退场 |
//!
//! 死代码不搬 (P5 getdata 先例): `paintAction` (:112-170, 全工程无调用, 与 panel
//! paintComponent 内联块重复) / `drawCoordinates`×2 + `searchMin/searchMax`×4 (依赖
//! 恒 null 的 `FlightAnalyzer fA` 字段, 调用即 NPE) / 死字段 pixIndex/Index/useBlkx/
//! ggx4/ggy4/Blkx/fX/fY (声明后无读写点)。
//!
//! 组装契约 (overlays_field2.rs 同款):
//! - 窗口/拖动归 host; 固定几何 (0, screenH-500, 900, 500) 经
//!   [`OverlayHost::set_entry_fixed_pos`] 每次 materialize 重 applying (Java 每次
//! init/initPreview 的 setBounds 字面量; 位置存档键 thrustdFSX/Y 只写不读, 不参与
//! 定位 — Rust 侧 host 内存档同样不回读, 等价死数据);
//! - UIStateBus 订阅 (FM_OVERLAY_TOGGLE 仅游戏 init 挂接 / FM_CHANGED 两会话均挂
//!   — initFmHandleCache 被 init 与 initPreview 共用) 对应
//!   [`DrawFrameSimpl::toggle`]/[`DrawFrameSimpl::reload_fm`], 由组装层事件循环驱动;
//!   dispose 的退订由所有权 Drop 根治 (LIFETIMES §2.3);
//! - run() 线程循环 (needsThread=true: OverlayEntry.open 与 refreshPreview 均起线程,
//!   OverlayManager.java:303-309/:326-331) 由 [`DrawFrameSimplFeed`] 单线程驱动。
//!
//! 对拍备案 (审查 W3): rustcmp 套件现覆盖 FlightInfo/gauges/MiniHUD, 本组件渲染
//! 证据 = 单测级几何 oracle + 像素墨迹断言 (Java 语义逐式复算); FMUnpacked 同款。

use vm_core::base::format::{java_format_f, java_round_f32};
use crate::render::font::LoadedFont;
use crate::render::palette::aa;
use crate::platform::host::{OverlayHost, OverlaySpec};
use crate::overlays::spec_common::{keyed_spec, FontSlot};
use crate::render::canvas::{LineCapStyle, PixCanvas};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use vm_core::fm::data::FmData;
use vm_core::fm::FMManager;

// ---------------------------------------------------------------------------
// 几何/绘制原语 (Java findMin/findMax + drawXY/drawPoint/drawExample)
// ---------------------------------------------------------------------------

/// findMin (Java :172-180): 初值 Float.MAX_VALUE (非 Double — 保真)
fn find_min(x: &[f64]) -> f64 {
    let mut min = f32::MAX as f64;
    for &v in x {
        if v < min {
            min = v;
        }
    }
    min
}

/// findMax (Java :182-190): 初值 Float.MIN_VALUE = 最小正 subnormal ≈1.4e-45
/// (非最负 — Java 命名陷阱, 保真; 域内推力恒正无数值差, 空数组返回值同构)
fn find_max(x: &[f64]) -> f64 {
    let mut max = f64::from(f32::from_bits(1));
    for &v in x {
        if v > max {
            max = v;
        }
    }
    max
}

/// paintComponent 的几何段 (Java :557-592, init/initPreview 两处内联块同体)。
/// 公式逐式保真 — pxmax/pymin 族与 drawXY 局部重算存在刻度差 (ygap 非 10 倍数时
/// (int)(ygap/10)*10 ≠ ygap), Java 原样 (点列与网格线用不同比例尺)。
pub struct ChartGeom {
    pub dwidth: i32,  // 800
    pub dheight: i32, // 400
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xgap: i32,
    pub ygap: i32,
    /// 点列坐标换算基准 (drawPoint 实参)
    pub pxmin: i32,
    pub pymin: i32,
    /// 点列比例尺 (pxmax/pymax 派生, 与 drawXY 内部的 ggx/ggy 不同源)
    pub ggx4: f64,
    pub ggy4: f64,
    /// 灰度步进 = (int)(255.0f/(altThrNum+1))
    pub rgbx: i32,
}

/// 坐标系/点列换算 (调用侧已做 b==null || velThrNum==0 守卫)
pub fn chart_geometry(b: &FmData) -> ChartGeom {
    // 绘制坐标系
    let vt = b.velocity_thr.as_ref().unwrap(); // loader 契约: velThrNum>0 必有表
    let xn: Vec<f64> = vt[..b.vel_thr_num as usize].to_vec();

    let xmin_raw = find_min(&xn);
    let xmax_raw = find_max(&xn);

    let rows = b.max_thr_aft.as_ref().unwrap();
    let ymin_raw = find_min(&rows[(b.alt_thr_num - 1) as usize]);
    let ymax_raw = find_max(&rows[0]);

    // xmax对齐10 ((int) 截断向零)
    let xmin = (((xmin_raw / 10.0) as i32) * 10) as f64;
    let xmax = (((xmax_raw / 10.0) as i32) * 10) as f64;
    let ymin = (((ymin_raw / 10.0) as i32) * 10) as f64;
    let ymax = (((ymax_raw / 10.0) as i32) * 10) as f64;
    let dwidth = 800;
    let dheight = 400;
    let xgap = java_round_f32(((xmax as i32 + 1 - xmin as i32) / 5) as f32 / 5.0) * 5;
    let ygap = java_round_f32(((ymax as i32 + 1 - ymin as i32) / 5) as f32 / 5.0) * 5;
    let pxmin = xmin as i32;
    let pxmax = xmax as i32 + xgap;
    let pymin = ((ymin / 10.0) as i32) * 10;
    let pymax = ((ymax / 10.0) as i32) * 10 + (ygap / 10) * 10;
    let mut ggx4 = 0.0;
    let mut ggy4 = 0.0;
    if pxmax - pxmin != 0 {
        ggx4 = dwidth as f64 / (pxmax - pxmin) as f64;
    }
    if pymax - pymin != 0 {
        ggy4 = dheight as f64 / (pymax - pymin) as f64;
    }
    let rgbx = (255.0f32 / (b.alt_thr_num + 1) as f32) as i32;
    ChartGeom {
        dwidth,
        dheight,
        xmin,
        xmax,
        ymin,
        ymax,
        xgap,
        ygap,
        pxmin,
        pymin,
        ggx4,
        ggy4,
        rgbx,
    }
}

/// 三档字号字体组 (Java Application.defaultFontName / defaultNumfontName 的 PLAIN
/// 族: 标题 fontsize+6 / 轴单位 fontsize+4 / 刻度数字与图例 fontsize=12。
/// PORT: Java 字族 YaHei/Roboto → Rust 固定 sarasa regular —
/// fm_unpacked_data_overlay_spec 同款先例, cfg 缺省字体名时零偏差)
pub struct DfsFonts<'a> {
    /// 刻度数字 (defaultNumfontName PLAIN 12)
    pub num12: &'a LoadedFont,
    /// 轴单位 (defaultFontName PLAIN 16)
    pub text16: &'a LoadedFont,
    /// 标题 (defaultFontName PLAIN 18)
    pub text18: &'a LoadedFont,
    /// 图例文本 (defaultFontName PLAIN 12, 与 num12 同文件同号)
    pub text12: &'a LoadedFont,
}

/// drawXY (Java :248-310): 坐标轴 + 刻度 + 单位。xName/yName 实参 Java 未消费
/// (传参即死), 保形保留占位。线宽 3/1 交替 = setStroke(3)/setStroke(1) 的无状态
/// 逐调用展开; 裸 BasicStroke = CAP_SQUARE/JOIN_MITER。
#[allow(clippy::too_many_arguments)] // 签名对齐 Java drawXY(g, x, y, dwidth, dheight, title, xName, yName, xD, yD, xmin..ygap, fontsize)
fn draw_xy(
    cv: &mut PixCanvas,
    fonts: &DfsFonts,
    x: i32,
    y: i32,
    dwidth: i32,
    dheight: i32,
    title: &str,
    _x_name: &str,
    _y_name: &str,
    x_d: &str,
    y_d: &str,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    xgap: i32,
    ygap: i32,
    fontsize: i32,
    aa: bool,
) {
    let axis = [0u8, 0, 0, 250];
    let pxmin = xmin as i32;
    let pxmax = xmax as i32 + xgap;
    let pymin = ymin as i32;
    let pymax = ymax as i32 + ygap;
    let mut interval_x = xgap;
    let mut interval_y = ygap;
    if interval_x == 0 {
        interval_x = 1;
    }
    if interval_y == 0 {
        interval_y = 1;
    }
    let mut ggx = 0.0;
    let mut ggy = 0.0;
    if pxmax - pxmin != 0 {
        ggx = dwidth as f64 / (pxmax - pxmin) as f64;
    }
    if pymax - pymin != 0 {
        ggy = dheight as f64 / (pymax - pymin) as f64;
    }

    // 标题 (fontsize+6, drawString 基线 = y)
    let _ = fontsize; // 字号已烘进 fonts.text18
    cv.draw_text(fonts.text18, x + dwidth / 2, y, title, axis, aa);
    let y = y + 10; // 往下推10

    // x轴与箭头 (BasicStroke(3))
    cv.draw_line_cap(x, y + dheight, x + dwidth, y + dheight, 3.0, axis, LineCapStyle::Square, aa);
    let mut ii = (pxmax - pxmin) / interval_x;
    while ii >= 0 {
        // 坐标轴刻度 (BasicStroke(1))
        let tx = (x as f64 + (ii * interval_x) as f64 * ggx) as i32;
        cv.draw_line_cap(tx, y + dheight, tx, y, 1.0, axis, LineCapStyle::Square, aa);
        cv.draw_text(
            fonts.num12,
            tx,
            y + dheight + 15,
            &(pxmin + ii * interval_x).to_string(),
            axis,
            aa,
        );
        ii -= 1;
    }
    // x轴单位 (fontsize+4)
    cv.draw_text(fonts.text16, x + dwidth + 5, y + dheight, x_d, axis, aa);

    // y轴与箭头 (BasicStroke(3))
    cv.draw_line_cap(x, y + dheight, x, y, 3.0, axis, LineCapStyle::Square, aa);
    // y轴刻度
    ii = (pymax - pymin) / interval_y;
    while ii >= 0 {
        let ty = ((y + dheight) as f64 - (ii * interval_y) as f64 * ggy) as i32;
        cv.draw_line_cap(x, ty, x + dwidth, ty, 1.0, axis, LineCapStyle::Square, aa);
        cv.draw_text(
            fonts.num12,
            x - 40,
            ty,
            &(pymin + ii * interval_y).to_string(),
            axis,
            aa,
        );
        ii -= 1;
    }
    // y轴单位 (fontsize+4)
    cv.draw_text(fonts.text16, x - 5, y - 10, y_d, axis, aa);
}

/// drawPoint (Java :312-334): 逐高度行绘点 + 连线。
/// PORT: drawOval(x-1, y-1, 2, 2) 的 2×2 圆轮廓 ≈ 2×2 墨迹点 (PixCanvas 无椭圆
/// 原语, fill_rect 覆盖同外接盒)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawPoint(g, x, y, dwidth, dheight, ggx, ggy, ix, iy, pxmin, pymin, C)
fn draw_point(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    _dwidth: i32,
    dheight: i32,
    ggx: f64,
    ggy: f64,
    ix: &[f64],
    iy: &[f64],
    pxmin: i32,
    pymin: i32,
    c: [u8; 4],
    aa: bool,
) {
    let y = y + 10; // 往下推10
    // 绘点
    for ii in 0..ix.len() {
        let px = (x as f64 + (ix[ii] - pxmin as f64) * ggx) as i32 - 1;
        let py = ((y + dheight) as f64 - (iy[ii] - pymin as f64) * ggy) as i32 - 1;
        cv.fill_rect(px, py, 2, 2, c);
    }
    // 连线 (BasicStroke(1))
    if ix.is_empty() {
        return;
    }
    for ii in 0..ix.len() - 1 {
        let x0 = (x as f64 + (ix[ii] - pxmin as f64) * ggx) as i32;
        let y0 = ((y + dheight) as f64 - (iy[ii] - pymin as f64) * ggy) as i32;
        let x1 = (x as f64 + (ix[ii + 1] - pxmin as f64) * ggx) as i32;
        let y1 = ((y + dheight) as f64 - (iy[ii + 1] - pymin as f64) * ggy) as i32;
        cv.draw_line_cap(x0, y0, x1, y1, 1.0, c, LineCapStyle::Square, aa);
    }
}

/// drawExample (Java :336-343): 图例 (线段 + 文本)。fontsize 实参烘进字体对象。
#[allow(clippy::too_many_arguments)] // 对齐 Java drawExample(g, x, y, dheight, C, name, fontsize) — 字号入字体
fn draw_example(
    cv: &mut PixCanvas,
    f12: &LoadedFont,
    x: i32,
    y: i32,
    dheight: i32,
    c: [u8; 4],
    name: &str,
    aa: bool,
) {
    cv.draw_line_cap(x, y + dheight + 40, x + 20, y + dheight + 40, 1.0, c, LineCapStyle::Square, aa);
    cv.draw_text(f12, x + 25, y + dheight + 45, name, [0, 0, 0, 250], aa);
}

// ---------------------------------------------------------------------------
// DrawFrameSimpl (ui/overlay/DrawFrameSimpl.java)
// ---------------------------------------------------------------------------

/// 简化版 FM 曲线 overlay (DrawFrameSimpl.java:35)。窗口/拖动/事件订阅归组装层,
/// 本体承载: FM 句柄缓存 (paint 只读, 绝不查管理器 — P3/R3) + 自管可见性。
pub struct DrawFrameSimpl {
    /// Java :100 isPreview (preview 恒可见, 游戏模式走 toggle)
    pub is_preview: bool,
    /// Self-managed visibility state (Java :109, 游戏模式初始 false)
    pub visible: bool,
    /// fmHandle 缓存 (Java :104 volatile; 非 READY 句柄 blkx=null → None)
    fm_data: Option<Arc<FmData>>,
}

impl Default for DrawFrameSimpl {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawFrameSimpl {
    /// 构造 (Java 字段隐式初始化 §2.10: isPreview=false / visible=true / fmHandle=null)
    pub fn new() -> Self {
        DrawFrameSimpl {
            is_preview: false,
            visible: true,
            fm_data: None,
        }
    }

    /// init (Java :514-628) 的数据/状态面 (窗口操作归 host):
    /// initFmHandleCache (current 快照, :74) + isPreview=false + 隐藏起步。
    /// toggle 的 UIStateBus 订阅归组装层 (仅游戏 init 挂接)。
    pub fn init(&mut self, current_fm: Option<Arc<FmData>>) {
        // P3/R3: 改用句柄缓存 (原 Blkx = xc.getBlkx() 在加载未落定时可能拿到 null)
        self.fm_data = current_fm;
        self.is_preview = false;
        // Game mode: initially hidden
        self.visible = false;
    }

    /// initPreview (Java :630-721): isPreview=true + 恒可见 + 同一句柄缓存
    /// (initFmHandleCache 共用 — preview 实例同样订阅 FM_CHANGED)
    pub fn init_preview(&mut self, current_fm: Option<Arc<FmData>>) {
        self.is_preview = true;
        // Preview mode: always visible, no toggle subscription
        self.visible = true;
        self.fm_data = current_fm;
    }

    /// FM_CHANGED handler (Java :79-88, init/initPreview 均订阅):
    /// 句柄换 blkx + panel.repaint (Rust 由渲染节拍脏检查承接)
    pub fn reload_fm(&mut self, fmdata: Option<Arc<FmData>>) {
        self.fm_data = fmdata;
    }

    /// FM_OVERLAY_TOGGLE handler (Java :526-529, 仅游戏 init 挂接)
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// CloseAll 后回 preview 会话形态 (Java closeAll = 实例销毁 + refreshPreview
    /// 工厂新建 initPreview 实例): visible=true / is_preview=true。fm 缓存保留 —
    /// FM_CHANGED 常驻订阅 (initPreview 亦挂) 使其与 FMManager 恒同步。
    /// (FmUnpackedDataOverlay::reset_preview / ControlSurfaces::reset_preview 同族)
    pub fn reset_preview(&mut self) {
        self.visible = true;
        self.is_preview = true;
    }

    /// run() 的可见性判定 (Java :740): preview 恒显, 游戏模式用 toggle 态
    pub fn should_show(&self) -> bool {
        self.is_preview || self.visible
    }

    /// panel.paintComponent (Java :544-607, init/initPreview 同体) — 无 FM 或
    /// velThrNum==0 直接跳过 (null 守卫 :554-555)
    pub fn draw(&self, cv: &mut PixCanvas, fonts: &DfsFonts, aa: bool) {
        let b = match self.fm_data.as_deref() {
            Some(b) if b.vel_thr_num != 0 => b,
            _ => return,
        };
        let g = chart_geometry(b);
        draw_xy(
            cv, fonts, 50, 50, g.dwidth, g.dheight, "推力-真空速曲线", "真空速", "推力", "km/h", "kgf",
            g.xmin, g.xmax, g.ymin, g.ymax, g.xgap, g.ygap, 12, aa,
        );
        let vt = b.velocity_thr.as_ref().unwrap();
        let xn = &vt[..b.vel_thr_num as usize];
        let rows = b.max_thr_aft.as_ref().unwrap();
        let alt = b.altitude_thr.as_ref().unwrap();
        let fontsize = 12;
        for i in 0..b.alt_thr_num {
            // Java new Color((i+1)*rgbx ×3, 250) — 域内恒 ≤255
            let v = ((i + 1) * g.rgbx) as u8;
            let c = [v, v, v, 250];
            draw_point(
                cv, 50, 50, g.dwidth, g.dheight, g.ggx4, g.ggy4, xn, &rows[i as usize],
                g.pxmin, g.pymin, c, aa,
            );
            // String.format("高度%.0fm", altitudeThr[i])
            let name = format!("高度{}m", java_format_f(alt[i as usize], 0));
            draw_example(cv, fonts.text12, g.dwidth - 40, 60 + i * fontsize - g.dheight, g.dheight, c, &name, aa);
        }
        // 绘制点 / 连接点 (Java 尾注 — 已在 drawPoint 内完成)
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 挂载 (Java Controller.java:746-752 registerWithStrategy("thrustdFS"))
// ---------------------------------------------------------------------------

/// 推力曲线共享句柄 (flight_info/control_surfaces 先例: render 闭包与事件循环
/// 共享 state; Rc 恒留渲染线程)
pub type DrawFrameSimplHandle = Rc<RefCell<DrawFrameSimpl>>;

/// 推力曲线 OverlaySpec + live 句柄 (Java Controller.java:745-752: 键 thrustdFS,
/// 激活策略 config("enableFMPrint").and(jetOnly), previewEnabled=true)。
///
/// 初始态 = initPreview 形态 (恒可见 — Java 预览工厂); 游戏形态 (is_preview=false +
/// 隐藏起步 + 句柄重读) 由组装层在 OpenAllOverlays 处置 (单实例会话翻转模式,
/// ControlSurfaces/FmUnpacked 同款)。尺寸恒 900×500 (setBounds 字面量, 无 reinit 面
/// — Java reinitConfig :723-725 空实现); 定位经 host `set_entry_fixed_pos`。
pub fn draw_frame_simpl_spec(
    fonts_dir: &std::path::Path,
    fm: &Arc<FMManager>,
) -> Result<(DrawFrameSimplHandle, OverlaySpec), String> {
    let regular = fonts_dir.join("sarasa-mono-sc-regular.ttf");
    let f12 = FontSlot::new("DrawFrameSimpl", &regular, 12)?;
    let f16 = FontSlot::new("DrawFrameSimpl", &regular, 16)?;
    let f18 = FontSlot::new("DrawFrameSimpl", &regular, 18)?;
    let mut dfs = DrawFrameSimpl::new();
    // initFmHandleCache (:74): fmHandle = FMManager.current() 快照
    dfs.init_preview(fm.current().fmdata.clone().map(Arc::new));
    let handle: DrawFrameSimplHandle = Rc::new(RefCell::new(dfs));
    let render_handle = Rc::clone(&handle);
    let (r12, r16, r18) = (f12, f16, f18);
    Ok((
        handle,
        // Java registerWithStrategy("thrustdFS", ...) — LinkedHashMap 键
        keyed_spec(
            "thrustdFS",
            900,
            500,
            Box::new(move |cv: &mut PixCanvas| {
                // aa = 运行时仓 (cfg AAEnable 可关)
                let (n12, n16, n18) = (r12.get(), r16.get(), r18.get());
                let fonts = DfsFonts {
                    num12: &n12,
                    text16: &n16,
                    text18: &n18,
                    text12: &n12,
                };
                // PORT(panic 边界): 畸形 FM 短行的索引 panic (Java AIOOBE 由 EDT 吞,
                // 窗口存活) 不许毒化 host 槽位锁 — catch_unwind 吞帧留空画布
                // (FmUnpackedFeed tick 包 catch_unwind 的同族契约, PORTING §6)
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    render_handle.borrow().draw(cv, &fonts, aa());
                }));
                if r.is_err() {
                    vm_core::base::logger::error(
                        "DrawFrameSimpl",
                        "paint panic 已吞 (畸形 FM 推力表), 本帧空画布",
                    );
                }
            }),
            None,
        ),
    ))
}

// ---------------------------------------------------------------------------
// run() 循环驱动 (Java :737-767)
// ---------------------------------------------------------------------------

/// run() 退出分支的遥测输入 (Java :756 直读 `xc.S.sState.gear != 100 ||
/// (xc.S.speedv > 10 && xc.S.sState.throttle > 0)`)
pub struct DfsFlight {
    pub gear: i32,
    pub speedv: f64,
    pub throttle: i32,
}

/// DrawFrameSimpl 的 run() 循环驱动侧 (Java :737-767 单线程对位, 渲染线程循环调用)。
///
/// 每轮: 自管可见性落窗 (`shouldShow = isPreview || visible` 的 setVisible 拉起/
/// 隐藏 + repaint — repaint 归 host 渲染节拍脏检查) → `displayFmKey != 0` 时
/// sleepQuietly(1000) = 1000ms 泵节流。`displayFmKey == 0` 分支 Java 无睡眠热自旋
/// (Java bug — 项目先例 flight_log sleep 修复, 不保真), Rust 以渲染节拍 (~50ms)
/// 轮询判定; 条件命中 → sleep 10s → break → dispose (Rust: 10s 等待后 host.close
/// 走销毁链 — 存位置 + drop 窗口)。
pub struct DrawFrameSimplFeed {
    /// 1000ms 节流基准 (displayFmKey != 0 路径; 0 = 首轮放行)
    last_ms: i64,
    /// 10s 退场等待起点 (Some = 已命中退出条件, 线程沉睡中)
    exit_wait_start: Option<i64>,
    /// run 线程已终止 (dispose 后; CloseAll 会话收尾时复位)
    exited: bool,
}

impl Default for DrawFrameSimplFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawFrameSimplFeed {
    pub fn new() -> Self {
        DrawFrameSimplFeed {
            last_ms: 0,
            exit_wait_start: None,
            exited: false,
        }
    }

    /// 单轮驱动。`id` = host 注册键 ("thrustdFS"), `display_fm_key` = Application.
    /// displayFmKey 的 Rust 对位 (ControllerShared.flags.current_fm_hotkey_code,
    /// bind/handleFmHotkeyConfigChange 同步), `flight` = live Service 快照
    /// (None = 无 Service 的预览形态 — Java 此处 NPE 杀线程, Rust 冻结判定保窗口)。
    pub fn pump(
        &mut self,
        host: &mut OverlayHost,
        id: &str,
        handle: &DrawFrameSimplHandle,
        now_ms: i64,
        display_fm_key: i32,
        flight: Option<DfsFlight>,
    ) {
        if self.exited {
            return; // run 线程已终止 (Java dispose 后实例僵在 entry 里直至 closeAll)
        }
        if let Some(start) = self.exit_wait_start {
            // sleepQuietly(10000) 等待期: 线程沉睡不再迭代; 到点 break → dispose
            if now_ms.saturating_sub(start) >= 10_000 {
                vm_core::base::logger::info("DrawFrameSimpl", "Exiting run loop, disposing");
                host.close(id); // 销毁链 (Java dispose: 注销 + 窗口销毁)
                // openAll 跳过 / refreshPreviews 只跑 reinit, 死窗口不复活; 直到
                // closeAll (entry.close → instance=null) 才允许重建
                host.set_entry_zombie(id, true);
                self.exited = true;
            }
            return;
        }
        // 如果配置了热键: sleepQuietly(1000) 节流
        if display_fm_key != 0 && now_ms.saturating_sub(self.last_ms) < 1000 {
            return;
        }
        self.last_ms = now_ms;
        // Self-managed visibility: preview always visible, game mode uses toggle state
        let should_show = handle.borrow().should_show();
        host.set_entry_visible(id, should_show);
        if display_fm_key == 0 {
            if let Some(f) = flight {
                // 如果收起落架则关闭break (sState 缺省 gear/throttle=0 同判收起)
                if f.gear != 100 || (f.speedv > 10.0 && f.throttle > 0) {
                    self.exit_wait_start = Some(now_ms);
                }
            }
        }
    }

    /// CloseAllOverlays 会话收尾复位 (Java closeAll → 实例销毁; 下次 open/
    /// refreshPreview 重建新实例新线程 — 对位 feed 侧 run 循环重生)
    pub fn reset(&mut self) {
        self.last_ms = 0;
        self.exit_wait_start = None;
        self.exited = false;
    }
}

// ---------------------------------------------------------------------------
// 测试: 几何 oracle / 像素墨迹 / run 泵 (toggle + 自动退场) / host 固定几何
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
