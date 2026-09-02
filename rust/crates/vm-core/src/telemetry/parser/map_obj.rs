//! MapObj 的 Rust 移植 (src/parser/MapObj.java)
//! /map_obj.json 地图对象 (载具/图标/坐标) 的手写子串扫描解析 + Player 定位正则。
//!
//! PORT: §2.1 — Java charAt/substring 按 UTF-16 码元; 本域 (对象 type/icon 为游戏内部
//! 标识符, 数值/十六进制色) 纯 ASCII, 字节索引 + 整字符步进与 Java 逐码元推进等价
//! (mod.rs 公共 helper)。
//! PORT: java.awt.Color 仅作数据字段 → [u8;4] RGBA (POC 先例), new Color(r,g,b) 的
//! alpha 固定 255。
//! PORT: Java 内部类 Movobj/Staobj/Plaobj/Slcobj 无外部类引用使用点 → 独立 struct。
//!
//! 域内格式备注 (Java 8 oracle 实测): parseObj 的位置偏移按**紧凑 JSON** (冒号后无
//! 空格) 设计 — 现行 mock/游戏 `": "` 间隔格式会在 color[] 的 parseInt 处抛
//! NumberFormatException (由上层轮询线程兜住, OtherService.run 无 catch 会终止线程),
//! 保真保留; 玩家定位实际由 getPlayerLoc/getPlayerDir 正则路径承担 (Service 在用)。

use super::char_len_at;
use crate::base::string_helper::{get_data_float, get_data_int};

/// Java `while (t.charAt(eix) != c) eix++;` 的整字符步进等价 (§2.1, 域 ASCII)
macro_rules! scan_until {
    ($t:expr, $eix:expr, $c:expr) => {
        while $t.as_bytes()[$eix as usize] != $c {
            $eix += char_len_at($t, $eix as usize) as i32;
        }
    };
}

/// Java `while (t.charAt(eix) != c1 && t.charAt(eix) != c2) eix++;` 版本
macro_rules! scan_until2 {
    ($t:expr, $eix:expr, $c1:expr, $c2:expr) => {
        while $t.as_bytes()[$eix as usize] != $c1 && $t.as_bytes()[$eix as usize] != $c2 {
            $eix += char_len_at($t, $eix as usize) as i32;
        }
    };
}

pub struct Movobj {
    pub r#type: Option<String>,
    pub color: Option<String>,
    pub colorg: Option<[u8; 4]>,
    pub blink: i32,
    pub distance: f64,
    pub icon: Option<String>,
    pub icon_bg: Option<String>,
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
}

impl Default for Movobj {
    fn default() -> Self {
        Movobj {
            r#type: None,
            color: None,
            colorg: None,
            blink: 0,
            distance: 0.0,
            icon: None,
            icon_bg: None,
            x: 0.0,
            y: 0.0,
            dx: 0.0,
            dy: 0.0,
        }
    }
}

pub struct Staobj {
    pub r#type: Option<String>,
    pub color: Option<String>,
    pub colorg: Option<[u8; 4]>,
    pub blink: i32,
    pub icon: Option<String>,
    pub icon_bg: Option<String>,
    pub x: f64,
    pub y: f64,
}

impl Default for Staobj {
    fn default() -> Self {
        Staobj {
            r#type: None,
            color: None,
            colorg: None,
            blink: 0,
            icon: None,
            icon_bg: None,
            x: 0.0,
            y: 0.0,
        }
    }
}

pub struct Plaobj {
    pub r#type: Option<String>,
    pub color: Option<String>,
    pub colorg: Option<[u8; 4]>,
    pub blink: i32,
    pub icon: Option<String>,
    pub icon_bg: Option<String>,
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
}

impl Default for Plaobj {
    fn default() -> Self {
        Plaobj {
            r#type: None,
            color: None,
            colorg: None,
            blink: 0,
            icon: None,
            icon_bg: None,
            x: 0.0,
            y: 0.0,
            dx: 0.0,
            dy: 0.0,
        }
    }
}

pub struct Slcobj {
    pub r#type: Option<String>,
    pub color: Option<String>,
    pub colorg: Option<[u8; 4]>,
    pub blink: i32,
    pub icon: Option<String>,
    pub icon_bg: Option<String>,
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
}

