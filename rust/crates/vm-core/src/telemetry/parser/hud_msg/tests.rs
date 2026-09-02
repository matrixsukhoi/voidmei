use super::*;

/// mock 8111 线上格式; 断言值 = Java 8 oracle 实测
const HUDMSG_MOCK: &str = "{\"events\": [],\"damage\": [{\"id\": 532213658,\"msg\": \"player1_VS_player2\",\"sender\": \"someone\",\"enemy\": true,\"mode\": \"ES\"}]}";
const HUDMSG_MULTI: &str = "{\"events\": [],\"damage\": [{\"id\": 111,\"msg\": \"first\"}, {\"id\": 222,\"msg\": \"second 热msg\"}]}";

#[test]
fn update_parses_last_damage_object() {
    let mut hm = HudMsg::new();
    hm.init();
    assert_eq!(hm.update(HUDMSG_MOCK, 777), 532213658);
    let dmg = hm.dmg.as_ref().unwrap();
    assert_eq!(dmg.id, 532213658);
    assert_eq!(dmg.msg.as_deref(), Some("player1_VS_player2"));
    // sender/enemy/mode 从不赋值 → Java 默认 (oracle 实测 null/false)
    assert!(dmg.sender.is_none());
    assert!(!dmg.enemy);
    assert!(dmg.mode.is_none());
    assert!(dmg.updated);
}

#[test]
fn update_multi_damage_takes_last_line() {
    // getDmglastLine 取末尾对象 (去掉 "]}" 后回扫 '{'), CJK 消息按字符定位
    let mut hm = HudMsg::new();
    hm.init();
    assert_eq!(hm.update(HUDMSG_MULTI, 0), 222);
    let dmg = hm.dmg.as_ref().unwrap();
    assert_eq!(dmg.id, 222);
    assert_eq!(dmg.msg.as_deref(), Some("second 热msg"));
    assert!(dmg.updated);
}

#[test]
fn update_short_payload_returns_last_dmg() {
    // s.length() <= 30 → getDmglastLine 返回 "" → parseObj 返回 0 → 返回 lastDmg
    let mut hm = HudMsg::new();
    hm.init();
    assert_eq!(hm.update("{\"damage\": []}", 777), 777);
    assert!(!hm.dmg.as_ref().unwrap().updated);
    assert_eq!(hm.dmg.as_ref().unwrap().id, 0);
    assert!(hm.dmg.as_ref().unwrap().msg.is_none());
}

#[test]
fn update_empty_damage_array_returns_last_dmg() {
    let mut hm = HudMsg::new();
    hm.init();
    assert_eq!(hm.update("{\"events\": [],\"damage\": []}", 42), 42);
    assert!(!hm.dmg.as_ref().unwrap().updated);
}

#[test]
fn parse_obj_short_returns_zero() {
    let mut hm = HudMsg::new();
    hm.init();
    assert_eq!(hm.parse_obj("{\"id\": 123}"), 0); // length <= 20
}

#[test]
fn init_creates_default_damage() {
    let mut hm = HudMsg::new();
    assert!(hm.dmg.is_none()); // new 后未 init ≈ Java null
    hm.init();
    let dmg = hm.dmg.as_ref().unwrap();
    assert_eq!(dmg.id, 0);
    assert!(!dmg.updated);
}
