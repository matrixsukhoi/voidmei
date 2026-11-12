package prog.config;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import prog.config.SExpParser.SExp;

/**
 * 从现有设置模板的数据行（:type data）汇总出的字段目录。
 * 复用 ui_layout.cfg 的中文名/单位/精度/预览值/条件显隐，不另造字段清单。
 */
public final class MiniHudFieldCatalog {

    public static final class FieldDef {
        public final String key; // TelemetrySource getter（可含 " * 100"）
        public final String label; // 显示名
        public final String group; // 来源面板标题（右键菜单分组）
        public final String unit;
        public final int precision;
        public final String previewValue;
        public final String format;
        public final String unitSource;
        public final String precisionSource;
        public final SExp visibleWhen;
        public final SExp naWhen;

        FieldDef(String key, String label, String group, String unit, int precision, String previewValue,
                String format, String unitSource, String precisionSource, SExp visibleWhen, SExp naWhen) {
            this.key = key;
            this.label = label;
            this.group = group;
            this.unit = unit;
            this.precision = precision;
            this.previewValue = previewValue;
            this.format = format;
            this.unitSource = unitSource;
            this.precisionSource = precisionSource;
            this.visibleWhen = visibleWhen;
            this.naWhen = naWhen;
        }
    }

    private static final MiniHudFieldCatalog EMPTY = new MiniHudFieldCatalog(new LinkedHashMap<>());

    private final Map<String, FieldDef> fields;

    private MiniHudFieldCatalog(Map<String, FieldDef> fields) {
        this.fields = fields;
    }

    public static MiniHudFieldCatalog empty() {
        return EMPTY;
    }

    public static MiniHudFieldCatalog from(List<ConfigLoader.GroupConfig> configs) {
        Map<String, FieldDef> map = new LinkedHashMap<>();
        if (configs != null) {
            for (ConfigLoader.GroupConfig g : configs) {
                collect(g.rows, g.title, map);
            }
        }
        return new MiniHudFieldCatalog(map);
    }

    private static void collect(List<ConfigLoader.RowConfig> rows, String group, Map<String, FieldDef> out) {
        if (rows == null)
            return;
        for (ConfigLoader.RowConfig r : rows) {
            if ("DATA".equals(r.type) && r.property != null && !r.property.isEmpty() && !out.containsKey(r.property)) {
                String label = r.targetName != null && !r.targetName.isEmpty() ? r.targetName : r.label;
                out.put(r.property, new FieldDef(r.property, label, group, r.unit == null ? "" : r.unit,
                        r.precision, r.previewValue, r.format, r.unitSource, r.precisionSource,
                        r.visibleWhen, r.naWhen));
            }
            if (r.children != null && !r.children.isEmpty())
                collect(r.children, group, out);
        }
    }

    public List<FieldDef> all() {
        return new ArrayList<>(fields.values());
    }

    public FieldDef get(String key) {
        return fields.get(key);
    }

    public int size() {
        return fields.size();
    }
}
