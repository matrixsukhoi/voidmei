package ui.overlay;

import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;

import com.alee.laf.panel.WebPanel;

import parser.Blkx;
import prog.Application;
import prog.Controller;
import prog.i18n.Lang;
import prog.Service;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import ui.base.DraggableOverlay;

public class EngineControl extends DraggableOverlay implements FlightDataListener {

	private static final long serialVersionUID = 3063042782594625576L;

	public Controller xc;
	public Service s;
	public Blkx p;

	// Config
	public prog.config.ConfigLoader.GroupConfig groupConfig;
	String NumFont;
	public String FontName;
	int fontadd = 0;
	private int fontsize;
	private Font fontLabel;

	// Layout
	int WIDTH;
	int HEIGHT;
	int lx;
	int ly;
	public int rowNum;
	public int columnNum;

	// Components
	public java.util.List<ui.component.LinearGauge> gauges;
	WebPanel panel;
	private Container root;

	// Gauge Indexes
	int lidx_t = Integer.MAX_VALUE; // 油门
	int lidx_p = Integer.MAX_VALUE; // 桨距
	int lidx_m = Integer.MAX_VALUE; // 混合比
	int lidx_r = Integer.MAX_VALUE; // 散热器
	int lidx_c = Integer.MAX_VALUE; // 增压器
	int lidx_f = Integer.MAX_VALUE; // 燃油百分比

	// Logic State
	private boolean isJet;
	private boolean jetChecked;

	public EngineControl() {
		super();
		this.doit = false; // Disable base class polling
	}

	public void fclose() {
		dispose();
	}

	@Override
	public void dispose() {
		FlightDataBus.getInstance().unregister(this);
		super.dispose();
	}

	public void initPreview(Controller c, prog.config.ConfigLoader.GroupConfig groupConfig) {
		this.groupConfig = groupConfig;
		init(c, null, null, groupConfig);
		// Apply preview style from base class
		applyPreviewStyle();

		// Force initial update for Preview Mode (populates "PRE" values)
		updateGauges();

		// Setup drag listeners from base class
		setupDragListeners();
		setVisible(true);
	}

	@Override
	public void saveCurrentPosition() {
		if (groupConfig != null) {
			int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
			int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
			groupConfig.x = (double) getLocation().x / screenW;
			groupConfig.y = (double) getLocation().y / screenH;
			xc.configService.saveLayoutConfig();
		} else {
			// Fallback or legacy (xc is Controller which implements ConfigProvider)
			xc.setconfig("engineControlX", Integer.toString(getLocation().x));
			xc.setconfig("engineControlY", Integer.toString(getLocation().y));
		}
	}

