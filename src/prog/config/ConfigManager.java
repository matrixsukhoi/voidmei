package prog.config;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;
import prog.i18n.Lang;
import prog.util.Logger;
import prog.util.UIStateStorage;
import ui.util.DialogService;

/**
 * Manages dual-file configuration strategy with template hash detection and automatic merging.
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
     * Records details about what was merged during a config merge operation.
     */
    public static class MergeReport {
        public List<String> addedPanels = new ArrayList<>();
        public List<String> addedItems = new ArrayList<>();
        public List<String> updatedItems = new ArrayList<>();

        public boolean hasChanges() {
            return !addedPanels.isEmpty() || !addedItems.isEmpty() || !updatedItems.isEmpty();
        }
    }

    /**
     * Initializes configuration by handling first-run, template change detection, or parse errors.
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

        // Calculate template file hash
        String currentTemplateHash = calculateFileHash(TEMPLATE_PATH);

        // Scenario: First run - user config doesn't exist
        if (!userFile.exists()) {
            Logger.info("ConfigManager", "First run detected, copying template to user config");
            // Direct file copy to preserve original formatting exactly
            try {
                Files.copy(templateFile.toPath(), userFile.toPath());
            } catch (IOException e) {
                Logger.error("ConfigManager", "Failed to copy template: " + e.getMessage());
            }
            UIStateStorage.saveTemplateHash(currentTemplateHash);
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

        // Read stored hash from UIStateStorage
        String storedHash = UIStateStorage.loadTemplateHash();

        // Hash matches - no merge needed
        if (currentTemplateHash != null && currentTemplateHash.equals(storedHash)) {
            Logger.info("ConfigManager", String.format("Template unchanged, skipping merge, %s == %s", currentTemplateHash, storedHash));
            return userConfigs;
        }

        // Hash differs or missing - perform merge
        Logger.info("ConfigManager", "Template changed, merging configs");
        createBackup();

        // Load template with content for source text preservation
        String[] templateContentHolder = new String[1];
        List<GroupConfig> templateConfigs = ConfigLoader.loadConfigWithContent(TEMPLATE_PATH, templateContentHolder);

        MergeReport report = new MergeReport();
        List<GroupConfig> merged = mergeConfigs(templateConfigs, userConfigs, report);

        // Use format-preserving save with template content (since merged structure comes from template)
        ConfigLoader.saveConfigPreserving(USER_PATH, merged, templateContentHolder[0]);
        UIStateStorage.saveTemplateHash(currentTemplateHash);

        if (report.hasChanges()) {
            showMergeReport(report);
        }

        return merged;
    }

    /**
     * Calculates the MD5 hash of a file.
     *
     * @param filePath Path to the file
     * @return Hex string of the MD5 hash, or null on error
     */
    private static String calculateFileHash(String filePath) {
        try {
            MessageDigest md = MessageDigest.getInstance("MD5");
            byte[] content = Files.readAllBytes(Paths.get(filePath));
            byte[] hash = md.digest(content);
            StringBuilder sb = new StringBuilder();
            for (byte b : hash) {
                sb.append(String.format("%02x", b));
            }
            return sb.toString();
        } catch (Exception e) {
            Logger.error("ConfigManager", "Failed to calculate file hash: " + e.getMessage());
            return null;
        }
    }

    /**
     * Merges template and user configs, recording changes in the report.
     *
     * Merge rules by field type:
     * - x, y, alpha, visible, value, hotkey: Preserve user settings
     * - columns, desc, format: Use template (latest definition)
     * - New config items: Insert with default values from template
     *
     * @param template The template configuration (source of truth for structure)
     * @param user The user configuration (source of truth for user values)
     * @param report Records what was merged (can be null for silent merge)
     * @return Merged configuration
     */
    public static List<GroupConfig> mergeConfigs(List<GroupConfig> template, List<GroupConfig> user, MergeReport report) {
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
                if (report != null) {
                    report.addedPanels.add(templatePanel.title);
                }
            } else {
                // Merge existing panel
                GroupConfig mergedPanel = mergePanel(templatePanel, userPanel, report);
                merged.add(mergedPanel);
            }
        }

        return merged;
    }

    /**
     * Overload for backward compatibility (import, etc.) where no report is needed.
     */
    public static List<GroupConfig> mergeConfigs(List<GroupConfig> template, List<GroupConfig> user) {
        return mergeConfigs(template, user, null);
    }

    /**
     * Merges a single panel from template and user.
     */
    private static GroupConfig mergePanel(GroupConfig template, GroupConfig user, MergeReport report) {
        GroupConfig merged = new GroupConfig(template.title);

        // User-preserved fields
        merged.x = user.x;
        merged.y = user.y;
        merged.alpha = user.alpha;
        merged.visible = user.visible;

        // Template fields (structure)
        merged.columns = template.columns;
        merged.panelColumns = template.panelColumns;
        merged.fontName = template.fontName;
        merged.fontSize = template.fontSize;
        merged.hotkey = user.hotkey; // Preserve user hotkey
        merged.switchKey = template.switchKey;

        // Merge rows (pass panel title for report context)
        merged.rows = mergeRows(template.rows, user.rows, template.title, template.title, report);

        return merged;
    }

    /**
     * Merges row configurations recursively.
     *
     * @param panelTitle The panel title (for report display context)
     * @param groupPath Current path in hierarchy (e.g. "MiniHUD" or "MiniHUD/hud面板设置")
     */
    private static List<RowConfig> mergeRows(List<RowConfig> templateRows, List<RowConfig> userRows,
                                              String panelTitle, String groupPath, MergeReport report) {
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
                if (report != null) {
                    String displayName = templateRow.label;
                    report.addedItems.add(panelTitle + ": " + displayName);
                }
            } else {
                // Merge existing row
                RowConfig mergedRow = mergeRow(templateRow, userRow, panelTitle, report);
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
    private static RowConfig mergeRow(RowConfig template, RowConfig user, String panelTitle, MergeReport report) {
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
        merged.unitSource = template.unitSource;
        merged.precisionSource = template.precisionSource;

        // Preserve hasExplicitDefault from template
        merged.hasExplicitDefault = template.hasExplicitDefault;

        // User-preserved fields
        merged.value = user.value;

        // Determine if user value differs from template default
        // If same as template default, we can use template's source text
        boolean valueMatchesDefault = java.util.Objects.equals(user.value, template.defaultValue)
            || java.util.Objects.equals(user.value, template.value);

        if (valueMatchesDefault && template.sourceText != null) {
            // Value unchanged from template - preserve template's source text
            merged.sourceText = template.sourceText;
            merged.sourceStart = template.sourceStart;
            merged.sourceEnd = template.sourceEnd;
            merged.modified = false;
        } else {
            // User has customized this value - needs regeneration
            merged.sourceText = null;
            merged.modified = true;
        }

        // Merge children recursively (pass report through for nested items)
        if (template.children != null && !template.children.isEmpty()) {
            merged.children = mergeRows(template.children,
                user.children != null ? user.children : new ArrayList<>(),
                panelTitle, panelTitle + "/" + template.label, report);
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

            // Load template for merging (with content for format preservation)
            String[] templateContentHolder = new String[1];
            List<GroupConfig> templateConfigs = ConfigLoader.loadConfigWithContent(TEMPLATE_PATH, templateContentHolder);

            // Merge imported config with template (to ensure structure compatibility)
            List<GroupConfig> merged = mergeConfigs(templateConfigs, importedConfigs);

            // Save merged config with format preservation
            ConfigLoader.saveConfigPreserving(USER_PATH, merged, templateContentHolder[0]);
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
        // Run on EDT for thread safety (using DialogService to avoid overlay blocking)
        javax.swing.SwingUtilities.invokeLater(() -> {
            DialogService.showMessageDialog(
                null,
                Lang.mConfigErrorContent,
                Lang.mConfigErrorTitle,
                com.alee.laf.optionpane.WebOptionPane.ERROR_MESSAGE
            );
        });
    }

    /**
     * Shows a detailed merge report dialog listing what was added/updated.
     */
    private static void showMergeReport(MergeReport report) {
        StringBuilder sb = new StringBuilder();

        if (!report.addedPanels.isEmpty()) {
            sb.append(Lang.mMergeAddedPanels).append("\n");
            for (String panel : report.addedPanels) {
                sb.append("  \u2022 ").append(panel).append("\n");
            }
            sb.append("\n");
        }

        if (!report.addedItems.isEmpty()) {
            sb.append(Lang.mMergeAddedItems).append("\n");
            for (String item : report.addedItems) {
                sb.append("  \u2022 ").append(item).append("\n");
            }
            sb.append("\n");
        }

        if (!report.updatedItems.isEmpty()) {
            sb.append(Lang.mMergeUpdatedItems).append("\n");
            for (String item : report.updatedItems) {
                sb.append("  \u2022 ").append(item).append("\n");
            }
        }

        String message = sb.toString().trim();
        Logger.info("ConfigManager", "Merge report:\n" + message);

        // Using DialogService to avoid overlay blocking
        javax.swing.SwingUtilities.invokeLater(() -> {
            DialogService.showMessageDialog(
                null, message,
                Lang.mConfigMergedTitle,
                com.alee.laf.optionpane.WebOptionPane.INFORMATION_MESSAGE
            );
        });
    }
}
