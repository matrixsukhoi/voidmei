package ui.overlay;

import java.util.List;

import ui.base.FieldOverlay;
import ui.model.FieldDefinition;
import ui.model.FlightInfoConfig;
import ui.renderer.BOSStyleRenderer;
import ui.renderer.OverlayRenderer;

/**
 * FlightInfo overlay window displaying real-time flight data.
 * 
 * Extends FieldOverlay for event-driven updates.
 * Uses FlightInfoConfig for field definitions and BOSStyleRenderer for display.
 */
public class FlightInfo extends FieldOverlay {
	private static final long serialVersionUID = 6759127498151892589L;

	private FlightInfoConfig flightInfoConfig;
	private ui.model.TelemetrySource service;

	public FlightInfo() {
		super();
	}

	@Override
	protected OverlayRenderer createRenderer() {
		return new BOSStyleRenderer();
	}

	@Override
	protected List<FieldDefinition> getFieldDefinitions() {
		if (flightInfoConfig == null)
			return null;
		return flightInfoConfig.getFieldDefinitions();
	}

	/**
	 * Standardized initialization.
	 */
	public void init(prog.Controller c, prog.Service s, prog.config.OverlaySettings settings) {
		this.service = s;
		this.config = c;
		this.flightInfoConfig = ui.model.FlightInfoConfig.createDefault(c, settings.getGroupConfig());

		// Standardize style/font keys from Config object
		this.numFontKey = flightInfoConfig.numFontKey;
		this.labelFontKey = flightInfoConfig.labelFontKey;
		this.columnKey = flightInfoConfig.columnKey;
		this.edgeKey = flightInfoConfig.edgeKey;
		this.defaultShowEdge = flightInfoConfig.showEdge;
		this.title = flightInfoConfig.title;

		setOverlaySettings(settings);
		super.init(c, c.globalPool);

		if (s != null) {
			setVisible(true);
		}
	}

	public void onFlightData(prog.event.FlightDataEvent event) {
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
	 * Reinitialize with current config.
	 */
	@Override
	public void reinitConfig() {
		if (flightInfoConfig == null)
			return;

		super.reinitConfig();

		// Bind fields based on current telemetry source
		prog.config.ConfigLoader.GroupConfig groupConfig = flightInfoConfig.groupConfig;
		if (groupConfig != null && this.service != null) {
			bindDynamicFields(this.service, groupConfig.rows);
			repaint();
		}
	}

	private void bindDynamicFields(ui.model.TelemetrySource s,
			java.util.List<prog.config.ConfigLoader.RowConfig> rows) {
		ui.model.FieldManager fm = this.fieldManager;
		for (prog.config.ConfigLoader.RowConfig row : rows) {
			if ("DATA".equals(row.type) && row.property != null && !row.property.isEmpty()) {
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

				fm.setFieldVisible(row.property, row.getBool());
			}
			if (row.children != null && !row.children.isEmpty()) {
				bindDynamicFields(s, row.children);
			}
		}
	}

	// Manual loadPosition and saveCurrentPosition overrides removed.
	// DraggableOverlay handles this via OverlaySettings automatically.
}