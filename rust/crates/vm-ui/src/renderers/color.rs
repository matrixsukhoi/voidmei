//! ColorRowRenderer 的 iced 语义复刻 (src/ui/layout/renderer/ColorRowRenderer.java)
//! + ColorHelper 的解析/格式化移植 (src/prog/util/ColorHelper.java, vm-core 未译,
//! 就近落地本文件供 color_picker 共用)。
//!
//! ColorHelper 语义 (边缘行为经 Java 8 oracle 对拍, 2026-08-26, 用例值见 tests):
//! - parse_color: hex (#RRGGBB / #RRGGBBAA) 与十进制 ("R, G, B[, A]") 双格式,
//!   失败回落默认色 (cfg :value 为 hex, 用户编辑后存十进制 — 双格式互通的原因)。
//! - to_decimal_string: 配置存储格式 (向后兼容); to_hex_string: 显示格式 (大写)。
//!
//! 交互语义保真 (Java L30-136):
//! - 读: ctx.getStringFromConfigService(key, "255, 255, 255, 255") — 直读服务,
//!   无 PropertyBinder 分支 (ColorRowRenderer.java:34)。
//! - 写 (apply): 主键存十进制 (Java L124) + legacy 分键 keyR/G/B/A (Java L127-130,
//!   全库无读取方, 保真写入) + row.value=十进制串 (L133) + onSave (L135)。
//!
//! PORT(提交时机分歧): Java hex 输入 Enter/失焦提交 (L55-63); iced 无失焦消息且
//! 枚举冻结 → on_input 仅在解析出合法完整色串时发 ColorPicked (部分输入静默);
//! on_submit 补 Enter 面 (提交 current = Java 非法输入回落 initialColor 的提交,
//! L46-51)。失焦提交与逐键编辑的 draft 态留接线批。色块 (swatch) 点击弹层
//! (ColorPickerPopup) 需接线层持有打开状态 (color_picker::PickerState), 本批色块
//! 为纯展示。

use iced::widget::{container, text, text_input, Container, Row, Space};
use iced::{Border, Color, Element, Length};
use vm_core::config_loader::{ConfigValue, GroupConfig, RowConfig};
use vm_core::row_renderer_registry::RenderContext;

use super::{find_row_path, row_by_path, row_by_path_mut};
use crate::main_form::Message;

/// Java Color.WHITE (ColorRowRenderer.java:35 解析回落的默认白)
pub const WHITE: [u8; 4] = [255, 255, 255, 255];

/// Java ColorHelper.isHexFormat (L160-162): trim 后以 '#' 开头。
/// PORT(dead_code): ColorHelper 全量移植的 API 面 (Java 侧同样无生产调用方)。
#[allow(dead_code)]
pub fn is_hex_format(text: &str) -> bool {
    java_trim(text).starts_with('#')
}

/// Java String.trim(): 去两端码点 <= U+0020 的字符。
/// PORT: Rust str::trim 是 Unicode 空白集, 对 nbsp/U+3000 会多删 (oracle nbsp-hex
/// 用例: Java 不删 → 十进制路径解析失败 → 默认色), 必须按 Java 语义实现。
/// crate 内共享: button.rs 的 :fgcolor 逐段 trim 同为 Java String.trim 语义。
pub(crate) fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= '\u{0020}')
}

/// Java replaceAll("\\s+") 的字符集 [ \t\n\x0B\f\r]。
/// PORT: Rust is_ascii_whitespace 不含 \x0B, 显式对齐 (oracle vt-internal 用例)
fn is_java_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r')
}

/// Java ColorHelper.parseColor (L41-55): 双格式解析, 失败回落 default。
pub fn parse_color(text: &str, default: [u8; 4]) -> [u8; 4] {
    try_parse_color(text).unwrap_or(default)
}

/// 解析的 Option 形态 (hex 输入框的"合法完整色串"门控用, 见 view_row)。
pub fn try_parse_color(text: &str) -> Option<[u8; 4]> {
    let trimmed = java_trim(text);
    if trimmed.is_empty() {
        return None; // Java L42-44: null/trim 后空 → default
    }
    if trimmed.starts_with('#') {
        parse_hex_color(trimmed)
    } else {
        parse_decimal_color(trimmed)
    }
}

