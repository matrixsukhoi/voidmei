# VoidMei Manual Test Plan

This document outlines the manual testing procedures to verify the stability, functionality, and user experience of VoidMei, specifically targeting recent changes (`FMDataOverlay` refactor, Startup optimizations).

## 1. Startup & Initialization Test

**Objective**: Verify the "Zero-Flash" startup and smooth initialization.

| Step | Action | Expected Result |
|:----:|:-------|:----------------|
| 1 | Launch `VoidMei.exe` (or run debug script). | Application launches without crashing. |
| 2 | Observe the startup sequence. | Status bar shows "Loading...". |
| 3 | Wait for ~2 seconds. | Overlays should **fade in** or appear. |
| 4 | **Verify Flash**: Observe overlay content immediately upon appearance. | **CRITICAL**: Overlays must show valid data (or "-") immediately. **NO** brief flashes of empty transparent boxes or "null" text. |

## 2. Preview Mode Test (No Game Running)

**Objective**: Confirm overlays are configurable and visible in Preview Mode.

| Step | Action | Expected Result |
|:----:|:-------|:----------------|
| 1 | Ensure War Thunder is **NOT** running. | |
| 2 | Open the main VoidMei settings window. | Settings UI appears. |
| 3 | Navigate to the "Layout" or "Overlays" tab. | List of available overlays is shown. |
| 4 | Toggle switches for **Flight Info**, **Engine Info**, **FM Data**. | Corresponding overlays appear on screen. |
| 5 | **Verify Data**: Check `FM Data Overlay`. | It should display technical text (e.g., aerodynamic coefficients) for the P-51D (Sample Data). It should **NOT** be empty (Black box). |
| 6 | Drag overlays around. | Windows move smoothly. |
| 7 | Close and Restart VoidMei. | Overlays should reappear in the **new positions**. |

## 3. Game Integration Test

**Objective**: Verify real-time data integration and logic.

| Step | Action | Expected Result |
|:----:|:-------|:----------------|
| 1 | Start War Thunder and enter a "Test Flight". | |
| 2 | Start VoidMei. | Status bar indicates connection (Frequency/Port). |
| 3 | **Engine Info**: Throttle up. | `Thrust`, `RPM`, `Manifold` values should update in real-time. |
| 4 | **Flight Info**: Take off. | `Speed` and `Altitude` increase. |
| 5 | **FM Data Logic Check**: | |
| 5a | Sit on runway (Speed = 0). | **FM Data Overlay** is VISIBLE (if enabled). |
| 5b | Take off and exceed 200 km/h. | **FM Data Overlay** should automatically **HIDE**. |
| 5c | Land and come to a stop. | **FM Data Overlay** should **REAPPEAR**. |

## 4. Configuration & Layout Test

**Objective**: Verify external configuration file loading.

| Step | Action | Expected Result |
|:----:|:-------|:----------------|
| 1 | Close VoidMei. | |
| 2 | Open `ui_layout.cfg` in a text editor. | |
| 3 | Rename a label (e.g., change "摇杆面板" to "舵面值面板"). | File saves successfully. |
| 4 | Start VoidMei. | Settings window reflects the **NEW NAME** ("舵面值面板"). |
| 5 | Toggle the renamed switch. | The correct overlay (Stick/Axis) toggles. |

## 5. Hotkey Test

**Objective**: Verify Keyboard interaction (if Global Hook enabled).

| Step | Action | Expected Result |
|:----:|:-------|:----------------|
| 1 | In-game or Desktop, press `VC_P` (Default FM Data hotkey). | FM Data Overlay toggles visibility. |
| 2 | Press `Shift+O` (or configured Overlay toggle). | All overlays toggle visibility. |

## 6. Stability Stress Test

**Objective**: Ensure no memory leaks or crashes during transitions.

| Step | Action | Expected Result |
|:----:|:-------|:----------------|
| 1 | Rapidly toggle overlay switches on/off in Settings. | App remains responsive, no visual artifacts. |
| 2 | Switch between aircraft in game (creates new .blk events). | `FM Data Overlay` updates with **NEW** aircraft data automatically. |
