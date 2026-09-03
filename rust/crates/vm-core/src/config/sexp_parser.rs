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
//! Java `interface SExp` (仅 SList/SAtom 两个封闭实现) → `enum SExp`。
//! Java 引用可别名 — ConfigLoader.getKeywordSExp 把子树原对象直接存进
//! RowConfig.visibleWhen / naWhen — 故节点统一 `Rc<SExp>` 共享。
//! Rc 而非 Arc: 配置解析单线程。注意 Rc<SExp> 树 Send 但 !Sync —
//! 可整体 move 跨线程交接 (如热重载线程→UI 线程), 禁止跨线程共享同一棵树。
//! Java 枚举常量全大写 (STRING/LPAREN) → Rust 驼峰 (String/LParen), 语义不变。

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
    /// 未受检异常 → panic
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
    /// 非法数值 atom 是 cfg 语法错误 → panic (由 load_config 的 catch_unwind 兜住)。
    /// (波22: 原 Java NumberFormatException 消息复刻退役)
    pub fn get_double(&self) -> f64 {
        parse_double(&self.value).unwrap_or_else(|()| panic!("非法数值 atom: {:?}", self.value))
    }

    /// Java: `public int getInt() { return (int) getDouble(); }`
    /// Rust `f64 as i32` = JLS 5.1.3 (NaN→0, ±Inf/越界饱和到 i32 极值, 向零截断)
    /// — 历史基线逐值一致 (3.99→3 / 1e10→MAX / NaN→0)
    pub fn get_int(&self) -> i32 {
        self.get_double() as i32
    }

    /// Java: `public boolean getBool() { return Boolean.parseBoolean(value); }`
    /// 即 equalsIgnoreCase("true") — 历史基线: "TRUE"/"True"→true, 带空格→false;
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

/// Java `Character.isWhitespace(char)` 复刻 (JDK 8, build/历史基线)。
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

/// 数值解析 (波22: Java Double.parseDouble 一比一复刻退役, std `str::parse` 语义)。
/// 前置 java_trim 保留 (cfg 值可带首尾空白, Rust parse 不 trim)。
/// 随复刻退役的 Java 域特性 — cfg 值域 (普通十进制) 不可达, 已 grep 验证:
/// 十六进制浮点 ("0x1p1")、f/d 尾缀 ("1.5f")、大小写敏感的 NaN/Infinity 精确匹配。
fn parse_double(s: &str) -> Result<f64, ()> {
    java_trim(s).parse::<f64>().map_err(|_| ())
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

        // §2.1 — Java charAt 按 UTF-16 码元推进; 此处收集为 Vec<char> 按码点索引,
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
        parse_double(s).is_ok()
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
