//! HudMsg 的 Rust 移植 (src/parser/HudMsg.java)
//! http://127.0.0.1:8111/hudmsg?lastEvt=0&lastDmg=0 — 击杀/警告消息解析。
//!
//! PORT: §2.1 — 消息文本含 CJK (中文击杀消息), 故本文件索引一律按 char (字符)
//! 而非字节 — Java charAt/substring 按 UTF-16 码元, BMP 域 1 码元 = 1 字符等价;
//! 增补面 (astral) 字符 Java 计 2 码元、Rust 计 1 字符, 域内不可达。
//! Java 内部类 events/damage → 独立 struct (无外部类引用使用点)。

/// Java `public class events` (空类, 占位)
pub struct Events {}

/// Java `public class damage` — 伤害消息载荷; sender/enemy/mode 在 update/parseObj
/// 中从不赋值, 保持 Java 默认 (null/false)
#[derive(Default)]
pub struct Damage {
    pub id: i32,
    pub msg: Option<String>,
    pub sender: Option<String>,
    pub enemy: bool,
    pub mode: Option<String>,
    pub updated: bool,
}


pub struct HudMsg {
    s: String,
    /// Java `public damage dmg` — init() 前 null → Option; 未 init 即 update 会像
    /// Java 一样在 dmg 字段访问处 panic (NPE)
    pub dmg: Option<Damage>,
}

impl HudMsg {
    pub fn new() -> Self {
        HudMsg {
            s: String::new(),
            dmg: None,
        }
    }

    /// Java `String getDmglastLine()`: 取最后一个 {...} 对象切片 (去掉结尾 "]}" 2 字符)
    fn get_dmg_last_line(&self) -> String {
        // PORT: chars() ≈ UTF-16 码元 (BMP), 见模块注释
        let cs: Vec<char> = self.s.chars().collect();
        if cs.len() > 30 {
            let eix = cs.len() - 2;
            let mut bix = eix - 1;
            while cs[bix] != '{' {
                bix -= 1;
            }
            cs[bix..eix].iter().collect()
        } else {
            String::new()
        }
    }

    /// Java `String getLine(String a)`: 定位 a 后取首个 {...} 切片 (update 未使用,
    /// 保留公共行为)
    #[allow(dead_code)] // Java 同为死代码 (调用点已注释), 保真保留
    fn get_line(&self, a: &str) -> String {
        let cs: Vec<char> = self.s.chars().collect();
        let needle: Vec<char> = a.chars().collect();
        // Java: s.indexOf(a) — 首次出现, 无则 -1
        let mut bix: i32 = -1;
        if needle.len() <= cs.len() {
            for i in 0..=(cs.len() - needle.len()) {
                if cs[i..i + needle.len()] == needle[..] {
                    bix = i as i32;
                    break;
                }
            }
        }
        if bix != -1 {
            let mut eix = (bix + 1) as usize;
            while cs[eix] != '{' {
                eix += 1;
                if cs[eix] == ']' {
                    return String::new();
                }
            }
            let start = eix;
            eix += 1;
            while cs[eix] != '}' {
                eix += 1;
            }
            eix += 1;
            cs[start..eix].iter().collect()
        } else {
            String::new()
        }
    }

    pub fn parse_obj(&mut self, buf: &str) -> i32 {
        // Application.debugPrint(buf);
        let cs: Vec<char> = buf.chars().collect();
        let mut bix: usize;
        let mut eix: usize = 0;
        // id
        if cs.len() > 20 {
            // PORT: Java dmg 未 init 时此处 NPE ↔ unwrap panic
            let dmg = self.dmg.as_mut().unwrap();
            while cs[eix] != ':' {
                eix += 1;
            }
            eix += 1;
            eix += 1;
            bix = eix;
            while cs[eix] != ',' {
                eix += 1;
            }
            // Java: Integer.parseInt — 不 trim, 坏输入抛 NumberFormatException ↔ panic
            dmg.id = cs[bix..eix].iter().collect::<String>().parse::<i32>().unwrap();

            eix += 1;

            while cs[eix] != ':' {
                eix += 1;
            }
            eix += 1;
            while cs[eix] != '"' {
                eix += 1;
            }
            eix += 1;
            bix = eix;
            while cs[eix] != '"' {
                eix += 1;
            }
            dmg.msg = Some(cs[bix..eix].iter().collect());
            1
        } else {
            0
        }
    }

    pub fn init(&mut self) {
        //Application.debugPrint("hudMSG初始化了");
        self.dmg = Some(Damage::default());
    }

