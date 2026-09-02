//! FmUnpackedDataOverlay (ui/overlay/FMUnpackedDataOverlay.java) — FM 调试列表。
//! 重构波2 自 overlays_field2.rs 拆出 (后半)。
//!
//! FM 调试列表: BaseOverlay 斑马纹基座 + blkx 字段直读清单 (D4 砍反射段后的
//! 等价实现)。UIStateBus 订阅 (FM_OVERLAY_TOGGLE/FM_CHANGED) 对应
//! [`FmUnpackedDataOverlay::toggle`]/[`FmUnpackedDataOverlay::reload_fm_data`],
//! 由组装层的事件循环驱动 (vm-app win32 线程: 总线订阅转 channel → 循环内消费);
//! dispose 的退订由所有权 Drop 根治 (LIFETIMES §2.3), 无需显式方法。
//!
//! P5 组装契约三点已销号 (原 "host::OverlaySpec 不可表达" 豁口):
//! (a) 动态窗口高 — host `resize_entry` 基建 + [`FmUnpackedFeed::pump`] 在
//! tick 后按 `base.height` 变化落 resize (对位 Java adjustPosition 的 setSize
//! 副作用); (b) 逐条目可见性 — host `set_entry_visible` (per-entry, 幂等) +
//! pump 每 tick 落 `base.window_visible`; (c) spec 工厂
//! [`fm_unpacked_data_overlay_spec`] (flight_info/field_overlays 先例形态)。
//!
//! 对拍备案 (审查 W3): rustcmp 套件覆盖 FlightInfo/gauges/MiniHUD; FMUnpackedData
//! (ZebraList 首个生产消费者) 的渲染证据 = 单测级 oracle 色/几何 (WebLaF 离屏
//! 实测值, overlay_list tests) + 本模块墨迹断言; rustcmp 场景面扩充随渲染对拍
//! 工具批另行安排。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::render::font::LoadedFont;
use crate::render::palette::aa;

use crate::platform::host::{OverlayHost, OverlaySpec, ReinitFn};
use crate::overlays::list::BaseListOverlay;
use crate::platform::reinit::ReinitParams;
use crate::render::canvas::PixCanvas;
use vm_core::fm::data::{FmData, FmParts};
use vm_core::config::config_api::ConfigProvider;
use vm_core::fm::FMManager;
use vm_core::base::physics_constants::g;
use vm_core::lang::Lang;

// ---------------------------------------------------------------------------
// Java String.format 最小面 (Lang 模板域: %s / %d / %.0f~%.3f / %%)
// ---------------------------------------------------------------------------

/// printf 实参 (FMUnpackedDataOverlay.generateLines 传入 Lang 模板的三类占位)
#[derive(Clone, Copy, Debug)]
pub(crate) enum FmtArg<'a> {
    /// %s — null 实参以 "null" 文本呈现 (Java Formatter 行为)
    S(&'a str),
    /// %d — 襟翼档位序号 (i32 十进制)
    D(i32),
    /// %.Nf — 精度由模板解析
    F(f64),
}

