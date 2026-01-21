// SituationAware这个特性以后一定不会演化了
// 2026年1月17日

package ui.overlay;

import java.awt.Color;
import java.awt.Font;
import java.awt.GridLayout;
import java.awt.Toolkit;

import javax.swing.SwingConstants;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

import prog.Application;
import prog.Controller;
import prog.OtherService;
import prog.config.OverlaySettings;

import ui.base.DraggableOverlay;

public class SituationAware extends DraggableOverlay {
	/**
	 * 
	 */
	private static final long serialVersionUID = 5529573888335826342L;
	// doit, WIDTH, HEIGHT, xc, panel are partly redundant or need adjustment
	// xc is Controller

	// DraggableOverlay has 'config' (ConfigProvider) - we can use it.
	// But SituationAware heavily uses 'xc' (Controller). We can cast config to
	// Controller or keep xc.
	// Let's keep xc but initialize it from init.

	int WIDTH;
	int HEIGHT;
	Controller xc;
	private OverlaySettings settings;
	// Drag State
	private int dragStartX, dragStartY;

	int fontadd;
	WebPanel panel;
	OtherService xs;
	WebLabel enemy;
	WebLabel friend;
	WebLabel enemySpeed;
	WebLabel enemyDistance;

	public WebLabel createNewWebLabel(String text, int fontsize) {
		WebLabel title = new WebLabel(text);
		title.setHorizontalAlignment(SwingConstants.CENTER);
		title.setDrawShade(true);
		title.setForeground(new Color(245, 248, 250, 240));
		title.setShadeColor(Color.BLACK);
		String font = settings != null ? settings.getString("engineInfoFont", Application.defaultFontName)
				: Application.defaultFontName;
		title.setFont(new Font(font, Font.PLAIN, fontsize));
		title.setFontSize(fontsize);

		return title;
	}

	public void initPreview(Controller c, OverlaySettings settings) {
		init(c, null, settings);
		applyPreviewStyle();
		setVisible(true);
	}

	public void initpanel() {
		GridLayout l = new GridLayout(5, 3);
		panel.setLayout(l);
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
		fontadd = 2;
		WebLabel a1 = createNewWebLabel("敌机威胁", 14);
		enemy = createNewWebLabel("0", 14 + fontadd);
		WebLabel b1 = createNewWebLabel("架", 14 - fontadd);
		// enemy.setForeground(Color.RED);
		panel.add(a1);
		panel.add(enemy);
		panel.add(b1);

		panel.add(new WebLabel(""));
		panel.add(new WebLabel(""));
		panel.add(new WebLabel(""));

		WebLabel a3 = createNewWebLabel("水平速度", 14);
		enemySpeed = createNewWebLabel("0", 14 + fontadd);
		WebLabel b3 = createNewWebLabel("km/h", 14 - fontadd);

		panel.add(a3);
		panel.add(enemySpeed);
		panel.add(b3);

		WebLabel a4 = createNewWebLabel("水平距离", 14);
		enemyDistance = createNewWebLabel("0", 14 + fontadd);
		WebLabel b4 = createNewWebLabel("m", 14 - fontadd);

		panel.add(a4);
		panel.add(enemyDistance);
		panel.add(b4);
	}

	int lx;
	int ly;

	public void reinitConfig() {
		if (settings != null) {
			lx = settings.getWindowX(WIDTH);
			ly = settings.getWindowY(HEIGHT);
		} else {
			lx = Toolkit.getDefaultToolkit().getScreenSize().width - WIDTH;
			ly = 50;
		}

		setShadeWidth(0);
		this.setBounds(lx, ly, WIDTH, HEIGHT);
		repaint();
	}

	public void init(Controller c, OtherService s, OverlaySettings settings) {
		this.xc = c;
		this.config = c;
		this.settings = settings;
		this.xs = s;
		this.WIDTH = 250;
		this.HEIGHT = 150;

		this.setUndecorated(true);
		setupTransparentWindow();
		reinitConfig();

		setupDragListeners();

		// --- Panel ---
		panel = new WebPanel();
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));

		initpanel();
		this.add(panel);
		this.setShadeWidth(0);
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setTitle("SA信息条");
		setAlwaysOnTop(true);
		setFocusable(false);
		setFocusableWindowState(false);

		if (s != null)
			setVisible(true);
	}

	@Override
	public void saveCurrentPosition() {
		if (settings != null) {
			settings.saveWindowPosition(getLocation().x, getLocation().y);
		}
	}

	public void run() {
		while (doit) {
			try {
				Thread.sleep(500);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			// Application.debugPrint("刷新了");
			enemy.setText(Integer.toString(xs.enemycount));
			if (xs.enemycount > 0)
				enemy.setForeground(Color.RED);
			else
				enemy.setForeground(new Color(245, 248, 250, 240));
			// friend.setText(Integer.toString(xs.friendcount));

			enemySpeed.setText(Integer.toString((int) (xs.enemyspeed * 3.6)));

			enemyDistance.setText(Integer.toString((int) xs.distance));

			this.repaint();
		}
	}
}