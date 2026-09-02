//! EngineControlOverlay (ui/overlay/EngineControlOverlay.java) — 引擎控制条形仪表。
//! 重构波2 自 overlays_field1.rs 拆出。
//!
//! LabeledLinearGauge 条形仪表 (竖条 throttle/pitch/power + 横条 mixture/radiator/
//! compressor/fuel), COMPRESSOR 走 MarkedGauge 画 optimal 档标记; onFlightData
//! 节流间隔配置驱动 (loadRefreshInterval)。

use std::cell::RefCell;
use std::rc::Rc;

use crate::render::palette::{aa, colors};
use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::overlays::bars::LabeledLinearGauge;
use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::overlays::gauges::{GaugeBarStyle, GaugeMarker, MarkedGauge, MarkerType};
use vm_core::base::event::EventPayload;
use vm_core::base::format::{self, java_round_f64, java_round_f32};
use vm_core::formula::registry::FormulaView;
use vm_core::lang::Lang;
// EngineControlOverlay.java:50 DEFAULT_REFRESH_INTERVAL 的既有移植 (单一来源, 勿重复定义)
use crate::layout::ui_constants::ENGINE_DEFAULT_REFRESH_MS;

/// EngineControlOverlay.java:54-56 GaugeType 枚举 (ordinal 即 gaugeType 字段值)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeType {
    Throttle,
    Pitch,
    Power,
    Mixture,
    Radiator,
    Compressor,
    Fuel,
}

impl GaugeType {
    /// Java GaugeType.values()[gf.gaugeType] 反查
    pub fn from_ordinal(o: i32) -> GaugeType {
        match o {
            0 => GaugeType::Throttle,
            1 => GaugeType::Pitch,
            2 => GaugeType::Power,
            3 => GaugeType::Mixture,
            4 => GaugeType::Radiator,
            5 => GaugeType::Compressor,
            _ => GaugeType::Fuel,
        }
    }

    pub fn ordinal(self) -> i32 {
        match self {
            GaugeType::Throttle => 0,
            GaugeType::Pitch => 1,
            GaugeType::Power => 2,
            GaugeType::Mixture => 3,
            GaugeType::Radiator => 4,
            GaugeType::Compressor => 5,
            GaugeType::Fuel => 6,
        }
    }
}

/// Lang 标签访问器 (cfg 无 lang 快照, EngineControl 标签全部来自 Lang 静态字段)
fn lbl_throttle(l: &Lang) -> &'static str {
    l.e_throttle
}
fn lbl_proppitch(l: &Lang) -> &'static str {
    l.e_proppitch
}
fn lbl_power_percent(l: &Lang) -> &'static str {
    l.e_power_percent
}
fn lbl_mixture(l: &Lang) -> &'static str {
    l.e_mixture
}
fn lbl_radiator(l: &Lang) -> &'static str {
    l.e_radiator
}
fn lbl_compressor(l: &Lang) -> &'static str {
    l.e_compressor
}
fn lbl_fuel_per(l: &Lang) -> &'static str {
    l.e_fuel_per
}

/// 单个仪表定义 (EngineControlOverlay.initGaugeFields 的 addGaugeIfEnabled 参数快照,
/// ui_layout.cfg "引擎控制"→"发动机元素" 组的 switch-inv :target 即 disableKey)。
/// PORT: 无 PartialEq — label 是 fn 指针, 地址比较无意义 (rustc 同款告警)
#[derive(Debug, Clone, Copy)]
pub struct EngineGaugeDef {
    /// 开关键 ("true" 时该仪表不建)
    pub disable_key: &'static str,
    /// 字段键 (GaugeField key)
    pub key: &'static str,
    /// Lang 标签访问器
    pub label: fn(&Lang) -> &'static str,
    pub unit: &'static str,
    pub gauge_type: GaugeType,
    pub max_value: i32,
    pub is_horizontal: bool,
}

