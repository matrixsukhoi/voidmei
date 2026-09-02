//! 对应 Java: `src/ui/layout/renderer/RendererConfigHelper.java` (包 ui.layout.renderer)。
//!
//! 配置渲染器的统一读写助手
//! 消除 SliderRowRenderer、ComboRowRenderer、SwitchRowRenderer 中的重复配置读写代码
//!
//! 读取优先级：
//! 1. PropertyBinder（GroupConfig 中的字段绑定）
//! 2. ConfigurationService（全局配置服务）
//! 3. 默认值（来自 row.value）
//!
//! 写入逻辑：
//! 1. 尝试 PropertyBinder 写入
//! 2. 同步到 ConfigurationService（用于 overlay 控制键）
//!
//! PORT: Java 类仅含 static 方法 → Rust 模块自由函数 (exception_helper/string_helper 先例)。
//! PORT: 形参类型 `RowRenderer.RenderContext` (RowRenderer.java:27-52 嵌套接口) 的
//! trait 定义落位于 [`crate::row_renderer_registry`] (RowRenderer.java 的占位翻译,
//! 嵌套归属保真), 本文件 re-export 引入 — 对应 Java 侧 `import
//! ui.layout.renderer.RowRenderer.RenderContext` (RendererConfigHelper.java:6)。
//! 全 crate 必须共用该单一类型: render() 收到的 `&dyn RenderContext` 要能直接
//! 传入本助手六函数 (两个同名 trait = 名义类型不兼容, 审查 blocker)。
//! 注意与 `crate::layout::RenderContext` (ui/renderer/RenderContext.java, 布局公式
//! 上下文) 是**同名不同物**的两个 Java 类型。

use vm_core::config_loader::{GroupConfig, RowConfig};

/// Context object providing callbacks and state for rendering.
///
/// PORT: 定义在 [`crate::row_renderer_registry::RenderContext`] (Java 为
/// `ui.layout.renderer.RowRenderer.RenderContext` 嵌套接口, 唯一实现是
/// DynamicDataPage.java:126-175 的匿名类), 此处 re-export 使本助手六方法的
/// 契约类型与 `RowRenderer::render` 的形参类型归一 (对应 Java 侧 import 语句)。
/// PORT: 方法签名取 `&self` (非 `&mut`): Java 实现的 syncToConfigService →
/// ConfigurationService.setConfig → **同步** publish CONFIG_CHANGED →
/// DynamicDataPage 自身的 handler rebuild() 重入 (§2.8) — `&mut` 版在
/// RefCell 实现下即借用 panic; 共享与重入安全由实现侧内部可变性承担
/// (与 config_api::config_provider::ConfigProvider::set_config 的同款裁决)。
pub use crate::row_renderer_registry::RenderContext;

/// PORT (D7 重设计): Java `prog.util.PropertyBinder` 是通用反射工具 —
/// `target.getClass().getField(property)` 按名读/写任意对象的 public 字段。
/// DECISIONS.md D7 弃译清单: "PropertyBinder → C 类重设计为编译期 match 注册表"。
/// 全库对该类的唯一消费点即 RendererConfigHelper 六方法 (绑定 GroupConfig 字段),
/// 故收窄为 GroupConfig 专属注册表落位于此:
/// - 反射 `getField(name)` → [`resolve`] 对 12 个 public 字段名 match (键 = Java
///   字段名原样, cfg 的 `:target` 直达, 如 "fontSize"/"panelColumns"/"fontName");
/// - `field.get()` + `instanceof` → [`get`] 返回 [`FieldValue`] + 类型化 getter
///   的 match 臂 (instanceof 不中 = 默认值);
/// - `field.set()` 的类型检查 → [`set`] 的 match 臂 (不匹配 = Java 未受检
///   IllegalArgumentException 上抛 → panic!, PORTING §1)。
///
/// Utility class for reflection-based property binding.
/// Allows dynamic get/set of object fields by name, eliminating switch-case
/// boilerplate.
mod property_binder {
    use vm_core::config_loader::GroupConfig;

