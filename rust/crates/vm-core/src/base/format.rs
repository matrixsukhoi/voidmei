//! FastNumberFormatter 的 Rust 移植 (src/ui/util/FastNumberFormatter.java)
//! 语义逐行对齐: 半舍入 (floor(x+0.5)), 负零抑制, NaN→"N/A", TIME_MM_SS
//!
//! java_printf 子模块: Java `String.format` printf 引擎 (Lang 模板域唯一真相)。

mod java_printf;

pub use java_printf::{java_format_f, java_string_format, FmtArg};

/// Java Math.round(double) = floor(x + 0.5)
pub fn java_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// Java Math.round(float)→int 窄版 (重构波3: 全仓 12 处私有副本收敛于此)
pub fn java_round_f32(x: f32) -> i32 {
    (x + 0.5).floor() as i32
}

/// Java Math.round(double) 经 (int) 窄化的常用形态
pub fn java_round_f64(x: f64) -> i32 {
    (x + 0.5).floor() as i32
}

/// 数值格式化, 对应 Java format(double, char[], int)
/// precision: 0-5 位小数
pub fn format(value: f64, precision: u8) -> String {
    if value.is_nan() {
        return "N/A".to_string();
    }

    let mut out = String::new();
    let mut value = value;
    if value < 0.0 {
        // 负零抑制: 绝对值不足半个最低位时不输出负号
        let mut threshold = 0.5;
        for _ in 0..precision {
            threshold /= 10.0;
        }
        if value <= -threshold {
            out.push('-');
        }
        value = -value;
    }

    let mut scale = 1.0f64;
    for _ in 0..precision {
        scale *= 10.0;
    }

    let mut integral = value as i64;
    let fractional = value - integral as f64;
    let mut scaled_fraction = java_round(fractional * scale);
    if scaled_fraction as f64 >= scale {
        integral += 1;
        scaled_fraction = 0;
    }

    if integral == 0 {
        out.push('0');
    } else {
        out.push_str(&integral.to_string());
    }

    if precision > 0 {
        out.push('.');
        // 补零到 precision 位
        let s = scaled_fraction.to_string();
        for _ in s.len()..precision as usize {
            out.push('0');
        }
        out.push_str(&s);
    }
    out
}

/// MM'SS 时间格式化, 对应 Java formatTime(double, char[])
/// (power_info 油温耐热时间使用)
pub fn format_time(value: f64) -> String {
    if value.is_nan() || value < 0.0 {
        return "--'--".to_string();
    }
    let total_seconds = value as i32;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    let mut out = String::new();
    if minutes < 10 {
        out.push_str(&format!("0{minutes}"));
    } else if minutes < 100 {
        out.push_str(&minutes.to_string());
    } else if minutes < 1000 {
        out.push_str(&minutes.to_string()); // 3 位
    } else {
        out.push_str("999"); // 溢出封顶
    }
    out.push('\'');
    out.push_str(&format!("{:02}", seconds));
    out
}

/// Java `String.format("%N.Mf", d)` 的数值段 (不含宽度): 对**最短往返十进制**
/// HALF_UP。全库唯一真相 (原 config_loader / flight_analyzer 的算法级私有副本
/// java_format_f4/java_format_f1 已收割于此, 波19; Java 8 oracle 实证,
/// 本模块 build/oracle_hud 全格式串对拍):
/// - 2.675 → "2.68" (Rust `{:.2}` 会给 "2.67");
/// - -0.4 → "-0" / -0.04 → "-0.0" (舍入到零仍保留负号);
/// - NaN/Infinity 原样 ("NaN"/"Infinity"/"-Infinity");
/// - `exp10 > 25` 是纯实现切点, 非语义边界: else 支路的 scaled 定点累加在 u128
///   内, 10^308 量级会溢出; 该域最短表示位数 n ≤ 17 < keep, 判定位恒 0, 无舍入,
///   走 digits + 补零的字符串路径;
/// - JDK-4511638 已知分歧: Java 8 旧 dtoa
///   在大值域 (~1e17 起) 偶发非最短 toString, 而 %f 按**自身 toString 的数字**
///   展开 — 1e23 → "9.999999999999999E22" → "99999999999999990000000", 既非精确
///   二进制 (...91611392) 也非最短展开; Rust `{:e}` 给真最短 "1e23" → 本实现输出
///   "100000000000000000000000"。HUD 值域 (速度/高度/能量 < 10^7) 距该域不可达
///   (Java 8 oracle fuzz 35k 例仅 1e23 一例分歧)。
pub fn java_f(d: f64, prec: usize) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
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
pub fn pad_width(mut s: String, width: usize, left_align: bool) -> String {
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

/// Java `String.format("%0Nd", v)` (long 域): '0' 标志的零填充, 符号感知
/// (负号后补零, 宽度含符号位; 已超宽不截断)。
pub fn java_d0(v: i64, width: usize) -> String {
    let s = v.to_string();
    if s.len() >= width {
        return s;
    }
    let (sign, digits) = match s.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", s.as_str()),
    };
    let fill = "0".repeat(width - s.len());
    format!("{sign}{fill}{digits}")
}

/// Java `String.format("%+.Nf", d)` 的 '+' 标志: 非负值强制带 '+' (含 +0.0)。
/// NaN 不加号 (Java 8 oracle 实测 "+NaN" 不存在, 恒 "NaN")。
pub fn java_f_plus(d: f64, prec: usize) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 {
            "+Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if d.is_sign_negative() {
        java_f(d, prec)
    } else {
        format!("+{}", java_f(d, prec))
    }
}

/// Java `String.format("%.0f", d)` 的整数化 (重构波13 上收, 原 vm-overlay
/// FlapAngleBar::fmt_pct3 / CompassGauge::fmt_heading3 的共同内核)。
/// 舍入 = 精确二进制小数 HALF_UP (`m − floor(m) ≥ 0.5`, m=|d| — 不能写
/// v+0.5: 0.49999999999999994+0.5 会进到 1.0, Java oracle 输出 "0");
/// 负零与舍到零的负数保 '-'; NaN/±∞ 输出常量 "NaN"/"Infinity"/"-Infinity"。
/// 与 [`java_f`] 的刻意差异 (两脉各有 oracle 钉死的消费域, 不可互换):
/// 舍入结果 ≥ 2^63 时按**精确十进制**展开 (`{:.0}`; org.json 畸形遥测可达,
/// as i64 饱和串 9223372036854775807 是错误输出), [`java_f`] 则按最短往返
/// 表示展开 — 该巨值域属 Java 8 旧 dtoa 已知未决域, 两模型输出不同。
/// 消费方 (vm-overlay "%3.0f" 家族) 配 [`pad_width`] 组合右对齐。
pub fn java_f0_exact(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    // PORT: Formatter 对 ±∞ 输出常量 (org.json "1e999"→inf 可达)
    if d.is_infinite() {
        return if d > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    // 负号判定含 -0.0 (Java Formatter 对负零输出 "-0", v < 0.0 对 -0.0 为 false)
    let neg = d < 0.0 || (d == 0.0 && d.is_sign_negative());
    let m = d.abs();
    let f = m.floor();
    let r = if m - f >= 0.5 { f + 1.0 } else { f };
    // r ≥ 2^63 超 i64 域, 按完整十进制展开 (此域 ULP≥2048, m 必为整值, 无舍入分歧)
    let mut s = if r >= 9_223_372_036_854_775_808.0 {
        format!("{:.0}", r)
    } else {
        format!("{}", r as i64)
    };
    if neg {
        s.insert(0, '-');
    }
    s
}

#[cfg(test)]
mod tests;
