//! VoidMei FlightInfoOverlay Rust 复现 (POC)
//! CLI: (无参)=live 游戏模式 / --preview 预览可拖拽 / --render-png 离屏导出 / compare 比对

use vm_core::{fields, format, layout};

use vm_core::layout::RenderCtx;
use vm_overlay::render::{FieldText, FontTriple, DEFAULT_COLORS};

const USAGE: &str = "\
voidmei-overlay — VoidMei FlightInfoOverlay 的 Rust 复现

用法:
  voidmei-overlay                       live 模式: 轮询 8111, 穿透+置顶
  voidmei-overlay --preview             预览模式: preview-value 静态渲染, 可拖拽
  voidmei-overlay --render-png <p>      离屏渲染导出 PNG (--meta 可同时导出度量)
  voidmei-overlay --gauge <name> <p>    gauge 对拍基线导出 (linear|compass|attitude)
  voidmei-overlay --minihud --out <p>   MiniHUD 整帧对拍导出 (默认配置, preview 数据)
  voidmei-overlay compare <a.png> <b.png>  像素比对
选项 (render-png):
  --font-add N      字号增量 (默认 0)
  --column N        列数 (默认 1)
  --num-height N    覆盖 numHeight (默认按字体度量计算)
  --aa on|off       抗锯齿 (默认 on, 对齐当前 AAEnable)
  --fonts <dir>     字体目录 (默认 自动探测 ./fonts 或 ../fonts)
  --meta <p.json>   同时导出布局度量 JSON
选项 (gauge):
  --data <f.txt>    数值参数注入 (每行 key=value, 缺省走共享默认数据)
  --aa on|off       抗锯齿 (默认 on)
  --fonts <dir>     字体目录 (同上)
