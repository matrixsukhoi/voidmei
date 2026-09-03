use super::*;

/// Sarasa 字体度量快照: 校准 numHeight 的依据 (Java FontMetrics 实测 24/6/1 @24px)
#[test]
fn sarasa_metrics_snapshot() {
    let f = LoadedFont::new(
        std::path::Path::new("../../../fonts/sarasa-mono-sc-bold.ttf"),
        24,
    )
    .unwrap();
    let m = f.metrics();
    println!("rust metrics @24px: {:?}", m);
    // dump 原始表值用于校准分析
    let data = std::fs::read("../../../fonts/sarasa-mono-sc-bold.ttf").unwrap();
    let face = Face::parse(&data, 0).unwrap();
    println!("upem={}", face.units_per_em());
    println!(
        "hhea: ascender={} descender={} line_gap={}",
        face.ascender(),
        face.descender(),
        face.line_gap()
    );
    if let Some(os2) = face.tables().os2 {
        println!(
            "os2 typo: asc={} desc={} linegap={}",
            os2.typographic_ascender(),
            os2.typographic_descender(),
            os2.typographic_line_gap()
        );
        println!(
            "os2 win:  asc={} desc={}",
            os2.windows_ascender(),
            os2.windows_descender()
        );
    }
    assert!(m.height > 0);
}
