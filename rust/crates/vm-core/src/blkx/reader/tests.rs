// PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
// 不改成 struct 字面量以保持与 Java 测试源逐行对应
#![allow(clippy::field_reassign_with_default)]

use super::*;

fn charsum(s: &str) -> u64 {
    s.bytes().map(|b| b as u64).sum()
}

/// 项目内真机 FM 数据根 (cargo 测试 cwd = crate 根, data/ 缺失自动跳过, 对齐
/// build.py test 语义 — D4 验收注)
fn fm_root() -> String {
    format!(
        "{}/../../../data/aces/gamedata/flightmodels",
        env!("CARGO_MANIFEST_DIR")
    )
}

// ---- oracle: cut_c1~c8 — 块提取/嵌套/大小写/未闭合/无空格块头 ----
#[test]
fn java8_oracle_cut() {
    assert_eq!(
        cut("unit {\n\tWing {\n\t\tsweep:r = 25\n\t}\n}\n", "Wing"),
        "\n\t\tsweep:r = 25\n\t",
        "c1 嵌套块"
    );
    assert_eq!(cut("unit {\n}\n", "Wing"), "null", "c2 未找到");
    assert_eq!(cut("a { b { c", "a"), " b { c", "c3 未闭合返回余段");
    assert_eq!(cut("MODS { q }", "mods"), " q ", "c4 大小写不敏感");
    assert_eq!(cut("mods{q}", "mods"), "null", "c5 成员版无无空格回退");
    assert_eq!(cut("x { i { y } t }", "x"), " i { y } t ", "c6 嵌套平衡");
    assert_eq!(cut("a { b } c }", "a"), " b ", "c7 首个配对即止");
    assert_eq!(cut("", "a"), "null", "c8 空文本");
}

// ---- oracle: ga_g1~g7 — 多行累积/末行无换行/未找到/无等号/点分路径/大小写 ----
#[test]
fn java8_oracle_get_array() {
    let mut b = Blkx::default();
    b.data = Some("t1 {\n k:r = 1\n k:r = 2\n}\n".to_string());
    assert_eq!(b.get_array("t1.k"), " 1\n 2\n", "g1 多行含行尾\\n");
    b.data = Some("t1 {\n k:r = 1\n k:r = 2".to_string());
    assert_eq!(b.get_array("t1.k"), " 1\n 2", "g2 末行无换行不带\\n");
    b.data = Some("nothing here".to_string());
    assert_eq!(b.get_array("t1.k"), "", "g3 未找到");
    b.data = Some("t1 {\n k noeq\n}\n".to_string());
    assert_eq!(b.get_array("t1.k"), "", "g4 匹配但无等号");
    b.data = Some("A {\n B {\n  v:r = 7\n }\n}\n".to_string());
    assert_eq!(b.get_array("A.B.v"), " 7\n", "g5 两级点分");
    b.data = Some("t1 {\n K:r = 9\n}\n".to_string());
    assert_eq!(b.get_array("t1.k"), " 9\n", "g6 toUpperCase 定位");
    b.data = Some("k:t = \"x\"\nk:t = \"y\"\n".to_string());
    assert_eq!(b.get_array("k"), " \"x\"\n \"y\"\n", "g7 顶层多值");
}

