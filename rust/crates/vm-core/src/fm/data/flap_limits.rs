//! 襟翼限速/限角查表原语 (波8 自 derived/hud_calculator 迁入, 循环拆解:
//! formula::eval 曾调 hud_calculator 的本组函数, 而 hud_calculator 又 impl
//! formula::registry::FormulaView — formula ↔ derived 循环边在此销号;
//! 函数本体是 FmData.flaps_destruction 表的查表+线性插值, 属 FM 数据原语,
//! hud_calculator 的 calculate() 从未调用它们, 逐字搬迁零行为差)。

use super::FmData;

/// 对应 Java `private static double getFlapAllowAngle(double ias, boolean
/// isDowningFlap, Blkx blkx)`。
/// **双胞胎合一** (设计 §7): Service 侧 methods_engine 曾有一份逐行同构的
/// Service 版 (Java 侧本就是两份逐行拷贝),
/// 现统一走本实现 (含 Java 的 `!blkx.valid → 125` 防御分支;
/// 生产链两调用方的 blkx 均来自 READY 句柄, valid 恒真, 该分支不可达 —
/// Service 版测试 mock 需 valid=true 对齐生产形态)。
/// 形参 isDowningFlap 在 Java 方法体内未使用 — 签名保真, `_` 前缀消告警。
pub fn get_flap_allow_angle(ias: f64, _is_downing_flap: bool, fmdata: Option<&FmData>) -> f64 {
    if ias == 0.0 {
        return 125.0;
    }
    let fmdata = match fmdata {
        None => return 125.0,
        Some(b) => b,
    };
    if !fmdata.valid {
        return 125.0;
    }

    // Java 直接解引用 FlapsDestructionIndSpeed (doLoad=false 构造的 blkx 上
    // 为 null → NPE) — unwrap panic 复刻同一硬失败。⚠ 过渡期同 is_v_wing:
    // Blkx::parse 的 valid=true 不保证本字段 Some (getload 该段未译),
    // 接线 service_loop 前须等 getload 波次落地。
    let speeds = fmdata.flaps_destruction_ind_speed.as_ref().unwrap();

    let mut i: i32 = 0;
    while i < fmdata.flaps_destruction_num - 1 {
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
    // Java `* 100.0f` (float 字面量提升 double) — 100 精确可表示, 值同 100.0
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
/// (Java Service 版) — 当前襟翼开度下的允许速度。
/// **双胞胎合一** (设计 §7): 与 getFlap_allow_angle 同族, Service 版
/// (methods_engine) 曾有逐行同构拷贝, 统一走本实现; 签名对齐 angle 版
/// 收 Option<&Blkx>。flapPercent==0/无 FM → f64::MAX (Java Double.MAX_VALUE,
/// 与 resetvaria 侧 Float.MAX_VALUE 刻意不同, 保真)。
pub fn get_flap_allow_speed(
    flap_percent: i32,
    is_downing_flap: bool,
    fmdata: Option<&FmData>,
) -> f64 {
    if flap_percent == 0 {
        return f64::MAX;
    }
    let fmdata = match fmdata {
        None => return f64::MAX,
        Some(b) => b,
    };
    let flaps_destruction_num = fmdata.flaps_destruction_num;
    let table = fmdata.flaps_destruction_ind_speed.as_ref().unwrap();
    let mut i: i32 = 0;
    while i < flaps_destruction_num - 1 {
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
