package prog.util;

import java.lang.reflect.Field;

/**
 * Utility class for reflection-based property binding.
 * Allows dynamic get/set of object fields by name, eliminating switch-case
 * boilerplate.
 */
public class PropertyBinder {

    /**
     * Gets a field value from an object by field name.
     * 
     * @param target   The object to read from
     * @param property The field name
     * @return The field value, or null if not found
     */
    public static Object get(Object target, String property) {
        if (target == null || property == null)
            return null;
        try {
            Field field = target.getClass().getField(property);
            return field.get(target);
        } catch (NoSuchFieldException | IllegalAccessException e) {
            // Field not found or not accessible, return null
            return null;
        }
    }

    /**
     * Gets a field value as int.
     */
    public static int getInt(Object target, String property, int defaultValue) {
        Object val = get(target, property);
        if (val instanceof Number) {
            return ((Number) val).intValue();
        }
        return defaultValue;
    }

    /**
     * Gets a field value as String.
     */
    public static String getString(Object target, String property, String defaultValue) {
        Object val = get(target, property);
        if (val instanceof String) {
            return (String) val;
        }
        return defaultValue;
    }

    /**
     * Gets a field value as boolean.
     */
    public static boolean getBool(Object target, String property, boolean defaultValue) {
        Object val = get(target, property);
        if (val instanceof Boolean) {
            return (Boolean) val;
        }
        return defaultValue;
    }

    /**
     * Sets a field value on an object by field name.
     * 
     * @param target   The object to modify
     * @param property The field name
     * @param value    The value to set
     * @return true if successful, false otherwise
     */
    public static boolean set(Object target, String property, Object value) {
        if (target == null || property == null)
            return false;
        try {
            Field field = target.getClass().getField(property);
            field.set(target, value);
            return true;
        } catch (NoSuchFieldException | IllegalAccessException e) {
            return false;
        }
    }

    /**
     * Sets an int field value.
     */
    public static boolean setInt(Object target, String property, int value) {
        return set(target, property, value);
    }

    /**
     * Sets a String field value.
     */
    public static boolean setString(Object target, String property, String value) {
        return set(target, property, value);
    }

    /**
     * Sets a boolean field value.
     */
    public static boolean setBool(Object target, String property, boolean value) {
        return set(target, property, value);
    }
}
