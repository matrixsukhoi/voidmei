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
	 * Initialize with FlightInfoConfig.
	 */
	public void init(ConfigProvider config, AttributePool pool, FlightInfoConfig flightInfoConfig) {
		this.flightInfoConfig = flightInfoConfig;

		// Set config keys from FlightInfoConfig
		this.numFontKey = flightInfoConfig.numFontKey;
		this.labelFontKey = flightInfoConfig.labelFontKey;
		// this.fontAddKey = flightInfoConfig.fontAddKey;
		this.columnKey = flightInfoConfig.columnKey;
		this.edgeKey = flightInfoConfig.edgeKey;
		this.defaultShowEdge = flightInfoConfig.showEdge;
		this.title = flightInfoConfig.title;

		// Set position keys - Deprecated/Removed in favor of GroupConfig
		// setPositionKeys(flightInfoConfig.posXKey, flightInfoConfig.posYKey);

		// Call parent init
		super.init(config, pool);

		// Initialize overlaySettings for FieldOverlay support
		if (flightInfoConfig.groupConfig != null && config instanceof prog.Controller) {
			setOverlaySettings(
					((prog.Controller) config).configService.getOverlaySettings(flightInfoConfig.groupConfig.title));
		}
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

	@Override
	protected int[] loadPosition(int defaultX, int defaultY) {
		if (flightInfoConfig != null && flightInfoConfig.groupConfig != null) {
			int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
			int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
			int x = (int) (flightInfoConfig.groupConfig.x * screenW);
			int y = (int) (flightInfoConfig.groupConfig.y * screenH);
			return new int[] { x, y };
		}
		return super.loadPosition(defaultX, defaultY);
	}

	@Override
	public void saveCurrentPosition() {
		if (flightInfoConfig != null && flightInfoConfig.groupConfig != null) {
			int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
			int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;

			flightInfoConfig.groupConfig.x = (double) getLocation().x / screenW;
			flightInfoConfig.groupConfig.y = (double) getLocation().y / screenH;

			if (config instanceof prog.Controller) {
				((prog.Controller) config).configService.saveLayoutConfig();
			}
		} else {
			super.saveCurrentPosition();
		}
	}
}