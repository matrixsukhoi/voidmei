//! StringHelper 的 Rust 移植 (src/prog/util/StringHelper.java)
//! 手写 JSON 子串提取 + 数值解析 (游戏 8111 遥测 State/Indicators 的取值层)。
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

/// 对应 Java `getStringBuilder(StringBuilder R, String S, char buf[], int buflen)`:
/// 在 R 中定位 S (**最后一次**出现), 向后扫到 ':', 跳过冒号与再 1 个码元,
/// 取到 ','/'}' 为止, 把子串写入 buf 自 buflen 起的位置; 找不到时 buf 不动。
///
/// PORT: Java 的 StringBuilder R 在方法内只读 (lastIndexOf/charAt/getChars),
/// 映射为 &str; char[] 目标缓冲映射为 &mut [u8] (UTF-8 字节) — CJK 值
/// 在 Java 里占 1 码元/字符、UTF-8 里占 3 字节/字符, 调用方缓冲需按
/// UTF-8 尺寸预留。越界/长度不足时 Java 抛 IndexOutOfBoundsException
/// ↔ Rust 切片 panic (语义一致)。
pub fn get_string_builder(r: &str, s: &str, buf: &mut [u8], buflen: usize) {
    let mut bix;
    let mut eix;
    match r.rfind(s) {
        Some(i) => bix = i, // Java: bix = R.lastIndexOf(S);
        None => return,     // Java: bix < 0 → 整块跳过, buf 原样
    }
    eix = bix;
    // while (eix < R.length() && R.charAt(eix) != ':') eix++;
    while eix < r.len() && r.as_bytes()[eix] != b':' {
        eix += char_len_at(r, eix);
    }
    // Java: eix++ — 无条件 +1 (':' 为 ASCII, +1 字节 = +1 码元);
    // 扫不到 ':' 时会越过串尾, 后续取子串 Java 抛异常 ↔ Rust panic
    eix += 1;
    // Java: bix = eix + 1 — 跳过 ':' 后 1 个码元 (域内为 ASCII 空格/引号)。
    // PORT: 越界分支的 eix + 1 对齐 Java 的越界量, 保持 panic 路径
    bix = if eix < r.len() { eix + char_len_at(r, eix) } else { eix + 1 };
    // while (eix < R.length() && R.charAt(eix) != ',' && R.charAt(eix) != '}') eix++;
    while eix < r.len() && r.as_bytes()[eix] != b',' && r.as_bytes()[eix] != b'}' {
        eix += char_len_at(r, eix);
    }

    // Java: R.getChars(bix, eix, buf, buflen);
    let src = &r[bix..eix]; // bix > eix → panic ≈ Java srcBegin > srcEnd 异常
    buf[buflen..buflen + src.len()].copy_from_slice(src.as_bytes());
}

/// 对应 Java `getString(String R, String S)`: 在 R 中定位 S (**第一次**出现),
/// 向后扫到 ':', 跳过冒号与再 1 个码元, 取到 ','/'}' 为止的子串;
/// 找不到返回 null → None。
pub fn get_string<'a>(r: &'a str, s: &str) -> Option<&'a str> {
    let mut bix;
    let mut eix;
    // Java: bix = R.indexOf(S); bix < 0 → return null
    bix = r.find(s)?;
    eix = bix;
    // while (eix < R.length() && R.charAt(eix) != ':') eix++;
    while eix < r.len() && r.as_bytes()[eix] != b':' {
        eix += char_len_at(r, eix);
    }
    // Java: eix++ — 无条件 +1 (':' 为 ASCII, +1 字节 = +1 码元);
    // 扫不到 ':' 时越过串尾, substring 抛异常 ↔ Rust 切片 panic
    eix += 1;
    // Java: bix = eix + 1 — 跳过 ':' 后 1 个码元 (域内为 ASCII 空格/引号)。
    // PORT: 值首字符为代理对时 Java 只跳高半码元 (得含孤立代理的坏串),
    // Rust 跳整字符 — 域内 (JSON 值首字符为 '"' 或数字) 不出现
    bix = if eix < r.len() { eix + char_len_at(r, eix) } else { eix + 1 };
    // while (eix < R.length() && R.charAt(eix) != ',' && R.charAt(eix) != '}') eix++;
    while eix < r.len() && r.as_bytes()[eix] != b',' && r.as_bytes()[eix] != b'}' {
        eix += char_len_at(r, eix);
    }

    Some(&r[bix..eix]) // Java: R.substring(bix, eix); 越界异常 ↔ panic
}

/// 对应 Java `getDataFloatC(CharSequence cs)`: null → fInvalid;
/// Float.parseFloat 是**单精度**解析后拓宽 double — Rust 用 parse::<f32>()
/// 再 as f64 逐位复刻 (如 "0.1" → 0.10000000149011612, 而非 double 的 0.1)。
/// PORT: Float.parseFloat 忽略首尾空白 (Java 8 oracle 实测 "  2.25"/"\t1.5\n"
/// 均正常解析) → 先 trim 再 parse; 否则冒号后双空格的脏 payload (getString
/// 只跳 1 码元, 子串带前导空格) 在 Java 走 parseFloat 正常、Rust 裸 parse panic。
/// 注意 Integer.parseInt **无** trim 语义, 见 get_data_int。
/// PORT: NumberFormatException (运行时异常, Service 顶层 catch 兜住) ↔ panic —
/// 本函数在 ~10Hz 轮询线程消费不可信网络输入, Service 移植时必须在轮询层
/// (等价于 Java Service 顶层 catch(Exception) 丢一轮继续) 做 catch_unwind 兜底,
/// 否则单条畸形遥测会杀死整个遥测线程。
/// PORT: 语法域残余差异 (域内 8111 JSON 数值不可达, 仅脏值暴露, 接 Service 需知晓):
/// Java 额外接受 '1.5f'/'1.5d' 后缀与 '0x1.8p1' 十六进制浮点 (oracle 实测 1.5/3.0),
/// Rust 拒绝 → panic; 反向 Rust 接受任意大小写 'inf'/'nan' (Java 仅精确
/// 'NaN'/'Infinity', oracle 实测 'inf' 抛 NumberFormatException)。
pub fn get_data_float_c(cs: Option<&str>) -> f64 {
    if let Some(cs) = cs {
        cs.trim().parse::<f32>().unwrap() as f64 // Java: Float.parseFloat(cs.toString()) (parseFloat 隐含 trim)
    } else {
        F_INVALID
    }
}

/// 对应 Java `getDataIntC(CharSequence cs)` — 注意 Java 源码返回类型就是
/// **double** (Integer.parseInt 结果拓宽), 保真保留; null → iInvalid 拓宽。
pub fn get_data_int_c(cs: Option<&str>) -> f64 {
    if let Some(cs) = cs {
        cs.parse::<i32>().unwrap() as f64 // Java: Integer.parseInt(cs.toString())
    } else {
        I_INVALID as f64
    }
}

/// 对应 Java `getDataFloat(String sdata)`: null → fInvalid;
/// 单精度解析后拓宽 + 隐含 trim + panic/语法域注意事项, 见 get_data_float_c 的 PORT 注释。
pub fn get_data_float(sdata: Option<&str>) -> f64 {
    if let Some(sdata) = sdata {
        sdata.trim().parse::<f32>().unwrap() as f64 // Java: Float.parseFloat(sdata) (parseFloat 隐含 trim)
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
        sdata.parse::<i32>().unwrap() // Java: Integer.parseInt(sdata)
    } else {
        I_INVALID
    }
}

#[cfg(test)]
mod tests;
