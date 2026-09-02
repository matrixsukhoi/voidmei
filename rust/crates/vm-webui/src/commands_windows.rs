//! P6 web 窗口域数据命令层: 三个 Java JDialog/WebFrame 辅助窗口的 Rust 数据面。
//!
//! | 命令 | Java 源 | 内容 |
//! |---|---|---|
//! | [`comparison_data`] | `ui/window/comparison/CompactComparisonWindow.java` | displayStructure + dataMap0/1 构造 + 胜负规则接线 (vm-core comparison rules 已译) |
//! | [`power_curve_data`] | `ui/window/comparison/PowerCurveWindow.java` | loadPowerCurves/loadSingleCurve: 曲线采样 (高度格点)/峰值谷值拐点检测/minPower/错误信息 |
//! | [`fm_list`] | `ui/window/comparison/GridSelectorDialog.java` | 机型选择搜索下拉的物理文件目录列表 (loadPlanes) |
//!
//! (flight_record_data / DrawFrame 数据链已删: 前端从未接线, 纯死代码。)
//!
//! 与 D9 主壳 IPC (commands.rs → mpsc → 主线程 dispatch) 的分工: 本组命令的
//! 计算面只依赖 vm-core (FMLoader/Blkx/对比规则, 全线程安全), **不经主线程
//! dispatcher** — AppShell (!Send) 不被触碰, vm-app 的 form_dispatch 零改动。
//!
//! 像素/布局 (GridBag/ChartPanel/Lang 之外的硬编码中文标题除外) 归 web 前端
//! (后续单元接), 本层只出数据。

use std::collections::HashMap;
use std::path::PathBuf;

use vm_core::fm::data::json::extract_fuel_modifications_json;
use vm_core::fm::data::{FmData, FuelModification, FuelType};
use vm_core::ui_support::comparison::comparison_rules::ComparisonRules;
use vm_core::base::file_utils::get_file_name_no_ex;
use vm_core::fm::data_paths;
use vm_core::fm::loader;
use vm_core::fm::power_extractor::{extract_stages_with_fuel, is_piston_engine};
use vm_core::lang::Lang;
use vm_core::base::logger;
use vm_core::fm::piston_model::generate_power_curve_advanced;

use crate::dto::{
    ComparisonDataDto, ComparisonRowDto, InflectionPointDto, PowerCurveDataDto, PowerCurveDto,
};

// =====================================================================
// 公共小件
// =====================================================================

/// Java `String.trim` 语义: 剥两端 <= U+0020 的码点。Rust `str::trim` 会多剥
/// NBSP 等 Unicode 空白 (§2.1 陷阱), fmdata 文本/机型名域为中文+ASCII, 逐字符
/// 复刻 Java 行为 (blkx/reader.rs 同款私有先例)。
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= '\u{20}')
}

/// Serialize → invoke 返回值 (命令薄壳共用)
fn to_json<T: serde::Serialize>(v: &T) -> Result<serde_json::Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// 名字空间差异回退的物理文件定位: 只查 `fm/<name>.json`, 不存在返回 None
/// (name 原样使用 — Java 拼串不做大小写规范化; blkx/blk 过渡期回落已随
/// blkx→json 迁移终态退役)。
/// 背景: name 是 fm/ 物理文件名（连字符, 如 a-10c）, 中央机型名是下划线
/// （a_10c）——少数不同名机型 FMLoader 判 MISSING, 按物理文件直读。
fn fallback_physical_file(name: &str) -> Option<PathBuf> {
    let f = data_paths::fm_dir().join(format!("fm/{name}.json"));
    if f.exists() { Some(f) } else { None }
}

// =====================================================================
// 1. 对比窗口数据 (CompactComparisonWindow.java)
// =====================================================================

/// Java `private static class DisplayItem` (isHeader + text)
#[derive(Debug, Clone)]
pub struct DisplayItem {
    pub is_header: bool,
    pub text: String,
}

// Regex to parse: "Property Name: Value [Unit]"
// Example: "空重(kg): 4644.0" -> Prop="空重(kg)", Val=4644.0
// Example: "临界速度(km/h): [144, 1167]" -> Prop="...", Val="[144, 1167]" (Complex)
// (CompactComparisonWindow.java PROP_PATTERN 原注释, 逐字保留 — PORTING.md §0.2)
//
// PORT: `([^:]+):\s*(.*)` 的 matches() (全串匹配) — `[^:]+` 无法跨越 ':', 故
// 第一个冒号即分界且要求 ≥1 个前置字符; `\s*` 吞组2头部空白 + group(2).trim()
// 归并为 java_trim (Java \s ⊂ trim 字符集, §2.1)
fn parse_prop_line(line: &str) -> Option<(String, String)> {
    let pos = line.find(':')?;
    if pos == 0 {
        return None; // ([^:]+) 需至少 1 个字符
    }
    let k = java_trim(&line[..pos]).to_string();
    let v = java_trim(&line[pos + 1..]).to_string();
    Some((k, v))
}

/// Java `findInStructure` (CompactComparisonWindow.java:388-394)
fn find_in_structure(list: &[DisplayItem], key: &str) -> i64 {
    for (i, item) in list.iter().enumerate() {
        if !item.is_header && item.text == key {
            return i as i64;
        }
    }
    -1
}

/// initUI 的解析段 (CompactComparisonWindow.java:195-252): lines0 建结构 +
/// dataMap0, lines1 合并独有键进结构、全量进 dataMap1。纯函数 (无 UI/文件)。
fn build_structure(
    lines0: &[String],
    lines1_safe: &[String],
) -> (Vec<DisplayItem>, HashMap<String, String>, HashMap<String, String>) {
    let mut structure: Vec<DisplayItem> = Vec::new();
    let mut map0: HashMap<String, String> = HashMap::new();
    let mut map1: HashMap<String, String> = HashMap::new();

    // 1. Build initial structure from lines0
    for l in lines0 {
        let l = java_trim(l);
        if l.is_empty() {
            continue;
        }
        if l.contains("------") {
            let h = java_trim(&l.replace('-', "")).to_string();
            structure.push(DisplayItem { is_header: true, text: h });
            continue;
        }
        if let Some((k, v)) = parse_prop_line(l) {
            structure.push(DisplayItem { is_header: false, text: k.clone() });
            map0.insert(k, v);
        }
    }

    // 2. Parse lines1 and merge unique keys
    let mut last_match_index: i64 = -1;
    // Find where the content starts (skip initial headers if possible, or just
    // merge)
    // Simple merge: scan lines1. If key exists, update index. If not, insert after
    // index.
    // (原注释保留)
    for l in lines1_safe {
        let l = java_trim(l);
        // Skip headers in merge for now to avoid duplications (原注释保留)
        if l.is_empty() || l.contains("------") {
            continue;
        }
        if let Some((k, v)) = parse_prop_line(l) {
            map1.insert(k.clone(), v);

            // Check struct
            let idx = find_in_structure(&structure, &k);
            if idx != -1 {
                last_match_index = idx;
            } else {
                // Insert after last match
                if last_match_index < structure.len() as i64 - 1 {
                    structure.insert(
                        (last_match_index + 1) as usize,
                        DisplayItem { is_header: false, text: k },
                    );
                } else {
                    structure.push(DisplayItem { is_header: false, text: k });
                }
                last_match_index += 1;
            }
        }
    }

    (structure, map0, map1)
}

