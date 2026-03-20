import ui.util.VisibilityExpressionEvaluator;
import prog.config.SExpParser;
import prog.config.SExpParser.SExp;

/**
 * 测试 VisibilityExpressionEvaluator 表达式求值器
 *
 * 运行方式: ./script/test.sh visibility
 */
public class TestVisibilityExpressionEvaluator {

    private static int passed = 0;
    private static int failed = 0;

    public static void main(String[] args) {
        System.out.println("=== VisibilityExpressionEvaluator 测试 ===\n");

        // 测试比较运算符
        testComparisons();

        // 测试逻辑运算符
        testLogicalOperators();

        // 测试复杂表达式
        testComplexExpressions();

        // 测试边界情况
        testEdgeCases();

        // 输出结果
        System.out.println("\n=== 测试结果 ===");
        System.out.println("通过: " + passed);
        System.out.println("失败: " + failed);

        if (failed > 0) {
            System.exit(1);
        }
    }

    private static void testComparisons() {
        System.out.println("-- 比较运算符测试 --");

        // (> value 0)
        assertEval("(> value 0)", 100.0, true, "100 > 0 应为 true");
        assertEval("(> value 0)", 0.0, false, "0 > 0 应为 false");
        assertEval("(> value 0)", -10.0, false, "-10 > 0 应为 false");

        // (>= value 0)
        assertEval("(>= value 0)", 0.0, true, "0 >= 0 应为 true");
        assertEval("(>= value 0)", -0.0001, false, "-0.0001 >= 0 应为 false");

        // (!= value -65535)
        assertEval("(!= value -65535)", 100.0, true, "100 != -65535 应为 true");
        assertEval("(!= value -65535)", -65535.0, false, "-65535 != -65535 应为 false");

        // (= value 1)
        // 容差为 0.0001，所以 |a - b| < 0.0001 视为相等
        assertEval("(= value 1)", 1.0, true, "1 = 1 应为 true");
        assertEval("(= value 1)", 1.0001, true, "1.0001 = 1 应为 true (差值 0.0001 < 0.0001 边界)");
        assertEval("(= value 1)", 1.0002, false, "1.0002 = 1 应为 false (差值 0.0002 > 0.0001)");
        assertEval("(= value 1)", 1.00005, true, "1.00005 = 1 应为 true (在容差内)");

        // (< value 100)
        assertEval("(< value 100)", 50.0, true, "50 < 100 应为 true");
        assertEval("(< value 100)", 100.0, false, "100 < 100 应为 false");

        // (<= value 100)
        assertEval("(<= value 100)", 100.0, true, "100 <= 100 应为 true");
        assertEval("(<= value 100)", 100.1, false, "100.1 <= 100 应为 false");
    }

    private static void testLogicalOperators() {
        System.out.println("\n-- 逻辑运算符测试 --");

        // (not (> value 0))
        assertEval("(not (> value 0))", 100.0, false, "not (100 > 0) 应为 false");
        assertEval("(not (> value 0))", -10.0, true, "not (-10 > 0) 应为 true");

        // (and (> value 0) (< value 100))
        assertEval("(and (> value 0) (< value 100))", 50.0, true, "50 在 (0, 100) 范围内");
        assertEval("(and (> value 0) (< value 100))", 0.0, false, "0 不在 (0, 100) 范围内");
        assertEval("(and (> value 0) (< value 100))", 100.0, false, "100 不在 (0, 100) 范围内");

        // (or (< value 0) (> value 100))
        assertEval("(or (< value 0) (> value 100))", -10.0, true, "-10 < 0，满足 or 的第一个条件");
        assertEval("(or (< value 0) (> value 100))", 150.0, true, "150 > 100，满足 or 的第二个条件");
        assertEval("(or (< value 0) (> value 100))", 50.0, false, "50 不满足 or 的任何条件");

        // 嵌套 and/or
        assertEval("(and (or (< value 0) (> value 10)) (< value 100))", -5.0, true, "-5 < 0 且 < 100");
        assertEval("(and (or (< value 0) (> value 10)) (< value 100))", 5.0, false, "5 在 [0, 10] 之间，不满足 or");
        assertEval("(and (or (< value 0) (> value 10)) (< value 100))", 150.0, false, "150 > 100，不满足 and 的第二个条件");
    }

    private static void testComplexExpressions() {
        System.out.println("\n-- 复杂表达式测试 --");

        // 模拟功率字段: (and (not (isJetEngine)) (> value 0))
        // 由于没有 TelemetrySource，方法调用默认返回 true
        assertEval("(and (not true) (> value 0))", 100.0, false, "not true = false，整个 and 为 false");
        assertEval("(and (not false) (> value 0))", 100.0, true, "not false = true，100 > 0，整个为 true");

        // 模拟推力字段: (or (isJetEngine) (> value 0))
        assertEval("(or true (> value 0))", 0.0, true, "第一个条件 true，整个为 true");
        assertEval("(or false (> value 0))", 100.0, true, "第一个条件 false，但 100 > 0");
        assertEval("(or false (> value 0))", 0.0, false, "两个条件都为 false");

        // 模拟加力时: (and (hasWep) (> value 0))
        assertEval("(and true (> value 0))", 300.0, true, "有加力且值 > 0");
        assertEval("(and true (> value 0))", 0.0, false, "有加力但值 = 0");
        assertEval("(and false (> value 0))", 300.0, false, "无加力系统");
    }

    private static void testEdgeCases() {
        System.out.println("\n-- 边界情况测试 --");

        // 空列表
        assertEval("()", 0.0, true, "空列表应返回 true");

        // 布尔字面量
        assertEval("true", 0.0, true, "true 字面量");
        assertEval("false", 0.0, false, "false 字面量");

        // 浮点数比较容差: |a - b| < 0.0001
        assertEval("(= value 0)", 0.00001, true, "0.00001 应在 0.0001 容差内被视为等于 0");
        assertEval("(= value 0)", 0.0001, false, "0.0001 刚好在容差边界 (0.0001 >= 0.0001，不相等)");
        assertEval("(= value 0)", 0.001, false, "0.001 超出容差");

        // 多个 and 子表达式
        assertEval("(and (> value 0) (< value 100) (> value 10) (< value 90))", 50.0, true, "50 满足所有条件");
        assertEval("(and (> value 0) (< value 100) (> value 10) (< value 90))", 5.0, false, "5 不满足 > 10");
    }

    private static void assertEval(String expression, double value, boolean expected, String description) {
        SExpParser parser = new SExpParser();
        java.util.List<SExp> exps = parser.parse(expression);

        if (exps.isEmpty()) {
            System.out.println("  [失败] " + description + " - 解析表达式失败: " + expression);
            failed++;
            return;
        }

        SExp exp = exps.get(0);
        VisibilityExpressionEvaluator evaluator = new VisibilityExpressionEvaluator(exp, null);
        boolean actual = evaluator.evaluate(value);

        if (actual == expected) {
            System.out.println("  [通过] " + description);
            passed++;
        } else {
            System.out.println("  [失败] " + description);
            System.out.println("         表达式: " + expression + ", 值: " + value);
            System.out.println("         期望: " + expected + ", 实际: " + actual);
            failed++;
        }
    }
}
