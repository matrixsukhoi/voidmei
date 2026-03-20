# FM Comparison Module Development Guide

## Overview

The `ui.window.comparison` package provides aircraft flight model (FM) comparison functionality. It displays side-by-side comparisons of FM properties with visual indicators showing which aircraft is better for each metric.

## File Overview

| File | Responsibility |
|------|----------------|
| `CompactComparisonWindow.java` | Main comparison dialog UI |
| `ComparisonFrame.java` | Full comparison frame |
| `ComparisonTable.java` | Table-based comparison view |
| `logic/ComparisonRule.java` | Rule interface for value extraction |
| `logic/ComparisonRules.java` | **Registry of all comparison rules** |
| `logic/rules/*.java` | Built-in rule implementations |

---

## Comparison Rule System

The comparison system uses a **manual rule-based approach** where each property must have an explicitly defined rule to show win/lose highlighting. Properties without rules display as a draw (grey color).

### Architecture

```
CompactComparisonWindow.addComparisonRow()
    ↓
ComparisonRules.get(propertyName)
    ↓
ComparisonRule.extractValue(rawValue)  → Double
ComparisonRule.isLowerBetter()         → boolean
    ↓
Determine winner: win = -1 (left), 0 (draw), 1 (right)
```

### Core Interface

```java
public interface ComparisonRule {
    Double extractValue(String rawValue);  // Extract comparable value
    boolean isLowerBetter();               // true=lower wins, false=higher wins
}
```

---

## Built-in Rule Types

### SimpleRule

Extracts the first number from a value string. Skips array values.

```java
// Lower is better (weight, drag)
rules.put("空重(kg)", SimpleRule.lowerIsBetter());

// Higher is better (speed, thrust)
rules.put("最大燃油重量(kg)", SimpleRule.higherIsBetter());
```

**Handles:** `"4644.0"` → `4644.0`

### ListIndexRule

Extracts a value at a specific index from a bracketed list.

```java
// Format: [min, max] - extract index 1 (max), higher is better
rules.put("临界速度(km/h)", new ListIndexRule(1, false));
```

**Handles:** `"[144, 1167]"` with index=1 → `1167.0`

### MultiListIndexRule

Extracts from nested lists at specific list and item indices.

```java
// Format: [a, b], [c, d] - extract list 0, item 1
rules.put("允许过载(满/半油)", new MultiListIndexRule(0, 1, false));
```

**Handles:** `"[8.5, -4.2], [10.1, -5.3]"` with listIndex=0, itemIndex=1 → `-4.2`

### LambdaRule

Custom extraction logic via lambda function.

```java
// Extract second number after "/" from "X / Y" format
private static final Pattern SLASH_SECOND = Pattern.compile("/\\s*(-?\\d+(\\.\\d+)?)");

rules.put("主阻力面积因数及加速度系数", new LambdaRule(
    raw -> {
        Matcher m = SLASH_SECOND.matcher(raw);
        return m.find() ? Double.parseDouble(m.group(1)) : null;
    },
    true // lower is better
));
```

**Handles:** `"0.25 / 0.35"` → `0.35`

---

## Adding New Rules

Edit `ComparisonRules.java` static initializer:

```java
static {
    // Simple numeric - lower is better
    rules.put("新属性名", SimpleRule.lowerIsBetter());

    // List extraction - index 2, higher is better
    rules.put("列表属性", new ListIndexRule(2, false));

    // Custom extraction
    rules.put("复杂属性", new LambdaRule(
        raw -> { /* extraction logic */ },
        false // higher is better
    ));
}
```

### Property Name Matching

Property names must **exactly match** the parsed property key from FM data. Check `lang/cur.properties` for the format strings used in `Blkx.java`:

| Lang Key | Format String | Property Name |
|----------|---------------|---------------|
| `bWeight` | `空重(kg): %.1f\n最大燃油重量(kg): %.1f\n` | `空重(kg)`, `最大燃油重量(kg)` |
| `bCritSpeed` | `临界速度(km/h): [%.0f, %.0f]\n` | `临界速度(km/h)` |
| `bDrag` | `主阻力面积因数及加速度系数: %.2f / %.2f\n...` | `主阻力面积因数及加速度系数` |

---

## Current Rules Reference

| Property | Rule Type | Extraction | Better |
|----------|-----------|------------|--------|
| 空重(kg) | SimpleRule | First number | Lower |
| 最大燃油重量(kg) | SimpleRule | First number | Higher |
| 临界速度(km/h) | ListIndexRule(1) | Index 1 | Higher |
| 允许过载(满/半油) | MultiListIndexRule(0,1) | List 0, Item 1 | Higher |
| 平均耐热条恢复速率 | SimpleRule | First number | Higher |
| 千米最大升力过载 | SimpleRule | First number | Higher |
| 主升力面积因数载荷 | SimpleRule | First number | Higher |
| 翼展效率 | SimpleRule | First number | Higher |
| 主阻力面积因数及加速度系数 | LambdaRule | After "/" | Lower |
| 诱导阻力因数及加速度系数 | LambdaRule | After "/" | Lower |
| 散热/油冷器阻力系数 | LambdaRule | Sum of both | Lower |

---

## UI Behavior

| Scenario | Color | Symbol |
|----------|-------|--------|
| Left wins (v0 better) | Green (v0) / Red (v1) | `▶` |
| Right wins (v1 better) | Red (v0) / Green (v1) | `◀` |
| Draw (no rule or equal) | Grey (both) | `-` |

---

## Testing

1. Run the application: `java -jar VoidMei.jar`
2. Open FM comparison window
3. Verify:
   - Properties with rules show colored win/lose highlighting
   - Properties without rules show grey (draw)
   - Extraction is correct for list/slash formats
