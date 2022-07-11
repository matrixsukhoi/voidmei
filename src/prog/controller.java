package prog;

import java.awt.AWTException;
import java.awt.Color;
import java.awt.Font;
import java.awt.Image;
import java.awt.Robot;
import java.awt.Toolkit;

import javax.swing.ImageIcon;

import com.alee.extended.panel.GroupPanel;
import com.alee.extended.time.ClockType;
import com.alee.extended.time.WebClock;
import com.alee.laf.label.WebLabel;
import com.alee.managers.notification.NotificationIcon;
import com.alee.managers.notification.NotificationManager;
import com.alee.managers.notification.WebNotification;

import parser.blkxparser;
import parser.flightAnalyzer;
import parser.flightLog;
import ui.statusBar;
import ui.stickValue;
import ui.attitudeIndicator;
import ui.minimalHUD;
import ui.drawFrame;
import ui.drawFrameSimpl;
import ui.engineInfo;
import ui.engineControl;
import ui.flightInfo;
import ui.gearAndFlaps;
import ui.mainform;
import ui.situationAware;
import ui.someUsefulData;

public class controller {

	public int flag;

	public boolean logon = false;

	public blkxparser blkx;

	Robot robot;

	engineControl F;
	statusBar SB;
	mainform M;
	minimalHUD H;
	otherService O;
	situationAware SA;
	flightInfo FL;
	flightLog Log;
	drawFrame dF;
	stickValue sV;
	gearAndFlaps fS;
	flapsControl flc;

	attitudeIndicator aI;

	Thread S1;
	Thread F1;
	Thread SB1;
	Thread M1;
	Thread H1;
	Thread O1;
	Thread SA1;
	Thread FL1;
	Thread Log1;
	Thread sV1;
	Thread fS1;
	Thread flc1;
	Thread pt1;

	Thread aI1;

	someUsefulData pt;
	public service S;
	public config cfg;
	// 存储参数
	// 主参数

	public long freqService;// Service取数据与计算周期
	// 发动机面板
	public long freqEngineInfo;// engineInfo刷新周期
	public long freqFlightInfo;
	// 人工地平仪
	public long freqAltitude;

	public long freqGearAndFlap;

	public long freqStickValue;
	//

	gcThread G;

	public static boolean engineInfoSwitch;// engineInfo面板开启
	public static boolean engineInfoEdge;// engineInfo面板边缘开启

	public static int engineInfoX;// engineInfo窗口位置
	public static int engineInfoY;

	public static Font engineInfoFont;

	public static int engineInfoOpaque;// engineInfo背景透明度

	public static boolean usetempratureInformation;

	public int lastEvt;
	public int lastDmg;
	public int step;

	Thread gc;

	public drawFrameSimpl thrustdFS;

	private Thread thrustdFS1;

	engineInfo FI;

	private Thread FI1;

	private voiceWarning vW;

	private Thread vW1;

	private uiThread uT;

	private Thread uT1;

	private boolean showStatus;

