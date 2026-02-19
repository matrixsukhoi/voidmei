# HUD Component Development Guide

## File Overview

| File | Lines | Purpose |
|------|-------|---------|
| `HUDComponent.java` | ~65 | Core interface for all HUD components |
| `AbstractHUDComponent.java` | ~35 | Base class with visibility management |
| `LinearGauge.java` | ~200 | Vertical/horizontal bar gauge with zero-GC optimization |
| `LabeledLinearGauge.java` | ~80 | LinearGauge with attached label |
| `SpeedRatioBar.java` | ~150 | Speed vs. limit ratio with warning zones |
| `FlapAngleBar.java` | ~100 | Flap deflection angle display |
| `CompassGauge.java` | ~240 | Heading indicator with inertial/body-fixed modes and North triangle |
| `CrosshairGauge.java` | ~120 | Aiming reticle (texture or software) |
| `AttitudeIndicatorGauge.java` | ~240 | Artificial horizon with inertial/body-fixed modes |
| `TextGauge.java` | ~100 | Numeric readout with label |
| `WarningOverlay.java` | ~80 | Warning message display |
| `row/` | 6 files | HUD row components for data display |

## HUDComponent Interface

All MiniHUD visual elements implement this interface:

```java
public interface HUDComponent {
    /**
     * Unique ID for the component (used for layout/config).
     */
    String getId();

    /**
     * Preferred size of the component.
     */
    Dimension getPreferredSize();

    /**
     * Draw the component at the specified position.
     * CRITICAL: Must be zero-allocation - no new objects in this method!
     */
    void draw(Graphics2D g2d, int x, int y);

    /**
     * Update component state from the centralized HUD Data snapshot.
     * Components should extract only the fields they care about.
     */
    default void onDataUpdate(HUDData data) {}

    /**
     * Check if component is visible.
     */
    default boolean isVisible() { return true; }

    /**
     * Set visibility.
     */
    default void setVisible(boolean visible) {}

    /**
     * Legacy generic update method.
     * @deprecated Use onDataUpdate(HUDData) instead.
     */
    @Deprecated
    default void update(Object data) {}
}
```

## AbstractHUDComponent Base Class

Provides common visibility management:

```java
public abstract class AbstractHUDComponent implements HUDComponent {
    protected boolean visible = true;

    @Override
    public boolean isVisible() { return visible; }

    @Override
    public void setVisible(boolean visible) { this.visible = visible; }

    @Override
    public abstract String getId();

    @Override
    public abstract Dimension getPreferredSize();

    @Override
    public abstract void draw(Graphics2D g2d, int x, int y);
}
```

## Existing Components Reference

| Component | Purpose | Data Source | Key Properties |
|-----------|---------|-------------|----------------|
| `LinearGauge` | Generic vertical/horizontal bar | Value 0-maxValue | `label`, `vertical`, `tickOnRight` |
| `LabeledLinearGauge` | LinearGauge with label | Value 0-maxValue | Inherits LinearGauge |
| `SpeedRatioBar` | Speed vs. limit ratio | TAS / VNE | Warning zones |
| `FlapAngleBar` | Flap deflection angle | Flap angle deg | Colored zones |
| `CrosshairGauge` | Aiming reticle | None (static) | Texture or software |
| `CompassGauge` | Heading indicator | Heading deg | `inertialMode` (离体/随体), North triangle |
| `AttitudeIndicatorGauge` | Artificial horizon + sideslip | Pitch, Roll, Slip, pitchValid | `inertialMode`, Dual-value display |
| `TextGauge` | Numeric readout | Arbitrary text | Template width |
| `WarningOverlay` | Warning messages | Warning state | Blink, colors |

## Row Components (`row/`)

| Component | Purpose |
|-----------|---------|
| `HUDRow` | Base row container |
| `HUDTextRow` | Simple text row |
| `HUDAkbRow` | AKB (Auto Kinetic Balance) data |
| `HUDEnergyRow` | Energy state display |
| `HUDFlapsRow` | Flaps position |
| `HUDManeuverRow` | Maneuver data (G-load, etc.) |

## Responsive Design Principles

### Unit-Based Sizing

**CRITICAL**: Never use hardcoded pixel values. Use `ctx.hudFontSize` as the base unit:

```java
// BAD: Hardcoded pixels (breaks on 4K/low-res screens)
g.fillRect(x, y, 200, 10);

// GOOD: Relative to font size
int width = (int) (ctx.hudFontSize * 8.5);  // ~8.5 character widths
int height = (int) (ctx.hudFontSize * 0.5); // Half line height
g.fillRect(x, y, width, height);
```

