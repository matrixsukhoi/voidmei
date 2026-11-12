package ui.minihud;

import java.util.LinkedHashMap;

/**
 * 自由画布上的一个原子组件实例。
 * 坐标是画布绝对坐标，单位 = 基准行高；没有父级/锚点/相对偏移。
 */
public final class MiniHudWidget {
    public String id;
    public String type;
    public double x;
    public double y;
    public double scale = 1.0;
    public int z;
    public boolean visible = true;
    public boolean locked = false;
    /** 可空的显式分组 id；组只影响选择/一起移动，不参与坐标换算。 */
    public String groupId;

    /** 组件属性；值统一存字符串，具体含义由组件 schema 决定。 */
    public final LinkedHashMap<String, String> props = new LinkedHashMap<>();

    public MiniHudWidget(String id, String type) {
        this.id = id;
        this.type = type;
    }

    public String get(String key, String def) {
        String v = props.get(key);
        return v != null ? v : def;
    }

    public boolean getBool(String key, boolean def) {
        String v = props.get(key);
        if (v == null || v.isEmpty())
            return def;
        return "1".equals(v) || "true".equalsIgnoreCase(v);
    }

    public int getInt(String key, int def) {
        try {
            String v = props.get(key);
            return (v == null || v.isEmpty()) ? def : Integer.parseInt(v);
        } catch (NumberFormatException e) {
            return def;
        }
    }

    public double getDouble(String key, double def) {
        try {
            String v = props.get(key);
            return (v == null || v.isEmpty()) ? def : Double.parseDouble(v);
        } catch (NumberFormatException e) {
            return def;
        }
    }

    public void set(String key, String value) {
        if (value == null)
            props.remove(key);
        else
            props.put(key, value);
    }

    public MiniHudWidget copy(String newId) {
        MiniHudWidget w = new MiniHudWidget(newId, type);
        w.x = x;
        w.y = y;
        w.scale = scale;
        w.z = z;
        w.visible = visible;
        w.locked = locked;
        w.groupId = groupId;
        w.props.putAll(props);
        return w;
    }
}
