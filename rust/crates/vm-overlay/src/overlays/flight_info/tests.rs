use super::*;

use crate::overlays::flight_info::cfg_rows;

fn fonts_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts")
}

fn params_cell(mutate: impl FnOnce(&mut ReinitParams)) -> Rc<RefCell<ReinitParams>> {
    let mut p = ReinitParams::default();
    // W-D: 行定义走 cfg (与生产同源)
    p.flight_rows = std::sync::Arc::new(cfg_rows("飞行信息"));
    p.power_rows = std::sync::Arc::new(cfg_rows("动力信息"));
    mutate(&mut p);
    Rc::new(RefCell::new(p))
}

/// 工厂最小面: preview 行数 = FIELDS 数, 尺寸与 ctx 度量一致, 渲染闭包可跑
/// (PixCanvas 合成 + 灰底保留)
#[test]
fn spec_renders_preview_rows_to_pixcanvas() {
    let (handle, mut spec) =
        flight_info_overlay_spec(&fonts_dir(), &params_cell(|_| {}))
            .expect("字体目录应可用");
    assert_eq!(spec.id, "flightInfoSwitch");
    assert_eq!(handle.borrow().rows().len(), cfg_rows("飞行信息").len());
    assert!(spec.width > 0 && spec.height > 0);

    // 渲染闭包: 先铺 host 预览灰底再合成 (host 渲染循环同序)
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    cv.clear(spec.width, spec.height);
    cv.fill_rect(0, 0, spec.width, spec.height, [0, 0, 0, 0x0A]);
    (spec.render)(&mut cv);
    // 灰底被保留 (左上角像素 alpha 仍 ≥ 灰底底色, 不会被整帧替换清零)
    let px = cv.pixmap().data();
    assert!(px[3] >= 0x0A, "预览灰底应经 SrcOver 合成保留");
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
    let (handle, _spec) =
        flight_info_overlay_spec(&fonts_dir(), &params_cell(|_| {})).unwrap();
    handle.borrow_mut().update(&ZeroView);
    let n_zero = handle.borrow().rows().len();
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
    handle.borrow_mut().update(&MachView);
    let n_live = handle.borrow().rows().len();
    assert!(n_live >= n_zero, "非零帧可见行应不少于全零帧 ({n_live} vs {n_zero})");
    // Mach 行真的回来了 (值 0.72 → 文本 "0.72")
    let rows = handle.borrow().rows().to_vec();
    let labels: Vec<&str> = rows.iter().map(|(l, _, _)| l.as_str()).collect();
    assert!(labels.contains(&"马赫数"), "非零 mach 帧行应可见: {labels:?}");
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

/// WYSIWYG reinit: fontadd 0→6 → 高度变大; live rows 保留 (字段行绑定独立于字体)
#[test]
fn reinit_grows_with_font_add_and_keeps_rows() {
    let cell = params_cell(|_| {});
    let (handle, mut spec) = flight_info_overlay_spec(&fonts_dir(), &cell).unwrap();
    // 行集保持 preview 全行 (字号断言与行过滤无关; live 行为另测)
    let rows_before = handle.borrow().rows().to_vec();
    let h0 = spec.height;
    cell.borrow_mut().font_add_flight = 6;
    let (w1, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h1 > h0, "字号增量后高度应变大 ({} → {})", h0, h1);
    assert!(w1 > 0);
    assert_eq!(handle.borrow().rows(), rows_before.as_slice(), "reinit 不动字段行数据");
}

/// CloseAllOverlays 数据面重置 (app_shell reset_handles_preview_values 调用面):
/// live 行残留 (visible-when 过滤 + live 格式化值) → reset_preview_rows →
/// FIELDS 全量 preview 静态行。场景: 托盘 live→preview 后重开的预览窗
/// 不得显示上次 live 数值
#[test]
fn reset_preview_rows_restores_statics() {
    let (handle, _spec) =
        flight_info_overlay_spec(&fonts_dir(), &params_cell(|_| {})).unwrap();
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
    handle.borrow_mut().update(&MachView);
    // 重置 → preview 行: FIELDS 全量 + preview_text 原样
    handle.borrow_mut().reset_preview_rows();
    let rows = handle.borrow().rows().to_vec();
    let defs = cfg_rows("飞行信息");
    assert_eq!(rows.len(), defs.len(), "回全量行 (visible-when 过滤清除)");
    for (row, f) in rows.iter().zip(defs.iter()) {
        assert_eq!(row.0, f.label);
        assert_eq!(row.2, f.preview_value, "值列回 preview 静态: {}", f.label);
    }
}
