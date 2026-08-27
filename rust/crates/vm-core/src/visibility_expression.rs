//! VisibilityExpressionEvaluator 的 Rust 移植 (src/ui/util/VisibilityExpressionEvaluator.java)
//!
//! 表达式求值器，用于计算 :visible-when 条件
//!
//! 表达式直接从 ConfigLoader 解析的 SExp 对象传入，无需额外解析字符串
//!
//! 支持的表达式语法：
//! - 布尔字面量: true, false
//! - 方法调用: (isJetEngine), (isPropEngine), (isPistonEngine), (isTurbopropEngine),
//!   (isEngineCheckDone), (hasWep)
//! - 值比较: (> value 0), (>= value 100), (!= value -65535), (= value 1)
//! - 逻辑组合: (not expr), (and expr1 expr2 ...), (or expr1 expr2 ...)
//!
//! 引擎类型方法说明：
//! - isJetEngine: 喷气机（涡喷、涡扇）
//! - isPropEngine: 螺旋桨（活塞+涡桨）
//! - isPistonEngine: 仅活塞机（用于进气压等仅活塞机显示的字段）
//! - isTurbopropEngine: 仅涡桨
//!
//! 示例配置:
//! :visible-when (and (isPistonEngine) (!= value 1))  ; 仅活塞机显示
//! :visible-when (and (not (isJetEngine)) (> value 0)) ; 螺旋桨机显示
//! :visible-when (!= value -65535)

use crate::sexp_parser::{AtomType, SExp};
use crate::ui_model::TelemetrySource;
use std::rc::Rc;

// PORT: 对应 Java `import ui.model.TelemetrySource;`。ui.model 批次落地前本文件曾按
// evaluator 实际调用的 7 个布尔方法定义过最小 trait, 现已切换到 crate::ui_model 的
// 完整接口 (天然涵盖那 7 个方法, javadoc 原位保留于该模块), 求值逻辑零改动。

/// 表达式求值器 — 见[模块文档][self]。
///
/// PORT: Java `private final SExp expression` 可为 null (ConfigLoader 未给
/// :visible-when/:na-when 时 RowConfig 字段为 null, 见 FieldOverlay.java:208 的
/// null 守卫) → `Option<Rc<SExp>>`; `Rc` 共享对应 Java 引用语义 (子树来自
/// ConfigLoader 解析树, sexp_parser 先例)。
pub struct VisibilityExpressionEvaluator<'a> {
    expression: Option<Rc<SExp>>,
    source: Option<&'a dyn TelemetrySource>,
}

