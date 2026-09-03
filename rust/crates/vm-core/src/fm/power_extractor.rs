//! Extracts engine parameters from parsed FM (Flight Model) data and converts
//! them to the format required by [`crate::fm::piston_model`].
//!
//! <p>This module bridges the gap between FmData's raw data arrays and the
//! structured CompressorStageParams objects needed for power calculations.
//!
//! <h3>WAPC Compatibility</h3>
//! <p>This implementation follows the wt-aircraft-performance-calculator (WAPC)
//! formulas, including:
//! <ul>
//!   <li>Deck power: Uses Main.Power for stage 0, 0.8× previous stage deck power for later stages</li>
//!   <li>WEP multiplier: 4-factor formula (octane, throttle, stage, RPM)</li>
//!   <li>WEP critical altitude: Supercharger pressure model</li>
//!   <li>ExactAltitudes detection: Old FM format handling</li>
//!   <li>definition_alt_power_adjuster: RPM-based power/altitude correction</li>
//!   <li>ConstRPM and Ceiling parameter propagation</li>
//!   <li>Fuel modifications: Soviet octane (1.8% power) and British octane (WEP boost)</li>
//! </ul>
//!
//! <h3>Fuel Modification Application Order (matching WAPC)</h3>
//! <pre>
//! 1. soviet_octane_adder()           ← modifies raw Power values
//! 2. definition_alt_power_adjuster() ← uses boosted values
//! 3. deck_power_maker()              ← cascades boost to all stages
//! 4. brrritish_octane_adder()        ← modifies WEP parameters
//! 5. wep_mulitiplierer()             ← uses modified WEP params
//! </pre>

use crate::base::atmosphere_model::{altitude_at_pressure, pressure};
use crate::fm::data::{CompressorData, FmData, FuelModification, FuelType};
use crate::fm::piston_model::{
    interpolate_power, supercharger_rpm_effect, torque_rpm_boost, CompressorStageParams,
};

/// Soviet octane power multiplier: 1.8% power increase.
/// Applied when addHorsePowers == 50 (WAPC soviet_octane_adder convention).
const SOVIET_OCTANE_POWER_MULT: f64 = 1.018;

/// 有效值回退: x > 0 取 x, 否则 fallback — FM 字段 0/缺省即"未定义"的
/// WAPC 惯用形态 (本文件 10+ 处 `if x > 0.0 { x } else { v }` 收敛)。
fn pos_or(x: f64, fallback: f64) -> f64 {
    if x > 0.0 {
        x
    } else {
        fallback
    }
}

/// [`pos_or`] 的 1.0 回退常用形态 (缺省乘数 = 不增益)。
fn or_one(x: f64) -> f64 {
    pos_or(x, 1.0)
}

/// Extracts compressor stage parameters from a parsed Blkx FM file.
///
/// - `blkx`: parsed FM file data
///
/// Returns array of CompressorStageParams, or None if not a piston engine
pub fn extract_stages(fmdata: Option<&FmData>) -> Option<Vec<CompressorStageParams>> {
    extract_stages_with_fuel(fmdata, None)
}

