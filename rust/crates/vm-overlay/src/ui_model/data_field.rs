//! 单个 overlay 显示字段模型 (Java DataField 一比一移植)。

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
    // Rust 版 `crate::base::format` 返回 String (无缓冲复写入 API), 故 buffer 直接改为
    // String 承接格式化产物; length 字段保留 int 语义 (有效字符数)。
    // Java 侧写入点: FieldOverlay (format/formatTime 落盘 + na-when 时 '-' 单字符)、
    // EngineControlOverlay (GaugeField 通道)、OverlayPngExport (PNG 调试导出);
    // 读取点: BOSStyleRenderer 的 gauge.draw(..., field.buffer, field.length)
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
    pub precision: i32,                                   // Default to integer
    pub format: Option<String>,                           // Custom format (e.g., "TIME_MM_SS")
    pub unit_supplier: Option<Box<dyn Fn() -> String>>,   // Dynamic unit source
    pub precision_supplier: Option<Box<dyn Fn() -> i32>>, // Dynamic precision source
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
            self.visible = value != na_string;
        }
    }
}

#[cfg(test)]
mod tests;
