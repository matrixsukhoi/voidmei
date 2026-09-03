//! 对应 Java: `src/ui/overlay/logic/HUDCalculator.java` (B 类)。
//!
//! Pure logic calculator for HUD Data.
//! Extracts raw data from FlightDataEvent (Data/State) and performs business
//! logic calculations.
//!
//! # 三个解耦点 (CLASSIFY.md §23-3/6 裁决)
//!
//! 1. **MinimalHUDContext 字体参数**: Java `calculate(..., MinimalHUDContext ctx)`
//!    的 `ctx` 形参在方法体内**从未被读取** (两个调用点一处传 null、
//!    一处传实句柄, 行为无差) —— Rust 签名直接砍除该参数,
//!    字体参数由 C 类组件层 (MiniHUD 波次) 自持, 本纯逻辑层不引入 C 类类型。
//! 2. **Application 静态颜色 6 处 → 参数注入**: Java 读
//!    `Application.colorWarning/colorNum/colorUnit` (LIFETIMES §1.2 配置驱动可变
//!    全局, ConfigurationService 重写)。Rust 侧以 [`HudColors`]
//!    参数注入, 调用方 (未来 Service 波次) 从 Theme/ConfigStore 快照取值;
//!    `java.awt.Color.RED/WHITE` (throttleColor 两分支) 是 JDK 常量, 原样落地
//!    为模块常量 [`COLOR_RED`]/[`COLOR_WHITE`]。
//! 3. **离屏 FontMetrics 度量点**: `getStringWidth` 原实现自建 1x1 离屏
//!    BufferedImage + 复用 Graphics2D (`synchronized(MEASURE_G)` 互斥 EDT 回退
//!    路径与 Service 线程, LIFETIMES §3.2)。Rust 无 AWT, 度量能力由调用方注入
//!    (vm-overlay font.rs 的 `FontHandle::measure`, 语义对齐
//!    charWidth=round(advance) 累加); 每调用栈局部化后共享测量容器与锁天然
//!    消亡 (LIFETIMES §3.2 建议 "更好是每调用栈局部分配")。
//!
//! PORT: Java `String.format` 的 `%N.Mf`/`%Nd`/`%Ns` → 私有 [`java_f`]+[`pad_width`]
//! 复刻 (非 Rust `format!` 直换): Java %f 是对**最短往返十进制**做 HALF_UP
//! (2.675→"2.68"), Rust `{:.N}` 是对精确二进制值半偶舍入 ("2.67") —— 双重分歧,
//! 全部格式串已按 Java 8 oracle (build/oracle_hud) 逐值对拍。
//! PORT: 默认 Locale 按非分组小数点处理 (zh-CN/en, 与 Application 运行域一致)。
//! PORT: NPE→panic 的降级契约在调用点: Java Service 以 catch(Exception)
//! 包裹 calculate, 失败仅 log、事件照发 (无 hudData); Rust 侧等价降级需**调用点粒度**
//! catch_unwind (§6 循环级会额外丢整轮事件发布, 降级幅度不同) — 归 vm-data Service
//! (D6) 波次落实, 过渡期警告见 calculate() 内 unwrap 处。
/// flaps 的正哨兵 (Java AIOOBE 域产物, 与缺数据哨兵 -65535 同判无效)
const FLAPS_INVALID_POS: i32 = 65535;

use crate::base::event::event_payload::EventPayload;
use crate::game_api::parser::F_INVALID; // 波21: 哨兵字面量收敛
use crate::base::format::{java_f, pad_width};
use crate::config::config_api::HUDSettings;
use crate::derived::hud_data::Builder;
use crate::derived::hud_data::HUDData;
use crate::fm::data::FmData;
use crate::formula::registry::FormulaView;
use crate::game_api::parser::{Indicators, State};

/// W7: var_value 桥取值 (NaN→0, 对齐原 getter map_or(0.0) 零值帧语义)
fn v(s: &dyn FormulaView, name: &str) -> f64 {
    s.var_value(name).unwrap_or(0.0)
}

/// `java.awt.Color.RED` = new Color(255, 0, 0) (alpha 255, Java 8 oracle)
const COLOR_RED: [u8; 4] = [255, 0, 0, 255];
/// `java.awt.Color.WHITE` = new Color(255, 255, 255) (alpha 255)
const COLOR_WHITE: [u8; 4] = [255, 255, 255, 255];