/// Extracts compressor stage parameters from a parsed Blkx FM file,
/// applying fuel quality modifications from the Central file.
///
/// <p>Fuel modifications are applied at the correct point in the WAPC pipeline:
/// Soviet octane is applied to raw power values BEFORE deck_power_maker and
/// definition_alt_power_adjuster, ensuring the boost cascades correctly through
/// the entire calculation chain.
///
/// - `blkx`:    parsed FM file data
/// - `fuel_mod`: fuel modification data from Central file, or None
///
/// Returns array of CompressorStageParams, or None if not a piston engine
pub fn extract_stages_with_fuel(
    fmdata: Option<&FmData>,
    fuel_mod: Option<&FuelModification>,
) -> Option<Vec<CompressorStageParams>> {
    // compNumSteps<=0 → None; i32<=0 守卫同时排除 as usize 的负值风险 (§2.2)
    let fmdata = fmdata?;
    if fmdata.comp_num_steps <= 0 {
        return None;
    }
    let n = fmdata.comp_num_steps as usize;

    // === Soviet octane: compute power multiplier ===
    // Applied to raw Compressor power values BEFORE deck_power_maker and
    // definition_alt_power_adjuster, matching WAPC call order:
    //   soviet_octane_adder() → definition_alt_power_adjuster() → deck_power_maker()
    // 顺序约束: spm 计算先于任何 comp* 数组解引用
    let spm = compute_soviet_power_multiplier(fuel_mod);

    // comp* 平行表与 compNumSteps 同批分配 (compNumSteps>0 时必已就绪) —
    // unwrap 对齐 NPE 语义 (§1): None = 病态输入
    // 波17 F5: 9 组平行表收拢为 CompressorData, 一次借用即得全部表
    let comp = fmdata.compressor.as_ref().unwrap();

    // Detect ExactAltitudes:
    // 1. If explicitly defined in FM file, use that
    // 2. Otherwise, if CompressorOmegaFactorSq is missing, set true (old format)
    let exact_altitudes = if let Some(explicit) = fmdata.explicit_exact_altitudes {
        explicit
    } else {
        !fmdata.has_comp_omega_factor_sq
    };

    // Determine "default RPM" — the RPM at which FM power values are defined
    let default_rpm = determine_default_rpm(fmdata);

    let mut stages: Vec<CompressorStageParams> = vec![CompressorStageParams::default(); n];

    // --- Pass 1: Basic parameter extraction + Soviet octane ---
    let mut stage_deck_power = vec![0.0f64; n];
    pass1_basic_and_soviet(&mut stages, &mut stage_deck_power, fmdata, comp, spm, exact_altitudes);

    // --- Pass 2: definition_alt_power_adjuster ---
    let needs_rpm_adjustment = needs_rpm_adjustment(fmdata, default_rpm);
    pass2_rpm_adjust(&mut stages, &mut stage_deck_power, fmdata, comp, default_rpm, spm, needs_rpm_adjustment);

    // Set stage0DeckAlt for all stages (used in WEP non-ExactAltitudes mode)
    let stage0_deck_alt = stages[0].deck_alt;
    for stage in stages.iter_mut().take(n) {
        stage.stage0_deck_alt = stage0_deck_alt;
    }

    // --- Pass 3: WEP parameters ---
    pass3_wep_params(&mut stages, fmdata, comp, exact_altitudes);

    // --- Pass 4: British octane (post-processing on WEP parameters) ---
    // Applied after WEP extraction, matching WAPC order where
    // brrritish_octane_adder modifies OctaneAfterburnerMult before wep_mulitiplierer
    if let Some(fm) = fuel_mod {
        apply_british_octane_bonus(&mut stages, fm, fmdata);
    }

    Some(stages)
}

