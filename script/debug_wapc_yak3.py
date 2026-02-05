#!/usr/bin/env python3
"""
Debug script for WAPC Yak-3 power curve calculation.
Outputs intermediate values for comparison with VoidMei.

Usage: python3 script/debug_wapc_yak3.py [path/to/yak-3.blkx]

If no path is provided, uses default path:
  /home/tu10ng/Downloads/voidmei/data/aces/gamedata/flightmodels/fm/yak-3.blkx

This script now validates hardcoded parameters against the actual blkx file,
then traces through WAPC's calculation functions, outputting values in a
format that can be diff'd against the VoidMei debug script output.
"""

import sys
import os
import re

# Add WAPC path
sys.path.insert(0, '/home/tu10ng/projects/wtapc/performance_calculators')

from ram_pressure_density_calculator import air_pressurer, altitude_at_pressure, air_densitier, ias_tas_er

# Default blkx file path
DEFAULT_BLKX_PATH = "/home/tu10ng/Downloads/voidmei/data/aces/gamedata/flightmodels/fm/yak-3.blkx"


# ============================================================================
# BLKX File Parser (Simplified)
# ============================================================================

def parse_blkx_file(filepath):
    """
    Parse a blkx file and extract key parameters for validation.
    Returns a dict with extracted values.
    """
    if not os.path.exists(filepath):
        print(f"WARNING: Blkx file not found: {filepath}")
        return None

    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()

    result = {}

    # Helper to extract a value by key
    def get_value(key, content_block=content, as_bool=False):
        # Look for pattern: key = value or key:r = value, etc.
        # The blkx format uses type suffixes like :r (real), :i (int), :b (bool)
        patterns = [
            # Match key:r = value, key:i = value, key:b = value
            rf'{re.escape(key)}:r\s*=\s*([0-9.eE+-]+)',
            rf'{re.escape(key)}:i\s*=\s*([0-9+-]+)',
            rf'{re.escape(key)}:b\s*=\s*(true|false|yes|no)',
            # Match key = value (without type suffix)
            rf'{re.escape(key)}\s*=\s*([0-9.eE+-]+)',
            rf'{re.escape(key)}\s*=\s*(true|false|yes|no)',
            # Match key:t = "string"
            rf'{re.escape(key)}:t\s*=\s*"([^"]+)"',
        ]
        for pattern in patterns:
            match = re.search(pattern, content_block, re.IGNORECASE)
            if match:
                val = match.group(1).strip().strip('"')
                if as_bool:
                    return val.lower() in ('true', 'yes', '1')
                try:
                    return float(val.split(',')[0].strip())
                except ValueError:
                    return val
        return None

    # Extract Compressor section
    comp_match = re.search(r'Compressor\s*\{([^}]+(?:\{[^}]*\}[^}]*)*)\}', content, re.DOTALL | re.IGNORECASE)
    comp_block = comp_match.group(1) if comp_match else content

    # Extract compressor parameters for stages 0 and 1
    for i in range(2):
        result[f"Altitude{i}"] = get_value(f"Altitude{i}", comp_block)
        result[f"Power{i}"] = get_value(f"Power{i}", comp_block)
        result[f"Ceiling{i}"] = get_value(f"Ceiling{i}", comp_block)
        result[f"PowerAtCeiling{i}"] = get_value(f"PowerAtCeiling{i}", comp_block)
        result[f"AltitudeConstRPM{i}"] = get_value(f"AltitudeConstRPM{i}", comp_block)
        result[f"PowerConstRPM{i}"] = get_value(f"PowerConstRPM{i}", comp_block)
        result[f"AfterburnerBoostMul{i}"] = get_value(f"AfterburnerBoostMul{i}", comp_block)
        result[f"PowerConstRPMCurvature{i}"] = get_value(f"PowerConstRPMCurvature{i}", comp_block)

    result["NumSteps"] = get_value("NumSteps", comp_block)
    result["ExactAltitudes"] = get_value("ExactAltitudes", comp_block, as_bool=True)
    result["CompressorOmegaFactorSq"] = get_value("CompressorOmegaFactorSq", comp_block)
    result["CompressorPressureAtRPM0"] = get_value("CompressorPressureAtRPM0", comp_block)
    result["SpeedManifoldMultiplier"] = get_value("SpeedManifoldMultiplier", comp_block)
    result["AfterburnerManifoldPressure"] = get_value("AfterburnerManifoldPressure", comp_block)

    # Extract Main section values
    main_match = re.search(r'Main\s*\{([^}]+(?:\{[^}]*\}[^}]*)*)\}', content, re.DOTALL | re.IGNORECASE)
    main_block = main_match.group(1) if main_match else content

    result["Main.Power"] = get_value("Power", main_block)
    result["AfterburnerBoost"] = get_value("AfterburnerBoost", main_block)
    result["ThrottleBoost"] = get_value("ThrottleBoost", main_block)
    result["OctaneAfterburnerMult"] = get_value("OctaneAfterburnerMult", main_block)

    return result


