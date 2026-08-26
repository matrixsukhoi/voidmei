//! src/parser 数据类 (A 类, CLASSIFY 第二批第 6 步): 8111 遥测 / 地图 / HUD 消息的
//! 手写子串提取解析器。Blkx / FlightAnalyzer / FlightLog 属 B 类后续批次, 不在本模块。
//! PORT: Java 包内默认可见性 (zb/getLine/parseObj 等) → Rust 模块内私有; 跨文件消费的
//! 公共 API 保持 pub。vm-data 里 serde_json 版 parse_state 是 POC 存量 (并存遗留),
//! 本模块按 Java 子串提取语义保真翻译, 二者互不影响。

pub mod hud_msg;
pub mod indicators;
pub mod map_info;
pub mod map_obj;
pub mod state;

pub use hud_msg::{Damage, Events, HudMsg};
pub use indicators::Indicators;
pub use map_info::{MapInfo, Zb};
pub use map_obj::{MapObj, Movobj, Plaobj, Slcobj, Staobj};
pub use state::State;

/// Java `charAt` 索引按 UTF-16 码元; 本包各手写扫描器操作游戏 8111 JSON (键/数值域),
/// 逐字节比较 ASCII 定界符不会误判多字节字符 (UTF-8 自同步), 循环推进按整字符步进
/// 与 Java 逐码元推进在 BMP 域等价 (string_helper 同款先例)。各文件的域差异见其
/// 模块级 PORT 注释 (hud_msg 因 CJK 消息走 char 向量)。
pub(crate) fn char_len_at(r: &str, i: usize) -> usize {
    r[i..].chars().next().map_or(0, char::len_utf8)
}
