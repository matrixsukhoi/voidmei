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
	private ui.model.TelemetrySource service;

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

		this.service = s;
		if (s != null) {
			bindEngineFields(s);
			setVisible(true);
		}
	}

	private void bindEngineFields(ui.model.TelemetrySource s) {
		ui.model.FieldManager fm = this.fieldManager;

		// Raw bind to internal keys defined in EngineInfoConfig
		fm.bind("hp", s::getHorsePower, 0);
		fm.bind("thrust", s::getThrust, 0);
		fm.bind("RPM", s::getRPM, 0);
		fm.bind("pitch", s::getPitch, 1);
		fm.bind("eff_eta", s::getPropEfficiency, () -> s.getPropEfficiency() > 0, 0);
		fm.bind("eff_hp", s::getEffHp, 0);
		fm.bind("power_percent", s::getPowerPercent, 0);

		// Fuel & WEP
		fm.bind("fuel_kg", s::getMassFuel, 0);
		fm.bind("fuel_time", () -> s.getFuelTimeMili() / 60000.0, 1); // Display as Minutes
		fm.bind("wep", s::getWepKg, () -> s.getWepKg() > 0, 0);
		fm.bind("wep_time", () -> s.getWepTime() / 60.0, () -> s.getWepTime() > 0, 1); // Display as Minutes

		// Temperatures & Limits
		fm.bind("temp", s::getWaterTemp, 0);
		fm.bind("oil_temp", s::getOilTemp, 0);
		fm.bind("heat_time", s::getHeatTolerance, 0);

		// Response & Pressure
		fm.bind("response", s::getEngineResponse, 0);
		fm.bind("pressure", () -> s.isImperial() ? s.getManifoldPressurePounds() : s.getManifoldPressure(),
				s::isManifoldPressureValid, 2);
	}

	private boolean lastImperial = false;
	private boolean firstData = true;

	@Override
	public void onFlightData(prog.event.FlightDataEvent event) {
		if (service != null) {
			boolean currentImperial = service.isImperial();
			if (firstData || currentImperial != lastImperial) {
				String unit = currentImperial ? "psi" : "Ata";
				fieldManager.updateFieldUnit("pressure", unit);
				lastImperial = currentImperial;
				firstData = false;
			}
		}
		super.onFlightData(event);
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
		firstData = true;

		// 1. Standard FieldOverlay reinit (Fonts, Layout, Window Size)
		super.reinitConfig();

		// 2. Override visibility based on GroupConfig rows from ui_layout.cfg
		prog.config.ConfigLoader.GroupConfig groupConfig = engineInfoConfig.groupConfig;
		if (groupConfig != null) {
			updateFieldVisibilityRecursive(groupConfig.rows);
			repaint();
		}
	}

	private void updateFieldVisibilityRecursive(List<prog.config.ConfigLoader.RowConfig> rows) {
		for (prog.config.ConfigLoader.RowConfig row : rows) {
			String labelKey = LABEL_TO_KEY.get(row.label);
			if (labelKey != null) {
				for (ui.model.FieldDefinition def : getFieldDefinitions()) {
					if (labelKey.equals(def.configKey)) {
						fieldManager.setFieldVisible(def.key, row.getBool());
					}
				}
			}
			if (row.children != null && !row.children.isEmpty()) {
				updateFieldVisibilityRecursive(row.children);
			}
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