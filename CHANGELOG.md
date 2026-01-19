# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-01-19

###- **Refactored Configuration System (Category 2)**:
  - Migrated remaining logic parameters from `config.properties` to `ui_layout.cfg`.
  - **Color Fix**: Removed legacy `checkColorDefault` logic that was interfering with color loading. Implemented `refreshGlobalColors()` to ensure `Application` state is actively updated from `ui_layout.cfg` immediately after load.
  - **Layout-First Configuration Logic**:
    - Upgraded `ConfigurationService.getConfig()` to prioritize reading from `ui_layout.cfg` over `config.properties` for ALL keys.
    - Upgraded `ConfigurationService.setConfig()` to enforce dual-write: updates both `ui_layout.cfg` (in-memory) and `config.properties`.
    - This establishes `ui_layout.cfg` as the **Single Source of Truth** for duplicated keys.
    - **Batch Removal**: Safely deleted duplicate boolean keys (`disableEngineInfoThrottle`, `enableStatusBar`, etc.) from `config.properties` as they are now reliably served from Layout.
    - **Fix**: Implemented correct `SWITCH_INV` inversion logic in `ConfigurationService` to ensure keys like `disableFlightInfoIAS` are interpreted correctly (Store=True vs App=False).
    - **Defaults**: Updated `ui_layout.cfg` defaults to enable MiniHUD elements (`displayCrosshair`, `drawHUDtext`) by default, preventing "empty overlay" issues on fresh Layout-First load.
  - Implemented **Typed Value Model**: Configuration items in `ui_layout.cfg` are now parsed into strict Java types (Integer, Boolean) at load time, eliminating manual string parsing in renderers.
  - Removed legacy `visible` and `defaultValue` fields in favor of a unified, typed `value` field.
  - Exposed hidden logic settings (AoA Warning Ratios) as sliders in the Layout UI.
- **Logic Exposure**: Exposed `miniHUDaoaWarningRatio` and `miniHUDaoaBarWarningRatio` as sliders (0-100%) in `ui_layout.cfg`. Updated backend key logic to support legacy (0.0-1.0) and new percentage values automatically.
- **Config Persistence**: Refactored `ConfigLoader` and `RowRenderer`s to persist control values (Sliders, Combos) directly into `ui_layout.cfg`, enabling migration away from `config.properties`.
- **UI**: Standardized `FontSize` control across `FlightInfo`, `EngineControl`, and `MiniHUD` overlays using the new `ui_layout.cfg` structure.
