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

// ---- LinearGaugeRenderer (LinearGaugeRenderer.java:20-74) ----

/// 布局推进: 竖条 dx += (5*fs)>>1; 横条画于 (x, y+fs>>1);
/// 首选尺寸 = (8*fs, (13*fs>>1) + (rowNum+1)*(fs + fs>>2)) — 移位优先级保真
#[test]
fn linear_gauge_renderer_layout_advancement() {
    let ctx = ctx_n(3);
    let mut g1 = GaugeField::new("rpm", "RPM", "R", 1, 3000, false);
    g1.update_gauge(0, "0");
    let mut g2 = GaugeField::new("thr", "THR", "%", 2, 110, false);
    g2.update_gauge(0, "0");
    let mut g3 = GaugeField::new("flt", "FLT", "%", 3, 100, true);
    g3.update_gauge(0, "0");
    let fields = [Field::Gauge(&g1), Field::Gauge(&g2), Field::Gauge(&g3)];
    let mut r: Box<dyn OverlayRenderer> = Box::new(LinearGaugeRenderer::default());
    let (w, h) = r.calculate_preferred_size(&fields, &ctx);
    // width=192; height=(24*4+24*9)>>1 + (1+1)*(24+6) = 156+60 = 216
    assert_eq!((w, h), (192, 216));
    let mut c = PixCanvas::new(w, h + 20).unwrap();
    let mut offset = [0, 0];
    r.render(&mut c, &fields, &ctx, &mut offset);
    let b = c.to_premul_bgra();
    let m = |t: &str| ctx.label_font.measure(t);
    // 竖条默认分支: 条左缘 = x + (labelW+valueW) + 2
    let tw1 = m("RPM") + m("0");
    let tw2 = m("THR") + m("0");
    assert_eq!(alpha(&b, w, tw1 + 2, 0), 42, "竖条1 条框左上");
    assert_eq!(alpha(&b, w, 60 + tw2 + 2, 0), 42, "竖条2 条框左上 (dx=(5*24)>>1=60)");
    // 横条: (x, y + fs>>1) = (0, 12), 边框盒 0..95 × 12..23; cur=0 → 分隔主线
    // 恰落列 0, 叠在边框上 (colorNum 叠 shade → SrcOver 加深, Java 同款叠色)
    assert!(
        (241..=245).contains(&(alpha(&b, w, 0, 12) as i32)),
        "横条框左上+分隔主线叠加"
    );
    assert_eq!(alpha(&b, w, 5, 12), 42, "横条框上边 (避开分隔线列)");
    assert_eq!(alpha(&b, w, 95, 12), 42, "横条框右上");
    assert_eq!(alpha(&b, w, 96, 12), 0, "横条盒外");
}

/// 横条接线 (经渲染器全链路): 填充宽 valW-2 / 分隔线主线+影线各 1 列,
/// 高度 thickness+字号+2 (gauges_bars 内核几何的渲染器侧钉死)
#[test]
fn linear_gauge_renderer_horizontal_wiring() {
    let ctx = ctx_n(3);
    let mut gf = GaugeField::new("flt", "FLT", "%", 3, 100, true);
    gf.update_gauge(50, "50"); // pix = round(50*96/100) = 48
    let fields = [Field::Gauge(&gf)];
    let mut r = LinearGaugeRenderer::default();
    let mut c = PixCanvas::new(192, 60).unwrap();
    let mut offset = [0, 0];
    r.render(&mut c, &fields, &ctx, &mut offset);
    let b = c.to_premul_bgra();
    // (x, y+12)=(0,12), length=96, thickness=12:
    // 边框盒 0..95 × 12..23; 填充 (1,13,46,10) → 列 1..46 × 行 13..22
    assert_eq!(alpha(&b, 192, 1, 13), 240, "填充左上内");
    assert_eq!(alpha(&b, 192, 46, 22), 240, "填充右下内 (valW-2)");
    assert_eq!(alpha(&b, 192, 47, 17), 0, "填充右外");
    // 分隔线: 主线列 48, 影线列 49, 行 12..12+26
    assert_eq!(alpha(&b, 192, 48, 17), 240, "主线");
    assert_eq!(alpha(&b, 192, 49, 17), 42, "影线 (+1)");
    assert_eq!(alpha(&b, 192, 47, 17), 0, "主线左邻空");
    assert_eq!(alpha(&b, 192, 50, 17), 0, "影线右邻空");
    assert_eq!(alpha(&b, 192, 48, 38), 240, "主线末端行 (y+thickness+font+2)");
    assert_eq!(alpha(&b, 192, 48, 39), 0, "线外");
}