/// Pass 1: 基本参数提取 + 苏联辛烷值增益 — WAPC soviet_octane_adder 位置
/// (对 Compressor 原始功率值预乘 spm, 先于 deck_power_maker 级联)。
/// `comp` 为 extract_stages_with_fuel 顶部一次解开的增压器表束
/// (unwrap 对齐 NPE — compNumSteps>0 时表必同批就绪)。
fn pass1_basic_and_soviet(
    stages: &mut [CompressorStageParams],
    stage_deck_power: &mut [f64],
    fmdata: &FmData,
    comp: &CompressorData,
    spm: f64,
    exact_altitudes: bool,
) {
    // Soviet octane multiplier (spm) is applied to all power values read from blkx,
    // BEFORE the deck_power_maker cascade. This matches WAPC's soviet_octane_adder()
    // which modifies Compressor["Power_i"] and Main["Power"] before initialization.
    let n = stages.len();

    for i in 0..n {
        stages[i].stage_index = i as i32;
        stages[i].exact_altitudes = exact_altitudes;

        // Critical altitude and power (with Soviet octane applied to raw power)
        stages[i].crit_alt = comp.alt[i];
        stages[i].crit_power = comp.power[i] * spm;

        // Store originals before adjustment (also boosted, matching WAPC order:
        // soviet_octane_adder modifies Compressor values, then
        // definition_alt_power_adjuster stores Old_ copies)
        stages[i].old_altitude = comp.alt[i];
        stages[i].old_power = comp.power[i] * spm;
        stages[i].old_power_new_rpm = comp.power[i] * spm;

        // ConstRPM parameters (Soviet octane applied if ConstRPM exists)
        // const_rpm_power 与 const_rpm_alt 同批分配, 后者非空前者必非空 — unwrap 对齐 NPE
        if let Some(const_rpm_alt) = comp.const_rpm_alt.as_ref() {
            if i < const_rpm_alt.len() {
                stages[i].const_rpm_alt = const_rpm_alt[i];
                stages[i].const_rpm_power =
                    comp.const_rpm_power.as_ref().unwrap()[i] * spm;
            }
        }

        // Ceiling parameters (Soviet octane applied to power)
        stages[i].ceiling_alt = comp.ceil[i];
        stages[i].ceiling_power = comp.ceil_pwr[i] * spm;

        // Power curve curvature
        stages[i].curvature = or_one(comp.rpm_ratio[i]);

        // RAM effect coefficient
        stages[i].speed_manifold_mult = or_one(fmdata.speed_to_manifold_multiplier);

        // Deck power: WAPC deck_power_maker logic
        // Stage 0: Main.Power (with Soviet octane); Stage 1+: 0.8× previous stage DECK power
        if i == 0 {
            stage_deck_power[i] = (if fmdata.deck_power > 0.0 {
                fmdata.deck_power
            } else {
                comp.power[0] * 0.8
            }) * spm;
        } else {
            stage_deck_power[i] = 0.8 * stage_deck_power[i - 1];
            let min_deck = 0.8 * comp.power[i] * spm;
            if stage_deck_power[i] < min_deck {
                stage_deck_power[i] = min_deck;
            }
        }
        stages[i].deck_power = stage_deck_power[i];
    }
}

/// Pass 2: definition_alt_power_adjuster — FM 功率/高度定义在更高转速 (WEP 或
/// default RPM) 时向军用转速基线折算, 并级联重算后续级的甲板功率。
fn pass2_rpm_adjust(
    stages: &mut [CompressorStageParams],
    stage_deck_power: &mut [f64],
    fmdata: &FmData,
    comp: &CompressorData,
    default_rpm: f64,
    spm: f64,
    needs_rpm_adjustment: bool,
) {
    // If FM power/altitude is defined for a higher RPM (WEP or default RPM) rather
    // than military RPM, adjust to military RPM baseline
    let n = stages.len();

    for i in 0..n {
        if needs_rpm_adjustment {
            adjust_power_and_altitude(
                &mut stages[i],
                fmdata,
                i,
                default_rpm,
                stage_deck_power,
                spm,
            );
            // After adjustment, update stageDeckPower and cascade to subsequent stages
            stage_deck_power[i] = stages[i].deck_power;
            // Recalculate deck power for subsequent stages based on adjusted values
            for j in (i + 1)..n {
                let mut new_deck = 0.8 * stage_deck_power[j - 1];
                let min_deck = 0.8 * comp.power[j] * spm;
                if new_deck < min_deck {
                    new_deck = min_deck;
                }
                stage_deck_power[j] = new_deck;
                stages[j].deck_power = new_deck;
            }
        }

        stages[i].old_power_new_rpm = stages[i].old_power;
        if needs_rpm_adjustment {
            stages[i].old_power_new_rpm =
                stages[i].old_power / torque_rpm_boost(fmdata.military_rpm, default_rpm);
        }
    }
}

