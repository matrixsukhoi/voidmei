//! Env — Application 静态只读区落位 (D8 表: 启动一次后只读 → 构造注入) +
//! 启动探测辅助 (字体目录/模板 cfg/DPI)。
//! 重构波2 自 app_shell.rs 拆出。

use std::path::{Path, PathBuf};

use vm_core::lang::Lang;
use vm_overlay::platform::extras::DpiHelper;

/// Application.java:60-129 静态字段中"启动一次后只读"组的落位
/// (LIFETIMES §1.2 → Env; 配置驱动可变组归 ConfigurationService 的 ApplicationState)。
#[derive(Debug, Clone)]
pub struct Env {
    /// Application.version (Java 读 MANIFEST; Rust 编译期注入)
    pub version: String,
    /// Application.appName = Lang.appName (Application.java:569)
    pub app_name: String,
    /// Application.httpHeader = Lang.httpHeader (:571)
    pub http_header: String,
    /// Application.appPort (Lang.httpPort parseInt, 失败 8111; :559-563)
    pub app_port: u16,
    /// 字体目录探测 (Java initFont 的 AWT 注册 → Rust 字体文件路径供给, D8: 字体→win32 线程)
    pub fonts_dir: PathBuf,
    /// 托盘图标 (Application.initSystemTray: "image/16x16.png")
    pub icon_path: PathBuf,
    /// 屏幕快照 (Application.getScreenSize/DPIHelper; D8: 屏幕尺寸→win32 线程启动快照)
    pub dpi: DpiHelper,
    /// Application.debug (OverlayContext.isDebug 的来源)
    pub debug: bool,
    /// 白盒端口 CLI 覆盖 (`--port` / mock-smoke 9222): 优先级压过 cfg 的
    /// httpPort 键 (smoke 踩坑: 用户 cfg 写死 httpPort=8111 令 9222 注入失效,
    /// 游戏在场时假 PASS / 离线时 FAIL)。生产 desktop 恒 None — cfg > Lang 不变
    pub port_override: Option<u16>,
}

impl Env {
    /// Java Application.main 启动序的只读区构造 (Lang → 端口 → 字体目录 → 屏幕探测)。
    pub fn probe(lang: &Lang, debug: bool) -> Env {
        let app_port = lang.http_port.parse::<u16>().unwrap_or(8111);
        Env {
            version: env!("CARGO_PKG_VERSION").to_string(),
            app_name: lang.app_name.to_string(),
            http_header: lang.http_header.to_string(),
            app_port,
            fonts_dir: probe_fonts_dir(),
            icon_path: PathBuf::from("image/16x16.png"),
            dpi: detect_dpi(),
            debug,
            port_override: None,
        }
    }
}

/// 字体目录探测: ./fonts → ../fonts (vm-overlay main.rs find_fonts_dir 同款,
/// 仓库根或 rust/ 下运行均可)
pub(crate) fn probe_fonts_dir() -> PathBuf {
    for cand in ["./fonts", "../fonts"] {
        if Path::new(cand).is_dir() {
            return PathBuf::from(cand);
        }
    }
    PathBuf::from("./fonts")
}

/// 仓库模板 ui_layout.cfg 探测。
/// 生产 CWD=仓库根 (java -jar / rust_run.sh); 测试 CWD=crate 根 (cargo 惯例),
/// 上溯三级 (vm-app → crates → rust → 仓库根) — vm-core/vm-overlay 测试同款路径
pub(crate) fn locate_template_cfg() -> Option<String> {
    let mut candidates: Vec<PathBuf> =
        [PathBuf::from("ui_layout.cfg"), PathBuf::from("../ui_layout.cfg")].to_vec();
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ui_layout.cfg"),
    );
    candidates.into_iter().find(|p| p.exists()).map(|p| {
        p.to_string_lossy().into_owned()
    })
}

/// Java Application.getScreenSize → DPIHelper.init() (DpiHelper.java:52)
#[cfg(target_os = "windows")]
fn detect_dpi() -> DpiHelper {
    DpiHelper::init()
}

/// 非 Windows: 屏幕探测未移植 (x11 波次), 100% 缩放回退 + 显式注明
#[cfg(not(target_os = "windows"))]
fn detect_dpi() -> DpiHelper {
    DpiHelper::fallback(1920, 1080, "非 Windows 屏幕探测未移植 (x11 波次)")
}
// Java 标准库语义助手已收敛 vm_core::base::java_compat
// (java_parse_boolean / current_time_millis), 本模块不再持本地副本。
