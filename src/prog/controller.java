package prog;

import java.awt.AWTException;
import java.awt.Font;
import java.awt.Image;
import java.awt.Toolkit;

import javax.swing.ImageIcon;

import com.alee.extended.panel.GroupPanel;
import com.alee.extended.time.ClockType;
import com.alee.extended.time.WebClock;
import com.alee.laf.label.WebLabel;
import com.alee.managers.notification.NotificationIcon;
import com.alee.managers.notification.NotificationManager;
import com.alee.managers.notification.WebNotification;

import parser.flightAnalyzer;
import parser.flightLog;
import ui.statusBar;
import ui.stickValue;
import ui.crosshair;
import ui.drawFrame;
import ui.engineInfo;
import ui.flightInfo;
import ui.gearAndFlaps;
import ui.mainform;
import ui.situationAware;

public class controller {

	public int flag;

	public boolean logon = false;

	engineInfo F;
	statusBar SB;
	mainform M;
	crosshair H;
	otherService O;
	situationAware SA;
	flightInfo FL;
	flightLog Log;
	drawFrame dF;
	stickValue sV;
	gearAndFlaps fS;
	flapsControl flc;
	
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
	
	service S;
	public config cfg;
	// 存储参数
	// 主参数

	public static int freqService;// Service取数据与计算周期
	// 发动机面板
	public static int freqengineInfo;// engineInfo刷新周期

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
	long overheatCheckMili;
	long restoreCheckMili;
	public int overheattime;
	public int restoretime;
	public int availableoverheattime;
	public boolean isfirstOverheat;
	WebNotification tempCheck;

	public void initStatusBar() {
		// 状态1，初始化状态条
		if (flag == 1) {
			// System.out.println("状态1，初始化状态条");
			SB = new statusBar();
			SB.init(this);
			SB.S1();
			SB1 = new Thread(SB);
			SB1.start();
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
			SB.S2();
			flag = 3;
		}
	}

	public void changeS3() {
		// 状态3，连接成功，释放状态条，打开面板
		// SB.repaint();
		if (flag == 3) {
			// 初始化MapObj以及Msg、gamechat

			// System.out.println("状态3，连接成功，释放状态条，打开面板");
			usetempratureInformation = Boolean.parseBoolean(getconfig("usetempInfoSwitch"));
			// System.out.println(usetempratureInformation);
			NotificationManager.showNotification(createWebNotificationTime(3000));
			SB.S3();
			SB.doit = false;
			SB.dispose();
			SB = null;
			restoretime = 120;
			isfirstOverheat = true;
			restoreCheckMili = System.currentTimeMillis();
			O = new otherService();
			O.init(this);
			O1 = new Thread(O);
			O1.start();
			

			flag = 4;
			openpad();

		}
	}

	public void S4toS1() {
		// 状态4，游戏返回，返回至状态1
		if (flag == 4) {
			// System.out.println("状态4，游戏退出，释放Service资源，返回至状态1");
			closepad();
			// 释放资源
			lastEvt = O.lastEvt;
			lastDmg = O.lastDmg;
			// System.out.println("最后DMGID"+lastDmg);
			O.close();
			O = null;
			O1 = null;
			S.clear();
			flag = 1;
		}

	}

