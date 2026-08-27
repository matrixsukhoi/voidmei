//! prog/config 接口层移植 (P2 批二): ConfigProvider / OverlaySettings / HUDSettings。
//! 仅含三个接口文件; ConfigurationService / ConfigLoader 等 B 类实现不在本批,
//! 后续落各自模块 (如 crate::config_loader) 后在此补实现。

pub mod config_provider;
pub mod hud_settings;
pub mod hud_settings_snapshot;
pub mod overlay_settings;

pub use config_provider::ConfigProvider;
pub use hud_settings::HUDSettings;
pub use hud_settings_snapshot::HudSettingsSnapshot;
pub use overlay_settings::OverlaySettings;
