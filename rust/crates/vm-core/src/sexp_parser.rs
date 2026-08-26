//! SExpParser 的 Rust 移植 (src/prog/config/SExpParser.java)
//!
//! A zero-dependency S-Expression parser.
//! Supports:
//! - Lists: (a b c)
//! - Strings: "hello world"
//! - Numbers: 123, 12.34
//! - Booleans: true, false
//! - Keywords: :x, :type
//! - Symbols: panel, group
//!
//! PORT: Java `interface SExp` (仅 SList/SAtom 两个封闭实现) → `enum SExp`。
//! Java 引用可别名 — ConfigLoader.getKeywordSExp 把子树原对象直接存进
//! RowConfig.visibleWhen / naWhen — 故节点统一 `Rc<SExp>` 共享。
//! Rc 而非 Arc: 配置解析单线程。注意 Rc<SExp> 树 Send 但 !Sync —
//! 可整体 move 跨线程交接 (如热重载线程→UI 线程), 禁止跨线程共享同一棵树。
//! PORT: Java 枚举常量全大写 (STRING/LPAREN) → Rust 驼峰 (String/LParen), 语义不变。

use std::fmt;
use std::rc::Rc;

// --- AST Nodes ---

/// Java: `public interface SExp { boolean isList(); boolean isAtom(); SList asList(); SAtom asAtom(); }`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExp {
    List(SList),
    Atom(SAtom),
}

impl SExp {
    /// Java: `boolean isList()`
    pub fn is_list(&self) -> bool {
        matches!(self, SExp::List(_))
    }

    /// Java: `boolean isAtom()`
    pub fn is_atom(&self) -> bool {
        matches!(self, SExp::Atom(_))
    }

    /// Java: SList.asList() 返回 this; SAtom.asList() 抛 IllegalStateException("Not a list")
    /// PORT: 未受检异常 → panic (PORTING.md §1)
    pub fn as_list(&self) -> &SList {
        match self {
            SExp::List(l) => l,
            SExp::Atom(_) => panic!("Not a list"),
        }
    }

    /// Java: SAtom.asAtom() 返回 this; SList.asAtom() 抛 IllegalStateException("Not an atom")
    pub fn as_atom(&self) -> &SAtom {
        match self {
            SExp::Atom(a) => a,
            SExp::List(_) => panic!("Not an atom"),
        }
    }
}

/// Java: `public static class SList implements SExp`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SList {
    /// Java: `public List<SExp> children = new ArrayList<>()`
    pub children: Vec<Rc<SExp>>,
}

impl SList {
    /// Java: 无参构造 (children 初始为空)
    pub fn new() -> SList {
        SList {
            children: Vec::new(),
        }
    }

    /// Java: `public void add(SExp exp)`
    pub fn add(&mut self, exp: Rc<SExp>) {
        self.children.push(exp);
    }
}

impl Default for SList {
    fn default() -> Self {
        SList::new()
    }
}

/// Java: SList.toString() — "(" + 子节点以单个空格连接 + ")"
impl fmt::Display for SList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut i = 0;
        while i < self.children.len() {
            // Java: sb.append(children.get(i).toString()) — 虚分派到元素实际类型
            write!(f, "{}", self.children[i])?;
            if i < self.children.len() - 1 {
                write!(f, " ")?;
            }
            i += 1;
        }
        write!(f, ")")
    }
}

/// Java: `public enum AtomType { STRING, NUMBER, BOOLEAN, KEYWORD, SYMBOL }`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomType {
    String,
    Number,
    Boolean,
    Keyword,
    Symbol,
}

/// Java: `public static class SAtom implements SExp`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SAtom {
    /// Java: `public String value`
    pub value: String,
    /// Java: `public AtomType type` — type 为 Rust 关键字, 原名保留用 raw identifier
    pub r#type: AtomType,
}

impl SAtom {
    /// Java: `public SAtom(String value, AtomType type)`
    pub fn new(value: String, r#type: AtomType) -> SAtom {
        SAtom { value, r#type }
    }

    /// Java: `public String getString()`
    pub fn get_string(&self) -> &str {
        &self.value
    }

    /// Java: `public double getDouble() { return Double.parseDouble(value); }`
    /// NumberFormatException (未受检) 传播为 panic (string_helper.rs 先例);
    /// 消息按 Java 8 oracle 分支: 空白剥净后为空 → "empty String",
    /// 非空非法 → For input string: "<trim 后串>" (FloatingDecimal 先 in.trim())
    pub fn get_double(&self) -> f64 {
        match java_parse_double(&self.value) {
            Ok(v) => v,
            Err(ParseDoubleErr::Empty) => panic!("empty String"),
            Err(ParseDoubleErr::Invalid) => panic!(
                "For input string: \"{}\"",
                self.value.trim_matches(|c: char| (c as u32) <= 0x20)
            ),
        }
    }

    /// Java: `public int getInt() { return (int) getDouble(); }`
    /// PORT: Rust `f64 as i32` = JLS 5.1.3 (NaN→0, ±Inf/越界饱和到 i32 极值, 向零截断)
    /// — Java 8 oracle 实测逐值一致 (3.99→3 / 1e10→MAX / NaN→0)
    pub fn get_int(&self) -> i32 {
        self.get_double() as i32
    }

    /// Java: `public boolean getBool() { return Boolean.parseBoolean(value); }`
    /// PORT: 即 equalsIgnoreCase("true") — Java 8 oracle: "TRUE"/"True"→true, 带空格→false;
    /// 非 ASCII 字符无单字符大写映射到 't'/'r'/'u'/'e', eq_ignore_ascii_case 等价
    pub fn get_bool(&self) -> bool {
        self.value.eq_ignore_ascii_case("true")
    }

    /// Java: `public boolean isKeyword() { return type == AtomType.KEYWORD; }`
    pub fn is_keyword(&self) -> bool {
        self.r#type == AtomType::Keyword
    }

    /// Java: `public boolean isSymbol() { return type == AtomType.SYMBOL; }`
    pub fn is_symbol(&self) -> bool {
        self.r#type == AtomType::Symbol
    }
}

/// Java: SAtom.toString() — STRING 原样加引号 (**不转义内部引号**, 忠实保留), 其余原样
impl fmt::Display for SAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.r#type == AtomType::String {
            write!(f, "\"{}\"", self.value)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

/// Java: toString() 虚分派 — 按运行时实际类型派发
impl fmt::Display for SExp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SExp::List(l) => l.fmt(f),
            SExp::Atom(a) => a.fmt(f),
        }
    }
}

