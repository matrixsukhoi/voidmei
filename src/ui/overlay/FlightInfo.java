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
	}

	// Manual loadPosition and saveCurrentPosition overrides removed.
	// DraggableOverlay handles this via OverlaySettings automatically.
}