/// Pass 3: WEP 参数 — WEP 乘数/临界高度/甲板高度/ConstRPM 高度 + 增压器增益
/// 显式置 0 的级禁用 WEP 处理。
fn pass3_wep_params(
    stages: &mut [CompressorStageParams],
    fmdata: &FmData,
    comp: &CompressorData,
    exact_altitudes: bool,
) {
    for i in 0..stages.len() {
        stages[i].wep_power_mult = calculate_wep_multiplier(fmdata, i);
        stages[i].wep_crit_alt = calculate_wep_critical_altitude(fmdata, &stages[i], i);
        stages[i].wep_deck_alt = calculate_wep_deck_altitude(fmdata, &stages[i], i);

        // WEP ConstRPM altitude (for non-ExactAltitudes FMs like F2G-1)
        if !exact_altitudes && stages[i].const_rpm_alt != 0.0 && stages[i].const_rpm_power > 0.0 {
            stages[i].wep_const_rpm_alt = calculate_wep_const_rpm_altitude(fmdata, &stages[i], i);
        }

        // Handle AfterburnerBoostMul explicitly set to 0 (no WEP for this stage)
        // Only disable WEP if the field EXISTS and is explicitly 0
        // If field is missing, WEP uses global AfterburnerBoost (handled by calculateWepMultiplier)
        let has_boost = comp.has_boost.as_ref().is_some_and(|hb| hb[i]);
        if has_boost && comp.boost[i] == 0.0 {
            stages[i].wep_deck_alt = 0.0;
            stages[i].wep_crit_alt = stages[i].crit_alt;
            stages[i].wep_power_mult = 1.0;
        }
    }
}

// ==================== Soviet Octane ====================

/// Computes the Soviet octane power multiplier from fuel modification data.
///
/// <p>Returns 1.0 (no change) if fuel modification is null, not Soviet type,
/// or addHorsePowers is not exactly 50.
///
/// - `fuel_mod`: fuel modification data, or null
///
/// Returns power multiplier (1.0 or 1.018)
fn compute_soviet_power_multiplier(fuel_mod: Option<&FuelModification>) -> f64 {
    let fuel_mod = match fuel_mod {
        Some(f) => f,
        None => return 1.0,
    };

    let is_soviet = fuel_mod.r#type == FuelType::SovietB95 || fuel_mod.r#type == FuelType::SovietB100;
    if !is_soviet {
        return 1.0;
    }

    // WAPC only applies the bonus when addHorsePowers == 50
    if (fuel_mod.soviet_octane_hp_bonus - 50.0).abs() > 0.01 {
        return 1.0;
    }

    SOVIET_OCTANE_POWER_MULT
}

// ==================== RPM Adjustment (definition_alt_power_adjuster) ====================

/// Checks if the FM defines power values at a higher RPM than military RPM.
/// If so, the power/altitude values need to be adjusted down to military RPM baseline.
fn needs_rpm_adjustment(fmdata: &FmData, default_rpm: f64) -> bool {
    (default_rpm - fmdata.military_rpm) > 5.0
}

/// Determines the "default RPM" — the RPM at which FM file power values are defined.
///
/// <p>Port of WAPC wep_rpm_ratioer priority logic:
/// <ol>
///   <li>ShaftRPMMax: if far from military AND close to WEP RPM</li>
///   <li>RPMNom: if far from military RPM</li>
///   <li>GovernorMaxParam: if far from military RPM</li>
///   <li>Fallback: military RPM (no adjustment needed)</li>
/// </ol>
fn determine_default_rpm(fmdata: &FmData) -> f64 {
    // Priority 1: ShaftRPMMax close to WEP but far from military
    if fmdata.shaft_rpm_max > 0.0
        && (fmdata.shaft_rpm_max - fmdata.military_rpm) > 5.0
        && (fmdata.shaft_rpm_max - fmdata.wep_rpm) < 5.0
    {
        return fmdata.shaft_rpm_max;
    }
    // Priority 2: RPMNom far from military
    if fmdata.rpm_nom > 0.0 && (fmdata.rpm_nom - fmdata.military_rpm) > 5.0 {
        return fmdata.rpm_nom;
    }
    // Priority 3: GovernorMaxParam far from military
    if fmdata.governor_max_param > 0.0 && (fmdata.governor_max_param - fmdata.military_rpm) > 5.0 {
        return fmdata.governor_max_param;
    }
    fmdata.military_rpm
}

