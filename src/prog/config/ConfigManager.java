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
import ui.minihud.MiniHudDocument;
import ui.minihud.MiniHudLayoutCodec;
import ui.minihud.MiniHudLegacyMigrator;
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
            List<GroupConfig> configs = ConfigLoader.loadConfig(TEMPLATE_PATH);
            ConfigLoader.saveConfig(USER_PATH, configs);
            UIStateStorage.saveTemplateHash(currentTemplateHash);
            return configs;
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

        // Hash matches - no merge needed，除非用户放回了旧版 MiniHUD 配置
        if (currentTemplateHash != null && currentTemplateHash.equals(storedHash)
                && !needsMiniHudMigration(userConfigs)) {
            Logger.info("ConfigManager", String.format("Template unchanged, skipping merge, %s == %s", currentTemplateHash, storedHash));
            return userConfigs;
        }

        // Hash differs/missing，或检测到旧 MiniHUD 布局 - perform merge
        if (currentTemplateHash != null && currentTemplateHash.equals(storedHash))
            Logger.info("ConfigManager", "Legacy MiniHUD layout detected, forcing merge");
        else
            Logger.info("ConfigManager", "Template changed, merging configs");
        createBackup();

        List<GroupConfig> templateConfigs = ConfigLoader.loadConfig(TEMPLATE_PATH);

        MergeReport report = new MergeReport();
        List<GroupConfig> merged = mergeConfigs(templateConfigs, userConfigs, report);

        ConfigLoader.saveConfig(USER_PATH, merged);
        UIStateStorage.saveTemplateHash(currentTemplateHash);

        if (report.hasChanges()) {
            showMergeReport(report);
        }

        return merged;
    }


    /**
     * 用户手工放回旧配置时，模板 hash 可能没变，但 MiniHUD 仍需要迁移：
     * - miniHudLayout 是 V1/坏串
     * - 还存在旧 showHUD / disableHUD 开关
     */
    private static boolean needsMiniHudMigration(List<GroupConfig> userConfigs) {
        GroupConfig mini = null;
        for (GroupConfig g : userConfigs) {
            if ("MiniHUD".equals(g.title)) {
                mini = g;
                break;
            }
        }
        if (mini == null)
            return false;
        Object layout = findRowValue(mini.rows, "miniHudLayout");
        if (layout != null && !String.valueOf(layout).isEmpty()
                && !MiniHudLayoutCodec.isValid(String.valueOf(layout)))
            return true;
        for (String key : MiniHudLegacyMigrator.LEGACY_KEYS) {
            if (findRowValue(mini.rows, key) != null)
                return true;
        }
        return false;
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
                // 准星从 MiniHUD 面板拆出时的一次性迁移：
                // 行是按面板隔离合并的，若不迁移，老用户的准星开关/样式会回默认
                if ("准星".equals(templatePanel.title)) {
                    seedCrosshairPanel(templatePanel, userPanelMap.get("MiniHUD"));
                }
                // New panel in template - use template as-is
                merged.add(templatePanel);
                Logger.info("ConfigManager", "Added new panel: " + templatePanel.title);
                if (report != null) {
                    report.addedPanels.add(templatePanel.title);
                }
            } else {
                // Merge existing panel
                GroupConfig mergedPanel = mergePanel(templatePanel, userPanel, report);
                // MiniHUD 逐项开关 → 自定义布局的一次性迁移（旧行随后被模板删除）
                if ("MiniHUD".equals(templatePanel.title)) {
                    seedMiniHudLayout(mergedPanel, userPanel);
                }
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
     * 旧 MiniHUD 逐项开关 → 自定义布局文档。
     * 只在 miniHudLayout 为空时执行（用户已自定义则不动）；缺失旧键按旧默认值。
     */
    private static void seedMiniHudLayout(GroupConfig target, GroupConfig legacy) {
        if (legacy == null)
            return;
        Object existing = findRowValue(target.rows, "miniHudLayout");
        // 新模板的隐藏行可能只是模板副本；V1 也可能只在用户旧面板里
        Object legacyLayout = findRowValue(legacy.rows, "miniHudLayout");
        Object effective = (existing != null && !String.valueOf(existing).isEmpty()) ? existing : legacyLayout;
        // 已存在的合法 V2 布局不覆盖；若只是用户旧面板里有、合并后目标为空则补回
        if (effective != null && MiniHudLayoutCodec.isValid(String.valueOf(effective))) {
            if (existing == null || String.valueOf(existing).isEmpty())
                setRowValue(target.rows, "miniHudLayout", String.valueOf(effective));
            return;
        }
        // 未发布的相对布局 V1：保留旧标签/可见性偏好，重建为自由画布 V2
        if (effective != null && String.valueOf(effective).startsWith("V1;")) {
            MiniHudDocument v1 = MiniHudLegacyMigrator.fromV1(String.valueOf(effective));
            if (v1 != null) {
                setRowValue(target.rows, "miniHudLayout", MiniHudLayoutCodec.encode(v1));
                Logger.info("ConfigManager", "Migrated V1 relative MiniHUD layout to free canvas");
                return;
            }
        }

        Map<String, String> legacyValues = new HashMap<>();
        for (String key : MiniHudLegacyMigrator.LEGACY_KEYS) {
            Object v = findRowValue(legacy.rows, key);
            if (v != null)
                legacyValues.put(key, String.valueOf(v));
        }

        // 总内容开关：新键存在则复制旧 drawHUDtext
        Object draw = findRowValue(legacy.rows, "drawHUDtext");
        if (draw != null && findRowValue(target.rows, "miniHudContentVisible") != null) {
            setRowValue(target.rows, "miniHudContentVisible", Boolean.parseBoolean(String.valueOf(draw)));
        }

        if (legacyValues.isEmpty()) {
            // 清掉无法识别的旧布局串，运行时回退默认布局（自愈）
            if (effective != null && !String.valueOf(effective).isEmpty())
                setRowValue(target.rows, "miniHudLayout", "");
            return;
        }
        MiniHudDocument doc = MiniHudLegacyMigrator.fromLegacy(legacyValues);
        setRowValue(target.rows, "miniHudLayout", MiniHudLayoutCodec.encode(doc));
        Logger.info("ConfigManager", "Migrated legacy MiniHUD switches to custom layout");
    }

    /**
     * 把老 MiniHUD 面板里的准星偏好复制到新「准星」面板。
     * 仅在「准星」面板不存在时调用，天然只跑一次；导入旧配置同样覆盖。
     */
    private static void seedCrosshairPanel(GroupConfig target, GroupConfig legacyMiniHUD) {
        if (legacyMiniHUD == null) {
            return;
        }
        Object enabled = findRowValue(legacyMiniHUD.rows, "displayCrosshair");
        Object name = findRowValue(legacyMiniHUD.rows, "crosshairName");
        Object size = findRowValue(legacyMiniHUD.rows, "crosshairScale");

        if (enabled != null) {
            setRowValue(target.rows, "displayCrosshair", enabled);
        }
        if (name != null) {
            setRowValue(target.rows, "crosshairName", name);
        }
        if (size != null) {
            int v;
            if (size instanceof Number) {
                v = ((Number) size).intValue();
            } else {
                try {
                    v = Integer.parseInt(size.toString());
                } catch (NumberFormatException e) {
                    return;
                }
            }
            v = Math.max(0, Math.min(200, v));
            setRowValue(target.rows, "crosshairSize", v);
        }
    }

    private static Object findRowValue(List<RowConfig> rows, String property) {
        for (RowConfig row : rows) {
            if (property.equals(row.property) && row.value != null) {
                return row.value;
            }
            if (row.children != null && !row.children.isEmpty()) {
                Object v = findRowValue(row.children, property);
                if (v != null) {
                    return v;
                }
            }
        }
        return null;
    }

    private static boolean setRowValue(List<RowConfig> rows, String property, Object value) {
        for (RowConfig row : rows) {
            if (property.equals(row.property)) {
                row.value = value;
                return true;
            }
            if (row.children != null && !row.children.isEmpty() && setRowValue(row.children, property, value)) {
                return true;
            }
        }
        return false;
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

        // MiniHUD 启用键改名 crosshairSwitch -> miniHUDSwitch 的一次性兼容
        if ("MiniHUD".equals(template.title)) {
            Object legacySwitch = findRowValue(user.rows, "crosshairSwitch");
            if (legacySwitch != null) {
                setRowValue(merged.rows, "miniHUDSwitch", legacySwitch);
            }
        }

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

            // Skip merge tracking for items without valid keys (e.g., info-only items with empty labels)
            // These items have no user-modifiable values and don't need precise matching
            if (key == null || key.isEmpty()) {
                merged.add(templateRow);
                continue;
            }

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
        // 动态单位/精度源（修复进气压单位显示问题）
        merged.unitSource = template.unitSource;
        merged.precisionSource = template.precisionSource;
        // 可见性和NA条件表达式（始终使用模板值，确保配置合并时保留显示控制逻辑）
        merged.visibleWhen = template.visibleWhen;
        merged.naWhen = template.naWhen;

        // User-preserved fields
        merged.value = user.value;

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