/// Java `String.format(template, args...)` 一比一 (Lang.bXXX 模板 + 实参)。
/// 支持域: `%s`/`%d`/`%.0f`~`%.9f`/`%%`; 其余转换符 Java 抛
/// UnknownFormatConversionException ↔ 此处 panic; `%d` 位点收浮点/字符串实参
/// Java 抛 IllegalFormatConversionException ↔ 此处同 panic (模板与实参由本模块
/// 成对提供, 用户改 lang 文件破坏配对时两语言同为崩溃语义)。
/// `%s` 位点收数值实参在 Java 合法 (toString 输出), 本实现防御 panic — 域内
/// 实参编译期成对不可达。
pub(crate) fn java_string_format(template: &str, args: &[FmtArg]) -> String {
    let mut out = String::new();
    let mut arg_i = 0usize;
    let bytes = template.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b != b'%' {
            // PORT: 模板为 ASCII 控制符 + CJK 文本, 非控制字节段整段透传
            // (按字节推进仅发生在 ASCII 控制符处, UTF-8 多字节序列不越界)
            let start = i;
            while i < bytes.len() && bytes[i] != b'%' {
                i += 1;
            }
            out.push_str(&template[start..i]);
            continue;
        }
        // '%' 分发
        let next = bytes.get(i + 1).copied();
        match next {
            Some(b'%') => {
                out.push('%'); // %% → 字面 %
                i += 2;
            }
            Some(b's') | Some(b'd') => {
                let arg = args.get(arg_i).unwrap_or_else(|| {
                    panic!("String.format 实参不足: {template:?} 第 {arg_i} 个占位")
                });
                arg_i += 1;
                match *arg {
                    FmtArg::S(s) => match next {
                        Some(b's') => out.push_str(s),
                        _ => panic!(
                            "String.format %d 收到字符串实参 (IllegalFormatConversionException): {template:?}"
                        ),
                    },
                    // Integer 的 %s/%d 位点 Java 均合法 (toString / 十进制)
                    FmtArg::D(v) => out.push_str(&v.to_string()),
                    FmtArg::F(_) => match next {
                        Some(b'd') => panic!(
                            "String.format %d 收到浮点实参 (IllegalFormatConversionException): {template:?}"
                        ),
                        // Java %s 收 Double 合法 (toString), 本实现防御 panic — 域内不可达
                        _ => panic!("模板 %s 位点收到数值实参 (域外防御): {template:?}"),
                    },
                }
                i += 2;
            }
            Some(b'.') => {
                // %.Nf
                let mut j = i + 2;
                let mut prec: u32 = 0;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    prec = prec * 10 + u32::from(bytes[j] - b'0');
                    j += 1;
                }
                if j >= bytes.len() || bytes[j] != b'f' {
                    panic!("String.format 未支持的转换符: {template:?} @ {i}");
                }
                // PORT: Java BigDecimal 任意精度合法, 本实现 u128 尾数累加上界 ≤9
                // (下方 as u8 截断与 10u128.pow 回绕均在此拦截); 超域仅模板漂移
                // 可达 → debug 断言, release 不引入 Java 没有的崩溃
                debug_assert!(prec <= 9, "String.format 精度超域 (.{prec}f > .9f): {template:?}");
                let arg = args.get(arg_i).unwrap_or_else(|| {
                    panic!("String.format 实参不足: {template:?} 第 {arg_i} 个占位")
                });
                arg_i += 1;
                match *arg {
                    FmtArg::F(v) => out.push_str(&java_format_f(v, prec as u8)),
                    FmtArg::S(_) | FmtArg::D(_) => {
                        panic!("模板 %.Nf 位点收到非数值实参: {template:?}")
                    }
                }
                i = j + 1;
            }
            _ => panic!("String.format 未支持的转换符: {template:?} @ {i}"),
        }
    }
    out
}

/// Java `String.format("%.{prec}f", d)` 一比一。
/// 语义模型 (vm-core flight_analyzer.rs java_format_f1 / config_loader.rs
/// java_format_f4 同源, Java 8 oracle 实证): 等价
/// `new BigDecimal(Double.toString(d)).setScale(prec, HALF_UP)` — 对**最短往返
/// 十进制表示**做 HALF_UP (5.25 → "5.3"), 而非精确二进制值展开; Rust `{:.N}`
/// 是对精确值的半偶舍入, 双重分歧 (2.675 → Java "2.68" vs Rust "2.67")。
/// NaN/Infinity 原样; 负号含 -0.0 (neg = is_sign_negative, Java Formatter 亦保留)。
/// 巨整数域 (exp10 > 25, double 间距 > 1 恒无有效小数): digits + 隐含尾零 + ".0"×prec。
pub(crate) fn java_format_f(d: f64, prec: u8) -> String {
    // 域界断言: prec≤9 时 u128 尾数 (整数部 ≤26 位 + 小数 9 位) 恒不溢出;
    // ≥39 时 10u128.pow 溢出 (Java BigDecimal 无此界, 属模板漂移信号)
    debug_assert!(prec <= 9, "java_format_f 精度超域: {prec}");
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = d.is_sign_negative(); // 含 -0.0 → "-0.0" (Java 亦然)
    let a = d.abs();
    let sci = format!("{:e}", a); // 最短往返表示 "D.DDDe±k"
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits = mant.replace('.', "");
    let digits = digits.as_bytes();
    let n = digits.len() as i32;

    let mut out = String::new();
    if exp10 > 25 {
        // 巨整数域: 全整数输出 + prec 位零小数 (域内 FM 数值不可达, 防御分支)
        out.push_str(&sci[..epos].replace('.', ""));
        out.push_str(&"0".repeat((exp10 - n + 1) as usize));
        if prec > 0 {
            out.push('.');
            out.push_str(&"0".repeat(prec as usize));
        }
    } else {
        // 最短表示的 i 号数字 (1-based, place = 10^(exp10-i+1)); 越界补 0
        let digit_at = |i: i32| -> u128 {
            if i < 1 {
                0
            } else {
                let idx = (i - 1) as usize;
                if idx < digits.len() {
                    u128::from(digits[idx] - b'0')
                } else {
                    0
                }
            }
        };
        // 保留到 10^-prec 位: i ≤ exp10 + 1 + prec; 判定位 = 其后一位 (HALF_UP:
        // ≥5 进位, 再后的剩余数字 < 1 单位不影响判定)
        let keep = exp10 + 1 + prec as i32;
        let mut scaled: u128 = 0; // = 整数 × 10^prec + 小数
        if keep > 0 {
            for i in 1..=keep {
                scaled = scaled * 10 + digit_at(i);
            }
        }
        if digit_at(keep + 1) >= 5 {
            scaled += 1; // HALF_UP (含精确 .5 进位; 进位可级联到整数部分)
        }
        let div = 10u128.pow(prec as u32);
        let int_part = scaled / div;
        let frac = scaled % div;
        out.push_str(&int_part.to_string());
        if prec > 0 {
            out.push('.');
            let s = frac.to_string();
            for _ in s.len()..prec as usize {
                out.push('0');
            }
            out.push_str(&s);
        }
    }
    if neg {
        out.insert(0, '-');
    }
    out
}

