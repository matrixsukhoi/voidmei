use super::*;

/// 紧凑格式 (冒号后无空格) — parseObj 的设计目标格式。
/// 五对象序列覆盖 sta/slc(flag0)/pla/mov/slc(flag1) 全部写值路径,
/// 断言值 = Java 8 oracle 实测。
const COMPACT_FULL: &str = "[{\"type\":\"stasel\",\"color\":\"#FF00FF\",\"color[]\":[255,0,255],\"blink\":3,\"icon\":\"View\",\"icon_bg\":\"ViewPlayer\",\"x\":0.31,\"y\":0.32},{\"type\":\"aircraft\",\"color\":\"#faC81E\",\"color[]\":[250,200,30],\"blink\":0,\"icon\":\"Player\",\"icon_bg\":\"none\",\"x\":0.350927,\"y\":0.358864,\"dx\":0.274005,\"dy\":0.961728},{\"type\":\"movsel\",\"color\":\"#00AA00\",\"color[]\":[0,170,0],\"blink\":2,\"icon\":\"Squad\",\"icon_bg\":\"selbg\",\"x\":0.61,\"y\":0.62,\"dx\":0.7,\"dy\":-0.3},{\"type\":\"aircraft\",\"color\":\"#f00C00\",\"color[]\":[240,12,0],\"blink\":1,\"icon\":\"EnemyFighter\",\"icon_bg\":\"none\",\"x\":0.421,\"y\":0.512,\"dx\":-0.5,\"dy\":0.25},{\"type\":\"ground\",\"color\":\"#174DFF\",\"color[]\":[23,77,255],\"blink\":0,\"icon\":\"bot\",\"icon_bg\":\"none\",\"x\":0.11,\"y\":0.22}]";

/// mock 8111 线上格式 (冒号后一空格) — Java 8 oracle 实测在 color[] 的
/// parseInt 抛 NumberFormatException ("]": [250"), 保真 panic
const MOCK_FORMAT_OBJ: &str = "[{\"type\": \"aircraft\",\"color\": \"#faC81E\",\"color[]\": [250, 200, 30],\"blink\": 0,\"icon\": \"Player\",\"icon_bg\": \"none\",\"x\": 0.350927,\"y\": 0.358864,\"dx\": 0.274005,\"dy\": 0.961728}]";

/// 真机抓取 map_obj 快照中的 Player 对象 (mock 线上格式), 用于正则路径
const PLAYER_MOCK: &str = "[{\"type\": \"airfield\", \"color\": \"#174DFF\", \"color[]\": [23, 77, 255], \"blink\": 0, \"icon\": \"none\", \"icon_bg\": \"none\", \"sx\": 0.359126, \"sy\": 0.560636, \"ex\": 0.359155, \"ey\": 0.511808}, {\"type\": \"aircraft\", \"color\": \"#faC81E\", \"color[]\": [250, 200, 30], \"blink\": 0, \"icon\": \"Player\", \"icon_bg\": \"none\", \"x\": 0.350927, \"y\": 0.358864, \"dx\": 0.274005, \"dy\": 0.961728}]";

