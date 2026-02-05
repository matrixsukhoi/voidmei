#!/usr/bin/env python3
# /// script
# requires-python = ">=3.8"
# dependencies = [
#     "matplotlib",
# ]
# ///
"""
Generate Yak-3 power curve at 300 km/h IAS using WTAPC functions.

This script:
1. Parses raw War Thunder .blkx text files (not JSON datamine format)
2. Converts to WTAPC-compatible nested dictionary structure
3. Calls WTAPC core calculation functions directly
4. Applies RAM effect at specified speed
5. Outputs power vs altitude curve with key nodes

Usage:
    uv run script/wtapc_yak3_power_curve.py [path/to/yak-3.blkx] [--speed N] [--plot]

Example:
    PYTHONPATH=/home/tu10ng/projects/wtapc/performance_calculators \
    uv run script/wtapc_yak3_power_curve.py \
        /home/tu10ng/Downloads/voidmei/data/aces/gamedata/flightmodels/fm/yak-3.blkx \
        --speed 300 --plot output.png
"""

import sys
import os
import re
import argparse

# Add WTAPC to path
WTAPC_PATH = '/home/tu10ng/projects/wtapc/performance_calculators'
if WTAPC_PATH not in sys.path:
    sys.path.insert(0, WTAPC_PATH)

# Import WTAPC core functions
from ram_pressure_density_calculator import (
    air_pressurer,
    altitude_at_pressure,
    air_densitier,
    ias_tas_er
)
from plane_power_calculator import (
    engine_shortcuter,
    rpm_er,
    wep_mp_er,
    wep_rpm_ratioer,
    definition_alt_power_adjuster,
    deck_power_maker,
    wep_mulitiplierer,
    variabler,
    equationer,
    old_type_fm_detector,
    exception_fixer,
)

# Default paths
DEFAULT_BLKX_PATH = "/home/tu10ng/Downloads/voidmei/data/aces/gamedata/flightmodels/fm/yak-3.blkx"


# =============================================================================
# BLKX Text Parser
# =============================================================================

def parse_blkx_value(value_str, type_suffix):
    """Parse a value string based on its type suffix."""
    value_str = value_str.strip()

    if type_suffix == 'b':  # Boolean
        return value_str.lower() in ('true', 'yes', '1')
    elif type_suffix == 'i':  # Integer
        return int(value_str)
    elif type_suffix == 'r':  # Real (float)
        return float(value_str)
    elif type_suffix == 't':  # Text/string
        return value_str.strip('"')
    elif type_suffix == 'p2':  # Point2 (two floats)
        parts = [float(x.strip()) for x in value_str.split(',')]
        return parts if len(parts) == 2 else parts
    elif type_suffix == 'p3':  # Point3 (three floats)
        parts = [float(x.strip()) for x in value_str.split(',')]
        return parts
    elif type_suffix == 'p4':  # Point4 (four floats)
        parts = [float(x.strip()) for x in value_str.split(',')]
        return parts
    else:
        # Unknown type - try float, then int, then string
        try:
            if '.' in value_str:
                return float(value_str)
            return int(value_str)
        except ValueError:
            return value_str


def parse_blkx_text(content):
    """
    Parse War Thunder .blkx text format into nested dictionary.

    Format examples:
        key:r = 1.5           -> {"key": 1.5}
        key:i = 10            -> {"key": 10}
        key:b = true          -> {"key": True}
        key:t = "text"        -> {"key": "text"}
        key:p2 = 1.0, 2.0     -> {"key": [1.0, 2.0]}
        Block { ... }         -> {"Block": {...}}
    """
    result = {}
    lines = content.split('\n')
    i = 0

    while i < len(lines):
        line = lines[i].strip()

        # Skip empty lines and comments
        if not line or line.startswith('//'):
            i += 1
            continue

        # Check for block start: "BlockName {"
        block_match = re.match(r'^(\w+)\s*\{', line)
        if block_match:
            block_name = block_match.group(1)
            # Find matching closing brace
            brace_count = 1
            block_start = i
            j = i + 1

            while j < len(lines) and brace_count > 0:
                check_line = lines[j]
                brace_count += check_line.count('{')
                brace_count -= check_line.count('}')
                j += 1

            # Extract block content (excluding braces)
            block_lines = lines[block_start + 1:j - 1]
            block_content = '\n'.join(block_lines)

            # Recursively parse block
            parsed_block = parse_blkx_text(block_content)

            # Handle duplicate block names (append number)
            if block_name in result:
                # Find next available name
                suffix = 1
                while f"{block_name}_{suffix}" in result:
                    suffix += 1
                block_name = f"{block_name}_{suffix}"

            result[block_name] = parsed_block
            i = j
            continue

        # Check for key:type = value
        kv_match = re.match(r'^(\w+):(\w+)\s*=\s*(.+)$', line)
        if kv_match:
            key = kv_match.group(1)
            type_suffix = kv_match.group(2)
            value_str = kv_match.group(3)

            value = parse_blkx_value(value_str, type_suffix)
            result[key] = value
            i += 1
            continue

        # Check for closing brace only
        if line == '}':
            i += 1
            continue

        # Unknown line format
        i += 1

    return result


