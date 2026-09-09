//! core.fm.field / core.fm.meta — FM 拆包字段原子组件 (fm.list 黑盒退役)。
//!
//! 每字段一实例: props.key → FmData 取值 (稳定 key 表, 计算行如
//! gload/lift350/drag 族的原公式内化在取值函数); 无 FM/字段缺/段开关关 →
//! preferred 归零 (行消失, 链式排布后件上移 = 原列表行消失语义)。
//! 数据面走 sidecar tick: FMManager 句柄直读 + 200ms 自节流 (原
//! FmUnpacked 泵节拍), show* 段开关逐 tick 读 fm_field_config 快照
//! (原 generateLines 的 isFieldEnabled 直读面)。单字段无 toggle/重开窗
//! 语义 → 恒返回 None 动作。preview (无 live 帧) = 字段静态示例值。
//!
//! 窗口高度自适应 (原 fm.list 的行数滞回 Resize): 原子化后由页面
//! refresh_sizing 承担 — 渲染线程 sidecar 节拍逐 tick 重算布局, 行归零/
//! 恢复 → 链式补位 → 包围盒收缩/扩张 → resize_entry, 无滞回逐步收敛。

use vm_core::base::format;
use vm_core::base::format::{java_string_format, FmtArg};
use vm_core::base::physics_constants::g;
use vm_core::fm::data::{FmData, FmParts};
use vm_core::lang::Lang;

use crate::layout::hud_layout_node::Dimension;
use crate::layout::RenderCtx;
use crate::overlays::flight_info::default_num_height;
use crate::render::canvas::PixCanvas;
use crate::render::fields::FontTriple;
use crate::render::palette::colors;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::fm_sidecar::{SidecarAction, SidecarCtx, WidgetSidecar};
use super::registry::PageFonts;
use super::registry::{HudWidget, PropKind, PropSchema, WidgetCategory, WidgetMeta};

/// 取数节流 (原 FmUnpackedDataOverlay.getRefreshInterval = 200ms)
const FM_FIELD_REFRESH_MS: i64 = 200;

/// 数值字段显示元数据: (label, unit, precision, preview 示例值)。
/// 静态 key 表 (原 generateLines 各 Lang 模板行拆成单字段后的显示面)。
static FM_NUM_META: &[(&str, &str, &str, u8, &str)] = &[
    ("weight.empty", "空重", "kg", 1, "3050.0"),
    ("weight.maxfuel", "最大燃油重量", "kg", 1, "780.5"),
    ("speed.crit", "临界速度", "km/h", 0, "828"),
    ("speed.vne", "VNE", "km/h", 0, "1050"),
    ("gload.full_neg", "过载满油负", "g", 1, "-8.4"),
    ("gload.full_pos", "过载满油正", "g", 1, "20.4"),
    ("gload.half_neg", "过载半油负", "g", 1, "-10.8"),
    ("gload.half_pos", "过载半油正", "g", 1, "25.8"),
    ("ctrl.elev_eff", "升降舵效速度", "km/h", 0, "580"),
    ("ctrl.aileron_eff", "副翼有效速度", "km/h", 0, "640"),
    ("ctrl.rudder_eff", "方向舵效速度", "km/h", 0, "700"),
    ("ctrl.elev_loss", "升降舵锁舵", "", 1, "0.3"),
    ("ctrl.aileron_loss", "副翼锁舵", "", 1, "0.4"),
    ("ctrl.rudder_loss", "方向舵锁舵", "", 1, "0.5"),
    ("nitro.amount", "加力质量", "kg", 1, "120.0"),
    ("nitro.minutes", "加力时限", "min", 1, "1.0"),
    ("heat.avg_recovery", "耐热恢复率", "", 1, "3.2"),
    ("lift350.no_flap", "最大升力过载", "g", 1, "5.0"),
    ("lift350.full_flap", "最大升力过载(襟)", "g", 1, "7.0"),
    ("inertia.p", "转动惯量P", "", 0, "8000"),
    ("inertia.r", "转动惯量R", "", 0, "12000"),
    ("inertia.y", "转动惯量Y", "", 0, "25000"),
    ("wing.area", "机翼面积", "m²", 1, "25.8"),
    ("fuselage.area", "机身面积", "m²", 1, "5.4"),
    ("wing.load_no_flap", "翼载", "", 2, "9.00"),
    ("wing.load_full_flap", "翼载(襟)", "", 2, "13.00"),
    ("wing.oswalds", "翼展效率", "", 2, "0.75"),
    ("wing.aspect_ratio", "展弦比", "", 1, "6.0"),
    ("wing.sweep_angle", "后掠角", "°", 1, "0.0"),
    ("drag.cd_s", "阻力面积因数", "", 2, "0.42"),
    ("drag.cd_accel", "阻力加速度系数", "", 3, "0.105"),
    ("drag.ind_cd", "诱导阻力因数", "", 3, "0.003"),
    ("drag.ind_accel", "诱导阻力加速度", "", 0, "12"),
    ("drag.radiator", "散热器阻力系数", "", 3, "0.021"),
    ("drag.oil_radiator", "油冷器阻力系数", "", 3, "0.017"),
];

