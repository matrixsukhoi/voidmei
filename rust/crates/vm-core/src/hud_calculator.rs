//! 对应 Java: `src/ui/overlay/logic/HUDCalculator.java` (B 类)。
//!
//! Pure logic calculator for HUD Data.
//! Extracts raw data from FlightDataEvent (Data/State) and performs business
//! logic calculations.
//!
//! # 三个解耦点 (CLASSIFY.md §23-3/6 裁决)
//!
//! 1. **MinimalHUDContext 字体参数**: Java `calculate(..., MinimalHUDContext ctx)`
//!    的 `ctx` 形参在方法体内**从未被读取** (两个调用点 Service.java:472 传 null、
//!    MiniHUDOverlay.java:446 传实句柄, 行为无差) —— Rust 签名直接砍除该参数,
//!    字体参数由 C 类组件层 (MiniHUD 波次) 自持, 本纯逻辑层不引入 C 类类型。
//! 2. **Application 静态颜色 6 处 → 参数注入**: Java 读
//!    `Application.colorWarning/colorNum/colorUnit` (LIFETIMES §1.2 配置驱动可变
//!    全局, ConfigurationService.java:136-139 重写)。Rust 侧以 [`HudColors`]
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
//! PORT: NPE→panic 的降级契约在调用点: Java Service.java:466-478 以 catch(Exception)
//! 包裹 calculate, 失败仅 log、事件照发 (无 hudData); Rust 侧等价降级需**调用点粒度**
//! catch_unwind (§6 循环级会额外丢整轮事件发布, 降级幅度不同) — 归 vm-data Service
//! (D6) 波次落实, 过渡期警告见 calculate() 内 unwrap 处。

use crate::blkx::Blkx;
use crate::config_api::HUDSettings;
use crate::event::flight_data_event::FlightDataEvent;
use crate::g;
use crate::hud_data::HUDData;
use crate::hud_data::Builder;
use crate::parser::{Indicators, State};
use crate::ui_model::TelemetrySource;

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

/// 对应 Java `public static HUDData calculate(FlightDataEvent, TelemetrySource,
/// Blkx, HUDSettings, MinimalHUDContext)` —— ctx 形参解耦砍除 (模块头注 1),
/// colors 参数为 Application 静态颜色注入 (模块头注 2)。
///
/// PORT: `event == null || source == null` 早退 → 两个 Option 参数;
/// `blkx != null` 判空 → `Option<&Blkx>`;
/// `(parser.State) event.getState()` 错误强转 (ClassCastException) → downcast_ref
/// (flight_data_event.rs OpaqueObject 契约: 类型不符返回 None, Java 里该值恒为
/// State/Indicators 或 null, None 分支仅承接 Java null)。
pub fn calculate<S: HUDSettings>(
    event: Option<&FlightDataEvent>,
    source: Option<&dyn TelemetrySource>,
    blkx: Option<&Blkx>,
    settings: &S,
    colors: &HudColors,
) -> HUDData {
    let mut b = Builder::default();

    let (event, source) = match (event, source) {
        (Some(e), Some(s)) => (e, s),
        // Java: if (event == null || source == null) return b.build();
        _ => return b.build(),
    };

    let payload = event.get_payload();
    let s_state: Option<&State> = event.get_state().and_then(|o| o.downcast_ref::<State>());
    let s_indic: Option<&Indicators> = event
        .get_indicators()
        .and_then(|o| o.downcast_ref::<Indicators>());

    // --- Raw Flight Data ---
    b.ias = source.get_ias();
    b.mach = source.get_mach();
    b.altitude = source.get_altitude();
    b.radio_altitude = source.get_radio_altitude();
    b.vertical_speed = source.get_sep();
    b.heading = source.get_compass();

    b.map_grid = payload.map_grid.clone();

    // --- Attitude ---
    let mut aviahp = 0.0;
    let mut aviar = 0.0;
    if let Some(s_indic) = s_indic {
        aviahp = s_indic.aviahorizon_pitch;
        aviar = s_indic.aviahorizon_roll;
    }

    b.pitch_valid = aviahp != -65535.0;
    if b.pitch_valid {
        b.pitch = -aviahp;
    } else {
        b.pitch = 0.0;
    }

    if aviar != -65535.0 {
        b.roll = -aviar;
    } else {
        b.roll = 0.0;
    }

    // --- AoS / System State ---
    if let Some(s_state) = s_state {
        if s_state.aos != -65535.0 {
            b.slip = s_state.aos;
        }

        b.throttle = s_state.throttle;
        if s_state.flaps == 65535 || s_state.flaps == -65535 {
            b.flaps = 0.0;
        } else {
            b.flaps = s_state.flaps as f64;
        }
        b.gear = s_state.gear as f64;
        b.airbrake = s_state.airbrake as f64;

        // Java 连续两次相同赋值 (源码 L73/L75), 原样保留
        b.airbrake = s_state.airbrake as f64;

        if b.throttle > 100 {
            b.throttle_color = COLOR_RED;
        } else {
            b.throttle_color = COLOR_WHITE; // Or default? HUDData defaults to GREEN, but white is standard
                                             // text.
        }

        let is_downing_flap = payload.is_downing_flap;
        b.flap_allow_angle = get_flap_allow_angle(b.ias, is_downing_flap, blkx);

        b.aoa = s_state.aoa;
        b.g_load = s_state.ny;
    }

    b.energy_m = source.get_energy_jkg() / g;

    b.is_mach_mode = settings.draw_hud_mach();
    b.is_gear_down = b.gear > 0.0;
    b.is_flaps_down = b.flaps > 0.0;
    b.is_airbrake_active = b.airbrake > 0.0;

    // --- Warning Logic ---
    let mut warn_vne = false;
    if b.is_airbrake_active && b.airbrake == 100.0 {
        warn_vne = true;
    }

    // Java: if (blkx != null && blkx.valid)
    let valid_blkx = blkx.filter(|x| x.valid);
    if let Some(blkx) = valid_blkx {
        // User requested formula: 1 - (nfweight / (nfweight + fuel))
        let nfweight = blkx.nofuelweight;
        let current_fuel = s_state.map_or(0.0, |s| s.mfuel);

        // Check for valid weights to avoid division by zero
        if nfweight > 0.0 && (nfweight + current_fuel) > 0.0 {
            b.maneuver_index = 1.0 - (nfweight / (nfweight + current_fuel));
        } else {
            b.maneuver_index = 0.0;
        }

        let mut vwing = 0.0;
        // PORT: Java `blkx.isVWing` (Boolean 装箱) 在布尔上下文自动拆箱, null → NPE
        // (§1 非受检异常 → panic)。"不可达"仅对 Java 生产链成立: FMLoader.load L101
        // 两参构造 = doLoad=true → getload L1333 必赋值; doLoad=false 构造 (lookup
        // 用) 上为 null 会真 NPE, unwrap 忠实复刻两者。
        // ⚠ 过渡期: Blkx::parse 等价 doLoad=false (getload 未译, reader.rs),
        // valid=true 而 is_v_wing=None → 此 unwrap 必 panic; getload 波次落地前
        // 禁止把 calculate() 接入 service_loop — panic 被 §6 catch_unwind 吞掉会
        // 变成 HUD 永久空帧的静默降级, 比崩溃更难被发现 (L312/L335 同因)。
        // PORT: Java 保真 — `blkx.isVWing && sIndic != null` 的直译 (is_some 检查 +
        // unwrap 取值), 不改成 if-let 以保持与 Java 源逐行对应
        #[allow(clippy::unnecessary_unwrap)]
        if blkx.is_v_wing.unwrap() && s_indic.is_some() {
            vwing = s_indic.unwrap().wsweep_indicator;
        }

        // Dynamic Vne calculation
        // PORT: 第二项 Java 是 `* 0.95f` (float 字面量提升), 与第一项的 0.95 (double)
        // 位级不同 (0.94999998807907104...), 不得合并 (§2.12)
        if b.ias >= blkx.get_vne_v_wing(vwing) * 0.95
            || b.mach >= blkx.get_mne_v_wing(vwing) * (0.95f32 as f64)
        {
            warn_vne = true;
        }

        // AoA Warnings
        // Java: b.flaps > 0 ? (int) b.flaps : 0 — (int) 截断/饱和/NaN→0 ↔ as i32 同语义
        let max_available_aoa =
            blkx.get_aoa_high_v_wing(vwing, if b.flaps > 0.0 { b.flaps as i32 } else { 0 });
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

        if available_aoa <= 0.0 {
            b.warn_stall = true;
        }
    } else {
        b.maneuver_index = 0.0;
        b.aoa_color = colors.color_num;
        b.aoa_bar_color = colors.color_num;
        b.aoa_ratio = b.aoa / 30.0;
    }
    b.warn_vne = warn_vne;

    // Warnings
    let radio_alt = b.radio_altitude;
    let radio_alt_valid = source.is_radio_altitude_valid();
    let always_radar = settings.always_show_radar_altitude();

    // Low altitude warning flag (for color/audio warnings) - always based on <=500m threshold
    if radio_alt_valid && radio_alt <= 500.0 {
        b.warn_altitude = true;
    }

    // --- Strings Formatting (using Data) ---
    if b.is_mach_mode {
        b.speed_str = format!("M{}", pad_width(java_f(b.mach, 2), 5, false));
    } else {
        let spd_pre = if settings.is_speed_label_disabled() { "" } else { "SPD" };
        // Java: (int) b.ias 截断后 %6d
        b.speed_str = format!("{spd_pre}{}", pad_width((b.ias as i32).to_string(), 6, false));
    }

    let alt_pre = if settings.is_altitude_label_disabled() { "" } else { "ALT" };
    // Display decision: separate from warning flag
    // When alwaysRadar is enabled, use radar altitude if valid; otherwise use warning-based logic
    let use_radar_alt = if always_radar { radio_alt_valid } else { b.warn_altitude };

    if use_radar_alt {
        b.alt_str = format!("{alt_pre}R{}", pad_width(java_f(b.radio_altitude, 0), 5, false));
    } else {
        b.alt_str = format!("{alt_pre}{}", pad_width(java_f(b.altitude, 0), 6, false));
    }

    // AoA 和 Energy 数据始终计算，显示/隐藏由组件级开关控制
    b.aoa_str = format!("α{}", pad_width(java_f(b.aoa, 0), 3, false));
    b.energy_str = format!("E{}", pad_width(java_f(b.energy_m, 0), 5, false));

    let sep_pre = if settings.is_sep_label_disabled() { "" } else { "SEP" };
    if b.vertical_speed > 0.0 {
        b.sep_str = format!("{sep_pre}↑{}", pad_width(java_f(b.vertical_speed, 0), 4, true));
    } else {
        b.sep_str = format!("{sep_pre}↓{}", pad_width(java_f(b.vertical_speed, 0), 4, true));
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
    } else if blkx.is_some() && blkx.unwrap().is_v_wing.unwrap() && s_indic.is_some() {
        b.flaps_wing_str = format!(
            "W{}",
            pad_width(java_f(s_indic.unwrap().wsweep_indicator * 100.0, 0), 3, false)
        );
    } else {
        b.flaps_wing_str = String::new();
    }
    b.airbrake_str = brk.to_string();
    b.gear_str = gear.to_string();

    if b.flaps > 0.0 {
        // Restore readable text if Bar is disabled
        if !settings.enable_flap_angle_bar() {
            b.mechanization_str = format!(
                "F{}{brk}{gear}",
                pad_width(java_f(b.flaps, 0), 3, false)
            );
        } else {
            // Bar enabled -> Hide text (keep Brk/Gear)
            // Java: String.format("%4s%s%s", "", brk, gear) — 空串补 4 空格
            b.mechanization_str = format!("{}{brk}{gear}", pad_width(String::new(), 4, false));
        }
    } else if blkx.is_some() && blkx.unwrap().is_v_wing.unwrap() && s_indic.is_some() {
        // approx logic (Java 行尾注释)
        b.mechanization_str = format!(
            "W{}{brk}{gear}",
            pad_width(java_f(s_indic.unwrap().wsweep_indicator * 100.0, 0), 3, false)
        );
    } else {
        b.mechanization_str = format!("{}{brk}{gear}", pad_width(String::new(), 4, false));
    }

    b.warn_configuration = in_action;

    // --- Speed Ratio Bar Logic ---
    b.speed_bar_speed_ratio = source.get_speed_limit_ratio();
    b.speed_bar_aileron_lock_ratio = source.get_aileron_lock_ratio();
    b.speed_bar_rudder_lock_ratio = source.get_rudder_lock_ratio();
    b.speed_bar_unit_mach_limit_ratio = source.get_unit_mach_limit_ratio();

    // Calculate Stall Ratio
    let mut current_limit = 1.0;
    // If we have valid speed ratio, derive the current limit (VNE or MachLimit_IAS)
    if b.speed_bar_speed_ratio > 0.0001 && b.ias > 1.0 {
        current_limit = b.ias / b.speed_bar_speed_ratio;
    } else if let Some(blkx) = valid_blkx {
        // Fallback to static VNE
        let mut vwing = 0.0;
        if source.is_wing_sweep_valid() {
            vwing = source.get_wing_sweep();
        }
        current_limit = blkx.get_vne_v_wing(vwing);
    }

    let stall_speed = source.get_stall_speed();
    if current_limit > 0.1 {
        b.speed_bar_stall_ratio = stall_speed / current_limit;
    } else {
        b.speed_bar_stall_ratio = 0.0;
    }

    b.build()
}