/// Java parseHexColor (L64-86): "#RRGGBB" (alpha=255) / "#RRGGBBAA";
/// 其他长度/非法数字落穿 → default。
fn parse_hex_color(hex: &str) -> Option<[u8; 4]> {
    let h = &hex[1..]; // '#' 恒 ASCII 单字节, [1..] 是合法切点
    // 非 ASCII 多字节字符在 Java parseInt 必失败 → default (顺带规避切包边界 panic)
    if !h.is_ascii() {
        return None;
    }
    let b = h.as_bytes();
    let byte = |r: std::ops::Range<usize>| u8::from_str_radix(&h[r], 16).ok();
    match b.len() {
        // Java: new Color(r, g, b) — 2 位十六进制恒 0-255, 无需钳位
        6 => Some([byte(0..2)?, byte(2..4)?, byte(4..6)?, 255]),
        8 => Some([byte(0..2)?, byte(2..4)?, byte(4..6)?, byte(6..8)?]),
        _ => None,
    }
}

/// Java parseDecimalColor (L95-118): "R, G, B[, A]", 全空白剔除 + 钳位 [0,255]。
fn parse_decimal_color(decimal: &str) -> Option<[u8; 4]> {
    // Java L97: replaceAll("\\s+", "") — 内部空白一并剔除
    let cleaned: String = decimal.chars().filter(|c| !is_java_ws(*c)).collect();
    // Java String.split(","): 尾部空串丢弃 (oracle "255, 85, 0," → 3 段 → a=255;
    // Rust split 原样保留 → 需模拟, 否则尾部逗号串解析失败偏离 Java)
    let mut parts: Vec<&str> = cleaned.split(',').collect();
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    if parts.len() < 3 {
        return None;
    }
    // Java parseInt: 十进制, 非法串抛异常 → default
    let r = parts[0].parse::<i32>().ok()?;
    let g = parts[1].parse::<i32>().ok()?;
    let b = parts[2].parse::<i32>().ok()?;
    let a = if parts.len() >= 4 { parts[3].parse::<i32>().ok()? } else { 255 };
    Some([clamp_u8(r), clamp_u8(g), clamp_u8(b), clamp_u8(a)])
}

/// Java clamp (L167-169)
fn clamp_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// Java toDecimalString (L126-131): 存储格式 "R, G, B, A"。
/// PORT: Java null 分支 ("255, 255, 255, 255") 在 Rust 类型下不可达。
pub fn to_decimal_string(c: &[u8; 4]) -> String {
    format!("{}, {}, {}, {}", c[0], c[1], c[2], c[3])
}

