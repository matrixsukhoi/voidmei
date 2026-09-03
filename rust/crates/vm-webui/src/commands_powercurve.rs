//! 功率曲线窗口数据命令域: `ui/window/comparison/PowerCurveWindow.java` 的
//! Rust 数据面 (loadPowerCurves/loadSingleCurve: 曲线采样 (高度格点)/峰值谷值
//! 拐点检测/minPower/错误信息)。像素/布局归 web 前端, 本层只出数据。
//!
//! 直算模式与 W3 备案见 [`crate::commands_comparison`] 模块头 (计算面只依赖
//! vm-core, 全线程安全, 不经主线程 dispatcher); fm1 归一化与物理文件回退
//! 复用 [`crate::commands_comparison`] 的跨域共用小件。

use vm_core::base::logger;
use vm_core::fm::data::json::extract_fuel_modifications_json;
use vm_core::fm::data::{FmData, FuelModification, FuelType};
use vm_core::fm::data_paths;
use vm_core::fm::loader;
use vm_core::fm::piston_model::generate_power_curve_advanced;
use vm_core::fm::power_extractor::{extract_stages_with_fuel, is_piston_engine};

use crate::commands::to_json;
use crate::commands_comparison::{fallback_physical_file, normalize_secondary};
use crate::dto::{InflectionKind, InflectionPointDto, PowerCurveDataDto, PowerCurveDto};

// Chart dimensions (数据相关常量; 窗口尺寸/边距是像素域归前端)
// Maximum altitude for chart display (m)
const MAX_DISPLAY_ALT: i32 = 10000;
// Altitude step for curve generation (m)
const ALT_STEP: i32 = 25;

