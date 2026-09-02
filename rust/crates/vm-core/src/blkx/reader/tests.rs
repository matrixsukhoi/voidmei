//! reader.rs 单测 — blkx→json 迁移终态: BlkText 文本原语 (cut/getone/getArray/
//! getlastone) 的 Java 8 oracle 对拍用例已随文本解析链退役删除 (历史断言形态
//! 见 git log; 树寻址语义的锁定在 json/tests.rs)。保留: java_trim 语义 +
//! 真机 JSON 全链路冒烟 (getload_from 经 parse_named_json)。

use super::*;

fn fm_root() -> String {
    format!(
        "{}/../../../data/aces/gamedata/flightmodels",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// 真机 spitfire_f24/bf-109e-4 JSON 全链路: 构造入口 + getload 字段装载 +
/// PASSPORT 曲线 (data/ 缺失自动跳过, realtests 先例)。
/// 断言值来自迁移期全量位级对拍 (2832/2832) 与旧文本 oracle 的等价值。
#[test]
fn parse_real_fm_files_json() {
    let root = fm_root();
    let phys_path = format!("{root}/fm/spitfire_f24.json");
    if !std::path::Path::new(&phys_path).exists() {
        return; // data/ 未解包 (对齐 build.py 跳过语义)
    }

    // 物理 FM: 顶层标量 + f32 域数值 (旧文本 oracle real_go " 11.3" 等价值)
    let phys = Blkx::parse_named_json(&phys_path, "fm/spitfire_f24.blk").unwrap();
    assert!(phys.valid, "物理文件 valid");
    assert_eq!(phys.read_file_name.as_deref(), Some("fm/spitfire_f24.blk"));
    assert_eq!(phys.wingspan, 11.3f32 as f64, "wingspan (f32 域)");
    assert_eq!(phys.emptyweight, 3550.0, "emptyweight");
    // PASSPORT 曲线缺席 → 空表 (旧 real_ga_empty 等价)
    assert_eq!(phys.loc.as_ref().unwrap().cur, 0, "spitfire 无 WEP 爬升曲线");

    // bf-109e-4: PASSPORT 曲线 (cut×2 + 多行累积的树版, 旧 real_ga_bf 等价)
    let bf = Blkx::parse_named_json(&format!("{root}/fm/bf-109e-4.json"), "fm/bf-109e-4.blk").unwrap();
    assert!(bf.valid);
    let loc = bf.loc.as_ref().unwrap();
    assert_eq!(loc.cur, 3, "minClimbTimeWep 三点");
    assert_eq!(loc.y, vec![0.0, 1000.0, 2000.0], "y = 高度轴");
    assert_eq!(loc.x, vec![0.0, 137.4, 271.4], "x = 时间轴");
    assert_eq!(bf.loc2.as_ref().unwrap().cur, 7, "maxSpeedNom 七点 (real_ga_bf2)");
}

/// java_trim 与 Java String.trim 同语义 (<= U+0020, 不含 NBSP)
#[test]
fn java_trim_matches_java_semantics() {
    assert_eq!(java_trim("  a\n"), "a");
    assert_eq!(java_trim("\u{00A0}x"), "\u{00A0}x", "NBSP 不剥 (Rust trim 会)");
    assert_eq!(java_trim("\t x \r\n"), "x");
}