// ---- oracle: glo_l1~l7 — 末次匹配/None 语义/无换行取到末尾/大小写 ----
#[test]
fn java8_oracle_getlastone() {
    let mut b = Blkx::default();
    b.data = Some("fmFile:t = \"fm/spitfire_f24.blk\"\n".to_string());
    assert_eq!(
        b.getlastone("fmfile"),
        Some(" \"fm/spitfire_f24.blk\"".to_string()),
        "l1 大小写不敏感, 值含前导空格与引号 (FMLoader L87 自行去壳)"
    );
    b.data = Some("a {\n b:r = 1\n}\nc:r = 2\n".to_string());
    assert_eq!(b.getlastone("c"), Some(" 2".to_string()), "l2");
    assert_eq!(b.getlastone("zzz"), None, "l3 未找到→null");
    b.data = Some("k no eq sign".to_string());
    assert_eq!(b.getlastone("k"), None, "l4 无等号→null");
    b.data = Some("k:r = 5".to_string());
    assert_eq!(b.getlastone("k"), Some(" 5".to_string()), "l5 无换行取到末尾");
    b.data = Some("K:r = 1\nk:r = 2\n".to_string());
    assert_eq!(b.getlastone("k"), Some(" 2".to_string()), "l6 rfind 取末次");
    b.data = Some("A {\n B {\n  v:r = 7\n }\n}\n".to_string());
    assert_eq!(b.getlastone("A.B.v"), Some(" 7".to_string()), "l7 点分+cut");
}

// ---- oracle: god_d1~d5 — 显式数据源定位/哨兵/点分/大小写 ----
#[test]
fn java8_oracle_getonein_data() {
    let b = Blkx::default();
    assert_eq!(b.getonein_data("blk {\n key:r = 5\n}\n", "key"), " 5", "d1");
    assert_eq!(b.getonein_data("blk {\n key:r = 5\n}\n", "zzz"), "null", "d2");
    assert_eq!(b.getonein_data("A {\n B {\n  v:r = 7\n }\n}\n", "A.B.v"), " 7", "d3");
    assert_eq!(b.getonein_data("k no eq\n", "k"), "null", "d4 无等号");
    assert_eq!(b.getonein_data("K:r = 1\n", "k"), " 1", "d5 大小写不敏感");
}

// ---- oracle: go_o1~o6 — getone 大小写敏感是源码本意 (toUpperCase 行已注释) ----
#[test]
fn java8_oracle_getone() {
    let mut b = Blkx::default();
    b.data = Some("Wingspan:r = 11.3\n".to_string());
    assert_eq!(b.getone("Wingspan"), " 11.3", "o1");
    assert_eq!(b.getone("wingspan"), "null", "o2 大小写敏感");
    b.data = Some("A {\n B {\n  v:r = 7\n }\n}\n".to_string());
    assert_eq!(b.getone("A.B.v"), " 7", "o3");
    assert_eq!(b.getone("A.b.v"), " 7", "o4 cut 环节大小写不敏感");
    b.data = Some("k:r = 5".to_string());
    assert_eq!(b.getone("k"), " 5", "o5 无换行");
    b.data = Some("k no eq\n".to_string());
    assert_eq!(b.getone("k"), "null", "o6 无等号");
}

// ---- oracle: ctor_n2~n5/n7/n8 — 构造器守卫 (parse_str 承接, content 即读入后 data) ----
#[test]
fn java8_oracle_parse_str_guards() {
    // n2/n3: 空与纯空白 → valid=false
    assert!(Blkx::parse_str("empty.blk", "").is_err(), "n2 空");
    assert!(Blkx::parse_str("ws.blk", "   \n\t \n").is_err(), "n3 纯空白");
    // n4: JSON 误喂 → valid=false (Err 文本对齐 Java warn 日志)
    let e = Blkx::parse_str("json.blk", "{\n  \"a\": 1\n}\n").unwrap_err();
    assert!(e.contains("JSON 格式文件误作 FM 加载"), "n4 Err 文本: {e}");
    // n5: 正常文件 → valid=true, data/readFileName/fmdata 齐
    let b = Blkx::parse_str("good.blk", "unit {\n\tWing {\n\t\tsweep:r = 25\n\t}\n}\n").unwrap();
    assert!(b.valid, "n5 valid");
    assert_eq!(b.data.as_deref(), Some("unit {\n\tWing {\n\t\tsweep:r = 25\n\t}\n}\n"));
    assert_eq!(b.read_file_name.as_deref(), Some("good.blk"));
    assert_eq!(b.fmdata.as_deref(), Some("找不到blkx文件\n请使用最新WT拆包aces.vromfs.bin"));
    // n7: Java trim 只剥 <= U+0020 — NBSP 保留 → 非空且非 JSON → valid=true
    assert!(Blkx::parse_str("nbsp.blk", "\u{00A0}{x\n").is_ok(), "n7 NBSP 保真");
    // n8: 前导 ASCII 空白后的 JSON → trim 后以 { 开头 → valid=false
    assert!(Blkx::parse_str("wsjson.blk", "  {\n\"a\": 1\n}\n").is_err(), "n8");
}

