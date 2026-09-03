//! FmUnpackedDataOverlay (ui/overlay/FMUnpackedDataOverlay.java) — FM 调试列表。
//! 重构波2 自 overlays_field2.rs 拆出 (后半)。
//!
//! FM 调试列表: BaseOverlay 斑马纹基座 + blkx 字段直读清单 (D4 砍反射段后的
//! 等价实现)。UIStateBus 订阅 (FM_OVERLAY_TOGGLE/FM_CHANGED) 对应
//! [`FmUnpackedDataOverlay::toggle`]/[`FmUnpackedDataOverlay::reload_fm_data`],
//! 由组装层的事件循环驱动 (vm-app 渲染线程: 总线订阅转 channel → 循环内消费);
//! dispose 的退订由所有权 Drop 根治, 无需显式方法。
//!
//! P5 组装契约三点已销号 (原 "host::OverlaySpec 不可表达" 豁口):
//! (a) 动态窗口高 — host `resize_entry` 基建 + [`FmUnpackedFeed::pump`] 在
//! tick 后按 `base.height` 变化落 resize (对位 Java adjustPosition 的 setSize
//! 副作用); (b) 逐条目可见性 — host `set_entry_visible` (per-entry, 幂等) +
//! pump 每 tick 落 `base.window_visible`; (c) spec 工厂
//! [`fm_unpacked_data_overlay_spec`] (flight_info/field_overlays 先例形态)。
//!
//! 对拍备案 (审查 W3): rustcmp 套件覆盖 FlightInfo/gauges/MiniHUD; FMUnpackedData
//! (ZebraList 首个生产消费者) 的渲染证据 = 单测级 基线 色/几何 (WebLaF 离屏
//! 实测值, overlay_list tests) + 本模块墨迹断言; rustcmp 场景面扩充随渲染对拍
//! 工具批另行安排。
//!
//! printf 引擎: 本地 FmtArg/java_string_format/java_format_f 副本已收割至
//! vm_core::base::format (重构波13, 历史基线 对拍等价)。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::render::font::LoadedFont;
use crate::render::palette::aa;