    pub fn update(&mut self, s: &str, last_dmg: i32) -> i32 {
        self.s = s.to_string();
        // Application.debugPrint(S);
        //String buf = getLine("damage");
        self.dmg.as_mut().unwrap().updated = false;
        let lastbuf = self.get_dmg_last_line();
        //Application.debugPrint(lastbuf);
        if self.parse_obj(&lastbuf) == 1 {
            //Application.debugPrint(dmg.id + " " + dmg.msg);
            self.dmg.as_mut().unwrap().updated = true;
            self.dmg.as_ref().unwrap().id
        } else {
            last_dmg
        }
    }
}

impl Default for HudMsg {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// mock 8111 线上格式; 断言值 = Java 8 oracle 实测
    const HUDMSG_MOCK: &str = "{\"events\": [],\"damage\": [{\"id\": 532213658,\"msg\": \"player1_VS_player2\",\"sender\": \"someone\",\"enemy\": true,\"mode\": \"ES\"}]}";
    const HUDMSG_MULTI: &str = "{\"events\": [],\"damage\": [{\"id\": 111,\"msg\": \"first\"}, {\"id\": 222,\"msg\": \"second 热msg\"}]}";

    #[test]
    fn update_parses_last_damage_object() {
        let mut hm = HudMsg::new();
        hm.init();
        assert_eq!(hm.update(HUDMSG_MOCK, 777), 532213658);
        let dmg = hm.dmg.as_ref().unwrap();
        assert_eq!(dmg.id, 532213658);
        assert_eq!(dmg.msg.as_deref(), Some("player1_VS_player2"));
        // sender/enemy/mode 从不赋值 → Java 默认 (oracle 实测 null/false)
        assert!(dmg.sender.is_none());
        assert!(!dmg.enemy);
        assert!(dmg.mode.is_none());
        assert!(dmg.updated);
    }

    #[test]
    fn update_multi_damage_takes_last_line() {
        // getDmglastLine 取末尾对象 (去掉 "]}" 后回扫 '{'), CJK 消息按字符定位
        let mut hm = HudMsg::new();
        hm.init();
        assert_eq!(hm.update(HUDMSG_MULTI, 0), 222);
        let dmg = hm.dmg.as_ref().unwrap();
        assert_eq!(dmg.id, 222);
        assert_eq!(dmg.msg.as_deref(), Some("second 热msg"));
        assert!(dmg.updated);
    }

    #[test]
    fn update_short_payload_returns_last_dmg() {
        // s.length() <= 30 → getDmglastLine 返回 "" → parseObj 返回 0 → 返回 lastDmg
        let mut hm = HudMsg::new();
        hm.init();
        assert_eq!(hm.update("{\"damage\": []}", 777), 777);
        assert!(!hm.dmg.as_ref().unwrap().updated);
        assert_eq!(hm.dmg.as_ref().unwrap().id, 0);
        assert!(hm.dmg.as_ref().unwrap().msg.is_none());
    }

    #[test]
    fn update_empty_damage_array_returns_last_dmg() {
        let mut hm = HudMsg::new();
        hm.init();
        assert_eq!(hm.update("{\"events\": [],\"damage\": []}", 42), 42);
        assert!(!hm.dmg.as_ref().unwrap().updated);
    }

    #[test]
    fn parse_obj_short_returns_zero() {
        let mut hm = HudMsg::new();
        hm.init();
        assert_eq!(hm.parse_obj("{\"id\": 123}"), 0); // length <= 20
    }

    #[test]
    fn get_line_extracts_object_slice() {
        // update 未调用的公共路径 (原为取 "damage" 数组首对象):
        // 从定位点后扫到 '{', 再扫到首个 '}' (对象无嵌套), 返回含两端花括号的整段
        let mut hm = HudMsg::new();
        hm.init();
        hm.update(HUDMSG_MOCK, 0);
        let line = hm.get_line("damage");
        assert_eq!(
            line,
            "{\"id\": 532213658,\"msg\": \"player1_VS_player2\",\"sender\": \"someone\",\"enemy\": true,\"mode\": \"ES\"}"
        );
    }

    #[test]
    fn get_line_missing_returns_empty() {
        let mut hm = HudMsg::new();
        hm.init();
        hm.update("{\"foo\": 1}", 0);
        assert_eq!(hm.get_line("damage"), "");
    }

    #[test]
    fn init_creates_default_damage() {
        let mut hm = HudMsg::new();
        assert!(hm.dmg.is_none()); // new 后未 init ≈ Java null
        hm.init();
        let dmg = hm.dmg.as_ref().unwrap();
        assert_eq!(dmg.id, 0);
        assert!(!dmg.updated);
    }
}
