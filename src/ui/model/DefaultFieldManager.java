package ui.model;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import prog.config.ConfigProvider;

/**
 * Default implementation of FieldManager.
 * Uses ArrayList for ordered storage and HashMap for fast lookup.
 */
public class DefaultFieldManager implements FieldManager {

    private final List<DataField> fields = new ArrayList<>();
    private final Map<String, DataField> fieldMap = new HashMap<>();
    private final ConfigProvider config;

    public DefaultFieldManager(ConfigProvider config) {
        this.config = config;
    }

    @Override
    public void addField(String key, String label, String unit, String configKey, boolean hideWhenNA,
            String exampleValue) {
        if (config != null) {
            String tmp = config.getConfig(configKey);
            if (tmp != null && !tmp.isEmpty() && Boolean.parseBoolean(tmp)) {
                return;
            }
        }

        DataField field = new DataField(key, label, unit, configKey, hideWhenNA);
        if (exampleValue != null) {
            field.currentValue = exampleValue;
        }
        fields.add(field);
        fieldMap.put(key, field);
    }

    @Override
    public void updateField(String key, String value, String naString) {
        DataField field = fieldMap.get(key);
        if (field != null) {
            field.setValueWithVisibility(value, naString);
        }
    }

    @Override
    public List<DataField> getFields() {
        return fields;
    }

    @Override
    public DataField getField(String key) {
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
    public void setFieldVisible(String key, boolean visible) {
        DataField field = fieldMap.get(key);
        if (field != null) {
            field.visible = visible;
        }
    }

    @Override
    public int visibleCount() {
        int count = 0;
        for (DataField field : fields) {
            if (field.visible)
                count++;
        }
        return count;
    }
}