#[test]
fn update_compact_matches_java_oracle() {
    let mut mo = MapObj::new();
    mo.init();
    mo.update(COMPACT_FULL);
    assert_eq!(mo.movcur, 1);
    assert_eq!(mo.stacur, 1);
    // aot = |atan(slc.dy/slc.dx) - atan(pla.dy/pla.dx)| (oracle 实测;
    // atan 为 libm 函数, 跨实现不保证逐位一致, 容差断言)
    assert!((mo.aot - 1.6981331041655807).abs() < 1e-9);
    // pla: Player 对象, 带 dx/dy (f32 单精度拓宽)
    assert_eq!(mo.pla.r#type.as_deref(), Some("aircraft"));
    assert_eq!(mo.pla.color.as_deref(), Some("#faC81E"));
    assert_eq!(mo.pla.colorg, Some([250, 200, 30, 255]));
    assert_eq!(mo.pla.blink, 0);
    assert_eq!(mo.pla.icon.as_deref(), Some("Player"));
    assert_eq!(mo.pla.icon_bg.as_deref(), Some("none"));
    assert_eq!(mo.pla.x, 0.350927f32 as f64);
    assert_eq!(mo.pla.y, 0.358864f32 as f64);
    assert_eq!(mo.pla.dx, 0.274005f32 as f64);
    assert_eq!(mo.pla.dy, 0.961728f32 as f64);
    // slc: 最后一个 selected 写值者 (flag1 的 movsel), 覆盖 flag0 的 stasel
    assert_eq!(mo.slc.r#type.as_deref(), Some("movsel"));
    assert_eq!(mo.slc.color.as_deref(), Some("#00AA00"));
    assert_eq!(mo.slc.colorg, Some([0, 170, 0, 255]));
    assert_eq!(mo.slc.blink, 2);
    assert_eq!(mo.slc.icon.as_deref(), Some("Squad"));
    assert_eq!(mo.slc.icon_bg.as_deref(), Some("selbg"));
    assert_eq!(mo.slc.x, 0.61f32 as f64);
    assert_eq!(mo.slc.y, 0.62f32 as f64);
    assert_eq!(mo.slc.dx, 0.7f32 as f64);
    assert_eq!(mo.slc.dy, -0.3f32 as f64);
    // mov: 普通移动对象 — Java 写值不含 dx/dy, 保持默认 0 (oracle 实测)
    assert_eq!(mo.mov[0].r#type.as_deref(), Some("aircraft"));
    assert_eq!(mo.mov[0].color.as_deref(), Some("#f00C00"));
    assert_eq!(mo.mov[0].colorg, Some([240, 12, 0, 255]));
    assert_eq!(mo.mov[0].blink, 1);
    assert_eq!(mo.mov[0].icon.as_deref(), Some("EnemyFighter"));
    assert_eq!(mo.mov[0].icon_bg.as_deref(), Some("none"));
    assert_eq!(mo.mov[0].x, 0.421f32 as f64);
    assert_eq!(mo.mov[0].y, 0.512f32 as f64);
    assert_eq!(mo.mov[0].dx, 0.0);
    assert_eq!(mo.mov[0].dy, 0.0);
    assert_eq!(mo.mov[0].distance, 0.0); // distance 无写值点
                                         // sta: 静态对象 (y 后直接 '}')
    assert_eq!(mo.sta[0].r#type.as_deref(), Some("ground"));
    assert_eq!(mo.sta[0].colorg, Some([23, 77, 255, 255]));
    assert_eq!(mo.sta[0].x, 0.11f32 as f64);
    assert_eq!(mo.sta[0].y, 0.22f32 as f64);
}

#[test]
fn update_mock_format_panics_like_java() {
    // Java 8 oracle: NumberFormatException For input string: "]": [250" —
    // 位置偏移按紧凑格式设计, 空格间隔格式扫偏 (保真 panic, 上层轮询兜底)
    let mut mo = MapObj::new();
    mo.init();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        mo.update(MOCK_FORMAT_OBJ);
    }));
    assert!(result.is_err());
}

#[test]
fn update_slc_zero_direction_gives_nan_aot() {
    // slc 由 flag0 的 selected 写值 (dx=dy=0) → atan(0/0)=NaN 传播 (oracle 实测)
    let mut mo = MapObj::new();
    mo.init();
    mo.update("[{\"type\":\"stasel\",\"color\":\"#FF00FF\",\"color[]\":[255,0,255],\"blink\":3,\"icon\":\"View\",\"icon_bg\":\"ViewPlayer\",\"x\":0.31,\"y\":0.32},{\"type\":\"aircraft\",\"color\":\"#faC81E\",\"color[]\":[250,200,30],\"blink\":0,\"icon\":\"Player\",\"icon_bg\":\"none\",\"x\":0.35,\"y\":0.36,\"dx\":0.27,\"dy\":0.96}]");
    assert_eq!(mo.slc.dx, 0.0);
    assert_eq!(mo.slc.dy, 0.0);
    assert!(mo.aot.is_nan());
}

#[test]
fn update_resets_cursors_between_rounds() {
    let mut mo = MapObj::new();
    mo.init();
    mo.update(COMPACT_FULL);
    assert_eq!(mo.movcur, 1);
    // 第二轮: 光标归零, slc.type 清空; pla 保留旧值 (Java 字段不重置)
    mo.update("[{\"type\":\"aircraft\",\"color\":\"#f00C00\",\"color[]\":[240,12,0],\"blink\":1,\"icon\":\"EnemyFighter\",\"icon_bg\":\"none\",\"x\":0.421,\"y\":0.512,\"dx\":-0.5,\"dy\":0.25}]");
    assert_eq!(mo.movcur, 1);
    assert_eq!(mo.stacur, 0);
    assert_eq!(mo.slc.r#type, Some(String::new()));
    assert_eq!(mo.pla.icon.as_deref(), Some("Player")); // 保留上一轮
}

#[test]
fn init_allocates_500_slots() {
    let mut mo = MapObj::new();
    mo.init();
    assert_eq!(mo.mov.len(), 500);
    assert_eq!(mo.sta.len(), 500);
    assert!(mo.mov.iter().all(|m| m.r#type.is_none()));
    assert_eq!(mo.movcur, 0);
    assert_eq!(mo.stacur, 0);
}

// ---- Player 定位正则路径 (Service 在用; 断言值 = Java 8 oracle 实测) ----

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

#[test]
fn get_airfield_loc_parses_but_never_writes() {
    // Java 原实现的 loc 写入已注释 — 只解析不落值
    let mut loc = [5.0, 6.0];
    MapObj::get_airfield_loc(
        "[{\"type\":\"airfield\",\"sx\":0.359126,\"sy\":0.560636}]",
        &mut loc,
    );
    assert_eq!(loc, [5.0, 6.0]);
}
