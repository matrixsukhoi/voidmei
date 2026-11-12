package ui.minihud;

import java.util.Map;

/**
 * 自由画布文档 ↔ 单行紧凑串（V2）。
 *
 * 记录：W|id|type|x|y|scale|visible|locked|z|groupId|props
 *      G|id|name
 * 记录分隔 ;，字段分隔 |，props 为 k=v,k=v。
 * 所有字符串值经 MiniHudTextCodec 百分号编码，故分隔符不会出现在值里。
 * 空串/坏串/旧 V1 返回 null，由调用方回退默认布局。
 */
public final class MiniHudLayoutCodec {

    private static final String HEADER = "V2";
    private static final int WIDGET_FIELDS = 11;
    private static final int GROUP_FIELDS = 3;

    private MiniHudLayoutCodec() {
    }

    public static String encode(MiniHudDocument doc) {
        if (doc == null)
            return "";
        StringBuilder sb = new StringBuilder(HEADER);
        for (MiniHudGroup g : doc.groups) {
            sb.append(';').append('G').append('|')
                    .append(MiniHudTextCodec.encode(g.id)).append('|')
                    .append(MiniHudTextCodec.encode(g.name));
        }
        for (MiniHudWidget w : doc.widgets) {
            sb.append(';').append('W').append('|')
                    .append(MiniHudTextCodec.encode(w.id)).append('|')
                    .append(MiniHudTextCodec.encode(w.type)).append('|')
                    .append(num(w.x)).append('|')
                    .append(num(w.y)).append('|')
                    .append(num(w.scale)).append('|')
                    .append(w.visible ? '1' : '0').append('|')
                    .append(w.locked ? '1' : '0').append('|')
                    .append(w.z).append('|')
                    .append(MiniHudTextCodec.encode(w.groupId)).append('|')
                    .append(encodeProps(w));
        }
        return sb.toString();
    }

    public static MiniHudDocument decode(String text) {
        if (!isValid(text))
            return null;
        String[] records = text.split(";", -1);
        MiniHudDocument doc = new MiniHudDocument();
        for (int i = 1; i < records.length; i++) {
            String rec = records[i];
            if (rec.isEmpty())
                continue;
            String[] f = rec.split("\\|", -1);
            if (f.length == 0)
                continue;
            if ("G".equals(f[0])) {
                if (f.length != GROUP_FIELDS)
                    return null;
                String id = MiniHudTextCodec.decode(f[1]);
                String name = MiniHudTextCodec.decode(f[2]);
                doc.groups.add(new MiniHudGroup(id, name.isEmpty() ? id : name));
            } else if ("W".equals(f[0])) {
                if (f.length != WIDGET_FIELDS)
                    return null;
                MiniHudWidget w = new MiniHudWidget(MiniHudTextCodec.decode(f[1]),
                        MiniHudTextCodec.decode(f[2]));
                w.x = doubleValue(f[3], 0);
                w.y = doubleValue(f[4], 0);
                w.scale = doubleValue(f[5], 1.0);
                w.visible = "1".equals(f[6]);
                w.locked = "1".equals(f[7]);
                w.z = intValue(f[8], 0);
                String gid = MiniHudTextCodec.decode(f[9]);
                w.groupId = gid.isEmpty() ? null : gid;
                decodeProps(f[10], w);
                doc.widgets.add(w);
            } else {
                return null;
            }
        }
        doc.validate();
        return doc;
    }

    public static MiniHudDocument decodeOrDefault(String text) {
        MiniHudDocument doc = decode(text);
        return doc != null ? doc : MiniHudDefaultLayout.create();
    }

    /** 空串/旧版本/坏串都视为无效，调用方应回退默认布局。 */
    public static boolean isValid(String text) {
        if (text == null || text.isEmpty())
            return false;
        if (!text.startsWith(HEADER + ";") && !HEADER.equals(text))
            return false;
        return decodeRecords(text) != null;
    }

    private static Object decodeRecords(String text) {
        String[] records = text.split(";", -1);
        for (int i = 1; i < records.length; i++) {
            String rec = records[i];
            if (rec.isEmpty())
                continue;
            String[] f = rec.split("\\|", -1);
            if (f.length == 0)
                return null;
            if ("G".equals(f[0])) {
                if (f.length != GROUP_FIELDS)
                    return null;
            } else if ("W".equals(f[0])) {
                if (f.length != WIDGET_FIELDS)
                    return null;
            } else {
                return null;
            }
        }
        return Boolean.TRUE;
    }

    private static String encodeProps(MiniHudWidget w) {
        StringBuilder sb = new StringBuilder();
        boolean first = true;
        for (Map.Entry<String, String> en : w.props.entrySet()) {
            if (!first)
                sb.append(',');
            first = false;
            sb.append(en.getKey()).append('=').append(MiniHudTextCodec.encode(en.getValue()));
        }
        return sb.toString();
    }

    private static void decodeProps(String raw, MiniHudWidget w) {
        if (raw == null || raw.isEmpty())
            return;
        for (String kv : raw.split(",", -1)) {
            if (kv.isEmpty())
                continue;
            int eq = kv.indexOf('=');
            if (eq <= 0)
                continue;
            w.props.put(kv.substring(0, eq), MiniHudTextCodec.decode(kv.substring(eq + 1)));
        }
    }

    private static String num(double v) {
        if (v == Math.rint(v) && Math.abs(v) < 1e9)
            return Long.toString((long) v);
        return Double.toString(v);
    }

    private static double doubleValue(String s, double def) {
        try {
            return Double.parseDouble(s);
        } catch (Exception e) {
            return def;
        }
    }

    private static int intValue(String s, int def) {
        try {
            return Integer.parseInt(s);
        } catch (Exception e) {
            return def;
        }
    }
}