**Conversion Guide:**
- 1 Unit ≈ 1 line height ≈ `ctx.hudFontSize` pixels
- Use floating-point multipliers for fine-tuning: `ctx.hudFontSize * 0.618`
- Common ratios: `0.5` (half), `0.618` (golden), `0.75` (three-quarters)

### Template Width for Stability

To prevent layout jitter when values change (e.g., 99 → 100):

```java
// BAD: Width changes with content
row.setText("ALT " + currentAlt);

// GOOD: Width locked to template
row.setTemplate("ALT 88888");  // Maximum expected width
row.setText("ALT " + currentAlt);
```

### Responsive getPreferredSize()

```java
@Override
public Dimension getPreferredSize() {
    // Calculate based on font metrics, not hardcoded values
    int width = (int) (ctx.hudFontSize * 4.0);
    int height = (int) (ctx.hudFontSize * 1.5);
    return new Dimension(width, height);
}
```

## Performance Requirements

### Zero-Allocation Rendering

The `draw()` method runs at 60+ FPS. **Never allocate objects inside draw():**

```java
// BAD: Creates garbage every frame
public void draw(Graphics2D g, int x, int y) {
    Color c = new Color(255, 0, 0);  // GC pressure!
    Font f = new Font("Arial", Font.BOLD, 14);  // GC pressure!
    String s = String.format("%.1f", value);  // GC pressure!
    g.setColor(c);
}

// GOOD: Reuse cached objects
private static final Color WARNING_COLOR = new Color(255, 0, 0);
private Font cachedFont;  // Set once in constructor/init
private final char[] charBuffer = new char[32];  // Reusable buffer

public void draw(Graphics2D g, int x, int y) {
    g.setColor(WARNING_COLOR);
    g.setFont(cachedFont);
    // Use pre-formatted char array instead of String.format
    g.drawChars(charBuffer, 0, charLen, x, y);
}
```

### Zero-GC Number Formatting

LinearGauge demonstrates zero-allocation number display:

```java
public class LinearGauge extends AbstractHUDComponent {
    // Pre-allocated buffer for value display
    public final char[] valueBuffer = new char[32];
    public int valueLen = 0;

    public void update(int value, char[] buf, int len) {
        this.curValue = value;
        if (len > 32) len = 32;
        System.arraycopy(buf, 0, this.valueBuffer, 0, len);
        this.valueLen = len;
    }

    @Override
    public void draw(Graphics2D g, int x, int y) {
        if (valueLen > 0) {
            g.drawChars(valueBuffer, 0, valueLen, x, y);
        }
    }
}
```

### Stateless Drawing

`draw()` must be a **pure function** with no side effects:

```java
// BAD: Mutates state in draw
public void draw(Graphics2D g, int x, int y) {
    this.lastDrawnX = x;  // Side effect!
    this.counter++;       // Side effect!
}

// GOOD: Read-only
public void draw(Graphics2D g, int x, int y) {
    g.drawString(this.cachedText, x, y);  // Only reads state
}
```

### Dirty Checking

Only repaint when data actually changes:

```java
private double lastValue = Double.NaN;

@Override
public void onDataUpdate(HUDData data) {
    double newValue = data.speedRatio;
    if (Math.abs(newValue - lastValue) < 0.001) {
        return;  // Skip redundant update
    }
    lastValue = newValue;
    // ... update internal state
}
```

## Adding a New Component

### Step 1: Create Component Class

```java
public class MyGauge extends AbstractHUDComponent {

    private final String id;
    private double currentValue;
    private MinimalHUDContext ctx;

    // Cached rendering resources - CRITICAL for zero-GC
    private static final Color FILL_COLOR = new Color(0, 200, 100);
    private static final Color BG_COLOR = new Color(40, 40, 40);

    public MyGauge(String id, MinimalHUDContext ctx) {
        this.id = id;
        this.ctx = ctx;
    }

    @Override
    public String getId() {
        return id;
    }

    @Override
    public Dimension getPreferredSize() {
        // Use ctx.hudFontSize for responsive sizing
        int width = (int) (ctx.hudFontSize * 4);
        int height = (int) (ctx.hudFontSize * 1.5);
        return new Dimension(width, height);
    }

    @Override
    public void draw(Graphics2D g, int x, int y) {
        if (!isVisible()) return;

        Dimension size = getPreferredSize();
        int fillWidth = (int) (size.width * currentValue);

        // Background
        g.setColor(BG_COLOR);
        g.fillRect(x, y, size.width, size.height);

        // Fill
        g.setColor(FILL_COLOR);
        g.fillRect(x, y, fillWidth, size.height);
    }

    @Override
    public void onDataUpdate(HUDData data) {
        // Clamp value to 0-1 range
        this.currentValue = Math.max(0, Math.min(1, data.throttle / 100.0));
    }
}
```