/// Java toHexString (L140-152): 显示格式, 大写十六进制 ("%02X")。
pub fn to_hex_string(c: &[u8; 4], include_alpha: bool) -> String {
    if include_alpha {
        format!("#{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])
    } else {
        format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
    }
}

/// 读链 (Java L33-35): 服务串 (cfg 内 hex / 编辑后十进制) 双格式解析,
/// 缺省 "255, 255, 255, 255"。
pub fn read_current(row: &RowConfig, ctx: &dyn RenderContext) -> [u8; 4] {
    match row.property.as_deref() {
        Some(key) => parse_color(&ctx.get_string_from_config_service(key, "255, 255, 255, 255"), WHITE),
        // Java key=null → getConfig 对 null key NPE (域内不可达: COLOR 行恒带
        // :target), 折叠为行值解析 — 与 switch.rs 的同类折叠一致
        None => parse_color(&row.get_str(), WHITE),
    }
}

/// 颜色变更写回 (Java applyColorChange L110-136)。经 main_form::update 的
/// ColorPicked 臂接线 (with_panel 模式, 与 switch/slider/combo 同构)。
pub fn apply(panel: &mut GroupConfig, key: &str, rgba: [u8; 4], ctx: &dyn RenderContext) {
    let Some(path) = find_row_path(&panel.rows, key) else {
        return;
    };
    let prop = row_by_path(&panel.rows, &path)
        .expect("find_row_path 已定位")
        .property
        .clone();
    let unified = to_decimal_string(&rgba);
    if let Some(p) = prop.as_deref() {
        // Java L124: 主键十进制存储 (向后兼容)
        ctx.sync_string_to_config_service(p, &unified);
        // Java L127-130: legacy 分键 (拆通道写; 全库无读取方, 纯兼容面, 保真写入)
        ctx.sync_string_to_config_service(&format!("{p}R"), &rgba[0].to_string());
        ctx.sync_string_to_config_service(&format!("{p}G"), &rgba[1].to_string());
        ctx.sync_string_to_config_service(&format!("{p}B"), &rgba[2].to_string());
        ctx.sync_string_to_config_service(&format!("{p}A"), &rgba[3].to_string());
    }
    // Java L133: row.value = unified (内存模型)
    row_by_path_mut(&mut panel.rows, &path)
        .expect("find_row_path 已定位")
        .value = Some(ConfigValue::Str(unified));
    // Java L135: onSave
    ctx.on_save();
}

/// 颜色行视图: [label | hex 输入框 | 色块] (Java createColorField 布局)。
pub fn view_row<'a>(
    row: &'a RowConfig,
    ctx: &dyn RenderContext,
    panel_title: &'a str,
) -> Element<'a, Message> {
    let current = read_current(row, ctx);
    let hex_display = to_hex_string(&current, true); // Java L38: hex 显示格式
    // 消息键: :target 优先; 无 :target 以 label 为键 (Java null-key NPE 域折叠)
    let key = row
        .property
        .clone()
        .or_else(|| (!row.label.is_empty()).then(|| row.label.clone()));
    let field: Element<'a, Message> = match key {
        Some(key) => {
            // Java L46-51 updateFromColor: Enter 提交 parseColor(text, initialColor) —
            // 非法输入亦以 initialColor 走完整 apply (写配置+onSave+回填文本)。
            // 无 draft 态下字段恒显 canonical: 合法输入已被 on_input 逐键提交
            // (Enter 时 current=已解析值), 非法输入视图已回弹 → current=旧值,
            // Enter 提交 current 即 Java 的 initialColor 语义
            let submit = Message::ColorPicked {
                panel: panel_title.to_string(),
                key: key.clone(),
                value: current,
            };
            text_input("", &hex_display)
                // 合法完整色串才提交 (Java Enter/失焦提交的逐键近似, 见模块文档)
                .on_input(move |s| match try_parse_color(&s) {
                    Some(c) => Message::ColorPicked {
                        panel: panel_title.to_string(),
                        key: key.clone(),
                        value: c,
                    },
                    None => Message::Ignore,
                })
                .on_submit(submit)
                .into()
        }
        None => text_input("", &hex_display).into(), // 无 on_input → 禁用态
    };
    Row::with_children(vec![
        text(row.label.clone()).width(Length::Fill).into(),
        field.into(),
        swatch(current).into(),
    ])
    .spacing(8)
    .into()
}