/// 部件段字段元数据 (五段共用同名字段, 段头 meta 行区分)
static FM_PART_FIELD_META: &[(&str, &str, &str, u8, &str)] = &[
    ("cd_min", "零升阻力系数", "", 3, "0.029"),
    ("cl0", "零攻角升力", "", 3, "0.050"),
    ("aoa_low", "临界攻角低", "°", 1, "-14.4"),
    ("aoa_high", "临界攻角高", "°", 1, "18.6"),
    ("cl_low", "临界升力低", "", 2, "-1.15"),
    ("cl_high", "临界升力高", "", 2, "1.55"),
];

/// 襟翼档 preview 示例值 (静态 6 档; 档数外的组件数据面归零)
const FLAP_PREVIEW_PCT: [&str; 6] = ["0", "95", "90", "85", "80", "75"];
const FLAP_PREVIEW_SPEED: [&str; 6] = ["640", "520", "480", "450", "420", "400"];

/// 部件段名 → FmData 部件字段
fn part_of<'a>(fm: &'a FmData, seg: &str) -> Option<&'a FmParts> {
    match seg {
        "no_flaps_wing" => fm.no_flaps_wing.as_ref(),
        "full_flaps_wing" => fm.full_flaps_wing.as_ref(),
        "fuselage" => fm.fuselage.as_ref(),
        "fin" => fm.fin.as_ref(),
        "stab" => fm.stab.as_ref(),
        _ => None,
    }
}

/// FM 数值字段取值 (key 表 + 计算公式内化; None = 行消失)。
/// 数据条件对位原 generateLines: gload 需 raw 表 / nitro>0 / 惯量三分量齐 /
/// 襟翼档数内 / 部件存在。
pub(crate) fn fm_num_value(fm: &FmData, key: &str) -> Option<f64> {
    // 部件段 "parts.{seg}.{field}"
    if let Some(rest) = key.strip_prefix("parts.") {
        let (seg, field) = rest.split_once('.')?;
        let p = part_of(fm, seg)?;
        return match field {
            "cd_min" => Some(p.cd_min),
            "cl0" => Some(p.cl0),
            "aoa_low" => Some(p.aoa_crit_low),
            "aoa_high" => Some(p.aoa_crit_high),
            "cl_low" => Some(p.cl_crit_low),
            "cl_high" => Some(p.cl_crit_high),
            _ => None,
        };
    }
    // 襟翼档段 "flap_limit.{i}.{pct|speed}" (原 Java AIOOBE 的档数守卫 → None)
    if let Some(rest) = key.strip_prefix("flap_limit.") {
        let (i, field) = rest.split_once('.')?;
        let i = i.parse::<usize>().ok()?;
        let table = fm.flaps_destruction_ind_speed.as_ref()?;
        if i >= fm.flaps_destruction_num.min(6) as usize {
            return None;
        }
        return match field {
            "pct" => Some(table[i][0] * 100.0),
            "speed" => Some(table[i][1]),
            _ => None,
        };
    }
    let raw = fm.raw_wing_crit_overload;
    let moi = fm.moment_of_inertia;
    match key {
        "weight.empty" => Some(fm.emptyweight),
        "weight.maxfuel" => Some(fm.maxfuelweight),
        "speed.crit" => Some(fm.critical_speed * 3.6),
        "speed.vne" => Some(fm.vne),
        // 原 add_g_load_limits 内联公式 (getMaxAllowGloadForWeight 同式)
        "gload.full_neg" => raw.map(|r| 1.2 * (2.0 * r[0] / (g * fm.grossweight) + 1.0)),
        "gload.full_pos" => raw.map(|r| 1.2 * (2.0 * r[1] / (g * fm.grossweight) - 1.0)),
        "gload.half_neg" => raw.map(|r| 1.2 * (2.0 * r[0] / (g * fm.halfweight) + 1.0)),
        "gload.half_pos" => raw.map(|r| 1.2 * (2.0 * r[1] / (g * fm.halfweight) - 1.0)),
        "ctrl.elev_eff" => Some(fm.elav_eff),
        "ctrl.aileron_eff" => Some(fm.aileron_eff),
        "ctrl.rudder_eff" => Some(fm.rudder_eff),
        "ctrl.elev_loss" => Some(fm.elav_power_loss),
        "ctrl.aileron_loss" => Some(fm.aileron_power_loss),
        "ctrl.rudder_loss" => Some(fm.rudder_power_loss),
        // 原 add_nitro 段内 nitro > 0 门控
        "nitro.amount" => (fm.nitro > 0.0).then_some(fm.nitro),
        "nitro.minutes" => (fm.nitro > 0.0).then_some(fm.nitro / (fm.nitro_decr * 60.0)),
        "heat.avg_recovery" => Some(fm.avg_eng_recovery_rate),
        "lift350.no_flap" => Some((fm.no_flap_wll + 1.0) / 2.0),
        "lift350.full_flap" => Some((fm.full_flap_wll + 1.0) / 2.0),
        // 原 add_inertia 三分量齐才显示; P=m[2] R=m[0] Y=m[1]
        "inertia.p" => moi.filter(|m| m.len() >= 3).map(|m| m[2]),
        "inertia.r" => moi.filter(|m| m.len() >= 3).map(|m| m[0]),
        "inertia.y" => moi.filter(|m| m.len() >= 3).map(|m| m[1]),
        "wing.area" => Some(fm.a_wing),
        "fuselage.area" => Some(fm.a_fuselage),
        "wing.load_no_flap" => Some(fm.no_flap_wll),
        "wing.load_full_flap" => Some(fm.full_flap_wll),
        "wing.oswalds" => Some(fm.oswalds_efficiency_number),
        "wing.aspect_ratio" => Some(fm.aspect_ratio),
        "wing.sweep_angle" => Some(fm.swept_wing_angle),
        // 原 add_drag 的派生系数两行
        "drag.cd_s" => Some(fm.cd_s),
        "drag.cd_accel" => Some(fm.cd_s / (fm.halfweight / 1000.0)),
        "drag.ind_cd" => Some(fm.ind_cd_f),
        "drag.ind_accel" => Some(fm.halfweight * fm.ind_cd_f),
        "drag.radiator" => Some(fm.radiator_cd),
        "drag.oil_radiator" => Some(fm.oil_radiator_cd),
        _ => None,
    }
}

