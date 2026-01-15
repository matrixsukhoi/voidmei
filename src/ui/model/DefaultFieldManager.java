package ui.model;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Default implementation of FieldManager.
 * Uses ArrayList for ordered storage and HashMap for fast lookup.
 */
public class DefaultFieldManager implements FieldManager {

    private final List<FlightField> fields = new ArrayList<>();
    private final Map<String, FlightField> fieldMap = new HashMap<>();
    private final ConfigProvider config;

    public DefaultFieldManager(ConfigProvider config) {
        this.config = config;
    }

    @Override
    public void addField(String key, String label, String unit, String configKey, boolean hideWhenNA) {
        // Check if field is disabled in config
        if (config != null) {
            String tmp = config.getConfig(configKey);
            if (tmp != null && !tmp.isEmpty() && Boolean.parseBoolean(tmp)) {
                return; // Field is disabled, don't add
            }
        }

        FlightField field = new FlightField(key, label, unit, configKey, hideWhenNA);
        fields.add(field);
        fieldMap.put(key, field);
    }

    @Override
    public void updateField(String key, String value, String naString) {
        FlightField field = fieldMap.get(key);
        if (field != null) {
            field.setValueWithVisibility(value, naString);
        }
    }

    @Override
    public List<FlightField> getFields() {
        return fields;
    }

    @Override
    public FlightField getField(String key) {
        return fieldMap.get(key);
    }

    @Override
    public void clearAll() {
        fields.clear();
        fieldMap.clear();
    }

    @Override
    public int size() {
        return fields.size();
    }

    @Override
    public int visibleCount() {
        int count = 0;
        for (FlightField field : fields) {
            if (field.visible)
                count++;
        }
        return count;
    }
}
