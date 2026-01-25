package prog.config;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;

import prog.config.SExpParser.SAtom;
import prog.config.SExpParser.SExp;
import prog.config.SExpParser.SList;

/**
 * Loader for dynamic overlay configuration files.
 * Refactored to use S-Expression (Lisp-like) syntax.
 */
public class ConfigLoader {

    public static class RowConfig {
        public String label;
        public String formula; // Kept for reflection paths (e.g. S.rpm)
        public String format;
        public Object value = true; // Typed value (Boolean, Integer, String)
        public Object defaultValue = null; // Default value for reset
        public String fgColor = null; // Foreground color (e.g. for buttons)
        public String desc = null; // Help description tooltip

        // Extended fields for control-type rows
        public String type = "DATA"; // DATA, HEADER, SLIDER, COMBO, SWITCH, BUTTON
        public String property = null; // Bound GroupConfig property (e.g., "fontSize")
        public int minVal = 0; // For SLIDER
        public int maxVal = 100; // For SLIDER
        public int groupColumns = 0; // For HEADER: specify columns for this group
        public List<RowConfig> children = new ArrayList<>();

        public RowConfig(String label, String formula, String format) {
            this.label = label;
            this.formula = formula;
            this.format = format;
        }

        public int getInt() {
            if (value instanceof Number)
                return ((Number) value).intValue();
            try {
                return Integer.parseInt(value.toString());
            } catch (Exception e) {
                return 0;
            }
        }

        public boolean getBool() {
            if (value instanceof Boolean)
                return (Boolean) value;
            return Boolean.parseBoolean(value.toString());
        }

        public String getStr() {
            return String.valueOf(value);
        }
    }

    public static class GroupConfig {
        public String title;
        public double x = 0.1;
        public double y = 0.1;
        public int alpha = 150;
        public int hotkey = 0; // 0 means no hotkey
        public boolean visible = false; // Default to false (hidden)
        public String fontName = null;
        public int fontSize = 0; // Font size adjustment (-6 to +20)
        public int columns = 2; // Number of columns for layout
        public int panelColumns = 2; // Number of columns for SETTINGS PANEL layout
        public String switchKey = null; // Config key for visibility switch (e.g. "flightInfoSwitch")
        public List<RowConfig> rows = new ArrayList<>();

        public GroupConfig(String title) {
            this.title = title;
        }
    }

    /**
     * Attempts to resolve key code from string (either "P" or "25")
     */
    private static int getKeyCodeFromText(String text) {
        if (text == null || text.trim().isEmpty())
            return 0;

        String t = text.trim();
        // 1. Try numeric
        try {
            return Integer.parseInt(t);
        } catch (NumberFormatException e) {
            // 2. Brute force lookup in common JNativeHook VC codes (typically < 256)
            for (int i = 1; i < 256; i++) {
                if (NativeKeyEvent.getKeyText(i).equalsIgnoreCase(t)) {
                    return i;
                }
            }
        }
        return 0;
    }

    // --- S-Expression Parsing Helpers ---

    private static String getKeywordString(SList list, String keyword, String def) {
        for (int i = 0; i < list.children.size() - 1; i++) {
            SExp curr = list.children.get(i);
            if (curr.isAtom() && curr.asAtom().isKeyword() && curr.asAtom().getString().equalsIgnoreCase(keyword)) {
                SExp next = list.children.get(i + 1);
                if (next.isAtom())
                    return next.asAtom().getString();
            }
        }
        return def;
    }

    private static double getKeywordDouble(SList list, String keyword, double def) {
        for (int i = 0; i < list.children.size() - 1; i++) {
            SExp curr = list.children.get(i);
            if (curr.isAtom() && curr.asAtom().isKeyword() && curr.asAtom().getString().equalsIgnoreCase(keyword)) {
                SExp next = list.children.get(i + 1);
                if (next.isAtom())
                    return next.asAtom().getDouble();
            }
        }
        return def;
    }

    private static int getKeywordInt(SList list, String keyword, int def) {
        return (int) getKeywordDouble(list, keyword, def);
    }

    private static boolean getKeywordBool(SList list, String keyword, boolean def) {
        for (int i = 0; i < list.children.size() - 1; i++) {
            SExp curr = list.children.get(i);
            if (curr.isAtom() && curr.asAtom().isKeyword() && curr.asAtom().getString().equalsIgnoreCase(keyword)) {
                SExp next = list.children.get(i + 1);
                if (next.isAtom())
                    return next.asAtom().getBool();
            }
        }
        return def;
    }

