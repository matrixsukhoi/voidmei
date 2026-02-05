# Utility Classes Development Guide

## Overview

The `prog.util` package provides **pure function utilities** for physics calculations, interpolation, and atmospheric modeling. All classes are stateless, thread-safe, and designed for zero-allocation in hot paths.

## File Overview

| File | Responsibility |
|------|----------------|
| `PhysicsConstants.java` | Physical constants (gravity, atmospheric parameters) |
| `Interpolation.java` | 1D/2D interpolation, linear interpolation utilities |
| `AtmosphereModel.java` | ISA standard atmosphere: pressure, density, IAS↔TAS, RAM effect |
| `PistonPowerModel.java` | Piston engine power curves: altitude/speed → power calculation |
| `PowerCurveHelper.java` | Power curve shape determination (hasConstRpm, ceilingIsUseful, etc.) |
| `FMPowerExtractor.java` | FM file → CompressorStageParams extraction (with fuel modifications) |
| `CalcHelper.java` | General math utilities |
| `HttpHelper.java` | HTTP request utilities for game API |
| `Logger.java` | Application logging |
| `StringHelper.java` | String formatting utilities |
| `FileUtils.java` | File I/O utilities |
| `FormulaEvaluator.java` | Runtime formula evaluation via reflection |

---

## Physics & Atmosphere Utilities

### PhysicsConstants

Centralized physical constants to ensure consistency across the codebase.

```java
import static prog.util.PhysicsConstants.*;

// Gravitational acceleration
double energy = velocity * velocity / (2 * g);  // g = 9.80 m/s²

// Atmospheric constants (ISA model)
double p = Math.pow(1 - PRESSURE_ALTITUDE_COEFF * alt, PRESSURE_ALTITUDE_EXP);
```

**Available Constants:**

| Constant | Value | Description |
|----------|-------|-------------|
| `G` / `g` | 9.80 | Gravitational acceleration (m/s²) |
| `PRESSURE_ALTITUDE_COEFF` | 0.0000225577 | Barometric formula coefficient (1/m) |
| `PRESSURE_ALTITUDE_EXP` | 5.25588 | Barometric formula exponent |
| `TEMP_LAPSE_RATE` | 0.0065 | Temperature lapse rate (K/m) |
| `SEA_LEVEL_PRESSURE` | 101325.0 | Sea level pressure (Pa) |
| `SEA_LEVEL_DENSITY` | 1.225 | Sea level air density (kg/m³) |
| `R_SPECIFIC_AIR` | 287.05 | Specific gas constant for air (J/(kg·K)) |
| `KELVIN_OFFSET` | 273.15 | Celsius to Kelvin offset |

---

### AtmosphereModel

International Standard Atmosphere (ISA) calculations for flight simulation.

```java
import static prog.util.AtmosphereModel.*;

// Relative pressure at altitude (sea level = 1.0)
double p = pressure(5000);  // → 0.533

// Altitude from pressure (inverse function)
double alt = altitudeAtPressure(0.5);  // → ~5574m

// Air density
double rho = density(p, 15.0, 5000);  // 15°C sea level temp

// Airspeed conversions
double tas = iasToTas(400, rho);  // IAS 400 km/h → TAS
double ias = tasToIas(500, rho);  // TAS 500 km/h → IAS

// RAM effect: equivalent altitude accounting for dynamic pressure
double effectiveAlt = ramEffectAltitude(
    5000,    // actual altitude (m)
    15.0,    // sea level temp (°C)
    500,     // speed (km/h)
    true,    // is IAS
    0.9      // SpeedManifoldMultiplier from FM
);
```

**Physical Background:**

The ISA barometric formula: `P/P₀ = (1 - 0.0000225577 × h)^5.25588`

RAM Effect: At high speeds, the intake captures dynamic pressure (`q = ½ρv²`), increasing total pressure available to the supercharger. This is equivalent to flying at a lower altitude.

---

### PistonPowerModel

Calculates piston engine power at any altitude/speed combination.

> ⚠️ **Note:** This is a **calculation engine** only. It requires `CompressorStageParams` to be populated from FM data externally.

#### Basic Usage

```java
import static prog.util.PistonPowerModel.*;

// Create compressor stage parameters (must be extracted from FM file)
CompressorStageParams stage = new CompressorStageParams();
stage.critAlt = 7000;       // Critical altitude (m)
stage.critPower = 2000;     // Power at critical altitude (hp)
stage.deckPower = 1800;     // Sea level power (hp)
stage.wepCritAlt = 6000;    // WEP critical altitude (m)
stage.wepPowerMult = 1.15;  // WEP power multiplier
stage.speedManifoldMult = 0.9;  // RAM effect coefficient

CompressorStageParams[] stages = {stage};

// Calculate power at specific altitude/speed
double power = optimalPowerAdvanced(
    stages,
    5000,    // altitude (m)
    true,    // WEP mode
    400,     // speed (km/h)
    true,    // is IAS
    15.0     // sea level temp (°C)
);

// Generate full power curve
double[] curve = generatePowerCurveAdvanced(stages, true, 0, false, 15, 50);
// Returns power from 0m to 10000m in 50m steps (201 points)
```

#### Multi-Stage Superchargers

For engines with multiple supercharger speeds:

```java
CompressorStageParams stage1 = new CompressorStageParams(3000, 1400, 1350);  // Low blower
CompressorStageParams stage2 = new CompressorStageParams(6500, 1300, 1100);  // High blower
stage1.stageIndex = 0;
stage2.stageIndex = 1;

CompressorStageParams[] stages = {stage1, stage2};

// optimalPowerAdvanced automatically selects the best stage
double power = optimalPowerAdvanced(stages, 5000, false, 0, false, 15);
```

