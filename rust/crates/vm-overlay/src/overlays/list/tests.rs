use super::*;

/// WebLabel 是 Font.PLAIN → regular 字重 (renderers.rs unit 字体同源)
const FONT: &str = "../../../fonts/sarasa-mono-sc-regular.ttf";

fn font(size: i32) -> LoadedFont {
    LoadedFont::new(std::path::Path::new(FONT), size).unwrap()
}

fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
    [d[0], d[1], d[2], d[3]]
}

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// 表头不消耗斑马索引: 偶/奇交替跨表头延续 (ZebraListRenderer.java:31-41)
#[test]
fn zebra_row_colors_header_skips_index() {
    let z = ZebraList::new();
    // 默认表头判定 contains (ZebraListRenderer.java:16)
    assert!(z.is_header("------fm器件: 机翼"));
    assert!(z.is_header("FM文件: spitfire"));
    assert!(!z.is_header("速度 600"));
    assert_eq!(
        z.row_background("FM文件: x", 0, 255),
        [80, 60, 0, 255],
        "表头 #503C00"
    );
    assert_eq!(
        z.row_background("a", 0, 255),
        [25, 25, 25, 255],
        "偶行 #191919"
    );
    assert_eq!(
        z.row_background("b", 1, 255),
        [40, 40, 40, 255],
        "奇行 #282828"
    );
    // 表头行出现后斑马索引不自增: 表头前 idx0(偶), 表头(不增), 后续 idx1(奇)
    assert_eq!(
        z.row_background("------fm器件", 1, 255),
        [80, 60, 0, 255],
        "表头行无视斑马索引"
    );
    assert_eq!(
        z.row_background("c", 2, 255),
        [25, 25, 25, 255],
        "idx2 回偶"
    );
    // alpha 透传 (BaseOverlay.alpha, 默认 180)
    assert_eq!(z.row_background("a", 0, 180), [25, 25, 25, 180]);
}

/// 表头谓词可插拔: FMUnpackedDataOverlay.java:87 的 startsWith 覆盖语义
/// (默认 contains 匹不中的 "fm器件x" 前缀式, 自定义谓词可命中)
#[test]
fn header_matcher_override() {
    let mut ov = BaseListOverlay::new(1440, 1.0, 12);
    // contains 默认: "FM文件" 命中
    assert!(ov.zebra.is_header("FM文件: x"));
    // 换 startsWith 谓词 (BaseOverlay.setHeaderMatcher 委托 renderer)
    ov.set_header_matcher(Box::new(|l| {
        l.starts_with("FM文件") || l.starts_with("------fm器件")
    }));
    assert!(ov.zebra.is_header("------fm器件: 机翼"));
    assert!(!ov.zebra.is_header("prefix FM文件")); // startsWith 不命中
}

/// 行高 = 2 + fm.getHeight() + 2; preferred 高 = 行数 × 行高 (vgap=0)
#[test]
fn row_height_and_preferred_height() {
    let f = font(14); // WebLabel 默认 PLAIN 14+add (BaseOverlay.setupFont:182)
    let m = f.metrics().height;
    assert!(m > 0);
    assert_eq!(ZebraList::row_height(&f), m + 4);
    let f16 = font(16);
    assert_eq!(ZebraList::row_height(&f16), f16.metrics().height + 4);
    let ls = lines(&["a", "b", "c"]);
    assert_eq!(ZebraList::preferred_height(&ls, &f), 3 * (m + 4));
    assert_eq!(
        ZebraList::preferred_height(&[], &f),
        0,
        "空列表 preferred 0"
    );
}

