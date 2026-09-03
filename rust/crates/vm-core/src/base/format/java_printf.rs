//! Java `String.format` printf 引擎 — 全库唯一真相 (Lang 模板域 +
//! fm/data/reader 的 fmdata 摘要串域, 两域引擎已合一)。
//!
//! 支持的格式子集: `%s` / `%d` / `%f`/`%.0f`~`%.9f` / `%%`。
//! 宽度域 (`%3d` 形态) 域内模板未用 — 解析时跳过、不填充; `%f` 缺省精度 6
//! (Java Formatter 同), 两条语义自 reader 版汇入。
//!
//! 与 base::format 其他函数的关系:
//! - 数值段 `%f` 直接复用 [`super::java_f`] (同一算法: 对最短往返十进制表示
//!   做 HALF_UP, Java 8 oracle 实证), [`java_format_f`] 是其 u8 精度薄包装;
//! - 宽度填充与符号标志不在扫描器子集内, 由同模块姊妹函数承担, 需要时调用方
//!   组合: `%Ns` 宽度 → [`super::pad_width`], `%0Nd` 零填充 → [`super::java_d0`],
//!   `%+.Nf` 强制正号 → [`super::java_f_plus`]。

/// printf 实参 (Lang 模板与 fmdata 摘要串两类占位)。
#[derive(Clone, Copy, Debug)]
pub enum FmtArg<'a> {
    /// %s — null 实参以 "null" 文本呈现 (Java Formatter 行为)
    S(&'a str),
    /// %d — 十进制序号 (i32)
    D(i32),
    /// %f/`%.Nf` — 精度由模板解析 (缺省 6)
    F(f64),
}

/// Java `String.format(template, args...)` 复刻 (支持子集见模块头)。
/// 模板与实参由调用方成对提供; 错配 (实参不足/类型不符/未支持的转换符) 在
/// Java 抛 UnknownFormatConversionException / IllegalFormatConversionException
/// ↔ 此处 panic — 用户改 lang 文件破坏配对时两语言同为崩溃语义。
/// `%s` 位点收数值实参在 Java 合法 (toString 输出), 本实现防御 panic — 域内
/// 实参编译期成对不可达。
pub fn java_string_format(template: &str, args: &[FmtArg]) -> String {
    let mut out = String::new();
    let mut arg_i = 0usize;
    let bytes = template.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b != b'%' {
            // PORT: 模板为 ASCII 控制符 + CJK 文本, 非控制字节段整段透传
            // (按字节推进仅发生在 ASCII 控制符处, UTF-8 多字节序列不越界)
            let start = i;
            while i < bytes.len() && bytes[i] != b'%' {
                i += 1;
            }
            out.push_str(&template[start..i]);
            continue;
        }
        // '%' 转换: [宽度数字] ('.'精度)? 转换符。
        // 宽度域域内模板未用 — 跳过不填充; '%f' 缺省精度 6 (Java Formatter 同)。
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        let mut prec: u32 = 6;
        if j < bytes.len() && bytes[j] == b'.' {
            j += 1;
            let mut p: u32 = 0;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                p = p * 10 + u32::from(bytes[j] - b'0');
                j += 1;
            }
            prec = p;
        }
        let conv = bytes.get(j).copied();
        match conv {
            Some(b'%') => {
                out.push('%'); // %% → 字面 % (不消耗实参)
                i = j + 1;
            }
            Some(b's') | Some(b'd') => {
                let arg = args.get(arg_i).unwrap_or_else(|| {
                    panic!("String.format 实参不足: {template:?} 第 {arg_i} 个占位")
                });
                arg_i += 1;
                match *arg {
                    FmtArg::S(s) => match conv {
                        Some(b's') => out.push_str(s),
                        _ => panic!(
                            "String.format %d 收到字符串实参 (IllegalFormatConversionException): {template:?}"
                        ),
                    },
                    // Integer 的 %s/%d 位点 Java 均合法 (toString / 十进制)
                    FmtArg::D(v) => out.push_str(&v.to_string()),
                    FmtArg::F(_) => match conv {
                        Some(b'd') => panic!(
                            "String.format %d 收到浮点实参 (IllegalFormatConversionException): {template:?}"
                        ),
                        // Java %s 收 Double 合法 (toString), 本实现防御 panic — 域内不可达
                        _ => panic!("模板 %s 位点收到数值实参 (域外防御): {template:?}"),
                    },
                }
                i = j + 1;
            }
            Some(b'f') => {
                // PORT: Java BigDecimal 任意精度合法, 本实现 u128 尾数累加上界 ≤9
                // (下方 as u8 截断与 10u128.pow 回绕均在此拦截); 超域仅模板漂移
                // 可达 → debug 断言, release 不引入 Java 没有的崩溃
                debug_assert!(
                    prec <= 9,
                    "String.format 精度超域 (.{prec}f > .9f): {template:?}"
                );
                let arg = args.get(arg_i).unwrap_or_else(|| {
                    panic!("String.format 实参不足: {template:?} 第 {arg_i} 个占位")
                });
                arg_i += 1;
                match *arg {
                    FmtArg::F(v) => out.push_str(&java_format_f(v, prec as u8)),
                    FmtArg::S(_) | FmtArg::D(_) => {
                        panic!("模板 %.Nf 位点收到非数值实参: {template:?}")
                    }
                }
                i = j + 1;
            }
            _ => panic!("String.format 未支持的转换符: {template:?} @ {i}"),
        }
    }
    out
}

/// Java `String.format("%.{prec}f", d)` 复刻 — [`super::java_f`] 的 u8 精度
/// 薄包装 (同一 HALF_UP 最短表示算法, 语义注记见该函数)。
/// 域界断言: prec ≤ 9 (u128 尾数累加上界, 超域属模板漂移信号)。
pub fn java_format_f(d: f64, prec: u8) -> String {
    debug_assert!(prec <= 9, "java_format_f 精度超域: {prec}");
    super::java_f(d, prec as usize)
}
