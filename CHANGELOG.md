# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-01-19

### Refactor
- **Configuration**: Removed legacy UI layout settings from `config.properties`. The following settings are now exclusively managed by `ui_layout.cfg`:
  - **Coordinates**: `flightInfoX/Y`, `engineInfoX/Y`, `engineControlX/Y`, `gearAndFlapsX/Y`, `attitudeIndicatorX/Y`, `stickValueX/Y`, `crosshairX/Y` (MiniHUD).
  - **Layout**: `flightInfoColumn`, `engineInfoColumn`.
  - **Fonts**: `flightInfoFontC`, `flightInfoFontaddC`, `engineInfoFont`, `engineInfoFontadd`, `GlobalNumFont`.
- **UI**: Standardized `FontSize` control across `FlightInfo`, `EngineControl`, and `MiniHUD` overlays using the new `ui_layout.cfg` structure.