/// addComparisonRow 的胜负判定 (CompactComparisonWindow.java:104-114)。
/// 入参用**展示串** (缺键已补 "-"), 与 Java 调用点一致 (extractValue("-") 无数字
/// → None → 平局)。
fn row_win(prop: &str, v0: &str, v1: &str, single_mode: bool) -> i32 {
    // Determine Winner using rule system
    let mut win = 0; // 0=draw, -1=left(v0), 1=right(v1)
    if let Some(rule) = ComparisonRules::get(prop) {
        if !single_mode {
            let d0 = rule.extract_value(Some(v0));
            let d1 = rule.extract_value(Some(v1));
            if let (Some(d0), Some(d1)) = (d0, d1) {
                if (d0 - d1).abs() > 0.001 {
                    let lower_is_better = rule.is_lower_better();
                    win = if d0 > d1 {
                        if lower_is_better { 1 } else { -1 }
                    } else if lower_is_better {
                        -1
                    } else {
                        1
                    };
                }
            }
        }
    }
    // No rule → win=0 → draw (grey color)
    win
}

/// determineWinner (buildCopyText 用, CompactComparisonWindow.java:434-442) —
/// 与 row_win 的差异: 入参是**原始可空值** (null 判在规则前)。
fn winner_name(
    prop: &str,
    v0: Option<&str>,
    v1: Option<&str>,
    fm0_name: &str,
    fm1_name: &str,
) -> Option<String> {
    let rule = ComparisonRules::get(prop)?;
    let (v0, v1) = (v0?, v1?);
    let d0 = rule.extract_value(Some(v0))?;
    let d1 = rule.extract_value(Some(v1))?;
    // Java `Math.abs(d0 - d1) < 0.001` float 比较, 原样保留
    #[allow(clippy::float_cmp)]
    {
        if (d0 - d1).abs() < 0.001 {
            return None;
        }
    }
    let lower_is_better = rule.is_lower_better();
    Some(
        (if d0 > d1 {
            if lower_is_better { fm1_name } else { fm0_name }
        } else if lower_is_better {
            fm0_name
        } else {
            fm1_name
        })
        .to_string(),
    )
}

/// Java `loadFmLines` (CompactComparisonWindow.java:460-491): FMLoader 标准链路
/// 加载, MISSING 时按物理文件直读回退; 返回 fmdata 的非空白行。
pub fn load_fm_lines(name: Option<&str>) -> Vec<String> {
    let name = match name {
        Some(n) if !java_trim(n).is_empty() => n,
        _ => return Vec::new(),
    };
    // P5 收编: 优先 FMLoader 标准链路（机型名 → 中央文件 → 物理文件 → 全量解析）。
    // 本方法只用 fmdata 文本 —— finalizeLoading 只清原始 data 文本，刻意保留 fmdata
    // （FMDataOverlay 依赖），故 READY 句柄的 blkx 直接可用，无需自行解析
    let handle = loader::load(Some(name));
    let fmdata: String = if handle.has_fm() {
        handle.fmdata.and_then(|b| b.fmdata).unwrap_or_default()
    } else {
        // 名字空间差异回退: name 是 fm/ 物理文件名（连字符, 如 a-10c）, 中央机型名
        // 是下划线（a_10c）——少数不同名机型 FMLoader 判 MISSING, 按物理文件直读
        match fallback_physical_file(name) {
            Some(f) => match FmData::parse_named_json(&f.to_string_lossy(), name) {
                Ok(b) => b.fmdata.unwrap_or_default(),
                Err(_) => {
                    // Java 构造器内 catch 产出 valid=false 对象 (fmdata=noblkx),
                    // 行为保真: 以 noblkx 文本顶位 (无冒号 → 解析段自然过滤)
                    Lang::init_lang().noblkx.to_string()
                }
            },
            // fmdata=noblkx (Blkx.java:1671 构造器头部赋值) — 与上方解析失败分支
            // 同文本顶位: load_fm_lines 返回 noblkx 两行 (无冒号 → 对比解析段全滤,
            // DTO 可观察结果不变; 直接消费行内容的调用点也与 Java 一致)
            None => Lang::init_lang().noblkx.to_string(),
        }
    };

    // fmdata is string.
    if fmdata.is_empty() {
        return Vec::new();
    }
    fmdata
        .split('\n')
        .filter(|s| !java_trim(s).is_empty())
        .map(String::from)
        .collect()
}

/// buildCopyText (CompactComparisonWindow.java:396-432): COPY 按钮文本
fn build_copy_text(
    fm0_name: &str,
    fm1_name: &str,
    single_mode: bool,
    structure: &[DisplayItem],
    map0: &HashMap<String, String>,
    map1: &HashMap<String, String>,
) -> String {
    let mut sb = String::new();

    // Header
    if single_mode {
        sb.push_str(&format!("========== Aircraft Data: {fm0_name} ==========\n\n"));
    } else {
        sb.push_str(&format!(
            "========== Comparison: {fm0_name} vs {fm1_name} ==========\n\n"
        ));
    }

    // Body
    for item in structure {
        if item.is_header {
            sb.push_str(&format!("---------- {} ----------\n", item.text));
        } else {
            let k = &item.text;
            let v0 = map0.get(k).map(String::as_str);
            let v1 = map1.get(k).map(String::as_str);

            if single_mode {
                sb.push_str(&format!("{}: {}\n", k, v0.unwrap_or("-")));
            } else {
                sb.push_str(&format!("{}: {} vs {}", k, v0.unwrap_or("-"), v1.unwrap_or("-")));
                if let Some(w) = winner_name(k, v0, v1, fm0_name, fm1_name) {
                    sb.push_str(&format!("  [{w} +]"));
                }
                sb.push('\n');
            }
        }
    }
    sb
}