/// FM 文本行取值 (版本行 + 器件段头; None = 行消失)。
/// Java %s 收 null 字段打印 "null" (Formatter 行为), Option 展开对齐;
/// 部件缺席 → 段头与段内字段一并消失 (原 addFmParts 的 null 整段跳过)。
/// 模板尾 \n 按 Java trim 语义剥除 (原 addLines 的行拆面)。
pub(crate) fn fm_text_value(fm: &FmData, key: &str) -> Option<String> {
    let lang = Lang::init_lang();
    let formatted = if key == "fm.version" {
        java_string_format(
            lang.b_fm_version,
            &[
                FmtArg::S(fm.read_file_name.as_deref().unwrap_or("null")),
                FmtArg::S(fm.version.as_deref().unwrap_or("null")),
            ],
        )
    } else {
        let seg = key.strip_prefix("parts.")?.split_once('.')?.0;
        let p = part_of(fm, seg)?;
        java_string_format(
            lang.b_fm_parts,
            &[FmtArg::S(p.name.as_deref().unwrap_or("null"))],
        )
    };
    Some(formatted.trim_matches(|c: char| c <= '\u{20}').to_string())
}

/// 数值字段元数据查找 (label/unit/precision/preview; flap 档动态拼 label)
fn fm_num_meta(key: &str) -> Option<(String, &'static str, u8, String)> {
    if let Some(rest) = key.strip_prefix("parts.") {
        let (_, field) = rest.split_once('.')?;
        let (_, l, u, p, v) = FM_PART_FIELD_META.iter().find(|(f, ..)| *f == field)?;
        return Some(((*l).to_string(), u, *p, (*v).to_string()));
    }
    if let Some(rest) = key.strip_prefix("flap_limit.") {
        let (i, field) = rest.split_once('.')?;
        let i = i.parse::<usize>().ok()?;
        if i >= 6 {
            return None;
        }
        return match field {
            "pct" => {
                Some((format!("襟翼{i}开度"), "%", 0, FLAP_PREVIEW_PCT[i].to_string()))
            }
            "speed" => Some((
                format!("襟翼{i}限速"),
                "km/h",
                0,
                FLAP_PREVIEW_SPEED[i].to_string(),
            )),
            _ => None,
        };
    }
    let (_, l, u, p, v) = FM_NUM_META.iter().find(|(k, ..)| *k == key)?;
    Some(((*l).to_string(), u, *p, (*v).to_string()))
}

/// 文本行 preview 示例值 (合法 key 校验兼任)
fn fm_text_preview(key: &str) -> Option<String> {
    Some(
        match key {
            "fm.version" => "FM文件: spitfire_mk24 - 2.35.0.9",
            "parts.no_flaps_wing.name" => "------fm器件 机翼 无襟翼------",
            "parts.full_flaps_wing.name" => "------fm器件 机翼 全襟翼------",
            "parts.fuselage.name" => "------fm器件 机身------",
            "parts.fin.name" => "------fm器件 垂尾------",
            "parts.stab.name" => "------fm器件 平尾------",
            _ => return None,
        }
        .to_string(),
    )
}