/// 对应 Java `private static double getFlapAllowAngle(double ias, boolean
/// isDowningFlap, Blkx blkx)`。
/// PORT: 形参 isDowningFlap 在 Java 方法体内未使用 — 签名保真, `_` 前缀消告警。
fn get_flap_allow_angle(ias: f64, _is_downing_flap: bool, blkx: Option<&Blkx>) -> f64 {
    // Java: if (ias == 0 || blkx == null || !blkx.valid) return 125;
    // (纯条件短路, 逐项保持判定顺序)
    if ias == 0.0 {
        return 125.0;
    }
    let blkx = match blkx {
        None => return 125.0,
        Some(b) => b,
    };
    if !blkx.valid {
        return 125.0;
    }

    // PORT: Java 直接解引用 FlapsDestructionIndSpeed (doLoad=false 构造的 blkx 上
    // 为 null → NPE) — unwrap panic 复刻同一硬失败 (§1)。⚠ 过渡期同 is_v_wing:
    // Blkx::parse 的 valid=true 不保证本字段 Some (getload L1189-1218 未译),
    // 接线 service_loop 前须等 getload 波次落地。
    let speeds = blkx.flaps_destruction_ind_speed.as_ref().unwrap();

    let mut i: i32 = 0;
    // Java: for (; i < FlapsDestructionNum - 1; i++) { if (...) break; }
    while i < blkx.flaps_destruction_num - 1 {
        if ias > speeds[i as usize][1] {
            break;
        }
        i += 1;
    }

    let x0: f64;
    let x1: f64;
    let y0: f64;
    let y1: f64;
    let t: f64;
    // PORT: Java `* 100.0f` (float 字面量提升 double) — 100 精确可表示, 值同 100.0
    if i == 0 {
        x0 = speeds[i as usize][1];
        y0 = speeds[i as usize][0] * 100.0;
        x1 = speeds[(i + 1) as usize][1];
        y1 = speeds[(i + 1) as usize][0] * 100.0;
        let k = calc_k(x0, y0, x1, y1);
        t = y0 + (ias - x0) * k;
        norm_flap_angle(t)
    } else {
        if ias == speeds[(i - 1) as usize][1] {
            return speeds[(i - 1) as usize][0] * 100.0;
        }
        x0 = speeds[(i - 1) as usize][1];
        y0 = speeds[(i - 1) as usize][0] * 100.0;
        x1 = speeds[i as usize][1];
        y1 = speeds[i as usize][0] * 100.0;
        let k = calc_k(x0, y0, x1, y1);
        t = y0 + (ias - x0) * k;
        norm_flap_angle(t)
    }
}

/// 对应 Java `private static double calcK(double x0, double y0, double x1, double y1)`。
fn calc_k(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    if (x1 - x0).abs() < 0.0001 {
        return 0.0;
    }
    (y1 - y0) / (x1 - x0)
}

/// 对应 Java `private static double normFlapAngle(double t)`。
fn norm_flap_angle(t: f64) -> f64 {
    if t < 0.0 {
        return 0.0;
    }
    if t < 125.0 {
        return t;
    }
    125.0
}

// --- Helper for Text Measurement ---

