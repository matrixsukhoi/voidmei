package ui;

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
import com.alee.laf.rootpane.WebFrame;

import prog.controller;
import prog.otherService;

public class situationAware extends WebFrame implements Runnable {
	/**
	 * 
	 */
	private static final long serialVersionUID = 5529573888335826342L;
	public volatile boolean doit = true;
	int WIDTH;
	int HEIGHT;
	controller xc;
	int isDragging;
	int xx;
	int yy;
	int fontadd;
	WebPanel panel;
	otherService xs;
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

	public void initPreview(controller c) {
		init(c, null);
		setShadeWidth(10);
		this.setVisible(false);
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 1));
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 1));
		setFocusableWindowState(true);
		setFocusable(true);

		addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
				/*
				 * if(A.tag==0){ if(f.mode==1){ A.setVisible(false);
				 * A.visibletag=0; } }
				 */
			}

			public void mousePressed(MouseEvent e) {
				isDragging = 1;
				xx = e.getX();
				yy = e.getY();

			}

			public void mouseReleased(MouseEvent e) {
				if (isDragging == 1) {
					isDragging = 0;
				}
				/*
				 * if(A.tag==0){ A.setVisible(false); }
				 */
			}
			/*
			 * public void mouseReleased(MouseEvent e){ if(A.tag==0){
			 * A.setVisible(true); } }
			 */
		});
		addMouseMotionListener(new MouseMotionAdapter() {
			public void mouseDragged(MouseEvent e) {
				if (isDragging == 1) {
					int left = getLocation().x;
					int top = getLocation().y;
					setLocation(left + e.getX() - xx, top + e.getY() - yy);
					saveCurrentPosition();
					setVisible(true);
					repaint();
				}
			}
		});
		setVisible(true);
	}

	public void saveCurrentPosition() {
		xc.setconfig("situationAwareX", Integer.toString(getLocation().x));
		xc.setconfig("situationAwareY", Integer.toString(getLocation().y));
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

	int lx;
	int ly;

	public void reinitConfig() {
		if (xc.getconfig("situationAwareX") != "")
			lx = Integer.parseInt(xc.getconfig("situationAwareX"));
		else
			lx = Toolkit.getDefaultToolkit().getScreenSize().width - WIDTH;
		if (xc.getconfig("situationAwareY") != "")
			ly = Integer.parseInt(xc.getconfig("situationAwareY"));
		else
			ly = 50;

		setShadeWidth(0);
		this.setBounds(lx, ly, WIDTH, HEIGHT);
		repaint();
	}

	public void init(controller c, otherService s) {
		xc = c;
		xs = s;
		WIDTH = 250;
		HEIGHT = 150;

		reinitConfig();

		this.setBounds(lx, ly, WIDTH, HEIGHT);
		panel = new WebPanel();
		// panel.setSize(WIDTH, HEIGHT);
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		// this.setUndecorated(true);

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
		setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);
	}

	public void run() {
		while (doit) {
			try {
				Thread.sleep(500);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			// app.debugPrint("刷新了");
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