use crate::overlays::list::BaseListOverlay;
use crate::overlays::spec_common::{keyed_spec, FontSlot};
use crate::platform::host::{OverlayHost, OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::render::canvas::PixCanvas;
use vm_core::base::format::{java_string_format, FmtArg};
use vm_core::base::physics_constants::g;
use vm_core::config::config_api::ConfigProvider;
use vm_core::fm::data::{FmData, FmParts};
use vm_core::fm::FMManager;
use vm_core::lang::Lang;

// Java String.format printf 引擎与 FmtArg 收敛于 vm_core::base::format
// (重构波13 收割本地副本, Lang 模板域: %s / %d / %.0f~%.9f / %%)

// ---------------------------------------------------------------------------
// FmUnpackedDataOverlay (ui/overlay/FMUnpackedDataOverlay.java)
// ---------------------------------------------------------------------------

/// FM 调试列表 overlay。组合 BaseListOverlay
/// (Java extends BaseOverlay — §1 禁强行继承, 公共行为已上提基座):
/// 自管可见性 (游戏模式热键切换) + blkx 字段直读清单。
///
/// Java 经 FMDataAdapter 持 volatile blkx; Rust 直持 `Arc<FmData>`
/// 快照 (单写者(事件循环) + tick 前快照承接 volatile 赋值语义)。
pub struct FmUnpackedDataOverlay {
    /// BaseOverlay 基座 (run 循环状态机: 脏检查/高度自适应/可见门控)
    pub base: BaseListOverlay,
    /// 自管可见态 (游戏模式热键切换)
    pub visible: bool,
    /// FMDataAdapter.getBlkx() 的等价持有 (None = 未加载 → 占位清单)
    fmdata: Option<Arc<FmData>>,
    /// Java config = c.getConfigProvider() (None ↔ Java null 容忍)
    config: Option<Arc<dyn ConfigProvider>>,
}

impl FmUnpackedDataOverlay {
    /// 构造 + BaseOverlay.init 几何 (super() 与 super.init 的
    /// 几何段合一; 行高度量 setup_font 由 init/reinit 时调用方补)。
    pub fn new(logical_height: i32, dpi_scale: f64, default_fontsize: i32) -> Self {
        FmUnpackedDataOverlay {
            base: BaseListOverlay::new(logical_height, dpi_scale, default_fontsize),
            visible: true,
            fmdata: None,
            config: None,
        }
    }

    /// 游戏模式 init: config 注入 + 隐藏起步 + 表头谓词 +
    /// BaseOverlay 数据供给挂接 (tick 内联)。UIStateBus 订阅 (toggle/FM_CHANGED)
    /// 归组装层事件循环, 对应 [`Self::toggle`]/[`Self::reload_fm_data`]。
    pub fn init(&mut self, config: Option<Arc<dyn ConfigProvider>>, font: &LoadedFont) {
        self.config = config;
        // Java this.isPreview = false — 继承自 BaseOverlay 的单一字段,
        // 对应 base.is_preview (run 门控的唯一读取方)
        self.base.is_preview = false;

        // 游戏模式: 初始隐藏
        self.visible = false;
        self.base.setup_font(font);

        // 表头谓词 (FM 部件表头以 "------fm器件" 开头)
        self.base.set_header_matcher(Box::new(|line| {
            line.starts_with("FM文件") || line.starts_with("------fm器件")
        }));
    }

    /// 预览模式 initPreview: 恒可见 + 同表头谓词。
    pub fn init_preview(&mut self, config: Option<Arc<dyn ConfigProvider>>, font: &LoadedFont) {
        self.config = config;
        // Java this.isPreview = true (BaseOverlay 单一字段, 同上)
        self.base.is_preview = true;

        // 预览模式: 恒可见
        self.visible = true;
        self.base.setup_font(font);

        self.base.set_header_matcher(Box::new(|line| {
            line.starts_with("FM文件") || line.starts_with("------fm器件")
        }));
    }

    /// FM_CHANGED handler 的 reloadFMData: 句柄换 blkx 快照。
    /// 非 READY 句柄 blkx=null → None → 占位 "[No Data Loaded]" (null 容忍)。
    /// 数据刷新由下一 tick 周期完成 (Java 注释)。
    /// Java 的 `payload instanceof FMHandle` 过滤由组装层承担 —
    /// P5 事件路由时非 FMHandle 载荷应保留旧 blkx 不调用本方法。
    pub fn reload_fm_data(&mut self, fmdata: Option<Arc<FmData>>) {
        self.fmdata = fmdata;
        // 数据在下一 run() 周期刷新
    }

    /// reinitConfig: adapter 直读 FMManager.current() 换新
    /// (调用方传入 current().blkx; 非 READY 句柄 blkx 为 null → None,
    /// setBlkx(null) 清空 → 占位容忍) + setupFont。
    pub fn reinit_config(&mut self, current_fm: Option<Arc<FmData>>, font: &LoadedFont) {
        self.fmdata = current_fm;
        // 字体/显示设置归 BaseOverlay
        self.base.setup_font(font);
    }

    /// FM_OVERLAY_TOGGLE handler: 翻转自管可见性。
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// CloseAll 后回 preview 会话形态 (Java closeAll = 实例销毁 + refreshPreview
    /// 工厂新建 initPreview 实例): visible=true / is_preview=true / lastData=null
    /// (预览窗回到当前 blkx 的清单态)。blkx/config 保留 — Java Controller 预览
    /// 工厂构造时同样 setBlkx(current), 等值; 表头谓词与 init_preview 设定相同
    /// 免重设。PORT(几何自纠, 审查 W2): base.height 保留 live 会话 adjustPosition
    /// 值不复位 — Java 工厂新实例回 init 高度, Rust 由首个 preview tick (pump 无
    /// 会话门控) 的 dirty → adjust_position 一步收敛到行数高度, 终态与 Java 一致。
    /// (ControlSurfaces::reset_preview / FlightInfo::reset_preview_rows 同族)
    pub fn reset_preview(&mut self) {
        self.visible = true;
        self.base.is_preview = true;
        self.base.clear_last_data();
    }

    /// isVisibleNow 覆写
    pub fn is_visible_now(&self) -> bool {
        self.visible
    }

    /// BaseOverlay.run() 单轮 (经 overlay_list 基座): 先同步 isVisibleNow
    /// 门控位, 再以当前 blkx/config 快照生成行清单。
    pub fn tick(&mut self) -> bool {
        self.base.visible_now = self.visible;
        let fmdata = self.fmdata.clone();
        let config = self.config.clone();
        self.base
            .tick(move || Some(generate_lines(fmdata.as_deref(), config.as_deref())))
    }

    /// generateLines — 独立函数便于直测。
    pub fn generate_lines(&self) -> Vec<String> {
        generate_lines(self.fmdata.as_deref(), self.config.as_deref())
    }

    /// updateUI 渲染段委托基座
    pub fn render(&mut self, cv: &mut PixCanvas, font: &LoadedFont, aa: bool) {
        self.base.render(cv, font, aa);
    }
}

/// generateLines: 按 ui_layout.cfg 开关过滤的 blkx 字段清单。
/// Lang 模板取 init_lang() 快照 (Java 读全局静态字段, 值同源 cur.properties)。
/// 15+ 个开关段改表驱动 (重构波15): 键-生成器对见 [`FM_FIELD_TABLE`],
/// 各段内条件 (nitro>0 / Option 守卫) 原样留在生成器内。
pub(crate) fn generate_lines(
    fmdata: Option<&FmData>,
    config: Option<&dyn ConfigProvider>,
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
    let ctx = LineCtx {
        lang: &lang,
        fmdata,
    };

    // ==================== FM Version (always shown) ====================
    // Java %s 收 null 字段打印 "null" (Formatter 行为), Option 展开对齐
    let fm_version = java_string_format(
        ctx.lang.b_fm_version,
        &[
            FmtArg::S(ctx.fmdata.read_file_name.as_deref().unwrap_or("null")),
            FmtArg::S(ctx.fmdata.version.as_deref().unwrap_or("null")),
        ],
    );
    add_lines(&mut lines, &fm_version);

    // ==================== 开关过滤段 (键序 = Java 块序, 保真) ====================
    for (key, gen) in FM_FIELD_TABLE {
        if is_field_enabled(config, key) {
            gen(&mut lines, &ctx);
        }
    }

    // If no fields are enabled or all filtered out, show a placeholder
    // fmVersion 恒入列 使本分支在 Java 亦不可达, 保真保留
    if lines.is_empty() {
        lines.push("FM Data Preview".to_string());
        lines.push("[No Fields Enabled]".to_string());
    }

    lines
}

/// 行生成器的共享入参 (闭包捕获面: Lang 快照 + FM 数据)
struct LineCtx<'a> {
    lang: &'a Lang,
    fmdata: &'a FmData,
}

