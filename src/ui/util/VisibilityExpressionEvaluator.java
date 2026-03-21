package ui.util;

import prog.config.SExpParser.SExp;
import prog.config.SExpParser.SList;
import prog.config.SExpParser.SAtom;
import ui.model.TelemetrySource;

/**
 * 表达式求值器，用于计算 :visible-when 条件
 *
 * 表达式直接从 ConfigLoader 解析的 SExp 对象传入，无需额外解析字符串
 *
 * 支持的表达式语法：
 * - 布尔字面量: true, false
 * - 方法调用: (isJetEngine), (isPropEngine), (isPistonEngine), (isTurbopropEngine),
 *             (isEngineCheckDone), (hasWep)
 * - 值比较: (> value 0), (>= value 100), (!= value -65535), (= value 1)
 * - 逻辑组合: (not expr), (and expr1 expr2 ...), (or expr1 expr2 ...)
 *
 * 引擎类型方法说明：
 * - isJetEngine: 喷气机（涡喷、涡扇）
 * - isPropEngine: 螺旋桨（活塞+涡桨）
 * - isPistonEngine: 仅活塞机（用于进气压等仅活塞机显示的字段）
 * - isTurbopropEngine: 仅涡桨
 *
 * 示例配置:
 * :visible-when (and (isPistonEngine) (!= value 1))  ; 仅活塞机显示
 * :visible-when (and (not (isJetEngine)) (> value 0)) ; 螺旋桨机显示
 * :visible-when (!= value -65535)
 */
public class VisibilityExpressionEvaluator {

    private final SExp expression;
    private final TelemetrySource source;

    /**
     * 构造函数
     * @param expression 已解析的 S-expression（直接来自 ConfigLoader）
     * @param source 遥测数据源（实现了引擎类型判断方法）
     */
    public VisibilityExpressionEvaluator(SExp expression, TelemetrySource source) {
        this.expression = expression;
        this.source = source;
    }

    /**
     * 求值，返回字段是否可见
     * @param value 当前字段的值
     * @return true 表示字段应显示，false 表示字段应隐藏
     */
    public boolean evaluate(double value) {
        return evaluateExp(expression, value);
    }

    /**
     * 递归求值 S-expression
     */
    private boolean evaluateExp(SExp exp, double value) {
        if (exp == null) {
            return true; // null 表达式默认显示
        }

        if (exp.isAtom()) {
            SAtom atom = exp.asAtom();

            // 布尔字面量
            if (atom.type == SAtom.AtomType.BOOLEAN) {
                return atom.getBool();
            }

            // 符号（无参方法调用，如 isJetEngine）
            if (atom.type == SAtom.AtomType.SYMBOL) {
                return callMethod(atom.getString());
            }

            return true; // 其他原子类型默认为 true
        }

        SList list = exp.asList();
        if (list.children.isEmpty()) {
            return true; // 空列表默认为 true
        }

        // 获取操作符（列表的第一个元素）
        SExp head = list.children.get(0);
        if (!head.isAtom()) {
            return true; // 第一个元素不是原子，默认为 true
        }

        String op = head.asAtom().getString();

        switch (op) {
            // 逻辑非
            case "not":
                if (list.children.size() < 2) return true;
                return !evaluateExp(list.children.get(1), value);

            // 逻辑与（所有子表达式都为 true）
            case "and":
                for (int i = 1; i < list.children.size(); i++) {
                    if (!evaluateExp(list.children.get(i), value)) {
                        return false;
                    }
                }
                return true;

            // 逻辑或（任一子表达式为 true）
            case "or":
                for (int i = 1; i < list.children.size(); i++) {
                    if (evaluateExp(list.children.get(i), value)) {
                        return true;
                    }
                }
                return false;

            // 大于
            case ">":
                if (list.children.size() < 3) return true;
                return getValue(list.children.get(1), value) > getValue(list.children.get(2), value);

            // 大于等于
            case ">=":
                if (list.children.size() < 3) return true;
                return getValue(list.children.get(1), value) >= getValue(list.children.get(2), value);

            // 小于
            case "<":
                if (list.children.size() < 3) return true;
                return getValue(list.children.get(1), value) < getValue(list.children.get(2), value);

            // 小于等于
            case "<=":
                if (list.children.size() < 3) return true;
                return getValue(list.children.get(1), value) <= getValue(list.children.get(2), value);

            // 等于（使用浮点数容差比较）
            case "=":
            case "==":
                if (list.children.size() < 3) return true;
                return Math.abs(getValue(list.children.get(1), value) - getValue(list.children.get(2), value)) < 0.0001;

            // 不等于
            case "!=":
                if (list.children.size() < 3) return true;
                return Math.abs(getValue(list.children.get(1), value) - getValue(list.children.get(2), value)) >= 0.0001;

            default:
                // 无参方法调用，如 (isJetEngine)
                return callMethod(op);
        }
    }

    /**
     * 获取表达式的数值
     * 支持 'value' 关键字代表当前字段值，以及数字字面量
     */
    private double getValue(SExp exp, double value) {
        if (exp.isAtom()) {
            SAtom atom = exp.asAtom();

            // 'value' 关键字代表当前字段的值
            if ("value".equals(atom.getString())) {
                return value;
            }

            // 数字字面量
            if (atom.type == SAtom.AtomType.NUMBER) {
                return atom.getDouble();
            }

            // 尝试将符号解析为数字
            try {
                return Double.parseDouble(atom.getString());
            } catch (NumberFormatException e) {
                return 0;
            }
        }
        return 0;
    }

    /**
     * 调用 TelemetrySource 上的方法
     * @param methodName 方法名称
     * @return 方法返回值，未知方法默认返回 true（显示）
     */
    private boolean callMethod(String methodName) {
        // 预览模式或数据源不可用时，默认显示所有字段
        if (source == null) {
            return true;
        }

        switch (methodName) {
            // 引擎类型判断
            case "isJetEngine":
                return source.isJetEngine();
            case "isPropEngine":
                return source.isPropEngine();
            case "isPistonEngine":
                // 活塞机（不包括涡桨），用于进气压等仅活塞机显示的字段
                return source.isPistonEngine();
            case "isTurbopropEngine":
                // 涡轮螺旋桨发动机
                return source.isTurbopropEngine();
            case "isEngineCheckDone":
                return source.isEngineCheckDone();

            // 飞机特性判断
            case "hasWep":
                return source.hasWep();

            // 未知方法：默认显示
            default:
                return true;
        }
    }
}
