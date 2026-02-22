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
| `ColorHelper.java` | Color parsing/formatting: hex (#RRGGBB) ↔ decimal (R,G,B,A) conversion |
| `GPUCompatibilityHelper.java` | GPU compatibility mode: save/load settings, check rendering mode |
| `DPIHelper.java` | High-DPI display support: scale detection, logical screen dimensions |
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

#### Peak WEP Power

Find the maximum WEP power across all altitude × speed combinations (useful for display/comparison):

```java
// Get peak WEP power (traverses altitude × speed grid)
double peakPower = peakWepPower(stages);
// Returns the highest power value accounting for RAM effect
```

**Algorithm:** Traverses a 2D grid of altitude × speed combinations:
- Altitude: 0–10000m, step 100m (101 points)
- Speed: 0–800 km/h IAS, step 50 km/h (17 points)
- Total: 1717 evaluations of `optimalPowerAdvanced(stages, alt, true, speed, true, 15.0)`

This accounts for RAM effect (`SpeedManifoldMultiplier`), where high-speed flight increases effective manifold pressure.

**Use Cases:**
- Displaying maximum engine performance in UI
- Comparing aircraft engine power
- Validating FM power calculations

**Note:** For aircraft-specific validation, always use real FM data via `FMPowerExtractor.extractStages(blkx)`. Do not use hardcoded benchmark parameters (see `doc/功率曲线调试手册.md` §7 for details).

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

### ColorHelper

Color parsing and formatting utility supporting both hex and decimal formats.

```java
import static prog.util.ColorHelper.*;

// Parse any format (auto-detects hex vs decimal)
Color c1 = parseColor("#FF5500AA", Color.WHITE);       // Hex RGBA
Color c2 = parseColor("#FF5500", Color.WHITE);          // Hex RGB (alpha=255)
Color c3 = parseColor("255, 85, 0, 170", Color.WHITE);  // Decimal RGBA
Color c4 = parseColor("255, 85, 0", Color.WHITE);       // Decimal RGB (alpha=255)

// Format for display (hex - user-friendly)
String hex = toHexString(color, true);   // "#FF5500AA" (with alpha)
String hex = toHexString(color, false);  // "#FF5500" (no alpha)

// Format for storage (decimal - backward compatible with ui_layout.cfg)
String dec = toDecimalString(color);     // "255, 85, 0, 170"

// Detect format type
boolean isHex = isHexFormat("#FF5500");  // true
boolean isHex = isHexFormat("255,0,0");  // false
```

**Design Notes:**
- Config storage uses decimal format for backward compatibility
- UI display uses hex format for easier manual editing
- `parseColor()` accepts both formats, enabling copy-paste from any source
- Invalid inputs return the provided default color (never throws)

---

### GPUCompatibilityHelper

Runtime helper for GPU compatibility mode (software rendering).

```java
import prog.util.GPUCompatibilityHelper;

// Save setting to gpu_compat.properties (user toggled in UI)
GPUCompatibilityHelper.saveSettings(true);

// Read current saved setting (what will apply on next startup)
boolean enabled = GPUCompatibilityHelper.isEnabled();

// Check if software rendering is currently active in this JVM
boolean active = GPUCompatibilityHelper.isSoftwareRenderingActive();

// Get human-readable description of current rendering mode
String mode = GPUCompatibilityHelper.getRenderingModeDescription();
// → "Direct3D Hardware Acceleration" or "Software Rendering (GPU acceleration disabled)"
```

**Architecture Notes:**

This helper works in conjunction with `prog.Launcher`:

1. **Launcher.java** (bootstrap) - Reads `gpu_compat.properties` and sets JVM system properties (`sun.java2d.d3d=false`, etc.) **before** any AWT class loads
2. **GPUCompatibilityHelper** (runtime) - Provides save/load methods for the UI toggle, and status queries

**Why separate from ui_layout.cfg?**

The GPU compat setting must be read before AWT initialization. Since `ConfigLoader` uses AWT classes, we need a simpler properties file that can be read with pure `java.io` classes.

**Methods:**

| Method | Description |
|--------|-------------|
| `saveSettings(boolean)` | Save to `gpu_compat.properties` |
| `isEnabled()` | Read saved setting from file |
| `isSoftwareRenderingActive()` | Check current JVM rendering mode |
| `getRenderingModeDescription()` | Human-readable mode description |

---

### DPIHelper

High-DPI display support for proper scaling on monitors with > 100% scaling (e.g., Windows 200% scaling).

```java
import prog.util.DPIHelper;
import prog.Application;

// Initialization (called once in Application.getScreenSize())
DPIHelper.init();

// Access global values (preferred - faster, no method call)
double scale = Application.dpiScale;        // 1.0 at 100%, 2.0 at 200%
int screenW = Application.logicalWidth;     // Logical screen width
int screenH = Application.logicalHeight;    // Logical screen height

// Direct API methods
double scaleX = DPIHelper.getScaleX();                    // Horizontal scale factor
double scaleY = DPIHelper.getScaleY();                    // Vertical scale factor
int logicalW = DPIHelper.getLogicalScreenWidth();         // Logical width
int logicalH = DPIHelper.getLogicalScreenHeight();        // Logical height
int physicalW = DPIHelper.getPhysicalScreenWidth();       // Physical (monitor) width
int physicalH = DPIHelper.getPhysicalScreenHeight();      // Physical (monitor) height

// Scaling utilities
int scaled = DPIHelper.scale(24);                         // 24 → 48 at 200% DPI
double scaledD = DPIHelper.scale(24.0);                   // Double version
int unscaled = DPIHelper.unscale(48);                     // 48 → 24 at 200% DPI
boolean hiDpi = DPIHelper.isHighDPI();                    // True if scale > 1.0
```

**Physical vs Logical Pixels:**

| 200% Scaling | Physical Pixels | Logical Pixels |
|--------------|-----------------|----------------|
| Screen Size  | 3840 × 2160     | 1920 × 1080    |
| Font 24pt    | 48 actual pixels| 24 logical     |

On high-DPI displays:
- `Toolkit.getScreenSize()` returns physical pixels (3840 × 2160)
- Swing operates in logical pixels (1920 × 1080)
- `DPIHelper` bridges this gap by detecting the actual scale factor

**Usage Patterns:**

```java
// In overlay reinitConfig() - scale font sizes
double dpiScale = Application.dpiScale;
fontSize = (int) Math.round((24 + fontadd) * dpiScale);

// In MainForm - use logical dimensions for positioning
int maxWidth = Math.min(800, Application.logicalWidth - 40);
setLocation(Application.logicalWidth / 2 - width / 2,
            Application.logicalHeight / 2 - height / 2);

// In MinimalHUDContext - scale HUD metrics
b.crossScale = (int) Math.round(baseCrossScale * dpiScale);
b.hudFontSize = (b.crossScale / 4) + (int) Math.round(fAdd * dpiScale);
```

**JVM Integration:**

The `-Dsun.java2d.uiScale=1` flag in `script/voidmeil4j.xml` disables Java's automatic bitmap scaling, ensuring:
- Crisp font rendering at native resolution
- Our code has full control over DPI handling
- Consistent behavior across JVM versions

**Design Notes:**
- Thread-safe via `synchronized` init
- Idempotent - safe to call `init()` multiple times
- Falls back to 1.0 scale if detection fails
- Uses `GraphicsConfiguration.getDefaultTransform()` for reliable DPI detection

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
| `peakWepPower()` | Find maximum WEP power across altitude × speed grid (with RAM effect) |

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