/// 对比窗口数据装配 (initUI 的数据段, CompactComparisonWindow.java:186-275)
pub fn comparison_data_impl(fm0_name: &str, fm1_name: Option<&str>) -> ComparisonDataDto {
    let single_mode = match fm1_name {
        None => true,
        Some(s) => s.is_empty(),
    };

    // Get Data
    let lines0 = load_fm_lines(Some(fm0_name));
    let lines1 = load_fm_lines(fm1_name);

    // Parse Data into Maps and Structure
    let lines1_safe = lines1;
    let (structure, map0, map1) = build_structure(&lines0, &lines1_safe);

    // Render (行清单: 标题行/属性行 + 胜负符号)
    let mut rows: Vec<ComparisonRowDto> = Vec::new();
    for item in &structure {
        if item.is_header {
            rows.push(ComparisonRowDto {
                is_header: true,
                text: item.text.clone(),
                value0: None,
                value1: None,
                win: 0,
                symbol: String::new(),
            });
            continue;
        }
        let k = &item.text;
        let v0 = map0.get(k).map(String::as_str);
        let v1 = map1.get(k).map(String::as_str);
        // If v0 is null, it means it's a key only in FM1.
        // If v1 is null, it means it's a key only in FM0 (or Single Mode).
        // (原注释保留)
        let disp0 = v0.unwrap_or("-").to_string();
        let win = if single_mode {
            0 // 单机模式无胜负 (Java addComparisonRow 的 !singleMode 守卫)
        } else {
            row_win(k, &disp0, v1.unwrap_or("-"), single_mode)
        };
        let sym = if win == -1 {
            "▶"
        } else if win == 1 {
            "◀"
        } else {
            "-"
        };
        rows.push(ComparisonRowDto {
            is_header: false,
            text: k.clone(),
            value0: Some(disp0),
            value1: if single_mode { None } else { Some(v1.unwrap_or("-").to_string()) },
            win,
            symbol: sym.to_string(),
        });
    }

    let title = if single_mode {
        format!("Aircraft Data: {fm0_name}")
    } else {
        format!("Comparison: {fm0_name} vs {}", fm1_name.unwrap_or(""))
    };
    let copy_text = build_copy_text(
        fm0_name,
        fm1_name.unwrap_or(""),
        single_mode,
        &structure,
        &map0,
        &map1,
    );

    ComparisonDataDto {
        fm0_name: fm0_name.to_string(),
        fm1_name: if single_mode { None } else { Some(fm1_name.unwrap_or("").to_string()) },
        single_mode,
        title,
        rows,
        copy_text,
    }
}

// =====================================================================
// 2. 功率曲线窗口数据 (PowerCurveWindow.java)
// =====================================================================

// Chart dimensions (数据相关常量; 窗口尺寸/边距是像素域归前端)
// Maximum altitude for chart display (m)
const MAX_DISPLAY_ALT: i32 = 10000;
// Altitude step for curve generation (m)
const ALT_STEP: i32 = 25;