// ---------------------------------------------------------------------------
// FmUnpackedDataOverlay (ui/overlay/FMUnpackedDataOverlay.java)
// ---------------------------------------------------------------------------

/// FM 调试列表 overlay (FMUnpackedDataOverlay.java:32)。组合 BaseListOverlay
/// (Java extends BaseOverlay — §1 禁强行继承, 公共行为已上提基座):
/// 自管可见性 (游戏模式热键切换) + blkx 字段直读清单。
///
/// PORT: Java 经 FMDataAdapter 持 volatile blkx; vm-core 的 FMDataAdapter
/// 尚消费 BlkxPlaceholder (fm_data_adapter.rs TODO(port)), 本组件按任务裁决
/// 直读真实 `blkx::Blkx` (D4 model 字段面), 避免占位类型第二真相源。
/// set_blkx 的 volatile 赋值语义由"单写者(事件循环)+tick 前快照"承接。
pub struct FmUnpackedDataOverlay {
    /// BaseOverlay 基座 (run 循环状态机: 脏检查/高度自适应/可见门控)
    pub base: BaseListOverlay,
    /// Self-managed visibility state (game mode toggle) (Java :42)
    pub visible: bool,
    /// FMDataAdapter.getBlkx() 的等价持有 (None = 未加载 → 占位清单)
    fmdata: Option<Arc<FmData>>,
    /// Java :39 config = c.getConfigProvider() (None ↔ Java null 容忍)
    config: Option<Arc<dyn ConfigProvider>>,
}

impl FmUnpackedDataOverlay {
    /// 构造 + BaseOverlay.init 几何 (Java :46-48 super() 与 :90 super.init 的
    /// 几何段合一; 行高度量 setup_font 由 init/reinit 时调用方补)。
    pub fn new(logical_height: i32, dpi_scale: f64, default_fontsize: i32) -> Self {
        FmUnpackedDataOverlay {
            base: BaseListOverlay::new(logical_height, dpi_scale, default_fontsize),
            visible: true,
            fmdata: None,
            config: None,
        }
    }

    /// 游戏模式 init (Java :57-94): config 注入 + 隐藏起步 + 表头谓词 +
    /// BaseOverlay 数据供给挂接 (tick 内联)。UIStateBus 订阅 (toggle/FM_CHANGED)
    /// 归组装层事件循环, 对应 [`Self::toggle`]/[`Self::reload_fm_data`]。
    pub fn init(&mut self, config: Option<Arc<dyn ConfigProvider>>, font: &LoadedFont) {
        self.config = config;
        // Java :64 this.isPreview = false — 继承自 BaseOverlay 的单一字段,
        // 对应 base.is_preview (run 门控 :235 的唯一读取方)
        self.base.is_preview = false;

        // Game mode: initially hidden
        self.visible = false;
        self.base.setup_font(font);

        // Set header matcher for styling (FM parts headers start with "------fm器件")
        self.base.set_header_matcher(Box::new(|line| {
            line.starts_with("FM文件") || line.starts_with("------fm器件")
        }));
    }

    /// 预览模式 initPreview (Java :103-122): 恒可见 + 同表头谓词。
    pub fn init_preview(&mut self, config: Option<Arc<dyn ConfigProvider>>, font: &LoadedFont) {
        self.config = config;
        // Java :110 this.isPreview = true (BaseOverlay 单一字段, 同上)
        self.base.is_preview = true;

        // Preview mode: always visible
        self.visible = true;
        self.base.setup_font(font);

        self.base.set_header_matcher(Box::new(|line| {
            line.starts_with("FM文件") || line.starts_with("------fm器件")
        }));
    }

