//! core.fm.list — FM 拆包数据"可伸长纯文本列表"复合组件 (F 修复)。
//!
//! 字段原子化 (526605a) 把本页拆成 83 个 core.fm.field/meta 碎片, 丢失了
//! "列表"的整体语义 (统一字号/行距/斑马基座/行集伸缩) — 本组件把整页收敛回
//! **一个**列表组件: 行集由 show* 段开关驱动增减, 高度随行数伸缩
//! (preferred_size → refresh_sizing 包围盒收敛链, 页 dataface=sidecar)。
//!
//! 样式 = 原 ZebraListRenderer 斑马基座 (字段原子化时误删, 语义自
//! overlays/list.rs 历史基线恢复): 表头深琥珀 #503C00 / 偶 #191919 /
//! 奇 #282828 / 行文本恒白, 满宽斑马条, 行高 = 2 + 字高 + 2, 左缩进 6,
//! 背景条 alpha=180 (Java BaseOverlay 默认; SrcOver 预合成历史基线值)。
//! 行内容 = 原 FMUnpackedDataOverlay::generateLines 表驱动 (fmVersion 表头
//! + 16 开关段 + 部件族), 逐字恢复。

use vm_core::base::format::{java_string_format, FmtArg};
use vm_core::base::physics_constants::g;
use vm_core::fm::data::{FmData, FmParts};
use vm_core::lang::Lang;

use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::fm_sidecar::{SidecarAction, SidecarCtx, WidgetSidecar};
use super::registry::{HudWidget, PageFonts, WidgetCategory, WidgetMeta};

/// tick 节流 (原 BaseListOverlay getRefreshInterval 默认 200ms)
const REFRESH_MS: i64 = 200;

// ---- ZebraListRenderer 调色板与几何 (历史基线逐字) ----
const HEADER_RGB: [u8; 3] = [80, 60, 0]; // 表头 #503C00 深琥珀
const ZEBRA_EVEN_RGB: [u8; 3] = [25, 25, 25]; // 偶行 #191919
const ZEBRA_ODD_RGB: [u8; 3] = [40, 40, 40]; // 奇行 #282828
const TEXT_COLOR: [u8; 4] = [255, 255, 255, 255]; // 行文本恒白不透明
const MARGIN_TOP: i32 = 2;
const MARGIN_LEFT: i32 = 6;
const MARGIN_BOTTOM: i32 = 2;
/// 行背景 alpha (Java BaseOverlay.alpha 默认 180)
const ROW_ALPHA: u8 = 180;

/// Java2D SrcOver 8bit 整数路径 (WebLaF 双遍背景合成历史基线,
/// 与原 list.rs 逐位一致 — 免 tiny-skia 多层叠 ±1 LSB 漂移)
fn java2d_src_over(s: [u8; 4], d: [u8; 4]) -> [u8; 4] {
    let sa = s[3] as u32;
    if sa == 0 {
        return d;
    }
    let da = d[3] as u32;
    let inv = 255 - sa;
    let oa = sa + (da * inv + 127) / 255;
    if oa == 0 {
        return [0, 0, 0, 0];
    }
    let mut out = [0u8; 4];
    for c in 0..3 {
        let sp = (s[c] as u32 * sa + 127) / 255;
        let dp = (d[c] as u32 * da + 127) / 255;
        let op = sp + (dp * inv + 127) / 255;
        out[c] = ((op * 255 + oa / 2) / oa) as u8;
    }
    out[3] = oa as u8;
    out
}

/// 表头判定 (原默认 headerMatcher: 含"fm器件"或"FM文件"的行)
fn is_header(line: &str) -> bool {
    line.contains("fm器件") || line.contains("FM文件")
}

// ---------------------------------------------------------------------------
// 行内容: generateLines 表驱动 (原 FMUnpackedDataOverlay 逐字恢复)
// ---------------------------------------------------------------------------