#### WEP Calculations

```java
// Calculate WEP power multiplier
double wepMult = wepPowerMultiplier(
    1.15,    // AfterburnerBoost
    1.0,     // ThrottleBoost
    1.0,     // AfterburnerBoostMul
    1.0,     // OctaneAfterburnerMult
    2400,    // Military RPM
    2600     // WEP RPM
);

// Calculate WEP critical altitude
double wepCritAlt = wepCriticalAltitude(
    7000,    // Military critical altitude
    1.42,    // Military manifold pressure (ata)
    1.65,    // WEP manifold pressure (ata)
    1.05,    // Supercharger RPM effect
    1.0      // AfterburnerPressureBoost
);
```

#### Power Curve Shape

```
Power (hp)
    ↑
    │    ┌─────────┐ ← Critical altitude (full boost maintained)
    │   /           \
    │  /             \ ← Above critical: power drops with pressure
    │ /               \
    │/                 \
    └──────────────────────→ Altitude (m)
   Deck    Crit Alt    Ceiling
```

---

### Interpolation

General-purpose interpolation utilities.

```java
import static prog.util.Interpolation.*;

// Linear interpolation between two points
double y = lerp(x, x0, y0, x1, y1);

// Slope calculation
double slope = slope(x0, y0, x1, y1);

// 1D table lookup with clamping
double[] xs = {0, 1000, 2000, 3000};
double[] ys = {100, 95, 85, 70};
double result = interp1d(1500, xs, ys);  // → 90

// 2D bilinear interpolation (for thrust tables, etc.)
double thrust = interp2d(altitude, mach, altitudes, machs, thrustTable);

// Zero-allocation sweep interpolation (for variable-geometry wings)
double vne = interpSweepLevel(vwing, sweepLevels,
    level -> level.vne,
    level -> level.sweep,
    defaultVne);
```

---

## Integration Architecture

### Full Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│               FM Parameter Extraction Layer                  │
│                                                              │
│  FM File (.blk) ──→ Blkx.java (parser)                     │
│  Central File ──→ Blkx.extractFuelModifications()           │
│                         ↓                                    │
│  FMPowerExtractor.extractStages(blkx, fuelMod)              │
│    • soviet_octane_adder (1.8% boost if B-95/B-100)         │
│    • definition_alt_power_adjuster (RPM corrections)        │
│    • deck_power_maker (sea level power per stage)           │
│    • wep_mulitiplierer + british_octane (WEP parameters)    │
│                         ↓                                    │
│  CompressorStageParams[] ──→ PowerCurveHelper (shape logic) │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                    Calculation Layer                          │
│  AtmosphereModel ←──── PistonPowerModel                     │
│    • pressure()         • powerAtAltitudeAdvanced()         │
│    • density()          • optimalPowerAdvanced()            │
│    • ramEffectAltitude()• generatePowerCurveAdvanced()      │
└─────────────────────────────────────────────────────────────┘
```

### Key API Methods

| Method | Description |
|--------|-------------|
| `powerAtAltitudeAdvanced()` | Single-stage power at altitude (WAPC variabler) |
| `optimalPowerAdvanced()` | Multi-stage optimal power selection |
| `generatePowerCurveAdvanced()` | Full power curve generation (0–10000m) |
| `findOptimalStageIndex()` | Optimal supercharger stage index for given altitude |

### Required FM Parameters

To use `PistonPowerModel` for any aircraft, these parameters must be extracted from FM files:

| FM Parameter | Maps To | Description |
|--------------|---------|-------------|
| `Power` | `deckPower` | Sea level power (hp) |
| `Altitude0/1/2` | `critAlt` | Critical altitude per stage (m) |
| `Power0/1/2` | `critPower` | Power at critical altitude (hp) |
| `Ceiling0/1/2` | `ceilingAlt` | Ceiling altitude per stage (m) |
| `PowerAtCeiling0/1/2` | `ceilingPower` | Power at ceiling altitude (hp) |
| `AltitudeConstRPM0/1/2` | `constRpmAlt` | ConstRPM bend altitude (0 is valid!) |
| `PowerConstRPM0/1/2` | `constRpmPower` | Power at ConstRPM bend |
| `AfterburnerBoost` | WEP calculation | WEP boost factor |
| `AfterburnerManifoldPressure` | WEP calculation | WEP manifold pressure (ata) |
| `SpeedManifoldMultiplier` | `speedManifoldMult` | RAM effect coefficient |
| `CompressorPressureAtRPM0` | `superchargerRpmEffect()` | Supercharger pressure at RPM=0 |
| `CompressorOmegaFactorSq` | `superchargerRpmEffect()` | Supercharger omega factor |

---

## Design Principles

1. **Pure Functions**: All methods are stateless with no side effects
2. **Thread Safety**: Safe to call from any thread without synchronization
3. **Zero Allocation**: Avoid object creation in hot paths (use primitive arrays)
4. **Static Import Friendly**: Use `import static` for cleaner code
5. **Fail Safe**: Invalid inputs return sensible defaults rather than throwing

## Adding New Utilities

When adding physics/math utilities:

1. Add constants to `PhysicsConstants.java` if reusable
2. Create pure static methods (no instance state)
3. Document units clearly in Javadoc
4. Provide usage examples in this file
5. Consider boundary conditions and NaN prevention
