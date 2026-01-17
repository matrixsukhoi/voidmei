package ui.overlay;

import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

import com.alee.laf.panel.WebPanel;

import prog.Application;
import prog.i18n.Lang;
import prog.config.ConfigProvider;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import ui.base.FieldOverlay;
import ui.model.FieldDefinition;
import ui.model.GaugeField;
import ui.renderer.LinearGaugeRenderer;
import ui.renderer.OverlayRenderer;

/**
 * Engine Control overlay for displaying throttle, pitch, mixture, radiator,
 * etc.
 * Fully event-driven: all data comes from FlightDataEvent.
 */
public class EngineControl extends FieldOverlay {

	private static final long serialVersionUID = 3063042782594625576L;

	// --- Constants ---
	private static final int BASE_FONT_SIZE = 24;
	private static final int WIDTH_MULTIPLIER = 8;
	private static final int SHADE_WIDTH = 10;
	private static final long DEFAULT_REFRESH_INTERVAL = 100; // ms
	private static final double ENGINE_REFRESH_MULTIPLIER = 2.0; // freqService * 2

	// Gauge Type Enum (replaces magic int constants)
	public enum GaugeType {
		THROTTLE, PITCH, MIXTURE, RADIATOR, COMPRESSOR, FUEL
	}

	// --- Instance Fields ---

	// Throttling
	private long refreshInterval = DEFAULT_REFRESH_INTERVAL;
	private long lastRefreshTime = 0;

	// Config (package-private for testing, not public API)
	prog.config.ConfigLoader.GroupConfig groupConfig;
	private int fontsize;
	private Font fontLabel;

	// Layout (package-private)
	int width;
	int height;
	private int rowNum;
	private int columnNum;

	// Logic State
	private boolean isJet;
	private boolean jetLabelUpdated;

	// Gauge data
	private List<GaugeField> gaugeFields;

	// Callbacks
	private Runnable onPositionSave;

	// --- Constructor ---

	public EngineControl() {
		super();
		this.title = "Engine Control";
		this.numFontKey = "NumFont";
		this.labelFontKey = "FontName";
		this.fontAddKey = "fontadd";
		this.edgeKey = "engineInfoEdge";
	}

	// --- FieldOverlay Abstract Methods ---

	@Override
	protected OverlayRenderer createRenderer() {
		return new LinearGaugeRenderer();
	}

	@Override
	protected List<FieldDefinition> getFieldDefinitions() {
		return new ArrayList<>(); // Not used - we use GaugeField
	}

	// --- Initialization ---

	public void init(ConfigProvider config, prog.config.ConfigLoader.GroupConfig groupConfig, Runnable onPositionSave) {
		this.config = config;
		this.groupConfig = groupConfig;
		this.onPositionSave = onPositionSave;

		setupTransparentWindow();

		panel = new WebPanel() {
			private static final long serialVersionUID = 1L;

			@Override
			public void paintComponent(Graphics g) {
				super.paintComponent(g);
				Graphics2D g2d = (Graphics2D) g;
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
				drawGauges(g2d, fontsize >> 1, (fontsize * 4) + ((fontsize * 9) >> 1));
			}
		};
		panel.setOpaque(false);
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
		this.add(panel);

		reinitConfig();
		subscribeToEvents();
		setVisible(true);
	}

	public void initPreview(ConfigProvider config, prog.config.ConfigLoader.GroupConfig groupConfig,
			Runnable onPositionSave) {
		init(config, groupConfig, onPositionSave);
		applyPreviewStyle();
		updateGaugesPreview();
		setupDragListeners();
		setVisible(true);
	}

	// --- Configuration ---

	@Override
	public void reinitConfig() {
		loadFontConfig();
		loadRefreshInterval();
		initGaugeFields();
		calculateLayout();
		loadPosition(); // After layout to avoid drift from shade width
		updateGaugesPreview();
		repaint();
	}

	private void loadFontConfig() {
		String labelFontVal = getConfigSafe(labelFontKey);
		String fontAddVal = getConfigSafe(fontAddKey);

		String fontName = !labelFontVal.isEmpty() ? labelFontVal : "Microsoft YaHei";
		int fontadd = parseIntSafe(fontAddVal, 0);

		fontsize = BASE_FONT_SIZE + fontadd;
		fontLabel = new Font(fontName, Font.BOLD, Math.round(fontsize / 2.0f));
	}

