package ui.model;

import java.util.List;

/**
 * Interface for managing overlay data fields.
 * Provides a unified API for adding, updating, and querying fields.
 */
public interface FieldManager {

        /**
         * Add a new field to the manager.
         */
        void addField(String key, String label, String unit, String configKey, boolean hideWhenNA, boolean hideWhenZero,
                        String previewValue, String format);

        /**
         * Update a field's value by key.
         */
        void updateField(String key, String value, String naString);

        /**
         * Set a field's visibility by key.
         */
        void setFieldVisible(String key, boolean visible);

        /**
         * Update a field's unit by key.
         */
        void updateFieldUnit(String key, String unit);

        /**
         * Bind a field to a zero-GC double supplier.
         */
        void bind(String key, java.util.function.DoubleSupplier supplier, int precision);

        /**
         * Bind a field to a zero-GC double supplier with an optional visibility
         * supplier.
         */
        void bind(String key, java.util.function.DoubleSupplier valueSupplier,
                        java.util.function.BooleanSupplier visibilitySupplier, int precision, String format);

        /**
         * Bind a field with dynamic unit and precision suppliers.
         */
        void bind(String key, java.util.function.DoubleSupplier valueSupplier,
                        java.util.function.BooleanSupplier visibilitySupplier, int precision, String format,
                        java.util.function.Supplier<String> unitSupplier, java.util.function.IntSupplier precisionSupplier);

        /**
         * Get all fields in order.
         */
        List<DataField> getFields();

        /**
         * Get a specific field by key.
         */
        DataField getField(String key);

        /**
         * Clear all fields.
         */
        void clearAll();

        /**
         * Get the number of fields.
         */
        int size();

        /**
         * Get the number of currently visible fields.
         */
        int visibleCount();
}
