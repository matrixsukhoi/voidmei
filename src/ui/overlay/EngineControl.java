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

	// Config
	public prog.config.ConfigLoader.GroupConfig groupConfig;
	private int fontsize;
	private Font fontLabel;

	// Layout
	int WIDTH;
	int HEIGHT;
	public int rowNum;
	public int columnNum;

	// Logic State (derived from event data)
	private boolean isJet;
	private boolean jetLabelUpdated;

	// Gauge Type Constants
	public static final int GAUGE_THROTTLE = 0;
	public static final int GAUGE_PITCH = 1;
	public static final int GAUGE_MIXTURE = 2;
	public static final int GAUGE_RADIATOR = 3;
	public static final int GAUGE_COMPRESSOR = 4;
	public static final int GAUGE_FUEL = 5;

	private java.util.List<GaugeField> gaugeFields;

	// Position save callback
	private Runnable onPositionSave;

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
		return new ArrayList<>();
	}

	// --- Initialization ---

	/**
	 * Initialize for game mode (event-driven).
	 */
	public void init(ConfigProvider config, prog.config.ConfigLoader.GroupConfig groupConfig, Runnable onPositionSave) {
		this.config = config;
		this.groupConfig = groupConfig;
		this.onPositionSave = onPositionSave;

		setupTransparentWindow();

		panel = new WebPanel() {
			private static final long serialVersionUID = 1L;

			@Override
			public void paintComponent(Graphics g) {
				// Clear the background to prevent ghost trails on transparent components
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

	/**
	 * Initialize for preview mode.
	 */
	public void initPreview(ConfigProvider config, prog.config.ConfigLoader.GroupConfig groupConfig,
			Runnable onPositionSave) {
		init(config, groupConfig, onPositionSave);
		applyPreviewStyle();
		updateGaugesPreview();
		setupDragListeners();
		setVisible(true);
	}

	@Override
	public void reinitConfig() {
		// Font configuration
		String labelFontVal = config != null ? config.getConfig(labelFontKey) : null;
		String fontAddVal = config != null ? config.getConfig(fontAddKey) : null;

		String FontName = (labelFontVal != null && !labelFontVal.isEmpty()) ? labelFontVal : "Microsoft YaHei";
		int fontadd = 0;
		if (fontAddVal != null && !fontAddVal.isEmpty()) {
			try {
				fontadd = Integer.parseInt(fontAddVal);
			} catch (NumberFormatException e) {
				fontadd = 0;
			}
		}

		fontsize = 24 + fontadd;
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontsize / 2.0f));

		// Position configuration
		int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
		int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
		int lx = 0, ly = 0;
		if (groupConfig != null) {
			lx = (int) (groupConfig.x * screenW);
			ly = (int) (groupConfig.y * screenH);
		}

		rowNum = 0;
		columnNum = 0;
		initGaugeFields();

		WIDTH = fontsize * 8;
		HEIGHT = (int) ((fontsize * 4 + (fontsize * 9) >> 1) + (rowNum + 1) * (1 * fontsize + (fontsize >> 2)));

		boolean showEdge = edgeKey != null && config != null && "true".equals(config.getConfig(edgeKey));
		setShadeWidth(showEdge ? 10 : 0);

		setSize(WIDTH, HEIGHT);
		setLocation(lx, ly);

		updateGaugesPreview();
		repaint();
	}

	private void initGaugeFields() {
		gaugeFields = new java.util.ArrayList<>();
		String tmp;

		tmp = config != null ? config.getConfig("disableEngineInfoThrottle") : "";
		if (!("true".equals(tmp))) {
			gaugeFields.add(new GaugeField("throttle", Lang.eThrottle, "%", GAUGE_THROTTLE, 110, false));
			columnNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoPitch") : "";
		if (!("true".equals(tmp))) {
			gaugeFields.add(new GaugeField("pitch", Lang.eProppitch, "%", GAUGE_PITCH, 100, false));
			columnNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoMixture") : "";
		if (!("true".equals(tmp))) {
			gaugeFields.add(new GaugeField("mixture", Lang.eMixture, "%", GAUGE_MIXTURE, 120, true));
			rowNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoRadiator") : "";
		if (!("true".equals(tmp))) {
			gaugeFields.add(new GaugeField("radiator", Lang.eRadiator, "%", GAUGE_RADIATOR, 100, true));
			rowNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoCompressor") : "";
		if (!("true".equals(tmp))) {
			gaugeFields.add(new GaugeField("compressor", Lang.eCompressor, "", GAUGE_COMPRESSOR, 1, true));
			rowNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoLFuel") : "";
		if (!("true".equals(tmp))) {
			gaugeFields.add(new GaugeField("fuel", Lang.eFuelPer, "%", GAUGE_FUEL, 100, true));
			rowNum++;
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

			// Skip specific gauges for jets
			if (isJet && (gf.gaugeType == GAUGE_RADIATOR || gf.gaugeType == GAUGE_COMPRESSOR
					|| gf.gaugeType == GAUGE_MIXTURE)) {
				continue;
			}

			// Skip compressor if stage is 0 (will be handled in onFlightData visibility)
			if (!gf.visible) {
				continue;
			}

			if (gf.isHorizontal) {
				gauge.vertical = false;
				gauge.draw(g2d, x, y + dy, 4 * fontsize, fontsize >> 1, fontLabel, fontLabel);
				dy += 1 * fontsize + (fontsize >> 2);
			} else {
				gauge.vertical = true;
				gauge.draw(g2d, x + dx, y, 4 * fontsize, fontsize >> 1, fontLabel, fontLabel);
				dx += (5 * fontsize) >> 1;
			}
		}
	}

	// --- Event Handling ---

	@Override
	protected void subscribeToEvents() {
		FlightDataBus.getInstance().register(this);
	}

	@Override
	public void onFlightData(FlightDataEvent event) {
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

		// Check jet status
		String engineCheckDone = data.get("engine_check_done");
		if ("true".equals(engineCheckDone) && !jetLabelUpdated) {
			String isJetStr = data.get("is_jet");
			isJet = "true".equals(isJetStr);
			if (isJet) {
				for (GaugeField gf : gaugeFields) {
					if (gf.gaugeType == GAUGE_PITCH) {
						gf.gauge.label = Lang.eThurstP;
					}
				}
			}
			jetLabelUpdated = true;
		}

		for (GaugeField gf : gaugeFields) {
			switch (gf.gaugeType) {
				case GAUGE_THROTTLE:
					updateGaugeFromData(gf, data, "throttle", "throttle_int");
					break;
				case GAUGE_PITCH:
					if (!isJet) {
						updateGaugeFromData(gf, data, "rpm_throttle", "rpm_throttle_int");
					} else {
						updateGaugeFromData(gf, data, "thrust_percent", "thrust_percent_int");
					}
					break;
				case GAUGE_MIXTURE:
					updateGaugeFromData(gf, data, "mixture", "mixture_int");
					// Hide if negative
					String mixInt = data.get("mixture_int");
					if (mixInt != null) {
						try {
							gf.visible = Integer.parseInt(mixInt) >= 0;
						} catch (NumberFormatException e) {
							gf.visible = true;
						}
					}
					break;
				case GAUGE_RADIATOR:
					updateGaugeFromData(gf, data, "radiator", "radiator_int");
					break;
				case GAUGE_COMPRESSOR:
					String compInt = data.get("compressor_int");
					if (compInt != null) {
						try {
							int stage = Integer.parseInt(compInt);
							gf.visible = stage > 0;
							if (stage - 1 > gf.gauge.maxValue) {
								gf.gauge.maxValue = stage - 1;
							}
							gf.gauge.update(stage - 1, String.format("%3s", data.get("compressor")));
						} catch (NumberFormatException e) {
							gf.visible = false;
						}
					}
					break;
				case GAUGE_FUEL:
					updateGaugeFromData(gf, data, "fuel_percent", "fuel_percent_int");
					break;
			}
		}
	}

	private void updateGaugeFromData(GaugeField gf, Map<String, String> data, String strKey, String intKey) {
		String strVal = data.get(strKey);
		String intVal = data.get(intKey);
		if (strVal != null && intVal != null) {
			try {
				int value = Integer.parseInt(intVal);
				gf.gauge.update(value, String.format("%3s", strVal));
			} catch (NumberFormatException e) {
				gf.gauge.update(0, String.format("%3s", strVal));
			}
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

	// --- Position Saving ---

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

	public void fclose() {
		dispose();
	}

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