/// Application 静态颜色 6 处消费 (colorWarning/colorNum/colorUnit) 的参数注入载体。
/// 取值来源对齐 Java: ConfigurationService.getColorConfig("fontWarn"/"fontNum"/"fontUnit"),
/// 缺省即 Application.java 字段初始化器 (见 [`HudColors::application_defaults`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudColors {
    pub color_warning: [u8; 4],
    pub color_num: [u8; 4],
    pub color_unit: [u8; 4],
}

impl HudColors {
    /// Application.java 字段初始化器缺省值 (Java 侧 CONFIG 未覆盖时的初值):
    /// colorWarning=(216,33,13,100) / colorNum=(27,255,128,240) / colorUnit=(166,166,166,220)
    pub fn application_defaults() -> Self {
        HudColors {
            color_warning: [216, 33, 13, 100],
            color_num: [27, 255, 128, 240],
            color_unit: [166, 166, 166, 220],
        }
    }
}

/// HUDData 计算 (Java HUDCalculator.calculate 的 Rust 形态, W-B 事件瘦身后
/// 直参: State/Indicators/payload 由调用方从共享 guard 借引用传入, 不经事件装箱)。
/// `state.is_none() || source.is_none()` 早退对位 Java 的 null 守卫。
/// 波14 拆解: 按数据主题提取为下方四个子函数, 调用序即语句序。
#[allow(clippy::too_many_arguments)]
pub fn calculate<S: HUDSettings>(
    state: Option<&State>,
    indic: Option<&Indicators>,
    payload: &EventPayload,
    source: Option<&dyn FormulaView>,
    fmdata: Option<&FmData>,
    settings: &S,
    colors: &HudColors,
) -> HUDData {
    let mut b = Builder::default();

    let (s_state, source) = match (state, source) {
        (Some(s), Some(src)) => (s, src),
        _ => return b.build(),
    };
    let s_indic = indic;

    read_flight_data(&mut b, source, s_state, s_indic, payload, settings);
    apply_fm_warnings(&mut b, source, fmdata, s_indic, settings, colors);
    format_display_strings(&mut b, source, payload, settings, fmdata, s_indic);
    compute_speed_bar(&mut b, source, fmdata);

    b.build()
}

/// 遥测提取段: 原始飞行数据 / 姿态 / 操纵面与载具状态 / 能量与显示模式标志
/// (calculate 的 Raw Flight Data ~ 标志段)。
fn read_flight_data<S: HUDSettings>(
    b: &mut Builder,
    source: &dyn FormulaView,
    s_state: &State,
    s_indic: Option<&Indicators>,
    payload: &EventPayload,
    settings: &S,
) {
    // --- Raw Flight Data ---
    b.ias = v(source, "ias");
    b.mach = v(source, "mach");
    b.altitude = v(source, "altitude");
    b.radio_altitude = v(source, "radio_altitude");
    b.vertical_speed = v(source, "sep");
    b.heading = v(source, "compass");

    b.map_grid = payload.map_grid.clone();

    // --- Attitude ---
    let mut aviahp = 0.0;
    let mut aviar = 0.0;
    if let Some(s_indic) = s_indic {
        aviahp = s_indic.aviahorizon_pitch;
        aviar = s_indic.aviahorizon_roll;
    }

    b.pitch_valid = aviahp != F_INVALID;
    if b.pitch_valid {
        b.pitch = -aviahp;
    } else {
        b.pitch = 0.0;
    }

    if aviar != F_INVALID {
        b.roll = -aviar;
    } else {
        b.roll = 0.0;
    }

    // --- AoS / System State --- (state 由早退守卫保证在场)
    {
        if s_state.aos != F_INVALID {
            b.slip = s_state.aos;
        }

        b.throttle = s_state.throttle;
        // 波21 具名化: flaps 双哨兵 (Java 65535 正哨兵 + -65535 缺数据哨兵同判无效)
        if s_state.flaps == FLAPS_INVALID_POS || s_state.flaps == F_INVALID as i32 {
            b.flaps = 0.0;
        } else {
            b.flaps = s_state.flaps as f64;
        }
        b.gear = s_state.gear as f64;
        b.airbrake = s_state.airbrake as f64;

        if b.throttle > 100 {
            b.throttle_color = COLOR_RED;
        } else {
            b.throttle_color = COLOR_WHITE; // Or default? HUDData defaults to GREEN, but white is standard
                                            // text.
        }

        // W-E: HUDData 副本统一走公式槽 (fm_flap_allow_angle 引擎函数在公式侧,
        // 无 FM → 125 与本地计算同值)
        b.flap_allow_angle = v(source, "flap_allow_angle");

        b.aoa = s_state.aoa;
        b.g_load = s_state.ny;
    }

    // W-E: 公式路径唯一 (回退拆除 — 内置公式随出厂分发, 缺失=用户显式删除)
    b.energy_m = source.get_formula_value("energy_m").unwrap_or(0.0);

    b.is_mach_mode = settings.draw_hud_mach();
    b.is_gear_down = b.gear > 0.0;
    b.is_flaps_down = b.flaps > 0.0;
    b.is_airbrake_active = b.airbrake > 0.0;
}