/// initGaugeFields (EngineControlOverlay.java:224-244) 的 7 条定义, 顺序原样
/// 7 仪表 disable 键 (ENGINE_GAUGE_DEFS 顺序; Java initGaugeFields 读
/// ui_layout.cfg:185-191 发动机元素组 switch-inv, 审查轮 1-B: 曾 never-wired
/// 恒显全部 7 条 — vm-app 经 OverlayInputs 按此表序传 [bool; 7])
pub const ENGINE_DISABLE_KEYS: [&str; 7] = [
    "disableEngineInfoThrottle",
    "disableEngineInfoPitch",
    "disableEngineInfoPower",
    "disableEngineInfoMixture",
    "disableEngineInfoRadiator",
    "disableEngineInfoCompressor",
    "disableEngineInfoLFuel",
];

pub const ENGINE_GAUGE_DEFS: &[EngineGaugeDef] = &[
    EngineGaugeDef {
        disable_key: "disableEngineInfoThrottle", key: "throttle",
        label: lbl_throttle, unit: "%",
        gauge_type: GaugeType::Throttle, max_value: 110, is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoPitch", key: "pitch",
        label: lbl_proppitch, unit: "%",
        gauge_type: GaugeType::Pitch, max_value: 100, is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoPower", key: "power",
        label: lbl_power_percent, unit: "%",
        gauge_type: GaugeType::Power, max_value: 100, is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoMixture", key: "mixture",
        label: lbl_mixture, unit: "%",
        gauge_type: GaugeType::Mixture, max_value: 120, is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoRadiator", key: "radiator",
        label: lbl_radiator, unit: "%",
        gauge_type: GaugeType::Radiator, max_value: 100, is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoCompressor", key: "compressor",
        label: lbl_compressor, unit: "",
        gauge_type: GaugeType::Compressor, max_value: 1, is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoLFuel", key: "fuel",
        label: lbl_fuel_per, unit: "%",
        gauge_type: GaugeType::Fuel, max_value: 100, is_horizontal: true,
    },
];

/// EngineControlOverlay.java:47-49 常量
const BASE_FONT_SIZE: i32 = 24;
const WIDTH_MULTIPLIER: i32 = 8;
/// serviceLoopIntervalMs * 2 (EngineControlOverlay.java:51 ENGINE_REFRESH_MULTIPLIER);
/// 默认间隔 100ms 复用 crate::layout::ui_constants::ENGINE_DEFAULT_REFRESH_MS (:50, 单一来源)
pub const ENGINE_REFRESH_MULTIPLIER: f64 = 2.0;

/// 单个仪表的运行态 (Java GaugeField + 其 Swing 组件; Rust 组件直接拥有)
pub struct EngineGauge {
    pub key: String,
    pub gauge_type: GaugeType,
    pub max_value: i32,
    pub is_horizontal: bool,
    /// 动态可见性 (PITCH/MIXTURE 无数据、COMPRESSOR 0 档时隐藏)
    pub visible: bool,
    /// Java GaugeField.gauge (LabeledLinearGauge 组件)
    pub gauge: LabeledLinearGauge,
    /// Java GaugeField.markedGauge (COMPRESSOR 专用)
    pub marked_gauge: Option<MarkedGauge>,
}

/// isJetHiddenGauge (EngineControlOverlay.java:356-361)
fn is_jet_hidden_gauge(t: GaugeType) -> bool {
    matches!(
        t,
        GaugeType::Pitch | GaugeType::Radiator | GaugeType::Compressor | GaugeType::Mixture
    )
}

