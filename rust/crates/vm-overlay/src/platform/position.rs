//! Rust 版独立配置 (POC 不解析 ui_layout.cfg S-expression)
//! 位置持久化: rust/user_pos.json 归一化坐标 (与 Java ui_layout.user.cfg :x/:y 同语义)

use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct UserPos {
    /// 归一化 (0.0-1.0, 相对屏幕)
    pub x: f64,
    pub y: f64,
}

fn pos_path() -> PathBuf {
    PathBuf::from("user_pos.json")
}

pub fn load_pos() -> Option<UserPos> {
    let text = std::fs::read_to_string(pos_path()).ok()?;
    let x = parse_json_number(&text, "x")?;
    let y = parse_json_number(&text, "y")?;
    Some(UserPos { x, y })
}

pub fn save_pos(x: f64, y: f64) {
    let text = format!("{{\"x\": {:.4}, \"y\": {:.4}}}\n", x, y);
    if let Err(e) = std::fs::write(pos_path(), text) {
        eprintln!("警告: 保存位置失败: {}", e);
    }
}

/// 极简 JSON 数值提取 ("\"x\": 0.0602"), 避免为此引 serde
fn parse_json_number(text: &str, key: &str) -> Option<f64> {
    let marker = format!("\"{}\"", key);
    let i = text.find(&marker)?;
    let rest = &text[i + marker.len()..];
    let j = rest.find(':')?;
    let num_part: String = rest[j + 1..]
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == 'e' || *c == 'E' || *c == '+')
        .collect();
    num_part.parse().ok()
}