/// FM 派生警告段: VNE 警告 + FM 在场时的机动指数/AoA 警告色与迎角余量
/// (无 FM 走降级分支) (calculate 的 Warning Logic 段)。
fn apply_fm_warnings<S: HUDSettings>(
    b: &mut Builder,
    source: &dyn FormulaView,
    fmdata: Option<&FmData>,
    s_indic: Option<&Indicators>,
    settings: &S,
    colors: &HudColors,
) {
    // --- Warning Logic (W-E: 公式路径唯一, 判定式在 formulas.cfg) ---
    let warn_vne = source
        .get_formula_value("warn_vne")
        .map(|v| v != 0.0)
        .unwrap_or(false);

    let valid_fmdata = fmdata.filter(|x| x.valid);
    if let Some(fmdata) = valid_fmdata {
        // W-E: 公式路径唯一 (公式式含同款零除守卫)
        b.maneuver_index = source.get_formula_value("maneuver_index").unwrap_or(0.0);

        let mut vwing = 0.0;
        // PORT: Java `blkx.isVWing` (Boolean 装箱) 在布尔上下文自动拆箱, null → NPE
        // (§1 非受检异常 → panic)。"不可达"仅对 Java 生产链成立: FMLoader.load L101
        // 两参构造 = doLoad=true → getload 必赋值 isVWing; doLoad=false 构造 (lookup
        // 用) 上为 null 会真 NPE, unwrap 忠实复刻两者。
        // getload 已落地 (reader.rs, 真机位级对拍): 生产链 READY 句柄的 is_v_wing
        // 恒 Some; None 仅剩手工构造的 doLoad=false 形态 (中央文件/旧测试) — 该
        // 形态调用本方法 = Java 对位 NPE, panic 由调用点 (vm-app feed 的整帧
        // catch_unwind) 收敛, 语义一致。
        // PORT: Java 保真 — `blkx.isVWing && sIndic != null` 的直译 (is_some 检查 +
        // unwrap 取值), 不改成 if-let 以保持与 Java 源逐行对应
        #[allow(clippy::unnecessary_unwrap)]
        if fmdata.is_v_wing.unwrap() && s_indic.is_some() {
            vwing = s_indic.unwrap().wsweep_indicator;
        }

        // AoA Warnings
        let max_available_aoa =
            fmdata.get_aoa_high_v_wing(vwing, if b.flaps > 0.0 { b.flaps as i32 } else { 0 });
        let available_aoa = max_available_aoa - b.aoa;

        if available_aoa < settings.get_aoa_warning_ratio() * max_available_aoa {
            b.aoa_color = colors.color_warning;
        } else {
            b.aoa_color = colors.color_num;
        }
        if available_aoa < settings.get_aoa_bar_warning_ratio() * max_available_aoa {
            b.aoa_bar_color = colors.color_unit;
        } else {
            b.aoa_bar_color = colors.color_num;
        }

        if max_available_aoa > 0.001 {
            b.aoa_ratio = available_aoa / max_available_aoa;
        } else {
            b.aoa_ratio = 0.0;
        }

        // W-E: 公式路径唯一
        b.warn_stall = source
            .get_formula_value("warn_stall")
            .map(|v| v != 0.0)
            .unwrap_or(false);
    } else {
        b.maneuver_index = 0.0;
        b.aoa_color = colors.color_num;
        b.aoa_bar_color = colors.color_num;
        b.aoa_ratio = b.aoa / 30.0;
    }
    b.warn_vne = warn_vne;
}