/// Java `loadFuelModification` (PowerCurveWindow.java:355-378): 尝试从中央文件
/// 载入燃油改装修正。P5 后仅回退路径调用 — 标准链路的燃油修正已由 FMLoader
/// 融入句柄的 compressorStages。路径统一走 `FMDataPaths::fmDir()` 拼装: 中央
/// 文件在 flightmodels 根目录, 物理文件在其 fm/ 子目录。
fn load_fuel_modification(fm_name: &str) -> Option<FuelModification> {
    // Central file (blkx→json 迁移终态: 只 .json)
    let cf = data_paths::fm_dir().join(format!("{fm_name}.json"));
    if !cf.exists() {
        return None;
    }
    // Central file exists but failed to parse — continue without fuel mod
    // (原 catch(Exception) 分支注释语义)
    let parsed = std::fs::read_to_string(&cf).ok().and_then(|data| {
        serde_json::from_str::<serde_json::Value>(&data)
            .ok()
            .map(|root| extract_fuel_modifications_json(&root))
    });
    match parsed {
        Some(mod_) => {
            if mod_.r#type != FuelType::None {
                logger::info("PowerCurveWindow", &format!("Fuel modification: {}", mod_.r#type));
            }
            Some(mod_)
        }
        None => {
            logger::debug("PowerCurveWindow", "Failed to parse Central file");
            None
        }
    }
}

/// 错误形态的 CurveData (Java: powerCurve=null/max=min=peak=0/points 空)
fn error_curve(fm_name: &str, message: String) -> PowerCurveDto {
    PowerCurveDto {
        fm_name: fm_name.to_string(),
        valid: false,
        power_curve: Vec::new(),
        alt_step: ALT_STEP,
        max_display_alt: MAX_DISPLAY_ALT,
        max_power: 0.0,
        min_power: 0.0,
        peak_altitude: 0,
        inflection_points: Vec::new(),
        error_message: Some(message),
    }
}

/// Java `loadSingleCurve` (PowerCurveWindow.java:234-315): 载入单个 FM 并生成
/// 功率-高度曲线。
///
/// P5 收编: 优先走 `FMLoader::load` 标准链路 (中央文件 → fmFile 字段 → 物理文件
/// → 全量解析)。READY 活塞机句柄的 `compressorStages` 即
/// `FMPowerExtractor.extractStages(blkx, 中央文件燃油修正)` 的产物, 本函数无需
/// 再自行解析中央文件提取 fuelMod。
///
/// 名字空间差异回退: fmName 来自 fm/ 物理文件目录列表 (连字符命名, 如 a-10c),
/// 而 FMLoader 按机型名 (中央文件名, 下划线命名, 如 a_10c) 查找 — 此时回退按
/// 物理文件直读 (行为与收编前一致)。
pub fn load_single_curve(fm_name: &str, wep_mode: bool, speed_kmh: i32) -> PowerCurveDto {
    // ---- 第一优先: FMLoader 标准链路（机型名 → 中央文件 → 物理文件）----
    let handle = loader::load(Some(fm_name));
    let (fmdata, stages) = if handle.has_fm() {
        // 活塞机句柄携带 extractStages 产物（已融入中央文件燃油修正）；
        // 喷气机 compressorStages 为 null → "不是活塞引擎" (Java :246-250)
        if handle.compressor_stages.is_none() {
            return error_curve(fm_name, format!("{fm_name} 不是活塞引擎"));
        }
        (handle.fmdata.unwrap(), handle.compressor_stages)
    } else {
        // ---- 回退: 按物理文件名直读（连字符机型, 见方法注释; JSON 优先,
        //      过渡期回落 blkx 文本链 — 对拍全绿观察期后收窄为 .json）----
        let Some(f) = fallback_physical_file(fm_name) else {
            return error_curve(fm_name, format!("找不到FM文件: {fm_name}"));
        };
        let parsed = FmData::parse_named_json(&f.to_string_lossy(), fm_name);
        match parsed {
            Ok(b) => {
                // Check if piston engine
                if !is_piston_engine(Some(&b)) {
                    return error_curve(fm_name, format!("{fm_name} 不是活塞引擎"));
                }
                // Try to load Central file for fuel modifications (回退路径下同名
                // 中央文件通常不存在，fuelMod 为 null，与收编前行为一致)
                let fuel_mod = load_fuel_modification(fm_name);
                let stages = extract_stages_with_fuel(Some(&b), fuel_mod.as_ref());
                (b, stages)
            }
            Err(_) => {
                // Java 构造器失败产出 valid=false 对象 (compNumSteps=0) →
                // isPistonEngine 判 false → "不是活塞引擎" 错误串, 行为保真
                return error_curve(fm_name, format!("{fm_name} 不是活塞引擎"));
            }
        }
    };

    let Some(stages) = stages else {
        // 喷气机句柄 stages=None (此分支标准链路已被上方 is_none 提前拦截, 回退
        // 路径 extractStages 对非活塞返回 None — 与 Java `stages == null` 同位)
        return error_curve(fm_name, format!("无法提取 {fm_name} 的发动机参数"));
    };
    if stages.is_empty() {
        return error_curve(fm_name, format!("无法提取 {fm_name} 的发动机参数"));
    }

    // Generate power curve (0m to 10000m)
    let mut power_curve =
        generate_power_curve_advanced(&stages, wep_mode, speed_kmh as f64, true, 15.0, ALT_STEP);

    // Multi-engine aircraft: multiply each point by engine count
    if fmdata.engine_num > 1 {
        // Java double * int 提升 double → as f64 (§2.4)
        for p in power_curve.iter_mut() {
            *p *= fmdata.engine_num as f64;
        }
    }

    // Find maximum/minimum power and peak altitude
    let max_alt_idx = MAX_DISPLAY_ALT / ALT_STEP;
    let mut max_power = 0.0f64;
    let mut min_power = f64::MAX;
    let mut peak_altitude = 0i32;

    let mut i = 0i32;
    while i <= max_alt_idx && (i as usize) < power_curve.len() {
        if power_curve[i as usize] > max_power {
            max_power = power_curve[i as usize];
            peak_altitude = i * ALT_STEP;
        }
        if power_curve[i as usize] < min_power {
            min_power = power_curve[i as usize];
        }
        i += 1;
    }

    // Identify inflection points
    let inflection_points = identify_inflection_points_for_curve(&power_curve, max_power);

    PowerCurveDto {
        fm_name: fm_name.to_string(),
        valid: true,
        power_curve,
        alt_step: ALT_STEP,
        max_display_alt: MAX_DISPLAY_ALT,
        max_power,
        min_power,
        peak_altitude,
        inflection_points,
        error_message: None,
    }
}

/// Returns true if any point in the list is within {@code minSepM} meters of {@code altM}.
/// (Java tooCloseToList, PowerCurveWindow.java:537-543)
fn too_close_to_list(alt_m: i32, min_sep_m: i32, list: &[InflectionPointDto]) -> bool {
    for p in list {
        if (p.altitude_m - alt_m).abs() < min_sep_m {
            return true;
        }
    }
    false
}

/// Java `identifyInflectionPointsForCurve` (PowerCurveWindow.java:397-535):
/// 直接按曲线几何形态检测拐点 — 局部极大 (峰) / 局部极小 (谷, 级间过渡) /
/// 凹凸翻转 (斜率 kink)。
pub fn identify_inflection_points_for_curve(
    power_curve: &[f64],
    max_power: f64,
) -> Vec<InflectionPointDto> {
    let mut result = Vec::new();

    let max_idx = (MAX_DISPLAY_ALT / ALT_STEP).min(power_curve.len() as i32 - 1);
    if max_idx < 6 {
        return result;
    }

    let min_sep_m = 300; // minimum separation between markers
    let noise_threshold = max_power * 0.005; // 0.5% of max power
    let hw = 4; // ±100m window (4 × 25m step)

    // ========== Phase 1: Collect peaks and valleys separately ==========
    // Java double[]{altM, power} → (i32, f64) 元组
    let mut peak_candidates: Vec<(i32, f64)> = Vec::new();
    let mut valley_candidates: Vec<(i32, f64)> = Vec::new();

    let mut i = hw;
    while i <= max_idx - hw {
        let left = power_curve[(i - hw) as usize];
        let center = power_curve[i as usize];
        let right = power_curve[(i + hw) as usize];

        let left_slope = center - left;
        let right_slope = right - center;

        let slope_sign_change_peak = left_slope > 0.0 && right_slope < 0.0;
        let slope_sign_change_valley = left_slope < 0.0 && right_slope > 0.0;

        let peak_prominence = center - left.min(right);
        let valley_prominence = left.max(right) - center;

        if slope_sign_change_peak && peak_prominence > noise_threshold {
            let mut best_idx = i;
            for j in i - hw..=i + hw {
                if power_curve[j as usize] > power_curve[best_idx as usize] {
                    best_idx = j;
                }
            }
            let alt_m = best_idx * ALT_STEP;
            let mut too_close = false;
            for prev in &peak_candidates {
                if (prev.0 - alt_m).abs() < min_sep_m {
                    too_close = true;
                    break;
                }
            }
            if !too_close {
                peak_candidates.push((alt_m, power_curve[best_idx as usize]));
            }
        } else if slope_sign_change_valley && valley_prominence > noise_threshold {
            let mut best_idx = i;
            for j in i - hw..=i + hw {
                if power_curve[j as usize] < power_curve[best_idx as usize] {
                    best_idx = j;
                }
            }
            let alt_m = best_idx * ALT_STEP;
            let mut too_close = false;
            for prev in &valley_candidates {
                if (prev.0 - alt_m).abs() < min_sep_m {
                    too_close = true;
                    break;
                }
            }
            if !too_close {
                valley_candidates.push((alt_m, power_curve[best_idx as usize]));
            }
        }
        i += 1;
    }

    // Sort by altitude (ascending) — Java Double.compare(a[0], b[0]); 高度域为
    // 精确整数, i32 全序逐位一致
    peak_candidates.sort_by_key(|c| c.0);
    valley_candidates.sort_by_key(|c| c.0);

    // ========== Phase 2: Add valleys (stage transitions) ==========
    for (alt_m, power) in &valley_candidates {
        let mut peaks_below = 0;
        for peak in &peak_candidates {
            if peak.0 < *alt_m {
                peaks_below += 1;
            }
        }

        let from_stage = 1.max(peaks_below);
        let to_stage = from_stage + 1;

        let label = format!("{from_stage}→{to_stage}档");
        result.push(InflectionPointDto {
            kind: "valley".to_string(),
            label,
            altitude_m: *alt_m,
            power: *power,
        });
    }

    // ========== Phase 3: Add peaks (critical altitudes) ==========
    let mut stage_num = 1;
    for (alt_m, power) in &peak_candidates {
        if too_close_to_list(*alt_m, min_sep_m, &result) {
            stage_num += 1;
            continue;
        }

        let label = format!("{stage_num}档");
        result.push(InflectionPointDto {
            kind: "peak".to_string(),
            label,
            altitude_m: *alt_m,
            power: *power,
        });
        stage_num += 1;
    }

    // ========== Phase 4: Detect slope kinks ==========
    let kink_half_window = 4;
    let avg_slope = (power_curve[max_idx as usize] - power_curve[0]).abs() / (max_idx * ALT_STEP) as f64;
    let kink_threshold = (avg_slope * 2.5).max(0.08);

    let mut i = kink_half_window;
    while i <= max_idx - kink_half_window {
        if too_close_to_list(i * ALT_STEP, min_sep_m, &result) {
            i += 1;
            continue;
        }

        let left_slope = (power_curve[i as usize] - power_curve[(i - kink_half_window) as usize])
            / (kink_half_window * ALT_STEP) as f64;
        let right_slope = (power_curve[(i + kink_half_window) as usize] - power_curve[i as usize])
            / (kink_half_window * ALT_STEP) as f64;

        let slope_change = (right_slope - left_slope).abs();
        let same_slope_direction = left_slope * right_slope >= 0.0;
        let is_peak_or_valley = !same_slope_direction;

        if !is_peak_or_valley && slope_change > kink_threshold {
            let mut best_idx = i;
            let mut best_change = slope_change;
            // 条件逐轮求值, i-2 可低于 hw 使循环体不执行
            let mut j = i - 2;
            while j <= i + 2 && j >= kink_half_window && j <= max_idx - kink_half_window {
                let ls = (power_curve[j as usize]
                    - power_curve[(j - kink_half_window) as usize])
                    / (kink_half_window * ALT_STEP) as f64;
                let rs = (power_curve[(j + kink_half_window) as usize] - power_curve[j as usize])
                    / (kink_half_window * ALT_STEP) as f64;
                let sc = (rs - ls).abs();
                if sc > best_change {
                    best_change = sc;
                    best_idx = j;
                }
                j += 1;
            }

            if !too_close_to_list(best_idx * ALT_STEP, min_sep_m, &result) {
                result.push(InflectionPointDto {
                    kind: "kink".to_string(),
                    label: "Kink".to_string(),
                    altitude_m: best_idx * ALT_STEP,
                    power: power_curve[best_idx as usize],
                });
            }
        }
        i += 1;
    }

    result
}

/// Java `calculateDisplayRange` (PowerCurveWindow.java:320-342): 双曲线合并显示域
fn calculate_display_range(curve0: &PowerCurveDto, curve1: &Option<PowerCurveDto>) -> (f64, f64) {
    let mut combined_max = 0.0f64;
    let mut combined_min = f64::MAX;

    if curve0.valid {
        combined_max = combined_max.max(curve0.max_power);
        combined_min = combined_min.min(curve0.min_power);
    }
    if let Some(c1) = curve1 {
        if c1.valid {
            combined_max = combined_max.max(c1.max_power);
            combined_min = combined_min.min(c1.min_power);
        }
    }

    // Handle case where no valid curves
    #[allow(clippy::float_cmp)]
    if combined_min == f64::MAX {
        combined_min = 0.0;
        combined_max = 1000.0;
    }

    // Round to nearest 100hp for clean grid lines
    (
        (combined_max / 100.0).ceil() * 100.0,
        (combined_min / 100.0).floor() * 100.0,
    )
}

/// Java `buildErrorMessage` (PowerCurveWindow.java:649-670)
fn build_error_message(curve0: &PowerCurveDto, curve1: &Option<PowerCurveDto>) -> Option<String> {
    let has_fm0 = curve0.valid;
    let has_fm1 = curve1.as_ref().map(|c| c.valid).unwrap_or(false);

    if !has_fm0 && !has_fm1 {
        // Both failed
        let mut sb = String::new();
        if let Some(e) = &curve0.error_message {
            sb.push_str(e);
        }
        if let Some(c1) = curve1 {
            if let Some(e) = &c1.error_message {
                if !sb.is_empty() {
                    sb.push_str(" | ");
                }
                sb.push_str(e);
            }
        }
        return if sb.is_empty() { Some("无法加载功率曲线".to_string()) } else { Some(sb) };
    } else if !has_fm0 && curve0.error_message.is_some() {
        return curve0.error_message.clone();
    } else if !has_fm1 && curve1.is_some() && curve1.as_ref().unwrap().error_message.is_some() {
        return curve1.as_ref().unwrap().error_message.clone();
    }
    None
}

/// Java `loadPowerCurves` (PowerCurveWindow.java:199-212) + 构造器的单双模式裁决
pub fn power_curve_data_impl(
    fm0_name: &str,
    fm1_name: Option<&str>,
    speed_kmh: i32,
    wep_mode: bool,
) -> PowerCurveDataDto {
    // Treat fm1Name == fm0Name as single curve mode (构造器 :183)
    let fm1_name = match fm1_name {
        Some(n) if !n.is_empty() && n != fm0_name => Some(n),
        _ => None,
    };

    // Load FM0 (primary curve)
    let curve0 = load_single_curve(fm0_name, wep_mode, speed_kmh);

    // Load FM1 (secondary curve) if in dual mode
    let curve1 = fm1_name.map(|n| load_single_curve(n, wep_mode, speed_kmh));

    // Calculate combined display range
    let (display_max_power, display_min_power) = calculate_display_range(&curve0, &curve1);
    let error_message = build_error_message(&curve0, &curve1);
    // Java isDualMode: fm1Name != null && curveData1 != null (不看 valid)
    let dual_mode = fm1_name.is_some() && curve1.is_some();

    PowerCurveDataDto {
        fm0_name: fm0_name.to_string(),
        fm1_name: fm1_name.map(str::to_string),
        dual_mode,
        speed_kmh,
        wep_mode,
        curve0,
        curve1,
        display_max_power,
        display_min_power,
        error_message,
    }
}

// =====================================================================
// 4. FM 机型列表 (GridSelectorDialog.java)
// =====================================================================

/// Java `loadPlanes` (GridSelectorDialog.java:151-163): fm/ 物理文件目录列表
/// (机型选择搜索下拉的数据源)。
pub fn load_planes() -> Vec<String> {
    // P5: 路径收编到 FMDataPaths（fm/ 物理文件目录 = flightmodels 根下 "fm" 子目录）
    let dir = data_paths::fm_dir().join("fm");
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            // 只收 .json (blkx→json 迁移: data/ 为双格式同名并存, 不过滤会
            // 每机型重复两项)
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".json") {
                continue;
            }
            // Strip extensions —— 用 FileUtils 统一按最后一个 '.' 剥后缀。
            // 原来的 .replace(".blk","") 链会把小写 ".blkx" 剥成残留尾字母 "x"
            // （如 "a_4h.blkx" → "a_4hx"），而 fmdata 解包产物全为小写 .blkx
            // (原注释保留)
            if let Some(stripped) = get_file_name_no_ex(Some(&name)) {
                names.push(stripped.to_string());
            }
        }
    }
    // Java Arrays.stream(...).sorted() — String 自然序 (UTF-16 码元); 域内文件名
    // 为 ASCII, Rust sort() (字节序) 逐位一致 (§2.1)
    names.sort();
    names.dedup();
    names
}