/// oracle: ctor_n1/n6 — 文件路径构造 (missing / 无行尾换行补 \n)
#[test]
fn parse_missing_and_readline_join() {
    // n1: 文件不存在 → Err (Java 静默 valid=false)
    let missing = format!("{}/blkx_test_missing_{}.blk", std::env::temp_dir().display(), line!());
    assert!(Blkx::parse(&missing).is_err(), "n1 不存在");

    // n6: 文件末行无换行 → readLine 语义补行尾 \n (oracle: "a:r = 1" → len8:sum453)
    let p = std::env::temp_dir().join(format!("blkx_test_nonl_{}.blk", line!()));
    std::fs::write(&p, b"a:r = 1").unwrap();
    let b = Blkx::parse(p.to_str().unwrap()).unwrap();
    assert!(b.valid, "n6 valid");
    assert_eq!(b.data.as_deref(), Some("a:r = 1\n"), "readLine 补 \\n");
    assert_eq!(b.data.as_deref().unwrap().len(), 8);
    assert_eq!(charsum(b.data.as_deref().unwrap()), 453, "oracle sum");
    assert_eq!(
        b.read_file_name.as_deref(),
        Some(p.file_name().unwrap().to_str().unwrap()),
        "单参入口 name 取文件名分量"
    );
    std::fs::remove_file(&p).ok();

    // n4 等价: JSON 文件走 parse → Err
    let pj = std::env::temp_dir().join(format!("blkx_test_json_{}.blk", line!()));
    std::fs::write(&pj, b"{\n  \"a\": 1\n}\n").unwrap();
    assert!(Blkx::parse(pj.to_str().unwrap()).is_err(), "JSON 误喂");
    std::fs::remove_file(&pj).ok();
}

/// oracle: real_* — 真机三文件全链路 (构造器 + 原语), data/ 缺失自动跳过
#[test]
fn parse_real_fm_files() {
    let root = fm_root();
    let phys_path = format!("{root}/fm/spitfire_f24.blkx");
    if !std::path::Path::new(&phys_path).exists() {
        return; // data/ 未解包 (D4: 对齐 build.py 跳过语义)
    }

    // 物理 FM: ctor_real_phys (len18559:sum1376835) + getone (real_go/go2)
    let phys = Blkx::parse(&phys_path).unwrap();
    assert!(phys.valid, "物理文件 valid");
    let data = phys.data.as_deref().unwrap();
    assert_eq!(data.len(), 18559, "real_phys len");
    assert_eq!(charsum(data), 1376835, "real_phys charsum");
    assert!(data.starts_with("AileronEffectiveSpeed:r = 482\nRudderEffe"), "real_phys head");
    assert!(data.ends_with(" {\n\n\t}\n\tIAS {\n\n\t}\n}\n"), "real_phys tail (readLine 补 \\n)");
    assert_eq!(phys.getone("Wingspan"), " 11.3", "real_go");
    assert_eq!(phys.getone("WingTaperRatio"), " 2", "real_go2");
    // real_ga_empty: spitfire 的 PASSPORT.ALT 为空块 → getArray 空串
    assert_eq!(phys.get_array("PASSPORT.ALT.minClimbTimeWep"), "", "real_ga_empty");

    // 中央文件: ctor_real_central (len46687:sum3233530) + getlastone("fmfile")
    let central_path = format!("{root}/spitfire_f24.blkx");
    let central = Blkx::parse(&central_path).unwrap();
    assert!(central.valid, "中央文件 valid");
    let cdata = central.data.as_deref().unwrap();
    assert_eq!(cdata.len(), 46687, "real_central len");
    assert_eq!(charsum(cdata), 3233530, "real_central charsum");
    assert!(cdata.starts_with("model:t = \"spitfire_f22\"\nfmFile:t = \"fm/"), "real_central head");
    assert_eq!(
        central.getlastone("fmfile"),
        Some(" \"fm/spitfire_f24.blk\"".to_string()),
        "real_glo (FMLoader L84 同款调用)"
    );

    // bf-109e-4: PASSPORT 曲线全链路 (cut×2 + 多行累积, real_ga_bf/bf2)
    let bf = Blkx::parse(&format!("{root}/fm/bf-109e-4.blkx")).unwrap();
    assert!(bf.valid);
    let ga = bf.get_array("PASSPORT.ALT.minClimbTimeWep");
    assert_eq!(ga.len(), 32, "real_ga_bf len");
    assert_eq!(charsum(&ga), 1342, "real_ga_bf sum");
    assert_eq!(ga, " 0, 0\n 1000, 137.4\n 2000, 271.4\n", "real_ga_bf 全文");
    assert_eq!(bf.get_array("PASSPORT.ALT.maxSpeedNom").len(), 95, "real_ga_bf2");
}