impl Default for Slcobj {
    fn default() -> Self {
        Slcobj {
            r#type: None,
            color: None,
            colorg: None,
            blink: 0,
            icon: None,
            icon_bg: None,
            x: 0.0,
            y: 0.0,
            dx: 0.0,
            dy: 0.0,
        }
    }
}

pub struct MapObj {
    num: usize,
    pub mov: Vec<Movobj>,
    pub movcur: i32,
    pub sta: Vec<Staobj>,
    pub pla: Plaobj,
    pub slc: Slcobj,
    pub stacur: i32,
    pub aot: f64,
    s: String,
}

impl MapObj {
    /// 对应 Java `new MapObj()`: 数组未分配 (≈ null), init 前调用 update 会 panic。
    /// PORT: Java `pla`/`slc` 也是裸声明 (null), 未 init 的 update 在 `slc.type=""` 即
    /// NPE; Rust default 实例静默继续 — 生产路径 OtherService 恒先 init, 不可达偏差
    pub fn new() -> Self {
        MapObj {
            num: 0,
            mov: Vec::new(),
            movcur: 0,
            sta: Vec::new(),
            pla: Plaobj::default(),
            slc: Slcobj::default(),
            stacur: 0,
            aot: 0.0,
            s: String::new(),
        }
    }

    fn get_line(&mut self) -> String {
        let eix: i32;
        let buf: String;
        // PORT: Java 原地改 s (substring 赋回), 此处克隆避免借用冲突, 语义不变
        let s = self.s.clone();
        let bix: i32 = s.find('{').map_or(-1, |v| v as i32);
        if bix != -1 {
            eix = s.find('}').map_or(-1, |v| v as i32);
            buf = s[bix as usize..(eix + 1) as usize].to_string();
            // Application.debugPrint("切片值"+buf);
            self.s = s[(eix + 1) as usize..].to_string();
            // Application.debugPrint("切片后"+s);
            buf
        } else {
            String::new()
        }
    }

    fn parse_obj(&mut self, t: &str) {
        let mut bix: i32;
        let mut eix: i32;
        let quoteloc = 1;
        // Java 局部变量声明不初始化、分支内赋值 — Rust 延迟初始化对应 (§2.10)
        let flag: i32;
        let r#type: String;
        let color: String;
        let colorg: [u8; 4];
        let blink: i32;
        let icon: String;
        let icon_bg: String;
        let x: f64;
        let y: f64;
        let dx: f64;
        let dy: f64;

        let mut is_player = false;
        let mut is_selected = false;
        // 先找type
        bix = t.find('"').map_or(-1, |v| v as i32);
        if bix != -1 {
            eix = bix + 5;
            scan_until!(t, eix, b':');
            bix = eix + 1 + quoteloc;
            eix = bix;
            scan_until!(t, eix, b'"');
            r#type = t[bix as usize..eix as usize].to_string();
            // Application.debugPrint(type.charAt(3));
            // 继续向下搜索Color
            eix += 2;
            // 找三个引号
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            bix = eix;
            scan_until!(t, eix, b'"');
            color = t[bix as usize..eix as usize].to_string();

            // 继续向下搜索Color[]
            eix += 2;
            // 找2个引号
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'[');
            eix += 1;
            bix = eix;
            scan_until!(t, eix, b',');
            let red = get_data_int(Some(&t[bix as usize..eix as usize]));
            // Application.debugPrint(red);
            eix += 1;
            bix = eix;
            scan_until!(t, eix, b',');
            let green = get_data_int(Some(&t[bix as usize..eix as usize]));
            eix += 1;
            bix = eix;
            scan_until!(t, eix, b']');
            let blue = get_data_int(Some(&t[bix as usize..eix as usize]));
            // Java 构造器对 0-255 外抛 IllegalArgumentException — 域内 color[] 恒 0-255, 不可达
            colorg = [red as u8, green as u8, blue as u8, 255];
            // Application.debugPrint(colorg);

            // 继续向下搜索blink
            eix += 2;
            // 找2个引号
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b':');
            eix += 1;
            bix = eix;
            scan_until!(t, eix, b',');
            blink = get_data_int(Some(&t[bix as usize..eix as usize]));
            // Application.debugPrint(blink);

            // 继续向下搜索icon
            eix += 1;
            // 找三个引号
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            bix = eix;
            scan_until!(t, eix, b'"');
            icon = t[bix as usize..eix as usize].to_string();
            if icon == "Player" {
                is_player = true;
            }

            // Application.debugPrint(icon);