/// 字符串格式化段: 速度/高度/AoA/能量/SEP/机动状态/构型 (襟翼-可变翼-减速板-
/// 起落架) 显示串族 (calculate 的 Strings Formatting 段)。
#[allow(clippy::too_many_arguments)]
fn format_display_strings<S: HUDSettings>(
    b: &mut Builder,
    source: &dyn FormulaView,
    payload: &EventPayload,
    settings: &S,
    fmdata: Option<&FmData>,
    s_indic: Option<&Indicators>,
) {
    // Warnings
    let radio_alt_valid = v(source, "radio_altitude_valid") != 0.0;
    let always_radar = settings.always_show_radar_altitude();

    // Low altitude warning flag (W-E: 公式路径唯一)
    b.warn_altitude = source
        .get_formula_value("warn_altitude")
        .map(|v| v != 0.0)
        .unwrap_or(false);

    // --- Strings Formatting (using Data) ---
    if b.is_mach_mode {
        b.speed_str = format!("M{}", pad_width(java_f(b.mach, 2), 5, false));
    } else {
        let spd_pre = if settings.is_speed_label_disabled() {
            ""
        } else {
            "SPD"
        };
        b.speed_str = format!(
            "{spd_pre}{}",
            pad_width((b.ias as i32).to_string(), 6, false)
        );
    }

    let alt_pre = if settings.is_altitude_label_disabled() {
        ""
    } else {
        "ALT"
    };
    // Display decision: separate from warning flag
    // When alwaysRadar is enabled, use radar altitude if valid; otherwise use warning-based logic
    let use_radar_alt = if always_radar {
        radio_alt_valid
    } else {
        b.warn_altitude
    };

    if use_radar_alt {
        b.alt_str = format!(
            "{alt_pre}R{}",
            pad_width(java_f(b.radio_altitude, 0), 5, false)
        );
    } else {
        b.alt_str = format!("{alt_pre}{}", pad_width(java_f(b.altitude, 0), 6, false));
    }

    // AoA 和 Energy 数据始终计算，显示/隐藏由组件级开关控制
    b.aoa_str = format!("α{}", pad_width(java_f(b.aoa, 0), 3, false));
    b.energy_str = format!("E{}", pad_width(java_f(b.energy_m, 0), 5, false));

    let sep_pre = if settings.is_sep_label_disabled() {
        ""
    } else {
        "SEP"
    };
    if b.vertical_speed > 0.0 {
        b.sep_str = format!(
            "{sep_pre}↑{}",
            pad_width(java_f(b.vertical_speed, 0), 4, true)
        );
    } else {
        b.sep_str = format!(
            "{sep_pre}↓{}",
            pad_width(java_f(b.vertical_speed, 0), 4, true)
        );
    }

    // Maneuver / Time
    if b.g_load > 1.5 || b.g_load < -0.5 {
        b.maneuver_state_str = format!("G{}", pad_width(java_f(b.g_load, 1), 5, false));
    } else {
        // PORT: Java `time != null && !time.isEmpty()` — Rust String 无 null,
        // EventPayload.timeStr 由 Builder 缺省 "--:--", null 分支坍缩
        let time = &payload.time_str;
        b.maneuver_state_str = if !time.is_empty() {
            format!("L{time}")
        } else {
            String::new()
        };
    }

    // Configuration（组件级拆分：襟翼/可变翼、减速板、起落架各自独立字符串）
    let mut brk = "";
    let mut gear = "";
    let mut in_action = false;
    if b.airbrake > 0.0 {
        brk = "BRK";
        if b.airbrake != 100.0 {
            in_action = true;
        }
    }
    if b.gear > 0.0 {
        gear = "GEA";
        if b.gear != 100.0 {
            in_action = true;
        }
    }

    if b.flaps > 0.0 {
        b.flaps_wing_str = format!("F{}", pad_width(java_f(b.flaps, 0), 3, false));
    } else if fmdata.is_some() && fmdata.unwrap().is_v_wing.unwrap() && s_indic.is_some() {
        b.flaps_wing_str = format!(
            "W{}",
            pad_width(
                java_f(s_indic.unwrap().wsweep_indicator * 100.0, 0),
                3,
                false
            )
        );
    } else {
        b.flaps_wing_str = String::new();
    }
    b.airbrake_str = brk.to_string();
    b.gear_str = gear.to_string();

    if b.flaps > 0.0 {
        // Restore readable text if Bar is disabled
        if !settings.enable_flap_angle_bar() {
            b.mechanization_str =
                format!("F{}{brk}{gear}", pad_width(java_f(b.flaps, 0), 3, false));
        } else {
            // Bar enabled -> Hide text (keep Brk/Gear)
            b.mechanization_str = format!("{}{brk}{gear}", pad_width(String::new(), 4, false));
        }
    } else if fmdata.is_some() && fmdata.unwrap().is_v_wing.unwrap() && s_indic.is_some() {
        // approx logic (Java 行尾注释)
        b.mechanization_str = format!(
            "W{}{brk}{gear}",
            pad_width(
                java_f(s_indic.unwrap().wsweep_indicator * 100.0, 0),
                3,
                false
            )
        );
    } else {
        b.mechanization_str = format!("{}{brk}{gear}", pad_width(String::new(), 4, false));
    }

    b.warn_configuration = in_action;
}

