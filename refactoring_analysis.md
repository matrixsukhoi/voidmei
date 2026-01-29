# Controller Architecture Analysis & Refactoring Plan

## Current Status Analysis
`src/prog/Controller.java` currently acts as a **God Class**, violating the Single Responsibility Principle (SRP). It mixes:
1.  **Application Lifecycle**: Managing threads (`Service`, `MainForm`, `OverlayManager`).
2.  **Data Parsing**: Directly handling `Blkx` file loading and parsing.
3.  **Configuration**: Acting as a `ConfigProvider` proxy.
4.  **Global State**: Holding public static fields for overlay states.
5.  **State Machine**: implementing rigid transitions (`changeS2`, `changeS3`).

## Doc Alignment (`GEMINI.md`)
- **Violations**: The documentation states `Controller` is a "Central coordinator", but in practice, it is doing heavy lifting (parsing) that belongs in `parser` or a service, and holding state that belongs in `config`.

## Refactoring Directions

### 1. Extract Flight Data Service (High Impact)
**Problem**: The `loadFMData`, `ensureBlkxLoaded`, and `getBlkx` methods occupy ~15% of the class (Lines 654-793) and handle low-level file parsing logic.
**Solution**: Create `prog.service.FlightModelService`.
-   **Move**: `Blkx` instance, `loadedFMName`, `identifiedFMName`, `globalPool`.
-   **Move**: All parsing logic (`loadFMData`, `ensureBlkxLoaded`).
-   **Benefit**: Controller becomes a pure coordinator; parsing logic is isolated and testable.

### 2. Eliminate Static Global State (High Stability)
**Problem**: Lines 83-93 declare public static fields (e.g., `engineInfoSwitch`, `engineInfoX`).
```java
public static boolean engineInfoSwitch; // Global mutable state
public static int engineInfoX;
```
**Solution**: Move these fully into `ConfigurationService` or generic `OverlaySettings`.
-   **Action**: Replace direct static access with `configService.getOverlaySettings("EngineInfo").visible`.
-   **Benefit**: Eliminates "Spooky Action at a Distance" where any class can mutate global state, reducing bugs and improving thread safety.

### 3. Modernize State Machine (Architecture)
**Problem**: Methods `changeS2`, `changeS3`, `S4toS1` represent a hardcoded, rigid state machine legacy.
**Solution**: Implement a proper **Event-Driven State Machine**.
-   **Action**: `Service.java` should publish `GameStatusEvent` (e.g., `CONNECTED`, `IN_GAME`, `MENU`).
-   **Action**: `Controller` subscribes to these events and triggers `overlayManager.openAll()` or `closeAll()` via handlers.
-   **Benefit**: Decouples `Service` from `Controller` (currently `Service` likely calls `controller.changeS3()` directly).

## Recommendation
Start with **Direction 1 (Extract Flight Data Service)** as it clears the most complex non-coordinator logic from the class.