/// isFieldEnabled 对位: 快照缺键/空串 → 默认启用; 否则 parseBoolean
/// (仅忽略大小写的 "true" 为真)
fn field_enabled(cfg: &dyn Fn(&str) -> Option<String>, key: &str) -> bool {
    match cfg(key) {
        None => true,
        Some(v) if v.is_empty() => true,
        Some(v) => v.eq_ignore_ascii_case("true"),
    }
}

/// 取值分派 (field=数值格式化 / meta=文本原样)
enum FmFetch {
    Num { precision: u8 },
    Text,
}

/// 组件共通数据面 (field/meta 同一节流/开关/preview 骨架)
struct FmFieldCore {
    key: String,
    fetch: FmFetch,
    /// show* 段开关键 (None = 恒显; 原段落开关的组件化承接)
    switch_key: Option<String>,
    preview_text: String,
    ctx: RenderCtx,
    fonts: FontTriple,
    /// 当前行文本 (数值格式化后 / meta 原文)
    text: String,
    shown: bool,
    last_ms: i64,
}

impl FmFieldCore {
    /// sidecar tick 单轮: preview 冻结 → 200ms 节流 → 段开关 → FM 取值。
    /// 恒 None 动作 (单字段无窗口交互面; 高度跟随由页面 refresh_sizing 承担)。
    fn tick(&mut self, ctx: &SidecarCtx) -> SidecarAction {
        // preview (无 live 帧) 保持静态示例值
        if ctx.frame.is_none() {
            return SidecarAction::None;
        }
        if ctx.now_ms.saturating_sub(self.last_ms) < FM_FIELD_REFRESH_MS {
            return SidecarAction::None;
        }
        self.last_ms = ctx.now_ms;
        // 段开关 (原 generateLines 逐 tick 直读 isFieldEnabled)
        if let Some(k) = &self.switch_key {
            if !field_enabled(ctx.fm_field_config, k) {
                self.shown = false;
                return SidecarAction::None;
            }
        }
        // FM 句柄直读 (P3: current() 快照, 绝不触发加载)
        let fetched = ctx.fm.current().fmdata.as_ref().and_then(|f| {
            match self.fetch {
                FmFetch::Num { precision } => {
                    fm_num_value(f, &self.key).map(|v| format::format(v, precision))
                }
                FmFetch::Text => fm_text_value(f, &self.key),
            }
        });
        match fetched {
            Some(t) => {
                self.shown = true;
                self.text = t;
            }
            None => self.shown = false, // 行消失 (preferred 归零, 链式补位)
        }
        SidecarAction::None
    }

    fn reset_preview(&mut self) {
        self.text = self.preview_text.clone();
        self.shown = true;
        self.last_ms = 0;
    }

    /// 阴影文本 (+1,+1) — data_field 同式
    #[allow(clippy::too_many_arguments)]
    fn draw_shaded(
        cv: &mut PixCanvas,
        font: &crate::render::font::LoadedFont,
        x: i32,
        baseline: i32,
        text: &str,
        color: [u8; 4],
        shade: [u8; 4],
        aa: bool,
    ) {
        cv.draw_text(font, x + 1, baseline + 1, text, shade, aa);
        cv.draw_text(font, x, baseline, text, color, aa);
    }
}

/// 工厂共通: 字号 (原列表 setupFont 的 14+add 面: 页面主字号基准 24 →
/// -10 折算; R4 组增量退役 → 页 doc.font.size_add 已含 + props.fontAdd)
fn core_of(
    props: &serde_json::Value,
    fctx: &FactoryCtx,
    key: &str,
    fetch: FmFetch,
    preview: String,
) -> Result<FmFieldCore, String> {
    let font_add = fctx.fonts.draw.size - 24 - 10
        + props.get("fontAdd").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    // column=2: FM 字段行 label 较长 (阻力系数类), 双列宽容纳
    let ctx = RenderCtx::new(font_add, 2, default_num_height(font_add));
    let fonts = FontTriple::load(
        fctx.fonts_dir
            .as_deref()
            .ok_or("fm 组件需要 FactoryCtx.fonts_dir")?,
        &ctx,
    )?;
    Ok(FmFieldCore {
        key: key.to_string(),
        fetch,
        switch_key: props
            .get("switchKey")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        preview_text: props
            .get("previewValue")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&preview)
            .to_string(),
        ctx,
        fonts,
        text: preview,
        shown: true,
        last_ms: 0,
    })
}

// =====================================================================
// core.fm.field — 单 FM 数值字段 (数值 + label + 单位 三段行)
// =====================================================================

/// FM 数值字段原子组件
pub struct FmFieldWidget {
    core: FmFieldCore,
    label: String,
    unit: String,
}