/// 像素: 表头/偶/奇满宽条 + 左缩进 6 处白字 + 底色兜底 (alpha=255 免预乘歧义)
#[test]
fn draw_zebra_rows_pixels() {
    let f = font(14);
    let mut z = ZebraList::new();
    let ls = lines(&["FM文件: spitfire", "速度 600", "高度 5000"]);
    let row_h = ZebraList::row_height(&f);
    let w = 120;
    let h = ZebraList::preferred_height(&ls, &f);
    let mut cv = PixCanvas::new(w, h).unwrap();
    z.draw(&mut cv, 0, 0, w, h, &ls, &f, 255, false);

    // 满宽条: 行两端 (含左 margin 区 x=0 与右缘 x=w-1) 均为行底色
    for (y, color, what) in [
        (0, [80, 60, 0, 255], "表头条"),
        (row_h, [25, 25, 25, 255], "偶行条"),
        (2 * row_h, [40, 40, 40, 255], "奇行条"),
    ] {
        let y2 = y + row_h - 1;
        assert_eq!(px(&cv, 0, y), color, "{what} 行顶左缘");
        assert_eq!(px(&cv, w - 1, y), color, "{what} 行顶右缘 (满宽拉伸)");
        assert_eq!(px(&cv, w - 1, y2), color, "{what} 行底右缘");
    }
    // 相邻行色不同 (斑马分界): 行0底 vs 行1顶
    assert_ne!(px(&cv, 0, row_h - 1), px(&cv, 0, row_h));
    // 行底恰为画布底 (preferred 高 = 行数×行高)
    assert_eq!(px(&cv, 5, h - 1), [40, 40, 40, 255], "末行底缘");
    // 白字存在: 表头行文本区 (x≥6, 基线 y=MARGIN_TOP+ascent) 有不透明白像素
    let baseline = MARGIN_TOP + f.metrics().ascent;
    let text_zone = (MARGIN_LEFT..w)
        .map(|x| (x, baseline))
        .find(|&(x, y)| px(&cv, x, y) == TEXT_COLOR);
    assert!(
        text_zone.is_some(),
        "基线上有白色字形像素 (x={:?})",
        text_zone
    );
    // 左 margin 列 x<MARGIN_LEFT 在无字形负 bearing 侵入时为纯底色:
    // x=0..2 距文本起笔 4px+, 全列均为表头色
    for x in 0..MARGIN_LEFT {
        assert_eq!(px(&cv, x, 0), [80, 60, 0, 255], "左边距列 {x}");
    }
}

/// panel 底色兜底 (#141414): 行数不足窗口高时下方露底;
/// 高度 clamp 时超出行被裁剪 (BaseOverlay.java:275-277 + 窗口裁剪)
#[test]
fn draw_panel_bg_and_clipped_rows() {
    let f = font(14);
    let mut z = ZebraList::new();
    let ls = lines(&["a", "b", "c"]);
    let row_h = ZebraList::row_height(&f);
    let w = 100;

    // 兜底: panel_h 比 preferred 高 3px (±2px 容差带 + 余量), 露底为 #141414
    let preferred = ZebraList::preferred_height(&ls, &f);
    let mut cv = PixCanvas::new(w, preferred + 3).unwrap();
    z.draw(&mut cv, 0, 0, w, preferred + 3, &ls, &f, 255, false);
    assert_eq!(
        px(&cv, 0, preferred),
        [20, 20, 20, 255],
        "行下方露 panel 底色"
    );
    assert_eq!(px(&cv, w - 1, preferred + 2), [20, 20, 20, 255]);
    // 空列表 = 纯 panel 底色 (初始无数据窗口)
    let mut cv0 = PixCanvas::new(w, 10).unwrap();
    z.draw(&mut cv0, 0, 0, w, 10, &[], &f, 255, false);
    assert!(cv0
        .pixmap()
        .data()
        .chunks_exact(4)
        .all(|p| p[0] == 20 && p[1] == 20 && p[2] == 20 && p[3] == 255));

    // 裁剪: panel_h 截断末行 (只画前 2 行 + 4px), 截断处之下露底色
    let cut = 2 * row_h + 4;
    let mut cv2 = PixCanvas::new(w, cut).unwrap();
    z.draw(&mut cv2, 0, 0, w, cut, &ls, &f, 255, false);
    assert_eq!(
        px(&cv2, 0, 2 * row_h),
        [25, 25, 25, 255],
        "第 3 行顶部可见 (idx2 偶色)"
    );
    assert_eq!(px(&cv2, 0, cut - 1), [25, 25, 25, 255]);
    // 画布外不可见; panel_h 之外的行整体不画 (此处无第 3 行完整区)
    let mut cv3 = PixCanvas::new(w, 2 * row_h).unwrap();
    z.draw(&mut cv3, 0, 0, w, 2 * row_h, &ls, &f, 255, false);
    assert_eq!(
        px(&cv3, 0, 2 * row_h - 1),
        [40, 40, 40, 255],
        "末行=第 2 行 (奇色)"
    );
}