/// Adjusts power and critical altitude from default RPM to military RPM.
/// Port of WAPC definition_alt_power_adjuster().
///
/// - `stage`:          stage parameters to adjust
/// - `blkx`:           raw FM data
/// - `i`:              stage index
/// - `default_rpm`:    the RPM at which FM values are defined
/// - `stage_deck_power`: deck power array for cascade
/// - `_spm`:           Soviet power multiplier (applied to raw blkx power reads; unused here)
// _spm 形参保真保留: spm 已在 Pass 1 预乘, 比值中相消 (见函数内 "spm cancels" 注)
fn adjust_power_and_altitude(
    stage: &mut CompressorStageParams,
    fmdata: &FmData,
    i: usize,
    default_rpm: f64,
    stage_deck_power: &mut [f64],
    _spm: f64,
) {
    let military_mp = fmdata.military_mp;
    if military_mp <= 0.0 {
        return;
    }

    let rpm_boost = torque_rpm_boost(fmdata.military_rpm, default_rpm);
    if rpm_boost <= 0.0 || (rpm_boost - 1.0).abs() < 0.001 {
        return;
    }

    // Calculate supercharger effect to find adjusted critical altitude
    let pressure_at_rpm0 = pos_or(fmdata.comp_pressure_at_rpm0, 0.3);
    // WAPC: missing → 1.0; explicit 0 → 0
    let omega_factor_sq = if fmdata.has_comp_omega_factor_sq {
        fmdata.comp_omega_factor_sq
    } else {
        1.0
    };
    let default_mil_rpm_effect = supercharger_rpm_effect(
        fmdata.military_rpm,
        default_rpm,
        pressure_at_rpm0,
        omega_factor_sq,
    );

    // Adjust critical altitude: remove the extra supercharger boost from higher RPM
    let fake_supercharger_strength = military_mp / pressure(stage.crit_alt);
    let real_supercharger_strength = fake_supercharger_strength / default_mil_rpm_effect;
    // PORT: Java Math.round(double)=floor(x+0.5) 返回 long 再拓宽 double (§2.3)
    let adjusted_crit_alt =
        java_round(altitude_at_pressure(military_mp / real_supercharger_strength)) as f64;

    // Adjust deck altitude similarly
    let fake_deck_strength = military_mp / pressure(0.0);
    let real_deck_strength = fake_deck_strength / default_mil_rpm_effect;
    let adjusted_deck_alt = altitude_at_pressure(military_mp / real_deck_strength);
    stage.deck_alt = adjusted_deck_alt;

    // Adjust power: interpolate on original curve at new crit alt, then divide by RPM boost
    // Note: deckPowerRatio uses raw blkx values — the spm cancels in the ratio
    let comp = fmdata.compressor.as_ref().unwrap(); // unwrap 对齐 NPE, 见顶部批注
    let deck_power_ratio = if fmdata.deck_power > 0.0 && stage.old_power > 0.0 {
        fmdata.deck_power / comp.power[0]
    } else {
        0.8
    };
    let adjusted_power = interpolate_power(
        stage.old_power,
        stage.old_altitude,
        stage.old_power * deck_power_ratio,
        stage.old_altitude - comp.alt[0],
        adjusted_crit_alt,
        1.0,
    ) / rpm_boost;

    // Adjust ConstRPM power
    if stage.const_rpm_power > 0.0 {
        if stage.const_rpm_power == stage.old_power {
            // Special case (Hornet Mk3): keep constRPM aligned with adjusted power
            stage.const_rpm_power = adjusted_power;
        } else {
            stage.const_rpm_power /= rpm_boost;
        }
    }

    // Adjust ceiling altitude
    if stage.ceiling_alt > 0.0 {
        let fake_ceil_strength = military_mp / pressure(stage.ceiling_alt);
        let real_ceil_strength = fake_ceil_strength / default_mil_rpm_effect;
        stage.ceiling_alt =
            java_round(altitude_at_pressure(military_mp / real_ceil_strength)) as f64;
    }

    stage.crit_alt = adjusted_crit_alt;
    stage.crit_power = adjusted_power;

    // Recalculate deck power after adjustment
    if i == 0 {
        // Deck power is interpolated on the original curve at the adjusted deck altitude, then /rpmBoost
        stage.deck_power = interpolate_power(
            stage.old_power,
            stage.old_altitude,
            stage_deck_power[0],
            0.0,
            adjusted_deck_alt,
            1.0,
        ) / rpm_boost;
        stage_deck_power[0] = stage.deck_power;
    }
}