	private void loadRefreshInterval() {
		String intervalVal = getConfigSafe("Interval");
		if (!intervalVal.isEmpty()) {
			long freqService = parseLongSafe(intervalVal, DEFAULT_REFRESH_INTERVAL);
			refreshInterval = (long) (freqService * ENGINE_REFRESH_MULTIPLIER);
		}
	}

	private void loadPosition() {
		int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
		int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
		int lx = 0, ly = 0;
		if (groupConfig != null) {
			lx = (int) (groupConfig.x * screenW);
			ly = (int) (groupConfig.y * screenH);
		}
		setLocation(lx, ly);
	}

	private void calculateLayout() {
		width = fontsize * WIDTH_MULTIPLIER;
		height = (int) ((fontsize * 4 + (fontsize * 9) >> 1) + (rowNum + 1) * (1 * fontsize + (fontsize >> 2)));

		boolean showEdge = "true".equals(getConfigSafe(edgeKey));
		setShadeWidth(showEdge ? SHADE_WIDTH : 0);

		setSize(width, height);
	}

	private void initGaugeFields() {
		gaugeFields = new ArrayList<>();
		rowNum = 0;
		columnNum = 0;

		// Define gauges with config check
		addGaugeIfEnabled("disableEngineInfoThrottle", "throttle", Lang.eThrottle, "%",
				GaugeType.THROTTLE.ordinal(), 110, false);
		addGaugeIfEnabled("disableEngineInfoPitch", "pitch", Lang.eProppitch, "%",
				GaugeType.PITCH.ordinal(), 100, false);
		addGaugeIfEnabled("disableEngineInfoMixture", "mixture", Lang.eMixture, "%",
				GaugeType.MIXTURE.ordinal(), 120, true);
		addGaugeIfEnabled("disableEngineInfoRadiator", "radiator", Lang.eRadiator, "%",
				GaugeType.RADIATOR.ordinal(), 100, true);
		addGaugeIfEnabled("disableEngineInfoCompressor", "compressor", Lang.eCompressor, "",
				GaugeType.COMPRESSOR.ordinal(), 1, true);
		addGaugeIfEnabled("disableEngineInfoLFuel", "fuel", Lang.eFuelPer, "%",
				GaugeType.FUEL.ordinal(), 100, true);
	}

	private void addGaugeIfEnabled(String disableKey, String key, String label, String unit,
			int gaugeType, int maxValue, boolean isHorizontal) {
		if (!"true".equals(getConfigSafe(disableKey))) {
			gaugeFields.add(new GaugeField(key, label, unit, gaugeType, maxValue, isHorizontal));
			if (isHorizontal) {
				rowNum++;
			} else {
				columnNum++;
			}
		}
	}

	// --- Helper Methods ---

	private String getConfigSafe(String key) {
		return config != null ? config.getConfig(key) : "";
	}

	private int parseIntSafe(String val, int defaultVal) {
		if (val == null || val.isEmpty())
			return defaultVal;
		try {
			return Integer.parseInt(val);
		} catch (NumberFormatException e) {
			return defaultVal;
		}
	}

	private long parseLongSafe(String val, long defaultVal) {
		if (val == null || val.isEmpty())
			return defaultVal;
		try {
			return Long.parseLong(val);
		} catch (NumberFormatException e) {
			return defaultVal;
		}
	}

	// --- Drawing ---

	private void drawGauges(Graphics2D g2d, int x, int y) {
		if (gaugeFields == null)
			return;

		int dx = 0;
		int dy = fontsize >> 1;

		for (GaugeField gf : gaugeFields) {
			ui.component.LinearGauge gauge = gf.gauge;

			// Skip for jets
			if (isJet && isJetHiddenGauge(gf.gaugeType)) {
				continue;
			}

			if (!gf.visible) {
				continue;
			}

			if (gf.isHorizontal) {
				gauge.vertical = false;
				gauge.draw(g2d, x, y + dy, 4 * fontsize, fontsize >> 1, fontLabel, fontLabel);
				dy += fontsize + (fontsize >> 2);
			} else {
				gauge.vertical = true;
				gauge.draw(g2d, x + dx, y, 4 * fontsize, fontsize >> 1, fontLabel, fontLabel);
				dx += (5 * fontsize) >> 1;
			}
		}
	}