/// 单段行生成器: 把格式化结果拆行入列 (各段一个 fn, 表驱动入口)
type LineFn = fn(&mut Vec<String>, &LineCtx);

/// 开关键 → 行段 的静态表 (顺序 = Java generateLines 的 if 块顺序)
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

// ---- 表驱动各段 (段内逻辑/PORT 注与原 if 块逐字一致) ----

/// Weight (空重/满油重)
fn add_weight(lines: &mut Vec<String>, ctx: &LineCtx) {
    let weight = java_string_format(
        ctx.lang.b_weight,
        &[
            FmtArg::F(ctx.fmdata.emptyweight),
            FmtArg::F(ctx.fmdata.maxfuelweight),
        ],
    );
    add_lines(lines, &weight);
}

/// Critical Speed (临界速度/VNE)
fn add_crit_speed(lines: &mut Vec<String>, ctx: &LineCtx) {
    let crit_speed = java_string_format(
        ctx.lang.b_crit_speed,
        &[
            FmtArg::F(ctx.fmdata.critical_speed * 3.6),
            FmtArg::F(ctx.fmdata.vne),
        ],
    );
    add_lines(lines, &crit_speed);
}

/// G-Load Limits (combined full/half fuel)
fn add_g_load_limits(lines: &mut Vec<String>, ctx: &LineCtx) {
    if let Some(raw) = ctx.fmdata.raw_wing_crit_overload {
        // 与 getMaxAllowGloadForWeight 同式内联 (Java 源如此, 不收敛去重)
        let full_neg = 1.2 * (2.0 * raw[0] / (g * ctx.fmdata.grossweight) + 1.0);
        let full_pos = 1.2 * (2.0 * raw[1] / (g * ctx.fmdata.grossweight) - 1.0);
        let half_neg = 1.2 * (2.0 * raw[0] / (g * ctx.fmdata.halfweight) + 1.0);
        let half_pos = 1.2 * (2.0 * raw[1] / (g * ctx.fmdata.halfweight) - 1.0);
        let load_factor = java_string_format(
            ctx.lang.b_allow_load_factor,
            &[
                FmtArg::F(full_neg),
                FmtArg::F(full_pos),
                FmtArg::F(half_neg),
                FmtArg::F(half_pos),
            ],
        );
        add_lines(lines, &load_factor);
    }
}