def validate_hardcoded_params(blkx_values):
    """
    Compare hardcoded parameters with parsed blkx values.
    Returns True if all values match, False otherwise.
    """
    if blkx_values is None:
        print(">>> SKIPPING VALIDATION - blkx file not found <<<\n")
        return True

    print("=== BLKX FILE VALIDATION ===\n")

    # Expected hardcoded values
    expected = {
        "Altitude0": 300,
        "Power0": 1310,
        "Ceiling0": 5000,
        "PowerAtCeiling0": 670,
        "AltitudeConstRPM0": 18300,
        "PowerConstRPM0": 1310,
        "PowerConstRPMCurvature0": 1,
        "Altitude1": 2600,
        "Power1": 1240,
        "Ceiling1": 9000,
        "PowerAtCeiling1": 510,
        "AltitudeConstRPM1": 18300,
        "PowerConstRPM1": 1240,
        "PowerConstRPMCurvature1": 1,
        "Main.Power": 1290,
        "ExactAltitudes": True,
        "SpeedManifoldMultiplier": 1.0,
        "AfterburnerBoost": 1.0,
        "CompressorOmegaFactorSq": 0,
        "CompressorPressureAtRPM0": 0.4,
        "AfterburnerManifoldPressure": 1.48,
    }

    all_match = True
    for key, expected_val in expected.items():
        actual_val = blkx_values.get(key)
        if actual_val is None:
            print(f"  {key}: actual=NOT_FOUND, expected={expected_val} [MISSING]")
            # Don't fail on missing values that might be in different sections
            continue

        if isinstance(expected_val, bool):
            match = actual_val == expected_val
        else:
            match = abs(float(actual_val) - float(expected_val)) < 0.01

        status = "OK" if match else "MISMATCH"
        print(f"  {key}: actual={actual_val}, expected={expected_val} [{status}]")
        if not match:
            all_match = False

    print()
    if all_match:
        print(">>> ALL VALIDATED VALUES MATCH HARDCODED EXPECTATIONS <<<")
    else:
        print(">>> SOME VALUES DIFFER FROM HARDCODED - SEE MISMATCHES ABOVE <<<")
    print()

    return all_match


# ============================================================================
# Yak-3 FM Parameters (hardcoded from blkx)
# ============================================================================

def setup_yak3_params():
    """
    Set up Yak-3 FM parameters as WAPC expects them.
    Based on the ACTUAL Yak-3 flight model file: yak-3.blkx
    """
    # Compressor dict (stage parameters) - from EngineType0.Compressor
    Compressor = {
        # Stage 0
        "Altitude0": 300,
        "Power0": 1310,
        "AfterburnerBoostMul0": 1,
        "AltitudeConstRPM0": 18300,  # ACTUAL VALUE - very high, effectively "useless"
        "PowerConstRPM0": 1310,
        "PowerConstRPMCurvature0": 1,
        "Ceiling0": 5000,
        "PowerAtCeiling0": 670,

        # Stage 1
        "Altitude1": 2600,
        "Power1": 1240,
        "AfterburnerBoostMul1": 1,
        "AltitudeConstRPM1": 18300,  # ACTUAL VALUE
        "PowerConstRPM1": 1240,
        "PowerConstRPMCurvature1": 1,
        "Ceiling1": 9000,
        "PowerAtCeiling1": 510,

        # Common parameters
        "ExactAltitudes": True,  # ExactAltitudes:b = true in FM
        "CompressorOmegaFactorSq": 0,  # ACTUAL VALUE = 0 (not 1.0!)
        "CompressorPressureAtRPM0": 0.4,  # ACTUAL VALUE (not 0.1!)
        "SpeedManifoldMultiplier": 1.0,
        "AfterburnerManifoldPressure": 1.48,  # ACTUAL VALUE (not 1.42!)
    }

    # Main dict (engine base parameters) - from EngineType0.Main
    Main = {
        "Power": 1290,  # Deck power (sea level)
        "military_RPM": 2700,  # RPMMax
        "WEP_RPM": 2700,  # RPMAfterburner (same, no WEP)
        "default_RPM": 2700,
        "ThrottleBoost": 1.0,
        "AfterburnerBoost": 1.0,  # No WEP boost
        "OctaneAfterburnerMult": 1,
    }

    # Temperature section for manifold pressures
    # Mode2 is military mode: ManifoldPressure = 1.447
    Main["Military_MP"] = 1.447  # ACTUAL VALUE from Mode2
    Main["WEP_MP"] = 1.48  # AfterburnerManifoldPressure (but no WEP)
    Main["Octane_MP"] = 1

    # Propeller dict - from PropellerType0.Governor
    Propeller = {
        "GovernorMaxParam": 2700,
        "GovernorAfterburnerParam": 2700,
    }

    return Compressor, Main, Propeller