    /// FM_CHANGED handler 的 reloadFMData (Java :130-136): 句柄换 blkx 快照。
    /// 非 READY 句柄 blkx=null → None → 占位 "[No Data Loaded]" (null 容忍)。
    /// 数据刷新由下一 tick 周期完成 (Java 注释 :135)。
    /// PORT: Java :131 的 `payload instanceof FMHandle` 过滤由组装层承担 —
    /// P5 事件路由时非 FMHandle 载荷应保留旧 blkx 不调用本方法。
    pub fn reload_fm_data(&mut self, fmdata: Option<Arc<FmData>>) {
        self.fmdata = fmdata;
        // Data will be refreshed on next run() cycle
    }

    /// reinitConfig (Java :142-151): adapter 直读 FMManager.current() 换新
    /// (调用方传入 current().blkx; 非 READY 句柄 blkx 为 null → None,
    /// setBlkx(null) 清空 → 占位容忍) + setupFont。
    pub fn reinit_config(&mut self, current_fm: Option<Arc<FmData>>, font: &LoadedFont) {
        self.fmdata = current_fm;
        // Font and display settings are handled by BaseOverlay
        self.base.setup_font(font);
    }

    /// FM_OVERLAY_TOGGLE handler (Java :72-75): 翻转自管可见性。
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// CloseAll 后回 preview 会话形态 (Java closeAll = 实例销毁 + refreshPreview
    /// 工厂新建 initPreview 实例): visible=true / is_preview=true / lastData=null
    /// (预览窗回到当前 blkx 的清单态)。blkx/config 保留 — Java Controller 预览
    /// 工厂构造时同样 setBlkx(current), 等值; 表头谓词与 init_preview 设定相同
    /// 免重设。PORT(几何自纠, 审查 W2): base.height 保留 live 会话 adjustPosition
    /// 值不复位 — Java 工厂新实例回 init 高度, Rust 由首个 preview tick (pump 无
    /// 会话门控) 的 dirty → adjust_position 一步收敛到行数高度, 终态与 Java 一致。
    /// (ControlSurfaces::reset_preview / FlightInfo::reset_preview_rows 同族)
    pub fn reset_preview(&mut self) {
        self.visible = true;
        self.base.is_preview = true;
        self.base.clear_last_data();
    }

    /// isVisibleNow 覆写 (Java :318-321)
    pub fn is_visible_now(&self) -> bool {
        self.visible
    }

    /// BaseOverlay.run() 单轮 (经 overlay_list 基座): 先同步 isVisibleNow
    /// 门控位, 再以当前 blkx/config 快照生成行清单。
    pub fn tick(&mut self) -> bool {
        self.base.visible_now = self.visible;
        let fmdata = self.fmdata.clone();
        let config = self.config.clone();
        self.base
            .tick(move || Some(generate_lines(fmdata.as_deref(), config.as_deref())))
    }

    /// generateLines (Java :157-278) — 独立函数便于直测。
    pub fn generate_lines(&self) -> Vec<String> {
        generate_lines(self.fmdata.as_deref(), self.config.as_deref())
    }

    /// updateUI 渲染段委托基座 (BaseOverlay.java:263-269)
    pub fn render(&mut self, cv: &mut PixCanvas, font: &LoadedFont, aa: bool) {
        self.base.render(cv, font, aa);
    }
}

