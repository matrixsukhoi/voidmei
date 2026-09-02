use super::*;
use crate::fm::status::FMStatus;

/// Java check(boolean, String) 计数式断言 → assert! 宏, 描述逐字保留
fn check(cond: bool, desc: &str) {
    assert!(cond, "FAIL: {desc}");
}

// ---- 合成数据 (TestFMStore.setupSyntheticData 一比一移植) ----

/// DATA_ROOT 的全部可能取值: 默认根 + java_main_sequence 临时注入的根
/// (见模块注释; load 读取时任取其一/中途切换均有效)
const ROOTS: [&str; 3] = ["./data", "testroot", "otherroot"];

/// 各候选根下的 flightmodels 目录 (中央文件所在)
fn fm_dir_of(root: &str) -> String {
    format!("{root}/aces/gamedata/flightmodels")
}

/// 合成数据铺设 (blkx→json 迁移终态: 只铺 .json)
fn write_json(root: &str, rel: &str, json_text: &str) {
    std::fs::write(format!("{}/{rel}.json", fm_dir_of(root)), json_text).unwrap();
}

/// 机型名统一 zzfmload_ 前缀: 各根下绝不与真机 FM / 其他测试文件重名
fn write_central(root: &str, name: &str) {
    // 最小中央文件 —— 只需 get_last_string_ci("fmfile") 能命中
    write_json(
        root,
        name,
        &format!("{{\"model\": \"{name}\", \"fmFile\": \"fm/{name}.blk\"}}"),
    );
}

/// 中央文件无 fmFile 行 → 触发目录约定回退
fn write_central_no_fmfile(root: &str, name: &str) {
    write_json(root, name, &format!("{{\"model\": \"{name}\"}}"));
}

/// 中央文件 fmFile 值不带 .blk 后缀 → 触发补后缀分支
fn write_central_noext(root: &str, name: &str) {
    write_json(
        root,
        name,
        &format!("{{\"model\": \"{name}\", \"fmFile\": \"fm/{name}\"}}"),
    );
}

/// 中央文件带苏联燃油改装块 → 触发 extract_fuel_modifications_json + info 日志分支
fn write_central_fuel(root: &str, name: &str) {
    write_json(
        root,
        name,
        &format!(
            "{{\"model\": \"{name}\", \"fmFile\": \"fm/{name}.blk\", \"modifications\": {{\"ussr_fuel_b-100\": {{\"effects\": {{\"addHorsePowers\": 50.0}}}}}}}}"
        ),
    );
}

/// 最小物理 FM —— 顶层标量的等价树; getload 对缺失字段全按 0 处理
/// （无 Jet/Compressor 块 → 按喷气形态、compNumSteps=0，extractStages 返回
/// null、peakThrust=0），最终 valid=true → READY。
fn write_physical(root: &str, name: &str) {
    write_json(
        root,
        &format!("fm/{name}"),
        "{\"synthetic-fm\": \"x\", \"EmptyMass\": 1000.0, \"Wingspan\": 11.0}",
    );
}

fn setup_synthetic_data() {
    for root in ROOTS {
        std::fs::create_dir_all(format!("{}/fm", fm_dir_of(root))).unwrap();

        // 可加载机型: central 指向 fm/<name>.blk, 物理文件存在
        write_central(root, "zzfmload_plane1");
        write_physical(root, "zzfmload_plane1");
        // fmFile 回退: central 无 fmFile → fm/<name>.blk 约定
        write_central_no_fmfile(root, "zzfmload_fb");
        write_physical(root, "zzfmload_fb");
        // fmFile 无 .blk 后缀: 剥引号后补 ".blk"
        write_central_noext(root, "zzfmload_nb");
        write_physical(root, "zzfmload_nb");
        // 燃油改装: soviet b-100 addHorsePowers=50
        write_central_fuel(root, "zzfmload_fuel");
        write_physical(root, "zzfmload_fuel");

        // CORRUPT 机型: central 在库但物理文件缺失
        write_central(root, "zzfmload_badplane");

        // ghost: 什么都不写 → MISSING
    }
}