// ==================== WEP Parameter Calculations ====================

/// Calculates the complete WEP power multiplier.
///
/// <p>Implements WAPC WEP_power_mult formula:
/// <pre>
/// WEP_mult = (1 + (AfterburnerBoost - 1) x OctaneAfterburnerMult)
///          x ThrottleBoost
///          x AfterburnerBoostMul[i]
///          x torque_rpm_boost(military_RPM, WEP_RPM)
/// </pre>
fn calculate_wep_multiplier(fmdata: &FmData, stage_index: usize) -> f64 {
    let comp_boost = &fmdata.compressor.as_ref().unwrap().boost; // unwrap 对齐 NPE, 见顶部批注
    let afterburner_boost = or_one(fmdata.aftb_coff);
    let octane_mult = or_one(fmdata.octane_afterburner_mult);
    let boost_effect = 1.0 + (afterburner_boost - 1.0) * octane_mult;

    let throttle_boost = or_one(fmdata.throttle_boost);
    let stage_mult = or_one(comp_boost[stage_index]);
    let rpm_boost = torque_rpm_boost(fmdata.military_rpm, fmdata.wep_rpm);

    boost_effect * throttle_boost * stage_mult * rpm_boost
}

/// WEP 增压器强度公共段: `base_strength × RPM 效率 × 级后燃压力增益`
/// (WEP 临界/甲板/ConstRPM 高度与英油重算四处逐字同款, 收敛于此)。
/// 系数回退: omega 未定义 → 1.0 (WAPC: missing → 1.0; explicit 0 → 0);
/// comp_pressure_at_rpm0 ≤ 0 → 0.3; 级压力增益缺失/为 0 → 1.0。
fn wep_supercharger_strength(fmdata: &FmData, base_strength: f64, stage_index: usize) -> f64 {
    let omega_factor_sq = if fmdata.has_comp_omega_factor_sq {
        fmdata.comp_omega_factor_sq
    } else {
        1.0
    };
    let rpm_effect = supercharger_rpm_effect(
        fmdata.military_rpm,
        fmdata.wep_rpm,
        pos_or(fmdata.comp_pressure_at_rpm0, 0.3),
        omega_factor_sq,
    );
    let pressure_boost = match fmdata.comp_afterburner_pressure_boost.as_ref() {
        Some(v) if stage_index < v.len() && v[stage_index] > 0.0 => v[stage_index],
        _ => 1.0,
    };
    base_strength * rpm_effect * pressure_boost
}

/// Calculates the WEP critical altitude using supercharger pressure model.
fn calculate_wep_critical_altitude(
    fmdata: &FmData,
    stage: &CompressorStageParams,
    stage_index: usize,
) -> f64 {
    // If WEP power multiplier ≈ 1.0, WEP is effectively military — no altitude shift
    let wep_mult = calculate_wep_multiplier(fmdata, stage_index);
    if (wep_mult - 1.0).abs() < 0.001 {
        return stage.crit_alt;
    }

    let military_mp = fmdata.military_mp;
    let wep_mp = fmdata.wep_manifold_pressure;

    if military_mp <= 0.0 || wep_mp <= 0.0 {
        return stage.crit_alt * 0.9;
    }

    // Use the adjusted (military) critical altitude for strength calculation
    let supercharger_strength = military_mp / pressure(stage.crit_alt);

    let wep_crit_pressure = wep_mp / wep_supercharger_strength(fmdata, supercharger_strength, stage_index);
    // PORT: Java Math.round(double)=floor(x+0.5) 返回 long 再拓宽 double (§2.3)
    java_round(altitude_at_pressure(wep_crit_pressure)) as f64
}