/// generateLines (Java :157-278): 按 ui_layout.cfg 开关过滤的 blkx 字段清单。
/// Lang 模板取 init_lang() 快照 (Java 读全局静态字段, 值同源 cur.properties)。
pub(crate) fn generate_lines(fmdata: Option<&FmData>, config: Option<&dyn ConfigProvider>) -> Vec<String> {
    let lang = Lang::init_lang();
    let mut lines: Vec<String> = Vec::new();
    let fmdata = match fmdata {
        None => {
            lines.push("FM Data Preview".to_string());
            lines.push("[No Data Loaded]".to_string());
            return lines;
        }
        Some(b) => b,
    };

    // ==================== FM Version (always shown) ====================
    // PORT: Java %s 收 null 字段打印 "null" (Formatter 行为), Option 展开对齐
    let fm_version = java_string_format(
        lang.b_fm_version,
        &[
            FmtArg::S(fmdata.read_file_name.as_deref().unwrap_or("null")),
            FmtArg::S(fmdata.version.as_deref().unwrap_or("null")),
        ],
    );
    add_lines(&mut lines, &fm_version);

    // ==================== Weight ====================
    if is_field_enabled(config, "showWeight") {
        let weight = java_string_format(
            lang.b_weight,
            &[FmtArg::F(fmdata.emptyweight), FmtArg::F(fmdata.maxfuelweight)],
        );
        add_lines(&mut lines, &weight);
    }

    // ==================== Critical Speed ====================
    if is_field_enabled(config, "showCritSpeed") {
        let crit_speed = java_string_format(
            lang.b_crit_speed,
            &[FmtArg::F(fmdata.critical_speed * 3.6), FmtArg::F(fmdata.vne)],
        );
        add_lines(&mut lines, &crit_speed);
    }

    // ==================== G-Load Limits (combined full/half fuel) ====================
    if is_field_enabled(config, "showGLoadLimits") {
        if let Some(raw) = fmdata.raw_wing_crit_overload {
            // PORT: 与 getMaxAllowGloadForWeight 同式内联 (Java 源如此, 不收敛去重)
            let full_neg = 1.2 * (2.0 * raw[0] / (g * fmdata.grossweight) + 1.0);
            let full_pos = 1.2 * (2.0 * raw[1] / (g * fmdata.grossweight) - 1.0);
            let half_neg = 1.2 * (2.0 * raw[0] / (g * fmdata.halfweight) + 1.0);
            let half_pos = 1.2 * (2.0 * raw[1] / (g * fmdata.halfweight) - 1.0);
            let load_factor = java_string_format(
                lang.b_allow_load_factor,
                &[FmtArg::F(full_neg), FmtArg::F(full_pos), FmtArg::F(half_neg), FmtArg::F(half_pos)],
            );
            add_lines(&mut lines, &load_factor);
        }
    }

    // ==================== Flap Speed Limits ====================
    // PORT: Java AIOOBE (num > 6) ↔ Rust 索引 panic 同构 (§1 崩溃语义)。
    // 线程模型差异: Java 仅杀死本 overlay 的 run 轮询线程, Rust tick/draw 在
    // 唯一主循环上 — P5 组装必须对逐 overlay tick/render 包 catch_unwind
    // (PORTING §6 先例), 本 panic 与 java_string_format 错配 panic 均属此契约
    if is_field_enabled(config, "showFlapLimits") {
        if let Some(table) = fmdata.flaps_destruction_ind_speed {
            for i in 0..fmdata.flaps_destruction_num {
                let flap_limit = java_string_format(
                    lang.b_flap_restrict,
                    &[
                        FmtArg::D(i),
                        FmtArg::F(table[i as usize][0] * 100.0),
                        FmtArg::F(table[i as usize][1]),
                    ],
                );
                add_lines(&mut lines, &flap_limit);
            }
        }
    }

    // ==================== Control Surface Effectiveness (combined) ====================
    if is_field_enabled(config, "showControlEffectiveness") {
        let eff_speed = java_string_format(
            lang.b_eff_speed_and_power_loss,
            &[
                FmtArg::F(fmdata.elav_eff),
                FmtArg::F(fmdata.aileron_eff),
                FmtArg::F(fmdata.rudder_eff),
                FmtArg::F(fmdata.elav_power_loss),
                FmtArg::F(fmdata.aileron_power_loss),
                FmtArg::F(fmdata.rudder_power_loss),
            ],
        );
        add_lines(&mut lines, &eff_speed);
    }

    // ==================== Nitro (only if present) ====================
    if is_field_enabled(config, "showNitro") && fmdata.nitro > 0.0 {
        let nitro = java_string_format(
            lang.b_nitro,
            &[FmtArg::F(fmdata.nitro), FmtArg::F(fmdata.nitro / (fmdata.nitro_decr * 60.0))],
        );
        add_lines(&mut lines, &nitro);
    }

    // ==================== Heat Recovery ====================
    if is_field_enabled(config, "showHeatRecovery") {
        let heat_recovery =
            java_string_format(lang.b_average_heat_recovery, &[FmtArg::F(fmdata.avg_eng_recovery_rate)]);
        add_lines(&mut lines, &heat_recovery);
    }

    // ==================== Max Lift Load ====================
    if is_field_enabled(config, "showMaxLiftLoad") {
        let max_lift_load = java_string_format(
            lang.b_max_lift_load350,
            &[
                FmtArg::F((fmdata.no_flap_wll + 1.0) / 2.0),
                FmtArg::F((fmdata.full_flap_wll + 1.0) / 2.0),
            ],
        );
        add_lines(&mut lines, &max_lift_load);
    }

    // ==================== Inertia ====================
    if is_field_enabled(config, "showInertia") {
        if let Some(m) = fmdata.moment_of_inertia {
            if m.len() >= 3 {
                let inertia = java_string_format(
                    lang.b_inertia,
                    &[FmtArg::F(m[2]), FmtArg::F(m[0]), FmtArg::F(m[1])],
                );
                add_lines(&mut lines, &inertia);
            }
        }
    }

    // ==================== Lift Parameters ====================
    if is_field_enabled(config, "showLift") {
        let lift = java_string_format(
            lang.b_lift,
            &[
                FmtArg::F(fmdata.a_wing),
                FmtArg::F(fmdata.a_fuselage),
                FmtArg::F(fmdata.no_flap_wll),
                FmtArg::F(fmdata.full_flap_wll),
                FmtArg::F(fmdata.oswalds_efficiency_number),
                FmtArg::F(fmdata.aspect_ratio),
                FmtArg::F(fmdata.swept_wing_angle),
            ],
        );
        add_lines(&mut lines, &lift);
    }

    // ==================== Drag Parameters ====================
    if is_field_enabled(config, "showDrag") {
        let drag = java_string_format(
            lang.b_drag,
            &[
                FmtArg::F(fmdata.cd_s),
                FmtArg::F(fmdata.cd_s / (fmdata.halfweight / 1000.0)),
                FmtArg::F(fmdata.ind_cd_f),
                FmtArg::F(fmdata.halfweight * fmdata.ind_cd_f),
                FmtArg::F(fmdata.radiator_cd),
                FmtArg::F(fmdata.oil_radiator_cd),
            ],
        );
        add_lines(&mut lines, &drag);
    }

    // ==================== FM Parts Sections ====================
    if is_field_enabled(config, "showNoFlapsWing") {
        add_fm_parts(&mut lines, &lang, fmdata.no_flaps_wing.as_ref());
    }
    if is_field_enabled(config, "showFullFlapsWing") {
        add_fm_parts(&mut lines, &lang, fmdata.full_flaps_wing.as_ref());
    }
    if is_field_enabled(config, "showFuselage") {
        add_fm_parts(&mut lines, &lang, fmdata.fuselage.as_ref());
    }
    if is_field_enabled(config, "showFin") {
        add_fm_parts(&mut lines, &lang, fmdata.fin.as_ref());
    }
    if is_field_enabled(config, "showStab") {
        add_fm_parts(&mut lines, &lang, fmdata.stab.as_ref());
    }

    // If no fields are enabled or all filtered out, show a placeholder
    // PORT: fmVersion 恒入列 (:169) 使本分支在 Java 亦不可达, 保真保留
    if lines.is_empty() {
        lines.push("FM Data Preview".to_string());
        lines.push("[No Fields Enabled]".to_string());
    }

    lines
}

