//! StringHelper 的 Rust 移植 (src/prog/util/StringHelper.java)
//! 手写 JSON 子串提取 + 数值解析 (游戏 8111 遥测 State/Indicators 的取值层)。
//! (波20 清场: get_string_builder/get_data_float_c/get_data_int_c 全库无生产
//! 调用, 已删; 剩余取数函数将随 parser serde 化一并退役, 届时本文件只留
//! I_INVALID/F_INVALID 哨兵常量)
//!
//! PORT: Java 类仅含 static 方法 → Rust 模块自由函数 (format.rs 先例)。
//! PORT: §2.1 — Java charAt/substring 按 UTF-16 码元计数, Rust 字符串是 UTF-8。
//! 本模块索引一律为字节偏移, 循环推进经 char_len_at 按"整个字符"步进:
//! BMP 字符 (含 CJK) 1 码元 = 1 字符, 与 Java 逐步语义等价; 定界符
//! ':'/','/'}' 均为 ASCII, 逐字节比较不会误判多字节字符 (UTF-8 自同步)。
//!
//! 域内格式备注 (游戏 8111 JSON 为 `"key": value` 冒号后带空格):
//! - 字符串值: getString 跳过的是前导空格, 返回值**含首尾引号**
//!   (调用方 Indicators 自行 substring(1, len-1) 去壳);
//! - 数值/布尔值: 返回裸值 `"123.45"` / `true`。

/// 对应 Java `public static final int iInvalid`
pub const I_INVALID: i32 = -65535;

/// 对应 Java `public static final double fInvalid`
pub const F_INVALID: f64 = -65535.0;

/// r[i..] 首字符的 UTF-8 字节数; i 越界返回 0。
/// PORT: Java 循环 `eix++` 逐 UTF-16 码元推进, 此处按整字符推进 —
/// BMP 内等价, 代理对 (astral) 处的差异见各调用点注释。
fn char_len_at(r: &str, i: usize) -> usize {
    r[i..].chars().next().map_or(0, char::len_utf8)
}

/// 对应 Java `getString(String R, String S)`: 在 R 中定位 S (**第一次**出现),
/// 向后扫到 ':', 跳过冒号与再 1 个码元, 取到 ','/'}' 为止的子串;
/// 找不到返回 null → None。
pub fn get_string<'a>(r: &'a str, s: &str) -> Option<&'a str> {
    let mut bix;
    let mut eix;
    bix = r.find(s)?;
    eix = bix;
    // while (eix < R.length() && R.charAt(eix) != ':') eix++;
    while eix < r.len() && r.as_bytes()[eix] != b':' {
        eix += char_len_at(r, eix);
    }
    // 扫不到 ':' 时越过串尾, substring 抛异常 ↔ Rust 切片 panic
    eix += 1;
    // PORT: 值首字符为代理对时 Java 只跳高半码元 (得含孤立代理的坏串),
    // Rust 跳整字符 — 域内 (JSON 值首字符为 '"' 或数字) 不出现
    bix = if eix < r.len() {
        eix + char_len_at(r, eix)
    } else {
        eix + 1
    };
    // while (eix < R.length() && R.charAt(eix) != ',' && R.charAt(eix) != '}') eix++;
    while eix < r.len() && r.as_bytes()[eix] != b',' && r.as_bytes()[eix] != b'}' {
        eix += char_len_at(r, eix);
    }

    Some(&r[bix..eix])
}

/// 对应 Java `getDataFloat(String sdata)`: null → fInvalid;
/// 单精度解析后拓宽 + 隐含 trim + panic/语法域注意事项, 见原 get_data_float_c
/// 的 PORT 注释 (Java Float.parseFloat 单精度 + 空白容忍, NumberFormatException
/// ↔ panic 由轮询层 catch_unwind 兜底)。
pub fn get_data_float(sdata: Option<&str>) -> f64 {
    if let Some(sdata) = sdata {
        sdata.trim().parse::<f32>().unwrap() as f64
    } else {
        F_INVALID
    }
}

/// 对应 Java `getDataInt(String sdata)`: null → iInvalid。
/// PORT: NumberFormatException (含溢出/小数点) ↔ panic。
/// PORT: Integer.parseInt **不** trim 首尾空白 (Java 8 oracle 实测 " 5"/"5 "
/// 均抛 NumberFormatException) — 与 Float.parseFloat 相反, 此处不加 trim 是保真。
pub fn get_data_int(sdata: Option<&str>) -> i32 {
    if let Some(sdata) = sdata {
        sdata.parse::<i32>().unwrap()
    } else {
        I_INVALID
    }
}

#[cfg(test)]
mod tests;