/// 引擎控制面板状态: 布局 (fontsize/width/height) + 仪表表 + 喷气机门控状态机。
/// 数据更新走 [`EngineControlState::update`] (onFlightData 路径), 绘制走
/// [`EngineControlState::draw`] (paintComponent → drawGauges)。
pub struct EngineControlState {
    /// loadFontConfig: round((24+fontadd)*dpiScale)
    pub font_size: i32,
    /// calculateLayout: fontsize*WIDTH_MULTIPLIER
    pub width: i32,
    /// calculateLayout: (移位优先级陷阱见 new 内注释)
    pub height: i32,
    /// 横向仪表数 (calculateLayout 的高度公式项; Java 包内可见, reinitConfig 复算)
    pub row_num: i32,
    /// 节流间隔 ms (EngineControlOverlay.java:61 refreshInterval, loadRefreshInterval
    /// 配置驱动: dataPollIntervalMs×2 → legacy "Interval"×2, 双键空保持默认 100)
    pub refresh_interval: i64,
    /// 节流基准 (:62 lastRefreshTime, System.currentTimeMillis 毫秒)
    pub last_refresh_time: i64,
    pub(crate) gauges: Vec<EngineGauge>,
    is_jet: bool,
    /// 引擎类型检测一次性闩锁 (updateStateFromPayload)
    jet_label_updated: bool,
    /// 增压器量程一次性写入闩锁
    compressor_max_value_set: bool,
}

impl EngineControlState {
    /// init → reinitConfig 链: loadFontConfig + loadRefreshInterval + initGaugeFields +
    /// calculateLayout + 末尾 updateGaugesPreview (:179-188)。
    /// cfg_true = `"true".equals(getConfigSafe(key))` 的配置探测 (POC 未接配置层,
    /// 恒 false 即全启用; 接配置层时传入 Boolean.parseBoolean 语义);
    /// cfg_str = getConfigSafe 的字符串读取 (loadRefreshInterval 专用, POC 传空串
    /// 读取器即恒默认间隔)
    pub fn new(
        lang: &Lang,
        font_add: i32,
        dpi_scale: f64,
        cfg_true: &dyn Fn(&str) -> bool,
        cfg_str: &dyn Fn(&str) -> String,
    ) -> Self {
        // loadFontConfig (EngineControlOverlay.java:191-200); label 字体由 draw 的
        // 调用方持有 (Java fontLabel 字段)
        let font_size = java_round_f64((BASE_FONT_SIZE as f64 + font_add as f64) * dpi_scale);
        // initGaugeFields (:224-244)
        let mut gauges = Vec::new();
        let mut row_num = 0;
        for def in ENGINE_GAUGE_DEFS {
            // addGaugeIfEnabled: !"true".equals(getConfigSafe(disableKey)) 才建
            if cfg_true(def.disable_key) {
                continue;
            }
            let label = (def.label)(lang);
            let gauge = EngineGauge {
                key: def.key.to_string(),
                gauge_type: def.gauge_type,
                max_value: def.max_value,
                is_horizontal: def.is_horizontal,
                visible: true,
                // GaugeField 构造: new LabeledLinearGauge(label, maxValue, !isHorizontal)
                gauge: LabeledLinearGauge::new(label, def.max_value, !def.is_horizontal),
                marked_gauge: None,
            };
            let mut gauge = gauge;
            // COMPRESSOR 用 MarkedGauge 画 optimal 档指示 (addGaugeIfEnabled 内)
            if def.gauge_type == GaugeType::Compressor {
                let style = GaugeBarStyle {
                    fill_color: colors().num,
                    background_color: [0, 0, 0, 0], // 透明背景
                    border_color: colors().shade_shape,
                    show_border: true,
                    vertical: !def.is_horizontal, // COMPRESSOR 横条 → vertical=false
                    stroke_width: 2,
                };
                let mut mg = MarkedGauge::new();
                mg.label = label.to_string();
                mg.set_max_value(def.max_value as f64);
                mg.set_bar_style(style);
                // optimal 档标记 (初始 ratio=-1 隐藏, colorWarning)
                mg.add_marker(GaugeMarker {
                    id: "optimal".to_string(),
                    marker_type: MarkerType::LineFull,
                    ratio: -1.0,
                    color: colors().warning,
                    ..GaugeMarker::default()
                });
                gauge.marked_gauge = Some(mg);
            }
            gauges.push(gauge);
            if def.is_horizontal {
                row_num += 1;
            } else {
                // columnNum 计数后从未参与公式 (Java 同, 仅循环结构保留)
            }
        }
        // calculateLayout (:214-222)
        let width = font_size * WIDTH_MULTIPLIER;
        // PORT: Java `(fontsize * 4 + (fontsize * 9) >> 1)` — JLS 移位优先级低于加法
        // → (13*fontsize)>>1 (LinearGaugeRenderer.java:71 同款陷阱, 勿加括号)
        let height =
            ((font_size * 4 + font_size * 9) >> 1) + (row_num + 1) * (font_size + (font_size >> 2));
        let mut st = EngineControlState {
            font_size,
            width,
            height,
            row_num,
            refresh_interval: ENGINE_DEFAULT_REFRESH_MS,
            last_refresh_time: 0,
            gauges,
            is_jet: false,
            jet_label_updated: false,
            compressor_max_value_set: false,
        };
        // reinitConfig 链内 loadRefreshInterval (:179/:202-212)
        st.load_refresh_interval(cfg_str);
        // reinitConfig 末尾 updateGaugesPreview (:187): 游戏模式与预览共用此初值 —
        // 全仪表 maxValue/2 且可见, 首个有效事件 (引擎检测 ~5s) 前显示半量程条
        st.update_preview();
        st
    }

