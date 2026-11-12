package ui.minihud;

import java.util.LinkedHashMap;

import ui.layout.Anchor;

/**
 * MiniHUD 布局中的一个条目（组件实例）。
 * 几何单位 = hudFontSize；父子锚点沿用 ModernHUDLayoutEngine 的相对坐标语义。
 */
public final class MiniHudEntry {
    public String id;
    public String type;
    public String parentId;
    public Anchor parentAnchor = Anchor.TOP_LEFT;
    public Anchor selfAnchor = Anchor.TOP_LEFT;
    public double unitX;
    public double unitY;
    public double scale = 1.0;
    public boolean visible = true;
    public boolean locked = false;
    public int z = 0;

    /** 条目属性：字段/文本/槽位/标签/单元格等，值均为字符串（便于序列化）。 */
    public final LinkedHashMap<String, String> props = new LinkedHashMap<>();

    public MiniHudEntry(String id, String type) {
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

    public MiniHudEntry copy(String newId) {
        MiniHudEntry e = new MiniHudEntry(newId, type);
        e.parentId = parentId;
        e.parentAnchor = parentAnchor;
        e.selfAnchor = selfAnchor;
        e.unitX = unitX;
        e.unitY = unitY;
        e.scale = scale;
        e.visible = visible;
        e.locked = locked;
        e.z = z;
        e.props.putAll(props);
        return e;
    }
}
