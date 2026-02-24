# UI Model Layer

## Overview

This package contains data models and interfaces for the UI layer, providing a clean separation between data sources (`Service`) and UI components (`Overlay`s).

## File Overview

| File | Purpose |
|------|---------|
| `TelemetrySource.java` | Zero-GC interface for high-frequency telemetry data |
| `FlightDataProvider.java` | Legacy data provider interface |
| `ServiceDataAdapter.java` | Adapter bridging Service to data consumers |
| `FieldDefinition.java` | Metadata for overlay fields |
| `FieldManager.java` | Interface for managing overlay fields |
| `DefaultFieldManager.java` | Default implementation of FieldManager |
| `GaugeField.java` | Model for gauge-style display fields |
| `DataField.java` | Model for text-style display fields |
| `FlightDataBindings.java` | Data binding utilities |
| `FlightInfoConfig.java` | Configuration for FlightInfoOverlay |
| `EngineInfoConfig.java` | Configuration for EngineInfoOverlay |

## TelemetrySource Interface

**Purpose**: Provides zero-allocation access to high-frequency telemetry data.

**Why it exists**: Traditional event-based data passing creates objects (Map, Event) every frame, causing GC pressure. `TelemetrySource` returns primitive `double` values directly, eliminating allocations.

### Method Categories

```java
public interface TelemetrySource {
    // Flight Data
    double getIAS();           // Indicated Airspeed (km/h or mph)
    double getTAS();           // True Airspeed
    double getMach();          // Mach number
    double getAoA();           // Angle of Attack (degrees)
    double getAoS();           // Angle of Sideslip
    double getNy();            // G-Force (load factor)
    double getVario();         // Climb Rate (m/s)

    // Altitude & Position
    double getAltitude();      // Barometric altitude (m)
    double getRadioAltitude(); // Radar altitude (m)
    boolean isRadioAltitudeValid();
    double getCompass();       // Heading (degrees)

    // Performance
    double getSEP();           // Specific Excess Power (m/s)
    double getAcceleration();  // Longitudinal acceleration
    double getTurnRate();      // Turn rate (deg/s)
    double getTurnRadius();    // Turn radius (m)
    double getRollRate();      // Roll rate (Wx)
    double getEnergyJKg();     // Specific energy (J/kg)

    // Engine - Power
    double getThrottle();      // Throttle position (0-110%)
    double getRPM();           // Engine RPM
    double getManifoldPressure();  // MAP (atm)
    boolean isManifoldPressureValid();
    double getThrust();        // Current thrust (kgf)
    double getHorsePower();    // Current HP
    double getEffHp();         // Effective HP
    double getPowerPercent();  // Power as % of max (capped at 100%)
    double getEngineResponse(); // Engine response rate

    // Engine - Temperature
    double getWaterTemp();     // Water/coolant temp
    double getOilTemp();       // Oil temperature
    double getHeatTolerance(); // Heat tolerance ratio

    // Engine - Control
    double getPitch();         // Propeller pitch (degrees)
    double getRPMThrottle();   // RPM throttle (pitch control %)
    double getUnknownMixture();// Mixture setting
    double getRadiator();      // Radiator opening (%)
    double getCompressorStage();// Supercharger stage

    // Fuel
    double getMassFuel();      // Remaining fuel (kg)
    double getFuelPercent();   // Fuel remaining (%)
    long getFuelTimeMili();    // Fuel time (ms)
    double getWepKg();         // WEP fuel consumed (kg)
    double getWepTime();       // WEP time remaining (s)

    // Propeller
    double getPropEfficiency();// Propeller efficiency

    // Control Surfaces (0.0-1.0 or %)
    double getGear();          // Landing gear position
    double getFlaps();         // Flap deflection
    double getAirbrake();      // Airbrake position
    double getAileron();       // Aileron deflection
    double getElevator();      // Elevator deflection
    double getRudder();        // Rudder deflection
    double getWingSweep();     // Variable sweep position
    boolean isWingSweepValid();

    // Speed Limits
    double getSpeedLimitRatio();    // Current speed / VNE
    double getAileronLockRatio();   // Speed ratio for aileron lock
    double getRudderLockRatio();    // Speed ratio for rudder lock
    double getUnitMachLimitRatio(); // Mach / critical Mach
    double getStallSpeed();         // Stall speed (km/h)

    // Units
    boolean isImperial();      // True if imperial units
    double getManifoldPressurePounds();  // MAP in psi
    double getManifoldPressureInchHg();  // MAP in inHg

    // Dynamic Display Methods (for unit-aware fields)
    double getManifoldPressureDisplay();      // Returns Ata (metric) or psi (imperial)
    String getManifoldPressureDisplayUnit();  // Returns "Ata" or "P/XX.X''" (live inHg)
    int getManifoldPressureDisplayPrecision(); // Returns 2 (metric) or 1 (imperial)

    // Attitude Indicator Data
    double getAviahorizonPitch();  // Pitch angle for artificial horizon (degrees)
    double getAviahorizonRoll();   // Roll angle for artificial horizon (degrees)

    // Engine Type & Aircraft Features (for :visible-when expressions)
    boolean isJetEngine();         // True if jet (turbojet/turbofan), requires ~5s detection
    boolean isPropEngine();        // True if propeller (piston/turboprop), requires ~5s detection
    boolean isEngineCheckDone();   // True after engine type detection completes
    boolean hasWep();              // True if aircraft has WEP/water injection system
}
```

