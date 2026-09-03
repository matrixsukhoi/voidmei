use super::*;
use crate::renderers::test_util::MapCtx;
use vm_core::config::config_loader::RowConfig;
use vm_core::ui_support::color::{parse_color, try_parse_color};

const DEF: [u8; 4] = [1, 2, 3, 4];

fn color_row(prop: Option<&str>, value: Option<&str>) -> RowConfig {
    let mut r = RowConfig::new("颜色".into(), None, "%s".into());
    r.r#type = "COLOR".into();
    r.property = prop.map(str::to_string);
    r.value = value.map(|v| ConfigValue::Str(v.to_string()));
    r
}

// ---- parse_color: hex 域 (基线: #FF5500/#ff5500aa/#FFGG00/#FFF/#/#FF5500A) ----
#[test]
fn parse_hex_formats() {
    assert_eq!(try_parse_color("#FF5500"), Some([255, 85, 0, 255]));
    assert_eq!(try_parse_color("#ff5500aa"), Some([255, 85, 0, 170])); // 小写
    assert_eq!(parse_color(" #FF5500 ", DEF), [255, 85, 0, 255]); // trim
                                                                  // 基线: 非法数字/长度 → 默认
    assert_eq!(parse_color("#FFGG00", DEF), DEF);
    assert_eq!(parse_color("#FFF", DEF), DEF);
    assert_eq!(parse_color("#", DEF), DEF);
    assert_eq!(parse_color("#FF5500A", DEF), DEF); // 7 位
    assert_eq!(parse_color("", DEF), DEF);
    assert_eq!(parse_color("   ", DEF), DEF);
    // 多字节 UTF-8 (Java parseInt 失败 + 防切包 panic)
    assert_eq!(parse_color("#FF中文", DEF), DEF);
}

// ---- parse_color: 十进制域 (基线: 钳位/尾逗号/非法串/分号) ----
#[test]
fn parse_decimal_formats() {
    assert_eq!(try_parse_color("255, 85, 0, 170"), Some([255, 85, 0, 170]));
    assert_eq!(try_parse_color("255,85,0"), Some([255, 85, 0, 255])); // 3 段 → a=255
    assert_eq!(
        try_parse_color(" 255 , 85 , 0 , 170 "),
        Some([255, 85, 0, 170])
    );
    // 基线: 尾逗号 → Java split 丢弃尾部空串 → 3 段
    assert_eq!(try_parse_color("255, 85, 0,"), Some([255, 85, 0, 255]));
    // 基线: 越界钳位 [0,255]
    assert_eq!(try_parse_color("300, -5, 0, 999"), Some([255, 0, 0, 255]));
    // 基线: 非法串 → 默认
    assert_eq!(parse_color("1,,3", DEF), DEF);
    assert_eq!(parse_color("a,b,c", DEF), DEF);
    assert_eq!(parse_color("256,10,10", DEF), [255, 10, 10, 255]); // 钳位不回默认
    assert_eq!(parse_color("255;0;0", DEF), DEF); // 分号非分隔符
    assert_eq!(parse_color("0xFF,0,0", DEF), DEF); // 十进制域不认 0x
    assert_eq!(parse_color("1.5,2,3", DEF), DEF);
    assert_eq!(parse_color("-FF,0,0", DEF), DEF);
}

// ---- 空白/trim 的 Java 语义 (基线: nbsp/vt/全角空格) ----
#[test]
fn java_trim_and_whitespace_semantics() {
    // nbsp 前缀: Java trim 不去 (>0x20) → 十进制路径解析失败 → 默认
    assert_eq!(parse_color("\u{00A0}#FF5500", DEF), DEF);
    // VT 内部: Java \s 含 \x0B → 剔除后解析成功
    assert_eq!(
        try_parse_color("255,\u{000B}85,\u{000B}0"),
        Some([255, 85, 0, 255])
    );
    // 控制符 (<=0x20) 前后缀: trim 剔除 → hex 路径成功
    assert_eq!(
        try_parse_color("\u{001F}#FF5500\u{0001}"),
        Some([255, 85, 0, 255])
    );
    // nbsp 内部: \s 不含 nbsp → parseInt 失败 → 默认
    assert_eq!(parse_color("255,\u{00A0}85, 0", DEF), DEF);
    // 全角空格: 同 nbsp → 默认
    assert_eq!(parse_color("\u{3000}255, 0, 0", DEF), DEF);
}

