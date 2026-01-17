package ui.overlay;

import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.splitpane.WebSplitPane;

import parser.Blkx;
import prog.Application;
import prog.Controller;
import prog.i18n.Lang;
import prog.Service;
import ui.UIBaseElements;
import ui.WebLafSettings;

public class EngineInfo extends WebFrame implements Runnable {
	/**
	 * 
	 */

	int isDragging = 0;
	int xx = 0;
	int yy = 0;
	public volatile boolean doit = true;
	WebSplitPane splitPane;
	public int overheattime;
	private static final long serialVersionUID = 3063042782594625576L;
	public Controller xc;
	public Service s;
	public Blkx p;
	public int wtload1;
	public int oilload1;
	public String status;
	String NumFont;

	long freq;
	long MainCheckMili;
	int WIDTH;
	int HEIGHT;
	int lx;
	int ly;
	int OP;
	Color transParentWhite = Application.colorNum;
	Color transParentWhitePlus = Application.colorNum;

	Color transparent = new Color(0, 0, 0, 0);
	Color lblShadeColor = Application.colorShade;

	WebPanel panel;

	public String FontName;
	int font1 = 12;
	int font2 = 14;
	int font3 = 10;
	int fontadd = 0;
	private int fontsize;
	private Font fontNum;
	private Font fontLabel;
	private Font fontUnit;
	static Color lblColor = Application.colorUnit;
	static Color lblNameColor = Application.colorLabel;
	static Color lblNumColor = Application.colorNum;

	public void fclose() {
		// this.setVisible(false);

		this.dispose();
		// this.close();
	}