/// 对应 Java `public static int getStringWidth(String text, java.awt.Font font)`
/// (离屏 Graphics2D 度量, 模块头注 3 的解耦点)。
///
/// PORT: `java.awt.Font` 参数化为泛型 `F` (字体句柄由调用方定义, 如 vm-overlay
/// 的 FontHandle); 度量闭包由调用方注入, 承接原 `MEASURE_G.setFont(font) +
/// getFontMetrics().stringWidth(text)`。三重早退守卫 (text null/空、font null → 0)
/// 与求值顺序逐条保持。
pub fn get_string_width<F>(text: Option<&str>, font: Option<&F>, measure: impl Fn(&F, &str) -> i32) -> i32 {
    if text.is_none() || text.unwrap().is_empty() || font.is_none() {
        return 0;
    }
    measure(font.unwrap(), text.unwrap())
}

/// Java `String.format("%N.Mf", d)` 的数值段 (不含宽度): 对**最短往返十进制**
/// HALF_UP。语义模型与 config_loader::java_format_f4 / flight_analyzer::java_format_f1
/// 同源 (Java 8 oracle 实证, 本模块 build/oracle_hud 全格式串对拍):
/// - 2.675 → "2.68" (Rust `{:.2}` 会给 "2.67");
/// - -0.4 → "-0" / -0.04 → "-0.0" (舍入到零仍保留负号);
/// - NaN/Infinity 原样 ("NaN"/"Infinity"/"-Infinity");
/// - `exp10 > 25` 是纯实现切点, 非语义边界: else 支路的 scaled 定点累加在 u128
///   内, 10^308 量级会溢出; 该域最短表示位数 n ≤ 17 < keep, 判定位恒 0, 无舍入,
///   走 digits + 补零的字符串路径;
/// - JDK-4511638 已知分歧 (同 config_loader::java_format_f4 裁决): Java 8 旧 dtoa
///   在大值域 (~1e17 起) 偶发非最短 toString, 而 %f 按**自身 toString 的数字**
///   展开 — 1e23 → "9.999999999999999E22" → "99999999999999990000000", 既非精确
///   二进制 (...91611392) 也非最短展开; Rust `{:e}` 给真最短 "1e23" → 本实现输出
///   "100000000000000000000000"。HUD 值域 (速度/高度/能量 < 10^7) 距该域不可达
///   (Java 8 oracle fuzz 35k 例仅 1e23 一例分歧)。
fn java_f(d: f64, prec: usize) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    // 负号含 -0.0: Java 舍入到零的负数仍输出 "-0"/"-0.0" (oracle 验证)
    let neg = d.is_sign_negative();
    let a = d.abs();
    // Rust `{:e}` 即最短往返科学计数 (与 Java Double.toString 同一最短表示)
    let sci = format!("{a:e}");
    let epos = sci.find('e').unwrap();
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits = sci[..epos].replace('.', "");
    let digits = digits.as_bytes();
    let n = digits.len() as i32;

    let mut out = String::new();
    if exp10 > 25 {
        // 巨整数域: digits + 隐含尾零 (+ 小数点补零)
        out.push_str(&sci[..epos].replace('.', ""));
        out.push_str(&"0".repeat((exp10 - n + 1) as usize));
        if prec > 0 {
            out.push('.');
            out.push_str(&"0".repeat(prec));
        }
    } else {
        // 最短表示的 i 号数字 (1-based, place = 10^(exp10-i+1)); 越界补 0
        let digit_at = |i: i32| -> u128 {
            if i < 1 {
                0
            } else {
                let idx = (i - 1) as usize;
                if idx < digits.len() {
                    u128::from(digits[idx] - b'0')
                } else {
                    0
                }
            }
        };
        // 保留到 10^-prec 位: i ≤ exp10 + 1 + prec; 判定位 = 其后一位
        // (HALF_UP: ≥5 进位, 再后的剩余数字 < 1 单位不影响判定; 进位可级联)
        let keep = exp10 + 1 + prec as i32;
        let mut scaled: u128 = 0;
        if keep > 0 {
            for i in 1..=keep {
                scaled = scaled * 10 + digit_at(i);
            }
        }
        if digit_at(keep + 1) >= 5 {
            scaled += 1;
        }
        let p10 = 10u128.pow(prec as u32);
        let int_part = scaled / p10;
        let frac = scaled % p10;
        out.push_str(&int_part.to_string());
        if prec > 0 {
            out.push('.');
            let fs = frac.to_string();
            for _ in fs.len()..prec {
                out.push('0');
            }
            out.push_str(&fs);
        }
    }
    if neg {
        out.insert(0, '-');
    }
    out
}