    /// 注册表键 → GroupConfig 字段选择子 (Java: getField 返回的 Field 对象)。
    /// 覆盖 GroupConfig 全部 12 个 public 字段 (ConfigLoader.java:80-97);
    /// GroupConfig 无父类字段, 故命中集合封闭。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum GroupField {
        Title,        // String  title
        X,            // double  x
        Y,            // double  y
        Alpha,        // int     alpha
        Hotkey,       // int     hotkey
        Visible,      // boolean visible
        FontName,     // String  fontName
        FontSize,     // int     fontSize
        Columns,      // int     columns
        PanelColumns, // int     panelColumns
        SwitchKey,    // String  switchKey
        Rows,         // List<RowConfig> rows
    }

    impl GroupField {
        /// Java `Field.getType().getSimpleName()` 等价物 (仅用于异常文案)
        fn kind_name(self) -> &'static str {
            match self {
                GroupField::Title | GroupField::FontName | GroupField::SwitchKey => "String",
                GroupField::X | GroupField::Y => "double",
                GroupField::Alpha
                | GroupField::Hotkey
                | GroupField::FontSize
                | GroupField::Columns
                | GroupField::PanelColumns => "int",
                GroupField::Visible => "boolean",
                GroupField::Rows => "List",
            }
        }
    }

    /// Java: `PropertyBinder.get()` 返回的 Object — GroupConfig 字段值的类型化镜像。
    /// 五种实际形态 + Null; `List<RowConfig>` 仅以 Rows 标记占位 (三个类型化
    /// getter 均按 instanceof 不中处理)。
    /// PORT: Java 引用字段可为 null (fieldName 未设置时 field.get 返回 null,
    /// instanceof String 不中 → 默认值) — Rust 侧 GroupConfig 的 Option<String>
    /// 映射为 Null 变体, 语义一致。
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum FieldValue<'a> {
        Null,
        Str(&'a str),
        Double(f64),
        Int(i32),
        Bool(bool),
        Rows,
    }

    /// Java: `PropertyBinder.set()` 的 Object value 形参 — 三个类型化 setter
    /// 的装箱值域 (Integer/Boolean/String)。
    #[derive(Debug, Clone, PartialEq)]
    enum BindingValue {
        Int(i32),
        Bool(bool),
        Str(String),
    }

    impl BindingValue {
        /// 装箱类名 (仅用于异常文案)
        fn kind_name(&self) -> &'static str {
            match self {
                BindingValue::Int(_) => "java.lang.Integer",
                BindingValue::Bool(_) => "java.lang.Boolean",
                BindingValue::Str(_) => "java.lang.String",
            }
        }
    }

    /// 注册表本体: 键 = Java 反射字段名 (精确匹配, getField 大小写敏感)。
    /// Java: NoSuchFieldException → 上层 catch → false/null。
    fn resolve(property: &str) -> Option<GroupField> {
        match property {
            "title" => Some(GroupField::Title),
            "x" => Some(GroupField::X),
            "y" => Some(GroupField::Y),
            "alpha" => Some(GroupField::Alpha),
            "hotkey" => Some(GroupField::Hotkey),
            "visible" => Some(GroupField::Visible),
            "fontName" => Some(GroupField::FontName),
            "fontSize" => Some(GroupField::FontSize),
            "columns" => Some(GroupField::Columns),
            "panelColumns" => Some(GroupField::PanelColumns),
            "switchKey" => Some(GroupField::SwitchKey),
            "rows" => Some(GroupField::Rows),
            _ => None,
        }
    }

    /// Checks if a public field exists on the target object.
    ///
    /// Java: `public static boolean hasField(Object target, String property)`
    /// (target == null / property == null 守卫由调用方 RendererConfigHelper 的
    /// 分支结构承担 — Rust 侧 target 为非空引用、property 为 &str)。
    pub(super) fn has_field(_group: &GroupConfig, property: &str) -> bool {
        resolve(property).is_some()
    }

    /// Gets a field value from an object by field name.
    ///
    /// Java: `public static Object get(Object target, String property)`
    ///
    /// @param target   The object to read from
    /// @param property The field name
    /// @return The field value, or null if not found
    fn get<'a>(group: &'a GroupConfig, property: &str) -> Option<FieldValue<'a>> {
        Some(match resolve(property)? {
            GroupField::Title => FieldValue::Str(group.title.as_str()),
            GroupField::X => FieldValue::Double(group.x),
            GroupField::Y => FieldValue::Double(group.y),
            GroupField::Alpha => FieldValue::Int(group.alpha),
            GroupField::Hotkey => FieldValue::Int(group.hotkey),
            GroupField::Visible => FieldValue::Bool(group.visible),
            GroupField::FontName => match group.font_name.as_deref() {
                Some(s) => FieldValue::Str(s),
                None => FieldValue::Null,
            },
            GroupField::FontSize => FieldValue::Int(group.font_size),
            GroupField::Columns => FieldValue::Int(group.columns),
            GroupField::PanelColumns => FieldValue::Int(group.panel_columns),
            GroupField::SwitchKey => match group.switch_key.as_deref() {
                Some(s) => FieldValue::Str(s),
                None => FieldValue::Null,
            },
            GroupField::Rows => FieldValue::Rows,
        })
    }

    /// Gets a field value as int.
    ///
    /// Java: `public static int getInt(Object target, String property, int defaultValue)`
    pub(super) fn get_int(group: &GroupConfig, property: &str, default_value: i32) -> i32 {
        match get(group, property) {
            Some(FieldValue::Int(i)) => i,
            // PORT: Java double→int (JLS 5.1.3: NaN→0, 越界饱和, 向零截断) 与
            // Rust `as i32` 逐位一致 (§2.2 的 long→int 双转陷阱不适用于浮点)
            Some(FieldValue::Double(d)) => d as i32,
            // Str/Bool/Rows/Null/未命中: instanceof Number 不中 → 默认值
            _ => default_value,
        }
    }

    /// Gets a field value as String.
    ///
    /// Java: `public static String getString(Object target, String property, String defaultValue)`
    pub(super) fn get_string(group: &GroupConfig, property: &str, default_value: &str) -> String {
        match get(group, property) {
            Some(FieldValue::Str(s)) => s.to_string(),
            _ => default_value.to_string(),
        }
    }

    /// Gets a field value as boolean.
    ///
    /// Java: `public static boolean getBool(Object target, String property, boolean defaultValue)`
    pub(super) fn get_bool(group: &GroupConfig, property: &str, default_value: bool) -> bool {
        match get(group, property) {
            Some(FieldValue::Bool(b)) => b,
            _ => default_value,
        }
    }

    /// Sets a field value on an object by field name.
    ///
    /// Java: `public static boolean set(Object target, String property, Object value)`
    ///
    /// @param target   The object to modify
    /// @param property The field name
    /// @param value    The value to set
    /// @return true if successful, false otherwise
    fn set(group: &mut GroupConfig, property: Option<&str>, value: BindingValue) -> bool {
        let Some(property) = property else {
            return false;
        };
        let field = match resolve(property) {
            Some(f) => f,
            None => return false,
        };
        match (field, value) {
            (GroupField::Title, BindingValue::Str(s)) => {
                group.title = s;
                true
            }
            // PORT: Java String 字段可空; 反射 set 装入非 null 值 → Option::Some
            (GroupField::FontName, BindingValue::Str(s)) => {
                group.font_name = Some(s);
                true
            }
            (GroupField::SwitchKey, BindingValue::Str(s)) => {
                group.switch_key = Some(s);
                true
            }
            (GroupField::Alpha, BindingValue::Int(i)) => {
                group.alpha = i;
                true
            }
            (GroupField::Hotkey, BindingValue::Int(i)) => {
                group.hotkey = i;
                true
            }
            (GroupField::FontSize, BindingValue::Int(i)) => {
                group.font_size = i;
                true
            }
            (GroupField::Columns, BindingValue::Int(i)) => {
                group.columns = i;
                true
            }
            (GroupField::PanelColumns, BindingValue::Int(i)) => {
                group.panel_columns = i;
                true
            }
            (GroupField::Visible, BindingValue::Bool(b)) => {
                group.visible = b;
                true
            }
            // PORT: Java 反射 Field.set 对 double 字段装 Integer 走拆箱+拓宽
            // (int→double, JLS 5.1.2) 成功写入 — JDK8 oracle 实测
            // (x.set(g, Integer.valueOf(5)) → x=5.0)。get_int 对 double 字段的
            // (Number).intValue() 截断是其读侧对偶。i32 as f64 精确无损。
            (GroupField::X, BindingValue::Int(i)) => {
                group.x = i as f64;
                true
            }
            (GroupField::Y, BindingValue::Int(i)) => {
                group.y = i as f64;
                true
            }
            // PropertyBinder 仅捕 NoSuchFieldException|IllegalAccessException →
            // 原样上抛), §1 映射 panic!。剩余组合 (Boolean/String 装入数值或
            // boolean 字段, Integer 装入 boolean/String/List 字段) 均 = JDK8
            // oracle 实测的 IllegalArgumentException 路径。域内不可达论据:
            // 现行 ui_layout.cfg 的 :target 仅绑 fontSize(int)/fontName(String),
            // 无越型绑定 — 但 slider 绑 x/y (double) 属 cfg 可达路径, 已由上方
            // 拓宽臂承接, 不落入此处
            (field, value) => panic!(
                "java.lang.IllegalArgumentException: Can not set {} field '{}' to {} \
                 (PropertyBinder.set 未捕获, 原样上抛)",
                field.kind_name(),
                property,
                value.kind_name()
            ),
        }
    }

    /// Sets an int field value.
    ///
    /// Java: `public static boolean setInt(Object target, String property, int value)`
    pub(super) fn set_int(group: &mut GroupConfig, property: Option<&str>, value: i32) -> bool {
        set(group, property, BindingValue::Int(value))
    }

    /// Sets a String field value.
    ///
    /// Java: `public static boolean setString(Object target, String property, String value)`
    ///
    /// PORT: Java setString(prop, null) 对 String 字段成功写 null (引用字段
    /// 接受 null, JDK8 oracle 实测 fontName→null); Rust `&str` 无法表达 null —
    /// 唯一调用方 ComboRowRenderer.java:60 域内恒非 null, 不可达。
    pub(super) fn set_string(group: &mut GroupConfig, property: Option<&str>, value: &str) -> bool {
        set(group, property, BindingValue::Str(value.to_string()))
    }

    /// Sets a boolean field value.
    ///
    /// Java: `public static boolean setBool(Object target, String property, boolean value)`
    pub(super) fn set_bool(group: &mut GroupConfig, property: Option<&str>, value: bool) -> bool {
        set(group, property, BindingValue::Bool(value))
    }
}

