use super::*;

use crate::overlays::flight_info::cfg_rows;

fn fonts_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts")
}

/// state 直装 (fields_grid 工厂同款: ctx + FontTriple + preview 行落位)
fn state_of() -> FlightInfoState {
    let ctx = RenderCtx::new(0, 1, default_num_height(0));
    let fonts =
        crate::render::fields::FontTriple::load(&fonts_dir(), &ctx).expect("字体目录应可用");
    let mut st = FlightInfoState::with_resources(
        std::sync::Arc::new(cfg_rows("飞行信息")),
        ctx,
        fonts,
    );
    st.reset_preview_rows();
    st
}

/// live 喂数: update 覆写 rows, visible-when 过滤生效 (Mach>0 才显示的行,
/// 零值数据帧下被滤除 → 行数少于 FIELDS 数)
///
/// 零值视图桩 (var_value 全 0; 名字先经 canonical 可达性检查 — 对位生产
/// "公式槽 ∪ registry" 单名制通道, 曾直接 registry.lookup 把断链掩成桩内
/// 硬编码, 假绿掩盖 live 7 行消失)
struct ZeroView;
impl vm_core::formula::registry::FormulaView for ZeroView {
    fn var_value(&self, name: &str) -> Option<f64> {
        canonical_var_name(name).map(|_| 0.0)
    }
}
#[test]
fn update_applies_visibility() {
    let mut handle = state_of();
    handle.update(&ZeroView);
    let n_zero = handle.rows().len();
    // 全零值: Mach (>0) 等条件行被滤; 至少 IAS 等直通行保留
    assert!(n_zero > 0 && n_zero <= cfg_rows("飞行信息").len());

    // 非零 Mach 帧行数应不少于全零帧 (Mach 行回归)
    struct MachView;
    impl vm_core::formula::registry::FormulaView for MachView {
        fn var_value(&self, name: &str) -> Option<f64> {
            match canonical_var_name(name).as_deref() {
                Some("mach") => Some(0.72),
                Some(_) => Some(0.0),
                None => None,
            }
        }
    }
    handle.update(&MachView);
    let n_live = handle.rows().len();
    assert!(
        n_live >= n_zero,
        "非零帧可见行应不少于全零帧 ({n_live} vs {n_zero})"
    );
    // Mach 行真的回来了 (值 0.72 → 文本 "0.72") — 行存 def 索引, 经 FIELDS 回查 label
    let rows = handle.rows().to_vec();
    let defs = cfg_rows("飞行信息");
    let labels: Vec<&str> = rows.iter().map(|(i, _)| defs[*i].label.as_str()).collect();
    assert!(
        labels.contains(&"马赫数"),
        "非零 mach 帧行应可见: {labels:?}"
    );
}

/// 守卫: FIELDS 全部 target 短名经 registry/公式集可达 — 断链即行消失/恒 0
#[test]
fn flight_info_targets_all_reachable() {
    for f in cfg_rows("飞行信息") {
        // 乘数表达式先拆 ("wing_sweep * 100" → wing_sweep), 裸名查可达
        let (var, _) = vm_core::formula::resolve_target(&f.source)
            .unwrap_or((vm_core::formula::TargetVar::Var(0), 1.0));
        let t = match &var {
            vm_core::formula::TargetVar::Var(vid) => {
                vm_core::formula::registry::registry().vars[*vid as usize].name
            }
            vm_core::formula::TargetVar::Formula(name) => name,
        };
        assert!(
            canonical_var_name(t).is_some(),
            "飞行信息行 {} 的 target {t} 解析断链 (registry/公式集缺失)",
            f.label
        );
    }
}

/// WYSIWYG reinit (state 资源重建段): fontadd 0→6 → 高度变大; rows 回 preview
/// 初值 (行开关变更即时生效的回填面; 字段行绑定独立于字体)
#[test]
fn reinit_grows_with_font_add_and_keeps_rows() {
    let mut handle = state_of();
    let rows_before = handle.rows().to_vec();
    let h0 = handle.ctx.total_height(handle.rows().len() as i32);
    let (w1, h1) = handle
        .reinit(&fonts_dir(), 6, 1, std::sync::Arc::new(cfg_rows("飞行信息")))
        .expect("reinit 应成功");
    assert!(h1 > h0, "字号增量后高度应变大 ({} → {})", h0, h1);
    assert!(w1 > 0);
    assert_eq!(
        handle.rows(),
        rows_before.as_slice(),
        "reinit 后行集回 preview 全量 (cfg 行定义未变)"
    );
}

/// CloseAllOverlays 数据面重置 (组件 reset_preview 的 state 面):
/// live 行残留 (visible-when 过滤 + live 格式化值) → reset_preview_rows →
/// FIELDS 全量 preview 静态行。场景: 托盘 live→preview 后重开的预览窗
/// 不得显示上次 live 数值
#[test]
fn reset_preview_rows_restores_statics() {
    let mut handle = state_of();
    // live 残留: 非零 Mach/IAS 帧 (行集与 preview 静态不同)
    struct MachView;
    impl vm_core::formula::registry::FormulaView for MachView {
        fn var_value(&self, name: &str) -> Option<f64> {
            match canonical_var_name(name).as_deref() {
                Some("mach") => Some(0.72),
                Some(_) => Some(0.0),
                None => None,
            }
        }
    }
    handle.update(&MachView);
    // 重置 → preview 行: FIELDS 全量 + preview_text 原样
    handle.reset_preview_rows();
    let rows = handle.rows().to_vec();
    let defs = cfg_rows("飞行信息");
    assert_eq!(rows.len(), defs.len(), "回全量行 (visible-when 过滤清除)");
    for (row, f) in rows.iter().zip(defs.iter()) {
        // (波22: 行形态 = def 索引 + 值文本; 索引序与全量 defs 一一对应)
        assert_eq!(defs[row.0].label, f.label);
        assert_eq!(row.1, f.preview_value, "值列回 preview 静态: {}", f.label);
    }
}

// 旧 flight_info_overlay_spec 工厂的渲染闭包测试已随工厂退役删除
// (W3: host 挂载面 = widgets::fields_grid 直通管线, 数据推进链断言见
//  vm-app render_feeds::feed_overlays_live_updates_all_handles)。