def load_blkx_file(filepath):
    """Load and parse a .blkx text file."""
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"Blkx file not found: {filepath}")

    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()

    return parse_blkx_text(content)


# =============================================================================
# WTAPC Dictionary Builder
# =============================================================================

def build_wtapc_fm_dict(parsed_blkx):
    """
    Transform parsed blkx into WTAPC-expected fm_dict structure.

    WTAPC expects:
    - fm_dict["EngineType0"] or fm_dict["Engine0"] containing:
      - "Main": {Power, ThrottleBoost, AfterburnerBoost, ...}
      - "Compressor": {Altitude0, Power0, Ceiling0, ...}
      - "Afterburner": {"IsControllable": bool}
      - "Propellor": {ThrottleRPMAuto0, GovernorMaxParam, ...}
      - "Temperature": {Mode0, Mode1, ...}

    Key mapping difference from raw blkx → datamine JSON:
      In raw blkx, ThrottleRPMAuto keys are in EngineType0.Main.
      In datamine JSON, they're in EngineType0.Propellor.
      WTAPC's engine_shortcuter sets Propeller=Engine["Propellor"] if
      it exists, otherwise falls back to Main. rpm_er then searches
      Propeller for ThrottleRPMAuto keys.

      We build "Propellor" by merging Governor data AND ThrottleRPMAuto
      entries from Main, matching the datamine JSON structure.
    """
    fm_dict = {}

    # Copy EngineType0 if present
    if "EngineType0" in parsed_blkx:
        fm_dict["EngineType0"] = parsed_blkx["EngineType0"].copy()

        # Ensure required nested dicts exist
        if "Main" not in fm_dict["EngineType0"]:
            fm_dict["EngineType0"]["Main"] = {}
        if "Compressor" not in fm_dict["EngineType0"]:
            fm_dict["EngineType0"]["Compressor"] = {}
        if "Afterburner" not in fm_dict["EngineType0"]:
            fm_dict["EngineType0"]["Afterburner"] = {"IsControllable": False}
        if "Temperature" not in fm_dict["EngineType0"]:
            fm_dict["EngineType0"]["Temperature"] = {}

        # Build "Propellor" dict (note WTAPC's spelling)
        # Merge: Governor data + ThrottleRPMAuto entries from Main
        propellor = {}

        # Copy Governor data from PropellerType0
        if "PropellerType0" in parsed_blkx and "Governor" in parsed_blkx["PropellerType0"]:
            propellor.update(parsed_blkx["PropellerType0"]["Governor"])

        # Move ThrottleRPMAuto entries from Main into Propellor
        # (this matches how datamine JSON structures the data)
        main = fm_dict["EngineType0"]["Main"]
        for key in list(main.keys()):
            if key.startswith("ThrottleRPMAuto"):
                propellor[key] = main[key]

        fm_dict["EngineType0"]["Propellor"] = propellor

    # Also copy Engine0 for reference
    if "Engine0" in parsed_blkx:
        fm_dict["Engine0"] = parsed_blkx["Engine0"].copy()

    # Copy PropellerType0 for Governor access
    if "PropellerType0" in parsed_blkx:
        fm_dict["PropellerType0"] = parsed_blkx["PropellerType0"].copy()

    return fm_dict


# =============================================================================
# RAM Effect Calculator (fixed version)
# =============================================================================

