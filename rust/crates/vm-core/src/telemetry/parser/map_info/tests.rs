use super::*;

/// 真机抓取的 /map_info 快照, mock 8111 线上格式 (冒号后一空格, 逗号后无空格)。
/// 波20 serde 化 + 修偏移: 原手写扫描 +3 偏移系统性丢数值首字符/负号
/// (6400.0→400.0、-32768.0→32768.0), oracle 曾锁定错值; 现断言正确值,
/// 且两种空白格式 (mock 紧凑 / python json.dumps) 解析结果一致。
const MAP_INFO_MOCK: &str = "{\"grid_size\": [57584.11328125,64194.1953125],\"grid_steps\": [6400.0,6400.0],\"grid_zero\": [-24816.11328125,31426.1953125],\"hud_type\": 0,\"map_generation\": 9,\"map_max\": [32768.0,32768.0],\"map_min\": [-32768.0,-32768.0],\"valid\": true}";

/// python 默认 json.dumps 格式 (逗号后带空格) — serde 下与 mock 格式等价
const MAP_INFO_DEFAULT: &str = "{\"grid_size\": [57584.11328125, 64194.1953125], \"grid_steps\": [6400.0, 6400.0], \"grid_zero\": [-24816.11328125, 31426.1953125], \"hud_type\": 0, \"map_generation\": 9, \"map_max\": [32768.0, 32768.0], \"map_min\": [-32768.0, -32768.0], \"valid\": true}";

#[test]
fn update_mock_format_correct_values() {
    let mut mi = MapInfo::new();
    mi.init();
    mi.update(MAP_INFO_MOCK);
    assert_eq!(mi.grid_steps_x, 6400.0); // 偏移 bug 修好: 不再丢首位 '6'
    assert_eq!(mi.grid_steps_y, 6400.0);
    assert_eq!(mi.grid_zero_x, -24816.11328125); // 负号不再丢
    assert_eq!(mi.grid_zero_y, 31426.1953125);
    assert_eq!(mi.map_max_x, 32768.0);
    assert_eq!(mi.map_max_y, 32768.0);
    assert_eq!(mi.map_min_x, -32768.0);
    assert_eq!(mi.map_min_y, -32768.0);
    assert_eq!(mi.cmapmaxsize_x, 65536.0);
    assert_eq!(mi.cmapmaxsize_y, 65536.0);
    assert_eq!(mi.in_game_offset, -0.7260696411132812);
    assert_eq!(mi.map_stage, 10.24);
}

#[test]
fn update_default_format_same_as_mock_format() {
    // 手写时代两种格式产出不同错值 (y 丢/不丢首字符); serde 下格式无感, 值恒一致
    let mut a = MapInfo::new();
    a.update(MAP_INFO_MOCK);
    let mut b = MapInfo::new();
    b.update(MAP_INFO_DEFAULT);
    assert_eq!(a.grid_steps_x, b.grid_steps_x);
    assert_eq!(a.grid_steps_y, b.grid_steps_y);
    assert_eq!(a.grid_zero_y, b.grid_zero_y);
    assert_eq!(a.map_min_y, b.map_min_y);
    assert_eq!(a.cmapmaxsize_y, b.cmapmaxsize_y);
    assert_eq!(a.in_game_offset, b.in_game_offset);
    assert_eq!(a.map_stage, b.map_stage);
}

#[test]
fn update_missing_keys_give_zeros() {
    // 缺键 (含畸形/空 JSON) → 0.0 (对齐手写时代 find 不到键的产出)
    let mut mi = MapInfo::new();
    mi.update("{\"foo\": [1.0, 2.0]}");
    assert_eq!(mi.grid_steps_x, 0.0);
    assert_eq!(mi.grid_zero_y, 0.0);
    assert_eq!(mi.map_max_x, 0.0);
    // grid_steps 全 0 → 派生除法产生 ±∞/NaN (手写时代同此, 下游按缺数据兜底)
    assert!(mi.map_stage.is_nan() || mi.map_stage.is_infinite());
}
