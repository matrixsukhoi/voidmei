use super::*;
use vm_core::config::config_loader::GroupConfig;

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