/// addFmParts (Java :283-290): 表头 + 4 数据行 (null 部件整段跳过)。
fn add_fm_parts(lines: &mut Vec<String>, lang: &Lang, p: Option<&FmParts>) {
    let p = match p {
        None => return,
        Some(p) => p,
    };
    add_lines(
        lines,
        &java_string_format(lang.b_fm_parts, &[FmtArg::S(p.name.as_deref().unwrap_or("null"))]),
    );
    add_lines(lines, &java_string_format(lang.b_cd_min, &[FmtArg::F(p.cd_min)]));
    add_lines(lines, &java_string_format(lang.b_cl0, &[FmtArg::F(p.cl0)]));
    add_lines(
        lines,
        &java_string_format(lang.b_ao_a_crit, &[FmtArg::F(p.aoa_crit_low), FmtArg::F(p.aoa_crit_high)]),
    );
    add_lines(
        lines,
        &java_string_format(lang.b_ao_a_crit_cl, &[FmtArg::F(p.cl_crit_low), FmtArg::F(p.cl_crit_high)]),
    );
}

/// addLines (Java :296-303): 按 \n 拆行, 逐行 trim, 跳过空行。
/// PORT: Java String.trim 只剥 ≤ U+0020 的字符 (§2.1), Rust `str::trim` 会
/// 多剥 U+3000 等全角空白 — 用 trim_matches 精确复刻 Java 语义。
pub(crate) fn add_lines(lines: &mut Vec<String>, formatted: &str) {
    for line in formatted.split('\n') {
        let trimmed = line.trim_matches(|c: char| c <= '\u{20}');
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }
}