# ============================================================================
# Helper functions (copied from WAPC for debugging)
# ============================================================================

def ConstRPM_is(Compressor, i):
    if all(k in Compressor for k in ("AltitudeConstRPM" + str(i), "PowerConstRPM" + str(i))):
        return True
    return False

def ConstRPM_bends_below_critalt(Compressor, i):
    if ConstRPM_is(Compressor, i) and -1 > Compressor["AltitudeConstRPM" + str(i)] - Compressor["Altitude" + str(i)]:
        return True
    return False

def ConstRPM_bends_above_crit_alt(Compressor, i):
    if ConstRPM_is(Compressor, i) and Compressor["AltitudeConstRPM" + str(i)] == Compressor["Altitude" + str(i)] and \
            Compressor["Power" + str(i)] - Compressor["PowerAtCeiling" + str(i)] > 1 and \
            Compressor.get("PowerConstRPMCurvature0", 1) > 1:
        return True
    return False

def ConstRPM_is_below_deck(Compressor, i):
    if ConstRPM_is(Compressor, i) and Compressor["AltitudeConstRPM" + str(i)] <= 0:
        return True
    return False

def ConstRPM_bends_below_WEP_critalt(Main, Compressor, i):
    if ConstRPM_is(Compressor, i) and -1 > Compressor["AltitudeConstRPM" + str(i)] - Main["WEP_crit_altitude"]:
        return True
    return False

def Power_is_deck_power(Main, Compressor, i):
    if Compressor["Altitude" + str(i)] == Main["Deck_Altitude" + str(i)]:
        return True
    return False

def Ceiling_is(Compressor, i):
    if all(k not in Compressor for k in ("Ceiling" + str(i), "PowerAtCeiling" + str(i))):
        return False
    return True

def Ceiling_is_useful(Compressor, i):
    """
    NOTE: This uses Compressor["Altitude" + str(i)] and Compressor["Power" + str(i)],
    which are the CURRENT values (possibly modified by definition_alt_power_adjuster),
    NOT the Old_ versions!
    """
    if all(k not in Compressor for k in ("Ceiling" + str(i), "PowerAtCeiling" + str(i))) or \
       (Compressor["Ceiling" + str(i)] - Compressor["Altitude" + str(i)] < 2) or \
       (Compressor["Power" + str(i)] - Compressor["PowerAtCeiling" + str(i)] < 2):
        return False
    return True

def torquer(Main, lower_RPM, higher_RPM):
    Torque_max_RPM = 0.75 * higher_RPM
    if lower_RPM <= 0:
        return 1.0
    WEP_military_RPM_boost = ((higher_RPM * ((2 * Torque_max_RPM * higher_RPM) - (higher_RPM ** 2))) /
                              (lower_RPM * ((2 * Torque_max_RPM * lower_RPM) - (lower_RPM ** 2))))
    return WEP_military_RPM_boost


def equationer(higher_power, higher, lower_power, lower, alt_RAM, curvature):
    """The interpolation function."""
    power_difference = 0
    if alt_RAM >= lower:
        power_difference = (higher_power - lower_power)
    elif alt_RAM < lower:
        power_difference = (lower_power - higher_power)

    denom = air_pressurer(higher) - air_pressurer(lower)
    if abs(denom) < 1e-9:
        return lower_power

    curve_equation = lower_power + power_difference * (abs((air_pressurer(alt_RAM) - air_pressurer(lower)) / denom)) ** curvature
    return curve_equation


# ============================================================================
# Initialization functions (with debug output)
# ============================================================================

def deck_power_maker_debug(Main, Compressor, i):
    """Creates deck power values with debug output."""
    if "Power0" not in Main:
        Main["Power0"] = Main["Power"]
    if "Deck_Altitude" + str(i) not in Main:
        Main["Deck_Altitude" + str(i)] = 0

    if "Power" + str(i) in Main:
        print(f"[deck_power_maker] stage={i}: Power{i}={Main['Power' + str(i)]} (already set)")
        return

    Main["Power" + str(i)] = 0.8 * Main["Power" + str(i-1)]
    if Main["Power" + str(i)] < (0.8 * Compressor["Power" + str(i)]):
        Main["Power" + str(i)] = 0.8 * Compressor["Power" + str(i)]

    print(f"[deck_power_maker] stage={i}: Power{i}={Main['Power' + str(i)]:.1f}")