def rameffect_er_fixed(alt, air_temp, speed, speed_type, speed_manifold_mult):
    """
    Calculate RAM effect equivalent altitude.

    Note: WTAPC's rameffect_er has a bug where it expects the scalar
    SpeedManifoldMultiplier but power_curve_culator passes the whole
    Compressor dict. We accept the scalar directly.
    """
    air_pressure = air_pressurer(alt)
    air_density = air_densitier(air_pressure, air_temp, alt)

    if speed == 0:
        return alt

    if speed_type == "IAS":
        TASspeed = ias_tas_er(speed, air_density)
    else:
        TASspeed = speed

    dynamic_pressure = (((air_density * ((TASspeed / 3.6) ** 2)) / 2) * speed_manifold_mult) / 101325
    total_pressure = air_pressure + dynamic_pressure
    alt_RAM = int(altitude_at_pressure(total_pressure))

    return alt_RAM


# =============================================================================
# Power Curve Calculator
# =============================================================================

def calculate_power_curve(fm_dict, speed=300, speed_type="IAS", air_temp=15,
                          alt_min=0, alt_max=12000, alt_step=50):
    """
    Calculate power curve using WTAPC functions.

    Returns:
        dict: {
            'stages': {0: [...], 1: [...]},  # Power at each alt for each stage
            'optimal': [...],                 # Max power across stages
            'altitudes': [...],               # Altitude values
            'speed_mult': float,              # SpeedManifoldMultiplier
            'key_points': [...]               # Notable altitude/power points
        }
    """
    # Extract engine components using WTAPC's function
    Engine, Compressor, Main, Afterburner, Propeller = engine_shortcuter(fm_dict)

    print(f"Engine shortcuter results:")
    print(f"  Afterburner controllable: {Afterburner}")
    print(f"  Main.Power: {Main.get('Power', 'N/A')}")
    print(f"  Compressor.ExactAltitudes: {Compressor.get('ExactAltitudes', 'N/A')}")
    print()

    # Count compressor stages
    num_stages = 0
    for stage in range(6):
        if f"Power{stage}" in Compressor:
            num_stages = stage + 1
    print(f"Number of compressor stages: {num_stages}")

    # Run WTAPC initialization functions
    old_type_fm_detector(Compressor, Main)
    exception_fixer("yak-3", Compressor, Main)
    rpm_er(fm_dict, Main, Propeller)
    wep_rpm_ratioer(Main, Compressor, Propeller)
    wep_mp_er(Engine, Compressor, Main, Afterburner)

    print(f"\nRPM values:")
    print(f"  military_RPM: {Main.get('military_RPM', 'N/A')}")
    print(f"  WEP_RPM: {Main.get('WEP_RPM', 'N/A')}")
    print(f"  default_RPM: {Main.get('default_RPM', 'N/A')}")
    print(f"  WEP-mil_RPM_EffectOnSupercharger: {Main.get('WEP-mil_RPM_EffectOnSupercharger', 'N/A')}")

    # Initialize each stage
    for i in range(num_stages):
        definition_alt_power_adjuster(Main, Compressor, Propeller, i)
        deck_power_maker(Main, Compressor, i)

    print(f"\nAfter initialization:")
    for i in range(num_stages):
        print(f"  Stage {i}: Altitude={Compressor.get(f'Altitude{i}', 'N/A')}, "
              f"Power={Compressor.get(f'Power{i}', 'N/A')}, "
              f"Old_Altitude={Compressor.get(f'Old_Altitude{i}', 'N/A')}")

    # Get speed multiplier
    speed_mult = Compressor.get("SpeedManifoldMultiplier", 1.0)
    print(f"\nSpeedManifoldMultiplier: {speed_mult}")
    print(f"Speed: {speed} {speed_type}")

    # Calculate power at each altitude
    altitudes = list(range(alt_min, alt_max + 1, alt_step))
    stage_powers = {i: [] for i in range(num_stages)}
    optimal_powers = []
    stage_selections = []
    key_points = []

    # Since Yak-3 has no WEP (AfterburnerBoost=1, IsControllable=false),
    # we calculate military mode only
    mode = "military"

    for i in range(num_stages):
        wep_mulitiplierer(False, Main, Compressor, i, mode)

    for alt in altitudes:
        # Calculate RAM effect
        alt_RAM = rameffect_er_fixed(alt, air_temp, speed, speed_type, speed_mult)

        # Calculate power for each stage
        powers = []
        for i in range(num_stages):
            higher_power, higher, lower_power, lower, curvature = variabler(
                Compressor, Main, i, alt_RAM, mode
            )
            power = equationer(higher_power, higher, lower_power, lower, alt_RAM, curvature)
            powers.append(power)
            stage_powers[i].append(round(power, 1))

        # Select optimal (max power)
        optimal = max(powers)
        optimal_stage = powers.index(optimal)
        optimal_powers.append(round(optimal, 1))
        stage_selections.append(optimal_stage)

    # Identify key points - actual curve inflection points, NOT FM static values
    key_altitudes = set()

    # Find actual peak power altitude (overall)
    peak_idx = optimal_powers.index(max(optimal_powers))
    key_altitudes.add(altitudes[peak_idx])

    # Find per-stage peaks (where each stage reaches max power)
    for i in range(num_stages):
        stage_peak_idx = stage_powers[i].index(max(stage_powers[i]))
        key_altitudes.add(altitudes[stage_peak_idx])

    # Find stage switch points
    prev_stage = stage_selections[0]
    for idx, stage in enumerate(stage_selections):
        if stage != prev_stage:
            key_altitudes.add(altitudes[idx])
            prev_stage = stage

    # Find where optimal curve crosses FM ceiling power values (actual ceiling effect points)
    for i in range(num_stages):
        ceil_pwr = Compressor.get(f"PowerAtCeiling{i}")
        if ceil_pwr:
            # Find altitude where this stage's power drops to ceiling power
            for idx, pwr in enumerate(stage_powers[i]):
                if pwr <= ceil_pwr and idx > 0 and stage_powers[i][idx-1] > ceil_pwr:
                    key_altitudes.add(altitudes[idx])
                    break

    # Add start and end points
    key_altitudes.add(altitudes[0])
    key_altitudes.add(altitudes[-1])

    # Calculate key points
    peak_power = max(optimal_powers)
    for alt in sorted(key_altitudes):
        if alt < alt_min or alt > alt_max:
            continue
        alt_RAM = rameffect_er_fixed(alt, air_temp, speed, speed_type, speed_mult)
        powers = []
        for i in range(num_stages):
            higher_power, higher, lower_power, lower, curvature = variabler(
                Compressor, Main, i, alt_RAM, mode
            )
            power = equationer(higher_power, higher, lower_power, lower, alt_RAM, curvature)
            powers.append(power)
        optimal = max(powers)
        optimal_stage = powers.index(optimal)
        is_peak = abs(optimal - peak_power) < 0.5
        key_points.append({
            'altitude': alt,
            'alt_RAM': alt_RAM,
            'power': round(optimal, 1),
            'stage': optimal_stage,
            'stage_powers': [round(p, 1) for p in powers],
            'is_peak': is_peak,
        })

    return {
        'stages': stage_powers,
        'optimal': optimal_powers,
        'altitudes': altitudes,
        'speed_mult': speed_mult,
        'key_points': key_points,
        'num_stages': num_stages,
        'Compressor': Compressor,
        'Main': Main,
    }