/// 默认 alpha=180 的三层合成 (Java 8 + WebLaF 基线): 间隙 = panel², 行 =
/// label over panel² — 直通值间隙 0xE9141414 / 表头 0xF93E3005 / 偶 0xF9181818 /
/// 奇 0xF9222222; PixCanvas 预乘存储 = round(直通×a/255) 与 Java 内部预乘逐位一致
#[test]
fn draw_default_alpha_premultiplied() {
    let f = font(14);
    let mut z = ZebraList::new();
    let row_h = ZebraList::row_height(&f);
    let ls = lines(&["FM文件: x", "a", "b"]); // 表头/偶/奇
    let rows_h = 3 * row_h;
    let mut cv = PixCanvas::new(60, rows_h + 5).unwrap();
    z.draw(&mut cv, 0, 0, 60, rows_h + 5, &ls, &f, 180, false);
    // 直通域 基线 复现 (合成模型自检, 与像素断言分层定位错误)
    let panel = rgba(PANEL_BG_RGB, 180);
    let panel2 = java2d_src_over(panel, java2d_src_over(panel, [0, 0, 0, 0]));
    assert_eq!(panel2, [20, 20, 20, 233], "间隙 = panel² = 0xE9141414");
    assert_eq!(
        java2d_src_over(rgba(HEADER_RGB, 180), panel2),
        [62, 48, 5, 249]
    );
    assert_eq!(
        java2d_src_over(rgba(ZEBRA_EVEN_RGB, 180), panel2),
        [24, 24, 24, 249]
    );
    assert_eq!(
        java2d_src_over(rgba(ZEBRA_ODD_RGB, 180), panel2),
        [34, 34, 34, 249]
    );
    // 预乘: 62·249/255=60.5→61, 48·249/255=46.9→47, 5·249/255=4.9→5;
    // 24·249/255=23.4→23; 34·249/255=33.2→33; 20·233/255=18.3→18
    assert_eq!(
        px(&cv, 0, 0),
        [61, 47, 5, 249],
        "表头 = oracle 0xF93E3005 预乘"
    );
    assert_eq!(
        px(&cv, 0, row_h),
        [23, 23, 23, 249],
        "偶行 = oracle 0xF9181818 预乘"
    );
    assert_eq!(
        px(&cv, 0, 2 * row_h),
        [33, 33, 33, 249],
        "奇行 = oracle 0xF9222222 预乘"
    );
    assert_eq!(
        px(&cv, 0, rows_h),
        [18, 18, 18, 233],
        "余量带 = oracle 0xE9141414 预乘"
    );
    assert_eq!(px(&cv, 59, rows_h + 4), [18, 18, 18, 233], "余量带满宽");
}