/// NPE 保真: 未 init/load 的对象 (data=None) 上调用原语 → Java NPE ↔ panic (§1)
#[test]
#[should_panic]
fn getone_on_null_data_panics_like_npe() {
    let b = Blkx::default();
    let _ = b.getone("k");
}

#[test]
#[should_panic]
fn get_array_on_null_data_panics_like_npe() {
    let b = Blkx::default();
    let _ = b.get_array("k");
}

#[test]
#[should_panic]
fn getlastone_on_null_data_panics_like_npe() {
    let b = Blkx::default();
    let _ = b.getlastone("k").unwrap();
}

/// java_trim 与 Java String.trim 同语义 (<= U+0020, 不含 NBSP)
#[test]
fn java_trim_matches_java_semantics() {
    assert_eq!(java_trim("  a\n\t"), "a");
    assert_eq!(java_trim("\u{0001}\u{0020}x\u{001F}"), "x");
    assert_eq!(java_trim("\u{00A0}{x"), "\u{00A0}{x", "NBSP 不剥 (Rust trim 会)");
    assert_eq!(java_trim(""), "");
    assert_eq!(java_trim("   "), "");
}

/// java_format: getload fmdata 串的 %s/%d/%N.Mf/%% 语义
#[test]
fn java_format_conversions() {
    assert_eq!(
        java_format("a %s b %d c %.2f%%", &[FmtArg::S("x".into()), FmtArg::D(7), FmtArg::F(2.675, 2)]),
        "a x b 7 c 2.68%"
    );
    // HALF_UP 对最短往返十进制 (2.675→2.68, Java %f 同 crate::format)
    assert_eq!(java_format("%.1f", &[FmtArg::F(-0.04, 1)]), "-0.0");
    // 无精度 %f 缺省 6 位
    assert_eq!(java_format("%f", &[FmtArg::F(1.5, 9)]), "1.500000");
}

