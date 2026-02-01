package prog.config;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;
import prog.i18n.Lang;
import prog.util.Logger;

/**
 * Manages dual-file configuration strategy with version detection and automatic merging.
 *
 * File responsibilities:
 * - ui_layout.cfg: Read-only template distributed with the program
 * - ui_layout.user.cfg: User configuration, actual read/write target
 * - ui_layout.user.cfg.bak: Automatic backup
 */
public class ConfigManager {

    private static final String TEMPLATE_PATH = "./ui_layout.cfg";
    private static final String USER_PATH = "./ui_layout.user.cfg";
    private static final String BACKUP_PATH = "./ui_layout.user.cfg.bak";

    /**
     * Initializes configuration by handling first-run, version upgrade, or parse errors.
     *
     * @return List of GroupConfig to use for the application
     */
    public static List<GroupConfig> initialize() {
        File templateFile = new File(TEMPLATE_PATH);
        File userFile = new File(USER_PATH);

        // Scenario: Template doesn't exist (shouldn't happen in normal distribution)
        if (!templateFile.exists()) {
            Logger.warn("ConfigManager", "Template file not found: " + TEMPLATE_PATH);
            return new ArrayList<>();
        }

        // Scenario: First run - user config doesn't exist
        if (!userFile.exists()) {
            Logger.info("ConfigManager", "First run detected, copying template to user config");
            try {
                Files.copy(templateFile.toPath(), userFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
            } catch (IOException e) {
                Logger.error("ConfigManager", "Failed to copy template: " + e.getMessage());
            }
            return ConfigLoader.loadConfig(USER_PATH);
        }

        // Try to load user config
        List<GroupConfig> userConfigs;
        try {
            userConfigs = ConfigLoader.loadConfig(USER_PATH);
        } catch (Exception e) {
            // Scenario: Parse error - use template temporarily
            Logger.error("ConfigManager", "Failed to parse user config: " + e.getMessage());
            showParseErrorDialog();
            return ConfigLoader.loadConfig(TEMPLATE_PATH);
        }

        // Load template for version comparison
        List<GroupConfig> templateConfigs = ConfigLoader.loadConfig(TEMPLATE_PATH);

        // Check if upgrade is needed
        if (needsUpgrade(userConfigs, templateConfigs)) {
            Logger.info("ConfigManager", "Version upgrade detected, merging configs");
            createBackup();

            String oldVersion = getMaxVersion(userConfigs);
            String newVersion = getMaxVersion(templateConfigs);

            List<GroupConfig> merged = mergeConfigs(templateConfigs, userConfigs);
            ConfigLoader.saveConfig(USER_PATH, merged);

            showUpgradeNotification(oldVersion, newVersion);
            return merged;
        }

        return userConfigs;
    }

    /**
     * Checks if user config needs upgrade by comparing versions.
     */
    private static boolean needsUpgrade(List<GroupConfig> userConfigs, List<GroupConfig> templateConfigs) {
        String userMaxVersion = getMaxVersion(userConfigs);
        String templateMaxVersion = getMaxVersion(templateConfigs);

        return compareVersions(userMaxVersion, templateMaxVersion) < 0;
    }

    /**
     * Gets the maximum version string from all panels.
     */
    private static String getMaxVersion(List<GroupConfig> configs) {
        String maxVersion = "0.0.0";
        for (GroupConfig gc : configs) {
            if (gc.cfgVersion != null && !gc.cfgVersion.isEmpty()) {
                if (compareVersions(gc.cfgVersion, maxVersion) > 0) {
                    maxVersion = gc.cfgVersion;
                }
            }
        }
        return maxVersion;
    }

    /**
     * Compares two version strings (semantic versioning: x.y.z).
     *
     * @return negative if v1 < v2, 0 if equal, positive if v1 > v2
     */
    private static int compareVersions(String v1, String v2) {
        if (v1 == null || v1.isEmpty()) v1 = "0.0.0";
        if (v2 == null || v2.isEmpty()) v2 = "0.0.0";

        String[] parts1 = v1.split("\\.");
        String[] parts2 = v2.split("\\.");

        int maxLen = Math.max(parts1.length, parts2.length);
        for (int i = 0; i < maxLen; i++) {
            int num1 = (i < parts1.length) ? parseIntSafe(parts1[i]) : 0;
            int num2 = (i < parts2.length) ? parseIntSafe(parts2[i]) : 0;
            if (num1 != num2) {
                return num1 - num2;
            }
        }
        return 0;
    }

    private static int parseIntSafe(String s) {
        try {
            return Integer.parseInt(s.trim());
        } catch (NumberFormatException e) {
            return 0;
        }
    }

    /**
     * Merges template and user configs.
     *
     * Merge rules by field type:
     * - x, y, alpha, visible, value: Preserve user settings
     * - cfgVersion, columns, desc, format: Use template (latest definition)
     * - New config items: Insert with default values from template
     *
     * @param template The template configuration (source of truth for structure)
     * @param user The user configuration (source of truth for user values)
     * @return Merged configuration
     */
    public static List<GroupConfig> mergeConfigs(List<GroupConfig> template, List<GroupConfig> user) {
        // Build map of user panels by title
        Map<String, GroupConfig> userPanelMap = new HashMap<>();
        for (GroupConfig gc : user) {
            userPanelMap.put(gc.title, gc);
        }

        List<GroupConfig> merged = new ArrayList<>();

        for (GroupConfig templatePanel : template) {
            GroupConfig userPanel = userPanelMap.get(templatePanel.title);

            if (userPanel == null) {
                // New panel in template - use template as-is
                merged.add(templatePanel);
                Logger.info("ConfigManager", "Added new panel: " + templatePanel.title);
            } else {
                // Merge existing panel
                GroupConfig mergedPanel = mergePanel(templatePanel, userPanel);
                merged.add(mergedPanel);
            }
        }

        return merged;
    }

    /**
     * Merges a single panel from template and user.
     */
    private static GroupConfig mergePanel(GroupConfig template, GroupConfig user) {
        GroupConfig merged = new GroupConfig(template.title);

        // User-preserved fields
        merged.x = user.x;
        merged.y = user.y;
        merged.alpha = user.alpha;
        merged.visible = user.visible;

        // Template fields (structure)
        merged.cfgVersion = template.cfgVersion;
        merged.columns = template.columns;
        merged.panelColumns = template.panelColumns;
        merged.fontName = template.fontName;
        merged.fontSize = template.fontSize;
        merged.hotkey = user.hotkey; // Preserve user hotkey
        merged.switchKey = template.switchKey;

        // Merge rows
        merged.rows = mergeRows(template.rows, user.rows);

        return merged;
    }

    /**
     * Merges row configurations recursively.
     */
    private static List<RowConfig> mergeRows(List<RowConfig> templateRows, List<RowConfig> userRows) {
        // Build map of user rows by property/target
        Map<String, RowConfig> userRowMap = new HashMap<>();
        buildRowMap(userRows, userRowMap);

        List<RowConfig> merged = new ArrayList<>();

        for (RowConfig templateRow : templateRows) {
            String key = getRowKey(templateRow);
            RowConfig userRow = userRowMap.get(key);

            if (userRow == null) {
                // New row in template - use template as-is
                merged.add(templateRow);
                Logger.info("ConfigManager", "Added new config item: " + key);
            } else {
                // Merge existing row
                RowConfig mergedRow = mergeRow(templateRow, userRow);
                merged.add(mergedRow);
            }
        }

        return merged;
    }

    /**
     * Builds a map of rows by their key (property or label).
     */
    private static void buildRowMap(List<RowConfig> rows, Map<String, RowConfig> map) {
        for (RowConfig row : rows) {
            String key = getRowKey(row);
            if (key != null && !key.isEmpty()) {
                map.put(key, row);
            }
            // Recurse for HEADER/group children
            if (row.children != null && !row.children.isEmpty()) {
                buildRowMap(row.children, map);
            }
        }
    }

    /**
     * Gets the unique key for a row (property if available, otherwise label).
     */
    private static String getRowKey(RowConfig row) {
        if (row.property != null && !row.property.isEmpty()) {
            return row.property;
        }
        return row.label;
    }

    /**
     * Merges a single row from template and user.
     */
    private static RowConfig mergeRow(RowConfig template, RowConfig user) {
        RowConfig merged = new RowConfig(template.label, template.formula, template.format);

        // Template fields (structure/definition)
        merged.type = template.type;
        merged.property = template.property;
        merged.unit = template.unit;
        merged.format = template.format;
        merged.desc = template.desc;
        merged.descImg = template.descImg;
        merged.previewValue = template.previewValue;
        merged.hideWhenZero = template.hideWhenZero;
        merged.precision = template.precision;
        merged.targetName = template.targetName;
        merged.minVal = template.minVal;
        merged.maxVal = template.maxVal;
        merged.groupColumns = template.groupColumns;
        merged.fgColor = template.fgColor;
        merged.defaultValue = template.defaultValue;

        // User-preserved fields
        merged.value = user.value;

        // Merge children recursively
        if (template.children != null && !template.children.isEmpty()) {
            merged.children = mergeRows(template.children,
                user.children != null ? user.children : new ArrayList<>());
        }

        return merged;
    }

    /**
     * Creates a backup of the current user config.
     */
    public static void createBackup() {
        File userFile = new File(USER_PATH);
        File backupFile = new File(BACKUP_PATH);

        if (userFile.exists()) {
            try {
                Files.copy(userFile.toPath(), backupFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
                Logger.info("ConfigManager", "Created backup: " + BACKUP_PATH);
            } catch (IOException e) {
                Logger.error("ConfigManager", "Failed to create backup: " + e.getMessage());
            }
        }
    }

    /**
     * Imports configuration from an external file.
     *
     * @param sourcePath Path to the config file to import
     * @return true if import was successful
     */
    public static boolean importConfig(String sourcePath) {
        File sourceFile = new File(sourcePath);
        if (!sourceFile.exists()) {
            Logger.error("ConfigManager", "Import source not found: " + sourcePath);
            return false;
        }

        try {
            // Create backup before import
            createBackup();

            // Load and validate source config
            List<GroupConfig> importedConfigs = ConfigLoader.loadConfig(sourcePath);
            if (importedConfigs.isEmpty()) {
                Logger.error("ConfigManager", "Import file is empty or invalid");
                return false;
            }

            // Load template for merging
            List<GroupConfig> templateConfigs = ConfigLoader.loadConfig(TEMPLATE_PATH);

            // Merge imported config with template (to ensure structure compatibility)
            List<GroupConfig> merged = mergeConfigs(templateConfigs, importedConfigs);

            // Save merged config
            ConfigLoader.saveConfig(USER_PATH, merged);
            Logger.info("ConfigManager", "Config imported successfully from: " + sourcePath);
            return true;
        } catch (Exception e) {
            Logger.error("ConfigManager", "Failed to import config: " + e.getMessage());
            return false;
        }
    }

    /**
     * Resets configuration to factory defaults by copying template to user config.
     *
     * @return true if reset was successful
     */
    public static boolean resetToFactory() {
        try {
            // Create backup before reset
            createBackup();

            // Copy template to user config
            File templateFile = new File(TEMPLATE_PATH);
            File userFile = new File(USER_PATH);

            if (!templateFile.exists()) {
                Logger.error("ConfigManager", "Template file not found for factory reset");
                return false;
            }

            Files.copy(templateFile.toPath(), userFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
            Logger.info("ConfigManager", "Config reset to factory defaults");
            return true;
        } catch (IOException e) {
            Logger.error("ConfigManager", "Failed to reset config: " + e.getMessage());
            return false;
        }
    }

    /**
     * Gets the path to the user config file.
     */
    public static String getUserConfigPath() {
        return USER_PATH;
    }

    /**
     * Gets the path to the template config file.
     */
    public static String getTemplateConfigPath() {
        return TEMPLATE_PATH;
    }

    /**
     * Shows a dialog when config parsing fails.
     */
    private static void showParseErrorDialog() {
        // Run on EDT for thread safety
        javax.swing.SwingUtilities.invokeLater(() -> {
            com.alee.laf.optionpane.WebOptionPane.showMessageDialog(
                null,
                Lang.mConfigErrorContent,
                Lang.mConfigErrorTitle,
                com.alee.laf.optionpane.WebOptionPane.ERROR_MESSAGE
            );
        });
    }

    /**
     * Shows a notification when config is upgraded.
     */
    private static void showUpgradeNotification(String oldVersion, String newVersion) {
        String message = String.format(Lang.mConfigUpgraded, oldVersion, newVersion);
        Logger.info("ConfigManager", message);
        // Optionally show UI notification
        javax.swing.SwingUtilities.invokeLater(() -> {
            com.alee.laf.optionpane.WebOptionPane.showMessageDialog(
                null,
                message,
                Lang.mConfigUpgradedTitle,
                com.alee.laf.optionpane.WebOptionPane.INFORMATION_MESSAGE
            );
        });
    }
}