/// 速度比例条段: 限速比/舵面锁定比/失速比 (calculate 的 Speed Ratio Bar 段)。
/// valid_fmdata 在本段按同式重筛 (filter 纯函数, 与警告段等值)。
fn compute_speed_bar(b: &mut Builder, source: &dyn FormulaView, fmdata: Option<&FmData>) {
    // --- Speed Ratio Bar Logic ---
    b.speed_bar_speed_ratio = v(source, "speed_limit_ratio");
    b.speed_bar_aileron_lock_ratio = v(source, "aileron_lock_ratio");
    b.speed_bar_rudder_lock_ratio = v(source, "rudder_lock_ratio");
    b.speed_bar_unit_mach_limit_ratio = v(source, "unit_mach_limit_ratio");

    // Calculate Stall Ratio
    let mut current_limit = 1.0;
    // If we have valid speed ratio, derive the current limit (VNE or MachLimit_IAS)
    if b.speed_bar_speed_ratio > 0.0001 && b.ias > 1.0 {
        current_limit = b.ias / b.speed_bar_speed_ratio;
    } else if let Some(fmdata) = fmdata.filter(|x| x.valid) {
        // Fallback to static VNE
        let mut vwing = 0.0;
        if v(source, "wing_sweep_valid") != 0.0 {
            vwing = v(source, "wing_sweep");
        }
        current_limit = fmdata.get_vne_v_wing(vwing);
    }

    let stall_speed = v(source, "stall_speed");
    if current_limit > 0.1 {
        b.speed_bar_stall_ratio = stall_speed / current_limit;
    } else {
        b.speed_bar_stall_ratio = 0.0;
    }
}

// --- Helper for Text Measurement ---

/// 对应 Java `public static int getStringWidth(String text, java.awt.Font font)`
/// (离屏 Graphics2D 度量, 模块头注 3 的解耦点)。
///
/// PORT: `java.awt.Font` 参数化为泛型 `F` (字体句柄由调用方定义, 如 vm-overlay
/// 的 FontHandle); 度量闭包由调用方注入, 承接原 `MEASURE_G.setFont(font) +
/// getFontMetrics().stringWidth(text)`。三重早退守卫 (text null/空、font null → 0)
/// 与求值顺序逐条保持。
pub fn get_string_width<F>(
    text: Option<&str>,
    font: Option<&F>,
    measure: impl Fn(&F, &str) -> i32,
) -> i32 {
    if text.is_none() || text.unwrap().is_empty() || font.is_none() {
        return 0;
    }
    measure(font.unwrap(), text.unwrap())
}

#[cfg(test)]
mod tests;