/// oracle 真机对拍 (Java 8 DumpGetload/DumpFmdata, 2026-08): spitfire_f24 物理
/// FM 经 getload 全量装载的关键字段**位级一致** (vne/翼数据/WLL/RPM/增压器/
/// 襟翼表/过载转换); fmdata 摘要串 41/42 行逐行一致 (唯一分歧 = 版本号段,
/// Java dump CWD 读到 data/aces/version 而测试 CWD 读不到, 环境差异非代码)。
/// data/ 缺失自动跳过 (对齐 build.py test 语义)。
#[test]
fn getload_real_fm_spitfire_matches_java8() {
    let root = fm_root();
    let phys = format!("{root}/fm/spitfire_f24.blkx");
    if !std::path::Path::new(&phys).exists() {
        return; // data/ 未解包
    }
    let b = Blkx::parse(&phys).unwrap();
    assert!(b.valid, "getload 全程无 panic (构造器 catch_unwind 未触发)");
    assert!(!b.is_jet);
    assert_eq!(b.engine_num, 1);
    // vne/mach 限 (Java oracle: 875.0 / 0.8700000047683716)
    assert_eq!(b.vne, 875.0);
    assert_eq!(b.vne_mach.to_bits(), (0.87f32 as f64).to_bits(), "Float.parseFloat 拓宽域");
    // 重量族 (oracle)
    assert_eq!(b.emptyweight, 3550.0);
    assert_eq!(b.maxfuelweight, 390.0);
    assert!(b.oil > 36.7 && b.oil < 36.71, "36.7f 拓宽域 (实际 {})", b.oil);
    assert_eq!(b.nofuelweight, 3586.7000007629395);
    // 翼面积/展弦比/翼载 (oracle 位级)
    assert_eq!(b.wingspan.to_bits(), 11.300000190734863f64.to_bits());
    assert_eq!(b.a_wing, 24.09599969536066);
    assert_eq!(b.aspect_ratio, 5.299220033406327);
    assert_eq!(b.no_flap_wll, 8.82388688872734);
    assert_eq!(b.full_flap_wll, 12.557070425608627);
    assert_eq!(b.cd_s, 0.20481600854137527);
    assert_eq!(b.ind_cd_f, 0.0667414559982402);
    // 部件极曲线 (oracle 位级抽检)
    let nf = b.no_flaps_wing.as_ref().unwrap();
    assert!(nf.aoa_crit_high > 17.79 && nf.aoa_crit_high < 17.81);
    assert_eq!(nf.cl_crit_high.to_bits(), 1.2999999523162842f64.to_bits());
    let ff = b.full_flaps_wing.as_ref().unwrap();
    assert_eq!(ff.aoa_crit_high, 16.0);
    assert_eq!(ff.cl_crit_high.to_bits(), 1.850000023841858f64.to_bits());
    // 舵面效率/损失 (oracle)
    assert_eq!(b.aileron_eff, 482.0);
    assert_eq!(b.rudder_eff, 400.0);
    assert_eq!(b.elav_eff, 400.0);
    // RPM 族 (oracle; milRPM/wepRMP 来自 ThrottleRPMAuto 2600/2750)
    assert_eq!(b.max_rpm, 2750.0);
    assert_eq!(b.max_allowed_rpm, 3100.0);
    assert_eq!(b.military_rpm, 2600.0);
    assert_eq!(b.wep_rpm, 2750.0);
    assert_eq!(b.governor_max_param, 2600.0);
    // 增压器两档 (oracle: 4100/8100m, 1510/1340hp)
    assert_eq!(b.comp_num_steps, 2);
    assert_eq!(b.comp_alt.as_deref(), Some(&[4100.0, 8100.0][..]));
    assert_eq!(b.comp_power.as_deref(), Some(&[1510.0, 1340.0][..]));
    assert!(b.military_mp > 1.61 && b.military_mp < 1.611);
    // WAPC 族 (oracle)
    assert_eq!(b.deck_power, 1360.0);
    assert_eq!(b.rpm_nom, 2600.0);
    assert!(b.wep_manifold_pressure > 2.22 && b.wep_manifold_pressure < 2.23);
    // 襟翼破坏表: [0.5,290]/[1.0,260]/[1.25,0] 哨兵 (oracle)
    assert_eq!(b.flaps_destruction_num, 2);
    let ft = b.flaps_destruction_ind_speed.as_ref().unwrap();
    assert_eq!(ft[0], [0.5, 290.0]);
    assert_eq!(ft[1], [1.0, 260.0]);
    assert_eq!(ft[2], [1.25, 0.0], "1.25x 插值哨兵行");
    // 耐热档 (oracle: maxEngLoad=6, Load0 水80/油60, avgRecover=2)
    assert_eq!(b.max_eng_load, 6);
    let el = b.eng_load.as_ref().unwrap();
    assert_eq!(el[0].water_limit, 80.0);
    assert_eq!(el[0].oil_limit, 60.0);
    assert_eq!(el[6].water_limit, 999.0, "哨兵档");
    assert_eq!(b.avg_eng_recovery_rate, 2.0);
    // 过载 (oracle 位级: raw [-138000, 225000] → 转换 [-7.298…, 12.656…])
    assert_eq!(b.raw_wing_crit_overload, Some([-138000.0, 225000.0]));
    let gl = b.max_allow_gload.unwrap();
    assert_eq!(gl[0], -7.298483255177185);
    assert_eq!(gl[1], 12.656222698658453);
    // 转动惯量/可变翼 (oracle)
    assert_eq!(b.moment_of_inertia, Some([9500.0, 22500.0, 13000.0]));
    assert_eq!(b.is_v_wing, Some(false));
    // fmdata 摘要串已构造 (version 段因测试 CWD 读不到 data/aces/version 为空,
    // 生产 CWD=repo root 时对齐 Java "2.57.1.103")
    let fm = b.fmdata.as_deref().unwrap();
    assert!(fm.contains("spitfire_f24.blkx"), "版本行头");
    assert!(fm.contains("襟翼限速(km/h)0: 50% / 290"), "%% 转义与档位行");
    assert!(fm.contains("翼展效率: 0.90 展弦比: 5.3"), "bLift 数值段");
}

