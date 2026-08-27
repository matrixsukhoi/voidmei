use super::*;

fn fonts_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts")
}

fn params_cell(mutate: impl FnOnce(&mut ReinitParams)) -> Rc<RefCell<ReinitParams>> {
    let mut p = ReinitParams::default();
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
    assert_eq!(handle.borrow().rows().len(), fields::FIELDS.len());
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
/// 零值 FlightValues 下被滤除 → 行数少于 FIELDS 数)
#[test]
fn update_from_values_applies_visibility() {
    let (handle, _spec) =
        flight_info_overlay_spec(&fonts_dir(), &params_cell(|_| {})).unwrap();
    let zero = FlightValues::default();
    handle.borrow_mut().update_from_values(&zero);
    let n_zero = handle.borrow().rows().len();
    // 全零值: Mach (>0) 等条件行被滤; 至少 IAS 等直通行保留
    assert!(n_zero > 0 && n_zero <= fields::FIELDS.len());

    // 非零 Mach 帧行数应不少于全零帧 (Mach 行回归)
    let mut v = FlightValues::default();
    v.mach = 0.72;
    v.ias = 450.0;
    handle.borrow_mut().update_from_values(&v);
    let n_live = handle.borrow().rows().len();
    assert!(n_live >= n_zero, "非零帧可见行应不少于全零帧 ({n_live} vs {n_zero})");
}

/// WYSIWYG reinit: fontadd 0→6 → 高度变大; live rows 保留 (字段行绑定独立于字体)
#[test]
fn reinit_grows_with_font_add_and_keeps_rows() {
    let cell = params_cell(|_| {});
    let (handle, mut spec) = flight_info_overlay_spec(&fonts_dir(), &cell).unwrap();
    // live 行覆盖 (行数可能少于 FIELDS — visible-when 过滤)
    handle.borrow_mut().update_from_values(&FlightValues::default());
    let rows_before = handle.borrow().rows().to_vec();
    let h0 = spec.height;
    cell.borrow_mut().font_add_flight = 6;
    let (w1, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert!(h1 > h0, "字号增量后高度应变大 ({} → {})", h0, h1);
    assert!(w1 > 0);
    assert_eq!(handle.borrow().rows(), rows_before.as_slice(), "reinit 不动字段行数据");
}
