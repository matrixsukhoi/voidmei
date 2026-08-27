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
mod tests;