def definition_alt_power_adjuster_debug(Main, Compressor, Propeller, i):
    """Adjusts altitude/power for RPM differences with debug output."""
    # Store original values
    Compressor["Old_Power" + str(i)] = Compressor["Power" + str(i)]
    Compressor["Old_Power_new_RPM" + str(i)] = Compressor["Old_Power" + str(i)]
    Compressor["Old_Altitude" + str(i)] = Compressor["Altitude" + str(i)]

    if Ceiling_is(Compressor, i):
        Compressor["Old_Ceiling" + str(i)] = Compressor["Ceiling" + str(i)]

    # Check if adjustment is needed
    needs_adjustment = (
        ("ShaftRPMMax" in Main and Main["ShaftRPMMax"] - Main["military_RPM"] > 5 and Main["ShaftRPMMax"] - Main["WEP_RPM"] < 5) or
        ("RPMNom" in Main and Main["RPMNom"] - Main["military_RPM"] > 5) or
        ("GovernorMaxParam" in Propeller and ((Propeller["GovernorMaxParam"] - Main["military_RPM"]) > 5))
    )

    print(f"[definition_alt_power_adjuster] stage={i}: adjusted={needs_adjustment}")
    print(f"  Old_Altitude{i}={Compressor['Old_Altitude' + str(i)]}, Altitude{i}={Compressor['Altitude' + str(i)]}")
    print(f"  Old_Power{i}={Compressor['Old_Power' + str(i)]}, Power{i}={Compressor['Power' + str(i)]}")
    print(f"  Old_Power_new_RPM{i}={Compressor['Old_Power_new_RPM' + str(i)]}")


def wep_multiplier_debug(octane, Main, Compressor, i, mode):
    """Calculates WEP critical altitudes with debug output."""
    if 'AfterburnerPressureBoost' + str(i) not in Compressor:
        Compressor['AfterburnerPressureBoost' + str(i)] = 1

    Compressor["Old_Altitude_WEPboost" + str(i)] = round(altitude_at_pressure(
        air_pressurer(Compressor["Old_Altitude" + str(i)]) / Compressor['AfterburnerPressureBoost' + str(i)]))

    Main["deck_supercharger_strength" + str(i)] = Main["Military_MP"] / air_pressurer(Main["Deck_Altitude" + str(i)])
    Main["WEP_deck_supercharger_strength"] = Main["deck_supercharger_strength" + str(i)] * Main.get("WEP-mil_RPM_EffectOnSupercharger", 1) * Compressor['AfterburnerPressureBoost' + str(i)]
    Main["crit_supercharger_strength" + str(i)] = Main["Military_MP"] / air_pressurer(Compressor["Altitude" + str(i)])
    Main["WEP_crit_supercharger_strength"] = Main["crit_supercharger_strength" + str(i)] * Main.get("WEP-mil_RPM_EffectOnSupercharger", 1) * Compressor['AfterburnerPressureBoost' + str(i)]

    if all(k in Compressor for k in ("Ceiling" + str(i), "PowerAtCeiling" + str(i))):
        Main["WEP_ceil_altitude"] = Compressor["Ceiling" + str(i)]
    else:
        Main["WEP_ceil_altitude"] = 0

    if Main["Octane_MP"] == 1:
        octane = False

    if octane:
        Main["WEP_deck_altitude"] = round(altitude_at_pressure(Main["Octane_MP"] / Main["WEP_deck_supercharger_strength"]))
        Main["WEP_crit_altitude"] = round(altitude_at_pressure(Main["Octane_MP"] / Main["WEP_crit_supercharger_strength"]))
    else:
        Main["WEP_deck_altitude"] = round(altitude_at_pressure(Main["WEP_MP"] / Main["WEP_deck_supercharger_strength"]))
        Main["WEP_crit_altitude"] = round(altitude_at_pressure(Main["WEP_MP"] / Main["WEP_crit_supercharger_strength"]))

    if mode == "WEP":
        if "AfterburnerBoostMul" + str(i) not in Compressor:
            Compressor["AfterburnerBoostMul" + str(i)] = 1
        Main["WEP_power_mult"] = ((1 + ((Main["AfterburnerBoost"] - 1) * Main["OctaneAfterburnerMult"])) *
                                  Main["ThrottleBoost"] * Compressor["AfterburnerBoostMul" + str(i)]) * torquer(Main, Main["military_RPM"], Main["WEP_RPM"])
        if Compressor["AfterburnerBoostMul" + str(i)] == 0:
            Main["WEP_deck_altitude"] = 0
            Main["WEP_crit_altitude"] = Compressor["Altitude" + str(i)]
            Main["WEP_power_mult"] = 1
    else:
        Main["WEP_power_mult"] = 1

    print(f"[wep_multiplier] stage={i}: WEP_power_mult={Main['WEP_power_mult']:.4f}")
    print(f"  WEP_crit_altitude={Main['WEP_crit_altitude']}, WEP_deck_altitude={Main['WEP_deck_altitude']}")
    print(f"  deck_supercharger_strength{i}={Main['deck_supercharger_strength' + str(i)]:.4f}")
    print(f"  crit_supercharger_strength{i}={Main['crit_supercharger_strength' + str(i)]:.4f}")

    return Main["WEP_power_mult"]


