package prog.util;

import java.util.Map;
import javax.script.Bindings;
import javax.script.ScriptContext;
import javax.script.ScriptEngine;
import javax.script.ScriptEngineManager;
import javax.script.SimpleBindings;

public class FormulaEvaluator {

    private static ScriptEngine engine;

    static {
        // Initialize Nashorn engine once
        ScriptEngineManager manager = new ScriptEngineManager();
        engine = manager.getEngineByName("JavaScript");
        if (engine == null) {
            System.err.println("Warning: JavaScript Engine not found (Nashorn missing?)");
        }
    }

    /**
     * Evaluates a math formula using the provided variable context.
     * 
     * @param formula   The math expression (e.g. "ias * 3.6")
     * @param variables A map of variable names to values
     * @return The result as an Object (usually Double or Integer)
     */
    public static Object evaluate(String formula, Map<String, Object> variables) {
        if (engine == null)
            return "No Engine";
        if (formula == null || formula.isEmpty())
            return "";

        try {
            // Apply variables to a new Bindings scope to avoid concurrency issues if
            // multi-threaded,
            // or just to keep scope clean.
            Bindings bindings = new SimpleBindings(variables);
            engine.setBindings(bindings, ScriptContext.ENGINE_SCOPE);

            return engine.eval(formula);
        } catch (Exception e) {
            return "Err"; // Simplistic error indication
        }
    }
}
