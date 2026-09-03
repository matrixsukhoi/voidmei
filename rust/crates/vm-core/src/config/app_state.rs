//! Application/Controller 消费面依赖桩 (重构波2 自 configuration_service.rs 尾部迁出):
//! Java 静态字段群 (五色/轮询间隔/端口/字体) 的服务持有态 + 抗锯齿值域枚举
//! + java.net.InetSocketAddress 的最小替身。

// =====================================================================
// java.awt.RenderingHints 抗锯齿常量 → 自定义枚举 (CLASSIFY 裁决)
// =====================================================================

/// 文本抗锯齿值域 (VALUE_TEXT_ANTIALIAS_ON / _OFF)。
/// Gasp = VALUE_TEXT_ANTIALIAS_GASP — Application 的声明默认值,
/// loadAppCheck 只写 On/Off, Gasp 仅为保真初始态 (§2.10)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAaSetting {
    On,
    Off,
    Gasp,
}

/// 图形抗锯齿值域 (VALUE_ANTIALIAS_ON / _OFF)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphAaSetting {
    On,
    Off,
}

// =====================================================================
// java.net.InetSocketAddress 消费面
// =====================================================================

/// java.net.InetSocketAddress 消费面: (hostname, port) 二元组。
/// Java 允许未解析主机名 (构造器内捕获 UnknownHostException, 持 hostname
/// 不解析), 故不以 SocketAddr (需可解析 IP) 建模; 端口保持 int — JDK 构造器
/// 对 port ∉ [0,65535] 抛 IllegalArgumentException (见 new)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InetSocketAddress {
    pub host: String,
    pub port: i32,
}

impl InetSocketAddress {
    /// Java: `new InetSocketAddress(String hostname, int port)`
    /// PORT §1: JDK 构造器对 port < 0 或 > 65535 抛 IllegalArgumentException —
    /// **不是** NumberFormatException, 不被 loadAppCheck 的 catch 捕获, 直接
    /// 传播出方法 (Java 调用方面对崩溃); Rust 以 panic! 复刻该抛出面。
    /// (hostname null 检查同抛 IAE, 但 &str 域不可达。)
    pub fn new(host: &str, port: i32) -> Self {
        if !(0..=65535).contains(&port) {
            panic!("port out of range: {port} (java.lang.IllegalArgumentException)");
        }
        InetSocketAddress {
            host: host.to_string(),
            port,
        }
    }
}

/// Application.defaultFont 的最小替身 (java.awt.Font 属 C 类)。
/// initFont() 落地前为 None (Java: 静态字段 null → getFontName() NPE)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppFont {
    /// Java: `Font.getFontName()` 的返回域
    pub name: String,
}

/// Application 静态字段的消费面 (依赖桩, 非翻译):
/// Java Application 五色静态字段 (colorNum/colorLabel/colorUnit/colorWarning/
/// colorShadeShape) 的快照形态 — cfg 全局键 fontNum/fontLabel/fontUnit/
/// fontWarn/fontShade (ui_layout.cfg:379-383) 经 loadFromConfig 覆盖为运行时
/// 真值; 组件消费经 vm-overlay 的 global_colors 受控全局仓
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalColors {
    pub num: [u8; 4],
    pub label: [u8; 4],
    pub unit: [u8; 4],
    pub warning: [u8; 4],
    pub shade_shape: [u8; 4],
}

impl GlobalColors {
    /// Java Application 静态初始值 (cfg 加载前的默认;
    /// 与 vm-overlay 各组件原编译期常量逐字节一致)
    pub const JAVA_DEFAULT: GlobalColors = GlobalColors {
        num: [27, 255, 128, 240],
        label: [27, 255, 128, 166],
        unit: [166, 166, 166, 220],
        warning: [216, 33, 13, 100],
        shade_shape: [0, 0, 0, 42],
    };
}

