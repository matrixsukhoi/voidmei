package ui.model;

import java.util.List;

/**
 * Interface for managing flight data fields.
 * Provides a unified API for adding, updating, and querying fields.
 */
public interface FieldManager {

    /**
     * Add a new field to the manager.
     * 
     * @param key        Unique identifier
     * @param label      Display label
     * @param unit       Unit string
     * @param configKey  Configuration key to check if disabled
     * @param hideWhenNA Whether to hide when value is N/A
     */
    void addField(String key, String label, String unit, String configKey, boolean hideWhenNA);

    /**
     * Update a field's value by key.
     * 
     * @param key      Field key
     * @param value    New value
     * @param naString The N/A string to check against for hideWhenNA fields
     */
    void updateField(String key, String value, String naString);

    /**
     * Get all fields in order.
     */
    List<FlightField> getFields();

    /**
     * Get a specific field by key.
     */
    FlightField getField(String key);

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
