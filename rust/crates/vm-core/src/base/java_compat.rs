//! JDK 标准库语义复刻的唯一真相模块 (全库唯一)。
//!
//! Java `String.trim` / `Boolean.parseBoolean` / `Integer.parseInt` /
//! `Double.toString` / `Float.toString` / `System.currentTimeMillis` 的
//! 语义复刻收敛点 — 此前散落在 config_loader / config_manager /
//! configuration_service / comparison rules / flight_analyzer / flight_log /
//! map_service / fm_manager / focus_monitor 的私有同构副本已全部收编于此;
//! 其他 crate 的副本由后续阶段切换到本模块。

use std::time::{SystemTime, UNIX_EPOCH};

/// Java `String.trim()`: 剥首尾所有 `<= U+0020` 的字符 (含 \n/\r/\t, 不含 NBSP)
/// — 与 Rust `str::trim` (Unicode White_Space, 会剥 U+3000 等) 不同。
/// 此类字符在 UTF-8 中均为单字节, 按字符裁剪与按字节裁剪等价。
pub fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
}

/// Java `Boolean.parseBoolean(String)` = equalsIgnoreCase("true") — 非 "true" 一律 false。
pub fn java_parse_boolean(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

/// Java `Integer.parseInt(String)` (radix 10) 复刻:
/// 可选 +/-, 至少一位数字, 溢出/空/非法 → Err (= NumberFormatException)。
/// PORT: Java Character.digit 接受 Unicode Nd 数字 (如 '٥'); parseInt 无 trim —
/// 域内 cfg 值为 ASCII, §2.15 (catch 吞异常给默认值由调用方 unwrap_or 完成)。
// Err 载荷恒 () — NumberFormatException 无消费面, 调用方只区分成败
#[allow(clippy::result_unit_err)]
pub fn java_parse_int(s: &str) -> Result<i32, ()> {
    let b = s.as_bytes();
    let (neg, digits) = match b.first() {
        Some(b'-') => (true, &b[1..]),
        Some(b'+') => (false, &b[1..]),
        _ => (false, b),
    };
    if digits.is_empty() {
        return Err(());
    }
    let mut acc: i64 = 0;
    for &d in digits {
        if !d.is_ascii_digit() {
            return Err(());
        }
        acc = acc * 10 + i64::from(d - b'0');
        if acc > i32::MAX as i64 + 1 {
            return Err(()); // 溢出 — Java 抛 NumberFormatException (§2.2 静默回绕不适用: parseInt 是解析不是运算)
        }
    }
    if neg {
        acc = -acc;
    }
    if !(i32::MIN as i64..=i32::MAX as i64).contains(&acc) {
        return Err(());
    }
    Ok(acc as i32)
}

/// [`java_parse_int`] 的 catch-回退形态: Java
/// `try { Integer.parseInt(s) } catch { default }` 惯用法的直译 (供 vm-app 等消费)。
pub fn java_parse_int_or(s: &str, default: i32) -> i32 {
    java_parse_int(s).unwrap_or(default)
}

/// Java `Double.toString(double)` 一比一复刻 (getStr/String.valueOf(Double) 与
/// saveConfig serializeAtom 的 Double 分支共用):
/// - 10^-3 ≤ |d| < 10^7 → 十进制平原式, 恒至少一位小数 ("1.0");
/// - 否则科学计数 "D.DDDE±x" ('E' 后仅负指数带 '-', 正指数无 '+');
/// - 最短可区分数字串; NaN/±0/±Inf 特判。
///
/// PORT: 数字串取 Rust `{:e}` 最短往返表示, 与 Java FloatingDecimal 在
/// JDK-4511638 域 (极罕见多位尾数, 如 1e23 Java 给 "9.999999999999999E22")
/// 外逐位一致 — cfg/遥测值域 oracle 对拍无差异 (分歧已由
/// config_loader/configuration_service/flight_log 三处测试固化)。
pub fn java_double_to_string(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0".to_string() } else { "0.0".to_string() };
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = d.is_sign_negative();
    let a = d.abs();
    // "{:e}" → "D.DDDe±n"; a > 0 有限, 恒此形态 (最短往返数字, 无尾随零)
    let sci = format!("{:e}", a);
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let mut s = String::new();
    if (-3..=6).contains(&exp10) {
        // 平原式
        if exp10 >= 0 {
            let ip = exp10 as usize + 1; // 整数部分位数
            if digits.len() > ip {
                s.push_str(&digits[..ip]);
                s.push('.');
                s.push_str(&digits[ip..]);
            } else {
                s.push_str(&digits);
                s.push_str(&"0".repeat(ip - digits.len()));
                s.push_str(".0"); // 恒至少一位小数
            }
        } else {
            s.push_str("0.");
            s.push_str(&"0".repeat((-exp10 - 1) as usize));
            s.push_str(&digits);
        }
    } else {
        // 科学计数
        s.push_str(&digits[..1]);
        s.push('.');
        if digits.len() > 1 {
            s.push_str(&digits[1..]);
        } else {
            s.push('0');
        }
        s.push('E');
        s.push_str(&exp10.to_string());
    }
    if neg {
        s.insert(0, '-');
    }
    s
}

/// Java 8 `Float.toString(float)` 一比一 (analyze 通知里 `(int)X / 10.0f` 与
/// flight_log 行格式保真共用) — [`java_double_to_string`] 的 f32 同构:
/// 10^-3 ≤ |f| < 10^7 → 十进制平原式恒至少一位小数 ("12.0"); 否则 "D.DDDE±x"
/// ('E' 后仅负指数带 '-'); 最短可区分数字串; NaN/±0/±Inf 特判。
/// PORT: 数字串取 Rust `{:e}` 最短往返表示 — 与 Java FloatingDecimal 在
/// JDK-4511638 域外逐位一致 (flight_analyzer/flight_log 的 oracle 测试固化)。
pub fn java_float_to_string(f: f32) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0.0".to_string() } else { "0.0".to_string() };
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = f.is_sign_negative();
    let a = f.abs();
    // "{:e}" → "D.DDDe±n"; a > 0 有限恒此形态 (最短往返数字, 无尾随零)
    let sci = format!("{:e}", a);
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let mut s = String::new();
    if (-3..=6).contains(&exp10) {
        // 平原式
        if exp10 >= 0 {
            let ip = exp10 as usize + 1; // 整数部分位数
            if digits.len() > ip {
                s.push_str(&digits[..ip]);
                s.push('.');
                s.push_str(&digits[ip..]);
            } else {
                s.push_str(&digits);
                s.push_str(&"0".repeat(ip - digits.len()));
                s.push_str(".0"); // 恒至少一位小数
            }
        } else {
            s.push_str("0.");
            s.push_str(&"0".repeat((-exp10 - 1) as usize));
            s.push_str(&digits);
        }
    } else {
        // 科学计数
        s.push_str(&digits[..1]);
        s.push('.');
        if digits.len() > 1 {
            s.push_str(&digits[1..]);
        } else {
            s.push('0');
        }
        s.push('E');
        s.push_str(&exp10.to_string());
    }
    if neg {
        s.insert(0, '-');
    }
    s
}

/// Java `System.currentTimeMillis()`: SystemTime → as_millis u128 → as i64 截断;
/// 时钟早于 epoch 时 Java 可得负值而 duration_since 报错 → 取 0。
/// 时间戳差值域 (epoch 毫秒) 远离 i64 溢出, 普通减法即可 (§2.2 无涉)。
pub fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