/// Java `loadFuelModification`: 尝试从中央文件
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
                logger::info(
                    "PowerCurveWindow",
                    &format!("Fuel modification: {}", mod_.r#type),
                );
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

/// Java `loadSingleCurve`: 载入单个 FM 并生成
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
        // 喷气机 compressorStages 为 null → "不是活塞引擎" (Java 同位判定)
        if handle.compressor_stages.is_none() {
            return error_curve(fm_name, format!("{fm_name} 不是活塞引擎"));
        }
        (handle.fmdata.unwrap(), handle.compressor_stages)
    } else {
        // ---- 回退: 按物理文件名直读 .json（连字符机型, 见方法注释）----
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
        // Java double * int 提升 double → as f64
        for p in power_curve.iter_mut() {
            *p *= fmdata.engine_num as f64;
        }
    }

    // Find maximum/minimum power and peak altitude
    let max_alt_idx = MAX_DISPLAY_ALT / ALT_STEP;
    let mut max_power = 0.0f64;
    let mut min_power = f64::MAX;
    let mut peak_altitude = 0i32;

    // 原 while 双条件 (i ≤ max_alt_idx 且 i < len) → 独占上界取小, 含端点语义不变
    let scan_len = ((max_alt_idx + 1) as usize).min(power_curve.len());
    for (i, p) in power_curve[..scan_len].iter().enumerate() {
        let i = i as i32;
        if *p > max_power {
            max_power = *p;
            peak_altitude = i * ALT_STEP;
        }
        if *p < min_power {
            min_power = *p;
        }
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
/// (Java tooCloseToList)
/// 波14 泛型化: 高度取值器 `alt_of` 由调用方给 (Phase3/4 传 DTO 的
/// altitude_m, Phase1 候选传元组 alt 分量), 收编峰/谷臂内两份内联同型检查。
fn too_close_to_list<T>(
    alt_m: i32,
    min_sep_m: i32,
    list: &[T],
    alt_of: impl Fn(&T) -> i32,
) -> bool {
    list.iter().any(|p| (alt_of(p) - alt_m).abs() < min_sep_m)
}

/// Phase1 单侧极值扫描臂 (波14 从峰/谷两份逐字对称的内联臂收敛而来):
/// 在 [hw, max_idx-hw] 逐点看 ±hw 斜率符号翻转 — `is_peak=true` 检测峰
/// (先升后降), false 检测谷 (先降后升); 突出度超噪声阈值且与已收录点
/// 间隔 ≥ min_sep_m 时, 收录窗口内最极值点 (峰取最大/谷取最小)。
/// 候选形态沿用 Java double[]{altM, power} → (i32, f64) 元组。
fn scan_extrema(
    power_curve: &[f64],
    max_idx: i32,
    hw: i32,
    noise_threshold: f64,
    min_sep_m: i32,
    is_peak: bool,
) -> Vec<(i32, f64)> {
    let mut candidates: Vec<(i32, f64)> = Vec::new();
    for i in hw..=max_idx - hw {
        let left = power_curve[(i - hw) as usize];
        let center = power_curve[i as usize];
        let right = power_curve[(i + hw) as usize];

        let left_slope = center - left;
        let right_slope = right - center;

        // 斜率符号翻转方向: 峰 = 先升后降, 谷 = 先降后升 (互斥)
        let sign_change = if is_peak {
            left_slope > 0.0 && right_slope < 0.0
        } else {
            left_slope < 0.0 && right_slope > 0.0
        };
        // 突出度: 峰 = 中心高过两肩低者; 谷 = 中心低过两肩高者
        let prominence = if is_peak {
            center - left.min(right)
        } else {
            left.max(right) - center
        };

        if sign_change && prominence > noise_threshold {
            // 窗口内最极值点 (峰最大/谷最小) 作为标注位置
            let mut best_idx = i;
            for j in i - hw..=i + hw {
                let is_better = if is_peak {
                    power_curve[j as usize] > power_curve[best_idx as usize]
                } else {
                    power_curve[j as usize] < power_curve[best_idx as usize]
                };
                if is_better {
                    best_idx = j;
                }
            }
            let alt_m = best_idx * ALT_STEP;
            if !too_close_to_list(alt_m, min_sep_m, &candidates, |c| c.0) {
                candidates.push((alt_m, power_curve[best_idx as usize]));
            }
        }
    }
    candidates
}

/// Java `identifyInflectionPointsForCurve`:
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
    // 峰/谷除比较方向外逐字对称 → 同一函数按方向扫两遍 (两臂条件互斥,
    // 且两列表随后各按高度排序, 遍历合拆不影响结果)
    let mut peak_candidates =
        scan_extrema(power_curve, max_idx, hw, noise_threshold, min_sep_m, true);
    let mut valley_candidates =
        scan_extrema(power_curve, max_idx, hw, noise_threshold, min_sep_m, false);

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
            kind: InflectionKind::Valley,
            label,
            altitude_m: *alt_m,
            power: *power,
        });
    }

    // ========== Phase 3: Add peaks (critical altitudes) ==========
    let mut stage_num = 1;
    for (alt_m, power) in &peak_candidates {
        if too_close_to_list(*alt_m, min_sep_m, &result, |p| p.altitude_m) {
            stage_num += 1;
            continue;
        }

        let label = format!("{stage_num}档");
        result.push(InflectionPointDto {
            kind: InflectionKind::Peak,
            label,
            altitude_m: *alt_m,
            power: *power,
        });
        stage_num += 1;
    }

    // ========== Phase 4: Detect slope kinks ==========
    let kink_half_window = 4;
    let avg_slope =
        (power_curve[max_idx as usize] - power_curve[0]).abs() / (max_idx * ALT_STEP) as f64;
    let kink_threshold = (avg_slope * 2.5).max(0.08);

    for i in kink_half_window..=max_idx - kink_half_window {
        if too_close_to_list(i * ALT_STEP, min_sep_m, &result, |p| p.altitude_m) {
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
            // 邻域夹逼 [i-2, i+2] ∩ [hw, max_idx-hw]: i-2 可低于 hw,
            // 下界抬到 kink_half_window 保循环体不越界 (原多条件 while 同语义)
            for j in (i - 2).max(kink_half_window)..=(i + 2).min(max_idx - kink_half_window) {
                let ls = (power_curve[j as usize] - power_curve[(j - kink_half_window) as usize])
                    / (kink_half_window * ALT_STEP) as f64;
                let rs = (power_curve[(j + kink_half_window) as usize] - power_curve[j as usize])
                    / (kink_half_window * ALT_STEP) as f64;
                let sc = (rs - ls).abs();
                if sc > best_change {
                    best_change = sc;
                    best_idx = j;
                }
            }

            if !too_close_to_list(best_idx * ALT_STEP, min_sep_m, &result, |p| p.altitude_m) {
                result.push(InflectionPointDto {
                    kind: InflectionKind::Kink,
                    label: "Kink".to_string(),
                    altitude_m: best_idx * ALT_STEP,
                    power: power_curve[best_idx as usize],
                });
            }
        }
    }

    result
}