/// 色块 (Java getColorSwatch): 背景色 + 灰边框方块。
/// 纯展示 — Java 点击打开弹层, 弹层状态归接线层 (color_picker.rs)。
fn swatch(c: [u8; 4]) -> Container<'static, Message> {
    container(Space::new(Length::Fixed(20.0), Length::Fixed(16.0)))
        .style(move |_| container::Style {
            background: Some(Color::from_rgba8(c[0], c[1], c[2], c[3] as f32 / 255.0).into()),
            border: Border {
                color: Color::from_rgba8(128, 128, 128, 1.0),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
}

// =====================================================================
// Tests — ColorHelper 边缘语义全部取自 Java 8 oracle 对拍 (默认色 1,2,3,4)
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::test_util::MapCtx;

    const DEF: [u8; 4] = [1, 2, 3, 4];

    fn color_row(prop: Option<&str>, value: Option<&str>) -> RowConfig {
        let mut r = RowConfig::new("颜色".into(), None, "%s".into());
        r.r#type = "COLOR".into();
        r.property = prop.map(str::to_string);
        r.value = value.map(|v| ConfigValue::Str(v.to_string()));
        r
    }

    // ---- parse_color: hex 域 (oracle: #FF5500/#ff5500aa/#FFGG00/#FFF/#/#FF5500A) ----
    #[test]
    fn parse_hex_formats() {
        assert_eq!(try_parse_color("#FF5500"), Some([255, 85, 0, 255]));
        assert_eq!(try_parse_color("#ff5500aa"), Some([255, 85, 0, 170])); // 小写
        assert_eq!(parse_color(" #FF5500 ", DEF), [255, 85, 0, 255]); // trim
        // oracle: 非法数字/长度 → 默认
        assert_eq!(parse_color("#FFGG00", DEF), DEF);
        assert_eq!(parse_color("#FFF", DEF), DEF);
        assert_eq!(parse_color("#", DEF), DEF);
        assert_eq!(parse_color("#FF5500A", DEF), DEF); // 7 位
        assert_eq!(parse_color("", DEF), DEF);
        assert_eq!(parse_color("   ", DEF), DEF);
        // 多字节 UTF-8 (Java parseInt 失败 + 防切包 panic)
        assert_eq!(parse_color("#FF中文", DEF), DEF);
    }

    // ---- parse_color: 十进制域 (oracle: 钳位/尾逗号/非法串/分号) ----
    #[test]
    fn parse_decimal_formats() {
        assert_eq!(try_parse_color("255, 85, 0, 170"), Some([255, 85, 0, 170]));
        assert_eq!(try_parse_color("255,85,0"), Some([255, 85, 0, 255])); // 3 段 → a=255
        assert_eq!(try_parse_color(" 255 , 85 , 0 , 170 "), Some([255, 85, 0, 170]));
        // oracle: 尾逗号 → Java split 丢弃尾部空串 → 3 段
        assert_eq!(try_parse_color("255, 85, 0,"), Some([255, 85, 0, 255]));
        // oracle: 越界钳位 [0,255]
        assert_eq!(try_parse_color("300, -5, 0, 999"), Some([255, 0, 0, 255]));
        // oracle: 非法串 → 默认
        assert_eq!(parse_color("1,,3", DEF), DEF);
        assert_eq!(parse_color("a,b,c", DEF), DEF);
        assert_eq!(parse_color("256,10,10", DEF), [255, 10, 10, 255]); // 钳位不回默认
        assert_eq!(parse_color("255;0;0", DEF), DEF); // 分号非分隔符
        assert_eq!(parse_color("0xFF,0,0", DEF), DEF); // 十进制域不认 0x
        assert_eq!(parse_color("1.5,2,3", DEF), DEF);
        assert_eq!(parse_color("-FF,0,0", DEF), DEF);
    }

    // ---- 空白/trim 的 Java 语义 (oracle: nbsp/vt/全角空格) ----
    #[test]
    fn java_trim_and_whitespace_semantics() {
        // nbsp 前缀: Java trim 不去 (>0x20) → 十进制路径解析失败 → 默认
        assert_eq!(parse_color("\u{00A0}#FF5500", DEF), DEF);
        // VT 内部: Java \s 含 \x0B → 剔除后解析成功
        assert_eq!(try_parse_color("255,\u{000B}85,\u{000B}0"), Some([255, 85, 0, 255]));
        // 控制符 (<=0x20) 前后缀: trim 剔除 → hex 路径成功
        assert_eq!(try_parse_color("\u{001F}#FF5500\u{0001}"), Some([255, 85, 0, 255]));
        // nbsp 内部: \s 不含 nbsp → parseInt 失败 → 默认
        assert_eq!(parse_color("255,\u{00A0}85, 0", DEF), DEF);
        // 全角空格: 同 nbsp → 默认
        assert_eq!(parse_color("\u{3000}255, 0, 0", DEF), DEF);
        // is_hex_format: trim 后 '#' 开头 (Java isHexFormat(" #abc") = true)
        assert!(is_hex_format(" #abc"));
        assert!(!is_hex_format("abc"));
    }

    // ---- 格式化 + 双格式往返 ----
    #[test]
    fn format_and_round_trip() {
        let c = [255, 85, 0, 170];
        assert_eq!(to_decimal_string(&c), "255, 85, 0, 170"); // oracle 格式
        assert_eq!(to_hex_string(&c, true), "#FF5500AA");
        assert_eq!(to_hex_string(&c, false), "#FF5500");
        // 存储串 ↔ 显示串 双向解析恒等 (双格式互通的根据)
        for c in [[0, 0, 0, 0], [255, 255, 255, 255], [232, 147, 50, 128], [1, 2, 3, 4]] {
            assert_eq!(parse_color(&to_decimal_string(&c), DEF), c);
            assert_eq!(parse_color(&to_hex_string(&c, true), DEF), c);
            assert_eq!(parse_color(&to_hex_string(&c, false), DEF), [c[0], c[1], c[2], 255]);
        }
    }

    // ---- 读链: 服务值 (hex/十进制) 双格式; 缺省白 (Java L34-35) ----
    #[test]
    fn read_current_service_and_default() {
        let row = color_row(Some("fontNum"), Some("#FF0000FF"));
        let mut ctx = MapCtx::default();
        ctx.set("fontNum", "#FF5500AA"); // cfg :value hex 形态
        assert_eq!(read_current(&row, &ctx), [255, 85, 0, 170]);
        ctx.set("fontNum", "255, 85, 0, 170"); // 用户编辑后十进制形态
        assert_eq!(read_current(&row, &ctx), [255, 85, 0, 170]);
        // 服务缺省 → "255, 255, 255, 255" → 白
        let ctx2 = MapCtx::default();
        assert_eq!(read_current(&row, &ctx2), WHITE);
        // 无 :target 折叠行值
        let row2 = color_row(None, Some("#00FF0080"));
        assert_eq!(read_current(&row2, &MapCtx::default()), [0, 255, 0, 128]);
    }

    // ---- 写链: 主键十进制 + 分键 R/G/B/A + row.value + on_save (Java L110-136) ----
    #[test]
    fn apply_writes_decimal_split_keys_and_saves() {
        let mut panel = GroupConfig::new("p".into());
        panel.rows.push(color_row(Some("fontWarn"), Some("#FF2400FF")));
        let ctx = MapCtx::default();

        apply(&mut panel, "fontWarn", [255, 36, 0, 128], &ctx);
        assert_eq!(
            panel.rows[0].value,
            Some(ConfigValue::Str("255, 36, 0, 128".into()))
        );
        assert_eq!(
            *ctx.calls.borrow(),
            vec![
                "syncStr:fontWarn=255, 36, 0, 128".to_string(),
                "syncStr:fontWarnR=255".to_string(),
                "syncStr:fontWarnG=36".to_string(),
                "syncStr:fontWarnB=0".to_string(),
                "syncStr:fontWarnA=128".to_string(),
                "on_save".to_string(),
            ]
        );
    }

    // 无 :target (label 键): 不触服务同步, 仍写 row.value + on_save (Java null-key
    // 域的折叠 — Java NPE, Rust 保守降级)
    #[test]
    fn apply_label_key_skips_service_sync() {
        let mut panel = GroupConfig::new("p".into());
        panel.rows.push(color_row(None, Some("#FFFFFFFF")));
        let ctx = MapCtx::default();

        apply(&mut panel, "颜色", [0, 0, 0, 255], &ctx);
        assert_eq!(panel.rows[0].value, Some(ConfigValue::Str("0, 0, 0, 255".into())));
        assert_eq!(*ctx.calls.borrow(), vec!["on_save".to_string()]);
    }

    // 未命中 key: 无副作用 (消息域外防护)
    #[test]
    fn apply_unknown_key_is_noop() {
        let mut panel = GroupConfig::new("p".into());
        panel.rows.push(color_row(Some("k"), Some("#FFFFFFFF")));
        let ctx = MapCtx::default();
        apply(&mut panel, "absent", [0, 0, 0, 0], &ctx);
        assert_eq!(panel.rows[0].value, Some(ConfigValue::Str("#FFFFFFFF".into())));
        assert!(ctx.calls.borrow().is_empty());
    }

    // ---- 真实服务链: apply 经 WriteContext → set_config 更新服务树行值 (typed) ----
    #[test]
    fn apply_real_service_chain_updates_row_value() {
        use crate::main_form::WriteContext;
        use std::sync::Arc;
        use vm_core::bus::EventBus;
        use vm_core::config_api::ConfigProvider;
        use vm_core::configuration_service::ConfigurationService;

        let p = std::env::temp_dir().join("vm_ui_color_svc.cfg");
        std::fs::write(
            &p,
            r##"(panel "p" (item "告警色" :type color :target "fontWarn" :value "#FF2400FF"))"##,
        )
        .unwrap();
        let bus = Arc::new(EventBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&bus)));
        config.load_layout(p.to_str().unwrap());

        let mut panel = config.get_layout_configs().unwrap().remove(0);
        let ctx = WriteContext::new(&config, &bus);
        apply(&mut panel, "fontWarn", [255, 36, 0, 128], &ctx);
        // 服务树: 行值 → 十进制串 (Java setConfig: Str 行存串)
        assert_eq!(config.get_config("fontWarn").unwrap(), "255, 36, 0, 128");
        let _ = std::fs::remove_file(&p);
    }

    // ---- 视图构建冒烟 (无 panic 即结构成立) ----
    #[test]
    fn view_row_builds() {
        let row = color_row(Some("fontNum"), Some("#FFFFFFFF"));
        let ctx = MapCtx::default();
        let _el = view_row(&row, &ctx, "面板");
        let row2 = color_row(None, Some("#FFFFFFFF"));
        let _el2 = view_row(&row2, &ctx, "面板");
    }
}