// =====================================================================
// tauri command 薄壳 (直接计算, 不经主线程 dispatcher — 见模块头分工说明)
//
// 备案 (审查 W3 — 接受直算, 不下放 blocking 池): comparison/power_curve/fm_list
// 的重计算 (Blkx 全量解析/双曲线采样/目录扫描) 直接跑在命令执行上下文, 双窗
// 并发查询理论可占住 async worker。实测两种下放实现均不可用: 引用
// `tauri::async_runtime::spawn_blocking` 或手写 std Future 桥 (线程 + waker)
// 都会向 cargo test 二进制拖入 comctl32 v6 依赖 (`TaskDialogIndirect`, 无 SxS
// manifest) → 加载即 STATUS_ENTRYPOINT_NOT_FOUND, 测试全灭 (二分定位实锤:
// 去掉对下放桥的引用即恢复干净导入表)。解锁路径: 为测试二进制嵌 common-controls
// v6 manifest (build.rs) 后再启用下放; 当前单窗使用无观测面, 按 reviewer 裁决
// "备案接受"。
// =====================================================================

/// 对比窗口数据 (CompactComparisonWindow: displayStructure + dataMap0/1 + 胜负)
#[tauri::command]
pub async fn comparison_data(
    fm0: String,
    fm1: Option<String>,
) -> Result<serde_json::Value, String> {
    to_json(&comparison_data_impl(&fm0, fm1.as_deref()))
}

