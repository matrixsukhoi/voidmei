//! 对应 Java: `src/ui/model/DataField.java` (一比一翻译)

/// PORT: `crate::visibility_expression::VisibilityExpressionEvaluator` 已完整落地,
/// 但其持有 `&'a dyn TelemetrySource` 借用 (求值时临时构造), 无法直接存入长寿命的
/// DataField 字段 —— 装配需 Rc/Arc<dyn TelemetrySource> 所有权重设计, 属 C 类
/// FieldOverlay 移植的整体决策, 本批不预判。故先以零字段占位类型顶住
/// (fm::handle::BlkxPlaceholder 先例)。真实构造点 (FlightInfoOverlay:138 /
/// PowerInfoOverlay:173, C 类, `new VisibilityExpressionEvaluator(row.naWhen, s)`)
/// 与求值点 (FieldOverlay:208 `naWhenEvaluator.evaluate(val)`) 均不在本批。
// TODO(port): FieldOverlay 移植时切换 na_when_evaluator 为真实求值器 —— 需把
// VisibilityExpressionEvaluator 的 &'a dyn TelemetrySource 借用改为共享所有权
// (或 DataField 只存 naWhen 表达式、由消费方持 source 临时构造求值器)。
#[derive(Debug, Clone)]
pub struct VisibilityExpressionEvaluatorPlaceholder;

/// Represents a single data field displayed in an overlay.
/// Generic version - can be used for any type of data display.
pub struct DataField {
    /// Unique identifier for this field (e.g., "ias", "tas", "mach")
    pub key: String,

    /// Display label (e.g., "指示空速", "真空速")
    pub label: String,

    /// Unit string (e.g., "Km/h", "M/s", "Deg")
    pub unit: String,

    /// Configuration key to check if this field is disabled
    pub config_key: String,

    /// Whether this field should be hidden when its value is N/A
    pub hide_when_na: bool,
    pub hide_when_zero: bool,

    /// Whether this field is currently visible
    pub visible: bool,

    /// Current formatted value to display
    pub current_value: String,

    // --- Zero-GC Pipeline Support ---
    // PORT: Java `char[32] buffer + int length` 零 GC 复用缓冲 → String。
    // 已译版 `crate::format` 返回 String (无缓冲复写入 API), 故 buffer 直接改为
    // String 承接格式化产物; length 字段保留 int 语义 (有效字符数)。
    // 全部写入点 (C 类, 本批均未译):
    //   - FieldOverlay.java:208-215 (format/formatTime 落盘 + na-when 时 '-' 单字符)
    //   - EngineControlOverlay.java:533-541 (GaugeField 通道, format 后把
    //     (buffer, length) 透传 gauge.update/markedGauge.update)
    //   - src/ui/debug/OverlayPngExport.java:204-210 (PNG 调试导出)
    // 读取点: BOSStyleRenderer.java:61 (gauge.draw(..., field.buffer, field.length))
    // 及 EngineControlOverlay 的 gauge.update(intVal, buffer, length)。
    // 契约差异: Java 复用数组, length 之外是上一轮的陈旧内容 (读取方约定只看前
    // length 个码元); Rust String 恰为有效内容, 无 stale tail —— 消费方按前
    // length 个字符取值即等价, 禁止再按 Java 习惯读"数组全长"。Java 超 32 码元
    // 抛 AIOOBE, String 自动增长, 失败模式不同 (已声明的延后接缝)。
    // 缓冲内容为纯 ASCII 数字域, UTF-16 码元数与字符数一致, 无计数差异;
    // 未来写入方必须保持 length = 字符计数 (非字节)。
    /// 零 GC 管线: 格式化产物缓冲 (Java 为 char[32] 复用缓冲)
    pub buffer: String,
    /// 缓冲有效长度 (字符数; Java int)
    pub length: i32,
    pub value_supplier: Option<Box<dyn Fn() -> f64>>,
    pub visibility_supplier: Option<Box<dyn Fn() -> bool>>,
    pub precision: i32, // Default to integer
    pub format: Option<String>, // Custom format (e.g., "TIME_MM_SS")
    pub unit_supplier: Option<Box<dyn Fn() -> String>>, // Dynamic unit source
    pub precision_supplier: Option<Box<dyn Fn() -> i32>>, // Dynamic precision source
    /// NA显示条件求值器，满足条件时显示 "-"
    pub na_when_evaluator: Option<VisibilityExpressionEvaluatorPlaceholder>,
}