/// Java `calculateDisplayRange`: 双曲线合并显示域
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

/// Java `buildErrorMessage`
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
        return if sb.is_empty() {
            Some("无法加载功率曲线".to_string())
        } else {
            Some(sb)
        };
    } else if !has_fm0 && curve0.error_message.is_some() {
        return curve0.error_message.clone();
    } else if !has_fm1 && curve1.is_some() && curve1.as_ref().unwrap().error_message.is_some() {
        return curve1.as_ref().unwrap().error_message.clone();
    }
    None
}

/// Java `loadPowerCurves` + 构造器的单双模式裁决
pub fn power_curve_data_impl(
    fm0_name: &str,
    fm1_name: Option<&str>,
    speed_kmh: i32,
    wep_mode: bool,
) -> PowerCurveDataDto {
    // Treat fm1Name == fm0Name as single curve mode (构造器裁决, normalize_secondary)
    let fm1_name = normalize_secondary(fm0_name, fm1_name);

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

/// 功率曲线窗口数据 (PowerCurveWindow.loadPowerCurves; fm1 空/==fm0 = 单曲线;
/// 直算不经主线程 — 见 commands_comparison 模块头分工说明)
#[tauri::command]
pub async fn power_curve_data(
    fm0: String,
    fm1: Option<String>,
    speed_kmh: i32,
    wep: bool,
) -> Result<serde_json::Value, String> {
    to_json(&power_curve_data_impl(&fm0, fm1.as_deref(), speed_kmh, wep))
}

// =====================================================================
// Tests — 拐点检测纯函数无依赖; 真机腿用项目内 data/ 的 spitfire, data 缺失
// 环境按 realtests 先例 SKIP+真因。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ensure_real_data;

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
        let peaks: Vec<&InflectionPointDto> = pts
            .iter()
            .filter(|p| p.kind == InflectionKind::Peak)
            .collect();
        let valleys: Vec<&InflectionPointDto> = pts
            .iter()
            .filter(|p| p.kind == InflectionKind::Valley)
            .collect();
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
                i if i <= 100 => i as f64 * 5.0,     // 0.2 hp/m
                _ => 500.0 + (i - 100) as f64 * 0.5, // 0.02 hp/m
            })
            .collect();
        let pts = identify_inflection_points_for_curve(&curve, 650.0);
        assert!(
            pts.iter()
                .all(|p| p.kind != InflectionKind::Peak && p.kind != InflectionKind::Valley),
            "同向曲线不应有峰谷: {pts:?}"
        );
        let kinks: Vec<&InflectionPointDto> = pts
            .iter()
            .filter(|p| p.kind == InflectionKind::Kink)
            .collect();
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
    fn 真机_功率曲线_单曲线_采样与拐点() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (power_curve_data 无数据源)");
            return;
        }
        let dto = power_curve_data_impl("spitfire_f24", None, 400, true);
        assert!(!dto.dual_mode);
        assert!(dto.curve1.is_none());
        let c0 = &dto.curve0;
        assert!(
            c0.valid,
            "spitfire_f24 应为可用活塞曲线: {:?}",
            c0.error_message
        );
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
            c0.inflection_points
                .iter()
                .any(|p| p.kind == InflectionKind::Peak),
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
}