选项 (minihud):
  --aa on|off       抗锯齿 (默认 on)
  --fonts <dir>     字体目录 (同上)
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("live 模式尚未实现 (M2/M3 里程碑)");
        std::process::exit(2);
    }
    match args[0].as_str() {
        "--preview" => {
            if let Err(e) = cmd_run_window(vm_overlay::OverlayMode::Preview) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        "--live" => {
            if let Err(e) = cmd_run_window(vm_overlay::OverlayMode::Live) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        "--log-values" => {
            if let Err(e) = cmd_log_values() {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        "--render-png" => {
            let out = args.get(1).cloned().unwrap_or_default();
            if out.is_empty() {
                eprintln!("错误: 缺少 --render-png <路径>");
                std::process::exit(1);
            }
            if let Err(e) = cmd_render_png(&out, &args[2..]) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        "--gauge" => {
            let name = args.get(1).cloned().unwrap_or_default();
            if name.is_empty() {
                eprintln!("错误: 缺少 --gauge <linear|compass|attitude>");
                std::process::exit(1);
            }
            if let Err(e) = cmd_render_gauge(&name, &args[2..]) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        "--minihud" => {
            if let Err(e) = cmd_render_minihud(&args[1..]) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        "compare" => {
            std::process::exit(vm_overlay::compare::cmd_compare(&args));
        }
        "analyze" => {
            std::process::exit(vm_overlay::compare::cmd_analyze(&args));
        }
        "--help" | "-h" => print!("{}", USAGE),
        other => {
            eprintln!("未知参数: {}\n\n{}", other, USAGE);
            std::process::exit(2);
        }
    }
}

/// 从参数表取 --key N (返回 None 表示未提供)
fn opt_num(args: &[String], key: &str) -> Option<i64> {
    args.iter().position(|a| a == key).and_then(|i| {
        args.get(i + 1).and_then(|v| v.parse().ok())
    })
}

/// 浮点版
fn opt_f64(args: &[String], key: &str) -> Option<f64> {
    args.iter().position(|a| a == key).and_then(|i| {
        args.get(i + 1).and_then(|v| v.parse().ok())
    })
}

fn opt_str<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1).map(|s| s.as_str()))
}

/// 字体目录探测: ./fonts → ../fonts (repo 根或 rust/ 下运行均可)
fn find_fonts_dir(explicit: Option<&str>) -> std::path::PathBuf {
    if let Some(d) = explicit {
        return std::path::PathBuf::from(d);
    }
    for cand in ["./fonts", "../fonts"] {
        if std::path::Path::new(cand).is_dir() {
            return std::path::PathBuf::from(cand);
        }
    }
    std::path::PathBuf::from("./fonts")
}

fn cmd_render_png(out: &str, args: &[String]) -> Result<(), String> {
    // 单字符实验模式: 指定字符画到 48x48 (基线 24, pen x=6), 用于光栅化差异分析
    if let Some(ch) = opt_str(args, "--single") {
        let fonts_dir = find_fonts_dir(opt_str(args, "--fonts"));
        let aa = opt_str(args, "--aa").unwrap_or("on") != "off";
        let ch = ch.chars().next().ok_or("--single 需要至少一个字符")?;
        let font = vm_overlay::font::LoadedFont::new(&fonts_dir.join("sarasa-mono-sc-bold.ttf"), 24)?;
        if let Some(g) = opt_f64(args, "--gamma") {
            *font.gamma.borrow_mut() = g as f32;
        }
        let mut canvas = vm_overlay::font::Canvas::new(48, 48);
        canvas.draw_text(&font, 6, 24, &ch.to_string(), [255, 255, 255, 255], aa);
        vm_overlay::render::save_png(&canvas, std::path::Path::new(out))?;
        return Ok(());
    }
    let meta_path = opt_str(args, "--meta").map(|s| s.to_string());
    let font_add = opt_num(args, "--font-add").unwrap_or(0) as i32;
    let column = opt_num(args, "--column").unwrap_or(1).max(1) as i32;
    let num_height_override = opt_num(args, "--num-height").map(|v| v as i32);
    let aa = match opt_str(args, "--aa").unwrap_or("on") {
        "on" => true,
        "off" => false,
        other => return Err(format!("--aa 仅支持 on|off: {}", other)),
    };
    let fonts_dir = find_fonts_dir(opt_str(args, "--fonts"));

    // numHeight: Java FontMetrics 对 Sarasa 的取整策略无法从 ttf 表推算,
    // 默认 font_add=0 用实测校准值 31; 其他字号由对拍脚本从 java meta 注入 --num-height
    let num_height = num_height_override.unwrap_or_else(|| default_num_height(font_add));

    let ctx = RenderCtx::new(font_add, column, num_height);
    let fonts = FontTriple::load(&fonts_dir, &ctx)?;
    // 三份字体的实际度量 (meta 导出用)
    let num_m = fonts.num.metrics();
    let label_m = fonts.label.metrics();
    let unit_m = fonts.unit.metrics();

    // preview 模式: 16 字段全部可见, 值为原样 preview-value 字符串;
    // --values 模式: 动态数据走 format + visible-when/na-when 求值
    let mut owned_values: Vec<String> = Vec::new();
    let texts: Vec<FieldText> = if let Some(vpath) = opt_str(args, "--values") {
        let values = parse_values_file(vpath)?;
        // 先记 (label, unit, 值索引), 循环后统一借阅避免借用冲突
        let mut idxs: Vec<(&'static str, &'static str, usize)> = Vec::new();
        for f in fields::FIELDS {
            let raw = match values.get(f.source.getter()) {
                Some(v) => *v,
                None => continue, // 未提供的字段不显示 (与 Java 一致)
            };
            // visible-when
            if let Some(cond) = f.visible_when {
                if !cond.eval(raw) {
                    continue;
                }
            }
            // 可变翼显示 ×100
            let v = if f.source == fields::FieldSource::WingSweepMul100 {
                raw * 100.0
            } else {
                raw
            };
            let text = match f.na_when {
                Some(cond) if cond.eval(v) => "-".to_string(),
                _ => format::format(v, f.precision),
            };
            owned_values.push(text);
            idxs.push((f.label, f.unit, owned_values.len() - 1));
        }
        idxs.into_iter()
            .map(|(label, unit, i)| FieldText {
                label,
                unit,
                value: &owned_values[i],
            })
            .collect()
    } else {
        fields::FIELDS
            .iter()
            .map(|f| FieldText {
                label: f.label,
                unit: f.unit,
                value: f.preview_text(),
            })
            .collect()
    };

    let visible = texts.len() as i32;
    let canvas = vm_overlay::render::render_fields(&texts, &ctx, &fonts, &DEFAULT_COLORS, aa);
    vm_overlay::render::save_png(&canvas, std::path::Path::new(&out))?;

    if let Some(mp) = meta_path {
        let meta = format_meta(&ctx, &num_m, &label_m, &unit_m, visible, aa);
        std::fs::write(&mp, meta).map_err(|e| format!("写 meta 失败: {}", e))?;
    }
    Ok(())
}

/// --gauge: gauge 对拍基线导出 (与 Java OverlayPngExport --gauge 逐像素对拍,
/// 常量表同源见 vm_overlay::parity_gauges 模块头)
fn cmd_render_gauge(name: &str, args: &[String]) -> Result<(), String> {
    let out = opt_str(args, "--out")
        .ok_or("缺少 --out <路径>")?
        .to_string();
    let aa = match opt_str(args, "--aa").unwrap_or("on") {
        "on" => true,
        "off" => false,
        other => return Err(format!("--aa 仅支持 on|off: {}", other)),
    };
    let fonts_dir = find_fonts_dir(opt_str(args, "--fonts"));
    let data = match opt_str(args, "--data") {
        Some(p) => vm_overlay::parity_gauges::GaugeData::parse_file(p)?,
        None => vm_overlay::parity_gauges::GaugeData::default(),
    };
    let cv = vm_overlay::parity_gauges::render_gauge(name, &data, &fonts_dir, aa)?;
    cv.save_png(std::path::Path::new(&out))?;
    println!("gauge {} -> {} ({}x{})", name, out, cv.width(), cv.height());
    Ok(())
}

/// numHeight 默认值 — 已平移 lib (flight_info.rs, 组装面共用), bin 复用
use vm_overlay::flight_info::{build_texts_from_values, default_num_height, flight_value};

/// --minihud: MiniHUD 整帧对拍导出 (与 Java OverlayPngExport --minihud 逐像素对拍,
/// 组装口径见 vm_overlay::parity_minihud 模块头)
fn cmd_render_minihud(args: &[String]) -> Result<(), String> {
    let out = opt_str(args, "--out")
        .ok_or("缺少 --out <路径>")?
        .to_string();
    let aa = match opt_str(args, "--aa").unwrap_or("on") {
        "on" => true,
        "off" => false,
        other => return Err(format!("--aa 仅支持 on|off: {}", other)),
    };
    let fonts_dir = find_fonts_dir(opt_str(args, "--fonts"));
    let cv = vm_overlay::parity_minihud::render_minihud(&fonts_dir, aa)?;
    cv.save_png(std::path::Path::new(&out))?;
    println!("minihud -> {} ({}x{})", out, cv.width(), cv.height());
    Ok(())
}

/// FlightValues → 16 getter 的映射 (fields.rs 的 source 对应)
// flight_value — 已平移 lib (flight_info.rs), bin 经 use 复用
/// --log-values: 从 8111 取一帧数据, 以 values.txt 格式输出 (回灌 Java --values 对拍用)
fn cmd_log_values() -> Result<(), String> {
    let timeout = std::time::Duration::from_millis(2000);
    let state_raw = vm_data::data::http::http_get(8111, "/state", timeout)?;
    let indic_raw = vm_data::data::http::http_get(8111, "/indicators", timeout)?;
    let st = vm_data::data::json::parse_state(&state_raw).ok_or("/state 解析失败")?;
    let ind = vm_data::data::json::parse_indicators(&indic_raw).ok_or("/indicators 解析失败")?;
    if !ind.valid {
        return Err("indicators.valid = false (游戏未在飞行中)".into());
    }
    let mut deriver = vm_data::Deriver::new(50);
    // 预热 SMA (模拟应用已运行状态, 单轮瞬态值无对拍意义)
    let mut v = deriver.step(&st, &ind, 50.0);
    for _ in 0..99 {
        v = deriver.step(&st, &ind, 50.0);
    }
    let mut out = String::from("# voidmei-overlay live frame (getter=value)\n");
    for f in fields::FIELDS {
        if let Some(val) = flight_value(&v, f.source.getter()) {
            out.push_str(&format!("{}={:.6}\n", f.source.getter(), val));
        }
    }
    print!("{}", out);
    Ok(())
}

/// 窗口模式 (preview 静态 / live 实时轮询)
fn cmd_run_window(mode: vm_overlay::OverlayMode) -> Result<(), String> {
    let fonts_dir = find_fonts_dir(None);
    let ctx = layout::RenderCtx::new(0, 1, default_num_height(0));
    let fonts = vm_overlay::render::FontTriple::load(&fonts_dir, &ctx)?;
    match mode {
        vm_overlay::OverlayMode::Preview => {
            let texts: Vec<vm_overlay::render::FieldText<'static>> = fields::FIELDS
                .iter()
                .map(|f| vm_overlay::render::FieldText {
                    label: f.label,
                    unit: f.unit,
                    value: f.preview_text(),
                })
                .collect();
            vm_overlay::window::run(mode, ctx, fonts, texts, &vm_overlay::render::DEFAULT_COLORS, true)
        }
        vm_overlay::OverlayMode::Live => {
            let snapshot = vm_data::data::start_polling(8111);
            vm_overlay::window::run_live(ctx, fonts, &vm_overlay::render::DEFAULT_COLORS, true, snapshot, build_texts_from_values)
        }
    }
}

// FlightValues → owned 元组 — 已平移 lib (flight_info.rs), bin 经上方 use 复用

/// values 文件: 每行 "getter名=数值", # 注释
fn parse_values_file(path: &str) -> Result<std::collections::HashMap<String, f64>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 {} 失败: {}", path, e))?;
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let k = line[..eq].trim().to_string();
            let v: f64 = line[eq + 1..]
                .trim()
                .parse()
                .map_err(|_| format!("数值解析失败: {}", line))?;
            map.insert(k, v);
        }
    }
    Ok(map)
}