	public void init(Controller xc, Service ts, Blkx tp, prog.config.ConfigLoader.GroupConfig groupConfig) {
		this.xc = xc;
		this.s = ts;
		this.p = tp;
		this.groupConfig = groupConfig;
		this.config = xc; // Set base class config provider

		reinitConfig();

		panel = new WebPanel() {
			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				g2d.setPaintMode();
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);

				drawTPMRC(g2d, 0, (fontsize * 9) >> 1);
			}
		};

		this.add(panel);
		// Transparent Setup from base usage or manual
		// DraggableOverlay has setupTransparentWindow() but EngineControl manual setup
		// was slightly different (WebLafSettings.setWindowOpaque(this))
		setupTransparentWindow();

		setSize(WIDTH, HEIGHT);
		setLocation(lx, ly);

		setTitle("EngineControl");

		jetChecked = false;
		root = getContentPane();

		if (xc.getconfig("engineInfoEdge").equals("true"))
			setShadeWidth(10);
		else {
			setShadeWidth(0);
		}

		// Event Registration
		subscribeToEvents();

		if (ts != null)
			setVisible(true);
	}

	protected void subscribeToEvents() {
		FlightDataBus.getInstance().register(this);
	}

	public void reinitConfig() {
		if (xc.getconfig("GlobalNumFont") != "")
			NumFont = xc.getconfig("GlobalNumFont");
		else
			NumFont = Application.defaultNumfontName;

		if (xc.getconfig("engineInfoFont") != "")
			FontName = xc.getconfig("engineInfoFont");
		else
			FontName = Application.defaultFont.getFontName();
		if (xc.getconfig("engineInfoFontadd") != "")
			fontadd = Integer.parseInt(xc.getconfig("engineInfoFontadd"));
		else
			fontadd = 0;

		// Use GroupConfig for position if available
		if (groupConfig != null) {
			int screenW = java.awt.Toolkit.getDefaultToolkit().getScreenSize().width;
			int screenH = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
			lx = (int) (groupConfig.x * screenW);
			ly = (int) (groupConfig.y * screenH);
		} else {
			if (xc.getconfig("engineControlX") != "")
				lx = Integer.parseInt(xc.getconfig("engineControlX"));
			else
				lx = 0;
			if (xc.getconfig("engineControlY") != "")
				ly = Integer.parseInt(xc.getconfig("engineControlY"));
			else
				ly = 860;
		}

		fontsize = 24 + fontadd;
		// 设置字体
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontsize / 2.0f));

		rowNum = 0;
		columnNum = 0;
		initLeftString();

		WIDTH = fontsize * 8;
		HEIGHT = (int) ((fontsize * 4 + (fontsize * 9) >> 1) + (rowNum + 1) * (1 * fontsize + (fontsize >> 2)));

		if (xc.getconfig("engineInfoEdge").equals("true"))
			setShadeWidth(10);
		else
			setShadeWidth(0);

		setSize(WIDTH, HEIGHT);
		setLocation(lx, ly);

		// Populate gauges with data (or "PRE" in preview mode) immediately after
		// rebuilding list
		updateGauges();

		repaint();
	}

	public void initLeftString() {
		gauges = new java.util.ArrayList<>();
		int leftUseNum = 0;
		String tmp;

		// 油门
		tmp = xc.getconfig("disableEngineInfoThrottle");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gauges.add(new ui.component.LinearGauge(String.format("%s", Lang.eThrottle), 110, true));
			lidx_t = leftUseNum++;
			columnNum++;
		}

		// 桨距
		tmp = xc.getconfig("disableEngineInfoPitch");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gauges.add(new ui.component.LinearGauge(String.format("%s", Lang.eProppitch), 100, true));
			lidx_p = leftUseNum++;
			columnNum++;
		}

		// 混合比
		tmp = xc.getconfig("disableEngineInfoMixture");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gauges.add(new ui.component.LinearGauge(String.format("%s", Lang.eMixture), 120, false));
			lidx_m = leftUseNum++;
			rowNum++;
		}

		// 散热器
		tmp = xc.getconfig("disableEngineInfoRadiator");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gauges.add(new ui.component.LinearGauge(String.format("%s", Lang.eRadiator), 100, false));
			lidx_r = leftUseNum++;
			rowNum++;
		}

		// 增压器
		tmp = xc.getconfig("disableEngineInfoCompressor");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gauges.add(new ui.component.LinearGauge(String.format("%s", Lang.eCompressor), 1, false));
			lidx_c = leftUseNum++;
			rowNum++;
		}

		tmp = xc.getconfig("disableEngineInfoLFuel");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			gauges.add(new ui.component.LinearGauge(String.format("%s", Lang.eFuelPer), 100, false));
			lidx_f = leftUseNum++;
			rowNum++;
		}

	}

	public void drawTPMRC(Graphics2D g2d, int x, int y) {
		int dx = 0;
		int dy = fontsize >> 1;

		for (int i = 0; i < gauges.size(); i++) {
			ui.component.LinearGauge gauge = gauges.get(i);

			if (isJet && (i == lidx_r || i == lidx_c || i == lidx_m))
				continue;
			if (i == lidx_r || i == lidx_c || i == lidx_m || i == lidx_f) {
				if ((i == lidx_c) && (s != null && s.sState.compressorstage == 0))
					continue;
				if ((i == lidx_m) && (s != null && s.sState.mixture < 0))
					continue;
				// 横着画
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

	// --- Event Driven Logic ---

	@Override
	public void onFlightData(FlightDataEvent event) {
		// Ensure UI updates on EDT
		javax.swing.SwingUtilities.invokeLater(() -> {
			updateGauges();
			if (root != null)
				root.repaint();
		});
	}

	public void __update_num(int idx, String s, int val) {
		if (idx < gauges.size()) {
			ui.component.LinearGauge gauge = gauges.get(idx);
			gauge.update(val, String.format("%3s", s));
		}
	}

	public void updateGauges() {
		// Preview Mode (s is null)
		if (s == null) {
			for (ui.component.LinearGauge g : gauges) {
				g.update(g.maxValue / 2, "PRE");
			}
			return;
		}

		int leftUseNum = gauges.size();
		if (jetChecked == false) {
			if (s.checkEngineFlag) {
				if (s.isEngJet()) {
					isJet = true;
					// 修改为推力百分比
					if (lidx_p < leftUseNum)
						gauges.get(lidx_p).label = Lang.eThurstP;
				}
				jetChecked = true;
			}
		}

		// 油门
		__update_num(lidx_t, s.throttle, s.sState.throttle);

		// 桨距
		if (!isJet) {
			if (!s.RPMthrottle.equals(Service.nastring)) {
				__update_num(lidx_p, s.RPMthrottle, s.sState.RPMthrottle);
			} else
				__update_num(lidx_p, s.RPMthrottle, 0);
		} else
			__update_num(lidx_p, s.sThurstPercent, (int) s.thurstPercent);

		// 混合比
		if (!s.mixture.equals(Service.nastring)) {
			__update_num(lidx_m, s.mixture, s.sState.mixture);
		} else {
			__update_num(lidx_m, s.mixture, 0);
		}

		// 散热器
		if (!s.radiator.equals(Service.nastring))
			__update_num(lidx_r, s.radiator, s.sState.radiator);
		else {
			__update_num(lidx_r, s.radiator, 0);
		}

		// 增压器
		if (lidx_c < leftUseNum) {
			ui.component.LinearGauge g = gauges.get(lidx_c);
			if (s.sState.compressorstage - 1 > g.maxValue)
				g.maxValue = s.sState.compressorstage - 1;
		}
		__update_num(lidx_c, s.compressorstage, s.sState.compressorstage - 1);

		__update_num(lidx_f, s.sfuelPercent, s.fuelPercent);
	}
}