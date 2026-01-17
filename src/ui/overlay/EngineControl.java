package ui.overlay;

import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.util.ArrayList;
import java.util.List;

import com.alee.laf.panel.WebPanel;

import parser.Blkx;
import prog.Application;
import prog.Controller;
import prog.i18n.Lang;
import prog.Service;
import prog.config.ConfigProvider;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import ui.base.FieldOverlay;
import ui.model.DataField;
import ui.model.FieldDefinition;
import ui.model.GaugeField;
import ui.renderer.LinearGaugeRenderer;
import ui.renderer.OverlayRenderer;
import ui.renderer.RenderContext;

/**
 * Engine Control overlay for displaying throttle, pitch, mixture, radiator,
 * etc.
 * Now extends FieldOverlay for the data-driven architecture.
 */
public class EngineControl extends FieldOverlay {

	private static final long serialVersionUID = 3063042782594625576L;

	// Legacy references (for direct Service access)
	public Controller xc;
	public Service s;
	public Blkx p;

	// Config
	public prog.config.ConfigLoader.GroupConfig groupConfig;
	private int fontsize;
	private Font fontLabel;

	// Layout
	int WIDTH;
	int HEIGHT;
	public int rowNum;
	public int columnNum;

	// Logic State
	private boolean isJet;
	private boolean jetChecked;

	// Gauge Type Constants
	public static final int GAUGE_THROTTLE = 0;
	public static final int GAUGE_PITCH = 1;
	public static final int GAUGE_MIXTURE = 2;
	public static final int GAUGE_RADIATOR = 3;
	public static final int GAUGE_COMPRESSOR = 4;
	public static final int GAUGE_FUEL = 5;