/// generateLines: 按 show* 开关过滤的 blkx 字段清单
pub(crate) fn generate_lines(
    fmdata: Option<&FmData>,
    config: &dyn Fn(&str) -> Option<String>,
) -> Vec<String> {
    let lang = Lang::init_lang();
    let mut lines: Vec<String> = Vec::new();
    let fmdata = match fmdata {
        None => {
            lines.push("FM Data Preview".to_string());
            lines.push("[No Data Loaded]".to_string());
            return lines;
        }
        Some(b) => b,
    };
    let ctx = LineCtx { lang: &lang, fmdata };
    // FM Version 表头 (恒入列; Java %s 收 null 打印 "null")
    let fm_version = java_string_format(
        ctx.lang.b_fm_version,
        &[
            FmtArg::S(ctx.fmdata.read_file_name.as_deref().unwrap_or("null")),
            FmtArg::S(ctx.fmdata.version.as_deref().unwrap_or("null")),
        ],
    );
    add_lines(&mut lines, &fm_version);
    // 开关过滤段 (键序 = Java 块序保真)
    for (key, gen) in FM_FIELD_TABLE {
        if is_field_enabled(config, key) {
            gen(&mut lines, &ctx);
        }
    }
    lines
}

struct LineCtx<'a> {
    lang: &'a Lang,
    fmdata: &'a FmData,
}

type LineFn = fn(&mut Vec<String>, &LineCtx);

static FM_FIELD_TABLE: &[(&str, LineFn)] = &[
    ("showWeight", add_weight),
    ("showCritSpeed", add_crit_speed),
    ("showGLoadLimits", add_g_load_limits),
    ("showFlapLimits", add_flap_limits),
    ("showControlEffectiveness", add_control_effectiveness),
    ("showNitro", add_nitro),
    ("showHeatRecovery", add_heat_recovery),
    ("showMaxLiftLoad", add_max_lift_load),
    ("showInertia", add_inertia),
    ("showLift", add_lift),
    ("showDrag", add_drag),
    ("showNoFlapsWing", add_no_flaps_wing),
    ("showFullFlapsWing", add_full_flaps_wing),
    ("showFuselage", add_fuselage),
    ("showFin", add_fin),
    ("showStab", add_stab),
];

/// isFieldEnabled: 键缺失/空 → 默认启用; 否则 parseBoolean
fn is_field_enabled(config: &dyn Fn(&str) -> Option<String>, key: &str) -> bool {
    match config(key) {
        None => true,
        Some(v) if v.is_empty() => true,
        Some(v) => v.eq_ignore_ascii_case("true"),
    }
}

/// addLines: 按 \n 拆行 + Java trim 语义 (只剥 ≤ U+0020) + 跳空行
pub(crate) fn add_lines(lines: &mut Vec<String>, formatted: &str) {
    for line in formatted.split('\n') {
        let trimmed = line.trim_matches(|c: char| c <= '\u{20}');
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }
}

fn add_weight(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_lines(
        lines,
        &java_string_format(
            ctx.lang.b_weight,
            &[
                FmtArg::F(ctx.fmdata.emptyweight),
                FmtArg::F(ctx.fmdata.maxfuelweight),
            ],
        ),
    );
}

fn add_crit_speed(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_lines(
        lines,
        &java_string_format(
            ctx.lang.b_crit_speed,
            &[
                FmtArg::F(ctx.fmdata.critical_speed * 3.6),
                FmtArg::F(ctx.fmdata.vne),
            ],
        ),
    );
}

fn add_g_load_limits(lines: &mut Vec<String>, ctx: &LineCtx) {
    if let Some(raw) = ctx.fmdata.raw_wing_crit_overload {
        let full_neg = 1.2 * (2.0 * raw[0] / (g * ctx.fmdata.grossweight) + 1.0);
        let full_pos = 1.2 * (2.0 * raw[1] / (g * ctx.fmdata.grossweight) - 1.0);
        let half_neg = 1.2 * (2.0 * raw[0] / (g * ctx.fmdata.halfweight) + 1.0);
        let half_pos = 1.2 * (2.0 * raw[1] / (g * ctx.fmdata.halfweight) - 1.0);
        add_lines(
            lines,
            &java_string_format(
                ctx.lang.b_allow_load_factor,
                &[
                    FmtArg::F(full_neg),
                    FmtArg::F(full_pos),
                    FmtArg::F(half_neg),
                    FmtArg::F(half_pos),
                ],
            ),
        );
    }
}

