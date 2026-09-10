//! /map_obj.json 的 Player 定位/朝向提取 (Service 在用的唯一路径)。
//!
//! 波20 清场: Java MapObj 的实例解析路径 (parseObj 位置扫描 + mov/sta/slc/pla
//! 对象池, 仅被未接线的 OtherService 消费) 已退役。
//! 波21 serde 化: 原手写 java.util.regex 回溯匹配器 (~170 行, 迁移期
//! "无权改 Cargo.toml" 的产物) 退役 — /map_obj.json 是标准 JSON, 直接遍历
//! 数组项取 icon=="Player" 的 x/y。语义差异备案: 原正则对 `"x" : 1.5`
//! (数字后带空白) 不匹配是正则的缺陷性紧语义, serde 宽松解析属有意修好;
//! 重复键 ("x":1,"x":9) preserve_order 语义后者胜, 与原贪婪回溯取后者一致。

use serde_json::Value;

/// Player 定位/朝向提取器的命名空间 (原 Java 静态方法宿主类)。
pub struct MapObj;

impl MapObj {
    /// Java `getPlayerLoc(jsonText, loc)`: 遍历对象数组, icon=="Player" 的
    /// x/y 写入 loc; 多个 Player 后者胜 (对齐原 while(find()) 逐个覆盖);
    /// 无匹配/畸形 JSON 不动 loc。
    pub fn get_player_loc(json_text: &str, loc: &mut [f64; 2]) {
        for (x, y) in Self::player_pairs(json_text, "x", "y") {
            loc[0] = x;
            loc[1] = y;
        }
    }

    /// Java `getPlayerDir(jsonText, dir)`: 同上, 取 dx/dy。
    pub fn get_player_dir(json_text: &str, dir: &mut [f64; 2]) {
        for (dx, dy) in Self::player_pairs(json_text, "dx", "dy") {
            dir[0] = dx;
            dir[1] = dy;
        }
    }

    /// 公共提取: 数组内 icon=="Player" 的对象按序产出 (k1, k2) 数值对;
    /// 键缺失/非数值/非数组/畸形 JSON 的对象跳过。
    fn player_pairs(json_text: &str, k1: &str, k2: &str) -> Vec<(f64, f64)> {
        let v: Value = serde_json::from_str(json_text).unwrap_or(Value::Null);
        v.as_array().map_or(Vec::new(), |arr| {
            arr.iter()
                .filter(|obj| obj.get("icon").and_then(Value::as_str) == Some("Player"))
                .filter_map(|obj| {
                    let a = obj.get(k1).and_then(Value::as_f64)?;
                    let b = obj.get(k2).and_then(Value::as_f64)?;
                    Some((a, b))
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests;
