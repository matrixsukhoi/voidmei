//! MapInfo 的 Rust 移植 (src/parser/MapInfo.java)
//! /map_info JSON 解析 (地图范围/缩放/阶段) — 手写子串扫描, 非完整 JSON 解析。
//!
//! PORT: §2.1 — Java charAt/substring 按 UTF-16 码元; 本域 (map_info 键与数值) 纯
//! ASCII, 字节索引 + 整字符步进与 Java 逐码元推进等价 (mod.rs 公共 helper)。
//! Java `class zb` (包私有) → 同模块 pub struct Zb。

use super::char_len_at;

/// Java `class zb` — 坐标对
#[derive(Debug, Clone, Copy)]
pub struct Zb {
    pub x: f64,
    pub y: f64,
}

impl Default for Zb {
    fn default() -> Self {
        Zb { x: 0.0, y: 0.0 } // Java: new zb() 数值字段默认 0
    }
}

pub struct MapInfo {
    s: String,
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
    tp: Zb,
    pub map_stage: f64,
}

impl MapInfo {
    /// 对应 Java `new MapInfo()`: 标量字段取 Java 默认值 (§2.10)
    pub fn new() -> Self {
        MapInfo {
            s: String::new(),
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
            tp: Zb::default(),
            map_stage: 0.0,
        }
    }

    /// Java `double StringtoFloat(String a)`:
    /// Float.parseFloat 单精度解析后拓宽 double, 且 parseFloat 隐含 trim 首尾空白 —
    /// 多行/缩进格式 payload 的子串带前导空白时靠 trim 对齐 Java (string_helper 先例)
    fn string_to_float(a: &str) -> f64 {
        if !a.is_empty() {
            a.trim().parse::<f32>().unwrap() as f64
        } else {
            0.0
        }
    }

    pub fn get_map_info_parser_array(&self, t: &str) -> Zb {
        let s = &self.s;
        let mut bix: i32;
        let mut eix: i32;
        let mut a = Zb::default(); // Java: new zb()
        bix = s.find(t).map_or(-1, |v| v as i32);

        if bix >= 0 {
            eix = bix;
            while s.as_bytes()[eix as usize] != b':' {
                eix += char_len_at(s, eix as usize) as i32;
            }
            eix += 1;
            // PORT: bix = eix + 3 — Java 原样的 +3 偏移, 系统性跳过数值首字符
            // (负号或首位数字, 见模块测试的 oracle 值), 属上游既有行为, 保真保留
            bix = eix + 3;
            while s.as_bytes()[eix as usize] != b',' {
                eix += char_len_at(s, eix as usize) as i32;
                if eix == s.len() as i32 + 1 {
                    break;
                }
            }

            a.x = MapInfo::string_to_float(&s[bix as usize..eix as usize]);

            bix = eix + 2;
            while s.as_bytes()[eix as usize] != b']' {
                eix += char_len_at(s, eix as usize) as i32;
                if eix == s.len() as i32 + 1 {
                    break;
                }
            }
            eix -= 1;

            a.y = MapInfo::string_to_float(&s[bix as usize..eix as usize]);
        }
        a
    }

    pub fn init(&mut self) {}

    pub fn update(&mut self, s: &str) {
        self.s = s.to_string();
        // System.out.print(s);
        self.tp = self.get_map_info_parser_array("grid_steps");
        self.grid_steps_x = self.tp.x;
        self.grid_steps_y = self.tp.y;
        self.tp = self.get_map_info_parser_array("grid_zero");
        self.grid_zero_x = self.tp.x;
        self.grid_zero_y = self.tp.y;
        self.tp = self.get_map_info_parser_array("map_max");
        self.map_max_x = self.tp.x;
        self.map_max_y = self.tp.y;
        self.tp = self.get_map_info_parser_array("map_min");
        self.map_min_x = self.tp.x;
        self.map_min_y = self.tp.y;
        self.cmapmaxsize_x = self.map_max_x - self.map_min_x;
        self.cmapmaxsize_y = self.map_max_y - self.map_min_y;
        self.in_game_offset =
            ((self.grid_zero_y - self.grid_zero_x) - (self.map_max_x + self.map_max_y)) / (self.grid_steps_x + self.grid_steps_y);
        self.map_stage = (self.map_max_x + self.map_max_y) * 2.0 / (self.grid_steps_x + self.grid_steps_y);

        // Application.debugPrint("ingame mapinfo offset:" + inGameOffset + "map stage: " + mapStage);
    }
}

