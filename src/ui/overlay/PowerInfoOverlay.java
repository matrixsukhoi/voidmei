package ui.overlay;

import java.util.List;

import ui.base.FieldOverlay;
import ui.layout.renderer.RowRendererRegistry;
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
public class PowerInfoOverlay extends FieldOverlay {
	private RowRendererRegistry registry;
	private static final long serialVersionUID = 1L;

	private EngineInfoConfig engineInfoConfig;
	private ui.model.TelemetrySource service;

	public PowerInfoOverlay() {
		super();
		this.registry = new RowRendererRegistry();
		setTitle("动力信息");
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
		this.service = s; // Assign service FIRST so reinitConfig can use it
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
		if (overlaySettings != null) {
			// Refresh Config object from latest data
			this.engineInfoConfig = ui.model.EngineInfoConfig.createDefault(this.config,
					overlaySettings.getGroupConfig());

			// Sync style configuration
			this.numFontKey = engineInfoConfig.numFontKey;
			this.labelFontKey = engineInfoConfig.labelFontKey;
			this.fontAddKey = engineInfoConfig.fontAddKey;
			this.columnKey = engineInfoConfig.columnKey;
			this.edgeKey = engineInfoConfig.edgeKey;
			this.defaultShowEdge = engineInfoConfig.showEdge;
			this.title = engineInfoConfig.title;
		}

		if (engineInfoConfig == null)
			return;

		firstData = true;

		// 1. Standard FieldOverlay reinit (Fonts, Layout, Window Size)
		super.reinitConfig();

		// 2. Bind the fields to data sources
		prog.config.ConfigLoader.GroupConfig groupConfig = engineInfoConfig.groupConfig;
		if (groupConfig != null && service != null) {
			bindDynamicFields(service, groupConfig.rows);
			repaint();
		}
	}

	// Manual loadPosition and saveCurrentPosition overrides removed.

	/**
	 * Dynamically binds fields based on ConfigLoader.RowConfig items.
	 */
	private void bindDynamicFields(ui.model.TelemetrySource s,
			java.util.List<prog.config.ConfigLoader.RowConfig> rows) {
		ui.model.FieldManager fm = this.fieldManager;

		for (prog.config.ConfigLoader.RowConfig row : rows) {
			// Bind if it's a DATA Item (and has a target)
			if ("DATA".equals(row.type) && row.property != null && !row.property.isEmpty()) {
				// Use ReflectBinder to create a zero-GC supplier
				java.util.function.DoubleSupplier supplier = ui.util.ReflectBinder.resolveDouble(s, row.property);

				// Resolve validity method (e.g., getRPM -> isRPMValid)
				String baseMethod = row.property.trim();
				if (baseMethod.contains("*")) {
					baseMethod = baseMethod.split("\\*")[0].trim();
				}
				if (baseMethod.startsWith("get")) {
					String validityMethod = "is" + baseMethod.substring(3) + "Valid";
					java.util.function.BooleanSupplier visibilitySupplier = ui.util.ReflectBinder.resolveBoolean(s,
							validityMethod);
					fm.bind(row.property, supplier, visibilitySupplier, row.precision);
				} else {
					fm.bind(row.property, supplier, row.precision);
				}

				// Apply visibility immediately based on config value
				fm.setFieldVisible(row.property, row.getBool());
			}

			// Recurse for groups
			if (row.children != null && !row.children.isEmpty()) {
				bindDynamicFields(s, row.children);
			}
		}
	}
}