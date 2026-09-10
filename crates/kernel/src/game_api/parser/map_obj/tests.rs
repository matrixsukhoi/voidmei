use super::*;

/// 真机抓取 map_obj 快照中的 Player 对象 (mock 线上格式)
const PLAYER_MOCK: &str = "[{\"type\": \"airfield\", \"color\": \"#174DFF\", \"color[]\": [23, 77, 255], \"blink\": 0, \"icon\": \"none\", \"icon_bg\": \"none\", \"sx\": 0.359126, \"sy\": 0.560636, \"ex\": 0.359155, \"ey\": 0.511808}, {\"type\": \"aircraft\", \"color\": \"#faC81E\", \"color[]\": [250, 200, 30], \"blink\": 0, \"icon\": \"Player\", \"icon_bg\": \"none\", \"x\": 0.350927, \"y\": 0.358864, \"dx\": 0.274005, \"dy\": 0.961728}]";

// ---- Player 定位提取 (Service 在用) ----
// (波21 serde 化: 原 java.util.regex 回溯匹配器用例改写为 serde 语义)

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
    // 数组顺序遍历逐个覆盖 → 最后一个 Player 对象胜出
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
fn get_player_loc_duplicate_key_last_wins() {
    // 重复键 preserve_order 语义后者胜 — 与原正则贪婪取最后一个 "x" 一致
    let mut loc = [0.0; 2];
    MapObj::get_player_loc("[{\"icon\":\"Player\",\"x\":1,\"x\":9,\"y\":8}]", &mut loc);
    assert_eq!(loc, [9.0, 8.0]);
}

#[test]
fn get_player_loc_no_match_leaves_untouched() {
    // 非 Player / 缺 y 键 / 值不精确等于 "Player" / 对象缺 x-y — 均不写 loc
    let mut loc = [11.0, 22.0];
    MapObj::get_player_loc("[{\"icon\":\"Bot\",\"x\":7,\"y\":8}]", &mut loc);
    MapObj::get_player_loc("[{\"icon\":\"Player\",\"x\":7}]", &mut loc);
    MapObj::get_player_loc("[{\"icon\":\"xPlayer\",\"x\":1,\"y\":2}]", &mut loc);
    MapObj::get_player_loc("[{\"icon\":\"Player\"},{\"x\":1,\"y\":2}]", &mut loc);
    assert_eq!(loc, [11.0, 22.0]);
}

#[test]
fn get_player_loc_whitespace_tolerant() {
    // 波21 语义修好备案: 原正则要求数字后紧跟逗号 (whitespace 不容忍是
    // 正则缺陷), serde 宽松解析 — 带空格间隔的合法 JSON 现在能取到
    let mut loc = [0.0; 2];
    MapObj::get_player_loc(
        "[{  \"icon\"  :  \"Player\" , \"x\" :  1.5 , \"y\" :  -2.25 }]",
        &mut loc,
    );
    assert_eq!(loc, [1.5, -2.25]);
}

#[test]
fn get_player_loc_malformed_json_leaves_untouched() {
    // 非法 JSON ("7." 小数点后无数字等) / 空串 → 不动 loc (对齐原语义)
    let mut loc = [11.0, 22.0];
    MapObj::get_player_loc("[{\"icon\":\"Player\",\"x\":7.,\"y\":8}]", &mut loc);
    MapObj::get_player_loc("", &mut loc);
    MapObj::get_player_loc("截断的响应", &mut loc);
    assert_eq!(loc, [11.0, 22.0]);
}

#[test]
fn get_player_loc_cjk_payload() {
    // CJK 键值混排: serde 按字符语义解析 (原手写匹配器按字符边界回溯)
    let mut loc = [0.0; 2];
    MapObj::get_player_loc(
        "[{\"名称\":\"玩家甲\",\"icon\":\"Player\",\"x\":1.5,\"y\":2.5}]",
        &mut loc,
    );
    assert_eq!(loc, [1.5, 2.5]);
    MapObj::get_player_loc(
        "[{\"icon\":\"步兵\",\"x\":9.9,\"y\":9.9},{\"备注\":\"测试\",\"icon\":\"Player\",\"x\":-1.5,\"y\":3.25}]",
        &mut loc,
    );
    assert_eq!(loc, [-1.5, 3.25]);
}

#[test]
fn get_player_loc_cjk_adjacent_to_key_does_not_match() {
    // "icon" 后紧跟 CJK (键名不同) — 全等键匹配不命中
    let mut loc = [11.0, 22.0];
    MapObj::get_player_loc("[{\"图标icon玩家\":\"Player\",\"x\":1,\"y\":2}]", &mut loc);
    assert_eq!(loc, [11.0, 22.0]);
}

#[test]
fn get_player_dir_matches_only_player() {
    // 非 Player 对象的 dx/dy 不取; Player 的取
    let mut dir = [0.0; 2];
    MapObj::get_player_dir(
        "[{\"icon\":\"Bot\",\"dx\":0.5,\"dy\":0.6},{\"icon\":\"Player\",\"dx\":-0.25,\"dy\":0.75}]",
        &mut dir,
    );
    assert_eq!(dir, [-0.25, 0.75]);
}
