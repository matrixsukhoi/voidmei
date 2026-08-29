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

/// W7: var_value 桥取值 (NaN→0, 对齐原 getter map_or(0.0) 零值帧语义)
fn v(s: &dyn TelemetrySource, name: &str) -> f64 {
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

    // 公式驱动 (阶段 2 A 级外置): 公式结果优先, 公式缺失/禁用/NaN 回退原计算
    // (迁移期双保险 — 用户删改内置公式时 HUD 数据不断链)
    b.energy_m = source
        .get_formula_value("energy_m")
        .unwrap_or_else(|| v(source, "energy_jkg") / g);

    b.is_mach_mode = settings.draw_hud_mach();
    b.is_gear_down = b.gear > 0.0;
    b.is_flaps_down = b.flaps > 0.0;
    b.is_airbrake_active = b.airbrake > 0.0;

    // --- Warning Logic (W4: 警告布尔公式接管优先, 判定式原样进 formulas.cfg) ---
    let mut warn_vne = source
        .get_formula_value("warn_vne")
        .map(|v| v != 0.0)
        .unwrap_or(false);
    let warn_vne_fallback = !warn_vne; // 公式缺失时走原判定
    if warn_vne_fallback && b.is_airbrake_active && b.airbrake == 100.0 {
        warn_vne = true;
    }

    // Java: if (blkx != null && blkx.valid)
    let valid_blkx = blkx.filter(|x| x.valid);
    if let Some(blkx) = valid_blkx {
        // User requested formula: 1 - (nfweight / (nfweight + fuel))
        let nfweight = blkx.nofuelweight;
        let current_fuel = s_state.map_or(0.0, |s| s.mfuel);

        // 公式驱动 (阶段 2 A 级外置, 公式式含同款零除守卫; fm.* 未接线时回退原式)
        b.maneuver_index = source.get_formula_value("maneuver_index").unwrap_or_else(|| {
            // Check for valid weights to avoid division by zero
            if nfweight > 0.0 && (nfweight + current_fuel) > 0.0 {
                1.0 - (nfweight / (nfweight + current_fuel))
            } else {
                0.0
            }
        });

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
        if blkx.is_v_wing.unwrap() && s_indic.is_some() {
            vwing = s_indic.unwrap().wsweep_indicator;
        }

        // Dynamic Vne calculation (公式接管时跳过; 位级注记:
        // 第二项 Java 是 `* 0.95f` 提升 = 0.94999998807907104, 不得与 0.95 合并 §2.12)
        if warn_vne_fallback {
            if b.ias >= blkx.get_vne_v_wing(vwing) * 0.95
                || b.mach >= blkx.get_mne_v_wing(vwing) * (0.95f32 as f64)
            {
                warn_vne = true;
            }
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

        // W4: 公式接管优先 (warn_stall), 回退原判定
        b.warn_stall = match source.get_formula_value("warn_stall") {
            Some(v) => v != 0.0,
            None => available_aoa <= 0.0,
        };
    } else {
        b.maneuver_index = 0.0;
        b.aoa_color = colors.color_num;
        b.aoa_bar_color = colors.color_num;
        b.aoa_ratio = b.aoa / 30.0;
    }
    b.warn_vne = warn_vne;

    // Warnings
    let radio_alt = b.radio_altitude;
    let radio_alt_valid = v(source, "radio_altitude_valid") != 0.0;
    let always_radar = settings.always_show_radar_altitude();

    // Low altitude warning flag - always based on <=500m threshold
    // (W4: 公式接管优先)
    b.warn_altitude = match source.get_formula_value("warn_altitude") {
        Some(v) => v != 0.0,
        None => radio_alt_valid && radio_alt <= 500.0,
    };

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
    b.speed_bar_speed_ratio = v(source, "speed_limit_ratio");
    b.speed_bar_aileron_lock_ratio = v(source, "aileron_lock_ratio");
    b.speed_bar_rudder_lock_ratio = v(source, "rudder_lock_ratio");
    b.speed_bar_unit_mach_limit_ratio = v(source, "unit_mach_limit_ratio");

    // Calculate Stall Ratio
    let mut current_limit = 1.0;
    // If we have valid speed ratio, derive the current limit (VNE or MachLimit_IAS)
    if b.speed_bar_speed_ratio > 0.0001 && b.ias > 1.0 {
        current_limit = b.ias / b.speed_bar_speed_ratio;
    } else if let Some(blkx) = valid_blkx {
        // Fallback to static VNE
        let mut vwing = 0.0;
        if v(source, "wing_sweep_valid") != 0.0 {
            vwing = v(source, "wing_sweep");
        }
        current_limit = blkx.get_vne_v_wing(vwing);
    }

    let stall_speed = v(source, "stall_speed");
    if current_limit > 0.1 {
        b.speed_bar_stall_ratio = stall_speed / current_limit;
    } else {
        b.speed_bar_stall_ratio = 0.0;
    }

    b.build()
}

/// 对应 Java `private static double getFlapAllowAngle(double ias, boolean
/// isDowningFlap, Blkx blkx)`。
/// **双胞胎合一** (设计 §7): Service 侧 methods_engine 曾有一份逐行同构的
/// Service 版 (Java Service.java L1108-1145 与 HUDCalculator.java:300-341
/// 本就是两份拷贝), 现统一走本实现 (含 Java 的 `!blkx.valid → 125` 防御分支;
/// 生产链两调用方的 blkx 均来自 READY 句柄, valid 恒真, 该分支不可达 —
/// Service 版测试 mock 需 valid=true 对齐生产形态)。
/// PORT: 形参 isDowningFlap 在 Java 方法体内未使用 — 签名保真, `_` 前缀消告警。
pub fn get_flap_allow_angle(ias: f64, _is_downing_flap: bool, blkx: Option<&Blkx>) -> f64 {
    // Java: if (ias == 0 || blkx == null || !blkx.valid) return 125;
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

/// 对应 Java `public double getFlapAllowSpeed(int flapPercent, Boolean isDowningFlap, FMHandle fm)`
/// (Service.java L1354-1427) — 当前襟翼开度下的允许速度。
/// **双胞胎合一** (设计 §7): 与 getFlap_allow_angle 同族, Service 版
/// (methods_engine) 曾有逐行同构拷贝, 统一走本实现; 签名对齐 angle 版
/// 收 Option<&Blkx>。flapPercent==0/无 FM → f64::MAX (Java Double.MAX_VALUE,
/// 与 resetvaria 侧 Float.MAX_VALUE 刻意不同, 保真)。
pub fn get_flap_allow_speed(flap_percent: i32, is_downing_flap: bool, blkx: Option<&Blkx>) -> f64 {
    if flap_percent == 0 {
        return f64::MAX;
    }
    let blkx = match blkx {
        None => return f64::MAX,
        Some(b) => b,
    };
    let flaps_destruction_num = blkx.flaps_destruction_num;
    let table = blkx.flaps_destruction_ind_speed.as_ref().unwrap();
    let mut i: i32 = 0;
    while i < flaps_destruction_num - 1 {
        // Java: flapPercent < ...[i][0] * 100.0f — int 提升 double (§2.12)
        if (flap_percent as f64) < table[i as usize][0] * 100.0 {
            break;
        }
        i += 1;
    }
    let i = i - 1;
    if i == -1 {
        // 下襟翼时直接越级使用下一级 (num=0 畸形 FM 域内是活条件, reader 回退全 miss)
        if is_downing_flap && flaps_destruction_num >= 1 {
            return table[0][1];
        }
        f64::MAX
    } else {
        if (flap_percent as f64) == table[i as usize][0] * 100.0 {
            return table[i as usize][1];
        }
        let x0 = table[i as usize][0] * 100.0;
        let y0 = table[i as usize][1];
        let x1 = table[(i + 1) as usize][0] * 100.0;
        let y1 = table[(i + 1) as usize][1];
        let k = calc_k(x0, y0, x1, y1);
        y0 + (flap_percent as f64 - x0) * k
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
pub(crate) fn java_f(d: f64, prec: usize) -> String {
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
mod tests;