// --- Tokenizer ---

/// Java: `private enum TokenType { LPAREN, RPAREN, STRING, NUMBER, BOOLEAN, KEYWORD, SYMBOL, EOF }`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    LParen,
    RParen,
    String,
    Number,
    Boolean,
    Keyword,
    Symbol,
    Eof,
}

/// Java: `private static class Token { TokenType type; String value; }`
#[derive(Debug, Clone)]
struct Token {
    r#type: TokenType,
    value: String,
}

impl Token {
    /// Java: `Token(TokenType type, String value)`
    fn new(r#type: TokenType, value: &str) -> Token {
        Token {
            r#type,
            value: value.to_string(),
        }
    }
}

/// Java `Character.isWhitespace(char)` 复刻 (JDK 8, build/oracle 实测)。
/// 与 Rust `char::is_whitespace` (Unicode White_Space) 的差异:
/// - U+0085/U+00A0/U+2007/U+202F (无断行空格): Rust true, **Java false**
/// - U+001C..U+001F (信息分隔符): **Java true**, Rust false
/// - U+180E: JDK8 true, 现代 Unicode false
fn java_is_whitespace(c: char) -> bool {
    match c {
        '\u{09}'..='\u{0d}' | '\u{1c}'..='\u{1f}' | '\u{180e}' => true,
        '\u{85}' | '\u{a0}' | '\u{2007}' | '\u{202f}' => false,
        _ => c.is_whitespace(),
    }
}

/// java_parse_double 的 Err 变体 — 对应 Java 8 NumberFormatException 消息 (oracle 实测):
/// 空白剥净后为空串 → "empty String"; 非空非法串 → For input string: "..."
#[derive(Debug)]
enum ParseDoubleErr {
    Empty,
    Invalid,
}