// ---- getAllplotdata 批次: transUnit/getAllplotdata/getplotdata ----
// oracle 来源: build/oracle/DumpPlot/DumpPlot.java (真机 bf-109e-4 metric 腿A +
// Passport 块头插大写 UNITSYSTEM 键的合成英制腿B), OpenJDK 1.8.0_342 实测 dump,
// doubleToLongBits 十六进制 → Rust to_bits() 逐位断言。

/// oracle 真机对拍 (DumpPlot 腿A): bf-109e-4 的 PASSPORT 曲线五条全量
/// (metric 路径, transUnit 空转 — 文件无 unitSystem 键), 行数与首尾锚点位级
/// 一致。parse_str = doLoad=false: getAllplotdata 只读 data 文本, 与 Java
/// DumpPlot (doLoad=true 全管线) 输出恒等 — getload 不触碰 loc 族。
/// data/ 缺失自动跳过 (对齐 build.py test 语义)。
#[test]
fn get_all_plotdata_real_bf109_metric_matches_java8() {
    let root = fm_root();
    let fm_path = format!("{root}/fm/bf-109e-4.blkx");
    if !std::path::Path::new(&fm_path).exists() {
        return; // data/ 未解包
    }
    let content = std::fs::read_to_string(&fm_path).unwrap();
    let mut b = Blkx::parse_str("fm/bf-109e-4.blk", &content).unwrap();
    b.get_all_plotdata();

    // 行数 (oracle cur: 3/6/7/7/3)
    assert_eq!(b.loc.as_ref().unwrap().cur, 3, "loc (minClimbTimeWep)");
    assert_eq!(b.loc0.as_ref().unwrap().cur, 6, "loc0 (minClimbTimeNom)");
    assert_eq!(b.loc1.as_ref().unwrap().cur, 7, "loc1 (maxSpeedWep)");
    assert_eq!(b.loc2.as_ref().unwrap().cur, 7, "loc2 (maxSpeedNom)");
    assert_eq!(b.loc3.as_ref().unwrap().cur, 3, "loc3 (maxRollRateLeft)");

    // 首尾锚点 (oracle doubleToLongBits hex, 逐位断言; 括号内十进制对照)
    let loc = b.loc.as_ref().unwrap();
    assert_eq!(loc.y[0].to_bits(), 0, "loc.y[0] (0.0)");
    assert_eq!(loc.x[0].to_bits(), 0, "loc.x[0] (0.0)");
    assert_eq!(loc.y[2].to_bits(), 0x409f400000000000, "loc.y[2] (2000.0)");
    assert_eq!(loc.x[2].to_bits(), 0x4070f66666666666, "loc.x[2] (271.4)");
    let loc0 = b.loc0.as_ref().unwrap();
    assert_eq!(loc0.y[5].to_bits(), 0x40b3880000000000, "loc0.y[5] (5000.0)");
    assert_eq!(loc0.x[5].to_bits(), 0x4090d26666666666, "loc0.x[5] (1076.6)");
    let loc1 = b.loc1.as_ref().unwrap();
    assert_eq!(loc1.x[0].to_bits(), 0x40775ae147ae147b, "loc1.x[0] (373.68)");
    assert_eq!(loc1.y[6].to_bits(), 0x40b7700000000000, "loc1.y[6] (6000.0)");
    assert_eq!(loc1.x[6].to_bits(), 0x407b847ae147ae14, "loc1.x[6] (440.28)");
    let loc2 = b.loc2.as_ref().unwrap();
    assert_eq!(loc2.x[0].to_bits(), 0x4075d8f5c28f5c29, "loc2.x[0] (349.56)");
    assert_eq!(loc2.x[6].to_bits(), 0x407a13d70a3d70a4, "loc2.x[6] (417.24)");
    let loc3 = b.loc3.as_ref().unwrap();
    assert_eq!(loc3.y[0].to_bits(), 0x40741d70a3d70a3d, "loc3.y[0] (321.84)");
    assert_eq!(loc3.x[0].to_bits(), 0x40413051eb851eb8, "loc3.x[0] (34.3775)");
    assert_eq!(loc3.y[2].to_bits(), 0x408219eb851eb852, "loc3.y[2] (579.24)");
    assert_eq!(loc3.x[2].to_bits(), 0x4049c8793dd97f63, "loc3.x[2] (51.5662)");
}

