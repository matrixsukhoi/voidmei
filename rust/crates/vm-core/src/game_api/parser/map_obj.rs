//! /map_obj.json 的 Player 定位/朝向提取 (Service 在用的唯一路径)。
//!
//! 波20 清场: Java MapObj 的实例解析路径 (parseObj 位置扫描 + mov/sta/slc/pla
//! 对象池, 仅被未接线的 OtherService 消费) 已随 map_service 退役; 本模块只保留
//! 原静态正则方法 getPlayerLoc/getPlayerDir。
//!
//! PORT: §2.1 — Java charAt/substring 按 UTF-16 码元; 本域 (JSON 键/数值) 纯 ASCII,
//! 字节索引 + 整字符步进与 Java 逐码元推进等价 (mod.rs 公共 helper)。
//!
//! PORT: PORTING.md 库映射 java.util.regex → regex crate, 但 vm-core 依赖清单不含
//! regex (本批无权改 Cargo.toml), 按原正则结构手写等价回溯匹配器; 两处模式同构
//! (\{[^{}]*K1\s*:\s*V1[^{}]*,[^{}]*K2\s*:\s*(NUM),[^{}]*K3\s*:\s*(NUM)[^{}]*\}),
//! 参数化复用。贪婪量词的"最长优先+回溯"尝试次序与 java.util.regex 一致 (双 x 键
//! oracle 用例验证取后位); 后续批次若引入 regex crate 可原样替换。

use super::char_len_at;

/// Player 定位/朝向提取器的命名空间 (原 Java 静态方法宿主类, 实例路径已退役)。
pub struct MapObj;

/// Java 正则 `\s`: [ \t\n\x0B\x0C\r] (无 UNICODE_CHARACTER_CLASS 标志的 ASCII 定义)
fn is_java_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// `\s*` 贪婪跳过 — ':' 与数字均非空白, 回溯无助, 确定性
fn skip_ws(t: &str, mut p: usize) -> usize {
    let b = t.as_bytes();
    while p < t.len() && is_java_ws(b[p]) {
        p += char_len_at(t, p);
    }
    p
}

/// `(-?\d+(\.\d+)?)` — \d = [0-9]; 后继原子 (',' ) 非数字, 量词无需有效回溯
fn try_number(t: &str, pos: usize) -> Option<(usize, &str)> {
    let b = t.as_bytes();
    let mut p = pos;
    if p < t.len() && b[p] == b'-' {
        p += 1;
    }
    let dstart = p;
    while p < t.len() && b[p].is_ascii_digit() {
        p += char_len_at(t, p);
    }
    if p == dstart {
        return None; // \d+ 至少 1 位
    }
    let mut end = p;
    if p + 1 < t.len() && b[p] == b'.' {
        let mut q = p + 1;
        while q < t.len() && b[q].is_ascii_digit() {
            q += char_len_at(t, q);
        }
        if q > p + 1 {
            end = q; // (\.\d+)? 贪婪: 点后至少 1 位才吞
        }
    }
    Some((end, &t[pos..end]))
}

/// `[^{}]*` 的最大延伸: 从 from 起到首个 '{'/'}' 或串尾
fn nb_extent(t: &str, from: usize) -> usize {
    let b = t.as_bytes();
    let mut p = from;
    while p < t.len() && b[p] != b'{' && b[p] != b'}' {
        p += char_len_at(t, p);
    }
    p
}

fn lit_at(t: &str, p: usize, lit: &str) -> bool {
    p <= t.len() && t[p..].starts_with(lit)
}

/// i (字符边界) 前一个字符的字节长度 — UTF-8 自同步: 回扫续字节 (0b10xxxxxx)
/// 到主字节即前一字符起点。回溯按整字符递减: ASCII 域恒 1 (与逐字节等价),
/// 非 ASCII 域避免索引落进字符中间令 t[p..] panic (java.util.regex 按码元
/// 正常回溯; BMP 域字符边界=码元边界, astral 中间码元处 ASCII 字面量必不命中)
fn prev_char_len(t: &str, i: usize) -> usize {
    let b = t.as_bytes();
    let mut j = i - 1;
    while j > 0 && b[j] & 0xC0 == 0x80 {
        j -= 1;
    }
    i - j
}