/// 脏检查生命周期 (BaseOverlay.run:236-241): 首帧必脏 → 同数据不脏 →
/// 变更脏 → null 不脏且保留基准 → 回到旧数据同基准仍不脏
#[test]
fn tick_dirty_check_lifecycle() {
    let f = font(14);
    let mut ov = BaseListOverlay::new(1440, 1.0, 12);
    ov.setup_font(&f);

    assert!(
        ov.tick(|| Some(lines(&["a", "b"]))),
        "首帧 (lastData=null → 必更新)"
    );
    assert_eq!(
        ov.height,
        2 * ZebraList::row_height(&f),
        "高度自适应到 preferred"
    );
    assert!(
        !ov.tick(|| Some(lines(&["a", "b"]))),
        "同数据 equals → 不更新"
    );
    assert!(ov.tick(|| Some(lines(&["a", "c"]))), "内容变化 → 更新");
    assert!(
        !ov.tick(|| None),
        "null 数据 → 不更新 (Java :237 null 检查)"
    );
    assert!(
        !ov.tick(|| Some(lines(&["a", "c"]))),
        "null 未污染基准, 仍与 lastData 同"
    );
    assert!(
        ov.tick(|| Some(lines(&["a"]))),
        "行数变化 → 更新, 高度随之降"
    );
    assert_eq!(ov.height, ZebraList::row_height(&f));
}

/// 门控语义 (BaseOverlay.run:231-249): 隐藏不取数不显示; preview 绕过门控;
/// shouldExit/doit 短路; 重现后同数据不重绘但窗口恢复显示
#[test]
fn tick_visibility_and_exit_gates() {
    let f = font(14);
    let mut ov = BaseListOverlay::new(1440, 1.0, 12);
    ov.setup_font(&f);
    assert!(ov.tick(|| Some(lines(&["x"]))));
    assert!(ov.window_visible, "可见分支置 window_visible");

    // 游戏模式隐藏 (isVisibleNow=false): supplier 不被调用, 窗口隐藏
    ov.visible_now = false;
    let mut called = false;
    assert!(!ov.tick(|| {
        called = true;
        Some(lines(&["y"]))
    }));
    assert!(
        !called,
        "隐藏分支不调 dataSupplier (Java :236 在可见分支内)"
    );
    assert!(!ov.window_visible, "setVisible(false)");

    // 重现: 同数据不重绘, 但窗口恢复显示 (Java :245-247 守卫置 true)
    ov.visible_now = true;
    assert!(!ov.tick(|| Some(lines(&["x"]))), "lastData 未变 → 不重绘");
    assert!(ov.window_visible);

    // preview 模式绕过 isVisibleNow (Java :235 isPreview ||)
    ov.visible_now = false;
    ov.is_preview = true;
    assert!(
        ov.tick(|| Some(lines(&["z"]))),
        "preview 隐藏态仍取数且变更脏"
    );
    assert!(ov.window_visible);
    ov.is_preview = false;

    // shouldExit: 短路且不取数 (Java :232-233 break)
    ov.should_exit = true;
    let mut called2 = false;
    assert!(!ov.tick(|| {
        called2 = true;
        Some(lines(&["w"]))
    }));
    assert!(!called2, "shouldExit 后不再取数");

    // stop(): doit=false 同 while 退出 (BaseOverlay.java:286-288)
    ov.should_exit = false;
    ov.stop();
    assert!(!ov.tick(|| Some(lines(&["v"]))));
    assert!(!ov.doit);
}