/// Calculates the WEP deck altitude.
fn calculate_wep_deck_altitude(
    fmdata: &FmData,
    stage: &CompressorStageParams,
    stage_index: usize,
) -> f64 {
    // If WEP power multiplier ≈ 1.0, WEP is effectively military — no deck shift
    let wep_mult = calculate_wep_multiplier(fmdata, stage_index);
    if (wep_mult - 1.0).abs() < 0.001 {
        return stage.deck_alt;
    }

    let military_mp = fmdata.military_mp;
    let wep_mp = fmdata.wep_manifold_pressure;

    if military_mp <= 0.0 || wep_mp <= 0.0 {
        return 0.0;
    }

    let deck_strength = military_mp / pressure(stage.deck_alt);
    // PORT: Java Math.round(double)=floor(x+0.5) 返回 long 再拓宽 double (§2.3)
    java_round(altitude_at_pressure(wep_mp / wep_supercharger_strength(fmdata, deck_strength, stage_index)))
        as f64
}

/// Calculates the WEP ConstRPM altitude for non-ExactAltitudes FMs.
fn calculate_wep_const_rpm_altitude(
    fmdata: &FmData,
    stage: &CompressorStageParams,
    stage_index: usize,
) -> f64 {
    let military_mp = fmdata.military_mp;
    let wep_mp = fmdata.wep_manifold_pressure;

    if military_mp <= 0.0 || wep_mp <= 0.0 || stage.const_rpm_alt == 0.0 {
        return 0.0;
    }

    let const_rpm_strength = military_mp / pressure(stage.const_rpm_alt);
    altitude_at_pressure(wep_mp / wep_supercharger_strength(fmdata, const_rpm_strength, stage_index))
}

// ==================== British Octane (Post-processing) ====================

/// Applies British fuel octane bonus to all compressor stages.
///
/// <p>Port of WAPC brrritish_octane_adder():
/// <ul>
///   <li>If invertEnableLogic is true: high octane is the default, so
///       the modification represents REMOVING it — no bonus applied</li>
///   <li>Otherwise: replaces OctaneAfterburnerMult in the WEP formula
///       with the fuel's afterburnerMult value, and recalculates WEP
///       critical altitude using afterburnerCompressorMult</li>
/// </ul>
///
/// - `stages`:  compressor stages to modify in-place
/// - `fuel_mod`: fuel modification containing British fuel parameters
/// - `blkx`:    parsed FM data for recalculating WEP altitudes
fn apply_british_octane_bonus(
    stages: &mut [CompressorStageParams],
    fuel_mod: &FuelModification,
    fmdata: &FmData,
) {
    let is_british = fuel_mod.r#type == FuelType::British150Octane
        || fuel_mod.r#type == FuelType::British100Spitfire;
    if !is_british {
        return;
    }

    // invertEnableLogic means the high-octane fuel is the DEFAULT state
    // The "modification" represents removing it, so we don't apply any bonus
    if fuel_mod.british_invert_logic {
        return;
    }

    // Recompute WEP power multiplier with fuel's afterburnerMult replacing OctaneAfterburnerMult
    // WAPC: Main["OctaneAfterburnerMult"] = fuel's afterburnerMult
    let afterburner_boost = or_one(fmdata.aftb_coff);
    let throttle_boost = or_one(fmdata.throttle_boost);
    let rpm_boost = torque_rpm_boost(fmdata.military_rpm, fmdata.wep_rpm);

    let comp = fmdata.compressor.as_ref().unwrap(); // unwrap 对齐 NPE, 见顶部批注
    for i in 0..stages.len() {
        let stage_mult = or_one(comp.boost[i]);

        // Handle stages with explicitly disabled WEP
        let has_boost = comp.has_boost.as_ref().is_some_and(|hb| hb[i]);
        if has_boost && comp.boost[i] == 0.0 {
            continue;
        }

        // WAPC formula with fuel's OctaneAfterburnerMult
        let fuel_boost_effect = 1.0 + (afterburner_boost - 1.0) * fuel_mod.british_afterburner_mult;
        stages[i].wep_power_mult = fuel_boost_effect * throttle_boost * stage_mult * rpm_boost;

        // Recalculate WEP critical altitude with fuel's compressor boost
        // WAPC: Octane_MP = Military_MP + (WEP_MP - Military_MP) × afterburnerCompressorMult
        if fmdata.military_mp > 0.0
            && fmdata.wep_manifold_pressure > 0.0
            && (stages[i].wep_power_mult - 1.0).abs() > 0.001
        {
            let octane_mp = fmdata.military_mp
                + (fmdata.wep_manifold_pressure - fmdata.military_mp)
                    * fuel_mod.british_afterburner_compressor_mult;

            let supercharger_strength = fmdata.military_mp / pressure(stages[i].crit_alt);
            let wep_crit_pressure =
                octane_mp / wep_supercharger_strength(fmdata, supercharger_strength, i);
            // PORT: Java Math.round(double)=floor(x+0.5) 返回 long 再拓宽 double (§2.3)
            stages[i].wep_crit_alt = java_round(altitude_at_pressure(wep_crit_pressure)) as f64;
        }
    }
}

