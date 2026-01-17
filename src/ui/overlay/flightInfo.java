package ui.overlay;

import java.util.List;

import parser.AttributePool;
import ui.base.FieldOverlay;
import prog.config.ConfigProvider;
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
public class flightInfo extends FieldOverlay {
	private static final long serialVersionUID = 6759127498151892589L;

	private FlightInfoConfig flightInfoConfig;

	public flightInfo() {
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
	 * Initialize with FlightInfoConfig.
	 */
	public void init(ConfigProvider config, AttributePool pool, FlightInfoConfig flightInfoConfig) {
		this.flightInfoConfig = flightInfoConfig;

		// Set config keys from FlightInfoConfig
		this.numFontKey = flightInfoConfig.numFontKey;
		this.labelFontKey = flightInfoConfig.labelFontKey;
		this.fontAddKey = flightInfoConfig.fontAddKey;
		this.columnKey = flightInfoConfig.columnKey;
		this.edgeKey = flightInfoConfig.edgeKey;
		this.defaultShowEdge = flightInfoConfig.showEdge;
		this.title = flightInfoConfig.title;

		// Set position keys
		setPositionKeys(flightInfoConfig.posXKey, flightInfoConfig.posYKey);

		// Call parent init
		super.init(config, pool);
	}

	/**
	 * Initialize for preview mode.
	 */
	public void initPreview(ConfigProvider config, AttributePool pool, FlightInfoConfig flightInfoConfig) {
		init(config, pool, flightInfoConfig);
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
}