/// isFieldEnabled (Java :309-316): config 缺失/键空 → 默认启用;
/// 否则 Boolean.parseBoolean (仅忽略大小写的 "true" 为真)。
fn is_field_enabled(config: Option<&dyn ConfigProvider>, field_key: &str) -> bool {
    match config {
        None => true,
        Some(c) => match c.get_config(field_key) {
            None => true,
            Some(v) if v.is_empty() => true,
            Some(v) => v.eq_ignore_ascii_case("true"),
        },
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 挂载 (Java Controller.java:726-743 registerWithPreview("enableFMPrint"))
// ---------------------------------------------------------------------------

/// FM拆包数据共享句柄 (flight_info/control_surfaces 先例: render 闭包与
/// 事件循环共享 state; Rc 恒留 win32 线程)
pub type FmUnpackedDataHandle = Rc<RefCell<FmUnpackedDataOverlay>>;

/// FM拆包数据 OverlaySpec + live 句柄 (Java Controller.java:726 注册键
/// enableFMPrint, previewEnabled=true)。
///
/// - `logical_height` — Application.logicalHeight 快照 (Env.dpi 探测; init 几何的
///   scaleFactor 与 adjustPosition 的钳制上限来源, 屏幕常量不入 ReinitParams);
/// - `config` — generateLines 逐 tick 读的 show* 开关面 (Java 每轮直读
///   ConfigProvider; Rust 配置树 !Send, 组装层注入快照适配器, CONFIG_CHANGED
///   刷新 — ActivationCache 同款"最后写胜出"等价);
/// - `fm` — reinit 闭包的 blkx 直读源 (Java reinitConfig 的
///   `FMManager.getInstance().current()`)。
///
/// 初始态 = initPreview 形态 (恒可见 + 空数据: 注册期 = Java 无实例形态 —
/// LinkedHashMap 条目仅配置记录)。数据装载有两条面 (审查 B2-2 修正, 原注释
/// "Java 预览实例无 run 线程" 为假前提): (a) 预览实例化时 Controller 的
/// previewInitializer 先 setBlkx(current) 再 initPreview (Controller.java:734-737)
/// — Rust 对位 = refresh_preview 冷激活的 reinit 闭包直读 current; (b) run() 线程
/// **预览同样在跑** (needsThread=true, OverlayManager.refreshPreview :326-331 也
/// new Thread(instance).start()) — 每 200ms generateLines → 数据/高度自适应生效,
/// Rust 对位 = FmUnpackedFeed::pump 不做会话门控 (win32 循环调用点)。
/// 游戏形态 (is_preview=false + 隐藏起步) 由组装层在 OpenAllOverlays 处置位 —
/// 单实例形态下 ControlSurfaces has_service 的同款会话翻转模式。
///
/// PORT(尺寸): 初始 spec 尺寸 = init 几何的 width × defaultFontsize·72 (Java
/// Window 首帧尺寸; 高度随后被 adjustPosition 按行数接管 — FmUnpackedFeed)。
/// PORT(字体): Java setupFont 的 fontName (cfg "FM拆包数据" 组) 为 Swing 逻辑
/// 字体族名; Rust 字体面固定 sarasa regular 文件 (FontTriple/loadFontConfig 各
/// overlay 同款先例, cfg 缺省 "Sarasa Mono SC" 时零偏差), PLAIN 14+fontSizeAdd。
pub fn fm_unpacked_data_overlay_spec(
    fonts_dir: &std::path::Path,
    logical_height: i32,
    params: &Rc<RefCell<ReinitParams>>,
    config: Option<Arc<dyn ConfigProvider>>,
    fm: &Arc<FMManager>,
) -> Result<(FmUnpackedDataHandle, OverlaySpec), String> {
    let (font_add, dpi_scale) = {
        let p = params.borrow();
        (p.font_add_fm, p.dpi_scale)
    };
    // Application.defaultFontsize = 12 (Lang defaultFontSize, Application.java:93)
    let mut ov = FmUnpackedDataOverlay::new(logical_height, dpi_scale, 12);
    let regular_path = fonts_dir.join("sarasa-mono-sc-regular.ttf");
    let font = Rc::new(RefCell::new(Rc::new(LoadedFont::new(
        &regular_path,
        14 + font_add,
    )?)));
    ov.init_preview(config, &font.borrow());
    let (w, h) = (ov.base.width, ov.base.height);
    let handle: FmUnpackedDataHandle = Rc::new(RefCell::new(ov));
    let render_handle = Rc::clone(&handle);
    let render_font = Rc::clone(&font);
    // reinit 闭包 (Java reinitConfig :142-151): setBlkx(current) + setupFont。
    // PORT(返回 None): Java reinitConfig 无 setBounds — 高度由下次数据变更的
    // adjustPosition 接管 (行高随新字体变化, 数据 dirty 时自纠); 此处仅清指纹
    let reinit_handle = Rc::clone(&handle);
    let reinit_font = Rc::clone(&font);
    let reinit_params = Rc::clone(params);
    let reinit_fm = Arc::clone(fm);
    let reinit_regular = regular_path;
    let reinit: ReinitFn = Box::new(move || {
        let fa = reinit_params.borrow().font_add_fm;
        let new_font = match LoadedFont::new(&reinit_regular, 14 + fa) {
            Ok(f) => Rc::new(f),
            Err(e) => {
                vm_core::base::logger::error("FMUnpackedData", &format!("reinit 字体重载失败: {}", e));
                return None;
            }
        };
        // P3: 直读 FMManager 句柄 (blkx None → 清空 → 占位容忍)
        let fmdata = reinit_fm.current().fmdata.clone().map(Arc::new);
        reinit_handle.borrow_mut().reinit_config(fmdata, &new_font);
        *reinit_font.borrow_mut() = new_font;
        None
    });
    Ok((
        handle,
        OverlaySpec {
            id: "enableFMPrint".to_string(),
            config_key: "enableFMPrint".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                let font = render_font.borrow();
                render_handle.borrow_mut().render(cv, &font, aa());
            }),
            reinit: Some(reinit),
        },
    ))
}