fn add_flap_limits(lines: &mut Vec<String>, ctx: &LineCtx) {
    if let Some(table) = ctx.fmdata.flaps_destruction_ind_speed {
        for i in 0..ctx.fmdata.flaps_destruction_num {
            add_lines(
                lines,
                &java_string_format(
                    ctx.lang.b_flap_restrict,
                    &[
                        FmtArg::D(i),
                        FmtArg::F(table[i as usize][0] * 100.0),
                        FmtArg::F(table[i as usize][1]),
                    ],
                ),
            );
        }
    }
}

fn add_control_effectiveness(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_lines(
        lines,
        &java_string_format(
            ctx.lang.b_eff_speed_and_power_loss,
            &[
                FmtArg::F(ctx.fmdata.elav_eff),
                FmtArg::F(ctx.fmdata.aileron_eff),
                FmtArg::F(ctx.fmdata.rudder_eff),
                FmtArg::F(ctx.fmdata.elav_power_loss),
                FmtArg::F(ctx.fmdata.aileron_power_loss),
                FmtArg::F(ctx.fmdata.rudder_power_loss),
            ],
        ),
    );
}

fn add_nitro(lines: &mut Vec<String>, ctx: &LineCtx) {
    if ctx.fmdata.nitro > 0.0 {
        add_lines(
            lines,
            &java_string_format(
                ctx.lang.b_nitro,
                &[
                    FmtArg::F(ctx.fmdata.nitro),
                    FmtArg::F(ctx.fmdata.nitro / (ctx.fmdata.nitro_decr * 60.0)),
                ],
            ),
        );
    }
}

fn add_heat_recovery(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_lines(
        lines,
        &java_string_format(
            ctx.lang.b_average_heat_recovery,
            &[FmtArg::F(ctx.fmdata.avg_eng_recovery_rate)],
        ),
    );
}

fn add_max_lift_load(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_lines(
        lines,
        &java_string_format(
            ctx.lang.b_max_lift_load350,
            &[
                FmtArg::F((ctx.fmdata.no_flap_wll + 1.0) / 2.0),
                FmtArg::F((ctx.fmdata.full_flap_wll + 1.0) / 2.0),
            ],
        ),
    );
}

fn add_inertia(lines: &mut Vec<String>, ctx: &LineCtx) {
    if let Some(m) = ctx.fmdata.moment_of_inertia {
        if m.len() >= 3 {
            add_lines(
                lines,
                &java_string_format(
                    ctx.lang.b_inertia,
                    &[FmtArg::F(m[2]), FmtArg::F(m[0]), FmtArg::F(m[1])],
                ),
            );
        }
    }
}

fn add_lift(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_lines(
        lines,
        &java_string_format(
            ctx.lang.b_lift,
            &[
                FmtArg::F(ctx.fmdata.a_wing),
                FmtArg::F(ctx.fmdata.a_fuselage),
                FmtArg::F(ctx.fmdata.no_flap_wll),
                FmtArg::F(ctx.fmdata.full_flap_wll),
                FmtArg::F(ctx.fmdata.oswalds_efficiency_number),
                FmtArg::F(ctx.fmdata.aspect_ratio),
                FmtArg::F(ctx.fmdata.swept_wing_angle),
            ],
        ),
    );
}

fn add_drag(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_lines(
        lines,
        &java_string_format(
            ctx.lang.b_drag,
            &[
                FmtArg::F(ctx.fmdata.cd_s),
                FmtArg::F(ctx.fmdata.cd_s / (ctx.fmdata.halfweight / 1000.0)),
                FmtArg::F(ctx.fmdata.ind_cd_f),
                FmtArg::F(ctx.fmdata.halfweight * ctx.fmdata.ind_cd_f),
                FmtArg::F(ctx.fmdata.radiator_cd),
                FmtArg::F(ctx.fmdata.oil_radiator_cd),
            ],
        ),
    );
}

