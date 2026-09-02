use super::*;
use crate::main_form::{update, Message};
use crate::renderers::test_util::{state_from_cfg, MapCtx};
use vm_core::config::config_loader::{ConfigValue, RowConfig};

fn combo_row(prop: Option<&str>, source: &str, value: Option<&str>) -> RowConfig {
    let mut r = RowConfig::new("字体".into(), None, "%s".into());
    r.r#type = "COMBO".into();
    r.property = prop.map(str::to_string);
    r.format = source.to_string(); // loader: :source 覆写 format
    r.value = value.map(|v| ConfigValue::Str(v.to_string()));
    r
}

// 字面量源: split(","); 空串 → [""] (Java split 逐位一致)
#[test]
fn resolve_literal_options() {
    assert_eq!(
        resolve_options("A,B,C", ""),
        vec!["A".to_string(), "B".to_string(), "C".to_string()]
    );
    assert_eq!(resolve_options("单选", ""), vec!["单选".to_string()]);
    assert_eq!(resolve_options("", ""), vec![String::new()]);
}

// _FONTS_: 当前值单选占位 (AWT 枚举无对应物, 见模块文档)
#[test]
fn resolve_fonts_placeholder() {
    assert_eq!(resolve_options("_FONTS_", "Sarasa Mono SC"), vec!["Sarasa Mono SC".to_string()]);
}

// _CROSSHAIRS_ 分发: 头部恒为软件渲染准星 (与 CWD 是否有目录无关)
#[test]
fn resolve_crosshairs_dispatch_keeps_head_item() {
    let opts = resolve_options("_CROSSHAIRS_", "");
    assert!(!opts.is_empty());
    assert_eq!(opts[0], SOFTWARE_CROSSHAIR);
}

// 目录条目去扩展名 + 头部项; 目录缺失 → 仅头部 (Java L76-85: dir.list()==null
// → String[0] → combined=["软件渲染准星"])。注入绝对路径, 不动进程 CWD。
#[test]
fn crosshair_options_dir_and_missing_dir() {
    let missing = std::env::temp_dir().join("vm_ui_combo_no_such_dir_zzz");
    let _ = std::fs::remove_dir_all(&missing);
    assert_eq!(
        crosshair_options(missing.to_str().unwrap()),
        vec![SOFTWARE_CROSSHAIR.to_string()]
    );

    let dir = std::env::temp_dir().join("vm_ui_combo_gunsight_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("alpha.png"), b"x").unwrap();
    std::fs::write(dir.join("beta"), b"x").unwrap();
    std::fs::write(dir.join("文件.tar.gz"), b"x").unwrap(); // 多点只截最后一个

    let opts = crosshair_options(dir.to_str().unwrap());
    assert_eq!(opts[0], SOFTWARE_CROSSHAIR);
    assert_eq!(opts.len(), 4, "alpha/beta/文件.tar.gz: {opts:?}");
    assert!(opts.contains(&"alpha".to_string()), "去扩展名: {opts:?}");
    assert!(opts.contains(&"beta".to_string()), "无扩展名原样: {opts:?}");
    assert!(opts.contains(&"文件.tar".to_string()), "多点截最后: {opts:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

// 读链已删 (D9 回显走整树 DTO); 读链的 Java 优先级语义 (PropertyBinder 组字段
// > 服务 > 行默认) 仍由 renderer_config_helper 的 read_string 测试锁定。

// 写链: row.value + 组字段 fontName + 服务同步 + on_save (Java L52-62)
#[test]
fn apply_writes_row_group_field_and_service() {
    let mut panel = GroupConfig::new("引擎信息".into());
    panel.rows.push(combo_row(Some("fontName"), "_FONTS_", Some("旧字体")));
    let ctx = MapCtx::default();

    apply(&mut panel, "fontName", "DIN Pro 400", &ctx);
    assert_eq!(panel.rows[0].value, Some(ConfigValue::Str("DIN Pro 400".into())));
    assert_eq!(panel.font_name.as_deref(), Some("DIN Pro 400"));
    assert_eq!(
        *ctx.calls.borrow(),
        vec!["syncStr:fontName=DIN Pro 400".to_string(), "on_save".to_string()]
    );
}

// 未命中 key 的消息: 写链不触达 (消息域外防护)
#[test]
fn apply_unknown_key_is_noop() {
    let mut panel = GroupConfig::new("p".into());
    panel.rows.push(combo_row(Some("style"), "A,B", Some("A")));
    let ctx = MapCtx::default();
    apply(&mut panel, "absent", "B", &ctx);
    assert_eq!(panel.rows[0].value, Some(ConfigValue::Str("A".into())));
    assert!(ctx.calls.borrow().is_empty());
}

// 真实链: INPUT 行经 Message::Combo 写回 — 服务 + 快照 + on_save 即落盘
// (ui_layout.cfg 实况: "8111端口" :type input :target httpPort)
// 路由等价备案 (原 text.rs): Java TextRowRenderer 闭包体与 Combo 逐步同构,
// 提交时机归 web 壳 (JS 输入框), 消息形状不变。
#[test]
fn combo_message_routes_text_write_chain() {
    let mut state = state_from_cfg(
        "text_route",
        r#"(panel "连接" (item "8111端口" :type input :target "httpPort" :value 8111 :default 8111))"#,
        None,
    );
    update(
        &mut state,
        Message::Combo { panel: "连接".into(), key: "httpPort".into(), value: "9222".into() },
    );
    assert_eq!(state.service_string("httpPort"), "9222");
    // Int 行经 setConfig 保持 Int 形态 (Java instanceof Integer 分支)
    assert_eq!(state.snapshot_row("连接", "httpPort").unwrap().get_int(), 9222);
}

// 真实链: 无 :target 文本行 — row.value 落快照 + onSave 即落盘 (persist 收敛
// 服务树; Java L57-67: prop=null 不落服务, row.value 在共享树本体上)
#[test]
fn combo_message_unkeyed_row_writes_row_value_only() {
    let persist = std::env::temp_dir().join("vm_ui_text_unkeyed_user.cfg");
    let _ = std::fs::remove_file(&persist);
    let mut state = state_from_cfg(
        "text_unkeyed",
        r#"(panel "P" (item "备注" :type text :value "旧"))"#,
        Some(persist.to_string_lossy().into_owned()),
    );
    update(
        &mut state,
        Message::Combo { panel: "P".into(), key: "备注".into(), value: "新".into() },
    );
    let row = state.snapshot_row("P", "备注").unwrap();
    assert_eq!(row.value, Some(ConfigValue::Str("新".into())));
    assert!(persist.exists(), "onSave 即落盘");
    // 挂起重放 → 落盘 → 重载: 服务树同 label 行持 "新" (get_config 按 label 命中)
    assert_eq!(state.service_string("备注"), "新");
    let _ = std::fs::remove_file(&persist);
}
