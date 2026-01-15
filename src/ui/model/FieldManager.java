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
    void addField(String key, String label, String unit, String configKey, boolean hideWhenNA);

    /**
     * Update a field's value by key.
     */
    void updateField(String key, String value, String naString);

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