	public void initPreview(Controller c) {

		init(c, null, null);
		// setShadeWidth(10);
		// this.setVisible(false);
		// this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 50));
		this.getWebRootPaneUI().setMiddleBg(Application.previewColor);// 中部透明
		this.getWebRootPaneUI().setTopBg(Application.previewColor);// 顶部透明

		panel.addMouseListener(new MouseAdapter() {
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
		panel.addMouseMotionListener(new MouseMotionAdapter() {
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

		this.setCursor(null);
		setVisible(true);
		// setFocusable(true);
		// setFocusableWindowState(true);

	}

	public void saveCurrentPosition() {
		xc.setconfig("engineInfoX", Integer.toString(getLocation().x));
		xc.setconfig("engineInfoY", Integer.toString(getLocation().y));
	}

	String[][] totalString;
	int useNum = 0;

	// 功率,喷气为0的话不显示
	int idx_hp = Integer.MAX_VALUE;
	// 推力
	int idx_thrust = Integer.MAX_VALUE;
	// 转速
	int idx_rpm = Integer.MAX_VALUE;
	// 桨距角, 喷气为0的话不显示
	int idx_prop = Integer.MAX_VALUE;
	// 桨效率,喷气为0的话不显示
	int idx_eff = Integer.MAX_VALUE;
	// 有效功率
	int idx_ehp = Integer.MAX_VALUE;

	// 有效功率或推力百分比
	int idx_eper = Integer.MAX_VALUE;

	// 进气压
	int idx_map = Integer.MAX_VALUE;

	// 燃油重量
	int idx_mfuel = Integer.MAX_VALUE;
	// 燃油时间
	int idx_fueltime = Integer.MAX_VALUE;

	// 加力重量(如果为0不显示)
	int idx_mwep = Integer.MAX_VALUE;
	// 加力时间(如果为0不显示)
	int idx_weptime = Integer.MAX_VALUE;

	// 温度
	int idx_temp = Integer.MAX_VALUE;
	// 油温
	int idx_oiltemp = Integer.MAX_VALUE;
	// 耐热时
	int idx_heattlr = Integer.MAX_VALUE;
	// 响应速
	int idx_engres = Integer.MAX_VALUE;

	private int[] doffset;
	private int numHeight;
	private int columnNum;
	Boolean[] totalSwitch;
	private Container root;

	public void initTextString() {
		totalSwitch = new Boolean[20];
		totalString = new String[20][];
		String tmp;
		for (int i = 0; i < 20; i++)
			totalString[i] = new String[3];

		// 马力
		tmp = xc.getconfig("disableEngineInfoHorsePower");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1200");
			totalString[useNum][1] = String.format("%s", Lang.ePower);
			totalString[useNum][2] = String.format("%s", "Hp");
			totalSwitch[useNum] = true;
			idx_hp = useNum++;
		}

		// 推力
		tmp = xc.getconfig("disableEngineInfoThrust");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1004");
			totalString[useNum][1] = String.format("%s", Lang.eThurst);
			totalString[useNum][2] = String.format("%s", "Kgf");
			totalSwitch[useNum] = true;
			idx_thrust = useNum++;
		}

		// 转速
		tmp = xc.getconfig("disableEngineInfoRPM");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "2400");
			totalString[useNum][1] = String.format("%s", Lang.eRPM);
			totalString[useNum][2] = String.format("%s", "Rpm");
			totalSwitch[useNum] = true;
			idx_rpm = useNum++;
		}

		// 桨距
		tmp = xc.getconfig("disableEngineInfoPropPitch");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "55");
			totalString[useNum][1] = String.format("%s", Lang.ePitchDeg);
			totalString[useNum][2] = String.format("%s", "Deg");
			totalSwitch[useNum] = true;
			idx_prop = useNum++;
		}

		// 桨效率
		tmp = xc.getconfig("disableEngineInfoEffEta");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "90");
			totalString[useNum][1] = String.format("%s", Lang.eEff);
			totalString[useNum][2] = String.format("%s", "%");
			totalSwitch[useNum] = true;
			idx_eff = useNum++;
		}

		// 有效功率
		tmp = xc.getconfig("disableEngineInfoEffHp");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1005");
			totalString[useNum][1] = String.format("%s", Lang.eEffPower);
			totalString[useNum][2] = String.format("%s", "Hp");
			totalSwitch[useNum] = true;
			idx_ehp = useNum++;
		}

		// 进气压
		// 有效功率或推力百分比
		tmp = xc.getconfig("disableEngineInfoPressure");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "1.52");
			totalString[useNum][1] = String.format("%s", Lang.eATM);
			totalString[useNum][2] = String.format("%s", "Ata");
			totalSwitch[useNum] = true;
			idx_map = useNum++;
		}

		// 有效功率或推力百分比
		tmp = xc.getconfig("disableEngineInfoPowerPercent");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "85");
			totalString[useNum][1] = String.format("%s", Lang.ePowerPercent);
			totalString[useNum][2] = String.format("%s", "%");
			totalSwitch[useNum] = true;
			idx_eper = useNum++;
		}

		// 燃油重
		tmp = xc.getconfig("disableEngineInfoFuelKg");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "121");
			totalString[useNum][1] = String.format("%s", Lang.eFuel);
			totalString[useNum][2] = String.format("%s", "Kg");
			totalSwitch[useNum] = true;
			idx_mfuel = useNum++;
		}

		// 燃油时
		tmp = xc.getconfig("disableEngineInfoFuelTime");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "15");
			totalString[useNum][1] = String.format("%s", Lang.eFueltime);
			totalString[useNum][2] = String.format("%s", "Min");
			totalSwitch[useNum] = true;
			idx_fueltime = useNum++;
		}

		// 加力重
		tmp = xc.getconfig("disableEngineInfoWepKg");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "15");
			totalString[useNum][1] = String.format("%s", Lang.eWep);
			totalString[useNum][2] = String.format("%s", "Kg");
			totalSwitch[useNum] = true;
			idx_mwep = useNum++;
		}

		// 加力时
		tmp = xc.getconfig("disableEngineInfoWepTime");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "81");
			totalString[useNum][1] = String.format("%s", Lang.eWeptime);
			totalString[useNum][2] = String.format("%s", "S");
			totalSwitch[useNum] = true;
			idx_weptime = useNum++;
		}

		// 温度
		tmp = xc.getconfig("disableEngineInfoTemp");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "115");
			totalString[useNum][1] = String.format("%s", Lang.eTemp);
			totalString[useNum][2] = String.format("%s", "C");
			totalSwitch[useNum] = true;
			idx_temp = useNum++;
		}

		// 油温
		tmp = xc.getconfig("disableEngineInfoOilTemp");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "105");
			totalString[useNum][1] = String.format("%s", Lang.eOil);
			totalString[useNum][2] = String.format("%s", "C");
			totalSwitch[useNum] = true;
			idx_oiltemp = useNum++;
		}

		// 耐热时
		tmp = xc.getconfig("disableEngineInfoHeatTolerance");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "3600");
			totalString[useNum][1] = String.format("%s", Lang.eOverheat);
			totalString[useNum][2] = String.format("%s", "S");
			totalSwitch[useNum] = true;
			idx_heattlr = useNum++;
		}

		tmp = xc.getconfig("disableEngineInfoEngResponse");
		if (!(tmp != "" && Boolean.parseBoolean(tmp) == true)) {
			totalString[useNum][0] = String.format("%5s", "10");
			totalString[useNum][1] = String.format("%s", Lang.eEngRes);
			totalString[useNum][2] = String.format("%s", "%/s");
			totalSwitch[useNum] = true;
			idx_engres = useNum++;
		}

	}

	public void __update_num(int idx, String s) {
		if (idx < useNum) {
			totalString[idx][0] = String.format("%5s", s);
		}
	}

	public static final String hp = "Hp";
	public static final String mhp = "MHp";

	public void updateString() {

		__update_num(idx_hp, s.sTotalHp);

		// 推力
		__update_num(idx_thrust, s.sTotalThr);
		// 转速
		__update_num(idx_rpm, s.rpm);
		// 桨距角, 喷气为0的话不显示
		__update_num(idx_prop, s.pitch[0]);
		// 桨效率,喷气为0的话不显示
		__update_num(idx_eff, s.sAvgEff);
		// 有效功率
		__update_num(idx_ehp, s.sTotalHpEff);

		if (idx_ehp < useNum) {
			if (s.bUnitMHp) {
				totalString[idx_ehp][2] = mhp;

			} else {
				totalString[idx_ehp][2] = hp;
			}
		}

		// 进气压
		// 飞马英制
		// 等于-不显示
		if (idx_map < useNum) {
			if (s.manifoldpressure == null || s.manifoldpressure.equals(Service.nastring)) {
				totalSwitch[idx_map] = false;
			} else {
				totalSwitch[idx_map] = true;
			}
		}

		if (s.iCheckAlt > 0) {
			__update_num(idx_map, s.pressurePounds);
			if (idx_map < useNum) {
				totalString[idx_map][2] = s.pressureInchHg;
			}

		} else {
			__update_num(idx_map, s.manifoldpressure);
			if (idx_map < useNum) {
				totalString[idx_map][2] = Service.pressureUnit;
			}
		}

		// 有效功率或推力百分比
		__update_num(idx_eper, s.sThurstPercent);
		// 燃油重量
		__update_num(idx_mfuel, s.sTotalFuel);
		// 燃油时间
		__update_num(idx_fueltime, s.sfueltime);
		// 加力重量(如果为0不显示)
		// 等于-不显示
		if (idx_mwep < useNum) {
			if (s.sNitro == null || s.sNitro.equals(Service.nastring)) {
				totalSwitch[idx_mwep] = false;
			}
		}
		__update_num(idx_mwep, s.sNitro);
		// 加力时间(如果为0不显示)
		if (idx_weptime < useNum) {
			if (s.sWepTime == null || s.sWepTime.equals(Service.nastring)) {
				totalSwitch[idx_weptime] = false;
			} else {
				totalSwitch[idx_weptime] = true;
			}
		}
		__update_num(idx_weptime, s.sWepTime);

		// 温度
		__update_num(idx_temp, s.watertemp);
		// 油温
		__update_num(idx_oiltemp, s.oiltemp);
		// 耐热时
		__update_num(idx_heattlr, s.sEngWorkTime);

		__update_num(idx_engres, s.SdThrustPercent);
		// 判断

		if (s.checkEngineFlag && s.isEngJet()) {
			// 关闭桨距
			if (idx_hp < useNum)
				totalSwitch[idx_hp] = false;
			if (idx_prop < useNum)
				totalSwitch[idx_prop] = false;
			if (idx_eff < useNum)
				totalSwitch[idx_eff] = false;

		} else {
			if (idx_hp < useNum)
				totalSwitch[idx_hp] = true;
			if (idx_prop < useNum)
				totalSwitch[idx_prop] = true;
			if (idx_eff < useNum)
				totalSwitch[idx_eff] = true;

		}

		// hp
		// if (idx_hp < useNum){
		// if (s.sTotalHp.equals(Service.nastring)){
		// totalSwitch[idx_hp] = false;
		//// Application.debugPrint("关闭"+idx_hp);
		// }
		// else{
		// totalSwitch[idx_hp] = true;
		// }
		// }
		//
		// if (idx_prop < useNum){
		// if (s.pitch[0].equals(Service.nastring)){
		// totalSwitch[idx_prop] = false;
		// }
		// else{
		// totalSwitch[idx_prop] = true;
		// }
		// }
		//
		// if (idx_eff < useNum){
		// if (s.sAvgEff.equals(Service.nastring)){
		// totalSwitch[idx_eff] = false;
		// }
		// else{
		// totalSwitch[idx_eff] = true;
		// }
		// }

	}

	private void updateDxDy(int num, int[] doffset) {
		// TODO Auto-generated method stub
		if (num % columnNum == 0) {
			doffset[1] += Math.round(1 * numHeight);
			// doffset[0] = 0;;
			doffset[0] = fontsize >> 1;
		} else {
			doffset[0] += 5 * fontsize;
		}

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
		// Application.debugPrint(fontadd);
		if (xc.getconfig("engineInfoX") != "")
			lx = Integer.parseInt(xc.getconfig("engineInfoX"));
		else
			lx = 0;
		if (xc.getconfig("engineInfoY") != "")
			ly = Integer.parseInt(xc.getconfig("engineInfoY"));
		else
			ly = 860;

		fontsize = 24 + fontadd;
		// 设置字体
		fontNum = new Font(NumFont, Font.BOLD, fontsize);
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontsize / 2.0f));
		fontUnit = new Font(NumFont, Font.PLAIN, Math.round(fontsize / 2.0f));

		numHeight = getFontMetrics(fontNum).getHeight();

		// 列
		if (xc.getconfig("engineInfoColumn") != "")
			columnNum = Integer.parseInt(xc.getconfig("engineInfoColumn"));
		else
			columnNum = 3;

		useNum = 0; // 重置计数
		initTextString();

		int addnum = (useNum % columnNum == 0) ? 0 : 1;

		WIDTH = (fontsize >> 1) + (int) ((columnNum + 0.5) * 5f * fontsize);
		HEIGHT = (int) (numHeight + (useNum / columnNum + addnum + 1) * 1.0f * numHeight);

		if (xc.getconfig("engineInfoEdge").equals("true"))
			setShadeWidth(10);// 玻璃效果边框
		else
			setShadeWidth(0);

		this.setBounds(lx, ly, WIDTH, HEIGHT);
		repaint();
	}

	public void init(Controller xc, Service ts, Blkx tp) {
		this.xc = xc;
		this.s = ts;
		this.p = tp;

		overheattime = 0;
		freq = xc.freqEngineInfo;

		reinitConfig();

		doffset = new int[2];

		panel = new WebPanel() {

			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw
				g2d.setPaintMode();
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
				// g2d.setRenderingHint(RenderingHints.KEY_RENDERING,
				// RenderingHints.VALUE_RENDER_QUALITY);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);

				doffset[0] = fontsize >> 1;
				doffset[1] = fontsize >> 1;
				int k = 0;
				for (int i = 0; i < useNum; i++) {
					if (totalSwitch[i] == false) {
						// Application.debugPrint("跳过"+i);
						continue;
					}
					UIBaseElements._drawLabelBOSType(g2d, doffset[0], doffset[1], 1, 3 * fontsize, fontNum, fontLabel,
							fontUnit,
							totalString[i][0], totalString[i][1], totalString[i][2]);
					updateDxDy(++k, doffset);

				}

				// g.dispose();
			}
		};

		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));

		this.getWebRootPaneUI().setMiddleBg(transparent);// 中部透明
		this.getWebRootPaneUI().setTopBg(transparent);// 顶部透明
		this.getWebRootPaneUI().setBorderColor(transparent);// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(transparent);// 外描边透明

		this.add(panel);

		setTitle("EngineInfo");
		WebLafSettings.setWindowOpaque(this);
		jetChecked = false;
		// this.setCloseOnFocusLoss(true);
		root = this.getContentPane();
		if (xc.getconfig("engineInfoEdge").equals("true"))
			setShadeWidth(10);// 玻璃效果边框
		else {
			setShadeWidth(0);
		}
		if (ts != null)
			setVisible(true);
	}

	long engineCheckMili;
	private boolean isJet;
	private boolean jetChecked;

	public void drawTick() {

		// 更新字符串

		updateString();

		root.repaint();
	}

	@Override
	public void run() {
		while (doit) {

			try {
				Thread.sleep(Application.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}

			if (s.SystemTime - engineCheckMili > xc.freqService) {
				engineCheckMili = s.SystemTime;
				if (s.sState != null) {
					if (jetChecked == false) {
						if (xc.Blkx != null && xc.Blkx.valid == true) {
							if (xc.Blkx.isJet) {

								isJet = true;
								// slider1.setVisible(false);
								// slider2.setVisible(false);
								// slider4.setVisible(false);
								// wsp1.setVisible(false);
								// // label_16.setVisible(false);
								// lefttitle1.setVisible(false);
								// lefttitle.setVisible(false);
								// bottomtitle1.setVisible(false);
								// bottomtitle.setVisible(false);
								//
								// // 油门设置
								// lefttitle2.setText(language.eThrottle);
								// slider3.setMaximum(110);
								// 修改为推力百分比
							}
							jetChecked = true;
						}
					}
					if (isJet) {
						// slider3.setValue(s.sState.throttle);
					} else {
						// slider1.setValue(s.sState.throttle);
						// slider2.setValue(s.sState.RPMthrottle);
						// slider3.setValue(s.sState.mixture);
						// slider4.setValue(s.sState.radiator);
						//
						// // slider5.setValue(s.sState.mfuel1);
						// wspsetStep(wsp1, s.sState.compressorstage);
					}

					drawTick();

				}
			}
		}
	}
}