/// FM拆包数据的组装面 tick 泵 — Java BaseOverlay.run() 线程循环 (while(doit)+
/// sleep(200)) 的单线程驱动侧: 200ms 节流 (getRefreshInterval, BaseOverlay.java:221)
/// → tick 单轮 (可见门控/取数/脏检查/高度自适应) → `base.window_visible` 落
/// per-entry set_visible → 高度变化落 resize_entry (adjustPosition 的 setSize
/// 副作用, 契约 (a)/(b) 接线)。
///
/// PORT(会话域, 审查 B2-2): Java needsThread=true — 游戏实例 (OverlayEntry.open
/// :303-309) 与**预览实例** (refreshPreview :326-331) 都起 run 线程, 两会话均
/// 200ms 轮询装载; 调用方 (win32 循环) 不做 preview 门控, 仅条目未激活
/// (host 槽位空 = Java 无实例) 时跳过。
///
/// PORT(panic 边界): tick 内 generateLines 的保真 panic 点 (flap AIOOBE /
/// java_string_format 错配, 见 generate_lines 的 PORT 注) 由本泵 catch_unwind
/// 兜住 (PORTING §6 先例 — 不杀 host 泵); panic 后置 doit=false = Java "异常
/// 杀死本 overlay 的 run 线程" 的冻结形态对位 (后续 tick 短路, 应用存活)。
pub struct FmUnpackedFeed {
    /// 节流基准 (Java run 循环的 sleep(200) 节拍; 0 = 首轮放行)
    last_ms: i64,
}

impl Default for FmUnpackedFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl FmUnpackedFeed {
    pub fn new() -> Self {
        FmUnpackedFeed { last_ms: 0 }
    }

    /// 单轮驱动。`id` = host 注册键 ("enableFMPrint"), `now_ms` 由调用方注入
    /// (System.currentTimeMillis, 测试可假时钟)。
    pub fn pump(
        &mut self,
        host: &mut OverlayHost,
        id: &str,
        handle: &FmUnpackedDataHandle,
        now_ms: i64,
    ) {
        // getRefreshInterval() = 200ms (BaseOverlay.java:221-223, 本组件未覆写;
        // 读 base 字段 = 单一真相源)
        let interval_ms = handle.borrow().base.refresh_interval_ms as i64;
        if now_ms.saturating_sub(self.last_ms) < interval_ms {
            return;
        }
        self.last_ms = now_ms;
        let before = {
            let fm = handle.borrow();
            (fm.base.width, fm.base.height)
        };
        let ticked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle.borrow_mut().tick();
        }));
        if ticked.is_err() {
            vm_core::base::logger::error(
                "FMUnpackedData",
                "run 轮 panic 已吞 (畸形 FM 字段, 对位 Java 杀 run 线程), 本 overlay 冻结",
            );
            handle.borrow_mut().base.stop(); // doit=false: 后续 tick 短路
            return;
        }
        let fm = handle.borrow();
        // run() 双分支的 setVisible 落地 (:245-249; set_entry_visible 幂等)
        host.set_entry_visible(id, fm.base.window_visible);
        // adjustPosition 的 setSize 副作用: 高度 (或宽) 变化才落 resize
        // (未变时避免清指纹引发无谓 present — Java 亦仅在变化时 setSize)
        let after = (fm.base.width, fm.base.height);
        if after != before {
            let _ = host.resize_entry(id, after.0, after.1);
        }
    }
}