// ---- 格式化 + 存储串解析恒等 ----
#[test]
fn format_and_round_trip() {
    let c = [255, 85, 0, 170];
    assert_eq!(to_decimal_string(&c), "255, 85, 0, 170"); // oracle 格式
                                                          // 存储串解析恒等 (双格式互通的根据; hex 域见 parse_hex_formats)
    for c in [
        [0, 0, 0, 0],
        [255, 255, 255, 255],
        [232, 147, 50, 128],
        [1, 2, 3, 4],
    ] {
        assert_eq!(parse_color(&to_decimal_string(&c), DEF), c);
    }
}

// ---- 写链: 主键十进制 + 分键 R/G/B/A + row.value + on_save (Java L110-136) ----
#[test]
fn apply_writes_decimal_split_keys_and_saves() {
    let mut panel = GroupConfig::new("p".into());
    panel
        .rows
        .push(color_row(Some("fontWarn"), Some("#FF2400FF")));
    let ctx = MapCtx::default();

    apply(&mut panel, "fontWarn", [255, 36, 0, 128], &ctx);
    assert_eq!(
        panel.rows[0].value,
        Some(ConfigValue::Str("255, 36, 0, 128".into()))
    );
    assert_eq!(
        *ctx.calls.borrow(),
        vec![
            "syncStr:fontWarn=255, 36, 0, 128".to_string(),
            "syncStr:fontWarnR=255".to_string(),
            "syncStr:fontWarnG=36".to_string(),
            "syncStr:fontWarnB=0".to_string(),
            "syncStr:fontWarnA=128".to_string(),
            "on_save".to_string(),
        ]
    );
}

// 无 :target (label 键): 不触服务同步, 仍写 row.value + on_save (Java null-key
// 域的折叠 — Java NPE, Rust 保守降级)
#[test]
fn apply_label_key_skips_service_sync() {
    let mut panel = GroupConfig::new("p".into());
    panel.rows.push(color_row(None, Some("#FFFFFFFF")));
    let ctx = MapCtx::default();

    apply(&mut panel, "颜色", [0, 0, 0, 255], &ctx);
    assert_eq!(
        panel.rows[0].value,
        Some(ConfigValue::Str("0, 0, 0, 255".into()))
    );
    assert_eq!(*ctx.calls.borrow(), vec!["on_save".to_string()]);
}

// 未命中 key: 无副作用 (消息域外防护)
#[test]
fn apply_unknown_key_is_noop() {
    let mut panel = GroupConfig::new("p".into());
    panel.rows.push(color_row(Some("k"), Some("#FFFFFFFF")));
    let ctx = MapCtx::default();
    apply(&mut panel, "absent", [0, 0, 0, 0], &ctx);
    assert_eq!(
        panel.rows[0].value,
        Some(ConfigValue::Str("#FFFFFFFF".into()))
    );
    assert!(ctx.calls.borrow().is_empty());
}

// ---- 真实服务链: apply 经 WriteContext → set_config 更新服务树行值 (typed) ----
#[test]
fn apply_real_service_chain_updates_row_value() {
    use crate::main_form::WriteContext;
    use std::sync::Arc;
    use vm_core::config::config_api::ConfigProvider;
    use vm_core::config::configuration_service::ConfigurationService;

    let p = std::env::temp_dir().join("vm_ui_color_svc.cfg");
    std::fs::write(
        &p,
        r##"(panel "p" (item "告警色" :type color :target "fontWarn" :value "#FF2400FF"))"##,
    )
    .unwrap();
    let bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.load_layout(p.to_str().unwrap());

    let mut panel = config.get_layout_configs().unwrap().remove(0);
    let ctx = WriteContext::new(&config, &bus);
    apply(&mut panel, "fontWarn", [255, 36, 0, 128], &ctx);
    // 服务树: 行值 → 十进制串 (Java setConfig: Str 行存串)
    assert_eq!(config.get_config("fontWarn").unwrap(), "255, 36, 0, 128");
    let _ = std::fs::remove_file(&p);
}
