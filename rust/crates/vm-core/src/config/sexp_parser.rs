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

use crate::base::java_compat::java_trim;

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
            Err(ParseDoubleErr::Invalid) => {
                panic!("For input string: \"{}\"", java_trim(&self.value))
            }
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
    let t = java_trim(s);
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
        return Ok(if neg {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
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
            e10 = e10.wrapping_mul(10).wrapping_add(i64::from(b - b'0'));
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
mod tests;
