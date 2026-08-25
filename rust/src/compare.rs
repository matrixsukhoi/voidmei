//! PNG 像素比对: 统计差异 + 可选热力图
//! 验收语义: 尺寸/meta 是整数运算, 尺寸不等直接 FAIL; 像素差异"尽力而为+人工审"

use png::{Decoder, Transformations};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

struct Image {
    width: u32,
    height: u32,
    /// RGBA
    data: Vec<u8>,
}

fn load_png(path: &Path) -> Result<Image, String> {
    let file = File::open(path).map_err(|e| format!("打开 {} 失败: {}", path.display(), e))?;
    let mut decoder = Decoder::new(BufReader::new(file));
    decoder.set_transformations(Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG info {} 失败: {}", path.display(), e))?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("PNG 解码 {} 失败: {}", path.display(), e))?;
    let expect = (info.width * info.height * 4) as usize;
    buf.truncate(expect);
    Ok(Image {
        width: info.width,
        height: info.height,
        data: buf,
    })
}

pub struct CompareResult {
    pub max_delta: u32,
    pub mean_delta: f64,
    pub diff_pixel_ratio: f64,
    pub alpha_max_delta: u32,
}

/// 逐像素比对 (RGBA 四通道), mean 在 alpha>0 并集上计算
fn compare(a: &Image, b: &Image) -> CompareResult {
    let mut max_delta = 0u32;
    let mut alpha_max = 0u32;
    let mut sum = 0f64;
    let mut count = 0u64;
    let mut diff_px = 0u64;
    for i in 0..(a.data.len() / 4) {
        let pa = &a.data[i * 4..i * 4 + 4];
        let pb = &b.data[i * 4..i * 4 + 4];
        let in_a = pa[3] > 0;
        let in_b = pb[3] > 0;
        let mut px_max = 0u32;
        for c in 0..4 {
            let d = (pa[c] as i32 - pb[c] as i32).unsigned_abs();
            px_max = px_max.max(d);
            if c == 3 {
                alpha_max = alpha_max.max(d);
            }
        }
        max_delta = max_delta.max(px_max);
        if px_max > 0 {
            diff_px += 1;
        }
        if in_a || in_b {
            sum += px_max as f64;
            count += 1;
        }
    }
    let total = (a.data.len() / 4) as u64;
    CompareResult {
        max_delta,
        mean_delta: if count > 0 { sum / count as f64 } else { 0.0 },
        diff_pixel_ratio: diff_px as f64 / total as f64,
        alpha_max_delta: alpha_max,
    }
}

/// 差异热力图: 黑=相同, 亮=差异大 (R=RGB 差, G=alpha 差)
fn heatmap(a: &Image, b: &Image, out: &Path) -> Result<(), String> {
    let mut buf = vec![0u8; (a.width * a.height * 4) as usize];
    for i in 0..(a.data.len() / 4) {
        let pa = &a.data[i * 4..i * 4 + 4];
        let pb = &b.data[i * 4..i * 4 + 4];
        let mut rgb_d = 0u32;
        for c in 0..3 {
            rgb_d = rgb_d.max((pa[c] as i32 - pb[c] as i32).unsigned_abs());
        }
        let a_d = (pa[3] as i32 - pb[3] as i32).unsigned_abs();
        let idx = i * 4;
        buf[idx] = (rgb_d.min(255)) as u8;
        buf[idx + 1] = (a_d.min(255)) as u8;
        buf[idx + 3] = 255;
    }
    let file = File::create(out).map_err(|e| format!("创建 {}: {}", out.display(), e))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), a.width, a.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| format!("PNG header: {}", e))?;
    w.write_image_data(&buf).map_err(|e| format!("PNG 数据: {}", e))?;
    Ok(())
}

