use super::*;

/// 调试: 打印字形 placement 与 ASCII 渲染, 排查基线定位
#[test]
fn glyph_placement_debug() {
    let f = LoadedFont::new(std::path::Path::new("../../../fonts/sarasa-mono-sc-bold.ttf"), 24).unwrap();
    let m = f.metrics();
    println!("metrics @24px: {:?}", m);
    for ch in ['5', '表'] {
        let g = f.glyph(ch, true).unwrap();
        println!("char {:?}: left={} top={} w={} h={}", ch, g.x, g.y, g.w, g.h);
    }
    // ASCII art: '5' 画在基线=24 的 48x48 画布 (draw_text 走修正后的 blit)
    let mut c = Canvas::new(48, 48);
    c.draw_text(&f, 0, 24, "5", [255, 255, 255, 255], true);
    for y in 0..48 {
        let mut row = String::new();
        for x in 0..48 {
            let a = c.buf[(y * 48 + x) * 4 + 3];
            row.push(if a > 200 { '#' } else if a > 80 { '+' } else if a > 10 { '.' } else { ' ' });
        }
        println!("{:02}|{}", y, row);
    }
}