/// Java `Double.parseDouble` 一比一复刻 (Java 8 oracle 双轮实测, build/oracle):
/// - 先 trim 两端 <= U+0020 的字符 (Java 实现内 `in.trim()`; Rust `parse` 不 trim — 差异点)
/// - `[sign] "NaN"` (符号被忽略: oracle -NaN 位形为正) / `[sign] "Infinity"` — 大小写敏感精确匹配
/// - 十六进制: `0[xX] hexDigits[.hexDigits] (p|P)[sign]decDigits [fFdD]?`
///   p 指数**必有** ("0x8" 拒), 后缀可挂 ("0x1p1f" 收)
/// - 十进制: `[sign] (digits[.digits] | .digits) ([eE][sign]digits)? [fFdD]?`
///   "5." ".5" "5.e2" "1e5f" 收; "1e" "1e+" "+" "" "1_000" "5,5" "nan" "infinity" 拒
///
/// Err 对应 NumberFormatException: isNumber 捕获→false, getDouble 传播→panic。
/// PORT: 极端输入与 Java 8 FloatingDecimal 存在域外分歧 (cfg 数值域内不受影响):
/// - >32 位十六进制尾数: u128 回绕 (§2.2 先例); >53 位尾数: as f64 预舍入
/// - 超长十进制尾数: 可能差 1 ulp (JDK-4511638 域, Rust parse 正确舍入)
/// - 次正规边界位形: 中间量若落入次正规区产生双重舍入 (需 ~250+ 位小数或
///   |e10|>2000, 详见 hex 求值处注释); 正常指数域已按 oracle 位级对齐
fn java_parse_double(s: &str) -> Result<f64, ParseDoubleErr> {
    // Java: FloatingDecimal.readJavaFormatString 首步 in.trim() — 去两端 <= ' '
    let t = s.trim_matches(|c: char| (c as u32) <= 0x20);
    if t.is_empty() {
        // oracle: parseDouble(""/"   ") 抛 NumberFormatException: empty String
        return Err(ParseDoubleErr::Empty);
    }
    let (neg, rest) = match t.as_bytes()[0] {
        b'-' => (true, &t[1..]),
        b'+' => (false, &t[1..]),
        _ => (false, t),
    };
    if rest == "NaN" {
        return Ok(f64::NAN);
    }
    if rest == "Infinity" {
        return Ok(if neg { f64::NEG_INFINITY } else { f64::INFINITY });
    }
    if rest.starts_with("0x") || rest.starts_with("0X") {
        let hex = &rest[2..];
        // 尾缀 f/F/d/D 只剥一个 (oracle: "0x1p1f" 收); 剩余非法由下述校验拒绝
        let hex = match hex.strip_suffix(|c: char| matches!(c, 'f' | 'F' | 'd' | 'D')) {
            Some(h) => h,
            None => hex,
        };
        // p|P 指数分隔必有 (oracle: "0x8"/"0x8f" 均拒)
        let Some(ppos) = hex.find(['p', 'P']) else {
            return Err(ParseDoubleErr::Invalid);
        };
        let (mant, exp) = hex.split_at(ppos);
        let exp = &exp[1..]; // 去掉 p/P
        let (int_part, frac_part) = match mant.split_once('.') {
            Some((a, b)) => (a, b),
            None => (mant, ""),
        };
        if !int_part.chars().all(|c| c.is_ascii_hexdigit())
            || !frac_part.chars().all(|c| c.is_ascii_hexdigit())
            || (int_part.is_empty() && frac_part.is_empty())
        {
            // oracle: "0x.p1" 拒 — 至少一位十六进制数字
            return Err(ParseDoubleErr::Invalid);
        }
        let (exp_neg, exp_digits) = match exp.as_bytes().first() {
            Some(b'-') => (true, &exp[1..]),
            Some(b'+') => (false, &exp[1..]),
            _ => (false, exp),
        };
        if exp_digits.is_empty() || !exp_digits.bytes().all(|b| b.is_ascii_digit()) {
            // oracle: "0x1p" 拒 — 指数至少一位十进制数字
            return Err(ParseDoubleErr::Invalid);
        }
        // 尾数按十六进制位累积 (≤32 位精确; 更长为域外极端输入, §2.2 回绕先例)
        let mut m: u128 = 0;
        let mut frac_bits: i64 = 0; // 小数每多一位十六进制 = 乘 16⁻¹ = 2⁻⁴
        for ch in int_part.chars() {
            m = m
                .wrapping_mul(16)
                .wrapping_add(u128::from(ch.to_digit(16).unwrap()));
        }
        for ch in frac_part.chars() {
            m = m
                .wrapping_mul(16)
                .wrapping_add(u128::from(ch.to_digit(16).unwrap()));
            frac_bits += 4;
        }
        let mut e10: i64 = 0;
        // PORT: Java int/long 静默回绕 — 超长指数位串 (≥19 位) 回绕成小值 (§2.2 先例,
        // 同上 m 尾数累积); 域外输入, 正常指数无差异
        for b in exp_digits.bytes() {
            e10 = e10
                .wrapping_mul(10)
                .wrapping_add(i64::from(b - b'0'));
        }
        if exp_neg {
            e10 = -e10;
        }
        // value = m × 2^(e10 − frac_bits)
        let shift = e10.wrapping_sub(frac_bits);
        // PORT: Java 单次舍入求值; 2^shift 整体先算会提前下溢/上溢 (oracle:
        // "0x40p-1080"=4.9E-324 而直接 powi(-1080)=0) — 拆半连乘, 两因子均在
        // 正规范围内精确表示, 仅最终乘积可能落次正规 (单次舍入, oracle 位级对齐:
        // 0x40p-1080/0x10000000000000p-1075/0x1fffffffffffff8p-1077/0x3p-1075/
        // 0x1p±1023/0x1p-2000 逐例核对)。|shift|>~2045 时中间量入次正规区,
        // 双重舍入与 Java 可能差 1 ulp — 域外极端, cfg 无十六进制浮点字面量
        let s1 = shift / 2; // i64 除法向零截断
        let s2 = shift - s1;
        let v = m as f64 * 2f64.powi(s1 as i32) * 2f64.powi(s2 as i32);
        return Ok(if neg { -v } else { v });
    }
    // 十进制: 剥尾缀后先自校验文法再委托 Rust parse 取值
    // (不能直接喂 Rust parse: 它还接受 "inf"/"NaN" 大小写不敏感 — 上面已拦截非数字形式)
    let core = match rest.strip_suffix(|c: char| matches!(c, 'f' | 'F' | 'd' | 'D')) {
        Some(c) => c,
        None => rest,
    };
    let (mant, exp) = match core.find(['e', 'E']) {
        Some(i) => (&core[..i], Some(&core[i + 1..])),
        None => (core, None),
    };
    let (int_part, frac_part) = match mant.split_once('.') {
        Some((a, b)) => (a, b),
        None => (mant, ""),
    };
    if !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
        || (int_part.is_empty() && frac_part.is_empty())
    {
        // 至少一位数字 — "." / "e5" / "+.e5" / "5,5" / "1_000" / "5-3" 全拒
        return Err(ParseDoubleErr::Invalid);
    }
    if let Some(e) = exp {
        let digits = match e.as_bytes().first() {
            Some(b'+') | Some(b'-') => &e[1..],
            _ => e,
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            // oracle: "1e" / "1e+" 拒
            return Err(ParseDoubleErr::Invalid);
        }
    }
    let v: f64 = core.parse().map_err(|_| ParseDoubleErr::Invalid)?;
    Ok(if neg { -v } else { v })
}

// --- Parser ---

/// Java: `public class SExpParser` — 实例字段 `private int pos; private List<Token> tokens;`
pub struct SExpParser {
    pos: usize,
    tokens: Vec<Token>,
}

impl SExpParser {
    /// Java: `new SExpParser()`
    pub fn new() -> SExpParser {
        SExpParser {
            pos: 0,
            tokens: Vec::new(),
        }
    }

    /// Java: `private List<Token> tokenize(String input)`
    fn tokenize(&self, input: &str) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        // Simple regex-based tokenizer
        // 1. Strings: "..."
        // 2. Comments: ;... (handled by pre-processing or regex)
        // 3. Special chars: ( )
        // 4. Whitespace (ignored)
        // 5. Atoms: everything else