fn add_no_flaps_wing(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.no_flaps_wing.as_ref());
}

fn add_full_flaps_wing(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.full_flaps_wing.as_ref());
}

fn add_fuselage(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.fuselage.as_ref());
}

fn add_fin(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.fin.as_ref());
}

fn add_stab(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.stab.as_ref());
}

/// addFmParts: 表头 + 4 数据行 (null 部件整段跳过)
fn add_fm_parts(lines: &mut Vec<String>, lang: &Lang, p: Option<&FmParts>) {
    let p = match p {
        None => return,
        Some(p) => p,
    };
    add_lines(
        lines,
        &java_string_format(lang.b_fm_parts, &[FmtArg::S(p.name.as_deref().unwrap_or("null"))]),
    );
    add_lines(lines, &java_string_format(lang.b_cd_min, &[FmtArg::F(p.cd_min)]));
    add_lines(lines, &java_string_format(lang.b_cl0, &[FmtArg::F(p.cl0)]));
    add_lines(
        lines,
        &java_string_format(
            lang.b_ao_a_crit,
            &[FmtArg::F(p.aoa_crit_low), FmtArg::F(p.aoa_crit_high)],
        ),
    );
    add_lines(
        lines,
        &java_string_format(
            lang.b_ao_a_crit_cl,
            &[FmtArg::F(p.cl_crit_low), FmtArg::F(p.cl_crit_high)],
        ),
    );
}

// ---------------------------------------------------------------------------
// 组件本体
// ---------------------------------------------------------------------------

pub struct FmListWidget {
    /// 当前行集 (渲染真源; preview = 无 FM 占位两行)
    lines: Vec<String>,
    /// 列表字体 (单档; "纯文本列表"的统一字号语义)
    font: LoadedFont,
    /// 宽度 (原 BaseOverlay.init 公式: 字号 × 36 × logicalHeight/1440)
    width: i32,
    /// 200ms 节流基准
    last_ms: i64,
}

impl FmListWidget {
    /// 行高 = 2 + 字高 + 2 (WebLabel margin + FontMetrics.height)
    fn row_height(&self) -> i32 {
        MARGIN_TOP + self.font.metrics().height + MARGIN_BOTTOM
    }

    /// 当前 preferred 高 (行数 × 行高 — "可伸长"面: 行集增减高度随动)
    fn height(&self) -> i32 {
        self.lines.len() as i32 * self.row_height()
    }

    /// 斑马渲染 (原 ZebraList::draw 语义: 满宽行条 + 白字, 表头不打断偶奇)
    fn draw_zebra(&self, cv: &mut PixCanvas, x: i32, y: i32, aa: bool) {
        let row_h = self.row_height();
        let w = self.width;
        let alpha = ROW_ALPHA;
        // panel 底色双叠 (WebLaF 双遍背景历史基线)
        let panel = [20u8, 20, 20, alpha];
        let panel2 = java2d_src_over(panel, java2d_src_over(panel, [0, 0, 0, 0]));
        let mut zebra = 0i32;
        for (i, line) in self.lines.iter().enumerate() {
            let ry = y + i as i32 * row_h;
            let header = is_header(line);
            let bg = if header {
                [HEADER_RGB[0], HEADER_RGB[1], HEADER_RGB[2], alpha]
            } else if zebra % 2 == 0 {
                [ZEBRA_EVEN_RGB[0], ZEBRA_EVEN_RGB[1], ZEBRA_EVEN_RGB[2], alpha]
            } else {
                [ZEBRA_ODD_RGB[0], ZEBRA_ODD_RGB[1], ZEBRA_ODD_RGB[2], alpha]
            };
            if !header {
                zebra += 1;
            }
            cv.fill_rect(x, ry, w, row_h, java2d_src_over(bg, panel2));
        }
        let ascent = self.font.metrics().ascent;
        for (i, line) in self.lines.iter().enumerate() {
            cv.draw_text(
                &self.font,
                x + MARGIN_LEFT,
                y + i as i32 * row_h + MARGIN_TOP + ascent,
                line,
                TEXT_COLOR,
                aa,
            );
        }
    }