/// Java printf 宽度语义: 不足补空格 (默认右对齐, '-' 左对齐), 超宽不截断。
/// 宽度按字符计 (数值/NaN/Infinity 输出纯 ASCII, 与 Java UTF-16 码元计数同值)。
fn pad_width(mut s: String, width: usize, left_align: bool) -> String {
    let len = s.chars().count();
    if len >= width {
        return s;
    }
    let fill = " ".repeat(width - len);
    if left_align {
        s.push_str(&fill);
    } else {
        s.insert_str(0, &fill);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blkx::{FmParts, SweepLevel};
    use crate::config_api::overlay_settings::OverlaySettings;
    use crate::event::event_payload::EventPayload;
    use crate::event::flight_data_event::OpaqueObject;

    // Java 8 oracle: build/oracle_hud (HudOracle.java, 本仓库)。
    // 位级断言 (to_bits) 锁定浮点链的运算顺序保真。

    // ==== Java MockSrc 的 Rust 同构 mock (字段集一致, 未用 getter 恒 0/false) ====
    struct MockSrc {
        ias: f64,
        mach: f64,
        alt: f64,
        ralt: f64,
        sep: f64,
        compass: f64,
        energy: f64,
        speed_ratio: f64,
        aileron_ratio: f64,
        rudder_ratio: f64,
        mach_limit_ratio: f64,
        stall_speed: f64,
        wsweep: f64,
        ralt_valid: bool,
        wsweep_valid: bool,
    }

    impl TelemetrySource for MockSrc {
        fn get_ias(&self) -> f64 { self.ias }
        fn get_tas(&self) -> f64 { 0.0 }
        fn get_mach(&self) -> f64 { self.mach }
        fn get_aoa(&self) -> f64 { 0.0 }
        fn get_aos(&self) -> f64 { 0.0 }
        fn get_ny(&self) -> f64 { 0.0 }
        fn get_vario(&self) -> f64 { 0.0 }
        fn get_altitude(&self) -> f64 { self.alt }
        fn get_radio_altitude(&self) -> f64 { self.ralt }
        fn is_radio_altitude_valid(&self) -> bool { self.ralt_valid }
        fn get_compass(&self) -> f64 { self.compass }
        fn get_sep(&self) -> f64 { self.sep }
        fn get_acceleration(&self) -> f64 { 0.0 }
        fn get_turn_rate(&self) -> f64 { 0.0 }
        fn get_turn_radius(&self) -> f64 { 0.0 }
        fn is_turn_radius_valid(&self) -> bool { false }
        fn get_roll_rate(&self) -> f64 { 0.0 }
        fn get_energy_jkg(&self) -> f64 { self.energy }
        fn get_mass_fuel(&self) -> f64 { 0.0 }
        fn get_total_weight(&self) -> f64 { 0.0 }
        fn get_fuel_time_mili(&self) -> i64 { 0 }
        fn get_throttle(&self) -> f64 { 0.0 }
        fn get_rpm(&self) -> f64 { 0.0 }
        fn get_manifold_pressure(&self) -> f64 { 0.0 }
        fn get_water_temp(&self) -> f64 { 0.0 }
        fn get_oil_temp(&self) -> f64 { 0.0 }
        fn get_pitch(&self) -> f64 { 0.0 }
        fn get_eff_hp(&self) -> f64 { 0.0 }
        fn get_thrust(&self) -> f64 { 0.0 }
        fn get_horse_power(&self) -> f64 { 0.0 }
        fn get_engine_response(&self) -> f64 { 0.0 }
        fn get_prop_efficiency(&self) -> f64 { 0.0 }
        fn get_wep_kg(&self) -> f64 { 0.0 }
        fn get_wep_time(&self) -> f64 { 0.0 }
        fn get_heat_tolerance(&self) -> f64 { 0.0 }
        fn get_power_percent(&self) -> f64 { 0.0 }
        fn get_manifold_pressure_pounds(&self) -> f64 { 0.0 }
        fn get_manifold_pressure_inch_hg(&self) -> f64 { 0.0 }
        fn get_manifold_pressure_display(&self) -> f64 { 0.0 }
        fn get_manifold_pressure_display_unit(&self) -> String { "Ata".to_string() }
        fn get_manifold_pressure_display_precision(&self) -> i32 { 2 }
        fn get_unknown_mixture(&self) -> f64 { 0.0 }
        fn get_radiator(&self) -> f64 { 0.0 }
        fn get_compressor_stage(&self) -> f64 { 0.0 }
        fn get_fuel_percent(&self) -> f64 { 0.0 }
        fn get_rpm_throttle(&self) -> f64 { 0.0 }
        fn get_gear(&self) -> f64 { 0.0 }
        fn get_flaps(&self) -> f64 { 0.0 }
        fn get_airbrake(&self) -> f64 { 0.0 }
        fn get_aileron(&self) -> f64 { 0.0 }
        fn get_elevator(&self) -> f64 { 0.0 }
        fn get_rudder(&self) -> f64 { 0.0 }
        fn get_wing_sweep(&self) -> f64 { self.wsweep }
        fn is_wing_sweep_valid(&self) -> bool { self.wsweep_valid }
        fn get_speed_limit_ratio(&self) -> f64 { self.speed_ratio }
        fn get_aileron_lock_ratio(&self) -> f64 { self.aileron_ratio }
        fn get_rudder_lock_ratio(&self) -> f64 { self.rudder_ratio }
        fn get_unit_mach_limit_ratio(&self) -> f64 { self.mach_limit_ratio }
        fn get_stall_speed(&self) -> f64 { self.stall_speed }
        fn is_imperial(&self) -> bool { false }
        fn get_aviahorizon_pitch(&self) -> f64 { 0.0 }
        fn get_aviahorizon_roll(&self) -> f64 { 0.0 }
        fn is_jet_engine(&self) -> bool { false }
        fn is_prop_engine(&self) -> bool { false }
        fn is_piston_engine(&self) -> bool { false }
        fn is_turboprop_engine(&self) -> bool { false }
        fn is_engine_check_done(&self) -> bool { false }
        fn has_wep(&self) -> bool { false }
        fn get_booster_fuel_kg(&self) -> f64 { 0.0 }
        fn get_booster_fuel_percent(&self) -> f64 { 0.0 }
        fn has_booster(&self) -> bool { false }
    }

    #[allow(clippy::too_many_arguments)]
    fn mk_src(
        ias: f64,
        mach: f64,
        alt: f64,
        ralt: f64,
        sep: f64,
        compass: f64,
        energy: f64,
        speed_ratio: f64,
        aileron_ratio: f64,
        rudder_ratio: f64,
        mach_limit_ratio: f64,
        stall_speed: f64,
        ralt_valid: bool,
        wsweep_valid: bool,
        wsweep: f64,
    ) -> MockSrc {
        MockSrc {
            ias,
            mach,
            alt,
            ralt,
            sep,
            compass,
            energy,
            speed_ratio,
            aileron_ratio,
            rudder_ratio,
            mach_limit_ratio,
            stall_speed,
            wsweep,
            ralt_valid,
            wsweep_valid,
        }
    }

    // ==== Java MockHud 的 Rust 同构 mock (开关字段集一致) ====
    struct MockHud {
        mach_mode: bool,
        spd_label_off: bool,
        alt_label_off: bool,
        sep_label_off: bool,
        flap_bar: bool,
        always_radar: bool,
        aoa_warn_ratio: f64,
        aoa_bar_warn_ratio: f64,
    }

    impl Default for MockHud {
        fn default() -> Self {
            MockHud {
                mach_mode: false,
                spd_label_off: false,
                alt_label_off: false,
                sep_label_off: false,
                flap_bar: false,
                always_radar: false,
                aoa_warn_ratio: 0.0,
                aoa_bar_warn_ratio: 0.0,
            }
        }
    }

    impl OverlaySettings for MockHud {
        type GroupConfig = ();

        fn get_window_x(&self, _width: i32) -> i32 { 0 }
        fn get_window_y(&self, _height: i32) -> i32 { 0 }
        fn save_window_position(&self, _x: f64, _y: f64) {}
        fn get_font_name(&self) -> String { "text".to_string() }
        fn get_num_font_name(&self) -> String { "num".to_string() }
        fn get_font_size_add(&self) -> i32 { 0 }
        fn get_bool(&self, _key: &str, def: bool) -> bool { def }
        fn get_int(&self, _key: &str, def: i32) -> i32 { def }
        fn get_string(&self, _key: &str, def: &str) -> String { def.to_string() }
        fn get_group_config(&self) -> Option<&Self::GroupConfig> { None }
        fn auto_hide_on_focus_loss(&self) -> bool { false }
    }

    impl HUDSettings for MockHud {
        fn get_num_font(&self) -> String { "DIN Pro 400".to_string() }
        fn get_crosshair_scale(&self) -> i32 { 40 }
        fn get_crosshair_name(&self) -> String { "crosshair_01".to_string() }
        fn is_display_crosshair(&self) -> bool { true }
        fn use_texture_crosshair(&self) -> bool { false }
        fn draw_hud_text(&self) -> bool { true }
        fn show_attitude_gauge(&self) -> bool { true }
        fn get_aoa_warning_ratio(&self) -> f64 { self.aoa_warn_ratio }
        fn get_aoa_bar_warning_ratio(&self) -> f64 { self.aoa_bar_warn_ratio }
        fn enable_flap_angle_bar(&self) -> bool { self.flap_bar }
        fn show_speed_bar(&self) -> bool { false }
        fn draw_hud_mach(&self) -> bool { self.mach_mode }
        fn is_speed_label_disabled(&self) -> bool { self.spd_label_off }
        fn is_altitude_label_disabled(&self) -> bool { self.alt_label_off }
        fn is_sep_label_disabled(&self) -> bool { self.sep_label_off }
        fn show_hud_speed(&self) -> bool { true }
        fn show_hud_aoa(&self) -> bool { true }
        fn show_hud_altitude(&self) -> bool { true }
        fn show_hud_energy(&self) -> bool { true }
        fn show_hud_mechanization(&self) -> bool { false }
        fn show_hud_flaps(&self) -> bool { true }
        fn show_hud_airbrake(&self) -> bool { true }
        fn show_hud_gear(&self) -> bool { true }
        fn show_hud_sep(&self) -> bool { true }
        fn show_hud_g_load(&self) -> bool { true }
        fn show_hud_maneuver_bar(&self) -> bool { true }
        fn is_attitude_indicator_inertial_mode(&self) -> bool { false }
        fn is_gpu_compatibility_mode(&self) -> bool { false }
        fn always_show_radar_altitude(&self) -> bool { self.always_radar }
    }

    // PORT: Java 保真 — 测试状态构造器逐字段喂值, 不打包成结构体
    #[allow(clippy::too_many_arguments)]
    fn mk_state(
        aos: f64,
        throttle: i32,
        flaps: i32,
        gear: i32,
        airbrake: i32,
        mfuel: f64,
        aoa: f64,
        ny: f64,
    ) -> State {
        State {
            aos,
            throttle,
            flaps,
            gear,
            airbrake,
            mfuel,
            aoa,
            ny,
            ..Default::default()
        }
    }

    fn mk_indic(aviahp: f64, aviar: f64, wsweep: f64) -> Indicators {
        // Indicators.army 为私有字段 (Java private), 经 new() 构造后逐字段赋值
        let mut i = Indicators::new();
        i.aviahorizon_pitch = aviahp;
        i.aviahorizon_roll = aviar;
        i.wsweep_indicator = wsweep;
        i
    }

    fn mk_event(
        payload: EventPayload,
        state: Option<State>,
        indic: Option<Indicators>,
    ) -> FlightDataEvent {
        FlightDataEvent::new(
            payload,
            state.map(|s| Box::new(s) as OpaqueObject),
            indic.map(|i| Box::new(i) as OpaqueObject),
        )
    }

    /// oracle: spit_flds — 真机 spitfire_f24 经 Java getload 后 calculate 消费的字段
    /// (Rust Blkx::parse 等价 doLoad=false, 手工复原同一 FM 状态)
    fn spitfire_blkx() -> Blkx {
        let mut b = Blkx::default();
        b.valid = true;
        b.is_v_wing = Some(false);
        b.nofuelweight = f64::from_bits(0x40ac_0566_6680_0000); // 3586.7000007629395
        b.vne = 875.0;
        b.vne_mach = f64::from_bits(0x3feb_d70a_4000_0000); // 0.8700000047683716
        b.flaps_destruction_num = 2;
        let mut rows = [[0.0; 2]; 6];
        rows[0] = [0.5, 290.0];
        rows[1] = [1.0, 260.0];
        rows[2] = [1.25, 0.0]; // 1.25x 哨兵行
        b.flaps_destruction_ind_speed = Some(rows);
        b.no_flaps_wing = Some(FmParts {
            aoa_crit_high: f64::from_bits(0x4031_cccc_c000_0000), // 17.799999237060547
            ..Default::default()
        });
        b.full_flaps_wing = Some(FmParts { aoa_crit_high: 16.0, ..Default::default() });
        // Java: 空 ArrayList (size 0) — 各 getter 的 null/<=1 守卫同 None 命中
        b.sweep_levels = None;
        b
    }

    /// oracle: vw_flds — 手搓可变翼 FM (Java doLoad=false + 逐字段赋值)
    fn vwing_blkx() -> Blkx {
        let mut vw = Blkx::default();
        vw.valid = true;
        vw.is_v_wing = Some(true);
        vw.nofuelweight = 3000.0;
        vw.vne = 700.0;
        vw.vne_mach = 0.8;
        vw.flaps_destruction_num = 3;
        let mut rows = [[0.0; 2]; 6];
        rows[0] = [0.2, 200.0];
        rows[1] = [0.5, 300.0];
        rows[2] = [1.0, 500.0];
        vw.flaps_destruction_ind_speed = Some(rows);
        vw.no_flaps_wing = Some(FmParts {
            aoa_crit_high: 17.5,
            aoa_crit_low: -15.0,
            ..Default::default()
        });
        vw.full_flaps_wing = Some(FmParts { aoa_crit_high: 22.0, ..Default::default() });
        vw.sweep_levels = Some(vec![
            SweepLevel {
                sweep: 0.0,
                vne: 760.0,
                vne_mach: 0.9,
                no_flaps: Some(FmParts {
                    aoa_crit_high: 17.5,
                    aoa_crit_low: -15.0,
                    ..Default::default()
                }),
                full_flaps: Some(FmParts { aoa_crit_high: 22.0, ..Default::default() }),
            },
            SweepLevel {
                sweep: 1.0,
                vne: 620.0,
                vne_mach: 0.78,
                no_flaps: Some(FmParts {
                    aoa_crit_high: 18.5,
                    aoa_crit_low: -14.0,
                    ..Default::default()
                }),
                full_flaps: Some(FmParts { aoa_crit_high: 23.0, ..Default::default() }),
            },
        ]);
        vw
    }

    /// oracle: dw_flds — 手搓降序行 FM (覆盖 else 分支 i-1 精确相等早退)
    fn descending_blkx() -> Blkx {
        let mut dw = Blkx::default();
        dw.valid = true;
        dw.is_v_wing = Some(false);
        dw.nofuelweight = 2500.0;
        dw.vne = 650.0;
        dw.flaps_destruction_num = 3;
        let mut rows = [[0.0; 2]; 6];
        rows[0] = [0.2, 500.0];
        rows[1] = [0.5, 300.0];
        rows[2] = [1.0, 200.0];
        dw.flaps_destruction_ind_speed = Some(rows);
        dw.no_flaps_wing = Some(FmParts { aoa_crit_high: 16.0, ..Default::default() });
        dw.full_flaps_wing = Some(FmParts { aoa_crit_high: 20.0, ..Default::default() });
        dw.sweep_levels = None;
        dw
    }

    const COLORS: HudColors = HudColors {
        color_warning: [216, 33, 13, 100],
        color_num: [27, 255, 128, 240],
        color_unit: [166, 166, 166, 220],
    };

    // ---- s1: event=null → Builder 全默认 (oracle s1_null_event) ----
    #[test]
    fn s1_null_event_returns_builder_defaults() {
        let src = mk_src(412.5, 0.62, 5300.0, 245.7, -13.2, 164.09, 14913.0, 0.83, 0.72, 0.66,
            0.9, 144.0, true, false, 0.0);
        let hud = MockHud::default();
        let h = calculate(None, Some(&src), None, &hud, &COLORS);
        assert_eq!(h, Builder::default().build());
        // 关键默认哨兵 (对齐 oracle 逐项): 三色 GREEN、全部空串、数值 0
        assert_eq!(h.aoa_color, [0, 255, 0, 255]);
        assert_eq!(h.speed_str, "");
        assert_eq!(h.maneuver_index, 0.0);
    }

    // ---- s2: source=null → 同一早退 (oracle s2_null_source) ----
    #[test]
    fn s2_null_source_returns_builder_defaults() {
        let ev = mk_event(EventPayload::builder().build(), Some(State::new()),
            Some(Indicators::new()));
        let hud = MockHud::default();
        let h = calculate(Some(&ev), None, None, &hud, &COLORS);
        assert_eq!(h, Builder::default().build());
    }

    // ---- s3: sState/sIndic null, blkx null (oracle s3_no_state) ----
    #[test]
    fn s3_no_state_keeps_defaults_and_formats() {
        let src = mk_src(412.5, 0.62, 5300.0, 245.7, -13.2, 164.09, 14913.0, 0.83, 0.72, 0.66,
            0.9, 144.0, true, false, 0.0);
        let hud = MockHud::default();
        let payload = EventPayload::builder()
            .map_grid("C4".to_string())
            .time_str("12:34".to_string())
            .is_downing_flap(true)
            .build();
        let ev = mk_event(payload, None, None);
        let h = calculate(Some(&ev), Some(&src), None, &hud, &COLORS);

        // aviahp/aviar 缺省 0 → pitch/roll = -0.0 (oracle 位级: 8000000000000000)
        assert_eq!(h.pitch.to_bits(), 0x8000_0000_0000_0000);
        assert_eq!(h.roll.to_bits(), 0x8000_0000_0000_0000);
        assert!(h.pitch_valid);
        // sState 缺 → throttle/flaps/gear/airbrake/flapAllowAngle 保持 Builder 默认 0
        assert_eq!(h.throttle, 0);
        assert_eq!(h.flap_allow_angle, 0.0);
        assert_eq!(h.energy_m.to_bits(), 0x4097_c6f0_5397_829c); // 1521.7346938775509
        assert_eq!(h.aoa_ratio, 0.0 / 30.0); // aoa=0 → else 分支
        assert_eq!(h.aoa_color, COLORS.color_num);
        // raltValid && 245.7 <= 500 → warnAltitude → 雷达高度显示
        assert!(h.warn_altitude);
        assert_eq!(h.speed_str, "SPD   412");
        assert_eq!(h.alt_str, "ALTR  246");
        assert_eq!(h.aoa_str, "α  0");
        assert_eq!(h.energy_str, "E 1522");
        assert_eq!(h.mechanization_str, "    ");
        assert_eq!(h.flaps_wing_str, "");
        assert_eq!(h.sep_str, "SEP↓-13 ");
        assert_eq!(h.maneuver_state_str, "L12:34"); // gLoad=0 → 时间分支
        assert_eq!(h.speed_bar_stall_ratio.to_bits(), 0x3fd2_8b30_84db_fe10);
        assert!(!h.warn_vne);
        assert!(!h.warn_configuration);
    }

    // ---- s4: 全量 state/indic, blkx=null (oracle s4_full_no_fm) ----
    #[test]
    fn s4_full_telemetry_without_fm() {
        let hud = MockHud { aoa_warn_ratio: 0.85, aoa_bar_warn_ratio: 0.95, ..Default::default() };
        let src = mk_src(412.5, 0.62, 5300.0, 245.7, -13.2, 164.09, 14913.0, 0.83, 0.72, 0.66,
            0.9, 144.0, true, false, 0.0);
        let payload = EventPayload::builder()
            .map_grid("C4".to_string())
            .time_str("12:34".to_string())
            .is_downing_flap(true)
            .build();
        let ev = mk_event(
            payload,
            Some(mk_state(-65535.0, 110, 50, 100, 80, 850.0, 8.3, 2.6)),
            Some(mk_indic(-65535.0, -40.55, 0.5)),
        );
        let h = calculate(Some(&ev), Some(&src), None, &hud, &COLORS);

        assert_eq!(h.ias.to_bits(), 0x4079_c800_0000_0000);
        assert_eq!(h.mach.to_bits(), 0x3fe3_d70a_3d70_a3d7);
        assert_eq!(h.altitude, 5300.0);
        assert_eq!(h.radio_altitude.to_bits(), 0x406e_b666_6666_6666);
        assert_eq!(h.vertical_speed.to_bits(), 0xc02a_6666_6666_6666);
        assert_eq!(h.heading.to_bits(), 0x4064_82e1_47ae_147b);
        // aviahp=-65535 → pitchValid=false, pitch=0; aviar=-40.55 → roll=40.55
        assert!(!h.pitch_valid);
        assert_eq!(h.pitch, 0.0);
        assert_eq!(h.roll.to_bits(), 0x4044_4666_6666_6666);
        assert_eq!(h.slip, 0.0); // AoS=-65535 → 不赋值, 保持默认
        assert_eq!(h.aoa.to_bits(), 0x4020_9999_9999_999a);
        assert_eq!(h.throttle, 110);
        assert_eq!(h.flaps, 50.0);
        assert_eq!(h.gear, 100.0);
        assert_eq!(h.airbrake, 80.0);
        assert_eq!(h.flap_allow_angle, 125.0); // blkx=null 短路
        assert_eq!(h.energy_m.to_bits(), 0x4097_c6f0_5397_829c);
        assert_eq!(h.g_load.to_bits(), 0x4004_cccc_cccc_cccd);
        assert_eq!(h.maneuver_index, 0.0);
        assert!(h.is_gear_down && h.is_flaps_down && h.is_airbrake_active);
        assert!(!h.is_mach_mode);
        assert!(!h.warn_vne); // airbrake=80 ≠ 100 且无 FM
        assert!(!h.warn_stall);
        assert!(h.warn_altitude);
        assert_eq!(h.aoa_color, COLORS.color_num);
        assert_eq!(h.aoa_bar_color, COLORS.color_num);
        assert_eq!(h.throttle_color, COLOR_RED); // throttle=110 > 100
        assert!(h.warn_configuration); // airbrake 80 ≠ 100 → inAction
        assert_eq!(h.map_grid, "C4");
        assert_eq!(h.speed_str, "SPD   412");
        assert_eq!(h.alt_str, "ALTR  246");
        assert_eq!(h.aoa_str, "α  8");
        assert_eq!(h.energy_str, "E 1522");
        assert_eq!(h.mechanization_str, "F 50BRKGEA"); // flapBar=false → 文字保留
        assert_eq!(h.flaps_wing_str, "F 50");
        assert_eq!(h.airbrake_str, "BRK");
        assert_eq!(h.gear_str, "GEA");
        assert_eq!(h.sep_str, "SEP↓-13 ");
        assert_eq!(h.maneuver_state_str, "G  2.6");
        assert_eq!(h.aoa_ratio.to_bits(), 0x3fd1_b4e8_1b4e_81b5); // 8.3/30
        assert_eq!(h.speed_bar_speed_ratio.to_bits(), 0x3fea_8f5c_28f5_c28f);
        assert_eq!(h.speed_bar_stall_ratio.to_bits(), 0x3fd2_8b30_84db_fe10);
        assert_eq!(h.speed_bar_unit_mach_limit_ratio, 0.9);
        assert_eq!(h.speed_bar_aileron_lock_ratio, 0.72);
        assert_eq!(h.speed_bar_rudder_lock_ratio, 0.66);
    }

    // ---- s5: 真机 spitfire FM (oracle s5_spitfire + spit_flds) ----
    #[test]
    fn s5_spitfire_real_fm_values() {
        let hud = MockHud { aoa_warn_ratio: 0.85, aoa_bar_warn_ratio: 0.95, ..Default::default() };
        let src = mk_src(610.0, 0.52, 3200.0, 4900.0, 5.1, 10.5, 9800.0, 0.0, 0.0, 0.0, 0.0,
            144.0, false, false, 0.0);
        let ev = mk_event(
            EventPayload::builder().build(),
            Some(mk_state(0.35, 100, 0, 55, 100, 350.0, 1.0, 1.0)),
            Some(mk_indic(-2.5, 15.0, -65535.0)),
        );
        let fm = spitfire_blkx();
        let h = calculate(Some(&ev), Some(&src), Some(&fm), &hud, &COLORS);

        assert_eq!(h.pitch.to_bits(), 0x4004_0000_0000_0000); // 2.5 = -(-2.5)
        assert_eq!(h.roll.to_bits(), 0xc02e_0000_0000_0000); // -15.0
        assert_eq!(h.slip.to_bits(), 0x3fd6_6666_6666_6666); // 0.35
        assert!(h.pitch_valid);
        assert_eq!(h.throttle, 100);
        assert_eq!(h.throttle_color, COLOR_WHITE); // 100 不 > 100
        // maneuverIndex = 1 - 3586.7…/(3586.7…+350)
        assert_eq!(h.maneuver_index.to_bits(), 0x3fb6_c29b_2566_fa68);
        // flapAllowAngle: ias=610 > row0[1]=290 → i=0 分支, 外推向负 → norm 0
        assert_eq!(h.flap_allow_angle, 0.0);
        assert!(h.warn_vne); // airbrake==100
        assert!(!h.warn_stall);
        assert!(!h.warn_altitude); // ralt invalid
        // getAoAHighVWing(0,0)=17.799… → available=16.799…
        assert_eq!(h.aoa_color, COLORS.color_num); // 16.8 >= 0.85*17.8
        assert_eq!(h.aoa_bar_color, COLORS.color_unit); // 16.8 < 0.95*17.8
        assert_eq!(h.aoa_ratio.to_bits(), 0x3fee_33c6_7784_272b);
        assert_eq!(h.energy_m.to_bits(), 0x408f_3fff_ffff_ffff);
        assert_eq!(h.speed_str, "SPD   610");
        assert_eq!(h.alt_str, "ALT  3200");
        assert_eq!(h.aoa_str, "α  1");
        assert_eq!(h.energy_str, "E 1000"); // 999.999… HALF_UP 进位
        assert_eq!(h.mechanization_str, "    BRKGEA");
        assert_eq!(h.flaps_wing_str, "");
        assert_eq!(h.sep_str, "SEP↑5   ");
        assert_eq!(h.maneuver_state_str, "L--:--"); // timeStr 缺省 "--:--"
        assert!(h.warn_configuration); // gear=55 ≠ 100
        // speedRatio=0 → currentLimit = getVNEVWing(0) = 875 → 144/875
        assert_eq!(h.speed_bar_stall_ratio.to_bits(), 0x3fc5_10ad_33c8_ff1f);
    }

    // ---- s6: mach 模式 + 标签禁用 + 失速警告 (oracle s6_mach_stall) ----
    #[test]
    fn s6_mach_mode_and_stall_warning() {
        let hud = MockHud {
            mach_mode: true,
            spd_label_off: true,
            alt_label_off: true,
            sep_label_off: true,
            always_radar: true,
            flap_bar: false,
            aoa_warn_ratio: 0.9,
            aoa_bar_warn_ratio: 0.7,
        };
        let src = mk_src(300.0, 0.82, 800.0, 120.0, -999.9, 359.9, 5200.0, 0.55, 0.3, 0.2, 0.44,
            155.0, true, false, 0.0);
        let payload = EventPayload::builder().map_grid("A1".to_string())
            .time_str(String::new()).build();
        let ev = mk_event(
            payload,
            Some(mk_state(-65535.0, 50, 100, 0, 0, 60.0, 30.5, 0.25)),
            Some(mk_indic(3.0, -65535.0, -65535.0)),
        );
        let fm = spitfire_blkx();
        let h = calculate(Some(&ev), Some(&src), Some(&fm), &hud, &COLORS);

        assert!(h.is_mach_mode);
        assert_eq!(h.pitch, -3.0);
        assert_eq!(h.roll, 0.0); // aviar=-65535 → 0
        assert_eq!(h.slip, 0.0); // AoS=-65535 → 默认
        assert_eq!(h.flaps, 100.0);
        assert_eq!(h.flap_allow_angle.to_bits(), 0x4040_aaaa_aaaa_aaaa); // ias=300 插值 33.33…
        assert_eq!(h.aoa, 30.5);
        assert!(h.warn_stall); // availableAoA = 17.8-30.5 < 0
        assert_eq!(h.aoa_ratio.to_bits(), 0xbfed_0000_0000_0000); // 负比率
        assert_eq!(h.aoa_color, COLORS.color_warning);
        assert_eq!(h.aoa_bar_color, COLORS.color_unit);
        assert!(h.warn_altitude); // 120 <= 500 且 valid
        assert_eq!(h.speed_str, "M 0.82");
        assert_eq!(h.alt_str, "R  120"); // alwaysRadar && raltValid
        assert_eq!(h.aoa_str, "α 31");
        assert_eq!(h.energy_str, "E  531");
        assert_eq!(h.mechanization_str, "F100"); // flapBar=false, brk/gear 空
        assert_eq!(h.flaps_wing_str, "F100");
        assert_eq!(h.sep_str, "↓-1000"); // sepPre 空 + 超宽不截断
        assert_eq!(h.maneuver_state_str, ""); // timeStr 空 & gLoad=0.25 在区间内
        assert_eq!(h.maneuver_index.to_bits(), 0x3f90_d91d_b002_b840);
        // currentLimit = 300/0.55 → 155/currentLimit
        assert_eq!(h.speed_bar_stall_ratio.to_bits(), 0x3fd2_2fc9_62fc_9630);
        assert!(!h.warn_vne);
        assert!(!h.warn_configuration);
    }

    // ---- s7: 手搓 vwing, ias 扫掠 getFlapAllowAngle 全分支 (oracle s7_vw_ias*) ----
    #[test]
    fn s7_vwing_flap_allow_angle_branches() {
        let hud = MockHud { aoa_warn_ratio: 0.85, aoa_bar_warn_ratio: 0.95, ..Default::default() };
        let vw = vwing_blkx();
        // (ias, 期望 flapAllowAngle 位级, 期望 warnVne)
        let cases: &[(f64, u64, bool)] = &[
            (0.0, 0x405f_4000_0000_0000, false),   // 125 (ias==0 短路)
            (150.0, 0x4029_0000_0000_0000, false), // 12.5 (i=num-1, 低于区间外推)
            (200.0, 0x4039_0000_0000_0000, false), // 25.0 (行边界不满足严格 >)
            (250.0, 0x4041_8000_0000_0000, false), // 35.0 (i=0 分支插值)
            (300.0, 0x4049_0000_0000_0000, false), // 50.0 (i=0 分支, 恰为 y1)
            (350.0, 0x4050_4000_0000_0000, false), // 65.0
            (700.0, 0x405f_4000_0000_0000, true),  // 125 (插值越上界钳位) + VNE 告警
        ];
        for &(ias, want_angle, want_vne) in cases {
            let src = mk_src(ias, 0.75, 4000.0, 600.0, 3.3, 45.0, 6000.0, 0.0, 0.0, 0.0, 0.0,
                120.0, false, true, 0.6);
            let ev = mk_event(
                EventPayload::builder().build(),
                Some(mk_state(0.1, 90, 0, 0, 0, 400.0, 4.0, 0.9)),
                Some(mk_indic(-1.0, 2.0, 0.6)),
            );
            let h = calculate(Some(&ev), Some(&src), Some(&vw), &hud, &COLORS);
            assert_eq!(h.flap_allow_angle.to_bits(), want_angle, "ias={ias}");
            assert_eq!(h.warn_vne, want_vne, "ias={ias}");
            // vwing=0.6 插值链 (oracle 恒定值)
            assert_eq!(h.maneuver_index.to_bits(), 0x3fbe_1e1e_1e1e_1e20, "ias={ias}");
            assert_eq!(h.aoa_ratio.to_bits(), 0x3fe8_ed9c_fe95_ec33, "ias={ias}");
            assert_eq!(h.speed_bar_stall_ratio.to_bits(), 0x3fc6_b8ce_0307_92ef, "ias={ias}");
            // flaps=0 且 isVWing && sIndic → W 分支
            assert_eq!(h.flaps_wing_str, "W 60", "ias={ias}");
            assert_eq!(h.mechanization_str, "W 60", "ias={ias}");
        }
    }

    // ---- s9: 降序行 — i-1 精确相等早退 + 越界钳位 (oracle s9_dw_ias*) ----
    #[test]
    fn s9_descending_rows_early_return_and_clamp() {
        let hud = MockHud { aoa_warn_ratio: 0.85, aoa_bar_warn_ratio: 0.95, ..Default::default() };
        let dw = descending_blkx();
        let mk_ev = || {
            let payload = EventPayload::builder().map_grid("B2".to_string())
                .time_str("01:02".to_string()).build();
            mk_event(
                payload,
                Some(mk_state(0.2, 105, 40, 30, 60, 520.0, 6.2, -0.8)),
                Some(mk_indic(2.0, -3.0, -65535.0)),
            )
        };
        let mk = |ias: f64| {
            mk_src(ias, 0.6, 2500.0, 700.0, -2.5, 270.0, 4100.0, 0.62, 0.4, 0.3, 0.5,
                130.5, false, false, 0.0)
        };

        // ias=640: i=0 分支, 外推 t=-1 → norm 0 (oracle s9_dw_ias640 全量)
        let h = calculate(Some(&mk_ev()), Some(&mk(640.0)), Some(&dw), &hud, &COLORS);
        assert_eq!(h.flap_allow_angle, 0.0);
        assert!(h.warn_vne); // 640 >= 650*0.95
        assert_eq!(h.aoa_ratio.to_bits(), 0x3fe4_ba2e_8ba2_e8bb);
        assert_eq!(h.maneuver_index.to_bits(), 0x3fc6_0a2c_1458_28b0);
        assert_eq!(h.speed_bar_stall_ratio.to_bits(), 0x3fc0_2e97_8d4f_df3b);
        assert_eq!(h.sep_str, "SEP↓-3  ");
        assert_eq!(h.maneuver_state_str, "G -0.8"); // gLoad=-0.8 < -0.5
        assert_eq!(h.energy_str, "E  418");
        assert_eq!(h.aoa_color, COLORS.color_warning);
        assert_eq!(h.aoa_bar_color, COLORS.color_unit);
        assert_eq!(h.mechanization_str, "F 40BRKGEA");
        assert_eq!(h.throttle_color, COLOR_RED); // 105 > 100
        assert_eq!(h.alt_str, "ALT  2500");

        // ias=300: i 扫到 num-1=2, ias == speeds[1][1]=300 → 早退 return 0.5*100
        let h = calculate(Some(&mk_ev()), Some(&mk(300.0)), Some(&dw), &hud, &COLORS);
        assert_eq!(h.flap_allow_angle, 50.0, "行值精确相等早退");
        assert_eq!(h.speed_str, "SPD   300");
        // vneMach 未赋值 (=0) → mach 0.6 >= 0*0.95 恒真 (oracle 同: warnVne=true)
        assert!(h.warn_vne);

        // ias=150: i=2 分支外推 t=125 恰触上界 → norm 保持 125 (oracle)
        let h = calculate(Some(&mk_ev()), Some(&mk(150.0)), Some(&dw), &hud, &COLORS);
        assert_eq!(h.flap_allow_angle, 125.0);
    }

    // ---- s10: 无效 blkx → flapAllowAngle=125 + else 分支 (oracle s10_invalid_fm) ----
    #[test]
    fn s10_invalid_fm_short_circuits() {
        let hud = MockHud { aoa_warn_ratio: 0.85, aoa_bar_warn_ratio: 0.95, ..Default::default() };
        let mut bad = descending_blkx();
        bad.valid = false;
        let src = mk_src(250.0, 0.5, 1000.0, 300.0, 1.0, 90.0, 2000.0, 0.0, 0.0, 0.0, 0.0,
            110.0, true, false, 0.0);
        let ev = mk_event(
            EventPayload::builder().build(),
            Some(mk_state(0.0, 80, 20, 10, 0, 100.0, 2.0, 1.5)),
            Some(mk_indic(0.0, 0.0, 0.0)),
        );
        let h = calculate(Some(&ev), Some(&src), Some(&bad), &hud, &COLORS);

        assert_eq!(h.flap_allow_angle, 125.0);
        assert_eq!(h.maneuver_index, 0.0);
        assert_eq!(h.aoa_color, COLORS.color_num);
        assert_eq!(h.aoa_bar_color, COLORS.color_num);
        assert_eq!(h.aoa_ratio.to_bits(), 0x3fb1_1111_1111_1111); // 2.0/30
        assert_eq!(h.throttle_color, COLOR_WHITE);
        assert_eq!(h.speed_str, "SPD   250");
        assert_eq!(h.alt_str, "ALTR  300");
        assert_eq!(h.aoa_str, "α  2");
        assert_eq!(h.energy_str, "E  204");
        assert_eq!(h.mechanization_str, "F 20GEA");
        assert_eq!(h.flaps_wing_str, "F 20");
        assert_eq!(h.sep_str, "SEP↑1   ");
        assert_eq!(h.maneuver_state_str, "L--:--");
        // currentLimit 保持 1.0 (blkx 无效不进 fallback) → 110/1.0
        assert_eq!(h.speed_bar_stall_ratio.to_bits(), 0x405b_8000_0000_0000);
    }

    // ---- fmt: String.format 语义电池 (oracle fmt|* 行, HALF_UP/宽度/负零) ----
    #[test]
    fn java8_oracle_format_battery() {
        // M%5.2f
        let m52 = |v: f64| format!("M{}", pad_width(java_f(v, 2), 5, false));
        assert_eq!(m52(0.82), "M 0.82");
        assert_eq!(m52(0.851), "M 0.85");
        assert_eq!(m52(0.855), "M 0.86"); // HALF_UP on 最短表示 0.855
        assert_eq!(m52(1.0), "M 1.00");
        assert_eq!(m52(12.345), "M12.35"); // 超宽不截断
        assert_eq!(m52(2.675), "M 2.68"); // Rust {:.2} 会给 2.67
        assert_eq!(m52(-0.004), "M-0.00");
        assert_eq!(m52(0.0), "M 0.00");
        assert_eq!(m52(-0.0), "M-0.00");
        assert_eq!(m52(-0.4), "M-0.40");
        assert_eq!(m52(0.45), "M 0.45");
        assert_eq!(m52(f64::NAN), "M  NaN");
        assert_eq!(m52(f64::INFINITY), "MInfinity");

        // R%5.0f / %6.0f / α%3.0f / E%5.0f
        assert_eq!(format!("R{}", pad_width(java_f(245.7, 0), 5, false)), "R  246");
        assert_eq!(format!("R{}", pad_width(java_f(0.0, 0), 5, false)), "R    0");
        assert_eq!(format!("R{}", pad_width(java_f(-0.5, 0), 5, false)), "R   -1");
        assert_eq!(format!("R{}", pad_width(java_f(999.5, 0), 5, false)), "R 1000");
        assert_eq!(format!("{}", pad_width(java_f(5300.5, 0), 6, false)), "  5301");
        assert_eq!(format!("{}", pad_width(java_f(-0.4, 0), 6, false)), "    -0");
        assert_eq!(format!("{}", pad_width(java_f(0.82, 0), 6, false)), "     1");
        assert_eq!(format!("α{}", pad_width(java_f(8.3, 0), 3, false)), "α  8");
        assert_eq!(format!("α{}", pad_width(java_f(30.5, 0), 3, false)), "α 31");
        assert_eq!(format!("α{}", pad_width(java_f(100.0, 0), 3, false)), "α100");
        assert_eq!(format!("α{}", pad_width(java_f(-0.04, 0), 3, false)), "α -0");
        assert_eq!(format!("E{}", pad_width(java_f(1521.7346938775509, 0), 5, false)), "E 1522");
        assert_eq!(format!("E{}", pad_width(java_f(999.9999999999999, 0), 5, false)), "E 1000");
        assert_eq!(format!("E{}", pad_width(java_f(530.6122448979592, 0), 5, false)), "E  531");
        assert_eq!(format!("E{}", pad_width(java_f(204.0816326530612, 0), 5, false)), "E  204");
        assert_eq!(format!("E{}", pad_width(java_f(418.3673469387755, 0), 5, false)), "E  418");

        // %-4.0f (左对齐)
        assert_eq!(pad_width(java_f(5.1, 0), 4, true), "5   ");
        assert_eq!(pad_width(java_f(-13.2, 0), 4, true), "-13 ");
        assert_eq!(pad_width(java_f(-999.9, 0), 4, true), "-1000"); // 超宽不截断
        assert_eq!(pad_width(java_f(-2.5, 0), 4, true), "-3  ");
        assert_eq!(pad_width(java_f(0.82, 0), 4, true), "1   ");
        assert_eq!(pad_width(java_f(f64::NAN, 0), 4, true), "NaN ");

        // G%5.1f
        assert_eq!(format!("G{}", pad_width(java_f(2.6, 1), 5, false)), "G  2.6");
        assert_eq!(format!("G{}", pad_width(java_f(-0.6, 1), 5, false)), "G -0.6");
        assert_eq!(format!("G{}", pad_width(java_f(-0.8, 1), 5, false)), "G -0.8");
        assert_eq!(format!("G{}", pad_width(java_f(-0.04, 1), 5, false)), "G -0.0");
        assert_eq!(format!("G{}", pad_width(java_f(0.0, 1), 5, false)), "G  0.0");
        assert_eq!(format!("G{}", pad_width(java_f(0.45, 1), 5, false)), "G  0.5");
        assert_eq!(format!("G{}", pad_width(java_f(f64::NAN, 1), 5, false)), "G  NaN");

        // F%3.0f / W%3.0f
        assert_eq!(format!("F{}", pad_width(java_f(50.0, 0), 3, false)), "F 50");
        assert_eq!(format!("F{}", pad_width(java_f(100.0, 0), 3, false)), "F100");
        assert_eq!(format!("W{}", pad_width(java_f(55.00000000000001, 0), 3, false)), "W 55");
        assert_eq!(format!("W{}", pad_width(java_f(0.6_f64 * 100.0, 0), 3, false)), "W 60");
        assert_eq!(format!("W{}", pad_width(java_f(-0.6, 0), 3, false)), "W -1");
        assert_eq!(format!("W{}", pad_width(java_f(f64::INFINITY, 0), 3, false)), "WInfinity");

        // %6d ((int) 强转语义: 截断/饱和/NaN→0, Java (int) ↔ Rust as i32 同)
        let d6 = |v: f64| pad_width((v as i32).to_string(), 6, false);
        assert_eq!(d6(412.9), "   412");
        assert_eq!(d6(-412.9), "  -412");
        assert_eq!(d6(1e9), "1000000000");
        assert_eq!(d6(-1e9), "-1000000000");
        assert_eq!(d6(1e300), "2147483647"); // Java (int) 饱和 ↔ as i32 饱和
        assert_eq!(d6(f64::NAN), "     0");
        assert_eq!(pad_width(412i32.to_string(), 6, false), "   412");
        assert_eq!(pad_width((-12345i32).to_string(), 6, false), "-12345");

        // %4s 空串
        assert_eq!(format!("{}{}{}", pad_width(String::new(), 4, false), "BRK", "GEA"), "    BRKGEA");
        assert_eq!(format!("{}{}{}", pad_width(String::new(), 4, false), "", ""), "    ");
    }

    // ---- getStringWidth 三重守卫 + 度量委托 ----
    #[test]
    fn get_string_width_guards_and_delegation() {
        struct FakeFont;
        // text null → 0 (度量闭包不被调用)
        assert_eq!(get_string_width(None, Some(&FakeFont), |_, _| panic!("不应度量")), 0);
        // text 空 → 0
        assert_eq!(get_string_width(Some(""), Some(&FakeFont), |_, _| panic!("不应度量")), 0);
        // font null → 0
        let none_font: Option<&FakeFont> = None;
        assert_eq!(get_string_width(Some("ABC"), none_font, |_, _| panic!("不应度量")), 0);
        // 正常: 委托调用方度量 (font + 原文直传)
        assert_eq!(get_string_width(Some("ABC"), Some(&FakeFont), |_, t| t.len() as i32 * 7), 21);
    }

    // ---- HudColors 注入缺省值 = Application.java 字段初始化器 (L105-111) ----
    #[test]
    fn hud_colors_application_defaults_match_java_initializers() {
        let c = HudColors::application_defaults();
        // colorWarning = new Color(216, 33, 13, 100)
        assert_eq!(c.color_warning, [216, 33, 13, 100]);
        // colorNum = new Color(27, 255, 128, 240)
        assert_eq!(c.color_num, [27, 255, 128, 240]);
        // colorUnit = new Color(166, 166, 166, 220)
        assert_eq!(c.color_unit, [166, 166, 166, 220]);
    }
}

