//! P6 批3 web 辅助窗口开窗层: 对比 (CompactComparisonWindow) / 功率曲线
//! (PowerCurveWindow) 两个 Java JDialog 的 Tauri WebviewWindow 对位。
//!
//! Java 对位关系:
//! - `ModalityType.MODELESS` → 独立 WebviewWindow (不阻塞主窗, 各自独立泵);
//! - `getWindowAncestor(p)` 的 parent 窗口祖先关系 = 主窗 (开窗位置对齐依据,
//!   `setLocationRelativeTo(owner)` → 新窗中心对齐主窗);
//! - `dispose()` → 窗口 X/CLOSE 按钮 (`getCurrentWindow().close()` → lib.rs
//!   on_window_event 非 main label 分支 destroy);
//! - 数据不在本层 — 前端窗口加载后经 W1 (`commands_windows::comparison_data` /
//!   `power_curve_data` / `fm_list`) 拉取, URL query 携带开窗参数
//!   (Java 构造器入参的传递面)。
//!
//! 线程约束: 窗口创建必须主线程 (tao) — 本模块由 vm-app dispatcher 在
//! `ShellForm::pump_once` 的主线程泵内调用, tauri-runtime-wry 对主线程调用走
//! 同步 handle_user_message (无事件循环死锁)。
//!
//! 重复开窗: Java 每次按钮 new 一个新 JDialog (可多实例并存); Tauri label
//! 唯一 → 同 label 单实例, 重开 = destroy 旧窗 + 按新参数重建 (数据全量重拉,
//! 与 Java 新窗等价, 备案差异: 不叠加多实例)。

use std::path::PathBuf;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, Wry};

use crate::MAIN_LABEL;

/// 对比窗口 label (capabilities/aux-windows.json 的 windows 域)
pub const COMPARISON_LABEL: &str = "comparison";
/// 功率曲线窗口 label
pub const POWER_CURVE_LABEL: &str = "powercurve";

/// 对比窗口尺寸: Java `pack()` 按内容自适应 (行清单数百行会超屏截断, JScrollPane
/// 被注释未启用) — web 取固定尺寸 + 内容区滚动 (可用性修正, 列宽比例保真)
const COMPARISON_SIZE: (f64, f64) = (560.0, 680.0);
/// 功率曲线窗口尺寸: Java `setSize(CHART_WIDTH + 80, CHART_HEIGHT + 150)`
const POWER_CURVE_SIZE: (f64, f64) = (1080.0, 800.0);

/// Java CompactComparisonWindow 构造器标题 (CompactComparisonWindow.java:40):
/// fm1 空 = 单机数据视图 "Aircraft Data: x", 否则 "Comparison: x vs y"
pub fn comparison_window_title(fm0: &str, fm1: Option<&str>) -> String {
    match fm1 {
        Some(n) if !n.is_empty() => format!("Comparison: {fm0} vs {n}"),
        _ => format!("Aircraft Data: {fm0}"),
    }
}

/// URL query 最小转义: 机型名域为 [a-z0-9_-], 防御 '&'/'#' 等破坏参数结构
fn url_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// 辅助窗口 URL (index.html?win=...&参数): 前端 main.tsx 按 win 路由到对应窗口根
fn aux_url(params: &[(&str, String)]) -> String {
    let mut url = String::from("index.html");
    for (i, (k, v)) in params.iter().enumerate() {
        url.push(if i == 0 { '?' } else { '&' });
        url.push_str(k);
        url.push('=');
        url.push_str(&url_escape(v));
    }
    url
}

/// 对比窗口 query (fm1 空/None = 单机数据视图, 不带参数)
fn comparison_query(fm0: &str, fm1: Option<&str>) -> String {
    let mut params = vec![("win", "comparison".to_string()), ("fm0", fm0.to_string())];
    if let Some(n) = fm1.filter(|s| !s.is_empty()) {
        params.push(("fm1", n.to_string()));
    }
    aux_url(&params)
}

/// 功率曲线 query。fm1 == fm0 归一为单曲线 (Java PowerCurveWindow.java:183
/// 构造器裁决, 提前在 URL 层生效)
fn power_curve_query(fm0: &str, fm1: Option<&str>, speed_kmh: i32, wep: bool) -> String {
    let fm1 = fm1.filter(|s| !s.is_empty() && *s != fm0);
    let mut params = vec![
        ("win", "powercurve".to_string()),
        ("fm0", fm0.to_string()),
        ("speed", speed_kmh.to_string()),
        ("wep", wep.to_string()),
    ];
    if let Some(n) = fm1 {
        params.push(("fm1", n.to_string()));
    }
    aux_url(&params)
}