/// 从 start ('{' 处) 尝试匹配整条模式, 成功返回 (整匹配结束位置, 捕获1, 捕获3)。
/// 各选择点 (k1/k2/k3/k4) 按从远到近枚举 = 贪婪 [^{}]* 最长优先。
fn match_from<'a>(
    t: &'a str,
    start: usize,
    key1: &str,
    val1: &str,
    key2: &str,
    key3: &str,
) -> Option<(usize, &'a str, &'a str)> {
    let b = t.as_bytes();

    let e1 = nb_extent(t, start + 1);
    let mut k1 = e1 as i64;
    while k1 >= (start + 1) as i64 {
        let k1u = k1 as usize;
        if lit_at(t, k1u, key1) {
            let mut p = k1u + key1.len();
            p = skip_ws(t, p);
            if b.get(p) == Some(&b':') {
                p = skip_ws(t, p + 1);
                if lit_at(t, p, val1) {
                    p += val1.len();
                    let e2 = nb_extent(t, p);
                    let mut k2 = e2 as i64;
                    while k2 >= p as i64 {
                        let k2u = k2 as usize;
                        if b.get(k2u) == Some(&b',') {
                            let e3 = nb_extent(t, k2u + 1);
                            let mut k3 = e3 as i64;
                            while k3 >= (k2u + 1) as i64 {
                                let k3u = k3 as usize;
                                if lit_at(t, k3u, key2) {
                                    let mut q = k3u + key2.len();
                                    q = skip_ws(t, q);
                                    if b.get(q) == Some(&b':') {
                                        q = skip_ws(t, q + 1);
                                        if let Some((q2, g1)) = try_number(t, q) {
                                            if b.get(q2) == Some(&b',') {
                                                let e4 = nb_extent(t, q2 + 1);
                                                let mut k4 = e4 as i64;
                                                while k4 >= (q2 + 1) as i64 {
                                                    let k4u = k4 as usize;
                                                    if lit_at(t, k4u, key3) {
                                                        let mut r = k4u + key3.len();
                                                        r = skip_ws(t, r);
                                                        if b.get(r) == Some(&b':') {
                                                            r = skip_ws(t, r + 1);
                                                            if let Some((r2, g3)) = try_number(t, r)
                                                            {
                                                                // 尾部 [^{}]*\}
                                                                let e5 = nb_extent(t, r2);
                                                                if e5 < t.len() && b[e5] == b'}' {
                                                                    return Some((e5 + 1, g1, g3));
                                                                }
                                                            }
                                                        }
                                                    }
                                                    k4 -= prev_char_len(t, k4u) as i64;
                                                }
                                            }
                                        }
                                    }
                                }
                                k3 -= prev_char_len(t, k3u) as i64;
                            }
                        }
                        k2 -= prev_char_len(t, k2u) as i64;
                    }
                }
            }
        }
        k1 -= prev_char_len(t, k1u) as i64;
    }
    None
}

/// java.util.regex Matcher.find() 循环的等价: 从左到右不重叠匹配, 收集 (捕获1, 捕获3)
fn find_pairs<'a>(
    text: &'a str,
    key1: &str,
    val1: &str,
    key2: &str,
    key3: &str,
) -> Vec<(&'a str, &'a str)> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < text.len() {
        // '\{' 只在 '{' 处可匹配, 其余起点直接步进一个字符
        if b[start] == b'{' {
            if let Some((end, g1, g3)) = match_from(text, start, key1, val1, key2, key3) {
                out.push((g1, g3));
                start = end; // 下一 find 从上次整匹配末尾起 (不重叠)
                continue;
            }
        }
        start += char_len_at(text, start);
    }
    out
}

impl MapObj {
    /// Java `public static void getPlayerLoc(String jsonText, double[] loc)`
    pub fn get_player_loc(json_text: &str, loc: &mut [f64; 2]) {
        // 正则表达式用于匹配整个JSON对象，并捕获icon为"Player"的x和y坐标
        for (g1, g3) in find_pairs(json_text, "\"icon\"", "\"Player\"", "\"x\"", "\"y\"") {
            let x: f64 = g1.parse().unwrap();
            let y: f64 = g3.parse().unwrap();
            // System.out.println("Player coordinates: x = " + x + ", y = " + y);
            loc[0] = x;
            loc[1] = y;
        }
    }

    /// Java `public static void getPlayerDir(String jsonText, double[] dir)`
    pub fn get_player_dir(json_text: &str, dir: &mut [f64; 2]) {
        // 正则表达式用于匹配整个JSON对象，并捕获icon为"Player"的x和y坐标
        for (g1, g3) in find_pairs(json_text, "\"icon\"", "\"Player\"", "\"dx\"", "\"dy\"") {
            let dx: f64 = g1.parse().unwrap();
            let dy: f64 = g3.parse().unwrap();
            // System.out.println("Player direction: dx = " + dx + ", dy = " + dy);
            dir[0] = dx;
            dir[1] = dy;
        }
    }
}

#[cfg(test)]
mod tests;
