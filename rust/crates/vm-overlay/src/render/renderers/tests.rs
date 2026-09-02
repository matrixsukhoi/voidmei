use super::*;

const FONTS: &str = "../../../fonts";

/// 几何断言走确定性非 AA 路径 (组件内核为 fillRect 像素精确复刻)
fn ctx_n(column_num: i32) -> RenderContext {
    let mut ctx = RenderContext::load(std::path::Path::new(FONTS), 0, column_num).unwrap();
    ctx.graph_aa = false;
    ctx.text_aa = false;
    ctx
}

/// 读像素: to_premul_bgra (BGRA) → [r,g,b,a], 颜色分量为预乘值 (断言以 alpha 为主:
/// 空=0, shade=42, colorNum 填充=240)
fn px(buf: &[u8], w: i32, x: i32, y: i32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [buf[i + 2], buf[i + 1], buf[i], buf[i + 3]]
}

fn alpha(buf: &[u8], w: i32, x: i32, y: i32) -> u8 {
    px(buf, w, x, y)[3]
}

fn data_field(label: &str, unit: &str, value: &str) -> DataField {
    let mut d = DataField::new("k", label, unit, "c", false, false);
    d.set_value(value);
    d
}

fn row_has_pixels(buf: &[u8], w: i32, y: i32) -> bool {
    (0..w).any(|x| alpha(buf, w, x, y) > 0)
}

// ---- RenderContext (RenderContext.java:68-81/:104-121) ----

/// 字号派生: label/unit = Math.round(fontSize/2.0f); num_height 同源度量
#[test]
fn render_context_geometry_and_half_sizes() {
    let ctx = ctx_n(3);
    assert_eq!(ctx.font_size(), 24);
    assert_eq!(ctx.num_font.size, 24);
    assert_eq!(ctx.label_font.size, 12); // round(12.0)=12
    assert_eq!(ctx.unit_font.size, 12);
    assert_eq!(ctx.num_height(), ctx.num_font.metrics().height);
    assert_eq!(ctx.field_width(), 72); // 3*fontSize
    assert_eq!(ctx.geom.total_width(), 432); // 12 + 3.5*5*24
    // fontAdd=1 → fontSize=25 → half = Math.round(12.5) = 13 (§2.3 半舍入)
    let ctx25 = RenderContext::load(std::path::Path::new(FONTS), 1, 3).unwrap();
    assert_eq!(ctx25.font_size(), 25);
    assert_eq!(ctx25.label_font.size, 13);
    assert_eq!(ctx25.unit_font.size, 13);
}

// ---- BosStyleRenderer (BOSStyleRenderer.java:21-90) ----

/// offset 网格推进 (columnNum 换行) + TextGauge 按 label 缓存 + 单位动态同步
#[test]
fn bos_style_offset_grid_and_cache() {
    let ctx = ctx_n(3);
    let f1 = data_field("A1", "Km/h", "100");
    let f2 = data_field("A2", "Km/h", "200");
    let f3 = data_field("A3", "Km/h", "300");
    let f4 = data_field("A4", "M", "400");
    let fields = [Field::Data(&f1), Field::Data(&f2), Field::Data(&f3), Field::Data(&f4)];
    let mut r = BosStyleRenderer::default();
    assert_eq!(r.cached_gauge_count(), 0);
    let dynr: &mut dyn OverlayRenderer = &mut r;
    let (w, h) = dynr.calculate_preferred_size(&fields, &ctx);
    assert_eq!(w, 432); // total_width
    assert_eq!(h, ctx.geom.total_height(4)); // 4 字段 3 列: addnum=1 → 3 行
    let mut c = PixCanvas::new(w, h).unwrap();
    let mut offset = [999, 999]; // 进入值应被覆盖 (Java :30-31)
    dynr.render(&mut c, &fields, &ctx, &mut offset);
    // 4 可见 → 4%3≠0: ox=12+120, oy=12+num_height
    assert_eq!(offset, [132, 12 + ctx.num_height()]);
    assert_eq!(r.cached_gauge_count(), 4, "按 label 缓存 4 个 TextGauge");
    // 单位动态同步 (Java :52-53): 同 label 换单位 → 缓存内更新, 不新建
    let f4b = data_field("A4", "Ft", "400");
    let fields2 = [Field::Data(&f4b)];
    let mut off2 = [0, 0];
    OverlayRenderer::render(&mut r, &mut c, &fields2, &ctx, &mut off2);
    assert_eq!(r.cached_unit("A4"), Some("Ft"));
    assert_eq!(r.cached_gauge_count(), 4, "缓存复用");
}