/// 高度自适应 (BaseOverlay.adjustPosition:272-284): clamp 到 logicalHeight-40;
/// ±2px 容差不调整, >2px 才 setSize
#[test]
fn height_adaptation_clamp_and_tolerance() {
    let f = font(14);
    let row_h = ZebraList::row_height(&f);
    let mut ov = BaseListOverlay::new(1000, 1.0, 12);
    ov.setup_font(&f);
    assert_eq!(
        ov.height,
        12 * 72,
        "初始 height 字段 = defaultFontsize*72 (:95)"
    );

    // clamp: 行数超逻辑屏高 (1000-40=960)
    let n = (960 / row_h + 10) as usize;
    let many: Vec<String> = (0..n).map(|i| format!("行{i}")).collect();
    let preferred = n as i32 * row_h;
    assert!(preferred > 960, "测试前提: preferred 超上限");
    ov.tick(move || Some(many.clone()));
    assert_eq!(ov.height, 960, "钳制到 logicalHeight-40 (:275-277)");

    // 容差: 差 ≤2px 不调整 (|P+2 - P| = 2)
    let small = lines(&["a", "b"]);
    ov.tick(move || Some(small.clone()));
    let p2 = 2 * row_h;
    assert_eq!(ov.height, p2, "差 >2 → 调整到 preferred");
    ov.height = p2 + 2;
    ov.tick(|| Some(lines(&["a", "c"])));
    assert_eq!(ov.height, p2 + 2, "差 2px ≤ 容差 → 不动 (:279)");
    ov.height = p2 + 3;
    ov.tick(|| Some(lines(&["a", "d"])));
    assert_eq!(ov.height, p2, "差 3px > 容差 → 调整");
}

/// init 几何公式 (BaseOverlay.java:88-95) 与默认字段值
#[test]
fn init_geometry_scaling() {
    // 1440p / 100%: scale=1.0 → fontSize 16, width 432, height 864
    let ov = BaseListOverlay::new(1440, 1.0, 12);
    assert_eq!((ov.font_size, ov.width, ov.height), (16, 432, 864));
    assert_eq!(ov.alpha, 180, "默认 alpha (:33)");
    assert_eq!(ov.refresh_interval_ms, 200, "默认 200ms (:222)");
    assert!(ov.visible_now && !ov.should_exit && ov.doit && !ov.is_preview);
    assert!(ov.last_data.is_none(), "初始 lastData = null (:37)");

    // 1080p / 100%: scale=0.75 → fontSize round(12)=12, width round(324)=324
    let ov = BaseListOverlay::new(1080, 1.0, 12);
    assert_eq!((ov.font_size, ov.width), (12, 324));

    // 1440p / 150% DPI: scale=1.5 → fontSize 24, width round(648)=648
    let ov = BaseListOverlay::new(1440, 1.5, 12);
    assert_eq!((ov.font_size, ov.width), (24, 648));

    // setAlpha
    let mut ov = BaseListOverlay::new(1440, 1.0, 12);
    ov.set_alpha(255);
    assert_eq!(ov.alpha, 255);
}

/// render: 窗口画布上按 lastData 出斑马条 (三层预合成); 无数据时纯 panel² 底色
#[test]
fn render_to_canvas() {
    let f = font(14);
    let mut ov = BaseListOverlay::new(1440, 1.0, 12);
    ov.setup_font(&f);
    ov.width = 80;
    ov.tick(|| Some(lines(&["FM文件: x", "a"])));
    let h = ov.height;
    let mut cv = PixCanvas::new(80, h).unwrap();
    ov.render(&mut cv, &f, false);
    let row_h = ZebraList::row_height(&f);
    // 预合成直铺: 表头 基线 0xF93E3005=(249,62,48,5) → 预乘 61/47/5;
    // 偶行 0xF9181818=(249,24,24,24) → 预乘 23 (render2d 头注预乘语义)
    assert_eq!(
        px(&cv, 0, 0),
        [61, 47, 5, 249],
        "表头条 (三层合成, alpha=249)"
    );
    assert_eq!(px(&cv, 0, row_h), [23, 23, 23, 249], "数据条 = 偶行预乘");

    // 无数据: 只铺 panel² 底色 (初始窗口, 间隙色 0xE9141414)
    let mut ov0 = BaseListOverlay::new(1440, 1.0, 12);
    ov0.width = 40;
    ov0.height = 12;
    let mut cv0 = PixCanvas::new(40, 12).unwrap();
    ov0.render(&mut cv0, &f, false);
    assert!(cv0.pixmap().data().chunks_exact(4).all(|p| p[3] == 233));
}