    /// loadRefreshInterval (EngineControlOverlay.java:202-212): 先取
    /// dataPollIntervalMs, 空则回退 legacy "Interval"; 两键皆空保持现值 (默认 100)。
    /// reinit 时宿主可再次调用以随配置更新间隔
    pub fn load_refresh_interval(&mut self, cfg_str: &dyn Fn(&str) -> String) {
        // Try new config key first, fallback to legacy key for backward compatibility
        let mut interval_val = cfg_str("dataPollIntervalMs");
        if interval_val.is_empty() {
            interval_val = cfg_str("Interval"); // Legacy key fallback
        }
        if !interval_val.is_empty() {
            // parseLongSafe (:301-309): null/空/解析异常 → defaultVal (§2.15)
            let service_loop_interval_ms =
                interval_val.parse::<i64>().unwrap_or(ENGINE_DEFAULT_REFRESH_MS);
            // PORT: Java (long)(long * double) 经 f64 再截断, 保持同路径
            self.refresh_interval = (service_loop_interval_ms as f64 * ENGINE_REFRESH_MULTIPLIER) as i64;
        }
    }

    pub fn gauges(&self) -> &[EngineGauge] {
        &self.gauges
    }

    pub fn gauge_by_key(&self, key: &str) -> Option<&EngineGauge> {
        self.gauges.iter().find(|g| g.key == key)
    }

    pub fn is_jet(&self) -> bool {
        self.is_jet
    }

    /// updateGaugesPreview (EngineControlOverlay.java:588-606): val=maxValue/2,
    /// COMPRESSOR 显示 1 基档号, 标记示例 ratio 0.5, 全部可见
    pub fn update_preview(&mut self) {
        for g in &mut self.gauges {
            let val = g.max_value / 2;
            let is_compressor = g.gauge_type == GaugeType::Compressor;
            let display_text = (if is_compressor { val + 1 } else { val }).to_string();
            g.gauge.gauge.update(val, &display_text);
            g.visible = true;
            if let Some(mg) = g.marked_gauge.as_mut() {
                mg.update_display(val, &display_text);
                // 预览示例 optimal 标记
                mg.update_marker_ratio("optimal", 0.5);
            }
        }
    }

    /// onFlightData (EngineControlOverlay.java:371-381) 的单事件语义: 节流闩
    /// (间隔 refreshInterval, 配置驱动) → (invokeLater lambda 内) updateResult
    /// (:383-397) = updateStateFromPayload + updateGaugesZeroGC。
    /// compressor_stages = FMManager.current().compressorStages 的档位数快照
    /// (None = 句柄非 READY / 无增压器 → Java null)。
    /// PORT: updateResult 的 legacy Map<String,String> 分支 (:391-395 →
    /// updateGaugeByType/updateGaugesFromData :547-586) 弃译 — 生产不可达
    /// (Service 恒实现 TelemetrySource, telemetrySource != null 恒真)。
    /// PORT: System.currentTimeMillis 由调用方注入 now_ms (field2 先例); 返回
    /// false = 节流跳过 (Java 原方法 void, 宿主可据此省重绘)
    pub fn update(
        &mut self,
        now_ms: i64,
        s: &dyn FormulaView,
        payload: &EventPayload,
        compressor_stages: Option<i32>,
    ) -> bool {
        // Throttle updates
        if now_ms - self.last_refresh_time < self.refresh_interval {
            return false;
        }
        self.last_refresh_time = now_ms;
        self.update_state_from_payload(payload, compressor_stages);
        self.update_gauges_zero_gc(s);
        true
    }