/// 内容包围盒: 不越左缘 (fontSize>>1), 第 4 字段落位第二行, 不超首选高
#[test]
fn bos_style_content_bounds() {
    let ctx = ctx_n(3);
    let f1 = data_field("A1", "Km/h", "100");
    let f2 = data_field("A2", "Km/h", "200");
    let f3 = data_field("A3", "Km/h", "300");
    let f4 = data_field("A4", "M", "400");
    let fields = [Field::Data(&f1), Field::Data(&f2), Field::Data(&f3), Field::Data(&f4)];
    let mut r = BosStyleRenderer::default();
    let (w, h) = r.calculate_preferred_size(&fields, &ctx);
    let mut c = PixCanvas::new(w, h).unwrap();
    let mut offset = [0, 0];
    r.render(&mut c, &fields, &ctx, &mut offset);
    let b = c.to_premul_bgra();
    let (mut min_x, mut min_y, mut max_y, mut any) = (i32::MAX, i32::MAX, i32::MIN, false);
    for y in 0..h {
        for x in 0..w {
            if alpha(&b, w, x, y) > 0 {
                any = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(any, "有内容");
    assert!(min_x >= 12, "内容不越左缘 (offset 初值 fontSize>>1), min_x={min_x}");
    assert!(min_y < 15, "首行标签贴近顶部, min_y={min_y}");
    assert!(max_y >= 12 + ctx.num_height() - 11, "第 4 字段进入第二行, max_y={max_y}");
    assert!(max_y < h, "不超首选高");
}

// ---- 数值文本双通道解析 ----

/// buffer/length 通道优先于字符串值 (TextGauge.java:54 / LinearGauge.java:266)
#[test]
fn value_text_channel_resolution() {
    let mut d = data_field("L", "U", "88");
    d.buffer = "88".to_string();
    d.length = 2;
    assert_eq!(bos_value_text(&d, "  88"), "88");
    d.length = 0;
    assert_eq!(bos_value_text(&d, "  88"), "  88");
    let mut gf = GaugeField::new("rpm", "RPM", "", 1, 100, false);
    gf.update_gauge(50, "50");
    assert_eq!(gauge_value_text(&gf), "50");
    gf.base.buffer = "49".to_string();
    gf.base.length = 2;
    assert_eq!(gauge_value_text(&gf), "49");
}

/// AA 路径冒烟: 默认 graphAA/textAA 全开不 panic 且有输出
#[test]
fn aa_smoke_all_renderers() {
    let ctx = RenderContext::load(std::path::Path::new(FONTS), 0, 3).unwrap();
    let mut g = GaugeField::new("rpm", "RPM", "", 1, 100, true);
    g.update_gauge(50, "50");
    let d = data_field("IAS", "Km/h", "800");
    let fields = [Field::Gauge(&g), Field::Data(&d)];
    let mut c = PixCanvas::new(432, 200).unwrap();
    let mut off = [0, 0];
    LinearGaugeRenderer::default().render(&mut c, &fields, &ctx, &mut off);
    let mut off = [0, 0];
    BosStyleRenderer::default().render(&mut c, &fields, &ctx, &mut off);
    let mut off = [0, 0];
    TextOnlyRenderer.render(&mut c, &fields, &ctx, &mut off);
    assert!(c.to_premul_bgra().iter().any(|&v| v != 0), "AA 输出非空");
}

// ---- 缓存失效契约 / 防护 ----

/// clear() 失效钩子: 清空后按新字段构造参数 (label, max_value) 重建组件 —
/// 等价 Java "LabeledLinearGauge 组件随 GaugeField 重建而消亡"
#[test]
fn linear_gauge_renderer_clear_rebuilds_with_new_range() {
    let ctx = ctx_n(3);
    let mut gf = GaugeField::new("rpm", "RPM", "", 1, 100, true);
    gf.update_gauge(50, "50");
    let fields = [Field::Gauge(&gf)];
    let mut r = LinearGaugeRenderer::default();
    let mut c = PixCanvas::new(192, 216).unwrap();
    let mut off = [0, 0];
    r.render(&mut c, &fields, &ctx, &mut off);
    assert_eq!(r.gauges.get("rpm").unwrap().gauge.max_value, 100);
    // 模拟 FieldOverlay.reinitConfig → clearAll 后同 key 字段换量程 (100→200)
    let mut gf2 = GaugeField::new("rpm", "RPM", "", 1, 200, true);
    gf2.update_gauge(50, "50");
    let fields2 = [Field::Gauge(&gf2)];
    r.clear();
    assert!(r.gauges.is_empty(), "clear 后缓存空");
    let mut off = [0, 0];
    r.render(&mut c, &fields2, &ctx, &mut off);
    assert_eq!(r.gauges.get("rpm").unwrap().gauge.max_value, 200, "重建后取新量程");
}

/// 不 clear 直接复用 → 首帧量程固化 (钉死契约失败面: 调用方必须 clear,
/// Java 中组件随字段重建天然刷新, Rust 缓存不会)
#[test]
fn linear_gauge_renderer_cache_freezes_range_without_clear() {
    let ctx = ctx_n(3);
    let mut gf = GaugeField::new("rpm", "RPM", "", 1, 100, true);
    gf.update_gauge(50, "50");
    let fields1 = [Field::Gauge(&gf)];
    let mut r = LinearGaugeRenderer::default();
    let mut c = PixCanvas::new(192, 216).unwrap();
    let mut off = [0, 0];
    r.render(&mut c, &fields1, &ctx, &mut off);
    let mut gf2 = GaugeField::new("rpm", "RPM", "", 1, 200, true);
    gf2.update_gauge(50, "50");
    let fields2 = [Field::Gauge(&gf2)];
    let mut off = [0, 0];
    r.render(&mut c, &fields2, &ctx, &mut off);
    assert_eq!(r.gauges.len(), 1, "同 key 复用");
    assert_eq!(r.gauges.get("rpm").unwrap().gauge.max_value, 100, "量程陈旧 (契约失败面)");
}

/// BOS clear(): 仅释放语义 (Java gaugeCache 本就跨字段重建存活, 无失效需求)
#[test]
fn bos_style_clear_releases_cache() {
    let ctx = ctx_n(3);
    let f = data_field("A1", "Km/h", "100");
    let fields = [Field::Data(&f)];
    let mut r = BosStyleRenderer::default();
    let mut c = PixCanvas::new(60, 40).unwrap();
    let mut off = [0, 0];
    r.render(&mut c, &fields, &ctx, &mut off);
    assert_eq!(r.cached_gauge_count(), 1);
    r.clear();
    assert_eq!(r.cached_gauge_count(), 0, "clear 后缓存空");
    let mut off = [0, 0];
    r.render(&mut c, &fields, &ctx, &mut off);
    assert_eq!(r.cached_gauge_count(), 1, "再渲染按需重建");
}

/// load() 字体校验: 损坏 ttf 返回 Err 而非 metrics() 处 panic
/// (Java new Font 静默回退 dialog 字体; Rust 无回退族 → 显式 Err, POC 裁决)
#[test]
fn load_rejects_corrupt_font() {
    let dir = std::env::temp_dir().join("vm_overlay_renderers_font_test");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("sarasa-mono-sc-bold.ttf"), b"definitely not a ttf").unwrap();
    let result = RenderContext::load(&dir, 0, 3);
    let _ = std::fs::remove_dir_all(&dir);
    let err = match result {
        Ok(_) => panic!("损坏字体应返回 Err, 却构造成功"),
        Err(e) => e,
    };
    assert!(
        err.contains("损坏") || err.contains("无法解析"),
        "错误信息应指向字体损坏: {err}"
    );
}

/// columnNum==0 程序化误用: 显式 panic (Java 同位置 ArithmeticException, 行为对等)
#[test]
#[should_panic(expected = "columnNum==0")]
fn bos_style_column_zero_panics_like_java() {
    let ctx = ctx_n(0);
    let f = data_field("A1", "Km/h", "100");
    let fields = [Field::Data(&f)];
    let mut r = BosStyleRenderer::default();
    let mut c = PixCanvas::new(10, 10).unwrap();
    let mut off = [0, 0];
    r.render(&mut c, &fields, &ctx, &mut off);
}

/// 默认态 palette() = Java 静态初始值 (对拍基线常量同源)。
/// 动态性 (set 后跟随) 不以 set 操演断言: 并行渲染测试读同一仓, set 窗口
/// 会互踩 (实测撞 linear_gauge 断言); palette() 方法体直调 colors() 无缓存,
/// 动态性由结构保证 + global_colors.rs 的仓往返测试覆盖
#[test]
fn palette_defaults_to_java_static_values() {
    assert_eq!(ctx_n(1).palette(), APPLICATION_COLORS);
}