/// oracle 合成英制对拍 (DumpPlot 腿B): bf-109e-4 + Passport 块头插**大写**
/// UNITSYSTEM 键 (getone 定位大小写敏感, 只有字面 "UNITSYSTEM" 键能喂到
/// sub_st; 真机键名恒小写 camelCase → 真数据不走换算) → transUnit 换算路径:
/// loc/loc0 的 y×0.3048f (x 不动), loc1/loc2 的 y×0.3048f + x×1.609344f,
/// loc3 的 y×1.609344f (x 不动)。f32 字面量拓宽域位级断言。
#[test]
fn get_all_plotdata_imperial_conversion_matches_java8() {
    let root = fm_root();
    let fm_path = format!("{root}/fm/bf-109e-4.blkx");
    if !std::path::Path::new(&fm_path).exists() {
        return; // data/ 未解包
    }
    let content = std::fs::read_to_string(&fm_path).unwrap();
    // 与 Java DumpPlot 同款插入 (Passport 块在行首无缩进, od 实测)
    let imperial =
        content.replacen("\nPassport {\n", "\nPassport {\n\t\tUNITSYSTEM:t = \"Imperial\"\n", 1);
    assert!(imperial.len() > content.len(), "UNITSYSTEM 行已插入");
    let mut b = Blkx::parse_str("fm/imperial.blk", &imperial).unwrap();
    b.get_all_plotdata();

    // 行数与 metric 腿一致 (换算只改数值不改行数)
    assert_eq!(b.loc.as_ref().unwrap().cur, 3);
    assert_eq!(b.loc0.as_ref().unwrap().cur, 6);
    assert_eq!(b.loc1.as_ref().unwrap().cur, 7);
    assert_eq!(b.loc2.as_ref().unwrap().cur, 7);
    assert_eq!(b.loc3.as_ref().unwrap().cur, 3);

    // 换算锚点 (oracle: float 字面量先取 f32 值再拓宽参与乘法)
    let loc = b.loc.as_ref().unwrap();
    assert_eq!(loc.y[1].to_bits(), 0x40730cccd0c00000, "1000 * 0.3048f = 304.80000376701355");
    assert_eq!(loc.x[1].to_bits(), 0x40612ccccccccccd, "loc.x 不换算 (137.4)");
    let loc0 = b.loc0.as_ref().unwrap();
    assert_eq!(loc0.y[5].to_bits(), 0x4097d00004f00000, "5000 * 0.3048f = 1524.0000188350677");
    let loc1 = b.loc1.as_ref().unwrap();
    assert_eq!(loc1.y[3].to_bits(), 0x408c933339200000, "3000 * 0.3048f = 914.4000113010406");
    assert_eq!(loc1.x[0].to_bits(), 0x4082cb098f6147ae, "373.68 * 1.609344f = 601.379668006897");
    let loc2 = b.loc2.as_ref().unwrap();
    assert_eq!(loc2.y[0].to_bits(), 0, "0.0 * 0.3048f = 0.0");
    assert_eq!(loc2.x[0].to_bits(), 0x4081947f9235c28f, "349.56 * 1.609344f = 562.5622905921936");
    let loc3 = b.loc3.as_ref().unwrap();
    assert_eq!(loc3.y[0].to_bits(), 0x40802f9c35f0a3d7, "321.84 * 1.609344f = 517.9512747573852");
    assert_eq!(loc3.x[0].to_bits(), 0x40413051eb851eb8, "loc3.x 不换算 (34.3775)");
}