    public static List<GroupConfig> loadConfig(String path) {
        List<GroupConfig> groups = new ArrayList<>();
        File file = new File(path);

        if (!file.exists())
            return groups;

        try {
            // Read file content
            StringBuilder sb = new StringBuilder();
            try (BufferedReader br = new BufferedReader(
                    new InputStreamReader(new FileInputStream(file), StandardCharsets.UTF_8))) {
                String line;
                while ((line = br.readLine()) != null) {
                    sb.append(line).append("\n");
                }
            }

            String content = sb.toString();
            // Legacy compatibility: Check if it starts with '[' (INI format)
            if (content.trim().startsWith("[")) {
                // Fallback to legacy parser if needed, or just warn.
                // For now, let's assume valid S-Expr input or strict migration.
                // If you needed legacy support, we'd paste the old parser code here.
            }

            SExpParser parser = new SExpParser();
            List<SExp> panels = parser.parse(content);

            for (SExp exp : panels) {
                if (!exp.isList())
                    continue;
                SList panelExp = exp.asList();
                if (panelExp.children.isEmpty() || !isSymbol(panelExp.children.get(0), "panel"))
                    continue;

                // (panel "Title" :k v ...)
                String title = "Unknown";
                if (panelExp.children.size() > 1 && panelExp.children.get(1).isAtom()) {
                    title = panelExp.children.get(1).asAtom().getString();
                }

                GroupConfig group = new GroupConfig(title);
                group.x = getKeywordDouble(panelExp, ":x", 0.1);
                group.y = getKeywordDouble(panelExp, ":y", 0.1);

                // Legacy coord conversion
                if (group.x > 2.0)
                    group.x /= java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
                if (group.y > 2.0)
                    group.y /= java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;

                group.alpha = getKeywordInt(panelExp, ":alpha", 150);
                group.visible = getKeywordBool(panelExp, ":visible", false);
                group.fontName = getKeywordString(panelExp, ":font", null);
                group.fontSize = getKeywordInt(panelExp, ":font-size", 0);
                group.columns = getKeywordInt(panelExp, ":columns", 2);
                group.panelColumns = getKeywordInt(panelExp, ":panel-columns", 2);
                group.switchKey = getKeywordString(panelExp, ":switch-key", null);

                String hotkeyStr = getKeywordString(panelExp, ":hotkey", null);
                if (hotkeyStr != null) {
                    group.hotkey = getKeyCodeFromText(hotkeyStr);
                }

                // Process children (items and groups)
                processPanelChildren(group.rows, panelExp);

                groups.add(group);
            }

        } catch (Exception e) {
            e.printStackTrace();
        }
        return groups;
    }

    private static void processPanelChildren(List<RowConfig> targetList, SList parentList) {
        for (SExp child : parentList.children) {
            if (!child.isList())
                continue;
            SList list = child.asList();
            if (list.children.isEmpty())
                continue;

            SExp head = list.children.get(0);
            if (!head.isAtom())
                continue;
            String type = head.asAtom().getString();

            if ("group".equalsIgnoreCase(type)) {
                // (group "Label" :k v ... children...)
                // Create a HEADER row
                String label = "Group";
                if (list.children.size() > 1)
                    label = list.children.get(1).asAtom().getString();

                RowConfig headerRow = new RowConfig(label, null, "%s");
                headerRow.type = "HEADER";
                headerRow.groupColumns = getKeywordInt(list, ":column", 0);
                headerRow.value = true;

                // Recurse for group children
                processPanelChildren(headerRow.children, list);

                targetList.add(headerRow);

            } else if ("item".equalsIgnoreCase(type)) {
                // (item "Label" :k v ...)
                String label = "Item";
                if (list.children.size() > 1)
                    label = list.children.get(1).asAtom().getString();

                RowConfig row = new RowConfig(label, null, "%s");
                String rawType = getKeywordString(list, ":type", "DATA");

                // Map logical types to internal types
                row.type = rawType.toUpperCase().replace("-", "_"); // switch-inv -> SWITCH_INV

                row.property = getKeywordString(list, ":target", null);
                row.format = getKeywordString(list, ":format", "%s");

                // Special handling for COMBO source and List paths which use format field
                // internally
                String source = getKeywordString(list, ":source", null);
                if (source != null)
                    row.format = source;

                // Value extraction
                // value defaults to true for switches, 0 for slider, null/string for others
                // But we need to check the SExp type
                row.value = extractValue(list, ":value");
                row.defaultValue = extractValue(list, ":default");
                row.fgColor = getKeywordString(list, ":fgcolor", null);
                row.desc = getKeywordString(list, ":desc", null);

                if (row.value == null) {
                    if (row.type.contains("SWITCH"))
                        row.value = true;
                    if (row.type.equals("SLIDER"))
                        row.value = 0;
                }

                // If default is missing, fallback to initial value
                if (row.defaultValue == null && !"BUTTON".equals(row.type)) {
                    row.defaultValue = row.value;
                }

                row.minVal = getKeywordInt(list, ":min", 0);
                row.maxVal = getKeywordInt(list, ":max", 100);

                // Compatibility mapping for 'formula' field
                // Legacy system used 'formula' for Reflection variable path (DATA rows)
                if ("DATA".equals(row.type)) {
                    row.formula = row.property;
                } else {
                    // For controls, formula isn't strictly needed by runtime if type is set,
                    // but let's be safe if something uses it for debugging or fallback
                    row.formula = row.property;
                }

                targetList.add(row);
            }
        }
    }

