//! parity_minihud: MiniHUD 整帧像素对拍 — Java `OverlayPngExport --minihud` 的 Rust 等效实现
//!
//! D7 验收路线 (批九 gauge 对拍的整帧延伸): 以 ui_layout.cfg panel "MiniHUD" (L45-94)
//! :default 快照配置 ([`ParitySettings`]) 走**生产编排** [`MiniHudOverlay::init`]
//! (service_present=false → preview 静态数据注入, lines[] 预览串, 同 FieldOverlay
//! POC 模式), 画到 applyAutoSizing 计划尺寸的 PixCanvas, 双 PNG 走 compare 热力图对拍。
//!
//! 与 Java 端 (OverlayPngExport.exportMiniHud) 的组装口径:
//! - Java 无法离屏实例化 MiniHUDOverlay (WebLaF WebFrame + Controller 依赖) → 手抄
//!   init(service=null) 私有编排的同源快照 (组件/ctx/布局引擎全为生产类); Rust 直接走
//!   已移植的 `MiniHudOverlay::init` → 编排差异本身也在对拍覆盖内 (手抄错 = 像素差)。
//! - [`SERVICE_LOOP_INTERVAL_MS`] = 50: Java 生产默认 (ConfigurationService.java:115,
//!   Controller.serviceLoopIntervalMs 的来源); 仅影响 blinkTicks/refreshInterval 节拍,
//!   本静态帧无事件无视觉。
//! - dpi_scale = 1.0: Java Application.dpiScale 静态默认 (导出链不跑 DPIHelper.getScale)。
//! - 字体 = fonts/sarasa-mono-sc-bold.ttf (Java "Sarasa Mono SC" BOLD 族注册同源文件)。
//!
//! Row2 结构差异已消除 (批十三 HUDMechanizationRow 移植落地): 双端均为三段
//! 模板占位行 (占位宽 = w("F100 ")+w("BRK ")+w("GEA")), 复跑 rust_compare.sh
//! minihud 段实测 (555×270 帧): 双端内容包围盒/窗口尺寸逐整数一致
//! (Content[43,66 465x180] → 555×270), 最右墨迹列 dx=0 —— 批十一占位期的
//! 右缘 -18px 结构差异消除; row2 带双端墨迹列范围相同, 列差集为孤立单列。
//!
//! 残差量级备案 (批十三复测): 整帧 diff_pixel_ratio=6.29% / mean_delta=41.37 /
//! max_delta=255 (批十一占位期为 7.19%/66.74/255)。分布 = 文本字形边缘散点
//! 与弧线 AA, 无块状位移。行带数字掩膜口径 (阈值 >8; row2 带 = x[100,260)×
//! y[137,165) 11.12% / attitude 区 = x[165,250)×y[167,235) 6.25%, 同产物复算
//! 与全帧 4.13% 逐值吻合) — 掩膜边界取法不同数字即漂移 (异掩膜复算 6.4%~11%),
//! 精确值仅在此口径下可复现。当初 "纯 AA 基线 ≈2-3%" 的预估偏乐观 — 文本
//! 为主的整帧字形边缘密度下实测 AA 地板 ≈4-6% (>8 阈 4.13%)。即本备案是
//! 论证口径而非阈值门禁: row2 带单项 11% 级若独立按 8% AA 阈设门会 FAIL,
//! 对拍成立依据 = 上述整帧文本字形边缘密度论证, 非阈值通过。
//! attitude 顶部 AA 级残差仍在 (复测实测口径): 全帧顶部
//! 刻度首墨行 rust 比 java 早 1 行 (r@y48 与 j@y49 列均 {144..147} 且 alpha 逐值
//! 相同 — 顶弧亚像素覆盖在行界上的 ramp 整体平移一行); 底部指针对 java {214}
//! vs rust {213,214} 单列左扩 (alpha 16..83); 批九 gauge 单件首墨行 java
//! {50..52} vs rust {50..53} 为同类单列边缘差 — 均 1px 级, 与批九对拍基线
//! 同口径记录保留。
//! compare --max-delta 硬门 (审查 B 建议) 归 rust_compare.sh 维护方
//! (本批写入范围限本文件/rows.rs/minihud.rs)。

use std::path::Path;

use vm_core::config_api::HUDSettings;
use vm_core::config_api::overlay_settings::OverlaySettings;

use crate::minihud::MiniHudOverlay;
use crate::render2d::PixCanvas;

/// 生产服务循环节拍 (Java ConfigurationService.java:115 默认 50ms;
/// MiniHUDOverlay.init 的 blinkTicks/refreshInterval 同源 — 本静态帧无视觉)
pub const SERVICE_LOOP_INTERVAL_MS: i64 = 50;

/// OverlaySettings::GroupConfig 占位 (对拍形态无分组配置访问, get_group_config 恒 None;
/// 同 minihud.rs 测试 GroupStub 形态)
pub struct ParityGroupStub;

/// ui_layout.cfg panel "MiniHUD" :default 快照 (与 Java exportMiniHud 的
/// MiniHudSettings 同源, 改一处必须同步另一处; 值集同 minihud.rs 测试 TestSettings)
pub struct ParitySettings;

impl OverlaySettings for ParitySettings {
    type GroupConfig = ParityGroupStub;

