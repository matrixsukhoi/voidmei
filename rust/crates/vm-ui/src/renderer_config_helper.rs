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
//! PORT: 形参类型 `RowRenderer.RenderContext` (RowRenderer 嵌套接口) 的
//! trait 定义落位于 [`crate::render_context`] (D9 后 RowRenderer 策略接口已退役,
//! 契约独立成文件), 本文件 re-export 引入 — 对应 Java 侧 import 语句。
//! 全 crate 必须共用该单一类型: 各 apply 写链与读写助手收到的 `&dyn RenderContext`
//! 要能直接互通 (两个同名 trait = 名义类型不兼容, 审查 blocker)。
//! 注意与 vm-overlay `render::renderers::RenderContext` (布局公式上下文)
//! 是**同名不同物**的两个 Java 类型。

use vm_core::base::java_compat::java_parse_int;
use vm_core::config::config_loader::{GroupConfig, RowConfig};

/// Context object providing callbacks and state for rendering.
///
/// PORT: 定义在 [`crate::render_context::RenderContext`] (Java 为
/// `ui.layout.renderer.RowRenderer.RenderContext` 嵌套接口, 唯一实现是
/// DynamicDataPage 的匿名类), 此处 re-export 使本助手六方法与
/// 各渲染器 apply 写链的契约类型归一 (对应 Java 侧 import 语句)。
/// PORT: 方法签名取 `&self` (非 `&mut`): Java 实现的 syncToConfigService →
/// ConfigurationService.setConfig → **同步** publish CONFIG_CHANGED →
/// DynamicDataPage 自身的 handler rebuild() 重入 (§2.8) — `&mut` 版在
/// RefCell 实现下即借用 panic; 共享与重入安全由实现侧内部可变性承担
/// (与 config_api::config_provider::ConfigProvider::set_config 的同款裁决)。
pub use crate::render_context::RenderContext;