    private static Object extractValue(SList list, String keyword) {
        for (int i = 0; i < list.children.size() - 1; i++) {
            SExp curr = list.children.get(i);
            if (curr.isAtom() && curr.asAtom().isKeyword() && curr.asAtom().getString().equalsIgnoreCase(keyword)) {
                SAtom val = list.children.get(i + 1).asAtom();
                if (val.type == SAtom.AtomType.BOOLEAN)
                    return val.getBool();
                if (val.type == SAtom.AtomType.NUMBER) {
                    double d = val.getDouble();
                    if (d == (int) d)
                        return (int) d;
                    return d;
                }
                return val.getString();
            }
        }
        return null;
    }

    private static boolean isSymbol(SExp exp, String name) {
        return exp.isAtom() && exp.asAtom().isSymbol() && exp.asAtom().getString().equalsIgnoreCase(name);
    }

    // --- Serialization ---

    public static void saveConfig(String path, List<GroupConfig> groups) {
        try (PrintWriter pw = new PrintWriter(
                new OutputStreamWriter(new FileOutputStream(path), StandardCharsets.UTF_8))) {

            for (GroupConfig group : groups) {
                pw.print("(panel ");
                pw.print(quote(group.title));
                pw.println();

                String indent = "  "; // 2 spaces base indent for panel attributes as per sample
                writeAttrLine(pw, indent, ":x", String.format("%.4f", group.x));
                writeAttrLine(pw, indent, ":y", String.format("%.4f", group.y));
                writeAttrLine(pw, indent, ":alpha", group.alpha);
                writeAttrLine(pw, indent, ":visible", group.visible);
                if (group.switchKey != null)
                    writeAttrLine(pw, indent, ":switch-key", group.switchKey);
                writeAttrLine(pw, indent, ":font", group.fontName);
                if (group.hotkey != 0)
                    writeAttrLine(pw, indent, ":hotkey", NativeKeyEvent.getKeyText(group.hotkey));
                if (group.fontSize != 0)
                    writeAttrLine(pw, indent, ":font-size", group.fontSize);
                if (group.columns != 2)
                    writeAttrLine(pw, indent, ":columns", group.columns);
                if (group.panelColumns != 0)
                    writeAttrLine(pw, indent, ":panel-columns", group.panelColumns);

                pw.println();
                pw.println();

                writeChildren(pw, group.rows, "  ");

                pw.println(")"); // Close panel
                pw.println();
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    private static void writeChildren(PrintWriter pw, List<RowConfig> rows, String indent) {
        for (RowConfig row : rows) {
            if ("HEADER".equals(row.type)) {
                pw.print(indent + "(group ");
                pw.print(quote(row.label));
                if (row.groupColumns > 0) {
                    pw.print(" :column " + row.groupColumns);
                }
                pw.println();

                // Recurse for children
                writeChildren(pw, row.children, indent + "  ");

                pw.println(indent + ")"); // Close group
            } else {
                // Item
                pw.print(indent + "(item ");
                pw.print(quote(row.label));

                String lispType = row.type.toLowerCase().replace("_", "-");
                pw.print(" :type " + lispType);

                if (row.property != null)
                    pw.print(" :target " + quote(row.property));
                else if (row.formula != null && "DATA".equals(row.type))
                    pw.print(" :target " + quote(row.formula));

                // Type specific fields
                if ("slider".equals(lispType)) {
                    pw.print(" :min " + row.minVal + " :max " + row.maxVal);
                }

                if ("combo".equals(lispType) || "filelist".equals(lispType) || "fmlist".equals(lispType)) {
                    pw.print(" :source " + quote(row.format));
                } else {
                    if (!"%s".equals(row.format))
                        pw.print(" :format " + quote(row.format));
                }

                // Value is last
                if (!"button".equals(lispType)) {
                    pw.print(" :value " + serializeAtom(row.value));
                }
                if (row.defaultValue != null && !"button".equals(lispType)) {
                    pw.print(" :default " + serializeAtom(row.defaultValue));
                }
                if (row.desc != null) {
                    pw.print(" :desc " + quote(row.desc));
                }

                pw.println(")");
            }
        }
    }

    private static String quote(String s) {
        if (s == null)
            return "\"\"";
        return "\"" + s.replace("\"", "\\\"") + "\"";
    }

    private static String serializeAtom(Object val) {
        if (val == null)
            return "\"\"";
        if (val instanceof String) {
            String s = (String) val;
            if (isNumeric(s))
                return s;
            return quote(s);
        }
        return String.valueOf(val);
    }

    private static boolean isNumeric(String s) {
        if (s == null || s.isEmpty())
            return false;
        int dotCount = 0;
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c == '-') {
                if (i != 0)
                    return false;
            } else if (c == '.') {
                if (++dotCount > 1)
                    return false;
            } else if (!Character.isDigit(c)) {
                return false;
            }
        }
        return true;
    }

    private static void writeAttrLine(PrintWriter pw, String indent, String key, Object val) {
        pw.print(indent + key + " ");
        pw.println(serializeAtom(val));
    }
}
