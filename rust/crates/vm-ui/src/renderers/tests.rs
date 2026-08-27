use super::*;
use vm_core::config_loader::ConfigValue;

// find_row_path: 按层定位 + 首个命中 (DFS 前序, 对位消息 key 的唯一来源行)
#[test]
fn find_row_path_locates_nested_and_misses() {
    let mut g = GroupConfig::new("p".into());
    let mut header = RowConfig::new("组".into(), None, "%s".into());
    header.r#type = "HEADER".into();
    let mut inner = RowConfig::new("内".into(), None, "%s".into());
    inner.r#type = "SWITCH".into();
    inner.property = Some("k2".into());
    header.children.push(inner);
    let mut top = RowConfig::new("顶".into(), None, "%s".into());
    top.r#type = "SWITCH".into();
    top.property = Some("k1".into());
    g.rows.push(top);
    g.rows.push(header);

    assert_eq!(find_row_path(&g.rows, "k1"), Some(vec![0]));
    assert_eq!(find_row_path(&g.rows, "k2"), Some(vec![1, 0]));
    assert_eq!(find_row_path(&g.rows, "absent"), None);
    // row_by_path / row_by_path_mut 往返
    assert_eq!(row_by_path(&g.rows, &[1, 0]).unwrap().property.as_deref(), Some("k2"));
    row_by_path_mut(&mut g.rows, &[1, 0]).unwrap().label = "改名".into();
    assert_eq!(row_by_path(&g.rows, &[1, 0]).unwrap().label, "改名");
    // 空路径 / 越界
    assert!(row_by_path(&g.rows, &[]).is_none());
    assert!(row_by_path(&g.rows, &[9]).is_none());
}

// 分发冒烟: 真实 cfg 树 → view_row 对九键 + 已注册未落地键 + 未知键全部产出
// 元素 (数据驱动, 对位 RowRendererRegistry.get 的恒有产出 + defaultRenderer 兜底)
#[test]
fn view_row_dispatches_all_registered_types() {
    let p = std::env::temp_dir().join("vm_ui_renderers_dispatch.cfg");
    std::fs::write(
        &p,
        r##"(panel "全类型"
  (item "开关" :type switch :target "k1" :value true)
  (item "反相" :type switch-inv :target "k2" :value false)
  (item "滑条" :type slider :target "k3" :min 0 :max 10 :value 5)
  (item "下拉" :type combo :target "k4" :source "A,B" :value "A")
  (item "颜色" :type color :target "fontWarn" :value "#FF2400FF")
  (item "文本" :type input :target "httpPort" :value 8111)
  (item "别名" :type text :value "t")
  (item "数据" :type data :target "getIAS" :value true)
  (item "按钮" :type button :target "factoryReset")
  (item "热键" :type hotkey :target "hudHotkey")
  (item "野类型" :type mystery :target "mkey")
)"##,
    )
    .unwrap();
    let config = vm_core::configuration_service::ConfigurationService::new(None);
    config.load_layout(p.to_str().unwrap());
    let groups = config.get_layout_configs().unwrap();
    let ctx = crate::main_form::ReadContext::new(&config);
    let panel = &groups[0];
    let no_opts = |_: &str, _: &str| Vec::<String>::new();
    for row in &panel.rows {
        // 每类型各产出元素 (panic/错型即败); hotkey 走 fallback_row 占位,
        // mystery 未知类型走 data::view_row (Java defaultRenderer 交互开关)
        let _el = view_row(row, panel, &ctx, &panel.title, &no_opts);
    }
    assert_eq!(panel.rows.len(), 11);
    assert_eq!(panel.rows[4].value, Some(ConfigValue::Str("#FF2400FF".into())));
}