/// Java `Integer.parseInt(String)` (radix 10) 复刻:
/// 可选 +/-, 至少一位数字, 溢出/空/非法 → Err (= NumberFormatException)。
/// PORT: config_loader.rs 已有同实现但为私有, 本文件禁改他文件 (PORTING §6
/// 只标注上报) → 暂存本地副本, 后续统一上提时二选一。
fn java_parse_int(s: &str) -> Result<i32, ()> {
    let b = s.as_bytes();
    let (neg, digits) = match b.first() {
        Some(b'-') => (true, &b[1..]),
        Some(b'+') => (false, &b[1..]),
        _ => (false, b),
    };
    if digits.is_empty() {
        return Err(());
    }
    let mut acc: i64 = 0;
    for &d in digits {
        if !d.is_ascii_digit() {
            return Err(());
        }
        acc = acc * 10 + i64::from(d - b'0');
        if acc > i32::MAX as i64 + 1 {
            return Err(()); // 溢出 — Java 抛 NumberFormatException
        }
    }
    if neg {
        acc = -acc;
    }
    if !(i32::MIN as i64..=i32::MAX as i64).contains(&acc) {
        return Err(());
    }
    Ok(acc as i32)
}

/// 读取字符串配置值
///
/// @param ctx        渲染上下文
/// @param groupConfig 组配置
/// @param row        行配置
/// @param defaultVal 默认值
/// @return 配置值
///
/// PORT: Java defaultVal 为可空 String; 唯一调用方 ComboRowRenderer 传
/// row.getStr() (String.valueOf, 恒非 null) → Rust 收窄为 &str, 行为域内等价。
pub fn read_string(
    ctx: &dyn RenderContext,
    group_config: &GroupConfig,
    row: &RowConfig,
    default_val: &str,
) -> String {
    // PORT: Java 两段 if (row.property != null && hasField) / else if (property != null)
    // 的短路与分支顺序原样保持 — 绑定命中(即使类型不符)后不再落 ConfigurationService
    if let Some(property) = row.property.as_deref() {
        if property_binder::has_field(group_config, property) {
            return property_binder::get_string(group_config, property, default_val);
        }
        return ctx.get_string_from_config_service(property, default_val);
    }
    default_val.to_string()
}