fn format_meta(
    ctx: &RenderCtx,
    num_m: &vm_overlay::font::FontMetricsCal,
    label_m: &vm_overlay::font::FontMetricsCal,
    unit_m: &vm_overlay::font::FontMetricsCal,
    visible: i32,
    aa: bool,
) -> String {
    let c = &DEFAULT_COLORS;
    let hx = |v: &[u8; 4]| format!("#{:02X}{:02X}{:02X}{:02X}", v[0], v[1], v[2], v[3]);
    format!(
        "{{\n\
  \"font_size\": {},\n\
  \"label_font_size\": {},\n\
  \"unit_font_size\": {},\n\
  \"column_num\": {},\n\
  \"num_height\": {},\n\
  \"total_width\": {},\n\
  \"total_height\": {},\n\
  \"visible_fields\": {},\n\
  \"aa\": {},\n\
  \"num_metrics\": {{\"ascent\": {}, \"descent\": {}, \"leading\": {}, \"height\": {}}},\n\
  \"label_metrics\": {{\"ascent\": {}, \"descent\": {}, \"leading\": {}, \"height\": {}}},\n\
  \"unit_metrics\": {{\"ascent\": {}, \"descent\": {}, \"leading\": {}, \"height\": {}}},\n\
  \"colors\": {{\"num\": \"{}\", \"label\": \"{}\", \"unit\": \"{}\", \"shade\": \"{}\"}}\n\
}}\n",
        ctx.font_size, ctx.label_font_size, ctx.unit_font_size, ctx.column_num,
        ctx.num_height, ctx.total_width(), ctx.total_height(visible), visible, aa,
        num_m.ascent, num_m.descent, num_m.leading, num_m.height,
        label_m.ascent, label_m.descent, label_m.leading, label_m.height,
        unit_m.ascent, unit_m.descent, unit_m.leading, unit_m.height,
        hx(&c.num), hx(&c.label), hx(&c.unit), hx(&c.shade),
    )
}
