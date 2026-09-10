//! 公式语言分词器: 中缀表达式 → Token 流 (错误带行列)。
//! 语法: doc/formula_system_design.md §3.1; 注释 `// 到行尾`

/// 词法单元
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
    Comma,
    Question,
    Colon,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Not,
}

/// 词法错误 (行/列 1 基)
#[derive(Debug, Clone)]
pub struct LexError {
    pub line: usize,
    pub col: usize,
    pub msg: String,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.msg)
    }
}

/// 分词主入口
pub fn lex(src: &str) -> Result<Vec<Tok>, LexError> {
    let mut toks = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    macro_rules! err {
        ($msg:expr) => {
            return Err(LexError {
                line,
                col,
                msg: $msg.to_string(),
            })
        };
    }

    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            // 注释: // 到行尾
            '/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                    col += 1;
                }
            }
            '\n' => {
                i += 1;
                line += 1;
                col = 1;
            }
            c if c.is_ascii_whitespace() => {
                i += 1;
                col += 1;
            }
            c if c.is_ascii_digit()
                || (c == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit()) =>
            {
                // 数字: [0-9]+("." [0-9]+)? ([eE][+-]?[0-9]+)?
                let start = i;
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
                if i < bytes.len()
                    && bytes[i] == b'.'
                    && i + 1 < bytes.len()
                    && (bytes[i + 1] as char).is_ascii_digit()
                {
                    i += 1;
                    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                        i += 1;
                    }
                }
                if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                    let mut j = i + 1;
                    if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
                        j += 1;
                    }
                    if j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
                        i = j;
                        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                let text = &src[start..i];
                let v: f64 = text.parse().map_err(|_| LexError {
                    line,
                    col,
                    msg: format!("非法数字字面量: {text}"),
                })?;
                toks.push(Tok::Num(v));
                col += text.len();
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let text = &src[start..i];
                toks.push(Tok::Ident(text.to_string()));
                col += text.len();
            }
            '+' => {
                toks.push(Tok::Plus);
                i += 1;
                col += 1;
            }
            '-' => {
                toks.push(Tok::Minus);
                i += 1;
                col += 1;
            }
            '*' => {
                toks.push(Tok::Star);
                i += 1;
                col += 1;
            }
            '/' => {
                toks.push(Tok::Slash);
                i += 1;
                col += 1;
            }
            '%' => {
                toks.push(Tok::Percent);
                i += 1;
                col += 1;
            }
            '^' => {
                toks.push(Tok::Caret);
                i += 1;
                col += 1;
            }
            '(' => {
                toks.push(Tok::LParen);
                i += 1;
                col += 1;
            }
            ')' => {
                toks.push(Tok::RParen);
                i += 1;
                col += 1;
            }
            ',' => {
                toks.push(Tok::Comma);
                i += 1;
                col += 1;
            }
            '?' => {
                toks.push(Tok::Question);
                i += 1;
                col += 1;
            }
            ':' => {
                toks.push(Tok::Colon);
                i += 1;
                col += 1;
            }
            '=' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                toks.push(Tok::EqEq);
                i += 2;
                col += 2;
            }
            '!' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                toks.push(Tok::NotEq);
                i += 2;
                col += 2;
            }
            '!' => {
                toks.push(Tok::Not);
                i += 1;
                col += 1;
            }
            '<' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                toks.push(Tok::Le);
                i += 2;
                col += 2;
            }
            '<' => {
                toks.push(Tok::Lt);
                i += 1;
                col += 1;
            }
            '>' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                toks.push(Tok::Ge);
                i += 2;
                col += 2;
            }
            '>' => {
                toks.push(Tok::Gt);
                i += 1;
                col += 1;
            }
            '&' if i + 1 < bytes.len() && bytes[i + 1] == b'&' => {
                toks.push(Tok::AndAnd);
                i += 2;
                col += 2;
            }
            '|' if i + 1 < bytes.len() && bytes[i + 1] == b'|' => {
                toks.push(Tok::OrOr);
                i += 2;
                col += 2;
            }
            _ => err!(format!("非法字符: {c:?}")),
        }
    }
    Ok(toks)
}
