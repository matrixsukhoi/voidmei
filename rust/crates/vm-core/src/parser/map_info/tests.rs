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
    assert_eq!((zb.x, zb.y), (0.0, 0.0));
}

#[test]
fn string_to_float_empty_and_trim() {
    assert_eq!(MapInfo::string_to_float(""), 0.0);
    assert_eq!(MapInfo::string_to_float(" 6400.0\n"), 6400.0);
    assert_eq!(MapInfo::string_to_float("31426.1953125"), 31426.1953125);
}