            // 继续向下搜索icon_bg
            eix += 2;
            // 找三个引号
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            bix = eix;
            scan_until!(t, eix, b'"');
            icon_bg = t[bix as usize..eix as usize].to_string();
            if icon_bg != "none" {
                is_selected = true;
            }
            // Application.debugPrint(iconBg);
            // 继续向下搜索x
            eix += 2;
            // 找2个引号
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b':');
            eix += 1;
            bix = eix;
            scan_until!(t, eix, b',');
            x = get_data_float(Some(&t[bix as usize..eix as usize]));
            // Application.debugPrint(x);
            // 继续向下搜索y
            eix += 1;
            // 找2个引号
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b'"');
            eix += 1;
            scan_until!(t, eix, b':');
            eix += 1;
            bix = eix;
            scan_until2!(t, eix, b',', b'}');
            y = get_data_float(Some(&t[bix as usize..eix as usize]));

            if t.as_bytes()[eix as usize] == b'}' {
                flag = 0;
            } else {
                flag = 1;
            }

            // Application.debugPrint(t.substring(bix,eix));

            // 再根据type判断是否取dx、dy
            // Application.debugPrint(flag);

            if flag == 0 {
                // 进入staobj写值
                if is_selected {
                    self.slc.r#type = Some(r#type.clone());
                    self.slc.color = Some(color.clone());
                    self.slc.colorg = Some(colorg);
                    self.slc.blink = blink;
                    self.slc.icon = Some(icon.clone());
                    self.slc.icon_bg = Some(icon_bg.clone());
                    self.slc.x = x;
                    self.slc.y = y;
                    self.slc.dx = 0.0;
                    self.slc.dy = 0.0;
                } else {
                    let cur = self.stacur as usize;
                    self.sta[cur].r#type = Some(r#type.clone());
                    self.sta[cur].color = Some(color.clone());
                    self.sta[cur].colorg = Some(colorg);
                    self.sta[cur].blink = blink;
                    self.sta[cur].icon = Some(icon.clone());
                    self.sta[cur].icon_bg = Some(icon_bg.clone());
                    self.sta[cur].x = x;
                    self.sta[cur].y = y;
                    // Application.debugPrint("s写值成功" + sta[stacur].toString());
                    self.stacur += 1;
                }
            }
            if flag == 1 {
                // 进入movobj判断
                // 继续向下搜索y
                // Application.debugPrint(t);
                eix += 1;
                // 找2个引号
                // Application.debugPrint(t);
                // Application.debugPrint("sad");
                scan_until!(t, eix, b'"');
                eix += 1;
                scan_until!(t, eix, b'"');
                eix += 1;
                scan_until!(t, eix, b':');
                eix += 1;
                bix = eix;
                scan_until2!(t, eix, b',', b'}');
                // Application.debugPrint(t.substring(bix,eix));
                dx = get_data_float(Some(&t[bix as usize..eix as usize]));

                // 继续向下搜索y
                eix += 1;
                // 找2个引号
                scan_until!(t, eix, b'"');
                eix += 1;
                scan_until!(t, eix, b'"');
                eix += 1;
                scan_until!(t, eix, b':');
                eix += 1;
                bix = eix;
                scan_until2!(t, eix, b',', b'}');
                // Application.debugPrint(t.substring(bix,eix));
                dy = get_data_float(Some(&t[bix as usize..eix as usize]));
                if !is_player {
                    if is_selected {
                        self.slc.r#type = Some(r#type.clone());

                        self.slc.color = Some(color.clone());
                        self.slc.colorg = Some(colorg);
                        self.slc.blink = blink;
                        self.slc.icon = Some(icon.clone());
                        self.slc.icon_bg = Some(icon_bg.clone());
                        self.slc.x = x;
                        self.slc.y = y;
                        self.slc.dx = dx;
                        self.slc.dy = dy;
                    } else {
                        let cur = self.movcur as usize;
                        self.mov[cur].r#type = Some(r#type.clone());

                        self.mov[cur].color = Some(color.clone());
                        self.mov[cur].colorg = Some(colorg);
                        self.mov[cur].blink = blink;
                        self.mov[cur].icon = Some(icon.clone());
                        self.mov[cur].icon_bg = Some(icon_bg.clone());
                        self.mov[cur].x = x;
                        self.mov[cur].y = y;
                        // PORT: Java 的 mov 写值不含 dx/dy (仅 pla/slc 写), 保真保留
                        // Application.debugPrint("m写值成功" + mov[movcur].toString());
                        self.movcur += 1;
                    }
                } else {
                    self.pla.r#type = Some(r#type.clone());
                    self.pla.color = Some(color.clone());
                    self.pla.colorg = Some(colorg);
                    self.pla.blink = blink;
                    self.pla.icon = Some(icon.clone());
                    self.pla.icon_bg = Some(icon_bg.clone());
                    self.pla.x = x;
                    self.pla.y = y;
                    self.pla.dx = dx;
                    self.pla.dy = dy;
                    // Application.debugPrint("玩家写值成功" + pla.toString());
                }
            }
        }
    }

    fn process_obj(&mut self) {
        let mut sobj = self.get_line();
        // PORT: Java `sobj != ""` 是引用比较 — getLine 的空返回是驻留字面量 "",
        // 引用相等成立才退出循环; Rust 值比较对"唯一空来源"等价 (§2.6)
        while !sobj.is_empty() {
            self.parse_obj(&sobj);
            sobj = self.get_line();
        }
        // Application.debugPrint(mov[movcur-1].x);
        //Application.debugPrint("切片完成");
    }

    fn init_mobj(&mut self) {
        for i in 0..self.num {
            self.mov[i] = Movobj::default();
            self.sta[i] = Staobj::default();
        }
    }

    pub fn init(&mut self) {
        self.num = 500;
        //Application.debugPrint("mapObj初始化了");
        // PORT: Java `new Movobj[num]` 是 null 槽位数组, 随后 initMobj 逐个 new —
        // 此处直接生成默认实例 (语义等同 init 后状态), 不引入 Clone 语义
        self.mov = (0..self.num).map(|_| Movobj::default()).collect();
        self.sta = (0..self.num).map(|_| Staobj::default()).collect();
        self.init_mobj();
        self.pla = Plaobj::default();
        self.slc = Slcobj::default();
        self.s = String::new();
    }

    pub fn calculate(&mut self) {
        self.aot = ((self.slc.dy / self.slc.dx).atan() - (self.pla.dy / self.pla.dx).atan()).abs();
    }

    pub fn update(&mut self, s: &str) {
        self.s = s.to_string();
        // Application.debugPrint("初始值"+s);
        self.movcur = 0;
        self.stacur = 0;
        self.slc.r#type = Some(String::new());
        self.process_obj();
        self.calculate();
    }
}