# =============================================================================
# Output Formatting
# =============================================================================

def print_power_curve(result, speed, speed_type):
    """Print power curve in a readable format."""
    print("\n" + "=" * 70)
    print(f"Yak-3 Power Curve @ {speed} km/h {speed_type}")
    print("=" * 70)

    # Key points
    print("\n--- Key Points ---")
    print(f"{'Alt(m)':>8} {'RAM(m)':>8} {'Power(hp)':>10} {'Stage':>6} {'Powers by Stage':>20}")
    print("-" * 60)
    for kp in result['key_points']:
        powers_str = ', '.join([f"{p:.0f}" for p in kp['stage_powers']])
        print(f"{kp['altitude']:>8} {kp['alt_RAM']:>8} {kp['power']:>10.1f} {kp['stage']:>6} [{powers_str}]")

    # Full curve (sampled)
    print("\n--- Power Curve (sampled) ---")
    print(f"{'Alt(m)':>8} {'Power(hp)':>10} {'Stage':>6}")
    print("-" * 30)

    alts = result['altitudes']
    powers = result['optimal']

    # Show every 500m or key points
    sample_alts = set(range(0, max(alts) + 1, 500))
    for kp in result['key_points']:
        sample_alts.add(kp['altitude'])

    prev_stage = None
    for idx, (alt, power) in enumerate(zip(alts, powers)):
        # Determine stage
        stage_powers_at_alt = [result['stages'][i][idx] for i in range(result['num_stages'])]
        stage = stage_powers_at_alt.index(max(stage_powers_at_alt))

        if alt in sample_alts or stage != prev_stage:
            marker = " *" if stage != prev_stage and prev_stage is not None else ""
            print(f"{alt:>8} {power:>10.1f} {stage:>6}{marker}")
            prev_stage = stage

    # Stage comparison
    print("\n--- Stage Comparison at Critical Altitudes ---")
    Compressor = result['Compressor']
    for i in range(result['num_stages']):
        crit_alt = Compressor.get(f'Altitude{i}', 'N/A')
        crit_pwr = Compressor.get(f'Power{i}', 'N/A')
        ceil_alt = Compressor.get(f'Ceiling{i}', 'N/A')
        ceil_pwr = Compressor.get(f'PowerAtCeiling{i}', 'N/A')
        print(f"Stage {i}: Critical={crit_alt}m @ {crit_pwr}hp, Ceiling={ceil_alt}m @ {ceil_pwr}hp")