/// 通用建窗: 销毁同 label 旧窗 (单实例, 见模块头) → 建新窗 → 中心对齐主窗。
/// 主窗查询失败 (理论上不发生) 落回构造期居中 (Java owner=null 时
/// setLocationRelativeTo 居屏的同效兜底)
fn build_aux_window(
    handle: &AppHandle<Wry>,
    label: &str,
    title: &str,
    size: (f64, f64),
    query: &str,
) -> Result<(), String> {
    if let Some(old) = handle.get_webview_window(label) {
        // 旧窗参数已过期 (cfg 换机), destroy 后重建触发前端全量重拉
        let _ = old.destroy();
    }
    let builder = WebviewWindowBuilder::new(
        handle,
        label,
        WebviewUrl::App(PathBuf::from(query.to_string())),
    )
    .title(title)
    .inner_size(size.0, size.1)
    .resizable(true)
    // Java setUndecorated(true): 无系统标题栏, 关闭走窗口内 CLOSE/ESC
    .decorations(false);

    let win = builder
        .build()
        .map_err(|e| format!("辅助窗口创建失败 ({label}): {e}"))?;

    // setLocationRelativeTo(owner): 新窗中心对齐主窗 (物理像素坐标)
    let centered = handle.get_webview_window(MAIN_LABEL).and_then(|main| {
        let pos = main.outer_position().ok()?;
        let msize = main.inner_size().ok()?;
        let scale = main.scale_factor().ok()?;
        let x = pos.x as f64 + (msize.width as f64 - size.0 * scale) / 2.0;
        let y = pos.y as f64 + (msize.height as f64 - size.1 * scale) / 2.0;
        Some(win.set_position(tauri::Position::Physical(PhysicalPosition::new(
            x as i32,
            y as i32,
        ))))
    });
    match centered {
        Some(Ok(())) | None => {}
        // 定位失败不阻开窗 (窗口仍由系统默认放置)
        Some(Err(e)) => {
            vm_core::logger::warn("WebWindows", &format!("辅助窗口定位失败 ({label}): {e}"));
        }
    }
    let _ = win.set_focus();
    Ok(())
}

/// 打开对比 web 窗口 (Java ButtonRowRenderer openComparison / FMListRowRenderer
/// View 按钮的 `new CompactComparisonWindow(...)` + setVisible(true))
pub fn open_comparison_window(
    handle: &AppHandle<Wry>,
    fm0: &str,
    fm1: Option<&str>,
) -> Result<(), String> {
    build_aux_window(
        handle,
        COMPARISON_LABEL,
        &comparison_window_title(fm0, fm1),
        COMPARISON_SIZE,
        &comparison_query(fm0, fm1),
    )
}

/// 打开功率曲线 web 窗口 (Java ButtonRowRenderer openPowerCurve 的
/// `new PowerCurveWindow(parent, fm0, fm1, speed, wep)` + setVisible(true))
pub fn open_power_curve_window(
    handle: &AppHandle<Wry>,
    fm0: &str,
    fm1: Option<&str>,
    speed_kmh: i32,
    wep: bool,
) -> Result<(), String> {
    build_aux_window(
        handle,
        POWER_CURVE_LABEL,
        "功率曲线",
        POWER_CURVE_SIZE,
        &power_curve_query(fm0, fm1, speed_kmh, wep),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 对比标题_单双模式() {
        // Java 构造器三目: fm1 null/空 → Aircraft Data
        assert_eq!(
            comparison_window_title("a_4h", Some("a6m5_zero")),
            "Comparison: a_4h vs a6m5_zero"
        );
        assert_eq!(comparison_window_title("a_4h", None), "Aircraft Data: a_4h");
        assert_eq!(
            comparison_window_title("a_4h", Some("")),
            "Aircraft Data: a_4h"
        );
    }

    #[test]
    fn 对比query_单机模式不带fm1() {
        assert_eq!(comparison_query("a_4h", None), "index.html?win=comparison&fm0=a_4h");
        assert_eq!(
            comparison_query("a_4h", Some("")),
            "index.html?win=comparison&fm0=a_4h"
        );
        assert_eq!(
            comparison_query("a_4h", Some("a6m5_zero")),
            "index.html?win=comparison&fm0=a_4h&fm1=a6m5_zero"
        );
        // 特殊字符转义 (机型名域不含, 防御面)
        assert_eq!(
            comparison_query("a&b", None),
            "index.html?win=comparison&fm0=a%26b"
        );
    }

    #[test]
    fn 功率曲线query_同名归一与参数序() {
        // fm1 == fm0 → 单曲线 (Java :183 构造器裁决)
        assert_eq!(
            power_curve_query("bf-109f-4", Some("bf-109f-4"), 400, true),
            "index.html?win=powercurve&fm0=bf-109f-4&speed=400&wep=true"
        );
        assert_eq!(
            power_curve_query("bf-109f-4", Some("p-51c-10-nt"), 0, false),
            "index.html?win=powercurve&fm0=bf-109f-4&speed=0&wep=false&fm1=p-51c-10-nt"
        );
        // 空串 fm1 归一
        assert_eq!(
            power_curve_query("bf-109f-4", Some(""), 0, false),
            "index.html?win=powercurve&fm0=bf-109f-4&speed=0&wep=false"
        );
    }
}