/// 仅收录 ConfigurationService.java 读/写触达的成员; 声明默认值 =
/// Application.java 字段初始化值 (§2.10 按有意保真)。
/// PORT: Java 全局静态 → 服务持有的注入态 (§2.9 禁裸全局; vm-app 波次收口)。
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationState {
    /// `public static long threadSleepTime = 33`
    pub thread_sleep_time: i64,
    /// `public static Color colorNum = new Color(27, 255, 128, 240)`
    pub color_num: [u8; 4],
    /// `public static Color colorLabel = new Color(27, 255, 128, 166)`
    pub color_label: [u8; 4],
    /// `public static Color colorUnit = new Color(166, 166, 166, 220)`
    pub color_unit: [u8; 4],
    /// `public static Color colorWarning = new Color(216, 33, 13, 100)`
    pub color_warning: [u8; 4],
    /// `public static Color colorShadeShape = new Color(0, 0, 0, 42)`
    pub color_shade_shape: [u8; 4],
    /// `public static int voiceVolumn = 100`
    pub voice_volumn: i32,
    /// `public static Boolean aaEnable = true`
    pub aa_enable: bool,
    /// `public static Object textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_GASP`
    pub text_aa_setting: TextAaSetting,
    /// `public static Object graphAASetting = RenderingHints.VALUE_ANTIALIAS_ON`
    pub graph_aa_setting: GraphAaSetting,
    /// `public static int displayFmKey = NativeKeyEvent.VC_P` (VC_P = 25)
    pub display_fm_key: i32,
    /// `public static int appPort` (声明默认 0)
    pub app_port: i32,
    /// `public static int appPortBkp` (声明默认 0)
    pub app_port_bkp: i32,
    /// `public static SocketAddress requestDest` (声明默认 null)
    pub request_dest: Option<InetSocketAddress>,
    /// `public static SocketAddress requestDestBkp` (声明默认 null)
    pub request_dest_bkp: Option<InetSocketAddress>,
    /// `public static String defaultNumfontName = "Roboto"`
    pub default_numfont_name: String,
    /// `public static String defaultFontName = "Microsoft YaHei UI"`
    pub default_font_name: String,
    /// `public static Font defaultFont` (声明默认 null; initFont 赋值)
    pub default_font: Option<AppFont>,
    /// `public static int screenWidth` (声明默认 0; getScreenSize() 覆写)
    pub screen_width: i32,
    /// `public static int screenHeight` (声明默认 0)
    pub screen_height: i32,
}

impl ApplicationState {
    /// Java 字段声明初始化值的等价构造
    pub fn new() -> Self {
        ApplicationState {
            thread_sleep_time: 33,
            color_num: [27, 255, 128, 240],
            color_label: [27, 255, 128, 166],
            color_unit: [166, 166, 166, 220],
            color_warning: [216, 33, 13, 100],
            color_shade_shape: [0, 0, 0, 42],
            voice_volumn: 100,
            aa_enable: true,
            text_aa_setting: TextAaSetting::Gasp,
            graph_aa_setting: GraphAaSetting::On,
            display_fm_key: 25,
            app_port: 0,
            app_port_bkp: 0,
            request_dest: None,
            request_dest_bkp: None,
            default_numfont_name: "Roboto".to_string(),
            default_font_name: "Microsoft YaHei UI".to_string(),
            default_font: None,
            screen_width: 0,
            screen_height: 0,
        }
    }
}

impl Default for ApplicationState {
    fn default() -> Self {
        ApplicationState::new()
    }
}

/// Controller 轮询间隔字段的消费面 (依赖桩, 非翻译):
/// loadAppCheck 写入的 6 个 `public long` 字段 (Java 声明默认 0, §2.10)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControllerIntervals {
    pub service_loop_interval_ms: i64,
    pub engine_info_interval_ms: i64,
    pub flight_info_interval_ms: i64,
    pub altitude_interval_ms: i64,
    pub gear_flaps_interval_ms: i64,
    pub control_input_interval_ms: i64,
}