def wep_rpm_ratioer_debug(Main, Compressor, Propeller):
    """Calculates RPM effect on supercharger with debug output."""
    if "ShaftRPMMax" in Main and Main["ShaftRPMMax"] - Main["military_RPM"] > 5 and Main["ShaftRPMMax"] - Main["WEP_RPM"] < 5:
        Main["default_RPM"] = Main["ShaftRPMMax"]
    elif "RPMNom" in Main and Main["RPMNom"] - Main["military_RPM"] > 5:
        Main["default_RPM"] = Main["RPMNom"]
    elif "GovernorMaxParam" in Propeller and ((Propeller["GovernorMaxParam"] - Main["military_RPM"]) > 5):
        Main["default_RPM"] = Propeller["GovernorMaxParam"]
    else:
        Main["default_RPM"] = Main["military_RPM"]

    Main["default-mil_RPM_EffectOnSupercharger"] = (1 + ((1 - Compressor["CompressorPressureAtRPM0"]) / Main["military_RPM"]) * (Main["default_RPM"] - Main["military_RPM"])) ** (1 + Compressor["CompressorOmegaFactorSq"])
    Main["WEP-mil_RPM_EffectOnSupercharger"] = (1 + ((1 - Compressor["CompressorPressureAtRPM0"]) / Main["military_RPM"]) * (Main["WEP_RPM"] - Main["military_RPM"])) ** (1 + Compressor["CompressorOmegaFactorSq"])

    print(f"[wep_rpm_ratioer] default_RPM={Main['default_RPM']}")
    print(f"  default-mil_RPM_EffectOnSupercharger={Main['default-mil_RPM_EffectOnSupercharger']:.6f}")
    print(f"  WEP-mil_RPM_EffectOnSupercharger={Main['WEP-mil_RPM_EffectOnSupercharger']:.6f}")


# ============================================================================
# variabler with debug output
# ============================================================================