    /// 测试断言面: 行集快照
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

fn f_fm_list(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let fonts_dir = fctx
        .fonts_dir
        .as_deref()
        .ok_or("fm.list 需要 FactoryCtx.fonts_dir")?;
    // 单档字体 = 页面字号 (R4 字号合一: fonts.draw.size 含 doc.font.size_add + dpi)
    let font = LoadedFont::new(&fonts_dir.join("sarasa-mono-sc-bold.ttf"), fctx.fonts.draw.size)?;
    // 宽度 = 原 BaseOverlay.init 公式 (defaultFontsize=24 基准 × 36 × 屏高比)
    let logical_h = fctx.gauge_cfg.map(|c| c.logical_height).unwrap_or(1080);
    let width =
        vm_core::base::format::java_round_f32(fctx.fonts.draw.size as f32 * 36.0 * (logical_h as f32 / 1440.0));
    Ok(Box::new(FmListWidget {
        // 构造即 preview 行集 (无 FM 占位两行 — 空 preferred 高 0 是非法画布)
        lines: generate_lines(None, &|_| None),
        font,
        width,
        last_ms: 0,
    }))
}

impl HudWidget for FmListWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, _env: &UpdateEnv) {
        // 数据面全走 sidecar tick (200ms 自节流 + FM 直读 + show* 过滤)
    }

    fn reset_preview(&mut self) {
        // 原黑盒 initPreview: 无 FM 占位两行
        self.lines = generate_lines(None, &|_| None);
        self.last_ms = 0;
    }

    fn sidecar(&mut self) -> Option<&mut (dyn WidgetSidecar + 'static)> {
        let s: &mut dyn WidgetSidecar = self;
        Some(s)
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        self.draw_zebra(cv, x, y, aa);
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        Dimension::new(self.width, self.height())
    }
}

use crate::layout::hud_layout_node::Dimension;

impl WidgetSidecar for FmListWidget {
    fn tick(&mut self, ctx: &mut SidecarCtx) -> SidecarAction {
        // 200ms 节流 (原 BaseListOverlay.refresh_interval_ms)
        if ctx.now_ms - self.last_ms < REFRESH_MS {
            return SidecarAction::None;
        }
        self.last_ms = ctx.now_ms;
        // FM 直读 (None = 未加载 → 占位; FM_CHANGED 载荷优先)
        let fm_changed = ctx.fm_changed.clone();
        let current = ctx.fm.current();
        let fm_ref: Option<&FmData> = fm_changed
            .as_deref()
            .or_else(|| current.fmdata.as_ref());
        let cfg = ctx.fm_field_config;
        let new_lines = generate_lines(fm_ref, &|k: &str| cfg(k));
        if new_lines != self.lines {
            self.lines = new_lines;
            // 高度随行数伸缩: preferred_size 变化经页级 refresh_sizing
            // 包围盒收敛链落地 (渲染节拍块对 sidecar 页逐 tick 收敛),
            // 原子化期间遗留的行归零/补位无空隙语义由行集整体重排天然成立
        }
        SidecarAction::None
    }
}

// ---------------------------------------------------------------------------
// 注册表
// ---------------------------------------------------------------------------

const FM_LIST_KEYS: &[&str] = &["enableFMPrint", "displayFmKey", "fontName"];

const FM_LIST_META: WidgetMeta = WidgetMeta {
    type_name: "core.fm.list",
    display_zh: "FM数据列表",
    category: WidgetCategory::List,
    composite: true,
    props_schema: &[],
    config_keys: FM_LIST_KEYS,
    data_shorts: &[],
    default_props: "{}",
    factory: f_fm_list,
};

pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] = &[FM_LIST_META];
