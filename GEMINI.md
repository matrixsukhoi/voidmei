# VoidMei Project Context

This document provides a comprehensive overview of the **VoidMei** project architecture, key features, and development standards. It is intended as a reference for AI assistants and developers.

## 1. Project Overview
VoidMei is a Java-based overlay and telemetry utility for flight simulation (War Thunder). It uses a custom Swing-based UI framework (WebLaF) to render real-time HUDs and configuration panels.

## 2. Architecture

### Core Packages (`src/`)
-   **`prog`**: Application kernel.
    -   `Service.java`: Singleton managing background threads and global state.
    -   `Controller.java`: Central coordinator and lifecycle manager.
    -   `OverlayManager.java`: Manages overlay windows and visibility.
    -   `event/`: Event Bus implementation (`UIStateBus`, `FlightDataBus`).
    -   `config/`: Configuration subsystems (`ConfigurationService`, `HUDSettings`).
    -   `audio/`: Voice warning logic (`VoiceResourceManager`, `VoiceWarning`).
    -   `util/`: Utilities, including `UIStateStorage` for persistence.
    -   `i18n/`: Localization (`Lang`).
-   **`ui`**: User Interface.
    -   `MainForm.java`: Entry point and configuration window.
    -   `overlay/`: Real-time HUD implementations (e.g., `MiniHUDOverlay`, `EngineControlOverlay`).
        -   `logic/`: Pure logic calculators (e.g., `HUDCalculator`).
        -   `model/`: Data models for overlays (`HUDData`).
    -   `component/`: Reusable widgets (`LinearGauge`, `CompassGauge`).
    -   `layout/`: Logic for parsing `ui_layout.cfg` and generating settings pages.
    -   `layout/renderer/`: Renderers for specific config types.
    -   `replica/`: UI Component factories (`ReplicaBuilder`).
    -   `model/`: Data binding and telemetry abstraction (`TelemetrySource`, `FlightDataProvider`).
    -   `base/`: Base classes (`DraggableOverlay`, `FieldOverlay`).
-   **`parser`**: Data Ingestion.
    -   Handles parsing of JSON/Telemetry from the game (`State`, `Indicators`, `Blkx`).

### Data Flow
1.  **Ingestion**: `parser` reads local host telemetry (JSON).
2.  **Processing**: `prog.Service` updates state and `prog.event.FlightDataBus` emits events.
3.  **Binding**: `ui.model.FlightDataProvider` adapts raw events into usable data bindings.
4.  **Rendering**: Overlays (`ui.overlay`) consume events/models and repaint. Settings (`ui.layout`) modify config which alters logic.

## 3. Key Features

### 3.1 Voice Warning System
A highly configurable voice alert system.
*   **Configuration**: Defined in `ui_layout.cfg` (Panel: "语音告警"). keys usually prefixed with `voice_`.
*   **Resources**: Audio files stored in `./voice/{packName}/`.
*   **Components**:
    *   `VoiceRowRenderer`: Renders individual voice items.
    *   `VoiceGlobalRenderer`: Renders the "Global Voice Pack" switch.
    *   `VoiceResourceManager`: Manages file loading and availability checks.
    *   `VoiceWarning`: Background thread (10Hz) executing alert logic.
*   **Smart Logic**:
    *   **Global Switch**: Switching the global pack *only* updates individual items if the target pack contains the specific file.
    *   **Dropdowns**: Individual specific voice dropdowns *only* list packs that contain that specific file (plus "default").
    *   **Customization**: Specific warnings (e.g., "Elevator Efficiency") can be disabled directly in `VoiceWarning.java` via comments if the algorithm is deemed incorrect.

### 3.2 MiniHUD
A performance-critical overlay for flight data.
*   **Optimization**: Uses **Dirty Checking** in UI rows (`HUDEnergyRow`, `HUDAkbRow`) to minimize string formatting cost.
*   **Rendering**: Decoupled from logic thread. `HUDCalculator` prepares raw data; Components handle formatting locally in `onDataUpdate`.
*   **Visuals**:
    *   **Intelligent Flap Indicator**: Toggleable via config (`enableFlapAngleBar`).
        *   **ON (Default)**: Shows visual Flap Bar. Text row hides flap percentage, showing only Gear/Brake status.
        *   **OFF**: Hides Flap Bar. Text row displays flap reading (e.g., "F 20").
    *   **Sentinel Handling**: Logic correctly interprets `65535` (or `-65535`) flap values from the game as `0` to prevent "F65535" display.
    *   **Dynamic Sizing**: `ModernHUDLayoutEngine` uses `LAYOUT_PADDING = 45` (increased from 25) to provide adequate breathing room around the HUD.
*   **Data Model**:
    *   Refactored `HUDData` nomenclature for clarity:
        *   `mechanizationStr` (was `flapsStr`): Represents Flaps + Gear + Airbrake configuration.
        *   `maneuverStateStr` (was `maneuverRowStr`): Represents dynamic state (G-Load vs Mission Time).

### 3.3 Engine Info Display (Smart Filtering)
*   **Logic**: `Service.java` forces specific metrics to `0.0` based on `iEngType` (Jet vs Prop) to support `hide-when-zero` logic.
    *   **Jets**: `ManifoldPressure`, `WaterTemp` -> forced to 0. (Oil Temp is *not* forced to 0).
    *   **Props**: `Thrust` -> 0 (naturally).
*   **Config**: `ui_layout.cfg` uses `:hide-when-zero true` to hide these irrelevant fields dynamically.

### 3.4 Dynamic UI Layout
*   **Source**: `ui_layout.cfg` (Custom Lisp-like syntax).
*   **Renderers**: `ui/layout/renderer/` contains classes mapped to config types.
*   **Rich Descriptions**: `:desc-image` supports high-definition images. `ReplicaBuilder` renders these at native resolution (no hardcoded scaling).
*   **Overlay Visibility**: Controlled by standard `(item ... :type switch :target "overlayConfigKey" ...)` items. The legacy `:switch-key` attribute on panels has been removed.

### 3.5 Persistence
*   **Window Position**: `MainForm` saves its X/Y coordinates to `ui_state.properties` on close.
*   **Restoration**: On startup, position is loaded. If the window is off-screen (e.g., monitor config change), it is automatically pulled back to visible coordinates.
*   **State**: Active tabs and other transient UI states are also persisted via `UIStateStorage`.

## 4. Development Guidelines

### Coding Standards
*   **UI Thread**: Swing components must be updated on the EDT (`SwingUtilities.invokeLater`).
*   **Event Bus**: Use `UIStateBus` for cross-component communication (e.g., config changes, refresh events).
*   **Performance**: Avoid object allocation in `paintComponent` or high-frequency loops (like `VoiceWarning.run`).

### Concurrency & Thread Safety
*   **Overlay Management**: `OverlayManager` methods (`open`, `close`, `refreshPreview`) must be `synchronized`.
    *   **Reason**: Configuration change events (`configChanged`) can fire rapidly and concurrently (e.g., during text input or bulk updates). Without synchronization, race conditions can spawn duplicate overlay instances ("Ghost Overlays").
*   **Event Handling**: Event subscribers might receive events on background threads. Ensure UI updates are dispatched to EDT.

### Common Patterns
*   **Renderer Pattern**: Implements `RowRenderer`. Must strictly adhere to constructing a `WebPanel` and binding to `ConfigService`.
*   **Dirty Check**: Store `lastValue` fields in UI components and only repaint/reformat when specific thresholds are exceeded.
