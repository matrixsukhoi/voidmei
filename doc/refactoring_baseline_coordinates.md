# MinimalHUD Refactoring Baseline Coordinates
Recorded: 2026-01-18

This file contains the baseline coordinates for MinimalHUD components as recorded from `Application.debugPrint` logs during a preview session. Use these values to verify component positions after refactoring.

## Baseline Values

| Component | X | Y | Note |
| :--- | :--- | :--- | :--- |
| **FlapAngleBar Text** | 104 | 33 | |
| **FlapAngleBar Bar** | 42 | 38 | |
| **FlapAngleBar Tick[20]** | 74 | 38 | |
| **FlapAngleBar Tick[33]** | 95 | 38 | |
| **FlapAngleBar Tick[60]** | 138 | 38 | |
| **FlapAngleBar Tick[100]** | 203 | 38 | |
| **ThrottleBar** | 7 | 182 | |
| **AttitudeIndicator** | 112 | 129 | |
| **TextLine[0]** | 42 | 70 | |
| **TextLine[1]** | 42 | 98 | |
| **TextLine[2]** | 42 | 126 | |
| **TextLine[3]** | 42 | 154 | |
| **TextLine[4]** | 42 | 182 | |
| **ManeuverIndex (Line 4)** | 42 | 182 | Overlaps with TextLine[4] base |
| **Compass** | 148 | 130 | |
| **Crosshair (Texture)** | 272 | 20 | |

## Raw Log Output
```
[12:06:33.829] [Legacy    ] Component: FlapAngleBar Text, x=104, y=33
[12:06:33.830] [Legacy    ] Component: FlapAngleBar Bar, x=42, y=38
[12:06:33.830] [Legacy    ] Component: FlapAngleBar Tick[20], x=74, y=38
[12:06:33.831] [Legacy    ] Component: FlapAngleBar Tick[33], x=95, y=38
[12:06:33.831] [Legacy    ] Component: FlapAngleBar Tick[60], x=138, y=38
[12:06:33.831] [Legacy    ] Component: FlapAngleBar Tick[100], x=203, y=38
[12:06:33.832] [Legacy    ] Component: ThrottleBar, x=7, y=182
[12:06:33.832] [Legacy    ] Component: AttitudeIndicator, x=112, y=129
[12:06:33.834] [Legacy    ] Component: TextLine[0], x=42, y=70
[12:06:33.835] [Legacy    ] Component: TextLine[1], x=42, y=98
[12:06:33.835] [Legacy    ] Component: TextLine[2], x=42, y=126
[12:06:33.836] [Legacy    ] Component: TextLine[3], x=42, y=154
[12:06:33.836] [Legacy    ] Component: TextLine[4], x=42, y=182
[12:06:33.836] [Legacy    ] Component: ManeuverIndex (Line 4), x=42, y=182
[12:06:33.837] [Legacy    ] Component: Compass, x=148, y=130
[12:06:33.838] [Legacy    ] Component: Crosshair (Texture), x=272, y=20
```