/// 功率曲线窗口数据 (PowerCurveWindow.loadPowerCurves; fm1 空/==fm0 = 单曲线)
#[tauri::command]
pub async fn power_curve_data(
    fm0: String,
    fm1: Option<String>,
    speed_kmh: i32,
    wep: bool,
) -> Result<serde_json::Value, String> {
    to_json(&power_curve_data_impl(&fm0, fm1.as_deref(), speed_kmh, wep))
}

/// FM 机型列表 (GridSelectorDialog 搜索下拉; fm/ 物理文件目录)。
///
/// 与 [`crate::commands::get_fm_list`] 双命令并存, 接线按窗口对号不可混用:
/// 设置页 FMLIST 行走 get_fm_list (mpsc → 主线程 dispatcher, 对位 Java
/// FMListRowRenderer); 对比/功率曲线窗口的机型下拉走本命令 (直连 vm-core,
/// 对位 GridSelectorDialog.loadPlanes)。当前两者数据面同源 (fm/ 目录 FileUtils
/// 剥后缀全枚举 + 排序), 差在通道 — dispatcher 版受主线程泵节流, 直连版即时;
/// 未来演化路径也不同, 前端用错源会造成两窗口列表不一致。
#[tauri::command]
pub async fn fm_list() -> Result<serde_json::Value, String> {
    to_json(&load_planes())
}

