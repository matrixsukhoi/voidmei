package ui;

import java.awt.Color;
import java.awt.Container;
import java.awt.Graphics;
import java.awt.Graphics2D;

import com.alee.laf.panel.WebPanel;
import ui.base.DraggableOverlay;
import ui.model.ConfigProvider;
import ui.model.DefaultFieldManager;
import ui.model.FieldManager;
import ui.model.FlightDataProvider;
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
	private FlightDataProvider dataProvider;

	// UI components
	private WebPanel panel;
	private Container root;

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
		setPositionKeys("flightInfoX", "flightInfoY");
	}

	/**
	 * Initialize for preview mode (with drag support).
	 */
	public void initPreview(ConfigProvider config) {
		// Use default configuration for preview
		init(config, null, FlightInfoConfig.createDefault(config));
		applyPreviewStyle();
		setupDragListeners();
		setVisible(true);
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
		if (dataProvider == null)
			return;

		String naString = dataProvider.getNAString();

		// Iterate over fields in manager instead of hardcoded keys
		// Note: FieldManager needs to support iteration, which we added via getFields()
		for (ui.model.FlightField field : fieldManager.getFields()) {
			String val = "---";
			java.util.function.Function<FlightDataProvider, String> binding = ui.model.FlightDataBindings
					.get(field.key);
			if (binding != null) {
				val = binding.apply(dataProvider);
			}
			fieldManager.updateField(field.key, val, naString);
		}
	}

	/**
	 * Reload configuration and reinitialize.
	 */
	public void reinitConfig() {
		if (flightInfoConfig == null)
			return;

		// Create render context from config using keys from FlightInfoConfig
		renderContext = RenderContext.fromConfig(config, this,
				flightInfoConfig.numFontKey,
				flightInfoConfig.labelFontKey,
				flightInfoConfig.fontAddKey,
				flightInfoConfig.columnKey);

		// Load position
		// Use keys from FlightInfoConfig
		setPositionKeys(flightInfoConfig.posXKey, flightInfoConfig.posYKey);
		int[] pos = loadPosition(0, 0);

		// Reinitialize fields
		initFields();

		// Set window bounds
		int width = renderContext.getTotalWidth();
		int height = renderContext.getTotalHeight(fieldManager.size());
		this.setBounds(pos[0], pos[1], width, height);

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

		repaint();
	}

	/**
	 * Main initialization with interfaces.
	 */
	public void init(ConfigProvider config, FlightDataProvider dataProvider, FlightInfoConfig fiConfig) {
		this.config = config;
		this.dataProvider = dataProvider;
		this.flightInfoConfig = fiConfig;

		// Create field manager with config
		fieldManager = new DefaultFieldManager(config);

		// Initialize configuration
		reinitConfig();

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

		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
		this.add(panel);

		setTitle("flightInfo");
		root = this.getContentPane();

		if (dataProvider != null) {
			setVisible(true);
		}
	}

	@Override
	public void drawTick() {
		if (root != null) {
			root.repaint();
		}
	}
}