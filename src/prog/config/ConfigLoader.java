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

/**
 * Loader for dynamic overlay configuration files.
 */
public class ConfigLoader {

    public static class RowConfig {
        public String label;
        public String formula;
        public String format;
        public boolean visible = true;

        // Extended fields for control-type rows
        public String type = "DATA"; // DATA, HEADER, SLIDER, COMBO, SWITCH
        public String property = null; // Bound GroupConfig property (e.g., "fontSize")
        public int minVal = 0; // For SLIDER
        public int maxVal = 100; // For SLIDER
        public String defaultValue = null; // Default/current value from config

        public RowConfig(String label, String formula, String format) {
            this.label = label;
            this.formula = formula;
            this.format = format;
        }
    }

    public static class GroupConfig {
        public String title;
        public double x = 0.1;
        public double y = 0.1;
        public int alpha = 150;
        public int hotkey = 0; // 0 means no hotkey
        public boolean visible = false; // Default to false (hidden)
        public String fontName = "Sarasa Mono SC";
        public int fontSize = 0; // Font size adjustment (-6 to +20)
        public int columns = 2; // Number of columns for layout
        public int panelColumns = 2; // Number of columns for SETTINGS PANEL layout
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

    public static List<GroupConfig> loadConfig(String path) {
        List<GroupConfig> groups = new ArrayList<>();
        File file = new File(path);

        if (!file.exists())
            return groups;

        try (BufferedReader br = new BufferedReader(
                new InputStreamReader(new FileInputStream(file), StandardCharsets.UTF_8))) {
            String line;
            GroupConfig currentGroup = null;
            boolean inCommentedSection = false;

            while ((line = br.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty())
                    continue;

                // Skip single-line comments
                if (line.startsWith("#") && !line.startsWith("#[")) {
                    continue;
                }

                // Check for commented section header: #[SectionName]
                if (line.startsWith("#[") && line.endsWith("]")) {
                    inCommentedSection = true;
                    currentGroup = null;
                    continue;
                }

                // Check for regular section header: [SectionName]
                if (line.startsWith("[") && line.endsWith("]")) {
                    inCommentedSection = false;
                    String title = line.substring(1, line.length() - 1).trim();
                    currentGroup = new GroupConfig(title);
                    groups.add(currentGroup);
                    continue;
                }

                // Skip all lines inside a commented section
                if (inCommentedSection) {
                    continue;
                }

                if (line.startsWith("X=")) {
                    if (currentGroup != null)
                        try {
                            double val = Double.parseDouble(line.substring(2).trim());
                            // 自动兼容旧版配置：如果数值大于 2.0，说明是旧的绝对像素坐标，自动转换为相对比例
                            if (val > 2.0) {
                                int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
                                val = val / screenW;
                            }
                            currentGroup.x = val;
                        } catch (Exception e) {
                        }
                } else if (line.startsWith("Y=")) {
                    if (currentGroup != null)
                        try {
                            double val = Double.parseDouble(line.substring(2).trim());
                            if (val > 2.0) {
                                int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
                                val = val / screenH;
                            }
                            currentGroup.y = val;
                        } catch (Exception e) {
                        }
                } else if (line.startsWith("Alpha=")) {
                    if (currentGroup != null)
                        try {
                            currentGroup.alpha = Integer.parseInt(line.substring(6).trim());
                        } catch (Exception e) {
                        }
                } else if (line.startsWith("Hotkey=")) {
                    if (currentGroup != null) {
                        currentGroup.hotkey = getKeyCodeFromText(line.substring(7));
                    }
                } else if (line.startsWith("Visible=")) {
                    if (currentGroup != null)
                        currentGroup.visible = line.substring(8).trim().equalsIgnoreCase("true");
                } else if (line.startsWith("Font=")) {
                    if (currentGroup != null)
                        currentGroup.fontName = line.substring(5).trim();
                } else if (line.startsWith("FontSize=")) {
                    if (currentGroup != null)
                        try {
                            currentGroup.fontSize = Integer.parseInt(line.substring(9).trim());
                        } catch (Exception e) {
                        }
                } else if (line.startsWith("Columns=")) {
                    if (currentGroup != null)
                        try {
                            currentGroup.columns = Integer.parseInt(line.substring(8).trim());
                        } catch (Exception e) {
                        }
                } else if (line.startsWith("PanelColumns=")) {
                    if (currentGroup != null)
                        try {
                            currentGroup.panelColumns = Integer.parseInt(line.substring(13).trim());
                        } catch (Exception e) {
                        }
                } else {
                    // Item line: Label || Formula || Format || Value/Visible
                    // Extended format: Label || TYPE:property:params || format || defaultValue
                    String[] parts = line.split("\\|\\|");
                    if (parts.length >= 2) {
                        String label = parts[0].trim();
                        String formula = parts[1].trim();
                        String fmt = parts.length > 2 ? parts[2].trim() : "%s";
                        String valueStr = parts.length > 3 ? parts[3].trim() : "";

                        // If config starts without a group, create a default one
                        if (currentGroup == null) {
                            currentGroup = new GroupConfig("General");
                            groups.add(currentGroup);
                        }

                        RowConfig rc = new RowConfig(label, formula, fmt);
                        rc.defaultValue = valueStr;

                        // Parse control type from formula
                        if (formula.equalsIgnoreCase("HEADER")) {
                            rc.type = "HEADER";
                            rc.visible = valueStr.isEmpty() || valueStr.equalsIgnoreCase("true");
                        } else if (formula.startsWith("SLIDER:")) {
                            // Format: SLIDER:property:min:max
                            rc.type = "SLIDER";
                            String[] sliderParts = formula.substring(7).split(":");
                            if (sliderParts.length >= 1)
                                rc.property = sliderParts[0];
                            if (sliderParts.length >= 2) {
                                try {
                                    rc.minVal = Integer.parseInt(sliderParts[1]);
                                } catch (Exception e) {
                                }
                            }
                            if (sliderParts.length >= 3) {
                                try {
                                    rc.maxVal = Integer.parseInt(sliderParts[2]);
                                } catch (Exception e) {
                                }
                            }
                            rc.visible = true;
                        } else if (formula.startsWith("COMBO:")) {
                            // Format: COMBO:property:optionSource
                            rc.type = "COMBO";
                            String[] comboParts = formula.substring(6).split(":", 2);
                            if (comboParts.length >= 1)
                                rc.property = comboParts[0];
                            // optionSource stored in format field for later processing
                            if (comboParts.length >= 2)
                                rc.format = comboParts[1];
                            rc.visible = true;
                        } else if (formula.startsWith("SWITCH:")) {
                            // Format: SWITCH:property
                            rc.type = "SWITCH";
                            rc.property = formula.substring(7);
                            rc.visible = valueStr.isEmpty() || valueStr.equalsIgnoreCase("true");
                        } else if (formula.startsWith("SWITCH_INV:")) {
                            // Format: SWITCH_INV:property (inverted: ON=false, OFF=true)
                            rc.type = "SWITCH_INV";
                            rc.property = formula.substring(11);
                            rc.visible = valueStr.isEmpty() || valueStr.equalsIgnoreCase("true");
                        } else {
                            // Regular DATA row
                            rc.type = "DATA";
                            rc.visible = valueStr.isEmpty() || valueStr.equalsIgnoreCase("true");
                        }

                        currentGroup.rows.add(rc);
                    }
                }
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
        return groups;
    }

    public static void saveConfig(String path, List<GroupConfig> groups) {
        try (PrintWriter pw = new PrintWriter(
                new OutputStreamWriter(new FileOutputStream(path), StandardCharsets.UTF_8))) {
            for (GroupConfig group : groups) {
                pw.println("[" + group.title + "]");
                pw.println("X=" + String.format("%.4f", group.x));
                pw.println("Y=" + String.format("%.4f", group.y));
                pw.println("Alpha=" + group.alpha);
                if (group.hotkey != 0) {
                    pw.println("Hotkey=" + NativeKeyEvent.getKeyText(group.hotkey));
                }
                pw.println("Visible=" + group.visible);
                pw.println("Font=" + group.fontName);
                if (group.fontSize != 0) {
                    pw.println("FontSize=" + group.fontSize);
                }
                if (group.columns != 2) {
                    pw.println("Columns=" + group.columns);
                }
                if (group.panelColumns != 2) {
                    pw.println("PanelColumns=" + group.panelColumns);
                }
                pw.println();

                for (RowConfig row : group.rows) {
                    if (row.formula != null && row.formula.trim().equalsIgnoreCase("HEADER")) {
                        pw.println(row.label + " || HEADER || %s || " + row.visible);
                    } else {
                        pw.println(row.label + " || " + row.formula + " || " + row.format + " || " + row.visible);
                    }
                }
                pw.println();
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