fn f_fm_field(
    props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let key = props
        .get("key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or("fm.field 需要 props.key (FM 字段标识)")?;
    let (label, unit, precision, preview) =
        fm_num_meta(key).ok_or_else(|| format!("fm.field 未知 key: {key}"))?;
    let label = props
        .get("label")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or(label);
    let unit = props
        .get("unit")
        .and_then(|v| v.as_str())
        .unwrap_or(unit)
        .to_string();
    let precision = props
        .get("precision")
        .and_then(|v| v.as_u64())
        .unwrap_or(precision as u64) as u8;
    let core = core_of(
        props,
        fctx,
        key,
        FmFetch::Num { precision },
        preview,
    )?;
    Ok(Box::new(FmFieldWidget {
        core,
        label,
        unit,
    }))
}

impl FmFieldWidget {
    /// 测试断言面: 当前值文本
    pub fn text(&self) -> &str {
        &self.core.text
    }

    /// 测试断言面: 当前显示判定 (行在场/消失)
    pub fn shown(&self) -> bool {
        self.core.shown
    }
}

impl HudWidget for FmFieldWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, _env: &UpdateEnv) {
        // 数据面全走 sidecar tick (200ms 自节流 + FM 直读)
    }

    fn reset_preview(&mut self) {
        self.core.reset_preview();
    }

    fn sidecar(&mut self) -> Option<&mut (dyn WidgetSidecar + 'static)> {
        let s: &mut dyn WidgetSidecar = self;
        Some(s)
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        if !self.core.shown {
            return; // 行消失语义: 不画 (preferred 高度已归零)
        }
        let (ox, oy) = self.core.ctx.start_offset();
        let pal = colors();
        // --- 数值 (右对齐 lwidth 竖线) ---
        let vw = self.core.fonts.num.measure(&self.core.text);
        let vx = x + ox + self.core.ctx.lwidth() - vw - self.core.ctx.num_padding();
        FmFieldCore::draw_shaded(
            cv,
            &self.core.fonts.num,
            vx,
            y + self.core.ctx.value_baseline(oy),
            &self.core.text,
            pal.num,
            pal.shade_shape,
            aa,
        );
        // --- 标签 ---
        FmFieldCore::draw_shaded(
            cv,
            &self.core.fonts.label,
            x + ox + self.core.ctx.lwidth(),
            y + oy,
            &self.label,
            pal.label,
            pal.shade_shape,
            aa,
        );
        // --- 单位 ---
        FmFieldCore::draw_shaded(
            cv,
            &self.core.fonts.unit,
            x + ox + self.core.ctx.lwidth(),
            y + self.core.ctx.unit_baseline(oy),
            &self.unit,
            pal.unit,
            pal.shade_shape,
            aa,
        );
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // 单行; 行消失 → 高度归零 (链式后件上移)
        let h = if self.core.shown {
            self.core.ctx.advance_y()
        } else {
            0
        };
        Dimension::new(self.core.ctx.total_width(), h)
    }
}

impl WidgetSidecar for FmFieldWidget {
    fn tick(&mut self, ctx: &mut SidecarCtx) -> SidecarAction {
        self.core.tick(ctx)
    }
}

// =====================================================================
// core.fm.meta — FM 文本行 (版本行 / 器件段头)
// =====================================================================

/// FM 文本行原子组件
pub struct FmMetaWidget {
    core: FmFieldCore,
}

fn f_fm_meta(
    props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let key = props
        .get("key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or("fm.meta 需要 props.key (FM 文本行标识)")?;
    let preview =
        fm_text_preview(key).ok_or_else(|| format!("fm.meta 未知 key: {key}"))?;
    let core = core_of(props, fctx, key, FmFetch::Text, preview)?;
    Ok(Box::new(FmMetaWidget { core }))
}

impl FmMetaWidget {
    /// 测试断言面: 当前行文本
    pub fn text(&self) -> &str {
        &self.core.text
    }

    /// 测试断言面: 当前显示判定
    pub fn shown(&self) -> bool {
        self.core.shown
    }
}

impl HudWidget for FmMetaWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, _env: &UpdateEnv) {
        // 数据面全走 sidecar tick
    }

    fn reset_preview(&mut self) {
        self.core.reset_preview();
    }

    fn sidecar(&mut self) -> Option<&mut (dyn WidgetSidecar + 'static)> {
        let s: &mut dyn WidgetSidecar = self;
        Some(s)
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        if !self.core.shown {
            return;
        }
        let (ox, oy) = self.core.ctx.start_offset();
        let pal = colors();
        // 单行文本 (num 档, 基线与字段行数值同面 — 版本行/段头醒目)
        FmFieldCore::draw_shaded(
            cv,
            &self.core.fonts.num,
            x + ox,
            y + self.core.ctx.value_baseline(oy),
            &self.core.text,
            pal.num,
            pal.shade_shape,
            aa,
        );
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        let h = if self.core.shown {
            self.core.ctx.advance_y()
        } else {
            0
        };
        Dimension::new(self.core.ctx.total_width(), h)
    }
}