// ==================== Public Utility Methods ====================

/// Checks if the aircraft uses piston engines.
///
/// - `blkx`: parsed FM file data
///
/// Returns true if piston engine (has compressor stages), false otherwise
pub fn is_piston_engine(fmdata: Option<&FmData>) -> bool {
    fmdata.is_some_and(|b| !b.is_jet && b.comp_num_steps > 0)
}

/// Gets the global WEP boost factor from FM data.
///
/// - `blkx`: parsed FM file data
///
/// Returns WEP boost factor, or 1.0 if not available
pub fn get_wep_boost_factor(fmdata: Option<&FmData>) -> f64 {
    let fmdata = match fmdata {
        Some(b) => b,
        None => return 1.0,
    };
    or_one(fmdata.aftb_coff)
}

/// Gets the RAM effect coefficient from FM data.
///
/// - `blkx`: parsed FM file data
///
/// Returns SpeedManifoldMultiplier, or 1.0 if not available
pub fn get_speed_manifold_multiplier(fmdata: Option<&FmData>) -> f64 {
    let fmdata = match fmdata {
        Some(b) => b,
        None => return 1.0,
    };
    or_one(fmdata.speed_to_manifold_multiplier)
}

// ==================== Java Math.round 复刻 (§2.3) ====================

/// Java `Math.round(double)` = `floor(x + 0.5)`, 返回 long
// PORT: Rust f64::round 是半偶舍入, 不可用; 与 format.rs / piston_power_model.rs 的
// java_round 同源实现; 调用处 `as f64` 复刻 Java long→double 拓宽
fn java_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

// =====================================================================
// Tests — Java 8 oracle 对拍 (PORTING.md §5.1 A 类策略, 断言源移植自
// TestSpitfireF24Power / TestTempestMk5Power)。
//
// 真机 fixture (spitfire_f24 / yak-3 / spitfire_ix / tempest_mkv) 按真实
// data/aces/gamedata/flightmodels/ 文件的 FM 参数手工构造, 全部数值 =
// Java 8 (OpenJDK 1.8.0_342) 编译产物 + 真实数据文件实测 dump 的 %.17g 值。
// TODO: 可切换 reader::parse 读真文件, 收窄 fixture 维护面。
//
// fixture 数值陷阱: FM 的 getdouble 族用 Float.parseFloat 赋值 double
// (24-bit 尾数, 见 data/mod.rs) — 真机字段须写 `1.61f32 as f64` 形式,
// 直接写 1.61f64 会对拍失败 (synthetic 组是 Java 双精度字面量直赋, 不带 f32 拓宽)。
// =====================================================================
#[cfg(test)]
mod tests;