/// 清理: 只删本测试落盘的文件; 目录仅在其为空时移除 (绝不触动既有 data/ 内容)
fn cleanup_synthetic_data() {
    for root in ROOTS {
        for name in [
            "zzfmload_plane1",
            "zzfmload_fb",
            "zzfmload_nb",
            "zzfmload_fuel",
            "zzfmload_badplane",
        ] {
            let _ = std::fs::remove_file(format!("{}/{name}.json", fm_dir_of(root)));
            let _ = std::fs::remove_file(format!("{}/fm/{name}.json", fm_dir_of(root)));
        }
        // 自内向外逐层 prune 空目录 (remove_dir 对非空目录失败即止)
        for dir in [
            format!("{}/fm", fm_dir_of(root)),
            fm_dir_of(root),
            format!("{root}/aces/gamedata"),
            format!("{root}/aces"),
            root.to_string(),
        ] {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

/// Drop 兜底清理 (断言 panic 展栈时也还原 cwd 下的合成文件)
struct CleanupOnDrop;
impl Drop for CleanupOnDrop {
    fn drop(&mut self) {
        cleanup_synthetic_data();
    }
}

/// TestFMStore 的 FMLoader 面 + 边界补充, 一次顺序执行
/// (DATA_ROOT 竞态已由多根铺数据免疫, 见模块注释 —— 直接 load, 无需重试)
fn run_cases() {
    reset_load_count();

    // -- 空名守卫: null / "" → UNRESOLVED 且不计入 loadCount --
    let h = load(None);
    check(h.status == FMStatus::Unresolved && h.name.is_none(), "null → UNRESOLVED");
    let h = load(Some(""));
    check(h.status == FMStatus::Unresolved, "空串 → UNRESOLVED");
    check(get_load_count() == 0, "空名不进入加载流程 (loadCount 不增)");

    // -- READY: central + physical 齐全 (大小写规范化) --
    let h = load(Some("ZZFMLOAD_PLANE1"));
    check(h.status == FMStatus::Ready, "合成齐全机型应 READY");
    check(h.name.as_deref() == Some("zzfmload_plane1"), "机型名规范化为小写");
    check(h.has_fm() && h.fmdata.is_some(), "READY 句柄应携带 fmdata");
    // readFileName 传参链锁死 (物理侧; 消费者 ui_model/fm_data_adapter.rs
    // get_fm_version —— 中央侧 name+".blk" 进 getload 版本串, 波次未落地
    // 暂无观察点)
    check(
        h.fmdata.as_ref().unwrap().read_file_name.as_deref() == Some("fm/zzfmload_plane1.blk"),
        "物理文件 readFileName = fmfile 相对路径 (Java L101)",
    );
    // PORT: getload 未落地 (try_load 步骤5 TODO) — 数值字段暂为 0,
    // getload 波次落地后此断言需更新为真实喷气/活塞口径
    check(h.peak_wep_power == 0.0 && h.peak_thrust == 0.0, "getload 未落地: 功率/推力暂为 0");
    check(h.compressor_stages.is_none(), "无 Compressor 块 → stages 为 None");
    check(get_load_count() == 1, "READY 路径 loadCount=1");

    // -- CORRUPT: central 在库但物理文件缺失 (TestFMStore badplane) --
    let h = load(Some("zzfmload_badplane"));
    check(h.status == FMStatus::Corrupt, "物理文件缺失应为 CORRUPT");
    check(h.is_missing_like() && !h.has_fm(), "CORRUPT 属 missing-like 且无 FM");

    // -- MISSING: 什么都不放 (TestFMStore ghost) --
    let h = load(Some("zzfmload_ghost"));
    check(h.status == FMStatus::Missing, "不在库机型应为 MISSING");
    check(h.is_missing_like(), "MISSING 属 missing-like");
    check(h.name.as_deref() == Some("zzfmload_ghost"), "MISSING 保留机型名");

    // -- fmFile 回退: central 未写 fmFile → fm/<name>.blk 约定 --
    let h = load(Some("zzfmload_fb"));
    check(h.status == FMStatus::Ready, "目录约定回退应命中物理文件");

    // -- fmFile 无 .blk 后缀 → 剥引号后补 ".blk" --
    let h = load(Some("zzfmload_nb"));
    check(h.status == FMStatus::Ready, "无后缀 fmFile 补 .blk 后应命中");

    // -- 燃油改装分支: soviet b-100 检出 (info 日志) 且不阻断加载 --
    let h = load(Some("zzfmload_fuel"));
    check(h.status == FMStatus::Ready, "带燃油改装的中央文件仍应 READY");

    check(get_load_count() == 6, "六次有效加载 (READY x4 + CORRUPT + MISSING)");
}

#[test]
fn loader_contract_synthetic() {
    // DATA_ROOT 测试串行锁 (test_guard): 与未来接入该锁的 DATA_ROOT 相关
    // 测试互斥 (java_main_sequence 未接入期间的翻转免疫靠多根铺数据)
    let _guard = crate::fm::test_support::data_root();
    let _cleanup = CleanupOnDrop;
    setup_synthetic_data();
    run_cases();
}

/// 边界: java_double_str 复刻 Java Double.toString 拼接形态 (日志文本保真)
#[test]
fn java_double_str_matches_java_concat() {
    assert_eq!(java_double_str(50.0), "50.0");
    assert_eq!(java_double_str(0.0), "0.0");
    assert_eq!(java_double_str(1.5), "1.5");
    assert_eq!(java_double_str(-0.0), "-0.0");
}

/// panic 载荷提取的边界形态
#[test]
fn panic_message_payload_kinds() {
    let p: Box<dyn std::any::Any + Send> = Box::new("boom");
    assert_eq!(panic_message(p.as_ref()), "boom", "&str 载荷");
    let p: Box<dyn std::any::Any + Send> = Box::new(String::from("bang"));
    assert_eq!(panic_message(p.as_ref()), "bang", "String 载荷");
    let p: Box<dyn std::any::Any + Send> = Box::new(42i32);
    assert_eq!(panic_message(p.as_ref()), "unknown panic payload", "非字符串载荷");
}