    /// updateStateFromPayload (EngineControlOverlay.java:409-439)
    fn update_state_from_payload(
        &mut self,
        payload: &EventPayload,
        compressor_stages: Option<i32>,
    ) {
        // 引擎类型只判一次 (检测完成约 5 秒)
        if !self.jet_label_updated && payload.engine_check_done {
            self.is_jet = payload.is_jet;
            self.jet_label_updated = true;
            // 增压器量程写 FM 档位数 (一次性); Java controller!=null 恒真 (init/initPreview
            // 均传入), POC 无此判
            if !self.compressor_max_value_set {
                if let Some(stages) = compressor_stages {
                    if stages > 1 {
                        for g in &mut self.gauges {
                            if g.gauge_type == GaugeType::Compressor {
                                g.gauge.gauge.max_value = stages - 1;
                                if let Some(mg) = g.marked_gauge.as_mut() {
                                    mg.set_max_value((stages - 1) as f64);
                                }
                                break;
                            }
                        }
                    }
                }
                self.compressor_max_value_set = true;
            }
        }
        // optimal 档标记 (每帧更新)
        self.update_optimal_compressor_marker(payload, compressor_stages);
    }

    /// updateOptimalCompressorMarker (EngineControlOverlay.java:445-468)
    fn update_optimal_compressor_marker(
        &mut self,
        payload: &EventPayload,
        compressor_stages: Option<i32>,
    ) {
        let optimal_stage = payload.optimal_compressor_stage;
        for g in &mut self.gauges {
            if g.gauge_type != GaugeType::Compressor {
                continue;
            }
            // Java 循环条件: markedGauge!=null 才处理并 break; null 则继续扫描后续仪表
            let Some(mg) = g.marked_gauge.as_mut() else { continue };
            match compressor_stages {
                Some(stages) if optimal_stage >= 0 && stages > 1 => {
                    // 档 0 = ratio 0, 档 n-1 = ratio 1
                    let ratio = optimal_stage as f64 / (stages - 1) as f64;
                    mg.update_marker_ratio("optimal", ratio);
                }
                // 无有效数据时隐藏标记
                _ => mg.update_marker_ratio("optimal", -1.0),
            }
            break;
        }
    }