impl WidgetSidecar for FmMetaWidget {
    fn tick(&mut self, ctx: &mut SidecarCtx) -> SidecarAction {
        self.core.tick(ctx)
    }
}

// =====================================================================
// 注册表
// =====================================================================

const FM_KEYS: &[&str] = &["enableFMPrint", "displayFmKey", "fontName", "fmFontSize"];

const FM_FIELD_META_ENTRY: WidgetMeta = WidgetMeta {
    type_name: "core.fm.field",
    display_zh: "FM字段",
    category: WidgetCategory::Text,
    composite: false,
    props_schema: &[
        PropSchema {
            key: "key",
            display_zh: "FM字段标识",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "switchKey",
            display_zh: "段开关键",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "label",
            display_zh: "描述",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "unit",
            display_zh: "单位",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "precision",
            display_zh: "小数位",
            kind: PropKind::Int,
        },
        PropSchema {
            key: "previewValue",
            display_zh: "预览值",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "fontAdd",
            display_zh: "字号增量",
            kind: PropKind::Int,
        },
    ],
    config_keys: FM_KEYS,
    data_shorts: &[],
    default_props: r#"{"key":"weight.empty"}"#,
    factory: f_fm_field,
};

const FM_META_META_ENTRY: WidgetMeta = WidgetMeta {
    type_name: "core.fm.meta",
    display_zh: "FM文本行",
    category: WidgetCategory::Text,
    composite: false,
    props_schema: &[
        PropSchema {
            key: "key",
            display_zh: "FM文本行标识",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "switchKey",
            display_zh: "段开关键",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "previewValue",
            display_zh: "预览值",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "fontAdd",
            display_zh: "字号增量",
            kind: PropKind::Int,
        },
    ],
    config_keys: FM_KEYS,
    data_shorts: &[],
    default_props: r#"{"key":"fm.version"}"#,
    factory: f_fm_meta,
};

pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] =
    &[FM_FIELD_META_ENTRY, FM_META_META_ENTRY];

