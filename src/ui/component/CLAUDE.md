# HUD Component Development Guide

## HUDComponent Interface

All MiniHUD visual elements implement this interface:

```java
public interface HUDComponent {
    String getId();                     // Unique identifier for layout/config
    Dimension getPreferredSize();       // Component dimensions
    void draw(Graphics2D g2d, int x, int y);  // Rendering entry point
    void onDataUpdate(HUDData data);    // Data refresh callback

    // Optional visibility control
    boolean isVisible();
    void setVisible(boolean visible);
}
```

## Existing Components Reference

| Component | Purpose | Data Source |
|-----------|---------|-------------|
| `LinearGauge` | Generic vertical/horizontal bar gauge | Generic value 0-1 |
| `SpeedRatioBar` | Speed vs. limit ratio with warning zones | TAS / VNE |
| `ThrottleBar` | Throttle position indicator (0-110%) | Throttle % |
| `FlapAngleBar` | Flap deflection angle display | Flap angle deg |
| `CrosshairGauge` | Aiming reticle (texture or software) | None (static) |
| `CompassGauge` | Heading indicator with cardinal marks | Heading deg |
| `AttitudeIndicatorGauge` | Artificial horizon | Pitch, Roll |
| `TextGauge` | Numeric readout with label | Arbitrary text |
| `LabeledLinearGauge` | LinearGauge with attached label | Value + label |

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

### Template Width for Stability

To prevent layout jitter when values change (e.g., 99 → 100):

```java
// BAD: Width changes with content
row.setText("ALT " + currentAlt);

// GOOD: Width locked to template
row.setTemplate("ALT 88888");  // Maximum expected width
row.setText("ALT " + currentAlt);
```

## Performance Requirements

### Zero-Allocation Rendering

The `draw()` method runs at 60+ FPS. **Never allocate objects inside draw():**

```java
// BAD: Creates garbage every frame
public void draw(Graphics2D g, int x, int y) {
    Color c = new Color(255, 0, 0);  // GC pressure!
    Font f = new Font("Arial", Font.BOLD, 14);  // GC pressure!
    g.setColor(c);
}

// GOOD: Reuse cached objects
private static final Color WARNING_COLOR = new Color(255, 0, 0);
private Font cachedFont;  // Set once in constructor/init

public void draw(Graphics2D g, int x, int y) {
    g.setColor(WARNING_COLOR);
    g.setFont(cachedFont);
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

    // Cached rendering resources
    private static final Color FILL_COLOR = new Color(0, 200, 100);

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
        int width = (int) (ctx.hudFontSize * 4);
        int height = (int) (ctx.hudFontSize * 1.5);
        return new Dimension(width, height);
    }

    @Override
    public void draw(Graphics2D g, int x, int y) {
        if (!isVisible()) return;

        Dimension size = getPreferredSize();
        int fillWidth = (int) (size.width * currentValue);

        g.setColor(FILL_COLOR);
        g.fillRect(x, y, fillWidth, size.height);
    }

    @Override
    public void onDataUpdate(HUDData data) {
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

**Anchor Alignment Formula:**
```
Self.Point(SelfAnchor) = Parent.Point(ParentAnchor) + Offset(Units * LineHeight)
```

**Example:** Compass right-aligned under Row 2:
```java
compassNode.setParent(row2)
    .setRelativePosition(0, 0.1)  // 0.1 units down
    .setAnchors(Anchor.BOTTOM_RIGHT, Anchor.TOP_RIGHT);
    //          ↑ Parent anchor      ↑ Self anchor
```

## Graphics Utilities

Use `GraphicsUtil` for precise rendering:

```java
// Precise line endpoints (no stroke overhang)
Stroke precise = GraphicsUtil.createPreciseStroke(2);
g.setStroke(precise);
g.drawLine(x, y, x + width, y);  // Exactly width pixels

// Rounded decorative lines
Stroke rounded = GraphicsUtil.createRoundedStroke(3);
```

## Common Pitfalls

- **Memory Leaks**: Creating `Color`/`Font` objects in `draw()` loop
- **Layout Jitter**: Not using template widths for variable-length text
- **Null Context**: `ctx` may be null during early initialization - check before use
- **Cycle Dependencies**: A → B → A layout relationships crash the engine
- **EDT Violations**: Modifying component state from non-EDT threads