        // PORT: §2.1 — Java charAt 按 UTF-16 码元推进; 此处收集为 Vec<char> 按码点索引,
        // BMP 内逐步等价, 定界符全 ASCII。Rust &str 不可能含孤立代理对 (Java String 可),
        // 该差异在合法 UTF-8 输入域内不可达。
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0usize;
        let len = chars.len();
        while i < len {
            let c = chars[i];

            if java_is_whitespace(c) {
                i += 1;
                continue;
            }

            if c == ';' {
                // Comment, skip to end of line
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }

            if c == '(' {
                tokens.push(Token::new(TokenType::LParen, "("));
                i += 1;
                continue;
            }

            if c == ')' {
                tokens.push(Token::new(TokenType::RParen, ")"));
                i += 1;
                continue;
            }

            if c == '"' {
                // String literal
                let mut sb = String::new();
                i += 1; // skip open quote
                while i < len {
                    let sc = chars[i];
                    if sc == '"' {
                        i += 1; // skip close quote
                        break;
                    }
                    if sc == '\\' && i + 1 < len {
                        // Simple escape handling — 反斜杠后一个字符**原样**收编 (不解释 \n 等)
                        i += 1;
                        sb.push(chars[i]);
                    } else {
                        sb.push(sc);
                    }
                    i += 1;
                }
                tokens.push(Token::new(TokenType::String, &sb));
                continue;
            }

            // Atom (Keyword, Number, Boolean, Symbol)
            let mut sb = String::new();
            while i < len {
                let ac = chars[i];
                // 注意: 空白/)/(/; 是原子定界符, 引号**不是** — Java 行为如此 (a"b" 是一个原子)
                if java_is_whitespace(ac) || ac == ')' || ac == '(' || ac == ';' {
                    break;
                }
                sb.push(ac);
                i += 1;
            }
            let atom_str = sb;
            if atom_str.is_empty() {
                continue;
            }

            if atom_str.starts_with(':') {
                tokens.push(Token::new(TokenType::Keyword, &atom_str));
            } else if atom_str == "true" || atom_str == "false" {
                tokens.push(Token::new(TokenType::Boolean, &atom_str));
            } else if self.is_number(&atom_str) {
                tokens.push(Token::new(TokenType::Number, &atom_str));
            } else {
                tokens.push(Token::new(TokenType::Symbol, &atom_str));
            }
        }
        tokens.push(Token::new(TokenType::Eof, ""));
        tokens
    }

    /// Java: `private boolean isNumber(String s)` — try parseDouble, 捕获 NFE 返回 false
    fn is_number(&self, s: &str) -> bool {
        java_parse_double(s).is_ok()
    }

    /// Java: `public List<SExp> parse(String input)`
    pub fn parse(&mut self, input: &str) -> Vec<Rc<SExp>> {
        self.tokens = self.tokenize(input);
        self.pos = 0;
        let mut expressions: Vec<Rc<SExp>> = Vec::new();

        while self.peek().r#type != TokenType::Eof {
            expressions.push(self.parse_expression());
        }
        expressions
    }

    /// Java: `private Token peek()` — 越界返回 EOF 哨兵 token
    fn peek(&self) -> Token {
        if self.pos >= self.tokens.len() {
            return Token::new(TokenType::Eof, "");
        }
        self.tokens[self.pos].clone()
    }

    /// Java: `private Token consume()`
    fn consume(&mut self) -> Token {
        let t = self.peek();
        self.pos += 1;
        t
    }

    /// Java: `private SExp parseExpression()`
    fn parse_expression(&mut self) -> Rc<SExp> {
        let t = self.peek();
        if t.r#type == TokenType::LParen {
            self.parse_list()
        } else {
            Rc::new(SExp::Atom(self.parse_atom()))
        }
    }

    /// Java: `private SList parseList()` (Rc 包装 = Java 引用返回的对应物)
    fn parse_list(&mut self) -> Rc<SExp> {
        self.consume(); // (
        let mut list = SList::new();
        while self.peek().r#type != TokenType::RParen && self.peek().r#type != TokenType::Eof {
            list.add(self.parse_expression());
        }
        self.consume(); // )
        Rc::new(SExp::List(list))
    }

    /// Java: `private SAtom parseAtom()`
    fn parse_atom(&mut self) -> SAtom {
        let t = self.consume();
        let r#type = match t.r#type {
            TokenType::String => AtomType::String,
            TokenType::Number => AtomType::Number,
            TokenType::Boolean => AtomType::Boolean,
            TokenType::Keyword => AtomType::Keyword,
            // Java default 分支: SYMBOL/RPAREN/EOF 一律落 SYMBOL
            _ => AtomType::Symbol,
        };
        SAtom::new(t.value, r#type)
    }
}