/// 读取整数配置值
///
/// @param ctx        渲染上下文
/// @param groupConfig 组配置
/// @param row        行配置
/// @param defaultVal 默认值
/// @return 配置值
pub fn read_int(
    ctx: &dyn RenderContext,
    group_config: &GroupConfig,
    row: &RowConfig,
    default_val: i32,
) -> i32 {
    if let Some(property) = row.property.as_deref() {
        if property_binder::has_field(group_config, property) {
            return property_binder::get_int(group_config, property, default_val);
        }
        let val = ctx.get_string_from_config_service(property, &default_val.to_string());
        // §2.15: catch 吞异常给默认值 → unwrap_or
        return java_parse_int(&val).unwrap_or(default_val);
    }
    default_val
}

/// 读取布尔配置值
///
/// @param ctx        渲染上下文
/// @param groupConfig 组配置
/// @param row        行配置
/// @param defaultVal 默认值
/// @return 配置值
pub fn read_bool(
    ctx: &dyn RenderContext,
    group_config: &GroupConfig,
    row: &RowConfig,
    default_val: bool,
) -> bool {
    if let Some(property) = row.property.as_deref() {
        if property_binder::has_field(group_config, property) {
            return property_binder::get_bool(group_config, property, default_val);
        }
        return ctx.get_from_config_service(property, default_val);
    }
    default_val
}

