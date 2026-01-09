package ui.util;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;

public class ConfigLoader {

    public static class RowConfig {
        public String label;
        public String formula;
        public String format;

        public RowConfig(String label, String formula, String format) {
            this.label = label;
            this.formula = formula;
            this.format = format;
        }
    }

    public static class GroupConfig {
        public String title;
        public int x = 100;
        public int y = 100;
        public int alpha = 150;
        public String fontName = "Sarasa Mono SC";
        public List<RowConfig> rows = new ArrayList<>();

        public GroupConfig(String title) {
            this.title = title;
        }
    }

    public static List<GroupConfig> loadConfig(String filePath) {
        List<GroupConfig> groups = new ArrayList<>();
        File file = new File(filePath);

        if (!file.exists())
            return groups;

        try (BufferedReader br = new BufferedReader(new InputStreamReader(new FileInputStream(file), "UTF-8"))) {
            String line;
            GroupConfig currentGroup = null;

            while ((line = br.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty() || line.startsWith("#"))
                    continue;

                if (line.startsWith("[") && line.endsWith("]")) {
                    String title = line.substring(1, line.length() - 1).trim();
                    currentGroup = new GroupConfig(title);
                    groups.add(currentGroup);
                } else if (line.startsWith("X=")) {
                    if (currentGroup != null)
                        try {
                            currentGroup.x = Integer.parseInt(line.substring(2).trim());
                        } catch (Exception e) {
                        }
                } else if (line.startsWith("Y=")) {
                    if (currentGroup != null)
                        try {
                            currentGroup.y = Integer.parseInt(line.substring(2).trim());
                        } catch (Exception e) {
                        }
                } else if (line.startsWith("Alpha=")) {
                    if (currentGroup != null)
                        try {
                            currentGroup.alpha = Integer.parseInt(line.substring(6).trim());
                        } catch (Exception e) {
                        }
                } else if (line.startsWith("Font=")) {
                    if (currentGroup != null)
                        currentGroup.fontName = line.substring(5).trim();
                } else {
                    // Item line: Label || Formula || Format
                    String[] parts = line.split("\\|\\|");
                    if (parts.length >= 2) {
                        String label = parts[0].trim();
                        String formula = parts[1].trim();
                        String fmt = parts.length > 2 ? parts[2].trim() : "%s";

                        // If config starts without a group, create a default one
                        if (currentGroup == null) {
                            currentGroup = new GroupConfig("General");
                            groups.add(currentGroup);
                        }
                        currentGroup.rows.add(new RowConfig(label, formula, fmt));
                    }
                }
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
        return groups;
    }
}
