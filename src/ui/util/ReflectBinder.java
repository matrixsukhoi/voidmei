package ui.util;

import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.util.function.DoubleSupplier;
import java.util.function.Supplier;

/**
 * Utility class to resolve string targets to zero-GC data accessors using
 * MethodHandles.
 */
public class ReflectBinder {

    /**
     * Resolves a target string to a DoubleSupplier.
     * 
     * Target formats:
     * - "getRPM" -> binds to service.getRPM()
     * - "getWingSweep * 100" -> binds to service.getWingSweep() * 100
     * 
     * @param service The service instance to bind to
     * @param target  The target string from config
     * @return Zero-GC DoubleSupplier
     */
    public static DoubleSupplier resolveDouble(Object service, String target) {
        if (target == null || target.isEmpty()) {
            return () -> 0.0;
        }

        if (service == null) {
            prog.util.Logger.warn("ReflectBinder", "Service is null, cannot bind target: " + target);
            return () -> 0.0;
        }

        String methodName = target.trim();
        double multiplier = 1.0;

        // Simple arithmetic support (* only for now)
        if (target.contains("*")) {
            String[] parts = target.split("\\*");
            if (parts.length == 2) {
                methodName = parts[0].trim();
                try {
                    multiplier = Double.parseDouble(parts[1].trim());
                } catch (NumberFormatException e) {
                    prog.util.Logger.warn("ReflectBinder", "Invalid multiplier in target: " + target);
                }
            }
        }

        // Try binding
        try {
            MethodHandles.Lookup lookup = MethodHandles.publicLookup();
            MethodHandle boundHandle = null;

            // Try standard types in order of likelihood
            Class<?>[] returnTypes = { double.class, float.class, long.class, int.class };
            for (Class<?> returnType : returnTypes) {
                try {
                    MethodType mt = MethodType.methodType(returnType);
                    MethodHandle handle = lookup.findVirtual(service.getClass(), methodName, mt);
                    boundHandle = handle.bindTo(service);
                    break;
                } catch (NoSuchMethodException e) {
                    // Try next type
                }
            }

            if (boundHandle == null) {
                prog.util.Logger.warn("ReflectBinder", "Method '" + methodName + "' not found with expected types in "
                        + service.getClass().getName());
                return () -> 0.0;
            }

            final MethodHandle finalHandle = boundHandle;
            final double finalMultiplier = multiplier;

            // Common case optimizing: double/float/int/long invocation
            return () -> {
                try {
                    Object result = finalHandle.invoke();
                    if (result instanceof Number) {
                        return ((Number) result).doubleValue() * finalMultiplier;
                    }
                    return 0.0;
                } catch (Throwable e) {
                    return 0.0;
                }
            };

        } catch (IllegalAccessException e) {
            prog.util.Logger.warn("ReflectBinder", "Access denied for method '" + methodName + "' in "
                    + service.getClass().getName() + ". Is it public?");
            return () -> 0.0;
        } catch (Exception e) {
            prog.util.Logger.warn("ReflectBinder", "Unexpected error binding '" + target + "': " + e.getMessage());
            return () -> 0.0;
        }
    }

    /**
     * Resolves a target string to a String Supplier.
     */
    public static Supplier<String> resolveString(Object service, String target) {
        if (target == null || target.isEmpty()) {
            return () -> "";
        }
        if (service == null)
            return () -> "";

        try {
            MethodHandles.Lookup lookup = MethodHandles.publicLookup();
            MethodType mt = MethodType.methodType(String.class);
            MethodHandle handle = lookup.findVirtual(service.getClass(), target.trim(), mt);
            MethodHandle boundHandle = handle.bindTo(service);

            return () -> {
                try {
                    return (String) boundHandle.invokeExact();
                } catch (Throwable e) {
                    return "";
                }
            };
        } catch (NoSuchMethodException e) {
            prog.util.Logger.warn("ReflectBinder", "String Method '" + target.trim() + "' NOT found in "
                    + service.getClass().getName());
            return () -> "";
        } catch (Exception e) {
            prog.util.Logger.warn("ReflectBinder",
                    "Could not bind String target: " + target + " (" + e.getMessage() + ")");
            return () -> "";
        }
    }

    /**
     * Resolves a target string to a BooleanSupplier.
     */
    public static java.util.function.BooleanSupplier resolveBoolean(Object service, String target) {
        if (target == null || target.isEmpty() || service == null) {
            return () -> true;
        }

        try {
            MethodHandles.Lookup lookup = MethodHandles.publicLookup();
            MethodType mt = MethodType.methodType(boolean.class);
            MethodHandle handle = lookup.findVirtual(service.getClass(), target.trim(), mt);
            MethodHandle boundHandle = handle.bindTo(service);

            return () -> {
                try {
                    return (boolean) boundHandle.invoke();
                } catch (Throwable e) {
                    return true;
                }
            };
        } catch (NoSuchMethodException e) {
            // Silence warning for validity checks as they are optional
            return null;
        } catch (Exception e) {
            prog.util.Logger.warn("ReflectBinder", "Error binding boolean '" + target + "': " + e.getMessage());
            return () -> true;
        }
    }
}