/// Flap Speed Limits (襟翼段限速)
/// Java AIOOBE (num > 6) ↔ Rust 索引 panic 同构。
/// 线程模型差异: Java 仅杀死本 overlay 的 run 轮询线程, Rust tick/draw 在
/// 唯一主循环上 — P5 组装必须对逐 overlay tick/render 包 catch_unwind
///, 本 panic 与 java_string_format 错配 panic 均属此契约
fn add_flap_limits(lines: &mut Vec<String>, ctx: &LineCtx) {
    if let Some(table) = ctx.fmdata.flaps_destruction_ind_speed {
        for i in 0..ctx.fmdata.flaps_destruction_num {
            let flap_limit = java_string_format(
                ctx.lang.b_flap_restrict,
                &[
                    FmtArg::D(i),
                    FmtArg::F(table[i as usize][0] * 100.0),
                    FmtArg::F(table[i as usize][1]),
                ],
            );
            add_lines(lines, &flap_limit);
        }
    }
}

/// Control Surface Effectiveness (combined)
fn add_control_effectiveness(lines: &mut Vec<String>, ctx: &LineCtx) {
    let eff_speed = java_string_format(
        ctx.lang.b_eff_speed_and_power_loss,
        &[
            FmtArg::F(ctx.fmdata.elav_eff),
            FmtArg::F(ctx.fmdata.aileron_eff),
            FmtArg::F(ctx.fmdata.rudder_eff),
            FmtArg::F(ctx.fmdata.elav_power_loss),
            FmtArg::F(ctx.fmdata.aileron_power_loss),
            FmtArg::F(ctx.fmdata.rudder_power_loss),
        ],
    );
    add_lines(lines, &eff_speed);
}

/// Nitro (only if present) — 表外附加条件 nitro > 0 留段内
fn add_nitro(lines: &mut Vec<String>, ctx: &LineCtx) {
    if ctx.fmdata.nitro > 0.0 {
        let nitro = java_string_format(
            ctx.lang.b_nitro,
            &[
                FmtArg::F(ctx.fmdata.nitro),
                FmtArg::F(ctx.fmdata.nitro / (ctx.fmdata.nitro_decr * 60.0)),
            ],
        );
        add_lines(lines, &nitro);
    }
}

/// Heat Recovery (发动机平均恢复率)
fn add_heat_recovery(lines: &mut Vec<String>, ctx: &LineCtx) {
    let heat_recovery = java_string_format(
        ctx.lang.b_average_heat_recovery,
        &[FmtArg::F(ctx.fmdata.avg_eng_recovery_rate)],
    );
    add_lines(lines, &heat_recovery);
}

/// Max Lift Load (350 段最大升力系数)
fn add_max_lift_load(lines: &mut Vec<String>, ctx: &LineCtx) {
    let max_lift_load = java_string_format(
        ctx.lang.b_max_lift_load350,
        &[
            FmtArg::F((ctx.fmdata.no_flap_wll + 1.0) / 2.0),
            FmtArg::F((ctx.fmdata.full_flap_wll + 1.0) / 2.0),
        ],
    );
    add_lines(lines, &max_lift_load);
}