	private java.util.List<GaugeField> gaugeFields;

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
		// Not used directly - we use GaugeField instead
		return new ArrayList<>();
	}

	// --- Custom Initialization for Engine Control ---

	/**
	 * Legacy initialization method for compatibility with OverlayManager.
	 */
	public void init(Controller c, Service ts, Blkx tp, prog.config.ConfigLoader.GroupConfig groupConfig) {
		this.xc = c;
		this.s = ts;
		this.p = tp;
		this.groupConfig = groupConfig;

		// Bridge to FieldOverlay's ConfigProvider-based init
		ConfigProvider configBridge = new ConfigProvider() {
			@Override
			public String getConfig(String key) {
				return xc != null ? xc.getconfig(key) : "";
			}
		};
		this.config = configBridge;

		setupTransparentWindow();

		panel = new WebPanel() {
			private static final long serialVersionUID = 1L;

			@Override
			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);

				drawTPMRC(g2d, fontsize >> 1, (fontsize * 4) + ((fontsize * 9) >> 1));
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

	public void initPreview(Controller c, prog.config.ConfigLoader.GroupConfig groupConfig) {
		this.groupConfig = groupConfig;
		init(c, null, null, groupConfig);
		applyPreviewStyle();
		updateGauges();
		setupDragListeners();
		setVisible(true);
	}

	@Override
	public void reinitConfig() {
		// Font configuration
		String numFontVal = config != null ? config.getConfig(numFontKey) : null;
		String labelFontVal = config != null ? config.getConfig(labelFontKey) : null;
		String fontAddVal = config != null ? config.getConfig(fontAddKey) : null;

		String NumFont = (numFontVal != null && !numFontVal.isEmpty()) ? numFontVal : "Consolas";
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

		updateGauges();
		repaint();
	}

	/**
	 * Initialize gauge fields based on configuration.
	 */
	private void initGaugeFields() {
		gaugeFields = new java.util.ArrayList<>();
		String tmp;

		tmp = config != null ? config.getConfig("disableEngineInfoThrottle") : "";
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gaugeFields.add(new GaugeField("throttle", Lang.eThrottle, "%", GAUGE_THROTTLE, 110, false));
			columnNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoPitch") : "";
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gaugeFields.add(new GaugeField("pitch", Lang.eProppitch, "%", GAUGE_PITCH, 100, false));
			columnNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoMixture") : "";
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gaugeFields.add(new GaugeField("mixture", Lang.eMixture, "%", GAUGE_MIXTURE, 120, true));
			rowNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoRadiator") : "";
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gaugeFields.add(new GaugeField("radiator", Lang.eRadiator, "%", GAUGE_RADIATOR, 100, true));
			rowNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoCompressor") : "";
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gaugeFields.add(new GaugeField("compressor", Lang.eCompressor, "", GAUGE_COMPRESSOR, 1, true));
			rowNum++;
		}

		tmp = config != null ? config.getConfig("disableEngineInfoLFuel") : "";
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gaugeFields.add(new GaugeField("fuel", Lang.eFuelPer, "%", GAUGE_FUEL, 100, true));
			rowNum++;
		}
	}

	// --- Drawing ---

	public void drawTPMRC(Graphics2D g2d, int x, int y) {
		int dx = 0;
		int dy = fontsize >> 1;

		for (GaugeField gf : gaugeFields) {
			ui.component.LinearGauge gauge = gf.gauge;

			// Skip specific gauges for jets
			if (isJet && (gf.gaugeType == GAUGE_RADIATOR || gf.gaugeType == GAUGE_COMPRESSOR
					|| gf.gaugeType == GAUGE_MIXTURE)) {
				continue;
			}

			// Skip compressor if stage is 0
			if (gf.gaugeType == GAUGE_COMPRESSOR && s != null && s.sState.compressorstage == 0) {
				continue;
			}
			// Skip mixture if negative
			if (gf.gaugeType == GAUGE_MIXTURE && s != null && s.sState.mixture < 0) {
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
			updateGauges();
			if (panel != null)
				panel.repaint();
		});
	}

	public void updateGauges() {
		if (gaugeFields == null)
			return;

		// Preview Mode (s is null)
		if (s == null) {
			for (GaugeField gf : gaugeFields) {
				gf.gauge.update(gf.gauge.maxValue / 2, "PRE");
			}
			return;
		}

		if (jetChecked == false) {
			if (s.checkEngineFlag) {
				if (s.isEngJet()) {
					isJet = true;
					for (GaugeField gf : gaugeFields) {
						if (gf.gaugeType == GAUGE_PITCH) {
							gf.gauge.label = Lang.eThurstP;
						}
					}
				}
				jetChecked = true;
			}
		}

		for (GaugeField gf : gaugeFields) {
			switch (gf.gaugeType) {
				case GAUGE_THROTTLE:
					gf.gauge.update(s.sState.throttle, String.format("%3s", s.throttle));
					break;
				case GAUGE_PITCH:
					if (!isJet) {
						if (!s.RPMthrottle.equals(Service.nastring)) {
							gf.gauge.update(s.sState.RPMthrottle, String.format("%3s", s.RPMthrottle));
						} else {
							gf.gauge.update(0, String.format("%3s", s.RPMthrottle));
						}
					} else {
						gf.gauge.update((int) s.thurstPercent, String.format("%3s", s.sThurstPercent));
					}
					break;
				case GAUGE_MIXTURE:
					if (!s.mixture.equals(Service.nastring)) {
						gf.gauge.update(s.sState.mixture, String.format("%3s", s.mixture));
					} else {
						gf.gauge.update(0, String.format("%3s", s.mixture));
					}
					break;
				case GAUGE_RADIATOR:
					if (!s.radiator.equals(Service.nastring)) {
						gf.gauge.update(s.sState.radiator, String.format("%3s", s.radiator));
					} else {
						gf.gauge.update(0, String.format("%3s", s.radiator));
					}
					break;
				case GAUGE_COMPRESSOR:
					if (s.sState.compressorstage - 1 > gf.gauge.maxValue) {
						gf.gauge.maxValue = s.sState.compressorstage - 1;
					}
					gf.gauge.update(s.sState.compressorstage - 1, String.format("%3s", s.compressorstage));
					break;
				case GAUGE_FUEL:
					gf.gauge.update(s.fuelPercent, String.format("%3s", s.sfuelPercent));
					break;
			}
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
			xc.configService.saveLayoutConfig();
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

	// Unused run() from DraggableOverlay - disabled
	@Override
	public void run() {
		// Event-driven - no polling
	}
}