// =====================================================================
// Tests — 数据面单测: 纯函数 (解析/合并/胜负/拐点) 无文件依赖; 真机腿用
// 项目内 data/ 的 spitfire blkx, data 缺失环境按 realtests 先例 SKIP+真因。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// 真机数据根注入 (data/ 缺失 → false, 调用方打印真因后 return)
    fn ensure_real_data() -> bool {
        // vm-webui 位于 rust/crates/vm-webui → 仓库根 = ../../.. (realtests 同款)
        let root = format!("{}/../../../data", env!("CARGO_MANIFEST_DIR"));
        if !std::path::Path::new(&root).join("aces/gamedata/flightmodels").exists() {
            return false;
        }
        data_paths::set_data_root(&root);
        true
    }

    // ---- 纯函数: PROP_PATTERN 解析 ----

    #[test]
    fn prop_line_冒号分界与列表值() {
        // Java javadoc 示例
        assert_eq!(
            parse_prop_line("空重(kg): 4644.0"),
            Some(("空重(kg)".to_string(), "4644.0".to_string()))
        );
        assert_eq!(
            parse_prop_line("临界速度(km/h): [144, 1167]"),
            Some(("临界速度(km/h)".to_string(), "[144, 1167]".to_string()))
        );
        // 多冒号: [^:]+ 无法跨越 ':', 第一个冒号分界
        assert_eq!(
            parse_prop_line("a: b: c"),
            Some(("a".to_string(), "b: c".to_string()))
        );
        // 冒号前 ≥1 字符 (Java ([^:]+) 空匹配失败)
        assert_eq!(parse_prop_line(": x"), None);
        // 无冒号整行丢弃
        assert_eq!(parse_prop_line("没有冒号的行"), None);
        // 前置空白冒号 → k trim 为空串 (Java 仍入 map, 保真)
        assert_eq!(
            parse_prop_line(" : 1.0"),
            Some(("".to_string(), "1.0".to_string()))
        );
    }

    // ---- 纯函数: 结构合并 ----

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn 结构合并_表头与fm1独有键插入() {
        let l0 = lines(&[
            "------ 基本参数 ------",
            "空重(kg): 1000.0",
            "最大燃油重量(kg): 500.0",
            "翼展效率: 0.9",
        ]);
        let l1 = lines(&[
            "------ 其他 ------", // merge 段跳过表头 (避免重复)
            "空重(kg): 1200.0",
            "独有属性A: 1",
            "翼展效率: 0.8",
            "独有属性B: 2",
        ]);
        let (structure, map0, map1) = build_structure(&l0, &l1);

        // 结构: header + 3 键 + FM1 独有两键按 lastMatch 插入:
        // 独有属性A 出现时最近命中是 空重(kg)(idx1) → 插在 idx2 (最大燃油重量
        // 之前); 独有属性B 出现时最近命中是 翼展效率(已到尾) → 尾部追加
        let texts: Vec<(bool, &str)> = structure
            .iter()
            .map(|d| (d.is_header, d.text.as_str()))
            .collect();
        assert_eq!(
            texts,
            vec![
                (true, "基本参数"),
                (false, "空重(kg)"),
                (false, "独有属性A"),
                (false, "最大燃油重量(kg)"),
                (false, "翼展效率"),
                (false, "独有属性B"),
            ]
        );
        assert_eq!(map0.get("空重(kg)").map(String::as_str), Some("1000.0"));
        assert_eq!(map1.get("空重(kg)").map(String::as_str), Some("1200.0"));
        assert!(!map0.contains_key("独有属性A"));
        assert_eq!(map1.get("独有属性B").map(String::as_str), Some("2"));
    }

    #[test]
    fn 结构合并_fm0为空时fm1键全量追加() {
        // lastMatchIndex 初值 -1: 结构空时 size-1 = -1, -1 < -1 假 → 尾部追加
        let l1 = lines(&["a: 1", "b: 2"]);
        let (structure, map0, map1) = build_structure(&[], &l1);
        assert_eq!(structure.len(), 2);
        assert_eq!(structure[1].text, "b");
        assert!(map0.is_empty());
        assert_eq!(map1.len(), 2);
    }

    // ---- 纯函数: 胜负规则接线 ----

    #[test]
    fn 胜负规则_接线_vm_core规则族() {
        // 空重: lower better — v0 重 → 右胜 (win=1)
        assert_eq!(row_win("空重(kg)", "5000.0", "4000.0", false), 1);
        // v0 轻 → 左胜
        assert_eq!(row_win("空重(kg)", "3000.0", "4000.0", false), -1);
        // 最大燃油重量: higher better
        assert_eq!(row_win("最大燃油重量(kg)", "800.0", "500.0", false), -1);
        // 临界速度: ListIndexRule(1) — "[144, 1167]" 取 1167, higher better
        assert_eq!(row_win("临界速度(km/h)", "[144, 1167]", "[144, 1300]", false), 1);
        // 允许过载: MultiListIndexRule(0,1) — "[8.5, -4.2], [10.1, -5.3]" 取 -4.2
        assert_eq!(
            row_win("允许过载(满/半油)", "[8.5, -4.2], [10.1, -5.3]", "[7.0, -3.0], [9.0, -4.0]", false),
            1
        );
        // 主阻力面积因数: Lambda 取 '/' 后第二个数, lower better
        assert_eq!(row_win("主阻力面积因数及加速度系数", "0.25 / 0.35", "0.20 / 0.30", false), 1);
        // 散热/油冷器: SLASH_BOTH 求和, lower better — 0.5+0.6=1.1 vs 0.4+0.5=0.9
        assert_eq!(row_win("散热/油冷器阻力系数", "0.5 / 0.6", "0.4 / 0.5", false), 1);
        // |d0-d1| <= 0.001 → 平
        assert_eq!(row_win("空重(kg)", "4000.0005", "4000.0", false), 0);
        // 缺键补 "-" 后 extract 失败 → 平 (Java 调用点形态)
        assert_eq!(row_win("空重(kg)", "-", "4000.0", false), 0);
        // 无规则属性 → 平
        assert_eq!(row_win("无规则属性", "1.0", "2.0", false), 0);
        // 单机模式恒平 (Java !singleMode 守卫)
        assert_eq!(row_win("空重(kg)", "5000.0", "4000.0", true), 0);
    }

    #[test]
    fn copy文本_胜负方名() {
        let winner = winner_name("空重(kg)", Some("5000.0"), Some("4000.0"), "fm0", "fm1");
        assert_eq!(winner.as_deref(), Some("fm1")); // 右侧轻 → fm1 胜
        // 缺键 (v1 None) → None (Java null 判在规则前)
        assert_eq!(winner_name("空重(kg)", Some("5000.0"), None, "fm0", "fm1"), None);
        assert_eq!(winner_name("无规则", Some("1"), Some("2"), "fm0", "fm1"), None);
    }

    // ---- 纯函数: 拐点检测 ----

    /// 双峰一谷合成曲线: 0..=80 升 (100→200), 80..=160 降 (→150), 160..=240 升 (→180),
    /// 240.. 缓降 — 峰@2000m 谷@4000m 峰@6000m。
    /// 注: 峰后必须严格下降 (Java 检测要求 rightSlope < 0, 平坦尾部不算峰)
    #[test]
    fn 拐点检测_双峰一谷() {
        let curve: Vec<f64> = (0..=400)
            .map(|i| match i {
                i if i <= 80 => 100.0 + i as f64 * 1.25,
                i if i <= 160 => 200.0 - (i - 80) as f64 * 0.625,
                i if i <= 240 => 150.0 + (i - 160) as f64 * 0.375,
                _ => 180.0 - (i - 240) as f64 * 0.05,
            })
            .collect();
        let pts = identify_inflection_points_for_curve(&curve, 200.0);
        let peaks: Vec<&InflectionPointDto> = pts.iter().filter(|p| p.kind == "peak").collect();
        let valleys: Vec<&InflectionPointDto> = pts.iter().filter(|p| p.kind == "valley").collect();
        assert_eq!(peaks.len(), 2, "双峰: {pts:?}");
        assert_eq!(valleys.len(), 1, "一谷: {pts:?}");
        // Phase 3 按高度升序编档号
        assert_eq!(peaks[0].altitude_m, 2000);
        assert_eq!(peaks[0].label, "1档");
        assert_eq!(peaks[1].altitude_m, 6000);
        assert_eq!(peaks[1].label, "2档");
        // 谷: 下方 1 峰 → 1→2档
        assert_eq!(valleys[0].altitude_m, 4000);
        assert_eq!(valleys[0].label, "1→2档");
        assert!((valleys[0].power - 150.0).abs() < 1e-9);
    }

    /// 同向斜率突变 (陡升→缓升) 无峰谷 → 只有 Kink 标注
    #[test]
    fn 拐点检测_斜率拐点() {
        let curve: Vec<f64> = (0..=400)
            .map(|i| match i {
                i if i <= 100 => i as f64 * 5.0, // 0.2 hp/m
                _ => 500.0 + (i - 100) as f64 * 0.5, // 0.02 hp/m
            })
            .collect();
        let pts = identify_inflection_points_for_curve(&curve, 650.0);
        assert!(
            pts.iter().all(|p| p.kind != "peak" && p.kind != "valley"),
            "同向曲线不应有峰谷: {pts:?}"
        );
        let kinks: Vec<&InflectionPointDto> = pts.iter().filter(|p| p.kind == "kink").collect();
        assert!(!kinks.is_empty(), "应有 Kink: {pts:?}");
        // 拐点位于 100 档 (2500m) 邻域
        assert!((kinks[0].altitude_m - 2500).abs() <= 100, "{kinks:?}");
        assert_eq!(kinks[0].label, "Kink");
    }

    /// 短曲线 (maxIdx < 6) 直接空结果 (Java 守卫)
    #[test]
    fn 拐点检测_短曲线守卫() {
        let pts = identify_inflection_points_for_curve(&[1.0, 2.0, 3.0], 3.0);
        assert!(pts.is_empty());
    }

    // ---- 真机腿 (data/ 缺失 SKIP) ----

    #[test]
    fn 真机_fm列表_物理文件目录() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (fm_list 无数据源)");
            return;
        }
        let planes = load_planes();
        assert!(planes.len() > 100, "fm/ 目录应有千级机型: {}", planes.len());
        assert!(planes.contains(&"spitfire_f24".to_string()), "应含 spitfire_f24");
        assert!(planes.contains(&"a-10c".to_string()), "应含连字符机型 a-10c");
        // 已排序
        let mut sorted = planes.clone();
        sorted.sort();
        assert_eq!(planes, sorted);
    }

    #[test]
    fn 真机_对比_单机模式() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (comparison_data 无数据源)");
            return;
        }
        let dto = comparison_data_impl("spitfire_f24", None);
        assert!(dto.single_mode);
        assert_eq!(dto.fm1_name, None);
        assert_eq!(dto.title, "Aircraft Data: spitfire_f24");
        assert!(!dto.rows.is_empty(), "fmdata 应产出行清单");
        // 表头行存在 ("------" 分节)
        assert!(
            dto.rows.iter().any(|r| r.is_header),
            "fmdata 应含 ------ 分节: {:?}",
            &dto.rows[..5.min(dto.rows.len())]
        );
        // fmdata 首行 = "FM文件: <fmfile> - <版本\n>" — bFmVersion 模板尾无换行,
        // 版本串 (version 文件 readLine 拼接) 自带 \n, 故首行在版本处断行。
        // (旧断言 "首行与空重粘一行" 是 version 读不到的环境假象: get_version
        // 曾硬编码 ./data 相对 cwd, cargo 测试 cwd 下读不到 → 空版本串粘住
        // 下一行; 走 fm_data_paths 后与 Java 生产一致, 空重独立成行)
        let fmfile = dto
            .rows
            .iter()
            .find(|r| !r.is_header && r.text == "FM文件")
            .expect("应含 FM文件 首行");
        assert!(
            fmfile.value0.as_deref().unwrap_or("").contains("fm/spitfire_f24.blk"),
            "FM文件 行值应含物理文件相对路径"
        );
        // 规则属性存在且值为数字 (Blkx.getload 格式化产物)
        let fuel = dto
            .rows
            .iter()
            .find(|r| !r.is_header && r.text == "最大燃油重量(kg)")
            .expect("应含 最大燃油重量(kg) 行");
        assert!(fuel.value0.as_deref().unwrap_or("-").parse::<f64>().is_ok());
        // 单机模式: 全部平局, value1 恒 None
        assert!(dto.rows.iter().all(|r| r.win == 0));
        assert!(dto.rows.iter().all(|r| r.value1.is_none()));
        // copy 文本单机形态
        assert!(dto.copy_text.starts_with("========== Aircraft Data: spitfire_f24"));
        assert!(dto.copy_text.contains("空重(kg): "));
    }

    #[test]
    fn 真机_对比_双机模式_胜负与合并() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (comparison_data 无数据源)");
            return;
        }
        let dto = comparison_data_impl("spitfire_f24", Some("spitfire_f22"));
        assert!(!dto.single_mode);
        assert_eq!(dto.fm1_name.as_deref(), Some("spitfire_f22"));
        assert_eq!(dto.title, "Comparison: spitfire_f24 vs spitfire_f22");
        // 最大燃油重量(kg) (SimpleRule higher-better): 胜负 = 值比较的确定性推演
        let fuel = dto
            .rows
            .iter()
            .find(|r| !r.is_header && r.text == "最大燃油重量(kg)")
            .expect("应含 最大燃油重量(kg) 行");
        let w0: f64 = fuel.value0.as_deref().unwrap().parse().unwrap();
        let w1: f64 = fuel.value1.as_deref().unwrap().parse().unwrap();
        let expect = if (w0 - w1).abs() > 0.001 {
            if w0 > w1 { -1 } else { 1 }
        } else {
            0
        };
        assert_eq!(
            fuel.win, expect,
            "最大燃油重量 higher-better: v0={w0} v1={w1} win 应为 {expect}"
        );
        assert_eq!(
            fuel.symbol,
            if expect == -1 { "▶" } else if expect == 1 { "◀" } else { "-" }
        );
        // 双机 copy 文本含胜负方名
        assert!(dto.copy_text.contains("vs"));
        assert!(dto.copy_text.contains("spitfire_f2"));
        // 有胜负符号的行集合与 win!=0 一致
        for r in &dto.rows {
            if r.is_header {
                continue;
            }
            let expect_sym = if r.win == -1 {
                "▶"
            } else if r.win == 1 {
                "◀"
            } else {
                "-"
            };
            assert_eq!(r.symbol, expect_sym);
        }
    }

    #[test]
    fn 真机_功率曲线_单曲线_采样与拐点() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (power_curve_data 无数据源)");
            return;
        }
        let dto = power_curve_data_impl("spitfire_f24", None, 400, true);
        assert!(!dto.dual_mode);
        assert!(dto.curve1.is_none());
        let c0 = &dto.curve0;
        assert!(c0.valid, "spitfire_f24 应为可用活塞曲线: {:?}", c0.error_message);
        // 曲线采样点数: 0..=10000m 步 25 → 401
        assert_eq!(c0.power_curve.len(), 401);
        assert_eq!(c0.alt_step, 25);
        assert_eq!(c0.max_display_alt, 10000);
        // 峰值功率量级 (Spitfire F24 格里芬 ~2000+ hp 级)
        assert!(c0.max_power > 1000.0, "峰值功率量级: {}", c0.max_power);
        assert!(c0.min_power > 0.0);
        assert!((0..=10000).contains(&c0.peak_altitude));
        // 拐点: 增压器级 → 至少一个峰
        assert!(
            c0.inflection_points.iter().any(|p| p.kind == "peak"),
            "应有峰标注: {:?}",
            c0.inflection_points
        );
        // 显示域: ceil/floor 到百 hp, 覆盖曲线值域
        assert!(dto.display_max_power >= c0.max_power);
        assert!(dto.display_min_power <= c0.min_power);
        assert_eq!(dto.display_max_power % 100.0, 0.0);
        assert_eq!(dto.display_min_power % 100.0, 0.0);
        assert_eq!(dto.error_message, None);
    }

    #[test]
    fn 真机_功率曲线_双曲线与同名归一() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (power_curve_data 无数据源)");
            return;
        }
        let dto = power_curve_data_impl("spitfire_f24", Some("spitfire_f22"), 0, false);
        assert!(dto.dual_mode);
        let c1 = dto.curve1.as_ref().expect("双曲线模式");
        assert!(c1.valid, "spitfire_f22 应可用: {:?}", c1.error_message);
        assert_eq!(c1.power_curve.len(), 401);
        // 合并显示域覆盖两条曲线
        assert!(dto.display_max_power >= c1.max_power);
        // fm1 == fm0 → 单曲线模式保真 (Java 构造器裁决)
        let single = power_curve_data_impl("spitfire_f24", Some("spitfire_f24"), 0, false);
        assert!(!single.dual_mode);
        assert!(single.curve1.is_none());
    }

    #[test]
    fn 真机_功率曲线_名字空间回退与喷气机错误() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (power_curve_data 无数据源)");
            return;
        }
        // a-10c: fm/ 物理文件连字符命名, 中央文件是 a_10c (下划线) → FMLoader 判
        // MISSING → 回退物理直读 → 喷气机非活塞 → "不是活塞引擎"
        let dto = power_curve_data_impl("a-10c", None, 0, true);
        assert!(
            !dto.curve0.valid,
            "喷气机应判非活塞 (回退+喷气双路径): {:?}",
            dto.curve0.error_message
        );
        assert_eq!(
            dto.curve0.error_message.as_deref(),
            Some("a-10c 不是活塞引擎")
        );
        assert_eq!(dto.error_message.as_deref(), Some("a-10c 不是活塞引擎"));
        // 双失败合并信息
        let both = power_curve_data_impl("a-10c", Some("a-10a_late"), 0, true);
        let msg = both.error_message.expect("双失败应有合并错误");
        assert!(msg.contains(" | "), "双失败 ' | ' 合并: {msg}");
        // 完全不存在的机型: 回退文件也不存在 → 找不到FM文件
        let missing = power_curve_data_impl("voidmei-nonexistent", None, 0, true);
        assert_eq!(
            missing.curve0.error_message.as_deref(),
            Some("找不到FM文件: voidmei-nonexistent")
        );
    }


    #[test]
    fn load_fm_lines_物理不存在返回noblkx两行() {
        // (Blkx.java:1671), loadFmLines 返回 noblkx 两行 — 与解析失败分支同文本
        let lines = load_fm_lines(Some("voidmei-nonexistent-fm"));
        let expect: Vec<String> = Lang::init_lang()
            .noblkx
            .split('\n')
            .filter(|s| !java_trim(s).is_empty())
            .map(String::from)
            .collect();
        assert_eq!(lines, expect, "物理缺失走 noblkx 文本 (与 Java 一致)");
        assert_eq!(lines.len(), 2);
        // DTO 可观察结果等价: noblkx 行无冒号无 ------ → 解析段全滤, rows 为空
        let dto = comparison_data_impl("voidmei-nonexistent-fm", None);
        assert!(dto.rows.is_empty(), "noblkx 行不产 DTO 行: {:?}", dto.rows);
    }
}