/// oracle 真机对拍: spitfire_f24 的 Passport 块为空 (Alt {}/IAS {} 无键,
/// real_ga_empty 已钉 getArray 空串) → 五条曲线全空 (cur=0, 空表); 走完整
/// doLoad=true 管线 + getAllplotdata, 验证 getload/getAllplotdata 两阶段共存
/// (spitfire 的 getload 真机测试已钉解析成功)。
#[test]
fn get_all_plotdata_real_spitfire_empty_curves() {
    let root = fm_root();
    let fm_path = format!("{root}/fm/spitfire_f24.blkx");
    if !std::path::Path::new(&fm_path).exists() {
        return; // data/ 未解包
    }
    let mut b = Blkx::parse(&fm_path).unwrap();
    assert!(b.valid);
    b.get_all_plotdata();
    for (name, lo) in [
        ("loc", &b.loc),
        ("loc0", &b.loc0),
        ("loc1", &b.loc1),
        ("loc2", &b.loc2),
        ("loc3", &b.loc3),
    ] {
        let lo = lo.as_ref().unwrap();
        assert_eq!(lo.cur, 0, "{name} 空块 → cur=0");
        assert!(lo.x.is_empty() && lo.y.is_empty(), "{name} 空表");
    }
    // transUnit 空转不 panic (unitSystem 键缺席 → getone "null" → sub_st "ul"),
    // finalize 收尾与生产管线 (FMLoader.load 第 6 步) 同序列
    b.finalize_loading();
    assert!(b.data.is_none());
}

/// getplotdata 畸形行防御 (P6 fuzz 发现的加固面): 非数字段/缺逗号行/尾部空串
/// 行跳过该数据点, 完好行照常解析 (Java split 丢尾空串 → len<2 跳过与 Rust
/// tmp[1]="" 解析失败丢弃两条路径, 见 reader.rs PORT 注)
#[test]
fn getplotdata_malformed_lines_skip_points() {
    let mut b = Blkx::default();
    b.data = Some(
        "blk {\n k:p2 = 1.0, 2.0\n k:p2 = nodigit, 3.0\n k:p2 = 4.0\n k = 5.0, 6.0\n k = 7.0, \n}\n"
            .to_string(),
    );
    let lo = b.getplotdata("blk.k");
    // 5 个 '\n' → 容量 5; 第2行 y 非数字丢弃, 第3行缺 ", " 丢弃,
    // 第5行尾部空串丢弃 → 仅 2 个完好点
    assert_eq!(lo.cur, 2, "畸形行跳过, cur=2");
    assert_eq!((lo.y[0], lo.x[0]), (1.0, 2.0));
    assert_eq!((lo.y[1], lo.x[1]), (5.0, 6.0));
}
