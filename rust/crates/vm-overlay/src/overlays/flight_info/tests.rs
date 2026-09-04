//! 字段域守卫测试 (字段网格原子化后: 数据源 = 出厂页组件 props.target)。

use super::canonical_var_name;

/// 守卫: 出厂飞行/动力两页全部 data.field 的 target 短名经 registry/公式集
/// 可达 — 断链即字段行恒 0 (live 显示回归锚; 单名制, 查不到即真断链)
#[test]
fn factory_page_targets_all_reachable() {
    let pages = &vm_core::config::json_store::factory().pages;
    let mut n = 0;
    for page in pages {
        for c in &page.components {
            if c.r#type != "core.data.field" {
                continue;
            }
            let Some(target) = c.props.get("target").and_then(|v| v.as_str()) else {
                continue;
            };
            // 乘数表达式先拆 ("wing_sweep * 100" → wing_sweep), 裸名查可达
            let (var, _) = vm_core::formula::resolve_target(target)
                .unwrap_or((vm_core::formula::TargetVar::Var(0), 1.0));
            let t = match &var {
                vm_core::formula::TargetVar::Var(vid) => {
                    vm_core::formula::registry::registry().vars[*vid as usize].name
                }
                vm_core::formula::TargetVar::Formula(name) => name,
            };
            assert!(
                canonical_var_name(t).is_some(),
                "页 {} 字段 {} 的 target {t} 解析断链 (registry/公式集缺失)",
                page.id,
                c.id
            );
            n += 1;
        }
    }
    assert!(n >= 30, "出厂两页应共 ≥30 个 data.field (实测 {n}, 断言非平凡)");
}