def plot_power_curve(result, speed, speed_type, save_path=None):
    """Plot power curve with matplotlib."""
    import matplotlib.pyplot as plt

    alts = result['altitudes']
    num_stages = result['num_stages']

    fig, ax = plt.subplots(figsize=(10, 6))

    # Plot each stage as a thin dashed line
    stage_colors = ['#4a90d9', '#e06c5a']
    stage_labels = ['Stage 0 (low gear)', 'Stage 1 (high gear)']
    for i in range(num_stages):
        ax.plot(alts, result['stages'][i],
                linestyle='--', linewidth=1, alpha=0.5,
                color=stage_colors[i], label=stage_labels[i])

    # Plot optimal envelope as a thick solid line
    ax.plot(alts, result['optimal'],
            linewidth=2.5, color='#2d2d2d', label='Optimal (max)')

    # Mark key points on the curve with annotations
    for kp in result['key_points']:
        is_peak = kp.get('is_peak', False)
        marker_size = 10 if is_peak else 6
        marker_style = '*' if is_peak else 'o'
        ax.plot(kp['altitude'], kp['power'], marker_style,
                color=stage_colors[kp['stage']], markersize=marker_size, zorder=5)
        if is_peak:
            label = f"{kp['altitude']}m / {kp['power']:.0f}hp  ★PEAK"
        else:
            label = f"{kp['altitude']}m / {kp['power']:.0f}hp"
        # Alternate annotation offset to reduce overlap
        offset_y = 14 if kp['power'] > result['optimal'][0] else -22
        ax.annotate(label,
                    xy=(kp['altitude'], kp['power']),
                    xytext=(10, offset_y), textcoords='offset points',
                    fontsize=8.5, color='#222222' if is_peak else '#444444',
                    fontweight='bold' if is_peak else 'normal',
                    arrowprops=dict(arrowstyle='-', color='#aaaaaa', lw=0.6) if abs(offset_y) > 15 else None)

    ax.set_xlabel('Altitude (m)', fontsize=12)
    ax.set_ylabel('Power (hp)', fontsize=12)
    ax.set_title(f'Yak-3 (VK-105PF2) Power vs Altitude @ {speed:.0f} km/h {speed_type}',
                 fontsize=13, fontweight='bold')
    ax.legend(loc='upper right', fontsize=9)
    ax.grid(True, alpha=0.3)
    ax.set_xlim(0, max(alts))
    ax.set_ylim(0, None)

    plt.tight_layout()

    if save_path:
        fig.savefig(save_path, dpi=150)
        print(f"\nPlot saved to: {save_path}")
    else:
        plt.show()


