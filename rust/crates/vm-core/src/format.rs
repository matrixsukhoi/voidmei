//! FastNumberFormatter 的 Rust 移植 (src/ui/util/FastNumberFormatter.java)
//! 语义逐行对齐: 半舍入 (floor(x+0.5)), 负零抑制, NaN→"N/A", TIME_MM_SS

/// Java Math.round(double) = floor(x + 0.5)
fn java_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
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

    let mut integral = value as i64; // Java (long) 截断向零
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
/// (FlightInfo 的 16 字段未用到 TIME_MM_SS, 其它 overlay 迁移时使用)
#[allow(dead_code)]
pub fn format_time(value: f64) -> String {
    if value.is_nan() || value < 0.0 {
        return "--'--".to_string();
    }
    let total_seconds = value as i32; // Java (int) 截断向零
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

#[cfg(test)]
mod tests;