### Implementation

`Service.java` implements `TelemetrySource`, returning internal fields directly:

```java
public class Service extends Thread implements TelemetrySource {
    private double mach, nVy, sep;  // Calculated fields

    @Override
    public double getMach() { return mach; }

    @Override
    public double getPowerPercent() {
        return Math.min(thurstPercent, 100.0);  // Capped at 100%
    }
}
```

### Usage Pattern

```java
public class MyOverlay implements FlightDataListener {
    private TelemetrySource source;

    public void init(Service service) {
        this.source = service;  // Service implements TelemetrySource
        FlightDataBus.getInstance().register(this);
    }

    @Override
    public void onFlightData(FlightDataEvent event) {
        // Use TelemetrySource for high-frequency values (zero GC)
        double ias = source.getIAS();
        double power = source.getPowerPercent();

        // Use EventPayload for low-frequency flags
        EventPayload payload = event.getPayload();
        boolean isJet = payload.isJet;
    }
}
```

## Dynamic Unit/Precision Support

For fields that need unit conversion based on aircraft type (e.g., manifold pressure showing Ata vs psi), the system supports **dynamic unit and precision suppliers**.

### DataField Dynamic Suppliers

```java
public class DataField {
    // ... existing fields ...

    // Dynamic suppliers for unit-aware fields
    public Supplier<String> unitSupplier;      // Called each frame to get current unit
    public IntSupplier precisionSupplier;      // Called each frame to get decimal places
}
```

### FieldManager Binding

Use the extended `bind()` method to attach dynamic suppliers:

```java
// Standard bind (static unit/precision)
fm.bind("getRPM", service::getRPM, null, 0, null);

// Dynamic bind (unit/precision from suppliers)
fm.bind("getManifoldPressureDisplay",
        service::getManifoldPressureDisplay,      // Value supplier
        service::isManifoldPressureValid,          // Visibility supplier
        2,                                         // Default precision
        null,                                      // Format
        service::getManifoldPressureDisplayUnit,   // Unit supplier
        service::getManifoldPressureDisplayPrecision); // Precision supplier
```

### How It Works

In `FieldOverlay.onFlightData()`, dynamic suppliers are called each frame:

```java
// Dynamic Precision
if (field.precisionSupplier != null) {
    field.precision = field.precisionSupplier.getAsInt();
}

// Dynamic Unit
if (field.unitSupplier != null) {
    String newUnit = field.unitSupplier.get();
    if (!newUnit.equals(field.unit)) {
        field.setUnit(newUnit);
    }
}
```

### Config-Driven Dynamic Binding

Fields can specify dynamic sources in `ui_layout.cfg`:

```lisp
(item "进气压" :type data
      :target "getManifoldPressureDisplay"
      :unit-source "getManifoldPressureDisplayUnit"
      :precision-source "getManifoldPressureDisplayPrecision"
      :unit "Ata"        ; Default for preview
      :precision 2       ; Default for preview
      :hide-when-zero true)
```

The `:unit-source` and `:precision-source` keywords specify TelemetrySource method names that return the dynamic unit string and precision integer respectively.

## GaugeField Model

Used by `EngineControlOverlay` for gauge-style displays:

```java
public class GaugeField {
    public String key;           // Config key
    public LinearGauge gauge;    // Visual component
    public MarkedGauge markedGauge; // Optional marked variant
    public int gaugeType;        // GaugeType ordinal
    public int maxValue;         // Gauge maximum
    public boolean isHorizontal; // Layout direction
    public boolean visible;      // Visibility state

    // Zero-GC buffer for number formatting
    public char[] buffer = new char[16];
    public int length;
}
```

## Best Practices

1. **Use TelemetrySource for numerics**: All high-frequency double values should be accessed via `TelemetrySource`, not event maps.

2. **Use EventPayload for flags**: Boolean flags and low-frequency state should come from `FlightDataEvent.getPayload()`.

3. **Never allocate in getters**: `TelemetrySource` implementations must return existing primitive values, never create objects.

4. **Cache references**: Store `TelemetrySource` reference once during init, reuse throughout lifecycle.
