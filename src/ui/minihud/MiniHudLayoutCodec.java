package ui.minihud;

import java.util.Map;

import ui.layout.Anchor;

/**
 * 布局文档 ↔ 单行紧凑串。
 * 格式：V1;id|type|parent|parentAnchor|selfAnchor|x|y|scale|visible|locked|z|props
 * - 记录分隔 ;，字段分隔 |
 * - props 为 k=v,k=v（cells 的值内部用 ~ 分格、: 分列，子值已百分号编码）
 * - 所有字符串值经 MiniHudTextCodec 编码，故分隔符永不出现在值里
 * 空串/坏串返回 null，由调用方回退默认布局。
 */
public final class MiniHudLayoutCodec {

    private static final String HEADER = "V1";
    private static final int FIELD_COUNT = 12;

    private MiniHudLayoutCodec() {
    }

    public static String encode(MiniHudLayoutDoc doc) {
        if (doc == null)
            return "";
        StringBuilder sb = new StringBuilder(HEADER);
        for (MiniHudEntry e : doc.entries) {
            sb.append(';');
            sb.append(enc(e.id)).append('|')
                    .append(enc(e.type)).append('|')
                    .append(enc(e.parentId)).append('|')
                    .append(e.parentAnchor.name()).append('|')
                    .append(e.selfAnchor.name()).append('|')
                    .append(num(e.unitX)).append('|')
                    .append(num(e.unitY)).append('|')
                    .append(num(e.scale)).append('|')
                    .append(e.visible ? '1' : '0').append('|')
                    .append(e.locked ? '1' : '0').append('|')
                    .append(e.z).append('|')
                    .append(encodeProps(e));
        }
        return sb.toString();
    }

    public static MiniHudLayoutDoc decode(String text) {
        if (text == null || text.isEmpty())
            return null;
        String[] records = text.split(";", -1);
        if (records.length == 0 || !HEADER.equals(records[0]))
            return null;
        MiniHudLayoutDoc doc = new MiniHudLayoutDoc();
        for (int i = 1; i < records.length; i++) {
            String rec = records[i];
            if (rec.isEmpty())
                continue;
            String[] f = rec.split("\\|", -1);
            if (f.length != FIELD_COUNT)
                return null;
            MiniHudEntry e = new MiniHudEntry(MiniHudTextCodec.decode(f[0]), MiniHudTextCodec.decode(f[1]));
            String parent = MiniHudTextCodec.decode(f[2]);
            e.parentId = parent.isEmpty() ? null : parent;
            e.parentAnchor = anchor(f[3], Anchor.TOP_LEFT);
            e.selfAnchor = anchor(f[4], Anchor.TOP_LEFT);
            e.unitX = doubleValue(f[5], 0);
            e.unitY = doubleValue(f[6], 0);
            e.scale = doubleValue(f[7], 1.0);
            e.visible = "1".equals(f[8]);
            e.locked = "1".equals(f[9]);
            e.z = intValue(f[10], 0);
            decodeProps(f[11], e);
            doc.entries.add(e);
        }
        doc.validate();
        return doc;
    }

    public static MiniHudLayoutDoc decodeOrDefault(String text) {
        MiniHudLayoutDoc doc = decode(text);
        return doc != null ? doc : MiniHudDefaultLayout.create();
    }

    public static String encodeProps(MiniHudEntry e) {
        StringBuilder sb = new StringBuilder();
        boolean first = true;
        for (Map.Entry<String, String> en : e.props.entrySet()) {
            if (!first)
                sb.append(',');
            first = false;
            sb.append(en.getKey()).append('=');
            // cells 的值是嵌套结构（已编码子值），不能再整体编码，否则分隔符丢失
            if ("cells".equals(en.getKey())) {
                sb.append(en.getValue());
            } else {
                sb.append(MiniHudTextCodec.encode(en.getValue()));
            }
        }
        return sb.toString();
    }

    public static void decodeProps(String raw, MiniHudEntry e) {
        if (raw == null || raw.isEmpty())
            return;
        for (String kv : raw.split(",", -1)) {
            if (kv.isEmpty())
                continue;
            int eq = kv.indexOf('=');
            if (eq <= 0)
                continue;
            String k = kv.substring(0, eq);
            String v = kv.substring(eq + 1);
            e.props.put(k, "cells".equals(k) ? v : MiniHudTextCodec.decode(v));
        }
    }

    private static String enc(String s) {
        return MiniHudTextCodec.encode(s);
    }

    private static String num(double v) {
        // 避免 1.0 存成 "1.0" 的冗长，且与 Locale 无关
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

    private static Anchor anchor(String s, Anchor def) {
        try {
            return Anchor.valueOf(s);
        } catch (Exception e) {
            return def;
        }
    }
}