	public void openpad() {
		if (getconfig("engineInfoSwitch").equals("true")) {
			F = new engineInfo();
			F1 = new Thread(F);
			F.init(this, S);
			F1.start();
			//
		}
		if(getconfig("flapsControlSwitch").equals("true")){
			flc=new flapsControl();
			try {
				flc.init(this);
			} catch (AWTException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			flc1=new Thread(flc);
			flc1.start();
		}

		if (Boolean.parseBoolean(getconfig("crosshairSwitch"))) {
			H = new crosshair();
			H1 = new Thread(H);
			H.init(this, S,O);
			H1.start();
		}
		if (Boolean.parseBoolean(getconfig("flightInfoSwitch"))) {
			FL = new flightInfo();
			FL1 = new Thread(FL);
			FL.init(this, S);
			FL1.start();
		}
		if (Boolean.parseBoolean(getconfig("enableLogging"))) {
			if (dF != null) {
				dF.doit = false;
				dF = null;
			}
			notification(language.cStartlog);
			Log = new flightLog();
			Log.init(this, S);
			Log1 = new Thread(Log);
			Log1.start();
			logon = true;
		}

		if (Boolean.parseBoolean(getconfig("enableAxis"))) {
			sV = new stickValue();
			sV.init(this, S);
			sV1 = new Thread(sV);
			sV1.start();
		}
		if (Boolean.parseBoolean(getconfig("enablegearAndFlaps"))) {
			fS = new gearAndFlaps();
			fS.init(this, S);
			fS1 = new Thread(fS);
			fS1.start();

		}
		if (app.debug) {
			// SA
			SA = new situationAware();
			SA.init(this, O);
			SA1 = new Thread(SA);
			SA1.start();

		}
		S.startTime = System.currentTimeMillis();
	}

	public void closepad() {
		if (getconfig("engineInfoSwitch").equals("true")) {
			F.doit = false;
			F1 = null;
			F.fclose();
			F = null;

		}
		if(getconfig("flapsControlSwitch").equals("true")){
			flc.close();
			flc=null;
			flc1=null;
		}
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
			notification(language.cSavelog + Log.fileName + language.cPlsopen);
			//System.out.println("阶段差:"+(Log.fA.curaltStage - Log.fA.initaltStage));
			if (Log.fA.curaltStage - Log.fA.initaltStage >= 1) {
				dF = new drawFrame();
				showdrawFrame(Log.fA);
			}

			logon = false;
			Log.close();
			Log1 = null;
			Log = null;

		}
		if (getconfig("enableAxis").equals("true")) {
			sV.doit = false;
			sV1 = null;
			sV.dispose();
			sV = null;

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
	}

	public void init() {
		initconfig();// 装载设置文件
		// 接收频率
		// System.out.println("controller执行了");
		usetempratureInformation = false;
		freqService = 80;
		overheatCheckMili = System.currentTimeMillis();
		// 刷新频率
		freqengineInfo = 100;
		flag = 0;
		lastEvt = 0;
		lastDmg = 0;

		// 状态0，初始化主界面和设置文件
		// System.out.println("状态0，初始化主界面");
		initconfig();// 装载设置文件
		M = new mainform();
		M1 = new Thread(M);
		M.init(this);
		M1.start();
		M.doit = true;
		// start();

	}

	public void start() {
		if (flag == 1) {
			freqService = Integer.parseInt(getconfig("Interval"));

			// System.out.println(freqService);
			// 状态1，释放设置窗口传参初始化后台
			// System.out.println("状态1，传参初始化Service");
			M.doit = false;
			M.dispose();
			M = null;
			System.gc();
			// NotificationManager.showNotification(createWebNotification("程序最小化至托盘，注意右上角状态条提示"));

			S = new service();

			S1 = new Thread(S);

			S.init(this);

			S1.start();

		}

	}

	public void Preview() {
		if (Boolean.parseBoolean(getconfig("engineInfoSwitch"))) {
			F = new engineInfo();
			F.initPreview(this);
		}
		if (Boolean.parseBoolean(getconfig("crosshairSwitch"))) {
			H = new crosshair();
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
		if (Boolean.parseBoolean(getconfig("enablegearAndFlaps"))) {
			fS = new gearAndFlaps();
			fS.initPreview(this);
		}
		if (app.debug) {
			SA = new situationAware();
			SA.initPreview(this);

			// dF=new drawFrame();
			// showdrawFrame(null);

		}
	}

	public void endPreviewengineInfo() {

		// System.out.println(F.getLocationOnScreen().x);
		// System.out.println(F.getLocationOnScreen().y);
		if (Boolean.parseBoolean(getconfig("engineInfoSwitch"))) {
			// shade问题需要加补偿
			// System.out.println(F.getLocationOnScreen().x);
			// System.out.println(F.getLocationOnScreen().y);
			setconfig("engineInfoX", Integer.toString(F.getLocationOnScreen().x - 15));
			setconfig("engineInfoY", Integer.toString(F.getLocationOnScreen().y - 15));
			F.dispose();
			F = null;
		}
		if (Boolean.parseBoolean(getconfig("crosshairSwitch"))) {
			// shade问题需要加补偿
			setconfig("crosshairX", Integer.toString(H.getLocationOnScreen().x + 10));
			setconfig("crosshairY", Integer.toString(H.getLocationOnScreen().y + 10));
			H.dispose();
			H = null;
		}
		if (Boolean.parseBoolean(getconfig("flightInfoSwitch"))) {

			setconfig("flightInfoX", Integer.toString(FL.getLocationOnScreen().x - 15));
			setconfig("flightInfoY", Integer.toString(FL.getLocationOnScreen().y - 15));
			FL.dispose();
			FL = null;
		}
		if (Boolean.parseBoolean(getconfig("enableAxis"))) {
			// System.out.println(sV.getLocationOnScreen().x );
			// System.out.println(sV.getLocationOnScreen().y);
			setconfig("stickValueX", Integer.toString(sV.getLocationOnScreen().x + 10));
			setconfig("stickValueY", Integer.toString(sV.getLocationOnScreen().y + 10));
			sV.dispose();
			sV = null;
		}
		if (Boolean.parseBoolean(getconfig("enablegearAndFlaps"))) {
			// System.out.println(fS.getLocationOnScreen().x );
			// System.out.println(fS.getLocationOnScreen().y);

			setconfig("gearAndFlapsX", Integer.toString(fS.getLocationOnScreen().x + 10));
			setconfig("gearAndFlapsY", Integer.toString(fS.getLocationOnScreen().y + 10));

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

		// 释放

		System.gc();

	}

	public void initconfig() {
		cfg = new config("./config/config.properties");
		// NotificationManager.showNotification(createWebNotification("配置信息读入完毕"));
	}

	public String getconfig(String key) {
		return cfg.getValue(key);
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
		clock.setTimePattern(language.cOpenpad);
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
		clock.setTimePattern(language.cEnginedmg);
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
		text1.setFont(new Font(app.DefaultFontName,Font.PLAIN,14));
		// text1.setVisible(false);
		// a.setWindowOpacity((float) (0.5));
		Image I=Toolkit.getDefaultToolkit().createImage("image/fubuki.jpg");
		ImageIcon A=new ImageIcon(I);
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

	void startOverheatTime() {
		overheatCheckMili = System.currentTimeMillis();
		step = 0;
		if (!isfirstOverheat) {
			/*
			 * System.out.println("第二次过热,上次过热时间：" + overheattime + "上次拥有时间" +
			 * restoretime + "本次恢复时长" + (overheatCheckMili - restoreCheckMili) /
			 * 500 + "下次可用时间" + ((restoretime - overheattime) +
			 * (overheatCheckMili - restoreCheckMili) / 500));
			 */
			restoretime = (int) ((restoretime - overheattime) + (overheatCheckMili - restoreCheckMili));
		}
		if (restoretime > 120)
			restoretime = 120;
	}

	void updateOverheatTime() {
		int Time = (int) ((System.currentTimeMillis() - overheatCheckMili) / 1000);
		if (F != null)
			F.updateOverheatTime(Time);
		availableoverheattime = restoretime - Time;
		if (usetempratureInformation) {
			if (availableoverheattime < 60) {
				if (step == 0) {
					tempCheck = createWebNotification(language.cWarn1min);
					NotificationManager.showNotification(tempCheck);
					step = 1;
				}
			}
			if (availableoverheattime < 30) {

				if (step == 1) {
					tempCheck = createWebNotificationEngineTime(30000);
					NotificationManager.showNotification(tempCheck);
					step = 2;
				}
			}
			if (availableoverheattime < 1) {
				if (step == 2) {
					tempCheck = createWebNotificationEngineBomb(language.cEngBomb);
					NotificationManager.showNotification(tempCheck);
					step = 3;
				}

			}
		}
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

	void endOverheatTime() {

		restoreCheckMili = System.currentTimeMillis() - 1000;
		overheattime = F.overheattime;
		if (overheattime > 5)
			isfirstOverheat = false;
		step = 0;
		if (F != null)
			F.updateOverheatTime(0);
	}
}