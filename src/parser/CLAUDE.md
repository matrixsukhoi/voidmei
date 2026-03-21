# Parser Module Development Guide

## Overview

The `parser` package handles data ingestion from War Thunder's HTTP API and flight model (.blk) files. All parsers convert raw game data into structured Java objects for use by the UI and calculation layers.

## File Overview

| File | Responsibility |
|------|----------------|
| `Blkx.java` | Flight model (.blk) file parser - extracts engine, aerodynamic, and performance data |
| `State.java` | Real-time state JSON parser (from `/state` endpoint) |
| `Indicators.java` | Cockpit indicators JSON parser (from `/indicators` endpoint) |
| `FlightAnalyzer.java` | Derived metrics calculation from raw telemetry |
| `FlightLog.java` | Flight data logging and replay |
| `FlightModelParser.java` | High-level FM parsing orchestration |
| `HudMsg.java` | HUD message parsing |
| `MapInfo.java` | Map information parser |
| `MapObj.java` | Map object (vehicles, markers) parser |

---

## Blkx.java - Flight Model Parser

The main FM file parser, extracting engine performance data, aerodynamic coefficients, and structural limits from War Thunder's `.blk` files.

### Thrust Table API

For jet engines, thrust is stored in 2D tables indexed by altitude and velocity.

#### Data Fields

| Field | Type | Description |
|-------|------|-------------|
| `altThrNum` | `int` | Number of altitude points in thrust table |
| `velThrNum` | `int` | Number of velocity points in thrust table |
| `maxThr` | `double[][]` | Military thrust table [altitude][velocity] (kgf) |
| `maxThrAft` | `double[][]` | Afterburner thrust table [altitude][velocity] (kgf) |
| `peakThrMil` | `double` | Peak military thrust (kgf) |
| `peakThrAft` | `double` | Peak afterburner thrust (kgf) |

#### peakThrust API

```java
// Get peak thrust value
double peakMil = blkx.peakThrust(false);  // Military thrust
double peakAft = blkx.peakThrust(true);   // Afterburner thrust
```

**Algorithm:** Traverses the full altitude × velocity grid to find maximum thrust:

```java
private double calculatePeakThrust(double[][] table) {
    double peak = 0;
    for (int i = 0; i < altThrNum; i++) {
        for (int j = 0; j < velThrNum; j++) {
            if (table[i][j] > peak) {
                peak = table[i][j];
            }
        }
    }
    return peak;
}
```

**Search Grid:**
- Altitude: `altThrNum` points (from FM `ThrustMax` table)
- Velocity: `velThrNum` points (from FM `ThrustMax` table)
- Total iterations: `altThrNum × velThrNum`

This correctly accounts for the fact that jet thrust varies with both altitude and airspeed, and peak thrust may occur at a specific altitude/speed combination rather than at sea level static conditions.

### Fuel Modification Support

The parser can extract fuel quality modifications from Central files:

```java
// Extract fuel modifications from Central file data
Blkx.FuelModification fuelMod = Blkx.extractFuelModifications(centralFileData);

// Available fuel types
FuelModification.FuelType.SOVIET_B95     // Soviet B-95 fuel
FuelModification.FuelType.SOVIET_B100    // Soviet B-100 fuel
FuelModification.FuelType.BRITISH_150_OCTANE  // British 150 octane
FuelModification.FuelType.BRITISH_100_SPITFIRE  // British 100 octane Spitfire
```

### Integration with PistonPowerModel

For piston engines, use `FMPowerExtractor` to convert `Blkx` data to `CompressorStageParams`:

```java
Blkx blkx = new Blkx(fmFilePath);
Blkx.FuelModification fuelMod = Blkx.extractFuelModifications(centralData);
CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx, fuelMod);

// Then use with PistonPowerModel
double power = PistonPowerModel.optimalPowerAdvanced(stages, altitude, isWep, speed, true, 15.0);
```

---

## State.java & Indicators.java

Parse real-time telemetry from War Thunder's local HTTP API (port 8111).

### Data Flow

```
War Thunder HTTP API
    ↓
State.java      → Aircraft position, orientation, speed, altitude
Indicators.java → Cockpit instruments, engine parameters, fuel
    ↓
FlightDataBus (event publisher)
    ↓
Overlay components
```

### Key Endpoints

| Endpoint | Parser | Data |
|----------|--------|------|
| `/state` | `State.java` | Position, velocity, orientation |
| `/indicators` | `Indicators.java` | Engine RPM, manifold pressure, temperatures |

---

## Design Principles

1. **Defensive Parsing**: Handle missing or malformed data gracefully
2. **Immutable Results**: Parsed objects should be treated as immutable
3. **Thread Safety**: Parsers may be called from background threads
4. **Fail Fast**: Invalid FM files should be detected early with clear error messages