def variabler_debug(Compressor, Main, i, alt_RAM, mode):
    """
    WAPC variabler() with debug output.
    Returns (higher_power, higher, lower_power, lower, curvature, branch_info)
    """
    curvature = 1
    branch = "UNKNOWN"

    if mode == "military" and alt_RAM <= Compressor["Altitude" + str(i)]:
        if ConstRPM_is(Compressor, i) and ConstRPM_is_below_deck(Compressor, i) and alt_RAM < Compressor["AltitudeConstRPM" + str(i)]:
            branch = "MIL_BELOW_CRIT_CONSTRPM_BELOW_DECK"
            higher = Compressor["AltitudeConstRPM" + str(i)]
            higher_power = 0
            lower = Compressor["AltitudeConstRPM" + str(i)] - 10
            lower_power = 0
        elif (not ConstRPM_bends_below_critalt(Compressor, i)) and not Power_is_deck_power(Main, Compressor, i):
            branch = "MIL_BELOW_CRIT_NORMAL"
            higher = Compressor["Altitude" + str(i)]
            higher_power = Compressor["Power" + str(i)]
            lower = Main["Deck_Altitude" + str(i)]
            lower_power = Main["Power" + str(i)]
        elif ConstRPM_bends_below_critalt(Compressor, i) and alt_RAM < Compressor["AltitudeConstRPM" + str(i)]:
            branch = "MIL_BELOW_CONSTRPM"
            higher = Compressor["AltitudeConstRPM" + str(i)]
            higher_power = Compressor["PowerConstRPM" + str(i)]
            lower = Main["Deck_Altitude" + str(i)]
            lower_power = Main["Power" + str(i)]
        elif ConstRPM_bends_below_critalt(Compressor, i) and alt_RAM >= Compressor["AltitudeConstRPM" + str(i)]:
            branch = "MIL_ABOVE_CONSTRPM_BELOW_CRIT"
            curvature = Compressor["PowerConstRPMCurvature" + str(i)]
            higher = Compressor["Altitude" + str(i)]
            higher_power = Compressor["Power" + str(i)]
            lower = Compressor["AltitudeConstRPM" + str(i)]
            lower_power = Compressor["PowerConstRPM" + str(i)]
        elif Power_is_deck_power(Main, Compressor, i):
            branch = "MIL_POWER_IS_DECK_POWER"
            higher = Compressor["Ceiling" + str(i)]
            higher_power = Compressor["PowerAtCeiling" + str(i)]
            lower = Compressor["Altitude" + str(i)]
            lower_power = Compressor["Power" + str(i)]
        else:
            branch = "MIL_BELOW_CRIT_FALLBACK"
            higher = Compressor["Altitude" + str(i)]
            higher_power = Compressor["Power" + str(i)]
            lower = Main["Deck_Altitude" + str(i)]
            lower_power = Main["Power" + str(i)]

    elif mode == "military" and Compressor["Altitude" + str(i)] < alt_RAM <= Compressor["Old_Altitude" + str(i)]:
        lower = Compressor["Altitude" + str(i)]
        lower_power = Compressor["Power" + str(i)]

        if not Ceiling_is_useful(Compressor, i):
            branch = "MIL_BETWEEN_CRIT_OLD_NO_CEILING"
            higher = Compressor["Old_Altitude" + str(i)]
            higher_power = ((equationer(Compressor["Old_Power_new_RPM" + str(i)], Compressor["Altitude" + str(i)],
                                        Main["Power" + str(i)], Main["Deck_Altitude" + str(i)],
                                        Compressor["Altitude" + str(i)], curvature))
                            * (air_pressurer(Compressor["Old_Altitude" + str(i)]) / air_pressurer(Compressor["Altitude" + str(i)])))
        elif Ceiling_is_useful(Compressor, i) and not ConstRPM_bends_above_crit_alt(Compressor, i):
            if Compressor["ExactAltitudes"]:
                branch = "MIL_BETWEEN_CRIT_OLD_CEILING_EXACT"
                higher = Compressor["Old_Altitude" + str(i)]
                higher_power = equationer(Compressor["PowerAtCeiling" + str(i)],
                                          altitude_at_pressure(air_pressurer(Compressor["Ceiling" + str(i)]) * (
                                                  air_pressurer(Compressor["Altitude" + str(i)]) / air_pressurer(Compressor["Altitude" + str(i)]))),
                                          Compressor["Old_Power_new_RPM" + str(i)], Compressor["Altitude" + str(i)],
                                          Compressor["Old_Altitude" + str(i)], curvature)
            else:
                branch = "MIL_BETWEEN_CRIT_OLD_CEILING_NON_EXACT"
                higher = Compressor["Ceiling" + str(i)]
                higher_power = Compressor["PowerAtCeiling" + str(i)]
        elif Ceiling_is_useful(Compressor, i) and ConstRPM_bends_above_crit_alt(Compressor, i):
            curvature = Compressor.get("PowerConstRPMCurvature" + str(i), 1)
            if Compressor["ExactAltitudes"]:
                branch = "MIL_BETWEEN_CRIT_OLD_CEILING_CONSTRPM_EXACT"
                higher = Compressor["Old_Altitude" + str(i)]
                higher_power = equationer(Compressor["PowerAtCeiling" + str(i)],
                                          altitude_at_pressure(air_pressurer(Compressor["Ceiling" + str(i)]) * (
                                                  air_pressurer(Compressor["Altitude" + str(i)]) / air_pressurer(Compressor["Altitude" + str(i)]))),
                                          Compressor["Old_Power_new_RPM" + str(i)], Compressor["Altitude" + str(i)],
                                          Compressor["Old_Altitude" + str(i)], curvature)
            else:
                branch = "MIL_BETWEEN_CRIT_OLD_CEILING_CONSTRPM_NON_EXACT"
                higher = Compressor["Ceiling" + str(i)]
                higher_power = Compressor["PowerAtCeiling" + str(i)]
        else:
            branch = "MIL_BETWEEN_CRIT_OLD_FALLBACK"
            higher = Compressor["Ceiling" + str(i)]
            higher_power = Compressor["PowerAtCeiling" + str(i)]

    elif mode == "military" and Compressor["Old_Altitude" + str(i)] < alt_RAM:
        if not Ceiling_is_useful(Compressor, i):
            branch = "MIL_ABOVE_OLD_NO_CEILING"
            lower = Compressor["Old_Altitude" + str(i)]
            lower_power = ((equationer(Compressor["Old_Power_new_RPM" + str(i)], Compressor["Altitude" + str(i)],
                                       Main["Power" + str(i)], Main["Deck_Altitude" + str(i)],
                                       Compressor["Altitude" + str(i)], curvature))
                           * (air_pressurer(Compressor["Old_Altitude" + str(i)]) / air_pressurer(Compressor["Altitude" + str(i)])))
            higher = alt_RAM
            higher_power = lower_power * (air_pressurer(alt_RAM) / air_pressurer(lower))
        elif Ceiling_is_useful(Compressor, i) and not ConstRPM_bends_above_crit_alt(Compressor, i):
            if Compressor["ExactAltitudes"]:
                branch = "MIL_ABOVE_OLD_CEILING_EXACT"
                lower = Compressor["Old_Altitude" + str(i)]
                lower_power = equationer(Compressor["PowerAtCeiling" + str(i)],
                                         altitude_at_pressure(air_pressurer(Compressor["Ceiling" + str(i)]) * (
                                                 air_pressurer(Compressor["Altitude" + str(i)]) / air_pressurer(Compressor["Altitude" + str(i)]))),
                                         Compressor["Old_Power_new_RPM" + str(i)], Compressor["Altitude" + str(i)],
                                         Compressor["Old_Altitude" + str(i)], curvature)
                higher = Compressor["Ceiling" + str(i)]
                higher_power = Compressor["PowerAtCeiling" + str(i)]
            else:
                branch = "MIL_ABOVE_OLD_CEILING_NON_EXACT"
                lower = Compressor["Altitude" + str(i)]
                lower_power = Compressor["Power" + str(i)]
                higher = Compressor["Ceiling" + str(i)]
                higher_power = Compressor["PowerAtCeiling" + str(i)]
        elif Ceiling_is_useful(Compressor, i) and ConstRPM_bends_above_crit_alt(Compressor, i):
            curvature = Compressor.get("PowerConstRPMCurvature" + str(i), 1)
            if Compressor["ExactAltitudes"]:
                branch = "MIL_ABOVE_OLD_CEILING_CONSTRPM_EXACT"
                higher = Compressor["Ceiling" + str(i)]
                higher_power = Compressor["PowerAtCeiling" + str(i)]
                lower = Compressor["Old_Altitude" + str(i)]
                lower_power = equationer(Compressor["PowerAtCeiling" + str(i)],
                                         altitude_at_pressure(air_pressurer(Compressor["Ceiling" + str(i)]) * (
                                                 air_pressurer(Compressor["Altitude" + str(i)]) / air_pressurer(Compressor["Altitude" + str(i)]))),
                                         Compressor["Old_Power_new_RPM" + str(i)], Compressor["Altitude" + str(i)],
                                         Compressor["Old_Altitude" + str(i)], curvature)
            else:
                branch = "MIL_ABOVE_OLD_CEILING_CONSTRPM_NON_EXACT"
                higher = Compressor["Ceiling" + str(i)]
                higher_power = Compressor["PowerAtCeiling" + str(i)]
                lower = Compressor["Old_Altitude" + str(i)]
                lower_power = Compressor["Power" + str(i)]
        else:
            branch = "MIL_ABOVE_OLD_FALLBACK"
            lower = Compressor["Altitude" + str(i)]
            lower_power = Compressor["Power" + str(i)]
            higher = Compressor["Ceiling" + str(i)]
            higher_power = Compressor["PowerAtCeiling" + str(i)]

    # For WEP mode (simplified - Yak-3 has no WEP)
    elif mode == "WEP":
        # Since Yak-3 has WEP_power_mult=1, WEP behaves same as military
        # This is a simplified path for this debug script
        branch = "WEP_SAME_AS_MILITARY"
        return variabler_debug(Compressor, Main, i, alt_RAM, "military")

    return higher_power, higher, lower_power, lower, curvature, branch


