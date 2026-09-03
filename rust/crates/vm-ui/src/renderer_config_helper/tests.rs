use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// DynamicDataPage 匿名 RenderContext (DynamicDataPage.java:126-175) 的最小 mock:
/// 字符串值表 + 读/写调用记录 (get_*/sync_* 语义逐条复刻该实现)。
/// PORT: 唯一删略点 — 真实 syncToConfigService 的 enableFMPrint 特例
/// (DynamicDataPage.java:148-151 还会 publish FM_PRINT_SWITCH_CHANGED,
/// Rust 侧事件常量已译 event/ui_state_events.rs) 未复刻: 全库无该事件
/// 订阅者 (LIFETIMES 审查#10)。C 类落地真实 RenderContext 时**勿照本
/// mock 抄**, 该 publish 须按 Java 原样保留。
#[derive(Default)]
struct MockCtx {
    config: HashMap<String, String>,
    synced: RefCell<Vec<(String, String)>>,
    reads: RefCell<Vec<String>>,
}

impl MockCtx {
    fn new() -> Self {
        Self::default()
    }
    fn set(&mut self, key: &str, val: &str) {
        self.config.insert(key.to_string(), val.to_string());
    }
    fn synced(&self) -> Vec<(String, String)> {
        self.synced.borrow().clone()
    }
}

impl RenderContext for MockCtx {
    fn on_save(&self) {}
    fn sync_to_config_service(&self, key: &str, value: bool) {
        // DynamicDataPage: setConfig(key, Boolean.toString(value))
        self.synced
            .borrow_mut()
            .push((key.to_string(), value.to_string()));
    }
    fn get_from_config_service(&self, key: &str, default_val: bool) -> bool {
        self.reads.borrow_mut().push(key.to_string());
        // DynamicDataPage.java:155-161: getConfig; null/空 → 默认; Boolean.parseBoolean
        match self.config.get(key) {
            Some(v) if !v.is_empty() => v.eq_ignore_ascii_case("true"),
            _ => default_val,
        }
    }
    fn sync_string_to_config_service(&self, key: &str, value: &str) {
        self.synced
            .borrow_mut()
            .push((key.to_string(), value.to_string()));
    }
    fn get_string_from_config_service(&self, key: &str, default_val: &str) -> String {
        self.reads.borrow_mut().push(key.to_string());
        // DynamicDataPage.java:169-174: getConfig; null/空 → 默认
        match self.config.get(key) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => default_val.to_string(),
        }
    }
}

fn row_with(property: Option<&str>) -> RowConfig {
    let mut row = RowConfig::new("标签".to_string(), None, String::new());
    row.property = property.map(str::to_string);
    row
}

// ---- read_string: PropertyBinder > ConfigurationService > default ----

// 绑定命中: 字段值压制服务端同名键 (优先级 1 > 2)
#[test]
fn read_string_bound_field_wins_over_service() {
    let mut group = GroupConfig::new("飞行信息".to_string());
    group.font_name = Some("Sarasa Mono SC".to_string());
    let mut ctx = MockCtx::new();
    ctx.set("fontName", "Arial");
    let row = row_with(Some("fontName"));
    assert_eq!(read_string(&ctx, &group, &row, "D"), "Sarasa Mono SC");
    assert!(
        ctx.reads.borrow().is_empty(),
        "绑定命中不得查询 ConfigurationService"
    );
}

// 绑定命中但类型不符 (int 字段当 String 读): instanceof String 不中 → 默认值,
// 不落服务 (Java 分支语义: hasField 为真即短路在 PropertyBinder 层)
#[test]
fn read_string_bound_wrong_type_returns_default_not_service() {
    let group = GroupConfig::new("g".to_string()); // font_size = 0 (int 字段)
    let mut ctx = MockCtx::new();
    ctx.set("fontSize", "9");
    let row = row_with(Some("fontSize"));
    assert_eq!(read_string(&ctx, &group, &row, "D"), "D");
    assert!(ctx.reads.borrow().is_empty());
}

// Java 引用字段为 null (fontName 未设置): field.get → null → instanceof 不中 → 默认值
#[test]
fn read_string_null_field_returns_default() {
    let group = GroupConfig::new("g".to_string()); // font_name = None
    let ctx = MockCtx::new();
    let row = row_with(Some("fontName"));
    assert_eq!(read_string(&ctx, &group, &row, "D"), "D");
}

// 未绑定属性 (非 GroupConfig 字段) → 落 ConfigurationService
#[test]
fn read_string_unbound_falls_to_config_service() {
    let group = GroupConfig::new("g".to_string());
    let mut ctx = MockCtx::new();
    ctx.set("crosshairName", "软件渲染准星");
    let row = row_with(Some("crosshairName"));
    assert_eq!(read_string(&ctx, &group, &row, "D"), "软件渲染准星");
    assert_eq!(ctx.reads.borrow().as_slice(), ["crosshairName"]);
}

