package ui.overlay;

import java.util.List;

import parser.AttributePool;
import ui.base.FieldOverlay;
import prog.config.ConfigLoader;
import prog.config.ConfigProvider;
import ui.model.EngineInfoConfig;
import ui.model.FieldDefinition;
import ui.renderer.BOSStyleRenderer;
import ui.renderer.OverlayRenderer;

/**
 * EngineInfo overlay window displaying real-time engine data.
 * 
 * Refactored to extend FieldOverlay for consistent event-driven updates.
 * Uses EngineInfoConfig for configuration and field definitions.
 */
public class EngineInfo extends FieldOverlay {
	private static final long serialVersionUID = 1L;

	private EngineInfoConfig engineInfoConfig;

	public EngineInfo() {
		super();
	}

	@Override
	protected OverlayRenderer createRenderer() {
		return new BOSStyleRenderer();
	}

	@Override
	protected List<FieldDefinition> getFieldDefinitions() {
		if (engineInfoConfig == null)
			return null;
		return engineInfoConfig.getFieldDefinitions();
	}

	/**
	 * Initialize with EngineInfoConfig.
	 */
	public void init(ConfigProvider config, AttributePool pool, EngineInfoConfig engineInfoConfig) {
		this.engineInfoConfig = engineInfoConfig;

		// Set config keys from EngineInfoConfig
		this.numFontKey = engineInfoConfig.numFontKey;
		this.labelFontKey = engineInfoConfig.labelFontKey;
		this.fontAddKey = engineInfoConfig.fontAddKey;
		// Map EngineInfo 'columns' key to FieldOverlay expected key
		this.columnKey = engineInfoConfig.columnKey;

		this.edgeKey = engineInfoConfig.edgeKey;
		this.defaultShowEdge = engineInfoConfig.showEdge;
		this.title = engineInfoConfig.title;

		// Set position keys - Deprecated/Removed in favor of GroupConfig
		// setPositionKeys(engineInfoConfig.posXKey, engineInfoConfig.posYKey);

		// Call parent init
		super.init(config, pool);
	}

	/**
	 * Initialize for preview mode.
	 */
	public void initPreview(ConfigProvider config, AttributePool pool, EngineInfoConfig engineInfoConfig) {
		init(config, pool, engineInfoConfig);
		applyPreviewStyle();
		setupDragListeners();
		setVisible(true);
		reinitConfig();
	}

	/**
	 * Custom reinitConfig to handle specific EngineInfo visibility logic if needed.
	 * FieldOverlay's default reinitConfig uses DefaultFieldManager which checks
	 * config.getConfig(key).
	 * Since EngineInfo data rows in ui_layout.cfg don't have separate switch keys
	 * (they are DATA rows),
	 * we need to ensure visibility works.
	 * 
	 * However, ConfigProvider (Controller) loads ui_layout.cfg into dynamicConfigs
	 * (List<GroupConfig>).
	 * It does NOT flatten row visibility into global config keys!
	 * 
	 * Solution: We must implement a custom ConfigProvider wrapper or handle
	 * visibility here?
	 * NO, `FieldOverlay` uses `RenderContext` for fonts/layout, but `FieldManager`
	 * uses `ConfigProvider` for visibility.
	 * 
	 * We need to override `initFields` to set visibility based on `GroupConfig`
	 * rows!
	 */
	@Override
	public void reinitConfig() {
		if (engineInfoConfig == null)
			return;

		// 1. Standard FieldOverlay reinit (Fonts, Layout, Window Size)
		super.reinitConfig();

		// 2. Override visibility based on GroupConfig rows from ui_layout.cfg
		// Use the GroupConfig passed in EngineInfoConfig
		ConfigLoader.GroupConfig groupConfig = engineInfoConfig.groupConfig;
		if (groupConfig != null) {
			for (ConfigLoader.RowConfig row : groupConfig.rows) {
				String labelKey = LABEL_TO_KEY.get(row.label);
				if (labelKey != null) {
					// Find field def with this configKey and set visibility
					for (ui.model.FieldDefinition def : getFieldDefinitions()) {
						if (labelKey.equals(def.configKey)) {
							fieldManager.setFieldVisible(def.key, row.visible);
						}
					}
				}
			}
			// Trigger repaint after visibility update
			repaint();
		}
	}

	@Override
	protected int[] loadPosition(int defaultX, int defaultY) {
		if (engineInfoConfig != null && engineInfoConfig.groupConfig != null) {
			int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
			int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
			int x = (int) (engineInfoConfig.groupConfig.x * screenW);
			int y = (int) (engineInfoConfig.groupConfig.y * screenH);
			return new int[] { x, y };
		}
		return super.loadPosition(defaultX, defaultY);
	}

	@Override
	public void saveCurrentPosition() {
		if (engineInfoConfig != null && engineInfoConfig.groupConfig != null) {
			int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
			int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;

			engineInfoConfig.groupConfig.x = (double) getLocation().x / screenW;
			engineInfoConfig.groupConfig.y = (double) getLocation().y / screenH;

			if (config instanceof prog.Controller) {
				((prog.Controller) config).configService.saveLayoutConfig();
			}
		} else {
			super.saveCurrentPosition();
		}
	}

	// Legacy Mapping for Visibility Lookup
	private static final java.util.Map<String, String> LABEL_TO_KEY = new java.util.HashMap<>();
	static {
		LABEL_TO_KEY.put("功率", "HorsePower");
		LABEL_TO_KEY.put("HorsePower", "HorsePower");
		LABEL_TO_KEY.put("推力", "Thrust");
		LABEL_TO_KEY.put("Thrust", "Thrust");
		LABEL_TO_KEY.put("转速", "RPM");
		LABEL_TO_KEY.put("RPM", "RPM");
		LABEL_TO_KEY.put("桨距角", "PropPitch");
		LABEL_TO_KEY.put("PropPitch", "PropPitch");
		LABEL_TO_KEY.put("桨效率", "EffEta");
		LABEL_TO_KEY.put("EffEta", "EffEta");
		LABEL_TO_KEY.put("实功率", "EffHp");
		LABEL_TO_KEY.put("EffHp", "EffHp");
		LABEL_TO_KEY.put("进气压", "Pressure");
		LABEL_TO_KEY.put("Pressure", "Pressure");
		LABEL_TO_KEY.put("动力量", "PowerPercent");
		LABEL_TO_KEY.put("PowerPercent", "PowerPercent");
		LABEL_TO_KEY.put("燃油量", "FuelKg");
		LABEL_TO_KEY.put("FuelKg", "FuelKg");
		LABEL_TO_KEY.put("燃油时", "FuelTime");
		LABEL_TO_KEY.put("FuelTime", "FuelTime");
		LABEL_TO_KEY.put("加力量", "WepKg");
		LABEL_TO_KEY.put("WepKg", "WepKg");
		LABEL_TO_KEY.put("加力时", "WepTime");
		LABEL_TO_KEY.put("WepTime", "WepTime");
		LABEL_TO_KEY.put("温度", "Temp");
		LABEL_TO_KEY.put("Temp", "Temp");
		LABEL_TO_KEY.put("油温", "OilTemp");
		LABEL_TO_KEY.put("OilTemp", "OilTemp");
		LABEL_TO_KEY.put("耐热时", "HeatTolerance");
		LABEL_TO_KEY.put("HeatTolerance", "HeatTolerance");
		LABEL_TO_KEY.put("响应速", "EngResponse");
		LABEL_TO_KEY.put("EngResponse", "EngResponse");
	}
}