// SituationAware这个特性以后一定不会演化了
// 2026年1月17日

package ui.overlay;

import java.awt.Color;
import java.awt.Font;
import java.awt.GridLayout;
import java.awt.Toolkit;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;

import javax.swing.SwingConstants;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

import prog.Controller;
import prog.OtherService;

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
	Controller xc; // Keep for now
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
		title.setFont(new Font(xc.getconfig("engineInfoFont"), Font.PLAIN, fontsize));
		title.setFontSize(fontsize);

		return title;

	}

	public void initPreview(Controller c) {
		// Preview mode needs a dummy GroupConfig or handle null?
		// Let's create a temporary config for preview.
		prog.config.ConfigLoader.GroupConfig dummy = new prog.config.ConfigLoader.GroupConfig("Preview");
		dummy.x = 0.5;
		dummy.y = 0.5;
		dummy.visible = true;

		init(c, null, dummy);
		applyPreviewStyle();
		// setupDragListeners() is called in init() with the config.
		// But for preview, we might want standard behavior?
		// The init() calls setupDragListeners(groupConfig) which updates the dummy
		// config.
		// This is fine for preview.
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

		/*
		 * WebLabel a2=createNewWebLabel("支援友机",14);
		 * friend=createNewWebLabel("0",14+fontadd);
		 * WebLabel b2=createNewWebLabel("架",14-2);
		 * //friend.setForeground(Color.BLUE);
		 * panel.add(a2);
		 * panel.add(friend);
		 * panel.add(b2);
		 */

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

	// GroupConfig reference
	prog.config.ConfigLoader.GroupConfig groupConfig;

	int lx;
	int ly;

	public void reinitConfig() {
		// Use GroupConfig if available, otherwise fallback or defaults
		if (groupConfig != null) {
			lx = (int) (groupConfig.x * Toolkit.getDefaultToolkit().getScreenSize().width);
			ly = (int) (groupConfig.y * Toolkit.getDefaultToolkit().getScreenSize().height);
		} else {
			// Legacy fallback (shouldn't happen with correct Controller init)
			if (xc.getconfig("situationAwareX") != "")
				lx = Integer.parseInt(xc.getconfig("situationAwareX"));
			else
				lx = Toolkit.getDefaultToolkit().getScreenSize().width - WIDTH;
			if (xc.getconfig("situationAwareY") != "")
				ly = Integer.parseInt(xc.getconfig("situationAwareY"));
			else
				ly = 50;
		}

		setShadeWidth(0);
		this.setBounds(lx, ly, WIDTH, HEIGHT);
		repaint();
	}

	public void init(Controller c, OtherService s, prog.config.ConfigLoader.GroupConfig groupConfig) {
		xc = c;
		this.config = c; // Set parent config provider
		this.groupConfig = groupConfig;
		xs = s;
		WIDTH = 250;
		HEIGHT = 150;

		// Setup Position Keys (optional, but DraggableOverlay uses them if set)
		// With GroupConfig, we don't need legacy keys "situationAwareX" unless for
		// fallback?
		// But DraggableOverlay loadPosition methods use them.
		// However, we want to use GroupConfig.x/y!

		// If we extend DraggableOverlay, we should use its init mechanism?
		// No, let's just use what we need.

		setupTransparentWindow();

		// --- Position & Config ---
		int screenW = Toolkit.getDefaultToolkit().getScreenSize().width;
		int screenH = Toolkit.getDefaultToolkit().getScreenSize().height;

		int x = (int) (groupConfig.x * screenW);
		int y = (int) (groupConfig.y * screenH);

		this.setBounds(x, y, WIDTH, HEIGHT);

		// Setup Dragging to update GroupConfig directly
		addMouseListener(new MouseAdapter() {
			@Override
			public void mousePressed(MouseEvent e) {
				// Delegate to Draggable logic if we want, or custom
				// But DraggableOverlay has setupDragListeners.
			}
		});

		// We override DraggableOverlay's dragging to update GroupConfig!
		setupDragListeners(groupConfig);

		// --- Panel ---
		panel = new WebPanel(); // Use parent's panel if possible, but DraggableOverlay initializes it in init()
		// DraggableOverlay's init() creates panel. We are doing custom init here.
		// Let's instantiate panel here.

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

		if (s != null && groupConfig.visible)
			setVisible(true);
	}

	protected void setupDragListeners(prog.config.ConfigLoader.GroupConfig groupConfig) {
		// Custom drag listener that updates GroupConfig
		addMouseListener(new MouseAdapter() {
			@Override
			public void mousePressed(MouseEvent e) {
				dragStartX = e.getX();
				dragStartY = e.getY();
			}

			@Override
			public void mouseReleased(MouseEvent e) {
				// Update Config Object (Memory)
				int screenW = Toolkit.getDefaultToolkit().getScreenSize().width;
				int screenH = Toolkit.getDefaultToolkit().getScreenSize().height;
				groupConfig.x = (double) getLocation().x / screenW;
				groupConfig.y = (double) getLocation().y / screenH;

				// Trigger save via Controller
				xc.configService.saveLayoutConfig();
			}
		});
		addMouseMotionListener(new MouseMotionAdapter() {
			@Override
			public void mouseDragged(MouseEvent e) {
				// Standard drag using captured start point
				setLocation(e.getXOnScreen() - dragStartX, e.getYOnScreen() - dragStartY);
			}
		});
	}

	@Override
	public void saveCurrentPosition() {
		// No-op or update GroupConfig if we hold reference?
		// Ideally we shouldn't mix methods.
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