impl Default for MapInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真机抓取的 /map_info 快照, mock 8111 线上格式 (冒号后一空格, 逗号后无空格)。
    /// 断言值 = Java 8 oracle 实测 — 注意 x 分量因 Java 的 +3 偏移系统性丢首字符
    /// (6400.0→400.0, -32768.0→32768.0), 逗号后无空格格式下 y 同样丢首字符。
    const MAP_INFO_MOCK: &str = "{\"grid_size\": [57584.11328125,64194.1953125],\"grid_steps\": [6400.0,6400.0],\"grid_zero\": [-24816.11328125,31426.1953125],\"hud_type\": 0,\"map_generation\": 9,\"map_max\": [32768.0,32768.0],\"map_min\": [-32768.0,-32768.0],\"valid\": true}";

    /// python 默认 json.dumps 格式 (逗号后带空格) — x 仍丢首字符, y 完整 (oracle 实测)
    const MAP_INFO_DEFAULT: &str = "{\"grid_size\": [57584.11328125, 64194.1953125], \"grid_steps\": [6400.0, 6400.0], \"grid_zero\": [-24816.11328125, 31426.1953125], \"hud_type\": 0, \"map_generation\": 9, \"map_max\": [32768.0, 32768.0], \"map_min\": [-32768.0, -32768.0], \"valid\": true}";

    #[test]
    fn update_mock_format_matches_java_oracle() {
        let mut mi = MapInfo::new();
        mi.init();
        mi.update(MAP_INFO_MOCK);
        assert_eq!(mi.grid_steps_x, 400.0); // 6400.0 丢首位 '6'
        assert_eq!(mi.grid_steps_y, 400.0); // 逗号无空格: y 也丢首位 '6'
        assert_eq!(mi.grid_zero_x, 24816.11328125); // 丢负号
        assert_eq!(mi.grid_zero_y, 1426.1953125); // 丢首位 '3'
        assert_eq!(mi.map_max_x, 2768.0);
        assert_eq!(mi.map_max_y, 2768.0);
        assert_eq!(mi.map_min_x, 32768.0); // 丢负号
        assert_eq!(mi.map_min_y, 32768.0); // 丢负号
        assert_eq!(mi.cmapmaxsize_x, -30000.0);
        assert_eq!(mi.cmapmaxsize_y, -30000.0);
        assert_eq!(mi.in_game_offset, -36.1573974609375);
        assert_eq!(mi.map_stage, 13.84);
    }

    #[test]
    fn update_default_format_matches_java_oracle() {
        let mut mi = MapInfo::new();
        mi.update(MAP_INFO_DEFAULT);
        assert_eq!(mi.grid_steps_x, 400.0);
        assert_eq!(mi.grid_steps_y, 6400.0); // 逗号后空格: y 完整
        assert_eq!(mi.grid_zero_x, 24816.11328125);
        assert_eq!(mi.grid_zero_y, 31426.1953125);
        assert_eq!(mi.map_max_x, 2768.0);
        assert_eq!(mi.map_max_y, 32768.0);
        assert_eq!(mi.map_min_x, 32768.0);
        assert_eq!(mi.map_min_y, -32768.0);
        assert_eq!(mi.cmapmaxsize_x, -30000.0);
        assert_eq!(mi.cmapmaxsize_y, 65536.0);
        assert_eq!(mi.in_game_offset, -4.253811465992647);
        assert_eq!(mi.map_stage, 10.451764705882352);
    }

    #[test]
    fn get_map_info_parser_array_missing_key_returns_zeros() {
        let mut mi = MapInfo::new();
        mi.update("{\"foo\": [1.0, 2.0]}");
        let zb = mi.get_map_info_parser_array("grid_steps");
        assert_eq!((zb.x, zb.y), (0.0, 0.0)); // Java: new zb() 未写值
    }

    #[test]
    fn string_to_float_empty_and_trim() {
        // Java: 空串 → 0 (length()==0 分支); parseFloat 隐含 trim
        assert_eq!(MapInfo::string_to_float(""), 0.0);
        assert_eq!(MapInfo::string_to_float(" 6400.0\n"), 6400.0);
        assert_eq!(MapInfo::string_to_float("31426.1953125"), 31426.1953125);
    }
}
