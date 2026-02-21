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
        public String targetName = null; // Display name for overlay if different from label
        public String formula; // Kept for reflection paths (e.g. S.rpm)
        public String format;
        public String unit = ""; // Unit string (e.g. "Hp")
        public Object value = true; // Typed value (Boolean, Integer, String)
        public Object defaultValue = null; // Default value for reset
        public String fgColor = null; // Foreground color (e.g. for buttons)
        public String desc = null; // Help description tooltip
        public String descImg = null; // Help image path (relative to project root)
        public String previewValue = null; // Default value for UI preview/placeholder
        public boolean hideWhenZero = false; // Hide if value is zero
        public int precision = 0; // Number of decimal places
        public String unitSource = null; // Method name for dynamic unit (e.g., "getManifoldPressureDisplayUnit")
        public String precisionSource = null; // Method name for dynamic precision (e.g., "getManifoldPressureDisplayPrecision")

        // Extended fields for control-type rows
        public String type = "DATA"; // DATA, HEADER, SLIDER, COMBO, SWITCH, BUTTON
        public String property = null; // Bound GroupConfig property (e.g., "fontSize")
        public int minVal = 0; // For SLIDER
        public int maxVal = 100; // For SLIDER
        public int groupColumns = 0; // For HEADER: specify columns for this group
        public List<RowConfig> children = new ArrayList<>();

        // Source text preservation fields (for format-preserving saves)
        public String sourceText = null;      // Original item text from source file
        public int sourceStart = -1;          // Start position in source file
        public int sourceEnd = -1;            // End position in source file
        public boolean modified = false;      // Whether value was modified by user
        public boolean hasExplicitDefault = false; // Whether source had :default attribute

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

        // Source text preservation fields (for format-preserving saves)
        public String sourceText = null;      // Original panel text from source file
        public int sourceStart = -1;          // Start position in source file
        public int sourceEnd = -1;            // End position in source file

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
        return loadConfigWithContent(path, null);
    }

    /**
     * Loads configuration, optionally storing source content for format-preserving saves.
     *
     * @param path Path to config file
     * @param contentHolder If non-null, will be set to [0]=source content for later use
     * @return List of GroupConfig
     */
    public static List<GroupConfig> loadConfigWithContent(String path, String[] contentHolder) {
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

            // Normalize line endings: remove all \r to ensure Unix LF format
            // This fixes ^M characters appearing when source file has CRLF line endings
            String content = sb.toString().replace("\r", "");

            // Store content for caller if requested
            if (contentHolder != null && contentHolder.length > 0) {
                contentHolder[0] = content;
            }

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

                // Store source text info for format-preserving saves
                group.sourceStart = panelExp.sourceStart;
                group.sourceEnd = panelExp.sourceEnd;
                if (group.sourceStart >= 0 && group.sourceEnd > group.sourceStart) {
                    group.sourceText = content.substring(group.sourceStart, group.sourceEnd);
                }

                // Process children (items and groups)
                processPanelChildren(group.rows, panelExp, content);

                groups.add(group);
            }

        } catch (Exception e) {
            e.printStackTrace();
        }
        return groups;
    }

    private static void processPanelChildren(List<RowConfig> targetList, SList parentList, String content) {
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

                // Store source text info for format-preserving saves
                headerRow.sourceStart = list.sourceStart;
                headerRow.sourceEnd = list.sourceEnd;
                if (content != null && headerRow.sourceStart >= 0 && headerRow.sourceEnd > headerRow.sourceStart) {
                    headerRow.sourceText = content.substring(headerRow.sourceStart, headerRow.sourceEnd);
                }

                // Recurse for group children
                processPanelChildren(headerRow.children, list, content);

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
                row.unit = getKeywordString(list, ":unit", "");
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

                // Check if :default was explicitly present in source
                row.hasExplicitDefault = hasKeyword(list, ":default");
                if (row.hasExplicitDefault) {
                    row.defaultValue = extractValue(list, ":default");
                }

                row.fgColor = getKeywordString(list, ":fgcolor", null);
                row.desc = getKeywordString(list, ":desc", null);
                row.descImg = getKeywordString(list, ":desc-img", null);
                row.previewValue = getKeywordString(list, ":preview-value", null);
                row.hideWhenZero = getKeywordBool(list, ":hide-when-zero", false);
                row.precision = getKeywordInt(list, ":precision", 0);
                row.unitSource = getKeywordString(list, ":unit-source", null);
                row.precisionSource = getKeywordString(list, ":precision-source", null);
                row.targetName = getKeywordString(list, ":target-name", null);

                if (row.value == null) {
                    if (row.type.contains("SWITCH"))
                        row.value = true;
                    if (row.type.equals("SLIDER"))
                        row.value = 0;
                }

                // If default is missing, fallback to initial value
                // But only set defaultValue internally if not INFO type (for runtime reset logic)
                if (row.defaultValue == null && !"BUTTON".equals(row.type) && !"INFO".equals(row.type)) {
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

                // Store source text info for format-preserving saves
                row.sourceStart = list.sourceStart;
                row.sourceEnd = list.sourceEnd;
                if (content != null && row.sourceStart >= 0 && row.sourceEnd > row.sourceStart) {
                    row.sourceText = content.substring(row.sourceStart, row.sourceEnd);
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

    /**
     * Checks if a keyword exists in an S-expression list.
     */
    private static boolean hasKeyword(SList list, String keyword) {
        for (SExp child : list.children) {
            if (child.isAtom() && child.asAtom().isKeyword()
                    && child.asAtom().getString().equalsIgnoreCase(keyword)) {
                return true;
            }
        }
        return false;
    }

    // --- Serialization ---

    /**
     * Saves configuration preserving original formatting for unmodified items.
     * Always outputs Unix line endings (\n) regardless of platform.
     *
     * @param path Path to save to
     * @param groups Configuration groups to save
     * @param originalContent Original file content (for extracting unmodified text)
     */
    public static void saveConfigPreserving(String path, List<GroupConfig> groups, String originalContent) {
        try (PrintWriter pw = new PrintWriter(
                new OutputStreamWriter(new FileOutputStream(path), StandardCharsets.UTF_8))) {

            for (int gi = 0; gi < groups.size(); gi++) {
                GroupConfig group = groups.get(gi);

                // Check if the entire panel can be output as-is
                if (!hasModifiedRows(group.rows) && group.sourceText != null && originalContent != null) {
                    // Whole panel unmodified - output original text
                    pw.print(group.sourceText);
                } else {
                    // Panel has modifications - regenerate with selective preservation
                    writeGroupPreserving(pw, group, originalContent);
                }

                writeln(pw);
                if (gi < groups.size() - 1) {
                    writeln(pw);
                }
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    /**
     * Writes a Unix line ending (\n) regardless of platform.
     * This ensures consistent output across Windows/Linux/Mac.
     */
    private static void writeln(PrintWriter pw) {
        pw.print('\n');
    }

    /**
     * Writes a string followed by a Unix line ending (\n).
     */
    private static void writeln(PrintWriter pw, String s) {
        pw.print(s);
        pw.print('\n');
    }

    /**
     * Checks if any rows (recursively) have been modified.
     */
    private static boolean hasModifiedRows(List<RowConfig> rows) {
        for (RowConfig row : rows) {
            if (row.modified || row.sourceText == null) {
                return true;
            }
            if (row.children != null && !row.children.isEmpty()) {
                if (hasModifiedRows(row.children)) {
                    return true;
                }
            }
        }
        return false;
    }

    /**
     * Writes a group/panel with selective preservation of unmodified items.
     */
    private static void writeGroupPreserving(PrintWriter pw, GroupConfig group, String originalContent) {
        pw.print("(panel ");
        pw.print(quote(group.title));
        writeln(pw);

        String indent = "  ";
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

        writeln(pw);
        writeln(pw);

        writeChildrenPreserving(pw, group.rows, indent, originalContent);

        writeln(pw, ")");
    }

    /**
     * Writes children with selective preservation of unmodified items.
     */
    private static void writeChildrenPreserving(PrintWriter pw, List<RowConfig> rows, String indent, String originalContent) {
        for (RowConfig row : rows) {
            if ("HEADER".equals(row.type)) {
                // Check if entire group can be preserved
                if (!row.modified && row.sourceText != null && !hasModifiedRows(row.children) && originalContent != null) {
                    // Output original group text with adjusted indentation
                    writeln(pw, adjustIndent(row.sourceText, indent));
                } else {
                    // Regenerate group
                    pw.print(indent + "(group ");
                    pw.print(quote(row.label));
                    if (row.groupColumns > 0) {
                        pw.print(" :column " + row.groupColumns);
                    }
                    writeln(pw);
                    writeChildrenPreserving(pw, row.children, indent + "  ", originalContent);
                    writeln(pw, indent + ")");
                }
            } else {
                // Item
                if (!row.modified && row.sourceText != null && originalContent != null) {
                    // Output original item text with adjusted indentation
                    writeln(pw, adjustIndent(row.sourceText, indent));
                } else {
                    // Regenerate item
                    writeItemRegenerated(pw, row, indent);
                }
            }
        }
    }

    /**
     * Adjusts indentation of source text to match target indent level.
     */
    private static String adjustIndent(String sourceText, String targetIndent) {
        if (sourceText == null || sourceText.isEmpty()) {
            return "";
        }

        // Find the original indentation (leading whitespace of first line)
        int firstNonSpace = 0;
        for (int i = 0; i < sourceText.length(); i++) {
            char c = sourceText.charAt(i);
            if (c != ' ' && c != '\t') {
                firstNonSpace = i;
                break;
            }
        }

        // Replace leading whitespace with target indent and normalize line endings
        String trimmed = sourceText.substring(firstNonSpace);

        // Handle multi-line source text by re-indenting each line
        // Use \r?\n to handle both CRLF and LF line endings
        String[] lines = trimmed.split("\r?\n");
        if (lines.length == 1) {
            return targetIndent + trimmed.trim();
        }

        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < lines.length; i++) {
            String line = lines[i];
            // Trim leading whitespace and add target indent
            String trimmedLine = line.replaceFirst("^\\s+", "");
            if (!trimmedLine.isEmpty()) {
                if (i > 0) {
                    sb.append("\n");
                }
                // For continuation lines, add extra indent
                String lineIndent = (i == 0) ? targetIndent : targetIndent + "  ";
                sb.append(lineIndent).append(trimmedLine);
            }
        }
        return sb.toString();
    }

    /**
     * Regenerates an item that was modified, respecting hasExplicitDefault.
     */
    private static void writeItemRegenerated(PrintWriter pw, RowConfig row, String indent) {
        pw.print(indent + "(item ");
        pw.print(quote(row.label));

        String lispType = row.type.toLowerCase().replace("_", "-");
        pw.print(" :type " + lispType);

        if (row.property != null)
            pw.print(" :target " + quote(row.property));

        if (row.targetName != null)
            pw.print(" :target-name " + quote(row.targetName));

        if (row.unit != null && !row.unit.isEmpty())
            pw.print(" :unit " + quote(row.unit));

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

        if (row.previewValue != null) {
            pw.print(" :preview-value " + quote(row.previewValue));
        }

        // Value - skip for button and info types
        if (!"button".equals(lispType) && !"info".equals(lispType)) {
            pw.print(" :value " + serializeAtom(row.value));
        }

        // Only output :default if it was explicitly present in the original
        if (row.hasExplicitDefault && row.defaultValue != null && !"button".equals(lispType)) {
            pw.print(" :default " + serializeAtom(row.defaultValue));
        }

        if (row.desc != null) {
            pw.print(" :desc " + quote(row.desc));
        }
        if (row.descImg != null) {
            pw.print(" :desc-img " + quote(row.descImg));
        }
        if (row.hideWhenZero) {
            pw.print(" :hide-when-zero true");
        }
        if (row.precision != 0) {
            pw.print(" :precision " + row.precision);
        }
        if (row.unitSource != null) {
            pw.print(" :unit-source " + quote(row.unitSource));
        }
        if (row.precisionSource != null) {
            pw.print(" :precision-source " + quote(row.precisionSource));
        }
        if (row.fgColor != null) {
            pw.print(" :fgcolor " + quote(row.fgColor));
        }

        writeln(pw, ")");
    }

    public static void saveConfig(String path, List<GroupConfig> groups) {
        try (PrintWriter pw = new PrintWriter(
                new OutputStreamWriter(new FileOutputStream(path), StandardCharsets.UTF_8))) {

            for (GroupConfig group : groups) {
                pw.print("(panel ");
                pw.print(quote(group.title));
                writeln(pw);

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

                writeln(pw);
                writeln(pw);

                writeChildren(pw, group.rows, "  ");

                writeln(pw, ")"); // Close panel
                writeln(pw);
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
                writeln(pw);

                // Recurse for children
                writeChildren(pw, row.children, indent + "  ");

                writeln(pw, indent + ")"); // Close group
            } else {
                // Item
                pw.print(indent + "(item ");
                pw.print(quote(row.label));

                String lispType = row.type.toLowerCase().replace("_", "-");
                pw.print(" :type " + lispType);

                if (row.property != null)
                    pw.print(" :target " + quote(row.property));

                if (row.unit != null && !row.unit.isEmpty())
                    pw.print(" :unit " + quote(row.unit));

                if (row.formula != null && "DATA".equals(row.type))
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
                if (row.descImg != null) {
                    pw.print(" :desc-img " + quote(row.descImg));
                }
                if (row.previewValue != null) {
                    pw.print(" :preview-value " + quote(row.previewValue));
                }
                if (row.hideWhenZero) {
                    pw.print(" :hide-when-zero true");
                }
                if (row.precision != 0) {
                    pw.print(" :precision " + row.precision);
                }
                if (row.unitSource != null) {
                    pw.print(" :unit-source " + quote(row.unitSource));
                }
                if (row.precisionSource != null) {
                    pw.print(" :precision-source " + quote(row.precisionSource));
                }
                if (row.targetName != null) {
                    pw.print(" :target-name " + quote(row.targetName));
                }
                if (row.fgColor != null) {
                    pw.print(" :fgcolor " + quote(row.fgColor));
                }

                writeln(pw, ")");
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
        writeln(pw, serializeAtom(val));
    }
}