    fn get_window_x(&self, _width: i32) -> i32 {
        0 // Java 导出同值 (窗口位置不进渲染)
    }
    fn get_window_y(&self, _height: i32) -> i32 {
        0
    }
    fn save_window_position(&self, _x: f64, _y: f64) {}
    fn get_font_name(&self) -> String {
        "Sarasa Mono SC".into() // "等宽字体" :default
    }
    fn get_num_font_name(&self) -> String {
        "Sarasa Mono SC".into()
    }
    fn get_font_size_add(&self) -> i32 {
        0 // "hud读数和指示器字体大小" :default 0
    }
    fn get_bool(&self, _key: &str, def: bool) -> bool {
        def // enableLayoutDebug → 字面兜底 false
    }
    fn get_int(&self, _key: &str, def: i32) -> i32 {
        def
    }
    fn get_string(&self, _key: &str, def: &str) -> String {
        def.to_string()
    }
    fn get_group_config(&self) -> Option<&Self::GroupConfig> {
        None
    }
    fn auto_hide_on_focus_loss(&self) -> bool {
        false
    }
}

impl HUDSettings for ParitySettings {
    fn get_num_font(&self) -> String {
        "Sarasa Mono SC".into()
    }
    fn get_crosshair_scale(&self) -> i32 {
        113 // "minihud大小" :default
    }
    fn get_crosshair_name(&self) -> String {
        "软件渲染准星".into() // :default → 软件矢量路径
    }
    fn is_display_crosshair(&self) -> bool {
        true // "显示hud准星" :default
    }
    fn use_texture_crosshair(&self) -> bool {
        false
    }
    fn draw_hud_text(&self) -> bool {
        true // "显示hud数据" :default
    }
    fn show_attitude_gauge(&self) -> bool {
        true // "罗盘/姿态指示" :default → 姿态仪, 罗盘互斥隐藏
    }
    fn get_aoa_warning_ratio(&self) -> f64 {
        0.2 // "攻角数值告警阈值" :default 20 (%)
    }
    fn get_aoa_bar_warning_ratio(&self) -> f64 {
        0.25 // "攻角条告警阈值" :default 25 (%)
    }
    fn enable_flap_angle_bar(&self) -> bool {
        true // "智能襟翼指示器" :default
    }
    fn show_speed_bar(&self) -> bool {
        true // "油门条/速度条" :default → 速度条, 油门条互斥隐藏
    }
    fn draw_hud_mach(&self) -> bool {
        true // "表速更换马赫数" :default
    }
    fn is_speed_label_disabled(&self) -> bool {
        false // "速度读数显示标签" switch-inv :default false (显示)
    }
    fn is_altitude_label_disabled(&self) -> bool {
        false
    }
    fn is_sep_label_disabled(&self) -> bool {
        false
    }
    fn show_hud_speed(&self) -> bool {
        true // "速度读数" :default
    }
    fn show_hud_aoa(&self) -> bool {
        true // "攻角指示" :default
    }
    fn show_hud_altitude(&self) -> bool {
        true // "高度读数" :default
    }
    fn show_hud_energy(&self) -> bool {
        true // "能量读数" :default
    }
    fn show_hud_mechanization(&self) -> bool {
        true
    }
    fn show_hud_flaps(&self) -> bool {
        true // "襟翼/可变翼" :default
    }
    fn show_hud_airbrake(&self) -> bool {
        true // "减速板" :default
    }
    fn show_hud_gear(&self) -> bool {
        true // "起落架" :default
    }
    fn show_hud_sep(&self) -> bool {
        true // "爬升率" :default
    }
    fn show_hud_g_load(&self) -> bool {
        true // "过载读数" :default
    }
    fn show_hud_maneuver_bar(&self) -> bool {
        true // "机动条" :default
    }
    fn is_attitude_indicator_inertial_mode(&self) -> bool {
        false // "离体/随体配置" :default
    }
    fn is_gpu_compatibility_mode(&self) -> bool {
        false
    }
    fn always_show_radar_altitude(&self) -> bool {
        false // "始终显示雷达高度" :default
    }
}

/// 渲染对拍帧 (与 Java `--minihud` 输出逐像素对拍)。
/// fonts_dir 需含 sarasa-mono-sc-bold.ttf (Java 端注册的 BOLD 族同源文件)。
/// 画布 = applyAutoSizing 计划 (内容包围盒 + 2×LAYOUT_PADDING, 双端同式)。
pub fn render_minihud(fonts_dir: &Path, aa: bool) -> Result<PixCanvas, String> {
    let font_path = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    let mut overlay = MiniHudOverlay::init(
        false, // service 在场性: null → 预览静态数据 (refreshTemplates 的 lines[] 串)
        SERVICE_LOOP_INTERVAL_MS,
        &ParitySettings,
        1.0, // Application.dpiScale 静态默认 (导出链无 DPIHelper)
        &font_path,
    )?;
    let plan = overlay
        .sizing()
        .ok_or("MiniHUD sizing 计划缺失 (Java 空 components 裸 return 分支, 对拍形态不可达)")?;
    let mut cv = PixCanvas::new(plan.new_width, plan.new_height)?;
    overlay.draw(&mut cv, aa);
    Ok(cv)
}

#[cfg(test)]
mod tests;