/// 写入字符串配置值
///
/// @param ctx        渲染上下文
/// @param groupConfig 组配置
/// @param property   属性名
/// @param value      新值
/// @return 是否成功写入 PropertyBinder
///
/// PORT(跨文件上报, C 类渲染器批次前必须裁决): 兄弟文件 row_renderer_registry.rs
/// 的 `RowRenderer::render` 占位签名给 `&GroupConfig`, 而本助手 write_* 需
/// `&mut GroupConfig` (Java 三个渲染器均在 render 内经本助手原地改 groupConfig,
/// PropertyBinder.set 反射写入同一活对象)。需在 C 类批次前显式抉择其一并记
/// DECISIONS.md: render 形参提为 `&mut GroupConfig` / 改传 `&RefCell<GroupConfig>`
/// / 经 ctx 通道改值。本文件不越文件改动 (PORTING §6)。
pub fn write_string(
    ctx: &dyn RenderContext,
    group_config: &mut GroupConfig,
    property: Option<&str>,
    value: &str,
) -> bool {
    let bound_success = property_binder::set_string(group_config, property, value);
    // 总是同步到 ConfigurationService
    if let Some(property) = property {
        ctx.sync_string_to_config_service(property, value);
    }
    bound_success
}

/// 写入整数配置值
///
/// @param ctx        渲染上下文
/// @param groupConfig 组配置
/// @param property   属性名
/// @param value      新值
/// @return 是否成功写入 PropertyBinder
pub fn write_int(
    ctx: &dyn RenderContext,
    group_config: &mut GroupConfig,
    property: Option<&str>,
    value: i32,
) -> bool {
    let bound_success = property_binder::set_int(group_config, property, value);
    // 总是同步到 ConfigurationService
    if let Some(property) = property {
        ctx.sync_string_to_config_service(property, &value.to_string());
    }
    bound_success
}

/// 写入布尔配置值
///
/// @param ctx        渲染上下文
/// @param groupConfig 组配置
/// @param property   属性名
/// @param value      新值
/// @return 是否成功写入 PropertyBinder
pub fn write_bool(
    ctx: &dyn RenderContext,
    group_config: &mut GroupConfig,
    property: Option<&str>,
    value: bool,
) -> bool {
    let bound_success = property_binder::set_bool(group_config, property, value);
    // 总是同步到 ConfigurationService
    if let Some(property) = property {
        ctx.sync_to_config_service(property, value);
    }
    bound_success
}

#[cfg(test)]
mod tests;