impl<'a> VisibilityExpressionEvaluator<'a> {
    /// 构造函数
    /// @param expression 已解析的 S-expression（直接来自 ConfigLoader）
    /// @param source 遥测数据源（实现了引擎类型判断方法）
    pub fn new(
        expression: Option<Rc<SExp>>,
        source: Option<&'a dyn TelemetrySource>,
    ) -> VisibilityExpressionEvaluator<'a> {
        VisibilityExpressionEvaluator {
            expression,
            source,
        }
    }

    /// 求值，返回字段是否可见
    /// @param value 当前字段的值
    /// @return true 表示字段应显示，false 表示字段应隐藏
    pub fn evaluate(&self, value: f64) -> bool {
        self.evaluate_exp(self.expression.as_ref(), value)
    }

    /// 递归求值 S-expression
    fn evaluate_exp(&self, exp: Option<&Rc<SExp>>, value: f64) -> bool {
        // PORT: Java 首行 `if (exp == null) return true;` → Option 判空, 位置保持
        if exp.is_none() {
            return true; // null 表达式默认显示
        }
        let exp = exp.unwrap();

        if exp.is_atom() {
            let atom = exp.as_atom();

            // 布尔字面量
            if atom.r#type == AtomType::Boolean {
                return atom.get_bool();
            }

            // 符号（无参方法调用，如 isJetEngine）
            if atom.r#type == AtomType::Symbol {
                return self.call_method(atom.get_string());
            }

            return true; // 其他原子类型默认为 true
        }

        let list = exp.as_list();
        if list.children.is_empty() {
            return true; // 空列表默认为 true
        }

        // 获取操作符（列表的第一个元素）
        let head = &list.children[0];
        if !head.is_atom() {
            return true; // 第一个元素不是原子，默认为 true
        }

        let op = head.as_atom().get_string();

        match op {
            // 逻辑非
            "not" => {
                if list.children.len() < 2 {
                    return true;
                }
                !self.evaluate_exp(Some(&list.children[1]), value)
            }

            // 逻辑与（所有子表达式都为 true）
            "and" => {
                for i in 1..list.children.len() {
                    if !self.evaluate_exp(Some(&list.children[i]), value) {
                        return false;
                    }
                }
                true
            }

            // 逻辑或（任一子表达式为 true）
            "or" => {
                for i in 1..list.children.len() {
                    if self.evaluate_exp(Some(&list.children[i]), value) {
                        return true;
                    }
                }
                false
            }

            // 大于
            ">" => {
                if list.children.len() < 3 {
                    return true;
                }
                self.get_value(&list.children[1], value) > self.get_value(&list.children[2], value)
            }

            // 大于等于
            ">=" => {
                if list.children.len() < 3 {
                    return true;
                }
                self.get_value(&list.children[1], value) >= self.get_value(&list.children[2], value)
            }

            // 小于
            "<" => {
                if list.children.len() < 3 {
                    return true;
                }
                self.get_value(&list.children[1], value) < self.get_value(&list.children[2], value)
            }

            // 小于等于
            "<=" => {
                if list.children.len() < 3 {
                    return true;
                }
                self.get_value(&list.children[1], value) <= self.get_value(&list.children[2], value)
            }

            // 等于（使用浮点数容差比较）
            "=" | "==" => {
                if list.children.len() < 3 {
                    return true;
                }
                (self.get_value(&list.children[1], value)
                    - self.get_value(&list.children[2], value))
                    .abs()
                    < 0.0001
            }

            // 不等于
            "!=" => {
                if list.children.len() < 3 {
                    return true;
                }
                (self.get_value(&list.children[1], value)
                    - self.get_value(&list.children[2], value))
                    .abs()
                    >= 0.0001
            }

            // 无参方法调用，如 (isJetEngine)
            _ => self.call_method(op),
        }
    }

    /// 获取表达式的数值
    /// 支持 'value' 关键字代表当前字段值，以及数字字面量
    fn get_value(&self, exp: &Rc<SExp>, value: f64) -> f64 {
        if exp.is_atom() {
            let atom = exp.as_atom();

            // 'value' 关键字代表当前字段的值
            // (Java `"value".equals(...)`: 只比字符串内容, 不区分原子类型 — STRING "value" 同样命中)
            if atom.get_string() == "value" {
                return value;
            }

            // 数字字面量
            if atom.r#type == AtomType::Number {
                return atom.get_double();
            }

            // 尝试将符号解析为数字
            // PORT: Java `try { Double.parseDouble } catch (NumberFormatException) { return 0; }`
            // → §2.15 先例 `parse().unwrap_or(default)`。Rust parse 与 Java parseDouble
            // 接受域有差 (Java 8 oracle 实测): Java 收而 Rust 拒 → 此处 0.0 (前后空白
            // ≤U+0020 / fFdD 后缀 / 十六进制浮点: (> " 5 " 3) Java true / Rust false,
            // (> "0x1p1" 1) 同); Rust 收而 Java 拒 → 非 0 (大小写不敏感 inf/nan/infinity:
            // (> "inf" 0) Java false / Rust true)。真实 cfg 的比较操作数均为 'value' 或
            // NUMBER 原子 (走上方 get_double, 与 Java 位级一致), 分歧仅在带引号字符串/
            // 符号当操作数的域外输入。位级对齐需 sexp_parser 导出 java_parse_double
            // (现为私有) — 跨文件改动, 本批次不越界 (PORTING.md §6), 上报主 agent。
            atom.get_string().parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        }
    }

    /// 调用 TelemetrySource 上的方法
    /// @param methodName 方法名称
    /// @return 方法返回值，未知方法默认返回 true（显示）
    fn call_method(&self, method_name: &str) -> bool {
        // 预览模式或数据源不可用时，默认显示所有字段
        if self.source.is_none() {
            return true;
        }
        let source = self.source.unwrap();

        match method_name {
            // 引擎类型判断
            "isJetEngine" => source.is_jet_engine(),
            "isPropEngine" => source.is_prop_engine(),
            // 活塞机（不包括涡桨），用于进气压等仅活塞机显示的字段
            "isPistonEngine" => source.is_piston_engine(),
            // 涡轮螺旋桨发动机
            "isTurbopropEngine" => source.is_turboprop_engine(),
            "isEngineCheckDone" => source.is_engine_check_done(),

            // 飞机特性判断
            "hasWep" => source.has_wep(),

            // 火箭助推器
            "hasBooster" => source.has_booster(),

            // 未知方法：默认显示
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests;
