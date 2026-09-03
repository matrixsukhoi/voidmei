//! 渲染器数据层残部 (对应 src/ui/layout/renderer/ 的纯数据函数)。
//!
//! **JSON 配置变更 (Phase 1)**: switch/slider/combo/color 四个 apply 写链与
//! RenderContext/renderer_config_helper (PropertyBinder 反射复刻) 已整体退役 —
//! 写链收敛为 main_form::write_control 直调 ConfigurationService
//! (组字段 vs 行值二分 + SWITCH_INV 反转, 语义等价、无快照/挂起/重放)。
//! 本模块仅存: 行定位助手 (main_form 消息定位/快照查询用) + combo 选项解析。

pub mod combo;

use vm_core::config::json_model::RowConfig;

// =====================================================================
// 行定位助手 (消息 key → 行路径)
// =====================================================================

/// 在行树内按绑定键 (property) DFS 定位行, 返回索引路径; 无 property 的行以
/// label 匹配 (与服务侧 update_rows_recursive 同一命中谓词)。
/// 消息 key 来自行自身 (前端控件携带), 恒可命中。
pub(crate) fn find_row_path(rows: &[RowConfig], key: &str) -> Option<Vec<usize>> {
    for (i, r) in rows.iter().enumerate() {
        if r.property.as_deref() == Some(key) || (r.property.is_none() && key == r.label) {
            return Some(vec![i]);
        }
        if !r.children.is_empty() {
            if let Some(mut tail) = find_row_path(&r.children, key) {
                let mut path = vec![i];
                path.append(&mut tail);
                return Some(path);
            }
        }
    }
    None
}

pub(crate) fn row_by_path<'a>(rows: &'a [RowConfig], path: &[usize]) -> Option<&'a RowConfig> {
    let (&first, rest) = path.split_first()?;
    let row = rows.get(first)?;
    if rest.is_empty() {
        Some(row)
    } else {
        row_by_path(&row.children, rest)
    }
}