    /// updateGaugesZeroGC (EngineControlOverlay.java:470-545)
    fn update_gauges_zero_gc(&mut self, s: &dyn FormulaView) {
        for g in &mut self.gauges {
            // 隐藏字段短路; COMPRESSOR/MIXTURE/PITCH 持续评估 (数据可能回归)
            if !g.visible
                && g.gauge_type != GaugeType::Compressor
                && g.gauge_type != GaugeType::Mixture
                // PITCH 需持续评估: 无桨距机型(自动桨)与手动桨机型间切换时恢复显示
                && g.gauge_type != GaugeType::Pitch
            {
                continue;
            }
            // 喷气机隐藏仪表跳过
            if self.is_jet && is_jet_hidden_gauge(g.gauge_type) {
                continue;
            }
            let mut val;
            let mut has_val = true;
            match g.gauge_type {
                GaugeType::Throttle => {
                    val = s.var_value("throttle").unwrap_or(0.0);
                }
                GaugeType::Pitch => {
                    // 无桨距数据(自动桨机型, 归一化后为-1) → 整条隐藏, 后续竖条自动补位
                    val = s.var_value("rpm_throttle").unwrap_or(0.0);
                    g.visible = val >= 0.0;
                    if !g.visible {
                        has_val = false;
                    }
                }
                GaugeType::Power => {
                    val = s.var_value("power_percent").unwrap_or(0.0);
                }
                GaugeType::Mixture => {
                    val = s.var_value("mixture_state").unwrap_or(0.0);
                    g.visible = val >= 0.0;
                    if !g.visible {
                        has_val = false;
                    }
                }
                GaugeType::Radiator => {
                    val = s.var_value("radiator").unwrap_or(0.0);
                }
                GaugeType::Compressor => {
                    val = s.var_value("compressor_stage").unwrap_or(0.0);
                    let stage = val as i32;
                    g.visible = stage > 0;
                    if stage > 0 {
                        // 显示 1 基档号, 条用 0 基值
                        val = (stage - 1) as f64;
                    } else {
                        has_val = false;
                    }
                }
                GaugeType::Fuel => {
                    val = s.var_value("fuel_percent").unwrap_or(0.0);
                }
            }
            if has_val {
                // PORT: Java (int) val 截断向零; 值域 0..120, as i32 语义一致
                let int_val = val as i32;
                let text = if g.gauge_type == GaugeType::Compressor {
                    format::format((int_val + 1) as f64, 0)
                } else {
                    format::format(val, 0)
                };
                g.gauge.gauge.update(int_val, &text);
                if let Some(mg) = g.marked_gauge.as_mut() {
                    mg.update_buffer(int_val, &text);
                }
            }
        }
    }