// =====================================================================
// Tests (原 fields_tests 的 FmUnpacked 语义面组件化改写)
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::registry::lookup_widget;
    use vm_core::base::bus::EventBus;
    use vm_core::fm::manager::FMManager;

    fn fonts_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts")
    }

    fn make(props: serde_json::Value) -> Box<dyn HudWidget> {
        let font = crate::render::font::LoadedFont::new(
            &fonts_dir().join("sarasa-mono-sc-bold.ttf"),
            24,
        )
        .unwrap();
        let rc = std::rc::Rc::new(font);
        let fonts = std::rc::Rc::new(super::super::registry::PageFonts {
            draw: std::rc::Rc::clone(&rc),
            small: std::rc::Rc::clone(&rc),
            s_small: rc,
        });
        let fctx = FactoryCtx {
            minihud_ctx: None,
            fonts,
            lang: None,
            fonts_dir: Some(fonts_dir()),
            gauge_cfg: None,
        };
        (lookup_widget("core.fm.field").unwrap().factory)(&props, &fctx).expect("工厂构造")
    }

    /// 全字段齐备的测试 FM (原 fields_tests full_fmdata 迁移)
    fn full_fmdata() -> FmData {
        let mut b = FmData::default();
        b.read_file_name = Some("spitfire_mk24".to_string());
        b.version = Some("2.35.0.9".to_string());
        b.emptyweight = 3050.0;
        b.maxfuelweight = 780.45; // %.1f HALF_UP → "780.5"
        b.critical_speed = 230.0; // ×3.6 → "828"
        b.vne = 1050.0;
        b.raw_wing_crit_overload = Some([-196000.0, 441000.0]);
        b.grossweight = 5000.0; // full → (-8.4, 20.4)
        b.halfweight = 4000.0; // half → (-10.8, 25.8)
        b.flaps_destruction_num = 2;
        let mut flaps = [[0.0; 2]; 6];
        flaps[0] = [0.0, 640.0];
        flaps[1] = [0.95, 520.0]; // ×100 = 94.99… → %.0f → "95"
        b.flaps_destruction_ind_speed = Some(flaps);
        b.elav_eff = 580.0;
        b.aileron_eff = 640.0;
        b.rudder_eff = 700.0;
        b.elav_power_loss = 0.25; // %.1f HALF_UP → "0.3"
        b.aileron_power_loss = 0.35;
        b.rudder_power_loss = 0.45;
        b.nitro = 120.0;
        b.nitro_decr = 2.0; // 120/(2·60) = 1.0
        b.avg_eng_recovery_rate = 3.25; // %.1f HALF_UP → "3.3"
        b.no_flap_wll = 9.0; // (9+1)/2 = 5.0
        b.full_flap_wll = 13.0;
        b.moment_of_inertia = Some([12000.0, 25000.0, 8000.0]); // P:m[2] R:m[0] Y:m[1]
        b.a_wing = 25.8;
        b.a_fuselage = 5.4;
        b.oswalds_efficiency_number = 0.75;
        b.aspect_ratio = 6.0;
        b.swept_wing_angle = 0.0;
        b.cd_s = 0.42;
        b.ind_cd_f = 0.003; // 4000·0.003 ≈ 12.000…002 → "12"
        b.radiator_cd = 0.021;
        b.oil_radiator_cd = 0.017;
        let mut wing = FmParts::default();
        wing.name = Some("机翼 无襟翼".to_string());
        wing.cd_min = 0.0285; // %.3f HALF_UP → "0.029"
        wing.cl0 = 0.05;
        wing.aoa_crit_low = -14.4;
        wing.aoa_crit_high = 18.6;
        wing.cl_crit_low = -1.15;
        wing.cl_crit_high = 1.55;
        b.no_flaps_wing = Some(wing);
        b.full_flaps_wing = Some(FmParts {
            name: Some("机翼 全襟翼".to_string()),
            cd_min: 0.0331,
            cl0: 0.12,
            aoa_crit_low: -13.1,
            aoa_crit_high: 20.2,
            cl_crit_low: -1.35,
            cl_crit_high: 1.85,
            ..Default::default()
        });
        b.fuselage = Some(FmParts {
            name: Some("机身".to_string()),
            cd_min: 0.0151,
            cl0: 0.02,
            aoa_crit_low: -27.9,
            aoa_crit_high: 27.9,
            cl_crit_low: -0.41,
            cl_crit_high: 0.49,
            ..Default::default()
        });
        b.fin = Some(FmParts {
            name: Some("垂尾".to_string()),
            cd_min: 0.0081,
            cl0: 0.0,
            aoa_crit_low: -16.2,
            aoa_crit_high: 16.2,
            cl_crit_low: -0.62,
            cl_crit_high: 0.62,
            ..Default::default()
        });
        b.stab = Some(FmParts {
            name: Some("平尾".to_string()),
            cd_min: 0.0062,
            cl0: -0.06,
            aoa_crit_low: -15.5,
            aoa_crit_high: 15.5,
            cl_crit_low: -0.55,
            cl_crit_high: 0.55,
            ..Default::default()
        });
        b
    }

    /// key 全表取值 + 计算行公式 + 精度/preview 元数据
    /// — 原 generate_lines_full_field_list 的单字段化
    #[test]
    fn fm_num_values_and_meta() {
        let b = full_fmdata();
        // 直读字段
        assert_eq!(fm_num_value(&b, "weight.empty"), Some(3050.0));
        assert_eq!(fm_num_value(&b, "wing.oswalds"), Some(0.75));
        // 计算行 (原模板内联公式)
        assert_eq!(fm_num_value(&b, "speed.crit"), Some(828.0));
        assert_eq!(fm_num_value(&b, "nitro.minutes"), Some(1.0));
        assert_eq!(fm_num_value(&b, "lift350.no_flap"), Some(5.0));
        assert_eq!(fm_num_value(&b, "drag.cd_accel"), Some(0.42 / 4.0));
        assert_eq!(fm_num_value(&b, "drag.ind_accel"), Some(4000.0 * 0.003));
        // 襟翼档 (档数 2 内)
        assert_eq!(fm_num_value(&b, "flap_limit.0.speed"), Some(640.0));
        assert!((fm_num_value(&b, "flap_limit.1.pct").unwrap() - 95.0).abs() < 1e-6);
        // 部件段
        assert_eq!(fm_num_value(&b, "parts.stab.cd_min"), Some(0.0062));
        assert_eq!(fm_num_value(&b, "parts.fin.cl_high"), Some(0.62));
        // 元数据 (精度对位原 printf 量词; preview 示例值)
        let (_, _, p, v) = fm_num_meta("weight.maxfuel").unwrap();
        assert_eq!((p, v.as_str()), (1, "780.5"));
        let (l, _, p, v) = fm_num_meta("flap_limit.1.speed").unwrap();
        assert_eq!((l.as_str(), p, v.as_str()), ("襟翼1限速", 0, "520"));
        let (l, _, p, _) = fm_num_meta("parts.fuselage.cd_min").unwrap();
        assert_eq!((l.as_str(), p), ("零升阻力系数", 3));
        // 文本行 (Lang 模板)
        assert_eq!(
            fm_text_value(&b, "fm.version").unwrap(),
            "FM文件: spitfire_mk24 - 2.35.0.9"
        );
        assert_eq!(
            fm_text_value(&b, "parts.no_flaps_wing.name").unwrap(),
            "------fm器件 机翼 无襟翼------"
        );
    }

    /// 数据条件: nitro=0 / 惯量缺 / gload raw 缺 / 襟翼档数外 / 部件缺
    /// — 原 generate_lines_nitro_gate + null 段跳过语义
    #[test]
    fn data_conditions_hide_field() {
        let mut b = full_fmdata();
        b.nitro = 0.0;
        assert_eq!(fm_num_value(&b, "nitro.amount"), None, "nitro≤0 段隐藏");
        assert_eq!(fm_num_value(&b, "nitro.minutes"), None);
        b.moment_of_inertia = None;
        assert_eq!(fm_num_value(&b, "inertia.p"), None);
        b.raw_wing_crit_overload = None;
        assert_eq!(fm_num_value(&b, "gload.full_pos"), None);
        // 襟翼档数外 (num=2, 档 2..5 归零)
        assert_eq!(fm_num_value(&b, "flap_limit.3.speed"), None);
        // 部件缺 → 段头与字段一并消失
        b.fin = None;
        assert_eq!(fm_num_value(&b, "parts.fin.cd_min"), None);
        assert_eq!(fm_text_value(&b, "parts.fin.name"), None);
        // null 字段打 "null" (Java Formatter 行为)
        let z = FmData::default();
        assert_eq!(
            fm_text_value(&z, "fm.version").unwrap(),
            "FM文件: null - null"
        );
    }

    /// sidecar 节拍: preview 冻结 / live 无 FM 归零 / reset 复原
    /// — 原 fm_overlay_toggle_visibility_gating + reset_preview 的组件面
    #[test]
    fn sidecar_tick_preview_and_reset() {
        let fm = FMManager::new(std::sync::Arc::new(EventBus::new()));
        let mut w = make(serde_json::json!({ "key": "wing.aspect_ratio" }));
        // 构造期 = preview 静态值
        let f = w.as_ref().as_any().downcast_ref::<FmFieldWidget>().unwrap();
        assert_eq!(f.text(), "6.0");
        // preview: frame None → 冻结
        let mut sctx = SidecarCtx {
            now_ms: 1000,
            page_id: "fm-list-default",
            fm: &fm,
            fm_field_config: &|_| None,
            display_fm_key: 0,
            frame: None,
            is_jet: false,
            toggle_pulse: false,
            game_mode_pulse: false,
            fm_changed: None,
        };
        w.sidecar().unwrap().tick(&mut sctx);
        let f = w.as_ref().as_any().downcast_ref::<FmFieldWidget>().unwrap();
        assert!(f.shown() && f.text() == "6.0", "preview 冻结静态值");
        // live: 无 FM → 行归零
        let mut sctx = SidecarCtx {
            now_ms: 1000,
            page_id: "fm-list-default",
            fm: &fm,
            fm_field_config: &|_| None,
            display_fm_key: 0,
            frame: Some(&NoFrame {}),
            is_jet: false,
            toggle_pulse: false,
            game_mode_pulse: false,
            fm_changed: None,
        };
        w.sidecar().unwrap().tick(&mut sctx);
        let f = w.as_ref().as_any().downcast_ref::<FmFieldWidget>().unwrap();
        assert!(!f.shown(), "live 无 FM 行消失");
        // reset_preview 复原
        w.reset_preview();
        let f = w.as_ref().as_any().downcast_ref::<FmFieldWidget>().unwrap();
        assert!(f.shown() && f.text() == "6.0", "reset 回 preview 静态");
    }

    /// 段开关: switchKey 关 → 行归零; parseBoolean 语义
    /// (缺键/空串默认启用, 仅忽略大小写 "true" 为真)
    #[test]
    fn switch_key_gates_row() {
        // isFieldEnabled 对位 (原 generateLines 开关语义)
        assert!(field_enabled(&|_| None, "showWeight"), "缺键默认启用");
        assert!(field_enabled(&|_| Some(String::new()), "showWeight"), "空串默认启用");
        assert!(field_enabled(&|_| Some("TRUE".to_string()), "showWeight"), "忽略大小写");
        assert!(!field_enabled(&|_| Some("yes".to_string()), "showWeight"), "非 true 关");
        // 组件层: false → 行归零
        let fm = FMManager::new(std::sync::Arc::new(EventBus::new()));
        let mut w = make(serde_json::json!({
            "key": "weight.empty", "switchKey": "showWeight"
        }));
        let mut sctx = SidecarCtx {
            now_ms: 1000,
            page_id: "fm-list-default",
            fm: &fm,
            fm_field_config: &|k: &str| (k == "showWeight").then(|| "false".to_string()),
            display_fm_key: 0,
            frame: Some(&NoFrame {}),
            is_jet: false,
            toggle_pulse: false,
            game_mode_pulse: false,
            fm_changed: None,
        };
        w.sidecar().unwrap().tick(&mut sctx);
        let f = w.as_ref().as_any().downcast_ref::<FmFieldWidget>().unwrap();
        assert!(!f.shown(), "showWeight=false → 行归零");
    }

    /// 空 FormulaView 桩 (live 帧 presence 标记)
    struct NoFrame {}
    impl vm_core::formula::registry::FormulaView for NoFrame {
        fn var_value(&self, _name: &str) -> Option<f64> {
            None
        }
    }
}
