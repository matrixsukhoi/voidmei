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
    private static java.util.Map<String, javax.script.CompiledScript> cache = new java.util.HashMap<>();

    public static Object evaluate(String formula, Map<String, Object> variables) throws Exception {
        if (engine == null)
            return "No Engine";
        if (formula == null || formula.isEmpty())
            return "";

        try {
            Bindings bindings = new SimpleBindings(variables);

            // Try cache first
            if (engine instanceof javax.script.Compilable) {
                javax.script.CompiledScript script = cache.get(formula);
                if (script == null) {
                    script = ((javax.script.Compilable) engine).compile(formula);
                    cache.put(formula, script);
                }
                return script.eval(bindings);
            } else {
                // Fallback for non-compilable engines
                engine.setBindings(bindings, ScriptContext.ENGINE_SCOPE);
                return engine.eval(formula);
            }
        } catch (Exception e) {
            return "Err";
        }
    }

    /**
     * Checks if the formula is syntactically valid.
     */
    public static boolean check(String formula) {
        if (formula == null || formula.trim().isEmpty() || formula.equalsIgnoreCase("HEADER"))
            return true;
        try {
            engine.eval(formula);
            return true;
        } catch (Exception e) {
            return false;
        }
    }

    private Object eval(String formula, Map<String, Object> variables) throws Exception {
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
