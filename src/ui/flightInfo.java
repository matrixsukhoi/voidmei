package ui;

import java.awt.Color;
import java.awt.Graphics;
import java.awt.Graphics2D;

import com.alee.laf.panel.WebPanel;
import ui.base.DraggableOverlay;
import ui.model.ConfigProvider;
import ui.model.DefaultFieldManager;
import ui.model.FieldManager;
import ui.model.FlightInfoConfig;
import ui.renderer.BOSStyleRenderer;
import ui.renderer.FlightInfoRenderer;
import ui.renderer.RenderContext;

/**
 * FlightInfo overlay window displaying real-time flight data.
 * Uses pluggable FieldManager for data and FlightInfoRenderer for styling.
 * 
 * Extends DraggableOverlay for common window management functionality.
 */
public class flightInfo extends DraggableOverlay {
	private static final long serialVersionUID = 6759127498151892589L;

	// Data provider
	// private FlightDataProvider dataProvider;
	private parser.AttributePool poolSource;
	private String naString = "-";

	// UI components
	private WebPanel panel;
	// private Container root;

	// Pluggable components
	private FieldManager fieldManager;
	private FlightInfoRenderer renderer;
	// Configuration object
	private FlightInfoConfig flightInfoConfig;

	private RenderContext renderContext;
	private int[] renderOffset;

	public flightInfo() {
		super();
		this.renderer = new BOSStyleRenderer();
		// Position keys are initialized in reinitConfig from config object
	}

	/**
	 * Initialize for preview mode (with drag support).
	 */
	public void initPreview(ConfigProvider config, parser.AttributePool pool, FlightInfoConfig flightInfoConfig) {
		// Use default configuration for preview
		init(config, pool, flightInfoConfig);
		applyPreviewStyle();
		setupDragListeners();
		setVisible(true);
		// Force re-layout after made visible to ensure correct Metrics/Insets
		reinitConfig();
	}

	/**
	 * Initialize all flight data fields.
	 */
	private void initFields() {
		fieldManager.clearAll();
		if (flightInfoConfig == null)
			return;

		for (ui.model.FieldDefinition def : flightInfoConfig.getFieldDefinitions()) {
			fieldManager.addField(def.key, def.label, def.unit, def.configKey, def.hideWhenNA);
		}
	}

	/**
	 * Update all field values from data provider.
	 * Called by uiThread to refresh display.
	 */
	public void updateString() {
		updateData();
	}

	@Override
	protected void updateData() {
		// if (dataProvider == null) return;

		// Iterate over fields in manager instead of hardcoded keys
		for (ui.model.FlightField field : fieldManager.getFields()) {
			String val = naString;
			if (poolSource != null) {
				Object obj = poolSource.getValue(field.key);
				if (obj != null && !"N/A".equals(obj)) {
					val = obj.toString();
				}
			}
			fieldManager.updateField(field.key, val, naString);
		}
	}

	public void reinitConfig() {
		if (flightInfoConfig == null)
			return;

		// Create render context from config using keys from FlightInfoConfig
		renderContext = RenderContext.fromConfig(config, this,
				flightInfoConfig.numFontKey,
				flightInfoConfig.labelFontKey,
				flightInfoConfig.fontAddKey,
				flightInfoConfig.columnKey);

		// Initialize Keys
		setPositionKeys(flightInfoConfig.posXKey, flightInfoConfig.posYKey);

		// Reinitialize fields
		initFields();

		// Set edge style
		boolean showEdge = false;
		if (config != null) {
			// Check config override first, then fall back to default
			String edgeVal = config.getConfig(flightInfoConfig.edgeKey);
			if (edgeVal != null && !edgeVal.isEmpty()) {
				showEdge = "true".equals(edgeVal);
			} else {
				showEdge = flightInfoConfig.showEdge;
			}
		}

		if (showEdge) {
			setShadeWidth(10);
		} else {
			setShadeWidth(0);
		}

		// Update Size and Position using legacy setBounds to preserve dynamic
		// properties
		// But executed in correct lifecycle (after realization)
		int width = renderContext.getTotalWidth();
		int height = renderContext.getTotalHeight(fieldManager.size());

		int[] pos = loadPosition(0, 0);
		this.setBounds(pos[0], pos[1], width, height);

		repaint();
	}

	/**
	 * Main initialization with interfaces.
	 */
	public void init(ConfigProvider config, parser.AttributePool pool, FlightInfoConfig flightInfoConfig) {
		this.config = config;
		this.flightInfoConfig = flightInfoConfig;
		this.poolSource = pool;

		// Create field manager with config
		fieldManager = new DefaultFieldManager(config);

		// Setup transparent window
		setupTransparentWindow();

		renderOffset = new int[2];

		// Create render panel
		panel = new WebPanel() {
			private static final long serialVersionUID = -9061280572815010060L;

			@Override
			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				renderer.render(g2d, fieldManager.getFields(), renderContext, renderOffset);
			}
		};
		panel.setOpaque(false);
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
		this.add(panel);

		setTitle(flightInfoConfig.title);

		// Initialize configuration (After panel is created)
		reinitConfig();

		setVisible(true);

		// Ensure layout is refreshed after visible
		reinitConfig();
	}

	@Override
	public void drawTick() {
		if (panel != null) { // Changed from root to panel
			panel.repaint();
		}
	}
}