/// analyze 子命令: 打印图像的行带分布与首批非透明像素 (对拍调试用)
pub fn cmd_analyze(args: &[String]) -> i32 {
    let path = match args.iter().position(|a| a == "analyze").and_then(|i| args.get(i + 1)) {
        Some(p) => p.clone(),
        None => {
            eprintln!("用法: analyze <p.png> [行带高度=31]");
            return 2;
        }
    };
    let band_h = args
        .iter()
        .position(|a| a == "analyze")
        .and_then(|i| args.get(i + 2))
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(31);
    let im = match load_png(Path::new(&path)) {
        Ok(im) => im,
        Err(e) => {
            eprintln!("错误: {}", e);
            return 1;
        }
    };
    println!("size: {}x{}", im.width, im.height);
    // 每行带非透明像素数
    let mut bands = Vec::new();
    for by in (0..im.height as i32).step_by(band_h.max(1) as usize) {
        let mut n = 0;
        for y in by..(by + band_h).min(im.height as i32) {
            for x in 0..im.width as i32 {
                if im.data[((y * im.width as i32 + x) * 4 + 3) as usize] > 0 {
                    n += 1;
                }
            }
        }
        bands.push(n);
    }
    println!("bands(opaque px): {:?}", bands);
    // 首批非透明像素坐标
    let mut shown = 0;
    for y in 0..im.height {
        for x in 0..im.width {
            let idx = ((y * im.width + x) * 4) as usize;
            if im.data[idx + 3] > 0 {
                println!("first opaque: ({},{}) rgba={:02x}{:02x}{:02x}{:02x}",
                    x, y, im.data[idx], im.data[idx+1], im.data[idx+2], im.data[idx+3]);
                shown += 1;
                break;
            }
        }
        if shown >= 3 {
            break;
        }
    }
    0
}

/// compare 子命令入口, 返回进程退出码
pub fn cmd_compare(args: &[String]) -> i32 {
    let pos = args.iter().position(|a| a == "compare").unwrap();
    let rest = &args[pos + 1..];
    let a_path = match rest.first() {
        Some(p) => p.clone(),
        None => {
            eprintln!("用法: compare <a.png> <b.png> [--heatmap out.png] [--max-delta N]");
            return 2;
        }
    };
    let b_path = match rest.get(1) {
        Some(p) => p.clone(),
        None => {
            eprintln!("用法: compare <a.png> <b.png> [--heatmap out.png] [--max-delta N]");
            return 2;
        }
    };
    let opts = &rest[2..];
    let heatmap_path = opts
        .iter()
        .position(|a| a == "--heatmap")
        .and_then(|i| opts.get(i + 1)).cloned();
    let max_delta_limit: Option<u32> = opts
        .iter()
        .position(|a| a == "--max-delta")
        .and_then(|i| opts.get(i + 1))
        .and_then(|v| v.parse().ok());

    let a = match load_png(Path::new(&a_path)) {
        Ok(im) => im,
        Err(e) => {
            eprintln!("错误: {}", e);
            return 1;
        }
    };
    let b = match load_png(Path::new(&b_path)) {
        Ok(im) => im,
        Err(e) => {
            eprintln!("错误: {}", e);
            return 1;
        }
    };

    // 尺寸不等: 布局公式是整数运算, 属硬性失败
    if a.width != b.width || a.height != b.height {
        eprintln!(
            "FAIL 尺寸不一致: {}x{} vs {}x{} (布局公式错误)",
            a.width, a.height, b.width, b.height
        );
        return 1;
    }

    let r = compare(&a, &b);
    println!(
        "size: {}x{}\nmax_delta: {}\nmean_delta: {:.4}\ndiff_pixel_ratio: {:.4}%\nalpha_max_delta: {}",
        a.width, a.height, r.max_delta, r.mean_delta, r.diff_pixel_ratio * 100.0, r.alpha_max_delta
    );

    if let Some(hp) = heatmap_path {
        if let Err(e) = heatmap(&a, &b, Path::new(&hp)) {
            eprintln!("错误: {}", e);
            return 1;
        }
        println!("heatmap: {}", hp);
    }

    if let Some(limit) = max_delta_limit {
        if r.max_delta > limit {
            eprintln!("FAIL max_delta {} > 阈值 {}", r.max_delta, limit);
            return 1;
        }
        println!("PASS (max_delta <= {})", limit);
    }
    0
}
