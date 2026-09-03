use super::*;

/// 真机抓取 map_obj 快照中的 Player 对象 (mock 线上格式)
const PLAYER_MOCK: &str = "[{\"type\": \"airfield\", \"color\": \"#174DFF\", \"color[]\": [23, 77, 255], \"blink\": 0, \"icon\": \"none\", \"icon_bg\": \"none\", \"sx\": 0.359126, \"sy\": 0.560636, \"ex\": 0.359155, \"ey\": 0.511808}, {\"type\": \"aircraft\", \"color\": \"#faC81E\", \"color[]\": [250, 200, 30], \"blink\": 0, \"icon\": \"Player\", \"icon_bg\": \"none\", \"x\": 0.350927, \"y\": 0.358864, \"dx\": 0.274005, \"dy\": 0.961728}]";

// ---- Player 定位正则路径 (Service 在用; 断言值 = Java 8 oracle 实测) ----
// (波20 清场: parseObj 实例路径用例已随 OtherService 退役)

#[test]
fn get_player_loc_and_dir_on_snapshot() {
    let mut loc = [0.0; 2];
    MapObj::get_player_loc(PLAYER_MOCK, &mut loc);
    assert_eq!(loc, [0.350927, 0.358864]);
    let mut dir = [0.0; 2];
    MapObj::get_player_dir(PLAYER_MOCK, &mut dir);
    assert_eq!(dir, [0.274005, 0.961728]);
}

#[test]
fn get_player_loc_last_match_wins() {
    // while(find()) 逐个覆盖 → 最后一个 Player 对象胜出 (oracle 实测)
    let mut loc = [0.0; 2];
    MapObj::get_player_loc(
        "[{\"icon\":\"Player\",\"x\":1.5,\"y\":2.5},{\"icon\":\"Player\",\"x\":3.75,\"y\":-4.25}]",
        &mut loc,
    );
    assert_eq!(loc, [3.75, -4.25]);
}

#[test]
fn get_player_loc_integer_and_negative_capture() {
    let mut loc = [0.0; 2];
    MapObj::get_player_loc("[{\"icon\":\"Player\",\"x\":7,\"y\":8}]", &mut loc);
    assert_eq!(loc, [7.0, 8.0]);
    let mut loc2 = [-1.0, -1.0];
    MapObj::get_player_loc("[{\"icon\":\"Player\",\"x\":-1.25,\"y\":-2}]", &mut loc2);
    assert_eq!(loc2, [-1.25, -2.0]);
}

#[test]
fn get_player_loc_greedy_takes_last_duplicate_key() {
    // [^{}]*"x" 贪婪回溯 → 取最后一个 "x" 键 (oracle 实测 x=9)
    let mut loc = [0.0; 2];
    MapObj::get_player_loc("[{\"icon\":\"Player\",\"x\":1,\"x\":9,\"y\":8}]", &mut loc);
    assert_eq!(loc, [9.0, 8.0]);
}

#[test]
fn get_player_loc_no_match_leaves_untouched() {
    // 无 Player / 缺 y 键 / 值不精确等于 "Player" / 跨花括号 — 均不写 loc (oracle 实测)
    let mut loc = [11.0, 22.0];
    MapObj::get_player_loc("[{\"icon\":\"Bot\",\"x\":7,\"y\":8}]", &mut loc);
    MapObj::get_player_loc("[{\"icon\":\"Player\",\"x\":7}]", &mut loc);
    MapObj::get_player_loc("[{\"icon\":\"xPlayer\",\"x\":1,\"y\":2}]", &mut loc);
    MapObj::get_player_loc("[{\"icon\":\"Player\"},{\"x\":1,\"y\":2}]", &mut loc);
    // 数字后必须紧跟逗号 (原正则无 \s 容忍, oracle 实测)
    MapObj::get_player_loc(
        "[{  \"icon\"  :  \"Player\" , \"x\" :  1.5 , \"y\" :  -2.25 }]",
        &mut loc,
    );
    // "7." 的小数点后无数字 → 该对象不匹配
    MapObj::get_player_loc("[{\"icon\":\"Player\",\"x\":7.,\"y\":8}]", &mut loc);
    assert_eq!(loc, [11.0, 22.0]);
}

#[test]
fn get_player_loc_cjk_payload_backtracks_on_char_boundaries() {
    // 非 ASCII 域: k1 回溯跨 CJK 键值的多字节区间 — java.util.regex 按码元
    // 正常回溯命中; Rust 按字符边界递减等价 (修复前字节递减在字符中间切片 panic)
    let mut loc = [0.0; 2];
    MapObj::get_player_loc(
        "[{\"名称\":\"玩家甲\",\"icon\":\"Player\",\"x\":1.5,\"y\":2.5}]",
        &mut loc,
    );
    assert_eq!(loc, [1.5, 2.5]);
    // 多对象 + CJK 值混排: 跨过非 Player 对象取后者
    MapObj::get_player_loc(
        "[{\"icon\":\"步兵\",\"x\":9.9,\"y\":9.9},{\"备注\":\"测试\",\"icon\":\"Player\",\"x\":-1.5,\"y\":3.25}]",
        &mut loc,
    );
    assert_eq!(loc, [-1.5, 3.25]);
}

#[test]
fn get_player_loc_cjk_adjacent_to_key_does_not_match() {
    // "icon" 后紧跟 CJK (无 ASCII 尾引号) — 字面量不命中, java.util.regex 同此
    let mut loc = [11.0, 22.0];
    MapObj::get_player_loc("[{\"图标icon玩家\":\"Player\",\"x\":1,\"y\":2}]", &mut loc);
    assert_eq!(loc, [11.0, 22.0]);
}

#[test]
fn get_player_dir_matches_only_player() {
    // 非 Player 对象的 dx/dy 不取; Player 的取 (oracle 实测)
    let mut dir = [0.0; 2];
    MapObj::get_player_dir(
        "[{\"icon\":\"Bot\",\"dx\":0.5,\"dy\":0.6},{\"icon\":\"Player\",\"dx\":-0.25,\"dy\":0.75}]",
        &mut dir,
    );
    assert_eq!(dir, [-0.25, 0.75]);
}
