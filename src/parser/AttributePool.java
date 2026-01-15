package parser;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.lang.reflect.Field;

/**
 * A dedicated Global Variable Pool for the application.
 * Supports efficient storage and dot-notation access to hierarchical data.
 * 
 * Replaces ad-hoc storage in blkx or service adapters.
 */
public class AttributePool {

    // The core storage
    private final Map<String, Object> pool = new ConcurrentHashMap<>();

    public AttributePool() {
    }

    /**
     * Put a value into the pool.
     */
    public void put(String key, Object value) {
        if (key == null)
            return;
        if (value == null) {
            pool.remove(key);
            return;
        }
        pool.put(key, value);
    }

    /**
     * Put all values from a map.
     */
    public void putAll(Map<String, Object> map) {
        if (map == null)
            return;
        pool.putAll(map);
    }

    /**
     * Get a value support dot-notation path.
     * e.g. "TAS", "state.TAS", "NoFlapsWing.CdMin"
     * 
     * Strategy:
     * 1. Check for exact key match (fastest).
     * 2. If path contains dot, split and traverse.
     */
    public Object getValue(String path) {
        if (path == null)
            return null;

        // 1. Exact match (O(1))
        Object val = pool.get(path);
        if (val != null)
            return val;

        // 2. Traversal
        if (path.indexOf('.') != -1) {
            return traverse(path);
        }

        return null;
    }

    private Object traverse(String path) {
        String[] parts = path.split("\\.");
        if (parts.length == 0)
            return null;

        // Start with the root object from pool (e.g. "state" or "NoFlapsWing")
        Object current = pool.get(parts[0]);
        if (current == null)
            return null;

        for (int i = 1; i < parts.length; i++) {
            if (current == null)
                return null;

            // If it's a Map, use get
            if (current instanceof Map) {
                current = ((Map<?, ?>) current).get(parts[i]);
            } else {
                // Use Reflection for POJOs (like blkx inner classes or parser.state)
                try {
                    Field f = current.getClass().getField(parts[i]);
                    current = f.get(current);
                } catch (Exception e) {
                    return null;
                }
            }
        }
        return current;
    }

    /**
     * Clear the pool.
     */
    public void clear() {
        pool.clear();
    }

    public int size() {
        return pool.size();
    }
}