/// 首选尺寸计数: 仅可见 GaugeField; Data 字段与不可见字段不参与
#[test]
fn linear_gauge_renderer_preferred_counts() {
    let ctx = ctx_n(3);
    let mut vis = GaugeField::new("a", "A", "", 1, 10, false);
    vis.update_gauge(0, "0");
    let mut invis = GaugeField::new("b", "B", "", 1, 10, true);
    invis.update_gauge(0, "0");
    invis.base.visible = false;
    let data = data_field("L", "U", "123");
    let fields = [Field::Gauge(&vis), Field::Gauge(&invis), Field::Data(&data)];
    let mut r = LinearGaugeRenderer::default();
    // 1 竖 0 横 (invis 横向不可见被跳过): 156 + 1*30 = 186
    assert_eq!(r.calculate_preferred_size(&fields, &ctx), (192, 186));
    // 2 横: 156 + 3*30 = 246
    let h1 = GaugeField::new("c", "C", "", 1, 10, true);
    let h2 = GaugeField::new("d", "D", "", 1, 10, true);
    let fields2 = [Field::Gauge(&h1), Field::Gauge(&h2)];
    assert_eq!(r.calculate_preferred_size(&fields2, &ctx), (192, 246));
}

/// 组件缓存复用: 第二帧同 key 不新建组件, 值变化走 update 脏检查
#[test]
fn linear_gauge_renderer_cache_reuse() {
    let ctx = ctx_n(3);
    let mut gf = GaugeField::new("rpm", "RPM", "", 1, 100, true);
    gf.update_gauge(50, "50");
    let fields1 = [Field::Gauge(&gf)];
    let mut r = LinearGaugeRenderer::default();
    let mut c = PixCanvas::new(192, 216).unwrap();
    let mut off = [0, 0];
    r.render(&mut c, &fields1, &ctx, &mut off);
    assert_eq!(r.gauges.len(), 1);
    let comp = r.gauges.get("rpm").unwrap();
    assert!(!comp.gauge.is_dirty(), "draw 后清脏");
    assert_eq!(comp.gauge.cur_value, 50);
    // 值变化 → update 置脏, 仍复用同组件
    gf.update_gauge(60, "60");
    let fields2 = [Field::Gauge(&gf)];
    r.render(&mut c, &fields2, &ctx, &mut off);
    assert_eq!(r.gauges.len(), 1, "缓存复用");
    assert_eq!(r.gauges.get("rpm").unwrap().gauge.cur_value, 60, "值同步进组件");
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

// ---- TextOnlyRenderer (TextOnlyRenderer.java:20-67) ----

/// 行布局: 基线 y=fontSize 起, 行高 (int)(fontSize*1.5); offset[0] 不动;
/// 首选尺寸 = (maxLen*fs/2 + fs, n*行高 + fs)
#[test]
fn text_only_layout() {
    let ctx = ctx_n(1);
    let f1 = data_field("L1", "", "12345");
    let f2 = data_field("L2", "", "7"); // set_value 补宽 → "    7" (5 码元)
    let fields = [Field::Data(&f1), Field::Data(&f2)];
    let mut r = TextOnlyRenderer;
    // 两字段均 %5s 后 5 字符: maxW=5*12=60 → (84, 2*36+24=96)
    assert_eq!(r.calculate_preferred_size(&fields, &ctx), (84, 96));
    let mut c = PixCanvas::new(200, 120).unwrap();
    let mut offset = [77, 5];
    r.render(&mut c, &fields, &ctx, &mut offset);
    assert_eq!(offset, [77, 96], "offset[0] 不动; offset[1] = 24 + 2*36");
    let b = c.to_premul_bgra();
    assert!((14..=26).any(|y| row_has_pixels(&b, 200, y)), "第一行 (基线 24)");
    assert!((30..=45).all(|y| !row_has_pixels(&b, 200, y)), "行间空");
    assert!((50..=62).any(|y| row_has_pixels(&b, 200, y)), "第二行 (基线 60)");
    assert!((66..120).all(|y| !row_has_pixels(&b, 200, y)), "第二行之下空");
}

/// 不可见字段全局过滤 (渲染器共用 base.visible 通道)
#[test]
fn field_visibility_filtering() {
    let ctx = ctx_n(1);
    let f1 = data_field("L1", "", "100");
    let mut f2 = data_field("L2", "", "200");
    f2.visible = false;
    let fields = [Field::Data(&f1), Field::Data(&f2)];
    let mut r = TextOnlyRenderer;
    assert_eq!(r.calculate_preferred_size(&fields, &ctx), (84, 60)); // 1 行: 36+24
    let mut c = PixCanvas::new(200, 120).unwrap();
    let mut offset = [0, 0];
    r.render(&mut c, &fields, &ctx, &mut offset);
    assert_eq!(offset[1], 60, "只推进 1 行");
    let b = c.to_premul_bgra();
    assert!((14..=26).any(|y| row_has_pixels(&b, 200, y)), "第一行在");
    assert!((30..120).all(|y| !row_has_pixels(&b, 200, y)), "第二行不可见");
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