impl Default for SExpParser {
    fn default() -> Self {
        SExpParser::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// 解析并取各顶层表达式的 Display 形式 (测试辅助)
    fn parse_str(s: &str) -> Vec<String> {
        let mut parser = SExpParser::new();
        parser.parse(s).iter().map(|e| e.to_string()).collect()
    }

    /// 解析并取唯一顶层表达式
    fn parse_one(s: &str) -> Rc<SExp> {
        let mut parser = SExpParser::new();
        let es = parser.parse(s);
        assert_eq!(es.len(), 1);
        es.into_iter().next().unwrap()
    }

    /// 顶层原子序列的 (值, 类型) — 分类测试辅助
    fn atom_types(s: &str) -> Vec<(String, AtomType)> {
        let mut parser = SExpParser::new();
        parser
            .parse(s)
            .into_iter()
            .map(|e| {
                let a = e.as_atom();
                (a.get_string().to_string(), a.r#type)
            })
            .collect()
    }

    // ---- tokenize / parse 边界 ----

    #[test]
    fn empty_and_whitespace_input_yield_no_expressions() {
        assert_eq!(parse_str(""), Vec::<String>::new());
        assert_eq!(parse_str("   \t\r\n"), Vec::<String>::new());
    }

    #[test]
    fn simple_list_of_symbols() {
        let e = parse_one("(a b c)");
        assert!(e.is_list() && !e.is_atom());
        let l = e.as_list();
        assert_eq!(l.children.len(), 3);
        for c in &l.children {
            assert!(c.is_atom() && c.as_atom().is_symbol());
        }
        assert_eq!(l.children[1].as_atom().get_string(), "b");
        assert_eq!(e.to_string(), "(a b c)");
    }

    #[test]
    fn multiple_top_level_expressions() {
        assert_eq!(parse_str("a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn nested_lists() {
        let e = parse_one("(a (b c) d)");
        let l = e.as_list();
        assert_eq!(l.children.len(), 3);
        let inner = l.children[1].as_list();
        assert_eq!(inner.to_string(), "(b c)");
        assert_eq!(e.to_string(), "(a (b c) d)");
    }

    #[test]
    fn empty_list_and_nested_empty() {
        let e = parse_one("()");
        assert_eq!(e.as_list().children.len(), 0);
        assert_eq!(e.to_string(), "()");
        assert_eq!(parse_one("(())").to_string(), "(())");
    }

    #[test]
    fn string_literal_with_spaces() {
        let e = parse_one("(a \"hello world\")");
        let a = e.as_list().children[1].as_atom();
        assert_eq!(a.r#type, AtomType::String);
        assert_eq!(a.get_string(), "hello world");
        assert_eq!(e.to_string(), "(a \"hello world\")");
    }

    #[test]
    fn string_escape_handling() {
        // 输入字符: " a \" b \\ c " — \" → ", \\ → \
        let e = parse_one(r#""a\"b\\c""#);
        let a = e.as_atom();
        assert_eq!(a.r#type, AtomType::String);
        assert_eq!(a.get_string(), "a\"b\\c");
    }

    #[test]
    fn string_escape_keeps_char_verbatim() {
        // Java: sb.append(input.charAt(i)) — \n 收编的是字母 'n', 不解释为换行
        let e = parse_one(r#""\n""#);
        assert_eq!(e.as_atom().get_string(), "n");
    }

    #[test]
    fn string_escape_at_end_appends_backslash() {
        // 末尾孤立反斜杠 (i+1 == len): 条件不满足, 走 else 原样收编 '\'
        let e = parse_one(r#""ab\"#);
        assert_eq!(e.as_atom().get_string(), "ab\\");
    }

    #[test]
    fn unterminated_string_takes_rest() {
        let e = parse_one("\"abc");
        let a = e.as_atom();
        assert_eq!(a.r#type, AtomType::String);
        assert_eq!(a.get_string(), "abc");
    }

    #[test]
    fn quote_does_not_break_atom() {
        // Java 原子定界符不含引号 — a"b" 整体是一个 SYMBOL
        let e = parse_one("a\"b\"c");
        let a = e.as_atom();
        assert!(a.is_symbol());
        assert_eq!(a.get_string(), "a\"b\"c");
    }

    #[test]
    fn semicolon_inside_string_kept() {
        let e = parse_one(r#"("a;b")"#);
        let a = e.as_list().children[0].as_atom();
        assert_eq!(a.r#type, AtomType::String);
        assert_eq!(a.get_string(), "a;b");
    }

    #[test]
    fn comments_skipped_to_end_of_line() {
        assert_eq!(parse_str("; (a)\n(b)"), vec!["(b)"]);
        assert_eq!(parse_str("(a) ; trailing comment"), vec!["(a)"]);
        // \r\n: 注释循环只认 \n, \r 留在注释里, \n 交回外层当空白
        assert_eq!(parse_str("x ; c\r\ny"), vec!["x", "y"]);
    }

    #[test]
    fn comment_without_newline_swallows_rest() {
        assert_eq!(parse_str("(a) ; no newline (b)"), vec!["(a)"]);
        // 未加引号的 a;b — 原子断在 ';', 注释吞掉 b
        assert_eq!(parse_str("a;b"), vec!["a"]);
    }

    #[test]
    fn keyword_atoms() {
        let types = atom_types(":x :type :cols");
        assert!(types.iter().all(|(_, t)| *t == AtomType::Keyword));
        // ':' 前缀优先于布尔/数字判定 — :true 是 KEYWORD 不是 BOOLEAN
        assert_eq!(atom_types(":true :5"), vec![
            (":true".into(), AtomType::Keyword),
            (":5".into(), AtomType::Keyword),
        ]);
    }

    #[test]
    fn boolean_atoms_exact_case() {
        assert_eq!(
            atom_types("true false"),
            vec![
                ("true".into(), AtomType::Boolean),
                ("false".into(), AtomType::Boolean),
            ]
        );
        // tokenizer 用 equals 精确匹配 — "True"/"TRUE" 落 SYMBOL
        assert_eq!(
            atom_types("True TRUE"),
            vec![
                ("True".into(), AtomType::Symbol),
                ("TRUE".into(), AtomType::Symbol),
            ]
        );
    }

    #[test]
    fn number_atom_classification() {
        // oracle 实测 parseDouble 均收 (含 NaN/Infinity/十六进制/后缀)
        assert!(atom_types("123 12.34 -5 +5 1e5 1E-5 .5 5. 5f 5d 1e5f NaN -NaN Infinity -Infinity 0x1p1 0X1.8P1")
            .iter()
            .all(|(_, t)| *t == AtomType::Number));
        // oracle 实测 parseDouble 均拒 → SYMBOL
        assert!(atom_types("5,5 abc 12.34.56 1_000 nan infinity INF 0x8 e5 5-")
            .iter()
            .all(|(_, t)| *t == AtomType::Symbol));
    }

    #[test]
    fn stray_rparen_becomes_symbol_atom() {
        // parseExpression 对非 LPAREN 一律走 parseAtom — 顶层多余的 ) 收编为 SYMBOL 原子
        let types = atom_types("a) b");
        assert_eq!(
            types,
            vec![
                ("a".into(), AtomType::Symbol),
                (")".into(), AtomType::Symbol),
                ("b".into(), AtomType::Symbol),
            ]
        );
        assert_eq!(parse_str("(a))"), vec!["(a)", ")"]);
    }

    #[test]
    fn unclosed_paren_terminates_at_eof() {
        let e = parse_one("(a b");
        assert_eq!(e.as_list().children.len(), 2);
        assert_eq!(e.to_string(), "(a b)");
    }

    #[test]
    fn parser_instance_reusable() {
        // Java 字段 pos/tokens 在 parse() 开头重置 — 同实例多次 parse 互不残留
        let mut parser = SExpParser::new();
        assert_eq!(parser.parse("(a) (b)").len(), 2);
        let second = parser.parse("(x y)");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].to_string(), "(x y)");
    }

    #[test]
    fn cjk_and_astral_chars_in_atoms() {
        // Vec<char> 逐步推进: CJK/BMP 内与 Java charAt 等价
        let e = parse_one("(速度 🚀)");
        let a = e.as_list().children[1].as_atom();
        assert_eq!(a.get_string(), "🚀");
        assert_eq!(e.to_string(), "(速度 🚀)");
    }

    // ---- 空白语义 (Character.isWhitespace 复刻) ----

    #[test]
    fn java_is_whitespace_matches_jdk8_oracle() {
        for c in [
            ' ', '\t', '\n', '\u{b}', '\u{c}', '\r', '\u{1c}', '\u{1d}', '\u{1e}', '\u{1f}',
            '\u{1680}', '\u{180e}', '\u{2000}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{205f}',
            '\u{3000}',
        ] {
            assert!(java_is_whitespace(c), "U+{:04X} 应为空白", c as u32);
        }
        for c in ['\u{85}', '\u{a0}', '\u{2007}', '\u{202f}', '\u{feff}', '\u{1b}', 'a', '0'] {
            assert!(!java_is_whitespace(c), "U+{:04X} 不应为空白", c as u32);
        }
    }

    #[test]
    fn nbsp_is_not_a_delimiter() {
        // U+00A0/U+202F 是 Java 非空白 → 原子的一部分 (与 Rust is_whitespace 相反, 保真点)
        for ws in ['\u{a0}', '\u{202f}'] {
            let src = format!("a{}b", ws);
            let e = parse_one(&src);
            let a = e.as_atom();
            assert!(a.is_symbol());
            assert_eq!(a.get_string(), src);
        }
    }

    #[test]
    fn info_separators_and_mongolian_vowel_split_atoms() {
        // U+001C..U+001F/U+180E 在 JDK8 是空白 → 切分原子 (Rust 原生不切, 保真点)
        for ws in ['\u{1c}', '\u{1f}', '\u{180e}'] {
            let src = format!("a{}b", ws);
            assert_eq!(parse_str(&src), vec!["a", "b"]);
        }
        assert_eq!(parse_str("a\r\nb"), vec!["a", "b"]);
        assert_eq!(parse_str("a\rb"), vec!["a", "b"]);
    }

    // ---- SAtom getters (Java 8 oracle 数值) ----

    #[test]
    fn get_double_oracle_table() {
        let cases = [
            ("123", 123.0),
            ("12.34", 12.34),
            (" 42 ", 42.0),        // parseDouble 隐含 trim
            ("\t+5\n", 5.0),
            (".5", 0.5),
            ("5.", 5.0),
            ("+.5", 0.5),
            ("1e5", 100000.0),
            ("1E-5", 1.0e-5),
            ("5f", 5.0),
            ("5d", 5.0),
            ("1.5F", 1.5),
            ("5e2d", 500.0),
            ("5.e2", 500.0),
            (".5f", 0.5),
            ("5.d", 5.0),
            ("0x1p1", 2.0),
            ("0X1.8P1", 3.0),
            ("0x.8p1", 1.0),
            ("0x1.p1", 2.0),
            ("0x8.p1", 16.0),
            ("0x1p1f", 2.0),
            ("0x1p-2", 0.25),
            ("-0x1p2", -4.0),
            ("+0x1p1", 2.0),
            ("0x1P+2", 4.0),
            ("0x1p-1075", 0.0), // oracle: 舍入到 0 (min subnormal 的一半, round-half-even)
            ("2147483647.9", 2147483647.9),
        ];
        for (s, want) in cases {
            let a = SAtom::new(s.into(), AtomType::Number);
            let got = a.get_double();
            assert!(
                (got - want).abs() < f64::EPSILON * want.abs().max(1.0),
                "{} → {} != {}",
                s,
                got,
                want
            );
        }
        // 特殊值
        assert!(SAtom::new("NaN".into(), AtomType::Number).get_double().is_nan());
        assert!(SAtom::new("-NaN".into(), AtomType::Number).get_double().is_nan());
        assert_eq!(SAtom::new("Infinity".into(), AtomType::Number).get_double(), f64::INFINITY);
        assert_eq!(SAtom::new("-Infinity".into(), AtomType::Number).get_double(), f64::NEG_INFINITY);
        assert_eq!(SAtom::new("1e310".into(), AtomType::Number).get_double(), f64::INFINITY);
        // 次正规数: 0 < 1e-310 < 最小正规数
        let sub = SAtom::new("1e-310".into(), AtomType::Number).get_double();
        assert!(sub > 0.0 && sub < f64::MIN_POSITIVE);
    }

    #[test]
    fn get_double_rejects_like_java() {
        for s in [
            "", "-", "+", "1e", "1e+", "1_000", "5,5", "nan", "infinity", "INF", "0x8", "0x8f",
            "0x1p", "0x.p1", "5-", "..5", "5..", "e5", "E5", "+.e5", ".e2", "00x1p1", "0 x1",
            "0x1p 2", "1e 5", "--5", "true", "5.5.5",
        ] {
            assert!(java_parse_double(s).is_err(), "[{}] 应抛 NumberFormatException", s);
        }
    }

    #[test]
    #[should_panic(expected = "For input string")]
    fn get_double_panics_like_java_number_format_exception() {
        // STRING 原子非数字 → Java NumberFormatException (未受检) 传播
        SAtom::new("abc".into(), AtomType::String).get_double();
    }

    #[test]
    #[should_panic(expected = "empty String")]
    fn get_double_panics_on_empty_string() {
        // Java 8 oracle: parseDouble("")/parseDouble("   ") 抛 NumberFormatException:
        // empty String (小写 e) — 与非空非法串的 "For input string" 消息分支不同
        SAtom::new(String::new(), AtomType::String).get_double();
    }

    #[test]
    #[should_panic(expected = "empty String")]
    fn get_double_panics_on_whitespace_only_string() {
        SAtom::new("   ".into(), AtomType::String).get_double();
    }

    #[test]
    fn hex_extreme_exponent_bit_exact() {
        // Java 8 oracle (1.8.0_342) doubleToLongBits 逐例核对 — 单次舍入语义。
        // 直接 `m as f64 * 2f64.powi(shift)` 整体求幂会提前下溢: 前三例旧实现
        // 分别得 0.0/0.0/2.2250738585072014e-308 (2 倍偏差)
        let cases = [
            ("0x40p-1080", 0x1u64),                      // 4.9E-324 最小次正规
            ("0x1fffffffffffff8p-1077", 0x30000000000000), // 8.900295434028806E-308
            ("0x10000000000000p-1075", 0x8000000000000),  // 1.1125369292536007E-308
            ("0x3p-1075", 0x2),                           // 1.0E-323 half-even 舍入
            ("0x1p-1074", 0x1),                           // 4.9E-324
            ("0x1p-1075", 0x0),                           // 半 ulp 舍入到 0 (偶)
            ("0x1p1023", 0x7fe0000000000000),             // 最大正规指数
            ("0x1p1024", 0x7ff0000000000000),             // Infinity
            ("0x1p-2000", 0x0),                           // 深度下溢
            ("0x7fp1", 0x406fc00000000000),               // 254.0 常规域回归
            ("0x1.0000000000001p0", 0x3ff0000000000001),  // 1.0000000000000002
        ];
        for (s, bits) in cases {
            let got = java_parse_double(s).unwrap();
            assert_eq!(got.to_bits(), bits, "{} → {:x} != {:x}", s, got.to_bits(), bits);
        }
    }

    #[test]
    fn get_int_jls_saturation_semantics() {
        // Java (int) double = JLS 5.1.3; Rust f64 as i32 同义 — oracle 逐值核对
        let cases = [
            ("3.99", 3),
            ("-3.99", -3),
            ("0.9999999999", 0),
            ("1e10", 2147483647),
            ("-1e10", i32::MIN),
            ("2.5e9", 2147483647),
            ("-2.5e9", i32::MIN),
            ("2147483647.9", 2147483647),
            ("-2147483648.9", i32::MIN),
            ("NaN", 0),
            ("Infinity", 2147483647),
            ("-Infinity", i32::MIN),
            ("9999", 9999),
        ];
        for (s, want) in cases {
            let a = SAtom::new(s.into(), AtomType::Number);
            assert_eq!(a.get_int(), want, "{}", s);
        }
    }

    #[test]
    fn get_bool_matches_java_parse_boolean() {
        // oracle: "TRUE"/"True" → true; 带空格/其他串 → false
        for s in ["true", "TRUE", "True"] {
            assert!(SAtom::new(s.into(), AtomType::Boolean).get_bool(), "{}", s);
        }
        for s in [" false", "false ", "yes", "", "truetrue", "ｔｒｕｅ", "false"] {
            assert!(!SAtom::new(s.into(), AtomType::Boolean).get_bool(), "{}", s);
        }
    }

    #[test]
    fn get_string_and_type_predicates() {
        let kw = SAtom::new(":type".into(), AtomType::Keyword);
        assert!(kw.is_keyword() && !kw.is_symbol());
        assert_eq!(kw.get_string(), ":type");
        let sym = SAtom::new("panel".into(), AtomType::Symbol);
        assert!(sym.is_symbol() && !sym.is_keyword());
        assert_eq!(sym.get_string(), "panel");
    }

    // ---- asList/asAtom 异常路径 ----

    #[test]
    #[should_panic(expected = "Not a list")]
    fn as_list_on_atom_panics() {
        parse_one("a").as_list();
    }

    #[test]
    #[should_panic(expected = "Not an atom")]
    fn as_atom_on_list_panics() {
        parse_one("(a)").as_atom();
    }

    // ---- Display (Java toString) ----

    #[test]
    fn display_list_joins_with_single_space() {
        let mut l = SList::new();
        l.add(Rc::new(SExp::Atom(SAtom::new("a".into(), AtomType::Symbol))));
        l.add(Rc::new(SExp::Atom(SAtom::new("b".into(), AtomType::Symbol))));
        l.add(Rc::new(SExp::Atom(SAtom::new("c".into(), AtomType::Symbol))));
        assert_eq!(l.to_string(), "(a b c)");
    }

    #[test]
    fn display_string_atom_does_not_reescape() {
        // Java toString: 直接拼引号, 内部引号不转义 (忠实保留原行为)
        let a = SAtom::new("he\"llo".into(), AtomType::String);
        assert_eq!(a.to_string(), "\"he\"llo\"");
        // 键值/数字/布尔原子原样输出
        for (v, t) in [
            (":type", AtomType::Keyword),
            ("12.34", AtomType::Number),
            ("true", AtomType::Boolean),
            ("panel", AtomType::Symbol),
        ] {
            assert_eq!(SAtom::new(v.into(), t).to_string(), v);
        }
    }

    #[test]
    fn display_is_virtual_dispatch() {
        // Rc<SExp> 的 Display 派发到运行时类型 (对应 Java toString 虚分派)
        let inner = Rc::new(SExp::List(SList::new()));
        let mut outer = SList::new();
        outer.add(inner.clone());
        outer.add(Rc::new(SExp::Atom(SAtom::new("x".into(), AtomType::Symbol))));
        assert_eq!(outer.to_string(), "(() x)");
        assert_eq!(Rc::new(SExp::List(outer)).to_string(), "(() x)");
    }

    // ---- :na-when 语义 (TestNaWhenParsing.java 用例移植) ----

    #[test]
    fn na_when_expression_structure() {
        // ui_layout.cfg 转半径: :na-when (> value 9999)
        let e = parse_one("(> value 9999)");
        let l = e.as_list();
        assert_eq!(l.children.len(), 3);
        assert_eq!(l.children[0].as_atom().get_string(), ">");
        assert!(l.children[0].as_atom().is_symbol());
        assert_eq!(l.children[1].as_atom().get_string(), "value");
        let n = l.children[2].as_atom();
        assert_eq!(n.r#type, AtomType::Number);
        assert_eq!(n.get_double(), 9999.0);
        assert_eq!(n.get_int(), 9999);

        // 复合表达式 (visible-when 形态): (and (not (isJetEngine)) (> value 0))
        let e = parse_one("(and (not (isJetEngine)) (> value 0))");
        let l = e.as_list();
        assert_eq!(l.children.len(), 3);
        assert_eq!(l.children[0].as_atom().get_string(), "and");
        assert_eq!(l.children[1].to_string(), "(not (isJetEngine))");
        assert_eq!(l.children[2].to_string(), "(> value 0)");
        // 与 Java toString 一致 — ConfigLoader.saveConfig 按此回写 cfg
        assert_eq!(
            e.to_string(),
            "(and (not (isJetEngine)) (> value 0))"
        );
    }

    /// 模拟 ConfigLoader.getKeywordSExp: 递归收集 keyword 后一个兄弟节点
    fn collect_keyword_values(exprs: &[Rc<SExp>], keyword: &str, out: &mut Vec<Rc<SExp>>) {
        for e in exprs {
            if let SExp::List(l) = &**e {
                let n = l.children.len();
                for i in 0..n {
                    if i + 1 < n {
                        if let SExp::Atom(a) = &*l.children[i] {
                            if a.is_keyword() && a.get_string().eq_ignore_ascii_case(keyword) {
                                out.push(l.children[i + 1].clone());
                            }
                        }
                    }
                }
                collect_keyword_values(&l.children, keyword, out);
            }
        }
    }

    #[test]
    fn ui_layout_cfg_na_when_expressions_parsed() {
        // TestNaWhenParsing.java 移植: 加载 ui_layout.cfg, 断言 :na-when / :visible-when
        // 的值都解析成了非空列表 (对应 "naWhen 表达式已解析!" 而非 "[警告] naWhen 为 null!")
        let cfg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ui_layout.cfg");
        let content = fs::read_to_string(&cfg_path).expect("ui_layout.cfg 应在仓库根");
        let mut parser = SExpParser::new();
        let panels = parser.parse(&content);
        assert!(!panels.is_empty(), "cfg 应解析出顶层 panel");

        for keyword in [":na-when", ":visible-when"] {
            let mut values = Vec::new();
            collect_keyword_values(&panels, keyword, &mut values);
            assert!(
                !values.is_empty(),
                "{} 在 ui_layout.cfg 中应存在",
                keyword
            );
            for v in &values {
                assert!(v.is_list(), "{} 的值应为表达式列表", keyword);
                assert!(
                    !v.as_list().children.is_empty(),
                    "{} 表达式不应为空列表",
                    keyword
                );
            }
            if keyword == ":na-when" {
                // 当前 cfg 有 7 处 :na-when (grep 核对), 快照防回归
                assert!(values.len() >= 7, ":na-when 数量 {}", values.len());
                let reprs: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                for expect in [
                    "(> value 9999)",
                    "(<= value 0)",
                    "(= value -65535)",
                    "(> value 90000)",
                    "(<= value -65535)",
                ] {
                    assert!(reprs.iter().any(|r| r == expect), "缺 {}", expect);
                }
            }
        }

        // 对应 TestNaWhenParsing 的搜索目标: 转半径行 (target 含 TurnRadius) 的
        // :na-when 表达式确已解析为 (> value 9999)
        let found = find_turn_radius_na_when(&panels);
        assert_eq!(found.as_deref(), Some("(> value 9999)"));
    }

    fn find_turn_radius_na_when(exprs: &[Rc<SExp>]) -> Option<String> {
        fn walk(e: &SExp) -> Option<String> {
            let SExp::List(l) = e else {
                return None;
            };
            let has_target = l.children.iter().any(|c| {
                matches!(
                    &**c,
                    SExp::Atom(a) if a.r#type == AtomType::String && a.get_string() == "getTurnRadius"
                )
            });
            if has_target {
                let n = l.children.len();
                for i in 0..n {
                    if i + 1 < n {
                        if let SExp::Atom(a) = &*l.children[i] {
                            if a.is_keyword() && a.get_string().eq_ignore_ascii_case(":na-when") {
                                return Some(l.children[i + 1].to_string());
                            }
                        }
                    }
                }
            }
            l.children.iter().find_map(|c| walk(c))
        }
        exprs.iter().find_map(|e| walk(e))
    }
}