/// F7 字段表 (GroupConfig 值字段单一真相): 每项 = (枚举变体, cfg 键名, rust 字段名,
/// 字段类型)。类型即 kind — 读写形态差异由 property_binder 的 GroupFieldAccess
/// 按类型抹平。消费点: 本文件 property_binder (GroupField 12 域, 追加 rows) 与
/// main_form.rs (PanelField — 直接消费 11 个值字段)。
macro_rules! group_field_table {
    ($m:ident) => {
        $m! {
            (Title, "title", title, String),
            (X, "x", x, f64),
            (Y, "y", y, f64),
            (Alpha, "alpha", alpha, i32),
            (Hotkey, "hotkey", hotkey, i32),
            (Visible, "visible", visible, bool),
            (FontName, "fontName", font_name, Option<String>),
            (FontSize, "fontSize", font_size, i32),
            (Columns, "columns", columns, i32),
            (PanelColumns, "panelColumns", panel_columns, i32),
            (SwitchKey, "switchKey", switch_key, Option<String>),
        }
    };
}
pub(crate) use group_field_table;

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
    use vm_core::base::logger;
    use vm_core::config::config_loader::{GroupConfig, RowConfig};

    /// GroupConfig 字段值的类型抹平面 (F7): 字段的 Rust 类型决定读写形态 —
    /// KIND_NAME 对位 Java Field.getType().getSimpleName() (仅用于异常文案);
    /// to_field_value 对位 field.get() 的类型化镜像; from_binding_value 对位
    /// field.set() 的类型检查 (不匹配 = Java 反射的 IllegalArgumentException
    /// 路径 → None, 由调用方 warn + 忽略)。
    trait GroupFieldAccess: Sized {
        const KIND_NAME: &'static str;
        fn to_field_value(&self) -> FieldValue<'_>;
        fn from_binding_value(value: BindingValue) -> Option<Self>;
    }

    impl GroupFieldAccess for String {
        const KIND_NAME: &'static str = "String";
        fn to_field_value(&self) -> FieldValue<'_> {
            FieldValue::Str(self.as_str())
        }
        fn from_binding_value(value: BindingValue) -> Option<Self> {
            match value {
                BindingValue::Str(s) => Some(s),
                _ => None,
            }
        }
    }

    /// PORT: Java 引用字段可为 null (fontName 未设置时 field.get 返回 null,
    /// instanceof String 不中 → 默认值) — Option 映射为 Null 变体, 语义一致;
    /// 反射 set 装入非 null 值 → Option::Some。
    impl GroupFieldAccess for Option<String> {
        const KIND_NAME: &'static str = "String";
        fn to_field_value(&self) -> FieldValue<'_> {
            match self.as_deref() {
                Some(s) => FieldValue::Str(s),
                None => FieldValue::Null,
            }
        }
        fn from_binding_value(value: BindingValue) -> Option<Self> {
            match value {
                BindingValue::Str(s) => Some(Some(s)),
                _ => None,
            }
        }
    }

    /// PORT: Java 反射 Field.set 对 double 字段装 Integer 走拆箱+拓宽
    /// (int→double, JLS 5.1.2) 成功写入 — JDK8 oracle 实测
    /// (x.set(g, Integer.valueOf(5)) → x=5.0)。get_int 对 double 字段的
    /// (Number).intValue() 截断是其读侧对偶。i32 as f64 精确无损。
    impl GroupFieldAccess for f64 {
        const KIND_NAME: &'static str = "double";
        fn to_field_value(&self) -> FieldValue<'_> {
            FieldValue::Double(*self)
        }
        fn from_binding_value(value: BindingValue) -> Option<Self> {
            match value {
                BindingValue::Int(i) => Some(i as f64),
                _ => None,
            }
        }
    }

    impl GroupFieldAccess for i32 {
        const KIND_NAME: &'static str = "int";
        fn to_field_value(&self) -> FieldValue<'_> {
            FieldValue::Int(*self)
        }
        fn from_binding_value(value: BindingValue) -> Option<Self> {
            match value {
                BindingValue::Int(i) => Some(i),
                _ => None,
            }
        }
    }

    impl GroupFieldAccess for bool {
        const KIND_NAME: &'static str = "boolean";
        fn to_field_value(&self) -> FieldValue<'_> {
            FieldValue::Bool(*self)
        }
        fn from_binding_value(value: BindingValue) -> Option<Self> {
            match value {
                BindingValue::Bool(b) => Some(b),
                _ => None,
            }
        }
    }

    /// rows 无装箱值域: 读侧 Rows 标记占位, 写侧恒 None (落 set 的越型分支)
    impl GroupFieldAccess for Vec<RowConfig> {
        const KIND_NAME: &'static str = "List";
        fn to_field_value(&self) -> FieldValue<'_> {
            FieldValue::Rows
        }
        fn from_binding_value(_value: BindingValue) -> Option<Self> {
            None
        }
    }

    // 注: 宏卫生限制 — 经两次宏展开的 ident 不与另一层展开的局部名统一, 故不做
    // 逐条目递归咀嚼, 全套在单次展开内生成 (局部名 group 同层)。

    /// 字段表 → GroupField 全套 (单次展开: 枚举 + kind_name + resolve + get + set)
    macro_rules! gen_group_field {
        ($( ($V:ident, $key:literal, $f:ident, $TY:ty) ),* $(,)?) => {
            /// 注册表键 → GroupConfig 字段选择子 (Java: getField 返回的 Field 对象)。
            /// 覆盖 GroupConfig 全部 12 个 public 字段;
            /// GroupConfig 无父类字段, 故命中集合封闭。
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            enum GroupField {
                $($V,)*
            }

            impl GroupField {
                /// Java Field.getType().getSimpleName() 等价物 (仅用于异常文案)
                fn kind_name(self) -> &'static str {
                    match self {
                        $(GroupField::$V => <$TY as GroupFieldAccess>::KIND_NAME,)*
                    }
                }
            }

            /// 注册表本体: 键 = Java 反射字段名 (精确匹配, getField 大小写敏感)。
            /// Java: NoSuchFieldException → 上层 catch → false/null。
            fn resolve(property: &str) -> Option<GroupField> {
                match property {
                    $($key => Some(GroupField::$V),)*
                    _ => None,
                }
            }

            /// Java: public static Object get(Object target, String property)
            fn get<'a>(group: &'a GroupConfig, property: &str) -> Option<FieldValue<'a>> {
                Some(match resolve(property)? {
                    $(GroupField::$V => group.$f.to_field_value(),)*
                })
            }

            /// Java: public static boolean set(Object target, String property, Object value)
            ///
            /// PORT: 越型组合 (Boolean/String 装入数值或 boolean 字段, Integer 装入
            /// boolean/String/List 字段) = JDK8 oracle 实测的 IllegalArgumentException
            /// 路径 (PropertyBinder 未捕获, 原样上抛)。cfg 是用户可编辑输入, 越型
            /// 绑定不该 panic 主线程 (A5) — warn 日志 + 忽略该次绑定 (返回 false,
            /// 调用方回落 row.value; write_* 的服务同步仍执行)
            fn set(group: &mut GroupConfig, property: Option<&str>, value: BindingValue) -> bool {
                let Some(property) = property else {
                    return false;
                };
                let field = match resolve(property) {
                    Some(f) => f,
                    None => return false,
                };
                let value_kind = value.kind_name();
                match (field, value) {
                    $((GroupField::$V, v) => match <$TY as GroupFieldAccess>::from_binding_value(v) {
                        Some(val) => {
                            group.$f = val;
                            true
                        }
                        None => {
                            logger::warn(
                                "RendererConfigHelper",
                                &format!(
                                    "越型绑定被忽略: {} 字段 '{}' 不能接受 {}",
                                    field.kind_name(),
                                    property,
                                    value_kind
                                ),
                            );
                            false
                        }
                    },)*
                }
            }
        };
    }

    /// 值字段表 + rows 追加 = 12 域注册表 (rows 非 PropertyBinder 可写值字段,
    /// 不进共享表 — 与 main_form 的 PanelField 域差一即在此)
    macro_rules! registry_table {
        ($( ($V:ident, $key:literal, $f:ident, $TY:ty) ),* $(,)?) => {
            gen_group_field! {
                $( ($V, $key, $f, $TY) , )*
                (Rows, "rows", rows, Vec<RowConfig>),
            }
        };
    }

    group_field_table!(registry_table);

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

    /// Checks if a public field exists on the target object.
    ///
    /// Java: `public static boolean hasField(Object target, String property)`
    /// (target == null / property == null 守卫由调用方 RendererConfigHelper 的
    /// 分支结构承担 — Rust 侧 target 为非空引用、property 为 &str)。
    pub(super) fn has_field(_group: &GroupConfig, property: &str) -> bool {
        resolve(property).is_some()
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
    /// 唯一调用方 ComboRowRenderer 域内恒非 null, 不可达。
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