    /// paintComponent → drawGauges (EngineControlOverlay.java:138-143/313-354):
    /// 起点 x=fontsize>>1, y=(fs*4)+((fs*6)>>1); 竖条画在 y-(4*fs), 横条画在 y+dy
    pub fn draw(&mut self, cv: &mut PixCanvas, font_label: &LoadedFont, aa: bool) {
        let fs = self.font_size;
        // paintComponent (EngineControlOverlay.java:143)
        let x = fs >> 1;
        let y = (fs * 4) + ((fs * 6) >> 1);
        let is_jet = self.is_jet;
        let mut dx = 0;
        let mut dy = fs >> 1;
        for g in &mut self.gauges {
            // 喷气机隐藏仪表跳过
            if is_jet && is_jet_hidden_gauge(g.gauge_type) {
                continue;
            }
            if !g.visible {
                continue;
            }
            // MarkedGauge 优先 (COMPRESSOR), 其余 LinearGauge
            if let Some(mg) = g.marked_gauge.as_mut() {
                if g.is_horizontal {
                    mg.draw(cv, x, y + dy, 4 * fs, fs >> 1, font_label, aa);
                    dy += fs + (fs >> 2);
                } else {
                    mg.draw(cv, x + dx, y - 4 * fs, 4 * fs, fs >> 1, font_label, aa);
                    dx += (5 * fs) >> 1;
                }
            } else {
                // Java 每帧原地赋 gauge.vertical = isHorizontal ? false : true
                g.gauge.gauge.vertical = !g.is_horizontal;
                if g.is_horizontal {
                    g.gauge.draw(cv, x, y + dy, 4 * fs, fs >> 1, font_label, aa);
                    dy += fs + (fs >> 2);
                } else {
                    // LinearGauge 逻辑自底向上改为自顶向下后, Y 需上移 (4*fontsize) 保持视觉位置
                    g.gauge.draw(cv, x + dx, y - 4 * fs, 4 * fs, fs >> 1, font_label, aa);
                    dx += (5 * fs) >> 1;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// live 喂数形态工厂 (minihud_overlay_spec 先例: render 闭包与喂入方共享句柄)
// ---------------------------------------------------------------------------

/// 引擎控制共享句柄
pub type EngineControlHandle = Rc<RefCell<EngineControlState>>;

/// 引擎控制 OverlaySpec + live 句柄 (Java Controller.java:654 注册键 enableEngineControl)。
/// `lang` 以 Rc 共享 (reinit 闭包重建 state 需要标签源; Lang !Clone)。
/// PORT(WYSIWYG): 字号/7 仪表 disable/轮询间隔随 [`ReinitParams`] 仓 — reinit
/// 闭包整体重建 EngineControlState + fontLabel (Java reinitConfig: loadFontConfig +
/// loadRefreshInterval + initGaugeFields + calculateLayout + updateGaugesPreview),
/// 返回新 (width, height) (Java setLocation 尺寸面)
pub fn engine_control_overlay_spec(
    fonts_dir: &std::path::Path,
    lang: Rc<Lang>,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(EngineControlHandle, OverlaySpec), String> {
    let (font_add, dpi_scale, interval_ms, disables) = {
        let p = params.borrow();
        (p.font_add_engine, p.dpi_scale, p.service_loop_interval_ms, p.engine_disables)
    };
    let interval_str = interval_ms.to_string();
    // init 链 (game 实例): initGaugeFields + calculateLayout + updateGaugesPreview
    // (半量程初值, 首个有效事件前的显示态; initPreview 的二次调用是 preview 专属)
    // cfg_true 按键名查 disables 表 (Java "true".equals(getConfigSafe(key));
    // 曾恒 false — 7 个 disable 开关从未生效, 启动首帧即与 Java 不一致)
    let state = build_engine_state(&lang, font_add, dpi_scale, &interval_str, &disables);
    // fontLabel = BOLD(round(fontSize/2.0f)) (loadFontConfig)
    let half = java_round_f32(state.font_size as f32 / 2.0);
    let bold_path = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    let font_label = Rc::new(RefCell::new(Rc::new(LoadedFont::new(&bold_path, half)?)));
    let (w, h) = (state.width, state.height);
    let handle: EngineControlHandle = Rc::new(RefCell::new(state));
    let render_handle = Rc::clone(&handle);
    let render_font = Rc::clone(&font_label);
    // reinit 闭包: 状态整体重建 (Java initGaugeFields 全量重排) + fontLabel 重载
    let reinit_handle = Rc::clone(&handle);
    let reinit_font = Rc::clone(&font_label);
    let reinit_lang = Rc::clone(&lang);
    let reinit_params = Rc::clone(params);
    let reinit_bold = bold_path;
    let reinit: ReinitFn = Box::new(move || {
        let (fa, dpi, iv, dis) = {
            let p = reinit_params.borrow();
            (p.font_add_engine, p.dpi_scale, p.service_loop_interval_ms, p.engine_disables)
        };
        let new_state = build_engine_state(&reinit_lang, fa, dpi, &iv.to_string(), &dis);
        let half = java_round_f32(new_state.font_size as f32 / 2.0);
        let new_font = match LoadedFont::new(&reinit_bold, half) {
            Ok(f) => Rc::new(f),
            Err(e) => {
                vm_core::base::logger::error("EngineControl", &format!("reinit 字体重载失败: {}", e));
                return None;
            }
        };
        let (w, h) = (new_state.width, new_state.height);
        *reinit_handle.borrow_mut() = new_state;
        *reinit_font.borrow_mut() = new_font;
        Some((w, h))
    });
    Ok((
        handle,
        OverlaySpec {
            id: "enableEngineControl".to_string(),
            config_key: "enableEngineControl".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                // aa = 运行时仓 (cfg AAEnable 可关 — 审查轮 1-A 第 7 处钉死点)
                render_handle.borrow_mut().draw(cv, &render_font.borrow(), aa());
            }),
            reinit: Some(reinit),
        },
    ))
}

/// EngineControlState::new 的 interval/disables 参数打包 (工厂初建与 reinit 共用)
fn build_engine_state(
    lang: &Lang,
    font_add: i32,
    dpi_scale: f64,
    interval_str: &str,
    disables: &[bool; 7],
) -> EngineControlState {
    EngineControlState::new(
        lang,
        font_add,
        dpi_scale,
        &|key: &str| ENGINE_DISABLE_KEYS
            .iter()
            .position(|k| *k == key)
            .map(|i| disables[i])
            .unwrap_or(false),
        &|_| interval_str.to_string(),
    )
}