### Step 2: Instantiate in MiniHUDOverlay

In `MiniHUDOverlay.initComponentsLayout()`:

```java
MyGauge myGauge = new MyGauge("myGauge", ctx);
components.add(myGauge);
```

### Step 3: Add to Layout Engine

```java
HUDLayoutNode myNode = new HUDLayoutNode("myGauge", myGauge);
myNode.setParent(row2)
      .setRelativePosition(0, 0.2)  // Offset in units
      .setAnchors(Anchor.BOTTOM_LEFT, Anchor.TOP_LEFT);
modernLayout.addNode(myNode);
```

### Step 4: Wire Data Updates

In `MiniHUDOverlay.updateComponents()`:

```java
if (myGauge != null) {
    myGauge.onDataUpdate(data);
}
```

## Layout Anchor System

Components are positioned relative to parents using anchor points:

```
TOP_LEFT ─────── TOP_CENTER ─────── TOP_RIGHT
    │                 │                  │
    │                 │                  │
CENTER_LEFT ──────── CENTER ────── CENTER_RIGHT
    │                 │                  │
    │                 │                  │
BOTTOM_LEFT ───── BOTTOM_CENTER ─── BOTTOM_RIGHT
```

### Anchor Alignment Formula

```
Self.Point(SelfAnchor) = Parent.Point(ParentAnchor) + Offset(Units * LineHeight)
```

### Example: Compass right-aligned under Row 2

```java
compassNode.setParent(row2)
    .setRelativePosition(0, 0.1)  // 0.1 units down
    .setAnchors(Anchor.BOTTOM_RIGHT, Anchor.TOP_RIGHT);
    //          ↑ Parent anchor      ↑ Self anchor
```

### Layout Resolution (DAG Topological Sort)

The layout engine resolves positions using topological sort:

```
1. Build dependency graph from parent references
2. Topological sort to find evaluation order
3. For each node in order:
   a. Get parent's computed position
   b. Calculate anchor point on parent
   c. Apply relative offset (in units * hudFontSize)
   d. Store computed position for child nodes
```

**Warning**: Circular dependencies (A → B → A) will crash the engine!

## Graphics Utilities

Use `GraphicsUtil` for precise rendering:

```java
// Precise line endpoints (no stroke overhang)
Stroke precise = GraphicsUtil.createPreciseStroke(2);
g.setStroke(precise);
g.drawLine(x, y, x + width, y);  // Exactly width pixels

// Rounded decorative lines
Stroke rounded = GraphicsUtil.createRoundedStroke(3);
g.setStroke(rounded);
g.drawLine(x, y, x + width, y);
```

## HUDData Model

Components receive data via `onDataUpdate(HUDData)`:

```java
public class HUDData {
    // Speed metrics
    public double TAS;           // True airspeed (km/h)
    public double IAS;           // Indicated airspeed
    public double mach;          // Mach number
    public double speedRatio;    // TAS / VNE

    // Altitude metrics
    public double altitude;      // Meters
    public double climbRate;     // m/s

    // Attitude
    public double pitch;         // Degrees
    public double roll;          // Degrees
    public double heading;       // Degrees (0-360)
    public double slip;          // Sideslip angle (degrees)
    public double AoA;           // Angle of attack
    public boolean pitchValid;   // True if pitch data available from API

    // Engine
    public double throttle;      // 0-100 (or 110 for WEP)
    public double rpm;           // Engine RPM
    public double manifold;      // Manifold pressure

    // Control surfaces
    public double flapAngle;     // Degrees
    public boolean gearDown;     // Landing gear state

    // Warnings
    public boolean overspeed;    // VNE exceeded
    public boolean stallWarning; // Near stall
}
```

## Common Pitfalls

| Issue | Symptom | Solution |
|-------|---------|----------|
| Memory Leaks | Heap growth, eventual OOM | Cache `Color`/`Font` as `static final` |
| Layout Jitter | UI elements jump around | Use template widths for text |
| Null Context | NullPointerException on init | Check `ctx != null` before using |
| Cycle Dependencies | Stack overflow in layout | Ensure DAG (no circular parent refs) |
| EDT Violations | Random corruption, freezes | Only modify state in `onDataUpdate` |
| Hardcoded Pixels | Broken on different DPI | Always use `ctx.hudFontSize` as unit |

## Testing Components

Since there are no automated tests, use the mock server:

```bash
# Start mock telemetry server
python3 script/mock_8111.py

# Run application in dev mode
java -jar VoidMei.jar

# Toggle preview mode in settings to see component live
```

Verify:
1. Component renders correctly at different crosshair scales
2. No flickering or jitter during data updates
3. Colors and fonts match design spec
4. No memory growth during extended use (check with VisualVM)