impl Default for MapObj {
    fn default() -> Self {
        Self::new()
    }
}

// ---- 静态正则方法 (Service.java:1837 在用的 Player 定位路径) ----
// PORT: PORTING.md 库映射 java.util.regex → regex crate, 但 vm-core 依赖清单不含
// regex (本批无权改 Cargo.toml), 按原正则结构手写等价回溯匹配器; 三处模式同构
// (\{[^{}]*K1\s*:\s*V1[^{}]*,[^{}]*K2\s*:\s*(NUM),[^{}]*K3\s*:\s*(NUM)[^{}]*\}),
// 参数化复用。贪婪量词的"最长优先+回溯"尝试次序与 java.util.regex 一致 (双 x 键
// oracle 用例验证取后位); 后续批次若引入 regex crate 可原样替换。

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
                                                            if let Some((r2, g3)) = try_number(t, r) {
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
fn find_pairs<'a>(text: &'a str, key1: &str, val1: &str, key2: &str, key3: &str) -> Vec<(&'a str, &'a str)> {
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

    /// Java `public static void getAirfieldLoc(String jsonText, double[][] loc)`
    pub fn get_airfield_loc(json_text: &str, _loc: &mut [f64; 2]) {
        // 正则表达式用于匹配整个JSON对象，并捕获icon为"Player"的x和y坐标
        // PORT: Java 原实现的 loc 写入已注释 (解析但丢弃), 参数仅为兼容签名;
        // 原签名 double[][] loc 此处压平为 &mut [f64;2] — 若日后解注释恢复
        // loc[i][0..1] 写入, 需改回 &mut [&mut [f64;2]] 嵌套形状
        for (_g1, _g3) in find_pairs(json_text, "\"type\"", "\"airfield\"", "\"sx\"", "\"sy\"") {
            // System.out.println("Airfield coordinates: x = " + x + ", y = " + y);
            // loc[0] = x;
            // loc[1] = y;
        }
    }
}

#[cfg(test)]
mod tests;