// PORT: Java 构造器 + 字段声明默认值 (visible=true / currentValue="---" /
// length=0 / precision=0 / 各 supplier=null) 显式初始化 (§2.10);
// key/label/configKey 为 final → Rust 无 const 字段, 以文档约定不可变。
impl DataField {
    pub fn new(
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
        hide_when_zero: bool,
    ) -> DataField {
        DataField {
            key: key.to_string(),
            label: label.to_string(),
            unit: unit.to_string(),
            config_key: config_key.to_string(),
            hide_when_na,
            hide_when_zero,
            visible: true,
            current_value: "---".to_string(),
            buffer: String::new(),
            length: 0,
            value_supplier: None,
            visibility_supplier: None,
            precision: 0,
            format: None,
            unit_supplier: None,
            precision_supplier: None,
            na_when_evaluator: None,
        }
    }

    /// Update the value with right-aligned formatting.
    pub fn set_value(&mut self, value: &str) {
        // PORT: Java String.format("%5s", value) = 右对齐补空格到宽 5, 超宽不截断;
        // Java 宽度按 UTF-16 码元计, Rust {:>5} 按字符计 —— 本字段值为 ASCII 数字域,
        // 两者一致
        self.current_value = format!("{:>5}", value);
    }

    pub fn set_unit(&mut self, unit: &str) {
        self.unit = unit.to_string();
    }

    /// Update the value and visibility based on hideWhenNA setting.
    // PORT: Java naString 形参可空 (equals(null) 恒 false → visible=true);
    // 调用点 (FieldOverlay) 实际传 "-" 常量, 按 &str 非空收参
    pub fn set_value_with_visibility(&mut self, value: &str, na_string: &str) {
        self.set_value(value);
        if self.hide_when_na {
            // Java: this.visible = !value.equals(naString) —— 取反保留
            self.visible = value != na_string;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Java 字段声明默认值: visible=true / currentValue="---" / length=0 /
    /// precision=0 / format=null / 各 supplier null
    #[test]
    fn initial_state() {
        let df = DataField::new("getIAS", "表  速", "Km/h", "disableFlightInfoIAS", true, false);
        assert_eq!(df.key, "getIAS");
        assert_eq!(df.label, "表  速");
        assert_eq!(df.unit, "Km/h");
        assert_eq!(df.config_key, "disableFlightInfoIAS");
        assert!(df.hide_when_na);
        assert!(!df.hide_when_zero);
        assert!(df.visible);
        assert_eq!(df.current_value, "---");
        assert_eq!(df.buffer, "");
        assert_eq!(df.length, 0);
        assert_eq!(df.precision, 0);
        assert!(df.format.is_none());
        assert!(df.value_supplier.is_none());
        assert!(df.visibility_supplier.is_none());
        assert!(df.unit_supplier.is_none());
        assert!(df.precision_supplier.is_none());
        assert!(df.na_when_evaluator.is_none());
    }

    /// Java String.format("%5s", v): 右对齐宽 5, 不足补前导空格, 超宽原样
    #[test]
    fn set_value_right_aligns_width_5() {
        let mut df = DataField::new("k", "l", "u", "c", false, false);
        df.set_value("123");
        assert_eq!(df.current_value, "  123");
        df.set_value("N/A");
        assert_eq!(df.current_value, "  N/A");
        df.set_value("-");
        assert_eq!(df.current_value, "    -");
        // 恰好 5 位: 不补不截
        df.set_value("12345");
        assert_eq!(df.current_value, "12345");
        // 超宽: 原样保留 (Java %5s 不截断)
        df.set_value("123456");
        assert_eq!(df.current_value, "123456");
    }

    #[test]
    fn set_unit_replaces() {
        let mut df = DataField::new("k", "l", "Ata", "c", false, false);
        df.set_unit("P/XX.X''");
        assert_eq!(df.unit, "P/XX.X''");
    }

    /// hideWhenNA=true: value 与 naString 相等 → 隐藏; 不等 → 显示
    #[test]
    fn set_value_with_visibility_na_match() {
        let mut df = DataField::new("k", "l", "u", "c", true, false);
        df.set_value_with_visibility("-", "-");
        assert!(!df.visible, "NA 值应触发隐藏");
        assert_eq!(df.current_value, "    -");
        df.set_value_with_visibility("800", "-");
        assert!(df.visible, "非 NA 值应显示");
        assert_eq!(df.current_value, "  800");
    }

    /// hideWhenNA=false: visible 不被触碰 (Java 无 else 分支, 预置 false 也保持)
    #[test]
    fn set_value_with_visibility_no_na_flag_leaves_visible() {
        let mut df = DataField::new("k", "l", "u", "c", false, false);
        df.set_value_with_visibility("-", "-");
        assert!(df.visible);
        // 预置 false (模拟前一轮隐藏) 后再更新: 仍不被翻回
        df.visible = false;
        df.set_value_with_visibility("800", "-");
        assert!(!df.visible, "hideWhenNA=false 时 visible 原样保持");
    }
}