	public void hideTaskbarSw() {
		if (app.debug) {
			robot.keyPress(17);
			robot.keyPress(192);
			try {
				Thread.sleep(50);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			robot.keyRelease(17);
			robot.keyRelease(192);
		}
	}

	public void initStatusBar() {

		// 测试全局

		// 状态1，初始化状态条
		if (flag == 1) {
			// System.out.println("状态1，初始化状态条");

			if (showStatus) {
				SB = new statusBar();
				SB.init(this);
				SB.S1();
				SB1 = new Thread(SB);
				SB1.start();
			}

			flag = 2;

		}
		// SB.repaint();
	}

	public void changeS2() {
		// 状态2，状态条连接成功，等待进入游戏
		// System.out.println(flag);
		// SB.repaint();
		if (flag == 2) {
			// System.out.println("状态2，状态条连接成功，等待进入游戏");
			// NotificationManager.showNotification(createWebNotification("您已连接成功，请加入游戏"));
			if (showStatus)
				SB.S2();
			flag = 3;
		}
	}

	public void changeS3() {
		// 状态3，连接成功，释放状态条，打开面板
		// SB.repaint();
		if (flag == 3) {

			// 自动隐藏任务栏

			// 初始化MapObj以及Msg、gamechat
			// System.out.println(S.iIndic.type);
			getfmdata(S.sIndic.type);
			// System.out.println("状态3，连接成功，释放状态条，打开面板");
			// usetempratureInformation =
			// Boolean.parseBoolean(getconfig("usetempInfoSwitch"));
			// System.out.println(usetempratureInformation);
			// NotificationManager.showNotification(createWebNotificationTime(3000));
			if (showStatus) {
				SB.S3();
				SB.doit = false;
				SB.dispose();
				SB = null;
			}
			System.gc();
			if (app.debug) {
				O = new otherService();
				O.init(this);
				O1 = new Thread(O);
				O1.start();
			}
			flag = 4;
			openpad();

		}
	}

	public void S4toS1() {
		// 状态4，游戏返回，返回至状态1
		if (flag == 4) {
			// System.out.println("状态4，游戏退出，释放Service资源，返回至状态1");
			// 不触发燃油低告警
			// S.fuelPercent = 100;

			closepad();
			// 释放资源
			if (app.debug) {
				lastEvt = O.lastEvt;
				lastDmg = O.lastDmg;
				// System.out.println("最后DMGID"+lastDmg);
				O.close();
				O = null;
				O1 = null;
			}

			S.clear();
			flag = 1;

			// 自动显示任务栏
			hideTaskbarSw();
		}

	}

	public void openpad() {
		// hideTaskbarSw();

		if (getconfig("enableFMPrint").equals("true")) {
			pt = new someUsefulData();
			pt1 = new Thread(pt);
			pt.init(this, blkx);
			pt1.start();

			if (blkx.isJet) {
				thrustdFS = new drawFrameSimpl();

				thrustdFS1 = new Thread(thrustdFS);
				thrustdFS.init(this);
				thrustdFS1.start();

			}
		}
		if (getconfig("enableVoiceWarn").equals("true")) {
			vW = new voiceWarning();
			vW1 = new Thread(vW);
			vW.init(this, this.S);
			vW1.start();
		}

		if (getconfig("engineInfoSwitch").equals("true")) {
			// F1 = new Thread(F);
			// FI1 = new Thread(FI);
			FI = new engineInfo();
			FI.init(this, S, blkx);

			// F1.start();
			// FI1.start();
			//
		}

		if (getconfig("enableEngineControl").equals("true")) {
			F = new engineControl();
			F.init(this, S, blkx);
		}

		// if (getconfig("flapsControlSwitch").equals("true")) {
		// flc = new flapsControl();
		// try {
		// flc.init(this);
		// } catch (AWTException e) {
		// // TODO Auto-generated catch block
		// e.printStackTrace();
		// }
		// flc1 = new Thread(flc);
		// flc1.start();
		// }

		if (Boolean.parseBoolean(getconfig("crosshairSwitch"))) {
			H = new minimalHUD();
			// H1 = new Thread(H);
			H.init(this, S, O);
			// H1.start();
		}
		if (Boolean.parseBoolean(getconfig("flightInfoSwitch"))) {
			FL = new flightInfo();
			// FL1 = new Thread(FL);
			FL.init(this, S);
			// FL1.start();
		}
		if (Boolean.parseBoolean(getconfig("enableLogging"))) {
			if (dF != null) {
				dF.doit = false;
				dF = null;
			}
			notification(lang.cStartlog);
			Log = new flightLog();
			Log.init(this, S);
			// Log1 = new Thread(Log);
			// Log1.start();
			logon = true;
		}

		if (Boolean.parseBoolean(getconfig("enableAxis"))) {
			sV = new stickValue();
			sV.init(this, S);
			// sV1 = new Thread(sV);
			// sV1.start();
		}

		if (Boolean.parseBoolean(getconfig("enableAttitudeIndicator"))) {
			aI = new attitudeIndicator();
			aI.init(this, S);
			// aI1 = new Thread(aI);
			// aI1.start();
		}

		if (Boolean.parseBoolean(getconfig("enablegearAndFlaps"))) {
			fS = new gearAndFlaps();
			fS.init(this, S);
			// fS1 = new Thread(fS);
			// fS1.start();

		}
		if (app.debug) {
			// SA
			SA = new situationAware();
			SA.init(this, O);
			SA1 = new Thread(SA);
			SA1.start();

		}

		uT = new uiThread(this);
		uT1 = new Thread(uT);
		uT1.start();
		S.startTime = System.currentTimeMillis();
	}

	public void closepad() {
		if (getconfig("enableVoiceWarn").equals("true")) {
			vW.doit = false;
			vW1 = null;
			vW = null;
		}

		if (getconfig("engineInfoSwitch").equals("true")) {
			// F.doit = false;
			FI.doit = false;
			FI1 = null;
			// F1 = null;
			FI.dispose();
			// F.dispose();
			FI = null;
			// F = null;

		}

		if ((getconfig("enableEngineControl").equals("true"))) {
			F.doit = false;
			F1 = null;
			F.dispose();
			F = null;
		}
		// if (getconfig("flapsControlSwitch").equals("true")) {
		// flc.close();
		// flc = null;
		// flc1 = null;
		// }

		if (getconfig("crosshairSwitch").equals("true")) {
			H.doit = false;
			H1 = null;
			H.dispose();
			H = null;
		}

		if (getconfig("flightInfoSwitch").equals("true")) {
			FL.doit = false;
			FL1 = null;
			FL.dispose();
			FL = null;
		}

		if (getconfig("enableLogging").equals("true")) {
			notification(lang.cSavelog + Log.fileName + lang.cPlsopen);
			// System.out.println("阶段差:"+(Log.fA.curaltStage -
			// Log.fA.initaltStage));
			if (Log.fA.curaltStage - Log.fA.initaltStage >= 1) {
				dF = new drawFrame();
				showdrawFrame(Log.fA);
			}

			// logon = false;
			Log.close();
			// Log1 = null;
			Log = null;

		}
		if (getconfig("enableAxis").equals("true")) {
			sV.doit = false;
			sV1 = null;
			sV.dispose();
			sV = null;

		}
		if (getconfig("enableAttitudeIndicator").equals("true")) {
			aI.doit = false;
			aI1 = null;
			aI.dispose();
			aI = null;
		}

		if (getconfig("enablegearAndFlaps").equals("true")) {
			fS.doit = false;
			fS1 = null;
			fS.dispose();
			fS = null;

		}

		if (app.debug) {
			SA.doit = false;
			SA1 = null;
			SA.dispose();
			SA = null;

			// 释放

		}
		if (getconfig("enableFMPrint").equals("true")) {
			if (pt != null) {
				pt.doit = false;
				pt1 = null;
				pt.dispose();
				pt = null;
			}

			if (thrustdFS != null) {
				thrustdFS.doit = false;
				thrustdFS1 = null;
				thrustdFS.dispose();
				thrustdFS = null;
			}
		}
		uT.doit = false;
		uT1 = null;
	}

	public void initconfig() {
		cfg = new config("./config/config.properties");
		if (cfg.getValue("firstTime").equals("True")) {
			// config_init()?
		}
		// NotificationManager.showNotification(createWebNotification("配置信息读入完毕"));
	}

	public String getconfig(String key) {
		return cfg.getValue(key);
	}

	public Color getColorConfig(String key) {
		int R, G, B, A;
		R = Integer.parseInt(getconfig(key + "R"));
		G = Integer.parseInt(getconfig(key + "G"));
		B = Integer.parseInt(getconfig(key + "B"));
		A = Integer.parseInt(getconfig(key + "A"));
		return new Color(R, G, B, A);
	}

	public void setColorConfig(String key, Color c) {
		int R = c.getRed();
		int G = c.getGreen();
		int B = c.getBlue();
		int A = c.getAlpha();

		setconfig(key + "R", Integer.toString(R));
		setconfig(key + "G", Integer.toString(G));
		setconfig(key + "B", Integer.toString(B));
		setconfig(key + "A", Integer.toString(A));
	}

	public void loadFromConfig() {
		// 修改完设置在读取
		freqService = Long.parseLong(getconfig("Interval"));

		// 刷新频率比例
		freqEngineInfo = (long) (freqService * 2f);

		freqFlightInfo = (long) (freqService * 1.5f);
		freqAltitude = (long) (freqService * 1.5f);

		freqGearAndFlap = (long) (freqService * 2f);
		freqStickValue = (long) (freqService * 1f);
		app.threadSleepTime = (long) (freqService / 2);

		// System.out.println(freqService);

		// 颜色

		// 修改颜色
		// System.out.println(R +", " + G + ", " + B + "," +A);
		app.colorNum = getColorConfig("fontNum");

		// 标签颜色
		app.colorLabel = getColorConfig("fontLabel");

		// 单位颜色
		app.colorUnit = getColorConfig("fontUnit");

		// 警告颜色
		app.colorWarning = getColorConfig("fontWarn");

		// 描边颜色
		app.colorShadeShape = getColorConfig("fontShade");
		
		
		// 声音
		app.voiceVolumn = Integer.parseInt(getconfig("voiceVolume"));
		// fontLabelR=32
		// fontLabelG=222
		// fontLabelB=64
		// fontLabelA=140
		//
		// fontUnitR=166
		// fontUnitG=166
		// fontUnitB=166
		// fontUnitA=220
		//
		// fontWarnR=216
		// fontWarnG=33
		// fontWarnB=13
		// fontWarnA=100
		//
		// fontShadeR=0
		// fontShadeG=0
		// fontShadeB=0
		// fontShadeA=42
		showStatus = true;
		if (getconfig("enableStatusBar") != "")
			showStatus = Boolean.parseBoolean(getconfig("enableStatusBar"));
		// 读取字体绘制方式
		app.drawFontShape = !Boolean.parseBoolean(getconfig("simpleFont"));
	}

	public controller() {
		initconfig();// 装载设置文件
		// 接收频率
		// System.out.println("controller执行了");
		loadFromConfig();
		usetempratureInformation = false;

		// 刷新频率
		flag = 0;
		lastEvt = 0;
		lastDmg = 0;

		// 状态0，初始化主界面和设置文件
		// System.out.println("状态0，初始化主界面");

		M = new mainform(this);
		M1 = new Thread(M);
		M1.start();

		// G = new gcThread();
		// G.init(this);
		// gc = new Thread(G);
		// gc.start();
		// start();

		// 初始化ROBOT
		// try {
		// robot = new Robot();
		// } catch (AWTException e) {
		// // TODO Auto-generated catch block
		// e.printStackTrace();
		// }

	}

	public void start() {
		if (flag == 1) {

			// System.out.println(freqService);
			// 状态1，释放设置窗口传参初始化后台
			// System.out.println("状态1，传参初始化Service");
			M.doit = false;
			M1 = null;
			M.dispose();
			M = null;

			System.gc();
			// NotificationManager.showNotification(createWebNotification("程序最小化至托盘，注意右上角状态条提示"));

			S = new service(this);
			S1 = new Thread(S);
			S1.start();

		}

	}

	public void Preview() {

		loadFromConfig();
		if (Boolean.parseBoolean(getconfig("engineInfoSwitch"))) {
			FI = new engineInfo();
			FI.initPreview(this);
		}
		if (Boolean.parseBoolean(getconfig("enableEngineControl"))) {
			F = new engineControl();
			F.initPreview(this);
		}
		if (Boolean.parseBoolean(getconfig("crosshairSwitch"))) {
			H = new minimalHUD();
			H.initPreview(this);
		}
		if (Boolean.parseBoolean(getconfig("flightInfoSwitch"))) {
			FL = new flightInfo();
			FL.initPreview(this);
		}
		if (Boolean.parseBoolean(getconfig("enableAxis"))) {
			sV = new stickValue();
			sV.initpreview(this);
		}
		if (Boolean.parseBoolean(getconfig("enableAttitudeIndicator"))) {
			aI = new attitudeIndicator();
			aI.initpreview(this);
		}

		if (Boolean.parseBoolean(getconfig("enablegearAndFlaps"))) {
			fS = new gearAndFlaps();
			fS.initPreview(this);
		}
		if (app.debug) {
			SA = new situationAware();
			SA.initPreview(this);
		}
	}

	public void endPreview() {

		// System.out.println(F.getLocationOnScreen().x);
		// System.out.println(F.getLocationOnScreen().y);
		if (Boolean.parseBoolean(getconfig("engineInfoSwitch"))) {
			// shade问题需要加补偿
			// System.out.println(F.getLocationOnScreen().x);
			// System.out.println(F.getLocationOnScreen().y);
			setconfig("engineInfoX", Integer.toString(FI.getLocationOnScreen().x - 25));
			setconfig("engineInfoY", Integer.toString(FI.getLocationOnScreen().y - 25));

			FI.dispose();
			FI = null;

		}
		if (Boolean.parseBoolean(getconfig("enableEngineControl"))) {
			setconfig("engineControlX", Integer.toString(F.getLocationOnScreen().x - 25));
			setconfig("engineControlY", Integer.toString(F.getLocationOnScreen().y - 25));
			F.dispose();
			F = null;
		}

		if (Boolean.parseBoolean(getconfig("crosshairSwitch"))) {
			// shade问题需要加补偿
			setconfig("crosshairX", Integer.toString(H.getLocationOnScreen().x));
			setconfig("crosshairY", Integer.toString(H.getLocationOnScreen().y));
			H.dispose();
			H = null;
		}
		if (Boolean.parseBoolean(getconfig("flightInfoSwitch"))) {

			setconfig("flightInfoX", Integer.toString(FL.getLocationOnScreen().x - 25));
			setconfig("flightInfoY", Integer.toString(FL.getLocationOnScreen().y - 25));
			FL.dispose();
			FL = null;
		}
		if (Boolean.parseBoolean(getconfig("enableAxis"))) {
			// System.out.println(sV.getLocationOnScreen().x );
			// System.out.println(sV.getLocationOnScreen().y);
			setconfig("stickValueX", Integer.toString(sV.getLocationOnScreen().x));
			setconfig("stickValueY", Integer.toString(sV.getLocationOnScreen().y));
			sV.dispose();
			sV = null;
		}
		if (Boolean.parseBoolean(getconfig("enableAttitudeIndicator"))) {
			setconfig("attitudeIndicatorX", Integer.toString(aI.getLocationOnScreen().x));
			setconfig("attitudeIndicatorY", Integer.toString(aI.getLocationOnScreen().y));
			setconfig("attitudeIndicatorWidth", Integer.toString(aI.getWidth() - 4));
			setconfig("attitudeIndicatorHeight", Integer.toString(aI.getHeight() - 4));
			aI.dispose();
			aI = null;
		}

		if (Boolean.parseBoolean(getconfig("enablegearAndFlaps"))) {
			// System.out.println(fS.getLocationOnScreen().x );
			// System.out.println(fS.getLocationOnScreen().y);

			setconfig("gearAndFlapsX", Integer.toString(fS.getLocationOnScreen().x));
			setconfig("gearAndFlapsY", Integer.toString(fS.getLocationOnScreen().y));

			fS.dispose();
			fS = null;
		}
		if (app.debug) {
			// System.out.println(SA.getLocationOnScreen().x );
			// System.out.println(SA.getLocationOnScreen().y);
			setconfig("situationAwareX", Integer.toString(SA.getLocationOnScreen().x - 15));
			setconfig("situationAwareY", Integer.toString(SA.getLocationOnScreen().y - 15));

			SA.dispose();
			SA = null;

		}
		saveconfig();

		loadFromConfig();
		// 释放

		System.gc();

	}

	public void setconfig(String key, String value) {
		cfg.setValue(key, value);
	}

	public void saveconfig() {
		cfg.saveFile("./config/config.properties", "8111");
		// NotificationManager.showNotification(createWebNotification("配置信息写入完毕"));
	}

	public static void notificationtime(String text, int time) {
		NotificationManager.showNotification(createWebNotifications(text, time));
	}

	public static void notificationtimeAbout(String text, int time) {
		NotificationManager.showNotification(createWebNotificationsAbout(text, time));
	}

	public static void notification(String text) {
		NotificationManager.showNotification(createWebNotification(text));
	}

	static WebNotification createWebNotificationTime(long time) {
		WebNotification a = new WebNotification();
		// WebLabel text1=new WebLabel(text);
		// text1.setFont(app.DefaultFont);
		// text1.setVisible(false);
		a.setFont(app.DefaultFont);
		a.setIcon(NotificationIcon.clock.getIcon());

		a.setWindowOpacity((float) (0.5));
		WebClock clock = new WebClock();
		clock.setClockType(ClockType.timer);
		clock.setTimeLeft(time);
		clock.setFont(app.DefaultFont);
		clock.setTimePattern(lang.cOpenpad);
		a.setContent(new GroupPanel(clock));
		// a.setOpaque(true);
		clock.start();
		a.setDisplayTime(time);

		a.setFocusable(false);
		return a;

	}

	static WebNotification createWebNotificationEngineTime(long time) {
		WebNotification a = new WebNotification();
		// WebLabel text1=new WebLabel(text);
		// text1.setFont(app.DefaultFont);
		// text1.setVisible(false);
		a.setFont(app.DefaultFont);
		a.setIcon(NotificationIcon.clock.getIcon());

		a.setWindowOpacity((float) (0.5));
		WebClock clock = new WebClock();
		clock.setClockType(ClockType.timer);
		clock.setTimeLeft(time);
		clock.setFont(app.DefaultFont);
		clock.setTimePattern(lang.cEnginedmg);
		a.setContent(new GroupPanel(clock));
		// a.setOpaque(true);
		clock.start();
		a.setDisplayTime(time);

		a.setFocusable(false);
		return a;

	}

	static WebNotification createWebNotifications(String text, int time) {
		WebNotification a = new WebNotification();
		WebLabel text1 = new WebLabel(text);
		text1.setFont(app.DefaultFont);
		// text1.setVisible(false);
		// a.setWindowOpacity((float) (0.5));

		a.setFont(app.DefaultFont);
		a.setIcon(NotificationIcon.information.getIcon());
		a.add(text1);
		a.setDisplayTime(time);
		a.setFocusable(false);
		return a;

	}

	static WebNotification createWebNotificationsAbout(String text, int time) {
		WebNotification a = new WebNotification();
		WebLabel text1 = new WebLabel(text);
		text1.setFont(new Font(app.DefaultFontName, Font.PLAIN, 14));
		// text1.setVisible(false);
		// a.setWindowOpacity((float) (0.5));
		Image I = Toolkit.getDefaultToolkit().createImage("image/fubuki.jpg");
		ImageIcon A = new ImageIcon(I);
		a.setFont(app.DefaultFont);
		a.setIcon(A);
		a.add(text1);
		a.setDisplayTime(time);
		a.setFocusable(false);
		return a;

	}

	static WebNotification createWebNotification(String text) {
		WebNotification a = new WebNotification();
		WebLabel text1 = new WebLabel(text);
		text1.setFont(app.DefaultFont);
		// text1.setVisible(false);
		// a.setWindowOpacity((float) (0.5));

		a.setFont(app.DefaultFont);
		a.setIcon(NotificationIcon.information.getIcon());
		a.add(text1);
		a.setDisplayTime(5000);
		a.setFocusable(false);
		return a;

	}

	static WebNotification createWebNotificationEngineBomb(String text) {
		WebNotification a = new WebNotification();
		WebLabel text1 = new WebLabel(text);
		text1.setFont(app.DefaultFont);
		// text1.setVisible(false);
		// a.setWindowOpacity((float) (0.5));

		a.setFont(app.DefaultFont);
		a.setIcon(NotificationIcon.error);
		a.add(text1);
		a.setDisplayTime(3000);
		a.setFocusable(false);
		return a;

	}

	public void showdrawFrame(flightAnalyzer fA) {
		dF.init(this, fA);
	}

	public void writeDown() {

		if (logon) {
			if (Log.doit == false) {
				Log.doit = true;
				// Log1.start();

			} else {
				Log.doit = true;
				System.out.println("线程同步错误");
			}
			// System.out.println(Log.doit);
		}
	}

	void getfmdata(String planename) {
		String fmfile = null;
		// String unitSystem;
		int i;
		// 读入fm

		blkx = new blkxparser("./data/aces/gamedata/flightmodels/" + planename + ".blkx", planename + ".blk");
		if (blkx.valid == true) {
			fmfile = blkx.getlastone("fmfile");
			fmfile = fmfile.substring(1, fmfile.length() - 1);
			if (fmfile.indexOf("blk") == -1)
				fmfile = fmfile + ".blk";
			for (i = 0; i < fmfile.length(); i++) {
				if (fmfile.charAt(i) == '/')
					break;
			}
			// System.out.println(fmfile);
			if (i + 1 >= fmfile.length()) {
				fmfile = planename + ".blk";
			} else
				fmfile = fmfile.substring(i + 1);
		}
		// System.out.println(fmfile);

		// 读入fmfile
		if (fmfile != null)
			blkx = new blkxparser("./data/aces/gamedata/flightmodels/fm/" + fmfile + "x", fmfile);

		if (blkx.valid == true) {// System.out.println(blkx.data);
			blkx.getAllplotdata();
			blkx.getload();
		}

	}

}