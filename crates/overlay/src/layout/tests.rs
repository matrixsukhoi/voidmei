use super::*;

/// 默认参数: fontAdd=0, columnNum=1 (ui_layout.cfg 默认)
#[test]
fn default_layout() {
    let ctx = RenderCtx::new(0, 1, 33);
    assert_eq!(ctx.font_size, 24);
    assert_eq!(ctx.label_font_size, 12);
    assert_eq!(ctx.unit_font_size, 12);
    assert_eq!(ctx.total_width(), 192); // 12 + 180
    assert_eq!(ctx.total_height(16), 33 * 18); // 1 + 16 + 1 行
    assert_eq!(ctx.total_height(15), 33 * 17); // 15 行 → 1 + 15 + 1
    assert_eq!(ctx.start_offset(), (12, 12));
    assert_eq!(ctx.advance_x(), 120);
    assert_eq!(ctx.advance_y(), 33);
    assert_eq!(ctx.lwidth(), 78); // 13*24>>2
    assert_eq!(ctx.num_padding(), 6); // max(4, 6)
}

#[test]
fn multi_column_layout() {
    // columnNum=3: 宽 = 12 + 3.5*120 = 432
    let ctx = RenderCtx::new(0, 3, 33);
    assert_eq!(ctx.total_width(), 432);
    // 16 字段 3 列 → ceil(16/3)=6 行 → 33*(1+6+1) = 264
    assert_eq!(ctx.total_height(16), 264);
    // 恰好整除: 6 字段 3 列 → 2 行 → 33*4
    assert_eq!(ctx.total_height(6), 33 * 4);
}

#[test]
fn font_add_variants() {
    // fontAdd=4: fontSize=28, labelSize=round(14)=14
    let ctx = RenderCtx::new(4, 1, 39);
    assert_eq!(ctx.font_size, 28);
    assert_eq!(ctx.label_font_size, 14);
    assert_eq!(ctx.lwidth(), (13 * 28) >> 2); // 91
    assert_eq!(ctx.num_padding(), 7);
    // fontAdd=-6: fontSize=18, round(9)=9
    let ctx = RenderCtx::new(-6, 1, 26);
    assert_eq!(ctx.font_size, 18);
    assert_eq!(ctx.label_font_size, 9);
    assert_eq!(ctx.num_padding(), 4); // max(4, 18/4=4) = 4
}

#[test]
fn value_baseline_position() {
    let ctx = RenderCtx::new(0, 1, 33);
    // y=12: (12+12+12+12)>>1 = 24
    assert_eq!(ctx.value_baseline(12), 24);
    assert_eq!(ctx.unit_baseline(12), 24); // 12+12
}