// 无属性 → 默认值
#[test]
fn read_string_no_property_returns_default() {
    let group = GroupConfig::new("g".to_string());
    let ctx = MockCtx::new();
    let row = row_with(None);
    assert_eq!(read_string(&ctx, &group, &row, "D"), "D");
    assert!(ctx.reads.borrow().is_empty());
}

// ---- read_int ----

// 绑定 int 字段直读
#[test]
fn read_int_bound_field() {
    let mut group = GroupConfig::new("g".to_string());
    group.font_size = 3;
    let ctx = MockCtx::new();
    let row = row_with(Some("fontSize"));
    assert_eq!(read_int(&ctx, &group, &row, 0), 3);
}

// 绑定 double 字段: (Number).intValue() = JLS 5.1.3 向零截断
#[test]
fn read_int_bound_double_field_truncates_toward_zero() {
    let mut group = GroupConfig::new("g".to_string());
    group.x = 7.9;
    group.y = -0.5;
    let ctx = MockCtx::new();
    assert_eq!(read_int(&ctx, &group, &row_with(Some("x")), 0), 7);
    assert_eq!(read_int(&ctx, &group, &row_with(Some("y")), 0), 0);
}

// 未绑定 → 服务端字符串解析
#[test]
fn read_int_unbound_parses_service_string() {
    let group = GroupConfig::new("g".to_string());
    let mut ctx = MockCtx::new();
    ctx.set("crosshairScale", "113");
    let row = row_with(Some("crosshairScale"));
    assert_eq!(read_int(&ctx, &group, &row, 0), 113);
}

// 未绑定 + 畸形串 → catch (Exception e) → 默认值; 键缺失 → 兜底串解析回默认值
#[test]
fn read_int_unbound_malformed_or_missing_returns_default() {
    let group = GroupConfig::new("g".to_string());
    let mut ctx = MockCtx::new();
    ctx.set("badKey", "abc");
    assert_eq!(read_int(&ctx, &group, &row_with(Some("badKey")), 7), 7);
    assert_eq!(read_int(&ctx, &group, &row_with(Some("absent")), 7), 7);
}

// ---- read_bool ----

// 绑定 boolean 字段直读 (visible), 压制服务端 "false"
#[test]
fn read_bool_bound_visible_field() {
    let mut group = GroupConfig::new("g".to_string());
    group.visible = true;
    let mut ctx = MockCtx::new();
    ctx.set("visible", "false");
    assert!(read_bool(&ctx, &group, &row_with(Some("visible")), false));
    assert!(ctx.reads.borrow().is_empty());
}

// 未绑定 → Boolean.parseBoolean(服务端串)
#[test]
fn read_bool_unbound_service_boolean() {
    let group = GroupConfig::new("g".to_string());
    let mut ctx = MockCtx::new();
    ctx.set("showSpeedBar", "true");
    assert!(read_bool(
        &ctx,
        &group,
        &row_with(Some("showSpeedBar")),
        false
    ));
    // parseBoolean 非 "true" 一律 false (含大小写不敏感匹配失败)
    ctx.set("weird", "TRUE "); // 尾随空格不等于 "true"
    assert!(!read_bool(&ctx, &group, &row_with(Some("weird")), false));
}

// 无属性 → 默认值
#[test]
fn read_bool_no_property_returns_default() {
    let group = GroupConfig::new("g".to_string());
    let ctx = MockCtx::new();
    assert!(!read_bool(&ctx, &group, &row_with(None), false));
    assert!(read_bool(&ctx, &group, &row_with(None), true));
}

// ---- write_*: 先试 PropertyBinder, 总是同步 ConfigurationService ----

#[test]
fn write_int_bound_mutates_group_and_syncs() {
    let mut group = GroupConfig::new("g".to_string());
    let ctx = MockCtx::new();
    assert!(write_int(&ctx, &mut group, Some("panelColumns"), 4));
    assert_eq!(group.panel_columns, 4);
    assert_eq!(
        ctx.synced(),
        vec![("panelColumns".to_string(), "4".to_string())]
    );
}

#[test]
fn write_string_bound_mutates_group_and_syncs() {
    let mut group = GroupConfig::new("g".to_string());
    let ctx = MockCtx::new();
    assert!(write_string(
        &ctx,
        &mut group,
        Some("fontName"),
        "DIN Pro 400"
    ));
    assert_eq!(group.font_name.as_deref(), Some("DIN Pro 400"));
    assert_eq!(
        ctx.synced(),
        vec![("fontName".to_string(), "DIN Pro 400".to_string())]
    );
}