def print_csv_output(result, speed, speed_type):
    """Print power curve as CSV for plotting."""
    print(f"\n# Yak-3 Power Curve @ {speed} km/h {speed_type}")
    print("# Alt(m),Power(hp),Stage,Stage0_Power,Stage1_Power")

    alts = result['altitudes']
    powers = result['optimal']

    for idx, (alt, power) in enumerate(zip(alts, powers)):
        stage_powers = [result['stages'][i][idx] for i in range(result['num_stages'])]
        stage = stage_powers.index(max(stage_powers))
        row = [str(alt), f"{power:.1f}", str(stage)]
        row.extend([f"{sp:.1f}" for sp in stage_powers])
        print(",".join(row))


# =============================================================================
# Main
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Generate Yak-3 power curve using WTAPC"
    )
    parser.add_argument(
        'blkx_path',
        nargs='?',
        default=DEFAULT_BLKX_PATH,
        help=f"Path to yak-3.blkx file (default: {DEFAULT_BLKX_PATH})"
    )
    parser.add_argument(
        '--speed', '-s',
        type=float,
        default=300,
        help="Airspeed for RAM effect (default: 300 km/h)"
    )
    parser.add_argument(
        '--speed-type', '-t',
        choices=['IAS', 'TAS'],
        default='IAS',
        help="Speed type (default: IAS)"
    )
    parser.add_argument(
        '--csv',
        action='store_true',
        help="Output as CSV for plotting"
    )
    parser.add_argument(
        '--plot',
        nargs='?',
        const=True,
        default=False,
        metavar='PATH',
        help="Plot power curve (optionally save to PATH, e.g., --plot output.png)"
    )
    parser.add_argument(
        '--alt-max',
        type=int,
        default=12000,
        help="Maximum altitude (default: 12000m)"
    )
    parser.add_argument(
        '--alt-step',
        type=int,
        default=50,
        help="Altitude step (default: 50m)"
    )

    args = parser.parse_args()

    print(f"Loading blkx file: {args.blkx_path}")

    # Parse blkx file
    try:
        parsed_blkx = load_blkx_file(args.blkx_path)
    except FileNotFoundError as e:
        print(f"Error: {e}")
        sys.exit(1)

    print(f"Parsed {len(parsed_blkx)} top-level blocks")

    # Build WTAPC-compatible dict
    fm_dict = build_wtapc_fm_dict(parsed_blkx)

    if "EngineType0" not in fm_dict:
        print("Error: Could not find EngineType0 in parsed file")
        sys.exit(1)

    print(f"Built fm_dict with EngineType0")

    # Calculate power curve
    result = calculate_power_curve(
        fm_dict,
        speed=args.speed,
        speed_type=args.speed_type,
        alt_max=args.alt_max,
        alt_step=args.alt_step
    )

    # Output results
    if args.plot:
        save_path = args.plot if isinstance(args.plot, str) else None
        plot_power_curve(result, args.speed, args.speed_type, save_path)
    elif args.csv:
        print_csv_output(result, args.speed, args.speed_type)
    else:
        print_power_curve(result, args.speed, args.speed_type)

    # Summary
    print("\n" + "=" * 70)
    print("Verification Summary")
    print("=" * 70)

    Compressor = result['Compressor']
    Main = result['Main']

    print(f"\nFrom FM file:")
    print(f"  Sea level power: {Main.get('Power', 'N/A')} hp")
    print(f"  Stage 0 critical: {Compressor.get('Altitude0', 'N/A')}m @ {Compressor.get('Power0', 'N/A')} hp")
    print(f"  Stage 1 critical: {Compressor.get('Altitude1', 'N/A')}m @ {Compressor.get('Power1', 'N/A')} hp")
    print(f"  Stage 0 ceiling: {Compressor.get('Ceiling0', 'N/A')}m @ {Compressor.get('PowerAtCeiling0', 'N/A')} hp")
    print(f"  Stage 1 ceiling: {Compressor.get('Ceiling1', 'N/A')}m @ {Compressor.get('PowerAtCeiling1', 'N/A')} hp")

    # Find power at verification altitudes
    print(f"\nCalculated power @ {args.speed} km/h {args.speed_type}:")
    check_alts = [0, 300, 2600, 5000, 9000]
    for check_alt in check_alts:
        if check_alt in result['altitudes']:
            idx = result['altitudes'].index(check_alt)
            power = result['optimal'][idx]
            print(f"  {check_alt}m: {power:.1f} hp")


if __name__ == "__main__":
    main()