/// Inertia (惯量, 三分量齐才显示)
fn add_inertia(lines: &mut Vec<String>, ctx: &LineCtx) {
    if let Some(m) = ctx.fmdata.moment_of_inertia {
        if m.len() >= 3 {
            let inertia = java_string_format(
                ctx.lang.b_inertia,
                &[FmtArg::F(m[2]), FmtArg::F(m[0]), FmtArg::F(m[1])],
            );
            add_lines(lines, &inertia);
        }
    }
}

/// Lift Parameters (升力参数族)
fn add_lift(lines: &mut Vec<String>, ctx: &LineCtx) {
    let lift = java_string_format(
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
    );
    add_lines(lines, &lift);
}

/// Drag Parameters (阻力参数族)
fn add_drag(lines: &mut Vec<String>, ctx: &LineCtx) {
    let drag = java_string_format(
        ctx.lang.b_drag,
        &[
            FmtArg::F(ctx.fmdata.cd_s),
            FmtArg::F(ctx.fmdata.cd_s / (ctx.fmdata.halfweight / 1000.0)),
            FmtArg::F(ctx.fmdata.ind_cd_f),
            FmtArg::F(ctx.fmdata.halfweight * ctx.fmdata.ind_cd_f),
            FmtArg::F(ctx.fmdata.radiator_cd),
            FmtArg::F(ctx.fmdata.oil_radiator_cd),
        ],
    );
    add_lines(lines, &drag);
}

/// FM Parts: 无襟翼机翼段
fn add_no_flaps_wing(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.no_flaps_wing.as_ref());
}

/// FM Parts: 满襟翼机翼段
fn add_full_flaps_wing(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.full_flaps_wing.as_ref());
}

/// FM Parts: 机身段
fn add_fuselage(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.fuselage.as_ref());
}

/// FM Parts: 垂尾段
fn add_fin(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.fin.as_ref());
}

/// FM Parts: 平尾段
fn add_stab(lines: &mut Vec<String>, ctx: &LineCtx) {
    add_fm_parts(lines, ctx.lang, ctx.fmdata.stab.as_ref());
}