	private boolean isJetHiddenGauge(int gaugeType) {
		return gaugeType == GaugeType.RADIATOR.ordinal()
				|| gaugeType == GaugeType.COMPRESSOR.ordinal()
				|| gaugeType == GaugeType.MIXTURE.ordinal();
	}

	// --- Event Handling ---

	@Override
	protected void subscribeToEvents() {
		FlightDataBus.getInstance().register(this);
	}

	@Override
	public void onFlightData(FlightDataEvent event) {
		// Throttle updates
		long now = System.currentTimeMillis();
		if (now - lastRefreshTime < refreshInterval) {
			return;
		}
		lastRefreshTime = now;

		javax.swing.SwingUtilities.invokeLater(() -> {
			updateGaugesFromEvent(event);
			if (panel != null)
				panel.repaint();
		});
	}

	private void updateGaugesFromEvent(FlightDataEvent event) {
		if (gaugeFields == null)
			return;

		Map<String, String> data = event.getData();

		// Check jet status once
		if (!jetLabelUpdated && "true".equals(data.get("engine_check_done"))) {
			isJet = "true".equals(data.get("is_jet"));
			if (isJet) {
				for (GaugeField gf : gaugeFields) {
					if (gf.gaugeType == GaugeType.PITCH.ordinal()) {
						gf.gauge.label = Lang.eThurstP;
					}
				}
			}
			jetLabelUpdated = true;
		}

		// Update each gauge
		for (GaugeField gf : gaugeFields) {
			updateGaugeByType(gf, data);
		}
	}

	private void updateGaugeByType(GaugeField gf, Map<String, String> data) {
		GaugeType type = GaugeType.values()[gf.gaugeType];
		switch (type) {
			case THROTTLE:
				updateGaugeFromData(gf, data, "throttle", "throttle_int");
				break;
			case PITCH:
				if (!isJet) {
					updateGaugeFromData(gf, data, "rpm_throttle", "rpm_throttle_int");
				} else {
					updateGaugeFromData(gf, data, "thrust_percent", "thrust_percent_int");
				}
				break;
			case MIXTURE:
				updateGaugeFromData(gf, data, "mixture", "mixture_int");
				gf.visible = parseIntSafe(data.get("mixture_int"), 0) >= 0;
				break;
			case RADIATOR:
				updateGaugeFromData(gf, data, "radiator", "radiator_int");
				break;
			case COMPRESSOR:
				int stage = parseIntSafe(data.get("compressor_int"), 0);
				gf.visible = stage > 0;
				if (stage - 1 > gf.gauge.maxValue) {
					gf.gauge.maxValue = stage - 1;
				}
				gf.gauge.update(stage - 1, String.format("%3s", data.get("compressor")));
				break;
			case FUEL:
				updateGaugeFromData(gf, data, "fuel_percent", "fuel_percent_int");
				break;
		}
	}

	private void updateGaugeFromData(GaugeField gf, Map<String, String> data, String strKey, String intKey) {
		String strVal = data.get(strKey);
		String intVal = data.get(intKey);
		if (strVal != null && intVal != null) {
			int value = parseIntSafe(intVal, 0);
			gf.gauge.update(value, String.format("%3s", strVal));
		}
	}

	private void updateGaugesPreview() {
		if (gaugeFields == null)
			return;
		for (GaugeField gf : gaugeFields) {
			gf.gauge.update(gf.gauge.maxValue / 2, "PRE");
			gf.visible = true;
		}
	}

	// --- Position ---

	@Override
	public void saveCurrentPosition() {
		if (groupConfig != null) {
			int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
			int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
			groupConfig.x = (double) getLocation().x / screenW;
			groupConfig.y = (double) getLocation().y / screenH;
			if (onPositionSave != null) {
				onPositionSave.run();
			}
		} else {
			super.saveCurrentPosition();
		}
	}

	// --- Cleanup ---

	@Override
	public void dispose() {
		FlightDataBus.getInstance().unregister(this);
		super.dispose();
	}

	@Override
	public void run() {
		// Event-driven - no polling
	}
}