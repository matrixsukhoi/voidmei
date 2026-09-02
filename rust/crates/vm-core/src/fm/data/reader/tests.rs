//! reader.rs 单测 — blkx→json 迁移终态: BlkText 文本原语 (cut/getone/getArray/
//! getlastone) 的 Java 8 oracle 对拍用例已随文本解析链退役删除 (历史断言形态
//! 见 git log; 树寻址语义的锁定在 json/tests.rs)。保留: 真机 JSON 全链路冒烟
//! (getload_from 经 parse_named_json)。PASSPORT 曲线链已删 (2026-09 死代码
//! 清理, Rust 无 DrawFrame 消费), 曲线 oracle 一并退役。

use super::*;

fn fm_root() -> String {
    format!(
        "{}/../../../data/aces/gamedata/flightmodels",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// 真机 spitfire_f24/bf-109e-4 JSON 全链路: 构造入口 + getload 字段装载
/// (data/ 缺失自动跳过, realtests 先例)。
/// 断言值来自迁移期全量位级对拍 (2832/2832) 与旧文本 oracle 的等价值。
#[test]
fn parse_real_fm_files_json() {
    let root = fm_root();
    let phys_path = format!("{root}/fm/spitfire_f24.json");
    if !std::path::Path::new(&phys_path).exists() {
        return; // data/ 未解包 (对齐 build.py 跳过语义)
    }

    // 物理 FM: 顶层标量 + f32 域数值 (旧文本 oracle real_go " 11.3" 等价值)
    let phys = FmData::parse_named_json(&phys_path, "fm/spitfire_f24.blk").unwrap();
    assert!(phys.valid, "物理文件 valid");
    assert_eq!(phys.read_file_name.as_deref(), Some("fm/spitfire_f24.blk"));
    assert_eq!(phys.wingspan, 11.3f32 as f64, "wingspan (f32 域)");
    assert_eq!(phys.emptyweight, 3550.0, "emptyweight");

    // bf-109e-4: 装载面冒烟 (曲线 oracle 随曲线链退役)
    let bf = FmData::parse_named_json(&format!("{root}/fm/bf-109e-4.json"), "fm/bf-109e-4.blk").unwrap();
    assert!(bf.valid);
    assert_eq!(bf.engine_num, 1, "bf-109e-4 单发");
}