/// addFmParts: 表头 + 4 数据行 (null 部件整段跳过)。
fn add_fm_parts(lines: &mut Vec<String>, lang: &Lang, p: Option<&FmParts>) {
    let p = match p {
        None => return,
        Some(p) => p,
    };
    add_lines(
        lines,
        &java_string_format(
            lang.b_fm_parts,
            &[FmtArg::S(p.name.as_deref().unwrap_or("null"))],
        ),
    );
    add_lines(
        lines,
        &java_string_format(lang.b_cd_min, &[FmtArg::F(p.cd_min)]),
    );
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

/// addLines: 按 \n 拆行, 逐行 trim, 跳过空行。
/// Java String.trim 只剥 ≤ U+0020 的字符, Rust `str::trim` 会
/// 多剥 U+3000 等全角空白 — 用 trim_matches 精确复刻 Java 语义。
pub(crate) fn add_lines(lines: &mut Vec<String>, formatted: &str) {
    for line in formatted.split('\n') {
        let trimmed = line.trim_matches(|c: char| c <= '\u{20}');
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }
}

/// isFieldEnabled: config 缺失/键空 → 默认启用;
/// 否则 Boolean.parseBoolean (仅忽略大小写的 "true" 为真)。
fn is_field_enabled(config: Option<&dyn ConfigProvider>, field_key: &str) -> bool {
    match config {
        None => true,
        Some(c) => match c.get_config(field_key) {
            None => true,
            Some(v) if v.is_empty() => true,
            Some(v) => v.eq_ignore_ascii_case("true"),
        },
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 挂载 (Java Controller registerWithPreview("enableFMPrint"))
// ---------------------------------------------------------------------------

/// FM拆包数据共享句柄 (flight_info/control_surfaces 先例: render 闭包与
/// 事件循环共享 state; Rc 恒留渲染线程)
pub type FmUnpackedDataHandle = Rc<RefCell<FmUnpackedDataOverlay>>;

/// FM拆包数据 OverlaySpec + live 句柄 (Java Controller 注册键
/// enableFMPrint, previewEnabled=true)。
///
/// - `logical_height` — Application.logicalHeight 快照 (Env.dpi 探测; init 几何的
///   scaleFactor 与 adjustPosition 的钳制上限来源, 屏幕常量不入 ReinitParams);
/// - `config` — generateLines 逐 tick 读的 show* 开关面 (Java 每轮直读
///   ConfigProvider; Rust 配置树 !Send, 组装层注入快照适配器, CONFIG_CHANGED
///   刷新 — ActivationCache 同款"最后写胜出"等价);
/// - `fm` — reinit 闭包的 blkx 直读源 (Java reinitConfig 的
///   `FMManager.getInstance().current()`)。
///
/// 初始态 = initPreview 形态 (恒可见 + 空数据: 注册期 = Java 无实例形态 —
/// LinkedHashMap 条目仅配置记录)。数据装载有两条面 (审查 B2-2 修正, 原注释
/// "Java 预览实例无 run 线程" 为假前提): (a) 预览实例化时 Controller 的
/// previewInitializer 先 setBlkx(current) 再 initPreview
/// — Rust 对位 = refresh_preview 冷激活的 reinit 闭包直读 current; (b) run() 线程
/// **预览同样在跑** (needsThread=true, OverlayManager.refreshPreview 也
/// new Thread(instance).start()) — 每 200ms generateLines → 数据/高度自适应生效,
/// Rust 对位 = FmUnpackedFeed::pump 不做会话门控 (渲染线程循环调用点)。
/// 游戏形态 (is_preview=false + 隐藏起步) 由组装层在 OpenAllOverlays 处置位 —
/// 单实例形态下 ControlSurfaces has_service 的同款会话翻转模式。
///
/// PORT(尺寸): 初始 spec 尺寸 = init 几何的 width × defaultFontsize·72 (Java
/// Window 首帧尺寸; 高度随后被 adjustPosition 按行数接管 — FmUnpackedFeed)。
/// PORT(字体): Java setupFont 的 fontName (cfg "FM拆包数据" 组) 为 Swing 逻辑
/// 字体族名; Rust 字体面固定 sarasa regular 文件 (FontTriple/loadFontConfig 各
/// overlay 同款先例, cfg 缺省 "Sarasa Mono SC" 时零偏差), PLAIN 14+fontSizeAdd。
pub fn fm_unpacked_data_overlay_spec(
    fonts_dir: &std::path::Path,
    logical_height: i32,
    params: &Rc<RefCell<ReinitParams>>,
    config: Option<Arc<dyn ConfigProvider>>,
    fm: &Arc<FMManager>,
) -> Result<(FmUnpackedDataHandle, OverlaySpec), String> {
    let (font_add, dpi_scale) = {
        let p = params.borrow();
        (p.fm.font_add, p.dpi_scale)
    };
    // Application.defaultFontsize = 12 (Lang defaultFontSize)
    let mut ov = FmUnpackedDataOverlay::new(logical_height, dpi_scale, 12);
    let regular_path = fonts_dir.join("sarasa-mono-sc-regular.ttf");
    let font = FontSlot::new("FMUnpackedData", &regular_path, 14 + font_add)?;
    ov.init_preview(config, &font.get());
    let (w, h) = (ov.base.width, ov.base.height);
    let handle: FmUnpackedDataHandle = Rc::new(RefCell::new(ov));
    let render_handle = Rc::clone(&handle);
    let render_font = font.clone();
    // reinit 闭包 (Java reinitConfig): setBlkx(current) + setupFont。
    // PORT(返回 None): Java reinitConfig 无 setBounds — 高度由下次数据变更的
    // adjustPosition 接管 (行高随新字体变化, 数据 dirty 时自纠); 此处仅清指纹
    let reinit_handle = Rc::clone(&handle);
    let reinit_font = font;
    let reinit_params = Rc::clone(params);
    let reinit_fm = Arc::clone(fm);
    let reinit_regular = regular_path;
    let reinit: ReinitFn = Box::new(move || {
        let fa = reinit_params.borrow().fm.font_add;
        if !reinit_font.reload(&reinit_regular, 14 + fa) {
            return None;
        }
        // P3: 直读 FMManager 句柄 (blkx None → 清空 → 占位容忍)
        let fmdata = reinit_fm.current().fmdata.clone().map(Arc::new);
        let font = reinit_font.get();
        reinit_handle.borrow_mut().reinit_config(fmdata, &font);
        None
    });
    Ok((
        handle,
        keyed_spec(
            "enableFMPrint",
            w,
            h,
            Box::new(move |cv: &mut PixCanvas| {
                let font = render_font.get();
                render_handle.borrow_mut().render(cv, &font, aa());
            }),
            Some(reinit),
        ),
    ))
}

/// FM拆包数据的组装面 tick 泵 — Java BaseOverlay.run() 线程循环 (while(doit)+
/// sleep(200)) 的单线程驱动侧: 200ms 节流 (getRefreshInterval)
/// → tick 单轮 (可见门控/取数/脏检查/高度自适应) → `base.window_visible` 落
/// per-entry set_visible → 高度变化落 resize_entry (adjustPosition 的 setSize
/// 副作用, 契约 (a)/(b) 接线)。
///
/// PORT(会话域, 审查 B2-2): Java needsThread=true — 游戏实例 (OverlayEntry.open)
/// 与**预览实例** (refreshPreview) 都起 run 线程, 两会话均
/// 200ms 轮询装载; 调用方 (渲染线程循环) 不做 preview 门控, 仅条目未激活
/// (host 槽位空 = Java 无实例) 时跳过。
///
/// PORT(panic 边界): tick 内 generateLines 的保真 panic 点 (flap AIOOBE /
/// java_string_format 错配, 见 generate_lines 的 PORT 注) 由本泵 catch_unwind
/// 兜住; panic 后置 doit=false = Java "异常
/// 杀死本 overlay 的 run 线程" 的冻结形态对位 (后续 tick 短路, 应用存活)。
pub struct FmUnpackedFeed {
    /// 节流基准 (Java run 循环的 sleep(200) 节拍; 0 = 首轮放行)
    last_ms: i64,
}

impl Default for FmUnpackedFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl FmUnpackedFeed {
    pub fn new() -> Self {
        FmUnpackedFeed { last_ms: 0 }
    }

    /// 单轮驱动。`id` = host 注册键 ("enableFMPrint"), `now_ms` 由调用方注入
    /// (System.currentTimeMillis, 测试可假时钟)。
    pub fn pump(
        &mut self,
        host: &mut OverlayHost,
        id: &str,
        handle: &FmUnpackedDataHandle,
        now_ms: i64,
    ) {
        // getRefreshInterval() = 200ms (本组件未覆写;
        // 读 base 字段 = 单一真相源)
        let interval_ms = handle.borrow().base.refresh_interval_ms as i64;
        if now_ms.saturating_sub(self.last_ms) < interval_ms {
            return;
        }
        self.last_ms = now_ms;
        let before = {
            let fm = handle.borrow();
            (fm.base.width, fm.base.height)
        };
        let ticked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle.borrow_mut().tick();
        }));
        if ticked.is_err() {
            vm_core::base::logger::error(
                "FMUnpackedData",
                "run 轮 panic 已吞 (畸形 FM 字段, 对位 Java 杀 run 线程), 本 overlay 冻结",
            );
            handle.borrow_mut().base.stop(); // doit=false: 后续 tick 短路
            return;
        }
        let fm = handle.borrow();
        // run() 双分支的 setVisible 落地 (set_entry_visible 幂等)
        host.set_entry_visible(id, fm.base.window_visible);
        // adjustPosition 的 setSize 副作用: 高度 (或宽) 变化才落 resize
        // (未变时避免清指纹引发无谓 present — Java 亦仅在变化时 setSize)
        let after = (fm.base.width, fm.base.height);
        if after != before {
            let _ = host.resize_entry(id, after.0, after.1);
        }
    }
}
