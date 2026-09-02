//! json.rs 单测 — 树原语语义逐条锁定 (含真实 FM JSON 结构形态) +
//! preserve_order canary + 燃油修正树版全分支。
//! 结构样例对齐真机 data/ (fm/spitfire_f24.json 实测):
//! 气动标量在顶层, NoFlaps/Fuselage 等极线块嵌套于 Aerodynamics,
//! 活塞引擎块为 Engine0 (喷气为 EngineType0), Compressor 是 Engine0 直下子块。

use super::*;

fn v(s: &str) -> Value {
    serde_json::from_str(s).expect("测试 JSON 非法")
}

// ==================== preserve_order canary ====================

/// serde_json preserve_order feature 被移除时此测试变红 — 键的文档序是
/// getone "首个匹配"/getlastone "末个匹配" 语义的前提 (BTreeMap 会按字母排序)。
#[test]
fn preserve_order_canary() {
    let root = v(r#"{"b": 1, "a": 2}"#);
    let keys: Vec<&str> = root.as_object().unwrap().keys().map(|s| s.as_str()).collect();
    assert_eq!(keys, ["b", "a"]);
}

// ==================== find_section_ci ====================

#[test]
fn section_nested穿透_aerodynamics() {
    // 真实形态: WingPlane 等块嵌套于 Aerodynamics 下 (文本版靠全文搜索穿透)
    let root = v(r#"{"Aerodynamics": {"WingPlane": {"Span": 11.3}}, "Mass": {}}"#);
    let wp = find_section_ci(&root, "WingPlane").expect("应穿透 Aerodynamics 找到");
    assert_eq!(wp["Span"], 11.3);
}

#[test]
fn section_ci_与_后缀语义() {
    // CI + 后缀 ("NAME {" 要求 NAME 后紧跟 " {"): "Plane" 命中 "WingPlane"
    // (后缀), "wing" 不命中 (前缀≠后缀), 全名 CI 命中
    let root = v(r#"{"WingPlane": {"Span": 1}}"#);
    assert!(find_section_ci(&root, "wingplane").is_some(), "全名 CI");
    assert!(find_section_ci(&root, "Plane").is_some(), "后缀命中");
    assert!(find_section_ci(&root, "wing").is_none(), "前缀不命中 (文本 cut 同)");
}

#[test]
fn section_文档序首个匹配() {
    let root = v(r#"{"Outer": {"Compressor": {"a": 1}}, "Compressor": {"b": 2}}"#);
    let c = find_section_ci(&root, "Compressor").unwrap();
    assert_eq!(c["a"], 1, "嵌套在前的块先于本层键命中 (DFS 前序 = 文档序)");
}

#[test]
fn section_标量键不算块() {
    // cut 找 "NAME {" — 值为标量的键 (如 "ThrustMax0": 2000) 不是 section
    let root = v(r#"{"ThrustMax0": 2000.0, "Mass": {"EmptyMass": 1.0}}"#);
    assert!(find_section_ci(&root, "ThrustMax0").is_none());
    assert!(find_section_ci(&root, "Mass").is_some());
}

#[test]
fn section_merge数组取首元素() {
    // 同名 section 重复 → wt_blk merge_fields 折叠为数组; 文本 cut 取首个
    let root = v(r#"{"WingPlaneSweep0": {"Vne": 1}, "WingPlaneSweep1": {"Vne": 2}}"#);
    let s = find_section_ci(&root, "WingPlaneSweep1").unwrap();
    assert_eq!(s["Vne"], 2);

    let merged = v(r#"{"Weapon": [{"a": 1}, {"a": 2}]}"#);
    let w = find_section_ci(&merged, "Weapon").unwrap();
    assert_eq!(w["a"], 1, "merge 数组取首元素 (= 文本首个同名块)");
}

// ==================== find_leaf_cs / find_leaf_ci(_last) ====================

#[test]
fn leaf_cs_大小写敏感_子串命中() {
    // getone 末段: text.find(label) — CS 子串
    let root = v(r#"{"Type": "Inline", "type": "typeFighter"}"#);
    assert_eq!(find_leaf_cs(&root, "Type").unwrap(), "Inline");
    assert_eq!(find_leaf_cs(&root, "type").unwrap(), "typeFighter");
    assert_eq!(find_leaf_cs(&root, "ype").unwrap(), "Inline", "子串语义");
    assert!(find_leaf_cs(&root, "TYPE").is_none(), "CS: 大写查不到");
}

#[test]
fn leaf_跳过_object_值_数组取首() {
    // 块名不是 leaf (文本版块名行无 '=', 扫向块内首个值行); 多分量值
    // (ElevatorsEffectiveSpeed 的 p2 数组): find_leaf_cs 返回**原始数组**,
    // 首行取值由 leaf_to_text 承担 (get_f64 取首 = 文本行首数)
    let root = v(
        r#"{"ElevatorsEffectiveSpeed": [400.0, 400.0],
            "WingPlane": {"Span": 11.3},
            "Vne": 875.0}"#,
    );
    let arr = find_leaf_cs(&root, "ElevatorsEffectiveSpeed").unwrap();
    assert_eq!(arr[0], 400.0, "p2 数组原样返回");
    assert_eq!(leaf_to_text(arr).unwrap(), "400.0, 400.0", "join(', ') ≡ 文本行值");
    assert!(find_leaf_cs(&root, "Span").is_some(), "穿透 object 继续找");
    assert_eq!(find_leaf_cs(&root, "Vne").unwrap(), 875.0);
}

#[test]
fn leaf_ci_last_文档序最后_merge取末() {
    // getlastone: CI rfind — fmFile 键混合大小写 (真机中央文件实测为 "fmFile")
    let central = v(
        r#"{"fmFile": "fm/spitfire_f24.blk",
            "modifications": {"fmFile": ["fm/a.blk", "fm/b.blk"]}}"#,
    );
    assert_eq!(
        get_last_string_ci(&central, "fmfile").unwrap(),
        "fm/b.blk",
        "CI 命中 + 文档序最后 (嵌套块内晚于顶层) + merge 数组取末元素"
    );
}

#[test]
fn leaf_ci_last_无引号干净串() {
    // JSON 字符串值本无引号; 文本链路的剥引号责任在 fm_loader JSON 分支免除
    let root = v(r#"{"fmFile": "fm/x.blk"}"#);
    assert_eq!(get_last_string_ci(&root, "fmfile").unwrap(), "fm/x.blk");
}

// ==================== 数值域 ====================

#[test]
fn num_f32_domain_位级对齐文本_parsefloat() {
    // 核心机制: JSON 十进制 → f64 → (as f32) as f64 ≡ 文本 parseFloat::<f32>() as f64
    let cases = ["1.42", "0.06146", "875.0", "2750.0", "0.1", "3.3"];
    for s in cases {
        let json_val: Value = serde_json::from_str(s).unwrap();
        let via_json = num_f32_domain(&json_val).unwrap();
        let via_text: f64 = s.parse::<f32>().unwrap() as f64;
        assert_eq!(
            via_json.to_bits(),
            via_text.to_bits(),
            "数值 {s} 的 JSON 链路与文本 parseFloat 域不一致"
        );
    }
    // f32 域生效证明: 1.42 经 f32 收窄后 ≠ f64 的 1.42
    let narrowed = num_f32_domain(&v("1.42")).unwrap();
    assert_ne!(narrowed.to_bits(), 1.42f64.to_bits());
}

#[test]
fn value_as_string_形态() {
    // Bool → "true"/"false" (BlkText `key:b = true` 行值形态, ExactAltitudes == "true" 比较)
    assert_eq!(value_as_string(&v("true")).unwrap(), "true");
    assert_eq!(value_as_string(&v("false")).unwrap(), "false");
    assert_eq!(value_as_string(&v("\"Inline\"")).unwrap(), "Inline");
    assert!(value_as_string(&v("{}")).is_none());
}

// ==================== extract_fuel_modifications_json ====================
// 镜像 types.rs 的 extract_fuel_modifications 测试分支; 结构对齐真机中央文件
// (modifications.150_octan_fuel.{invertEnableLogic, effects.{afterburnerMult,...}})。

fn mods_fuel(s: &str) -> Value {
    v(&format!(r#"{{"modifications": {s}}}"#))
}

#[test]
fn fuel_无modifications_默认() {
    let fm = extract_fuel_modifications_json(&v(r#"{"model": "x"}"#));
    assert_eq!(fm.r#type, FuelType::None);
    assert_eq!(fm.soviet_octane_hp_bonus, 0.0);
}

#[test]
fn fuel_苏联b100() {
    let fm = extract_fuel_modifications_json(&mods_fuel(
        r#"{"ussr_fuel_b-100": {"effects": {"addHorsePowers": 50.0}}}"#,
    ));
    assert_eq!(fm.r#type, FuelType::SovietB100);
    assert_eq!(fm.soviet_octane_hp_bonus, 50.0);
}

#[test]
fn fuel_苏联b95() {
    let fm = extract_fuel_modifications_json(&mods_fuel(
        r#"{"ussr_fuel_b-95": {"effects": {"addHorsePowers": 30.0}}}"#,
    ));
    assert_eq!(fm.r#type, FuelType::SovietB95);
    assert_eq!(fm.soviet_octane_hp_bonus, 30.0);
}

#[test]
fn fuel_英国150辛烷_真机值() {
    // spitfire_f24 中央文件实测值
    let fm = extract_fuel_modifications_json(&mods_fuel(
        r#"{"150_octan_fuel": {"invertEnableLogic": false,
              "effects": {"afterburnerMult": 1.42, "afterburnerCompressorMult": 1.33}}}"#,
    ));
    assert_eq!(fm.r#type, FuelType::British150Octane);
    assert_eq!(fm.british_afterburner_mult, 1.42);
    assert_eq!(fm.british_afterburner_compressor_mult, 1.33);
    assert!(!fm.british_invert_logic);
}

#[test]
fn fuel_英国100喷火_0值回退1() {
    let fm = extract_fuel_modifications_json(&mods_fuel(
        r#"{"100_octan_spitfire": {"invertEnableLogic": true,
              "effects": {"afterburnerMult": 0.0, "afterburnerCompressorMult": 0.0}}}"#,
    ));
    assert_eq!(fm.r#type, FuelType::British100Spitfire);
    assert_eq!(fm.british_afterburner_mult, 1.0, "0 值回退 1.0");
    assert_eq!(fm.british_afterburner_compressor_mult, 1.0);
    assert!(fm.british_invert_logic);
}

#[test]
fn fuel_苏联b100优先于b95() {
    // 互斥 return 顺序保真: b-100 在前
    let fm = extract_fuel_modifications_json(&mods_fuel(
        r#"{"ussr_fuel_b-95": {"effects": {"addHorsePowers": 30.0}},
            "ussr_fuel_b-100": {"effects": {"addHorsePowers": 50.0}}}"#,
    ));
    assert_eq!(fm.r#type, FuelType::SovietB100);
}

#[test]
fn fuel_effects缺失_数值保持默认() {
    let fm = extract_fuel_modifications_json(&mods_fuel(
        r#"{"ussr_fuel_b-100": {}}"#,
    ));
    assert_eq!(fm.r#type, FuelType::SovietB100);
    assert_eq!(fm.soviet_octane_hp_bonus, 0.0);
}

#[test]
fn fuel_嵌套穿透_section_ci_leaf_cs() {
    // modifications 不在根层 (嵌套于任意包装块下也能找到, 对齐文本全文搜索);
    // section 键名允许大小写变体 (cut_static 是 CI), 但 leaf 键名必须逐字符命中
    // (getDoubleFromBlock 是 CS) — 真机键名为小写驼峰 addHorsePowers (yak-3 实测)
    let root = v(
        r#"{"wrap": {"Modifications": {"ussr_fuel_b-100":
            {"Effects": {"addHorsePowers": 50.0}}}}}"#,
    );
    let fm = extract_fuel_modifications_json(&root);
    assert_eq!(fm.r#type, FuelType::SovietB100);
    assert_eq!(fm.soviet_octane_hp_bonus, 50.0);

    // leaf 键名大小写不匹配 → 两侧 (文本/JSON) 一致取 0
    let bad = v(
        r#"{"modifications": {"ussr_fuel_b-100":
            {"effects": {"AddHorsePowers": 50.0}}}}"#,
    );
    let fm2 = extract_fuel_modifications_json(&bad);
    assert_eq!(fm2.soviet_octane_hp_bonus, 0.0, "CS 语义: 键名不匹配时与文本版一致取默认");
}


// ==================== parity 实测裁决的语义锁定 ====================

#[test]
fn leaf_冒号label后缀匹配() {
    // "Vne:" 语义 = 键名以 Vne 结尾 (冒号紧跟键名): VneControl 不可冒充 Vne
    // (parity 实测: a-10a 的 vne 曾被子串匹配误取 VneControl=1000 而非回退 874)
    let root = v(r#"{"VneControl": 1000.0, "Mass": {"EmptyMass": 1.0}, "Vne": [874.0, 900.0]}"#);
    use super::{shape_leaf_label, KeyMatch};
    let (last, root_only, mode) = shape_leaf_label("Vne:");
    assert_eq!((last, root_only, mode), ("Vne", false, KeyMatch::Suffix));
    // Suffix 模式下 VneControl 不含 (以 Control 结尾)
    let node = &root;
    let hit = super::find_leaf_mode(node, last, mode, false).unwrap();
    assert_eq!(hit[0], 874.0, "Suffix 命中真 Vne 键");
}

#[test]
fn section_后缀不误吞_fuselageplane() {
    // cut("Fuselage") 不命中 "FuselagePlane" (parity 实测: a-10a 回退链曾被短路)
    let root = v(r#"{"Aerodynamics": {"FuselagePlane": {"Polar": {"CdMin": 0.02}}}}"#);
    assert!(find_section_ci(&root, "Fuselage").is_none(), "后缀语义: FuselagePlane 不以 Fuselage 结尾");
    assert!(find_section_ci(&root, "FuselagePlane").is_some());
}

#[test]
fn leaf_键名命中section_取块内首leaf() {
    // 引擎计数 getone("Engine1") 命中块名行 → 文本跨行扫到块内首个 '=' 行;
    // 树版: 键名命中 section 时返回其 DFS 首个 leaf (parity 实测 a-10a 双发)
    let root = v(
        r#"{"Engine1": {"Position": [1.0, 2.0, 3.0], "Mass": 50.0},
            "Engine2": {"Mass": 60.0}}"#,
    );
    let hit = find_leaf_cs(&root, "Engine1").expect("键名命中块应返回块内首 leaf");
    assert_eq!(hit[0], 1.0, "Position 是 Engine1 块内首个 leaf");
}

#[test]
fn leaf_to_text_merge三形态() {
    // Number 数组 = p2 值行 join; 嵌套数组 = merge 曲线取首行; 字符串数组 = merge 行取首
    assert_eq!(leaf_to_text(&v("[400.0, 400.0]")).unwrap(), "400.0, 400.0");
    assert_eq!(
        leaf_to_text(&v("[[0.0, 0.0], [1000.0, 137.4]]")).unwrap(),
        "0.0, 0.0",
        "merge 曲线取首行"
    );
    assert_eq!(leaf_to_text(&v(r#"["fm/a.blk", "fm/b.blk"]"#)).unwrap(), "fm/a.blk");
    assert_eq!(leaf_to_text(&v("[false, false]")).unwrap(), "false", "同名 bool merge 取首行");
}