#[test]
fn write_bool_bound_mutates_group_and_syncs() {
    let mut group = GroupConfig::new("g".to_string());
    let ctx = MockCtx::new();
    assert!(write_bool(&ctx, &mut group, Some("visible"), true));
    assert!(group.visible);
    assert_eq!(
        ctx.synced(),
        vec![("visible".to_string(), "true".to_string())]
    );
}

// 未注册属性: 绑定失败返回 false, 但仍同步服务端 ("总是同步" 注释)
#[test]
fn write_unknown_property_returns_false_but_still_syncs() {
    let mut group = GroupConfig::new("g".to_string());
    let ctx = MockCtx::new();
    assert!(!write_int(&ctx, &mut group, Some("crosshairScale"), 5));
    assert_eq!(group.panel_columns, 2, "组字段不受影响");
    assert_eq!(
        ctx.synced(),
        vec![("crosshairScale".to_string(), "5".to_string())]
    );
}

// property 为 null: set 返回 false 且跳过同步 (Java if (property != null) 守卫)
#[test]
fn write_none_property_no_sync_returns_false() {
    let mut group = GroupConfig::new("g".to_string());
    let ctx = MockCtx::new();
    assert!(!write_bool(&ctx, &mut group, None, true));
    assert!(!write_int(&ctx, &mut group, None, 1));
    assert!(!write_string(&ctx, &mut group, None, "x"));
    assert!(ctx.synced().is_empty());
}

// 类型不符: Java field.set 抛 IllegalArgumentException (未捕获上抛) — A5 修复:
// cfg 用户可编辑输入不 panic 主线程, 改忽略该次绑定 (false + 组字段不动 + 仍同步服务)
#[test]
fn write_type_mismatch_is_ignored_not_panic() {
    let mut group = GroupConfig::new("g".to_string());
    group.font_name = Some("旧".to_string());
    let ctx = MockCtx::new();
    assert!(!write_int(&ctx, &mut group, Some("fontName"), 1)); // String 字段 ← Integer
    assert_eq!(
        group.font_name.as_deref(),
        Some("旧"),
        "组字段不受越型绑定影响"
    );
    assert_eq!(
        ctx.synced(),
        vec![("fontName".to_string(), "1".to_string())]
    );
}

// Java 反射拓宽 (JLS 5.1.2): field.set(double 字段, Integer) 成功写入
// 5.0 (JDK8 oracle 实测) — slider 绑 x/y 是 cfg 可达路径, 非异常路径
#[test]
fn write_int_widens_into_double_field_like_java() {
    let mut group = GroupConfig::new("g".to_string());
    let ctx = MockCtx::new();
    assert!(write_int(&ctx, &mut group, Some("x"), 5));
    assert_eq!(group.x, 5.0);
    assert!(write_int(&ctx, &mut group, Some("y"), -3));
    assert_eq!(group.y, -3.0);
    assert_eq!(
        ctx.synced(),
        vec![
            ("x".to_string(), "5".to_string()),
            ("y".to_string(), "-3".to_string()),
        ]
    );
}

// Boolean 装入 double 字段: JDK8 oracle 实测仍抛 IllegalArgumentException
// (只拓宽数值包装类, Boolean 不参与) — A5 修复后同走忽略路径
#[test]
fn write_bool_into_double_field_is_ignored() {
    let mut group = GroupConfig::new("g".to_string());
    group.x = 1.5;
    let ctx = MockCtx::new();
    assert!(!write_bool(&ctx, &mut group, Some("x"), true)); // double 字段 ← Boolean
    assert_eq!(group.x, 1.5, "double 字段不接受 Boolean");
    assert_eq!(ctx.synced(), vec![("x".to_string(), "true".to_string())]);
}

// ---- 注册表完整性 (D7: 反射域 → 编译期 match 域) ----

// GroupConfig 全部 12 个 public 字段名命中; getField 精确匹配 (大小写敏感)
#[test]
fn has_field_registry_covers_all_java_public_fields() {
    let group = GroupConfig::new("g".to_string());
    for name in [
        "title",
        "x",
        "y",
        "alpha",
        "hotkey",
        "visible",
        "fontName",
        "fontSize",
        "columns",
        "panelColumns",
        "switchKey",
        "rows",
    ] {
        assert!(
            property_binder::has_field(&group, name),
            "{name} 应在注册表"
        );
    }
    assert!(!property_binder::has_field(&group, "FontSize"));
    assert!(!property_binder::has_field(&group, "fontsize"));
    assert!(!property_binder::has_field(&group, "crosshairScale"));
}

// trait 须可作 dyn 对象 (调用方以 &dyn RenderContext 解耦, 对应 Java 面向接口)
#[test]
fn render_context_object_safe_dyn() {
    let ctx: Box<dyn RenderContext> = Box::new(MockCtx::new());
    let group = GroupConfig::new("g".to_string());
    assert_eq!(read_string(ctx.as_ref(), &group, &row_with(None), "D"), "D");
}
