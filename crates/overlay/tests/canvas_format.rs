//! canvas 域像素格式黑盒场景: straight_bgra 直通 BGRA 帧契约。
//! 非渲染质量断言 — 只钉像素缓冲的格式语义 (字节序/alpha 域, 直通 vs 预乘),
//! 整数矩形 fill_rect 无 AA 无取整系统差, 可逐字节断言。
#![allow(non_snake_case)] // 中文场景命名是项目惯例

use expect_test::expect;
use overlay::render::canvas::PixCanvas;

/// 4x2 画布: (0,0) 2x1 不透明色块 | (2,0) 2x1 半透明色块 | 第二行保持初始透明。
/// fill_rect 颜色 = 直通 RGBA + SourceOver: 透明底上半透明色块的 alpha 保留原值,
/// 预乘存储 RGB 已乘 alpha — 两域恰在此分道, 是本组场景的观察点。
fn sample_canvas() -> PixCanvas {
    let mut c = PixCanvas::new(4, 2).expect("4x2 画布构造");
    c.fill_rect(0, 0, 2, 1, [200, 100, 50, 255]);
    c.fill_rect(2, 0, 2, 1, [10, 20, 30, 128]);
    c
}

/// BGRA 缓冲整帧逐像素 "x,y: B,G,R,A" (按字节序原样打印, 十进制)
fn dump_bgra(buf: &[u8], w: i32) -> String {
    buf.chunks_exact(4)
        .enumerate()
        .map(|(i, p)| {
            let (x, y) = ((i as i32) % w, (i as i32) / w);
            format!("{x},{y}: {},{},{},{}", p[0], p[1], p[2], p[3])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 直通 BGRA 帧: R/B 换位 + 颜色非预乘 + alpha 保留原值 (含初始透明 alpha=0 区)
#[test]
fn canvas_straight_bgra_字节序与直通alpha语义() {
    let mut c = sample_canvas();
    expect![[r#"
        0,0: 50,100,200,255
        1,0: 50,100,200,255
        2,0: 30,20,10,128
        3,0: 30,20,10,128
        0,1: 0,0,0,0
        1,1: 0,0,0,0
        2,1: 0,0,0,0
        3,1: 0,0,0,0"#]]
    .assert_eq(&dump_bgra(&c.straight_bgra(), 4));
}

/// 同帧两方法对照: 半透明像素处直通保持原色、预乘已乘 alpha; 不透明与透明区两域一致
#[test]
fn canvas_straight_bgra_与预乘bgra对照() {
    let mut c = sample_canvas();
    let straight = c.straight_bgra();
    let premul = c.to_premul_bgra();
    let actual = straight
        .chunks_exact(4)
        .zip(premul.chunks_exact(4))
        .enumerate()
        .map(|(i, (s, p))| {
            let (x, y) = ((i as i32) % 4, (i as i32) / 4);
            format!(
                "{x},{y}: straight={},{},{},{} premul={},{},{},{}",
                s[0], s[1], s[2], s[3], p[0], p[1], p[2], p[3]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        0,0: straight=50,100,200,255 premul=50,100,200,255
        1,0: straight=50,100,200,255 premul=50,100,200,255
        2,0: straight=30,20,10,128 premul=15,10,5,128
        3,0: straight=30,20,10,128 premul=15,10,5,128
        0,1: straight=0,0,0,0 premul=0,0,0,0
        1,1: straight=0,0,0,0 premul=0,0,0,0
        2,1: straight=0,0,0,0 premul=0,0,0,0
        3,1: straight=0,0,0,0 premul=0,0,0,0"#]]
    .assert_eq(&actual);
}
