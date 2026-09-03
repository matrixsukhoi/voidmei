//! 8111 遥测 / 地图 JSON 解析器 (serde_json::Value 取数层)。
//!
//! 波20 serde 化: 原手写子串扫描 (find 键名 → 扫 ':' → 取到 ','/'}') 退役 —
//! 8111 返回的是标准 JSON, 手写扫描的子串碰撞 (needle "TAS" 命中键 "TAS, km/h"
//! 纯靠巧合) / 值含逗号截断 / find-rfind 不一致等怪癖一并修好。键名映射
//! 对照真机快照 (script/mock_scenarios/snapshots/*.json) 逐字段核对。
//!
//! PORT 历史: 本模块原为 src/parser (Java A 类) 的一比一翻译;
//! (波20 清场: hud_msg 仅被未接线的 map_service 消费, 已退役)

pub mod indicators;
pub mod map_info;
pub mod map_obj;
pub mod state;

pub use indicators::Indicators;
pub use map_info::{MapInfo, Zb};
pub use map_obj::MapObj;
pub use state::State;

/// 对应 Java `StringHelper.iInvalid` — 缺数键的整型哨兵。
/// 下游 (formula registry / hud_calculator / voice_warning) 以此值判缺数据, 契约保留。
pub const I_INVALID: i32 = -65535;

/// 对应 Java `StringHelper.fInvalid` — 缺数键的浮点哨兵 (同上契约保留)。
pub const F_INVALID: f64 = -65535.0;

/// 全等键取 f64; 缺键/非数值 → F_INVALID 哨兵 (对齐手写时代 "缺键→哨兵" 产出契约;
/// 数值直接 f64 解析, 原 Float.parseFloat 的 f32 单精度拓宽位级复刻退役)。
pub(crate) fn v_f64(v: &serde_json::Value, key: &str) -> f64 {
    v.get(key)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(F_INVALID)
}

/// 全等键取 i32; 缺键/非整数(含溢出/小数) → I_INVALID 哨兵。
pub(crate) fn v_i32(v: &serde_json::Value, key: &str) -> i32 {
    v.get(key)
        .and_then(serde_json::Value::as_i64)
        .and_then(|x| i32::try_from(x).ok())
        .unwrap_or(I_INVALID)
}

/// 取 `[x, y]` 数值对 (map_info 的坐标数组); 缺键/越界/非数值 → 0.0
/// (对齐手写时代 find 不到键 → Zb::default 的 0.0 产出)。
pub(crate) fn v_xy(v: &serde_json::Value, key: &str) -> (f64, f64) {
    let a = v.get(key).and_then(serde_json::Value::as_array);
    let x = a
        .and_then(|a| a.first())
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = a
        .and_then(|a| a.get(1))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    (x, y)
}

/// r[i..] 首字符的 UTF-8 字节数; i 越界返回 0。
/// PORT: Java 循环 `eix++` 逐 UTF-16 码元推进, 此处按整字符推进 —
/// BMP 内等价。map_obj 正则路径与 ui_support/comparison 共用。
pub(crate) fn char_len_at(r: &str, i: usize) -> usize {
    r[i..].chars().next().map_or(0, char::len_utf8)
}
