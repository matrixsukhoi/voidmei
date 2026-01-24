package ui.overlay;

import java.util.List;

import ui.base.FieldOverlay;
import ui.model.FieldDefinition;
import ui.model.EngineInfoConfig;
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
	 * Standardized initialization.
	 */
	public void init(prog.Controller c, prog.Service s, prog.config.OverlaySettings settings) {
		this.config = c;
		this.engineInfoConfig = ui.model.EngineInfoConfig.createDefault(c, settings.getGroupConfig());

		// Standardize style/font keys from Config object
		this.numFontKey = engineInfoConfig.numFontKey;
		this.labelFontKey = engineInfoConfig.labelFontKey;
		this.fontAddKey = engineInfoConfig.fontAddKey;
		this.columnKey = engineInfoConfig.columnKey;
		this.edgeKey = engineInfoConfig.edgeKey;
		this.defaultShowEdge = engineInfoConfig.showEdge;
		this.title = engineInfoConfig.title;

		setOverlaySettings(settings);
		super.init(c, c.globalPool);

		if (s != null) {
			setVisible(true);
		}
	}

	/**
	 * Initialize for preview mode using standardized signature.
	 */
	public void initPreview(prog.Controller c, prog.config.OverlaySettings settings) {
		init(c, null, settings);
		applyPreviewStyle();
		setupDragListeners();
		setVisible(true);
		reinitConfig();
	}

	/**
	 * Custom reinitConfig to handle specific EngineInfo visibility logic.
	 */
	@Override
	public void reinitConfig() {
		if (engineInfoConfig == null)
			return;

		// 1. Standard FieldOverlay reinit (Fonts, Layout, Window Size)
		super.reinitConfig();

		// 2. Override visibility based on GroupConfig rows from ui_layout.cfg
		prog.config.ConfigLoader.GroupConfig groupConfig = engineInfoConfig.groupConfig;
		if (groupConfig != null) {
			for (prog.config.ConfigLoader.RowConfig row : groupConfig.rows) {
				String labelKey = LABEL_TO_KEY.get(row.label);
				if (labelKey != null) {
					for (ui.model.FieldDefinition def : getFieldDefinitions()) {
						if (labelKey.equals(def.configKey)) {
							fieldManager.setFieldVisible(def.key, row.getBool());
						}
					}
				}
			}
			repaint();
		}
	}

	// Manual loadPosition and saveCurrentPosition overrides removed.

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