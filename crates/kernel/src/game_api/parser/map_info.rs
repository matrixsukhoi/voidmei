//! /map_info.json 地图元信息 (地图范围/网格/阶段)。
//!
//! 波20 serde 化 + 修偏移: 原手写扫描 `bix = eix + 3` 系统性跳过数值首字符
//! (6400.0 → 400.0、-32768.0 → 32768.0 丢负号), 基线 曾锁定这些错值;
//! 现直接取 JSON 数组元素, 下游 cmapmaxsize/in_game_offset/map_stage 等
//! 地图几何首次得到正确输入。
//!
//! 键名对照真机快照: grid_steps/grid_zero/map_max/map_min 均为 [x, y] 数组。

use serde_json::Value;

use super::v_xy;

/// Java `class zb` — 坐标对
#[derive(Debug, Clone, Copy)]
pub struct Zb {
    pub x: f64,
    pub y: f64,
}

impl Default for Zb {
    fn default() -> Self {
        Zb { x: 0.0, y: 0.0 }
    }
}

/// 波4: Clone 供 Frame 帧快照整体克隆 (字段全值类型)
#[derive(Clone)]
pub struct MapInfo {
    pub grid_steps_x: f64,
    pub grid_steps_y: f64,
    pub grid_zero_x: f64,
    pub grid_zero_y: f64,
    pub map_generation: i32,
    pub map_max_x: f64,
    pub map_max_y: f64,
    pub map_min_x: f64,
    pub map_min_y: f64,
    pub cmapmaxsize_x: f64,
    pub cmapmaxsize_y: f64,
    /// 游戏内地图的偏移量
    pub in_game_offset: f64,
    pub map_stage: f64,
}

impl MapInfo {
    /// 对应 Java `new MapInfo()`: 标量字段取 Java 默认值
    pub fn new() -> Self {
        MapInfo {
            grid_steps_x: 0.0,
            grid_steps_y: 0.0,
            grid_zero_x: 0.0,
            grid_zero_y: 0.0,
            map_generation: 0,
            map_max_x: 0.0,
            map_max_y: 0.0,
            map_min_x: 0.0,
            map_min_y: 0.0,
            cmapmaxsize_x: 0.0,
            cmapmaxsize_y: 0.0,
            in_game_offset: 0.0,
            map_stage: 0.0,
        }
    }

    pub fn update(&mut self, s: &str) {
        // 畸形/空 JSON → Null, 全部取数走缺键分支 (0.0, 对齐手写时代 find 不到)
        let v: Value = serde_json::from_str(s).unwrap_or(Value::Null);
        (self.grid_steps_x, self.grid_steps_y) = v_xy(&v, "grid_steps");
        (self.grid_zero_x, self.grid_zero_y) = v_xy(&v, "grid_zero");
        (self.map_max_x, self.map_max_y) = v_xy(&v, "map_max");
        (self.map_min_x, self.map_min_y) = v_xy(&v, "map_min");
        self.cmapmaxsize_x = self.map_max_x - self.map_min_x;
        self.cmapmaxsize_y = self.map_max_y - self.map_min_y;
        self.in_game_offset = ((self.grid_zero_y - self.grid_zero_x)
            - (self.map_max_x + self.map_max_y))
            / (self.grid_steps_x + self.grid_steps_y);
        self.map_stage =
            (self.map_max_x + self.map_max_y) * 2.0 / (self.grid_steps_x + self.grid_steps_y);

        // Application.debugPrint("ingame mapinfo offset:" + inGameOffset + "map stage: " + mapStage);
    }
}

impl Default for MapInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