def rameffect_er_debug(alt, air_temp, speed, speed_type, speed_manifold_mult):
    """RAM effect calculation with debug output."""
    air_pressure = air_pressurer(alt)
    air_density = air_densitier(air_pressure, air_temp, alt)

    if speed == 0:
        return alt, "no_speed"

    if speed_type == "IAS":
        TASspeed = ias_tas_er(speed, air_density)
    else:
        TASspeed = speed

    dynamic_pressure = (((air_density * ((TASspeed / 3.6) ** 2)) / 2) * speed_manifold_mult) / 101325
    total_pressure = air_pressure + dynamic_pressure
    alt_RAM = int(altitude_at_pressure(total_pressure))  # NOTE: int() truncation!

    return alt_RAM, f"TAS={TASspeed:.1f}, q={dynamic_pressure:.6f}, p_total={total_pressure:.6f}"


# ============================================================================
# Main debug output
# ============================================================================

def main():
    print("=" * 70)
    print("WAPC Yak-3 Power Curve Debug Output")
    print("=" * 70)

    # Determine blkx file path
    blkx_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_BLKX_PATH
    print(f"\nUsing blkx file: {blkx_path}")

    # === VALIDATE HARDCODED PARAMETERS ===
    blkx_values = parse_blkx_file(blkx_path)
    validate_hardcoded_params(blkx_values)

    Compressor, Main, Propeller = setup_yak3_params()
    num_stages = 2

    # === PARAMETER EXTRACTION ===
    print("\n=== PARAMETER EXTRACTION ===\n")

    # Initialize wep_rpm_ratioer
    wep_rpm_ratioer_debug(Main, Compressor, Propeller)
    print()

    # Process each stage
    for i in range(num_stages):
        print(f"--- Stage {i} ---")
        definition_alt_power_adjuster_debug(Main, Compressor, Propeller, i)
        deck_power_maker_debug(Main, Compressor, i)
        print()

    # WEP multiplier for each stage
    for i in range(num_stages):
        wep_multiplier_debug(False, Main, Compressor, i, "military")
        print()

    # Ceiling_is_useful check
    print("--- Ceiling_is_useful checks ---")
    for i in range(num_stages):
        useful = Ceiling_is_useful(Compressor, i)
        ceil_alt = Compressor.get("Ceiling" + str(i), 0)
        alt_i = Compressor["Altitude" + str(i)]
        pwr_i = Compressor["Power" + str(i)]
        ceil_pwr = Compressor.get("PowerAtCeiling" + str(i), 0)
        print(f"[Ceiling_is_useful] stage={i}: {useful}")
        print(f"  Ceiling{i}={ceil_alt}, Altitude{i}={alt_i}, diff={ceil_alt - alt_i}")
        print(f"  Power{i}={pwr_i}, PowerAtCeiling{i}={ceil_pwr}, diff={pwr_i - ceil_pwr}")
    print()

    # === POWER CURVE (static, speed=0) ===
    print("=== POWER CURVE (military, static, stage 0) ===\n")

    altitudes = [0, 100, 200, 300, 400, 500, 675, 1000, 1500, 2000, 2600, 3000, 5000, 7000, 9000, 10000]

    for alt in altitudes:
        i = 0  # Stage 0
        alt_RAM = alt  # No RAM effect

        higher_power, higher, lower_power, lower, curvature, branch = variabler_debug(Compressor, Main, i, alt_RAM, "military")
        power = equationer(higher_power, higher, lower_power, lower, alt_RAM, curvature)

        print(f"[alt={alt}] ramAlt={alt_RAM}")
        print(f"  variabler: branch={branch}")
        print(f"    lower=({lower}, {lower_power:.1f}), higher=({higher}, {higher_power:.1f}), curv={curvature}")
        print(f"  equationer: power={power:.1f}")
        print()

    # === POWER CURVE (with RAM, 301 km/h IAS) ===
    print("=== POWER CURVE (military, 301km/h IAS, stage 0) ===\n")

    speed = 301
    air_temp = 15
    speed_type = "IAS"
    speed_manifold_mult = Compressor["SpeedManifoldMultiplier"]

    for alt in altitudes:
        i = 0  # Stage 0
        alt_RAM, ram_info = rameffect_er_debug(alt, air_temp, speed, speed_type, speed_manifold_mult)

        higher_power, higher, lower_power, lower, curvature, branch = variabler_debug(Compressor, Main, i, alt_RAM, "military")
        power = equationer(higher_power, higher, lower_power, lower, alt_RAM, curvature)

        print(f"[alt={alt}] ramAlt={alt_RAM} ({ram_info})")
        print(f"  variabler: branch={branch}")
        print(f"    lower=({lower}, {lower_power:.1f}), higher=({higher}, {higher_power:.1f}), curv={curvature}")
        print(f"  equationer: power={power:.1f}")
        print()

    # === OPTIMAL POWER (both stages) ===
    print("=== OPTIMAL POWER (military, static, both stages) ===\n")

    for alt in altitudes:
        powers = []
        for i in range(num_stages):
            alt_RAM = alt
            higher_power, higher, lower_power, lower, curvature, branch = variabler_debug(Compressor, Main, i, alt_RAM, "military")
            power = equationer(higher_power, higher, lower_power, lower, alt_RAM, curvature)
            powers.append(power)

        optimal = max(powers)
        optimal_stage = powers.index(optimal)
        print(f"[alt={alt}] stage0={powers[0]:.1f}hp, stage1={powers[1]:.1f}hp -> optimal={optimal:.1f}hp (stage {optimal_stage})")

    print("\n" + "=" * 70)
    print("Debug output complete")
    print("=" * 70)


if __name__ == "__main__":
    main()
