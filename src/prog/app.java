package prog;

import java.awt.AWTException;
import java.awt.Color;
import java.awt.Cursor;
import java.awt.Font;
import java.awt.FontFormatException;
import java.awt.GraphicsEnvironment;
import java.awt.Image;
import java.awt.MenuItem;
import java.awt.Point;
import java.awt.PopupMenu;
import java.awt.RenderingHints;
import java.awt.SystemTray;
import java.awt.Toolkit;
import java.awt.TrayIcon;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.image.BufferedImage;
import java.io.BufferedReader;
import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.net.InetSocketAddress;
import java.net.SocketAddress;
import java.nio.charset.Charset;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import javax.swing.SwingUtilities;


import com.alee.global.StyleConstants;
import com.alee.laf.WebLookAndFeel;
import com.github.kwhat.jnativehook.GlobalScreen;
import com.github.kwhat.jnativehook.NativeHookException;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;
import com.github.kwhat.jnativehook.keyboard.NativeKeyListener;

// import parser.stringHelper;

public class app {
	// 一些全局配置
	public static final boolean debug = false;
	// 测试FM
	public static boolean fmTesting = false;
	
	// 调试日志
	public static final boolean debugLog = false;
	public static final int maxEngLoad = 10;

	// 用于检查最新版本
	public static String owner = "matrixsukhoi";
	public static String repository = "voidmei";

	public static final long gcSeconds = 15;
	public static final Color previewColor = new Color(0, 0, 0, 10);

	public static String appName;
	public static String defaultNumfontName = "Roboto";
	public static String appTooltips;
	public static String version = "1.567";
	public static String httpHeader;
	public static int voiceVolumn = 100;
	public static String defaultFontName = "Microsoft YaHei UI";
	public static Font defaultFont;
	public static Font defaultFontBig;
	public static Font defaultFontBigBold;
	public static Font defaultFontSmall;
	public static int defaultFontsize;

	public static Process plugin = null;
	public static Runtime r;

	// 抗锯齿
	public static Boolean aaEnable = true;
	public static Object textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_GASP;
	public static Object graphAASetting = RenderingHints.VALUE_ANTIALIAS_ON;
//	public static Object textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_ON;

	public static Color colorFailure = new Color(255, 69, 0, 100);
	public static Color colorWarning = new Color(216, 33, 13, 100);
	public static Color colorShade = new Color(0, 0, 0, 240);
	public static Color colorShadeShape = new Color(0, 0, 0, 42);
	public static Color colorUnit = new Color(166, 166, 166, 220);
	public static Color colorLabel = new Color(27, 255, 128, 166);
	public static Color colorNum = new Color(27, 255, 128, 240);

	public static SocketAddress requestDest; // = new InetSocketAddress("127.0.0.1", 8111);
	public static SocketAddress requestDestBkp; // = new InetSocketAddress("127.0.0.1", 9222);
	public static int appPort;
	public static int appPortBkp;

	// 线程休眠时间
	public static long threadSleepTime = 33;
	// 图形环境
	public static GraphicsEnvironment environment;
	public static int screenWidth;
	public static int screenHeight;
	public static String[] fonts;


	public static controller ctr;

	public static Boolean displayFm = true;
	public static Boolean displayFmCtrl = false;
	// 空鼠标指针
	public static BufferedImage cursorImg = new BufferedImage(16, 16, BufferedImage.TYPE_INT_ARGB);
	public static Cursor blankCursor = Toolkit.getDefaultToolkit().createCustomCursor(cursorImg, new Point(0, 0),
			"blank cursor");

	// 是否开启
	public static boolean drawFontShape = false;
	public static ExecutorService threadPool;
	public static String getJavaVersion() {
		r = Runtime.getRuntime();
		try {

			String cmd1 = "java -version";
			Process p = r.exec(cmd1);
			/* 读取执行结果 */
			BufferedReader br = new BufferedReader(new InputStreamReader(p.getErrorStream(), "UTF-8"));
			String line = null;
			StringBuilder sb = new StringBuilder();
			while ((line = br.readLine()) != null) {
				sb.append(line);
			}
			p.getInputStream().close();
			/* 正则提取版本 */
			
			Pattern pt = Pattern.compile("\"[0-9].[0-9].*\"");
			Matcher m = pt.matcher(sb.toString());
			
			
//			app.debugPrint("输出"+sb.toString());
			if (m.find()) {
				String ret = m.group(0);
				return ret;
			}
			
		} catch (Exception e1) {
			e1.printStackTrace();
		}
		return "0.0";
	}
	public void pluginopen() {
		r = Runtime.getRuntime();
		int tasklist1 = -1;
		try {

			String cmd1 = "cmd.exe /c  tasklist";
			Process p = Runtime.getRuntime().exec(cmd1);
			StringBuffer out = new StringBuffer();
			byte[] b = new byte[1024];
			for (int n; (n = p.getInputStream().read(b)) != -1;) {
				out.append(new String(b, 0, n));
			}
			tasklist1 = out.toString().indexOf("TaskBarHider.exe");// 检查进程
		} catch (Exception e1) {
			e1.printStackTrace();
		}
		if (tasklist1 == -1) {
			// 程序在进程中没有发现
			try {
				plugin = r.exec(System.getProperty("user.dir") + "\\TaskBarHider.exe");
			} catch (IOException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
		} else {
			if (debug)
				debugPrint("TaskBarHider程序已经打开!");
		}

		// robot.keyRelease(17);
		// robot.keyRelease(192);
	}

	public void pluginoff() {
		plugin.destroy();
	}

	public static void debugPrint(String t) {
		System.out.println(t);
	}

	public static void initSystemTray() {
		if (SystemTray.isSupported()) {
			SystemTray tray = SystemTray.getSystemTray();
			Image image = Toolkit.getDefaultToolkit().getImage("image/16x16.png");
			TrayIcon icon = new TrayIcon(image);
			icon.setToolTip(appName);
			PopupMenu p = new PopupMenu("");
			MenuItem close = new MenuItem(lang.close);
			MenuItem about = new MenuItem(lang.about);
			close.setFont(defaultFont);
			about.setFont(defaultFont);
			p.setFont(defaultFont);
			close.addActionListener(new ActionListener() {

				public void actionPerformed(ActionEvent e) {
					tray.remove(icon);
					System.exit(0);
				}
			});
			about.addActionListener(new ActionListener() {

				public void actionPerformed(ActionEvent e) {
					// controller.s
					controller.notificationtimeAbout(lang.aboutcontentsub2, 24000);
					controller.notificationtimeAbout(lang.aboutcontentsub1, 16000);
					controller.notificationtimeAbout(lang.aboutcontent, 8000);

				}
			});
			p.add(about);
			p.add(close);
			icon.setPopupMenu(p);

			// left click
			icon.addMouseListener(new MouseAdapter() {
			@Override
			public void mouseClicked(MouseEvent e) {
				if (e.getButton() == MouseEvent.BUTTON1) {
						ctr.stop();
						ctr = new controller();
					}
				}
			});

			try {
				tray.add(icon);
			} catch (AWTException e1) {
				// TODO Auto-generated catch block
				// e1.printStackTrace();
				debugPrint(lang.failaddtoTray);
			}
		}
	}

	public static void checkOS() {
		if (Float.parseFloat(System.getProperty("os.version")) < 6.0) {
			controller.notificationtime(lang.Systemerror, 10000);
			try {
				Thread.sleep(10000);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
		}
	}

	public static void initFont() {
		environment = GraphicsEnvironment.getLocalGraphicsEnvironment();
		
		// 遍历所有font
		File file = new File("fonts/");
		String[] filelist = file.list();
		for (int i = 0; i < filelist.length; i++) {
			try {

			    //create the font to use. Specify the size!
			    Font customFont = Font.createFont(Font.TRUETYPE_FONT, new File("fonts/"+filelist[i]));
			    //register the font

			    environment.registerFont(customFont);

			} catch (IOException e) {
			    e.printStackTrace();

			} catch(FontFormatException e) {
			    e.printStackTrace();
			}
		}
		
		
		fonts = environment.getAvailableFontFamilyNames();// 获得系统字体
		
		Boolean findFont = false;
		for (int i = 0; i < fonts.length; i++) {
			// debugPrint(fonts[i]);
			if (fonts[i].equals(defaultFontName)) {
				findFont = true;
			}
		}
		if (!findFont) {
			debugPrint("font can not find\n");
			if (defaultFontName.equals("Microsoft YaHei UI")) {
				defaultFontName = "宋体";
			} else
				defaultFontName = "Arial";
		}
		defaultFont = new Font(defaultFontName, Font.PLAIN, defaultFontsize);
		defaultFontBig = new Font(defaultFontName, Font.PLAIN, defaultFontsize + 2);
		defaultFontBigBold = new Font(defaultFontName, Font.BOLD, defaultFontsize + 4);
		defaultFontSmall = new Font(defaultFontName, Font.PLAIN, defaultFontsize - 2);
		environment = null;
	}

	public static void getScreenSize() {
		screenWidth = Toolkit.getDefaultToolkit().getScreenSize().width;
		screenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
	}

	public static void setDebugLog(String path) {
		PrintStream out = null;
		try {
			out = new PrintStream(path);
		} catch (FileNotFoundException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		System.setOut(out);
	}

	public static void setErrLog(String path) {
		PrintStream out = null;
		try {
			out = new PrintStream(path);
		} catch (FileNotFoundException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		System.setErr(out);
	}

	public static void setUTF8() {
		if (!System.getProperty("file.encoding").equals("UTF-8")) {

			System.out.println("Default Charset=" + Charset.defaultCharset());
			System.out.println("file.encoding=" + System.getProperty("file.encoding"));
			System.out.println("Default Charset=" + Charset.defaultCharset());
			System.setProperty("file.encoding", "UTF-8");
			Field charset;
			try {
				charset = Charset.class.getDeclaredField("defaultCharset");

				charset.setAccessible(true);
				charset.set(null, null);
			} catch (NoSuchFieldException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			} catch (SecurityException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			} catch (IllegalArgumentException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			} catch (IllegalAccessException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
		}
	}

	public static void checkUpdate() {
		httpHelper httpClient = new httpHelper();
		try {
			/* 异步请求 */
			threadPool.submit(() -> {
				String res;
				res = httpClient.sendGetURL("https://api.github.com/repos/"+ owner + "/" + repository + "/releases/latest");
				// debugPrint(res);
				/* 截取tag_name */
				int sidx = res.indexOf("tag_name");
				int eidx = res.indexOf(",", sidx);
				res = res.substring(sidx, eidx);
				/* 正则匹配版本号 */
				Pattern pt = Pattern.compile("[0-9].([0-9])*");
				Matcher m = pt.matcher(res);
				if (m.find()) {
					String latestVersion = m.group(0);
					debugPrint("latest version is:" + latestVersion);
					if (Double.parseDouble(version) < Double.parseDouble(latestVersion)){
						String notice = "A newer version is released on github, version: " + latestVersion;
						controller.notificationtimeAbout(String.format(notice), 5000);
					}
				}
				return null;
			});

		} catch (Exception e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		
	}

	public static void checkBlkxUpdate() {
		httpHelper httpClient = new httpHelper();
		try {
			/* 异步请求 */
			threadPool.submit(() -> {
				String res;
				res = httpClient.sendGetURL("https://api.github.com/repos/"+ owner + "/" + repository + "/releases/latest");
				// debugPrint(res);
				/* 截取tag_name */
				int sidx = res.indexOf("tag_name");
				int eidx = res.indexOf(",", sidx);
				res = res.substring(sidx, eidx);
				/* 正则匹配版本号 */
				Pattern pt = Pattern.compile("[0-9].([0-9])*");
				Matcher m = pt.matcher(res);
				if (m.find()) {
					String latestVersion = m.group(0);
					debugPrint("latest version is:" + latestVersion);
					if (Double.parseDouble(version) < Double.parseDouble(latestVersion)){
						String notice = "A newer version is released on github, version: " + latestVersion;
						controller.notificationtimeAbout(String.format(notice), 5000);
					}
				}
				return null;
			});

		} catch (Exception e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		
	}
	public static void addDisplayFmListener() {
		try {
			GlobalScreen.registerNativeHook();
		} catch (NativeHookException ex) {
			debugPrint("There was a problem registering the native hook.");
			debugPrint(ex.getMessage());
		}

		GlobalScreen.addNativeKeyListener(new NativeKeyListener() {
			public void nativeKeyPressed(NativeKeyEvent e) {
				if (e.getKeyCode() == NativeKeyEvent.VC_P) {
					debugPrint("switch fmDisplay");
					displayFm = !displayFm;
				}
			}
		});
	}


	public static void initWebLaf() {
		WebLookAndFeel.install();

		StyleConstants.textRenderingHints = new RenderingHints(RenderingHints.KEY_TEXT_ANTIALIASING,
				RenderingHints.VALUE_TEXT_ANTIALIAS_ON);

		// WebLookAndFeel.set
		WebLookAndFeel.globalControlFont = defaultFont;
		WebLookAndFeel.globalTooltipFont = defaultFont;
		WebLookAndFeel.globalAlertFont = defaultFont;
		WebLookAndFeel.globalMenuFont = defaultFont;
		WebLookAndFeel.globalAcceleratorFont = defaultFont;
		WebLookAndFeel.globalTitleFont = defaultFont;
		WebLookAndFeel.globalTextFont = defaultFont;
		WebLookAndFeel.setDecorateFrames(true);
		WebLookAndFeel.setDecorateAllWindows(true);

		WebLookAndFeel.setAllowLinuxTransparency(true);
	}

	public static void main(String[] args) {

		// set output stream
		setUTF8();
		
		app.debugPrint("Java版本为 " + System.getProperty("java.version"));
		if (System.getProperty("java.version").indexOf("1.8.0") == -1) {
			System.out.println("检测到java版本非1.8.0版本，程序运行可能出现问题");
		}
		
		
		if (app.debugLog) {
			setDebugLog("./output.log");
			setErrLog("./error.log");
		}

		lang.initLang();

		// 初始化端口
		appPort = Integer.parseInt(lang.httpPort);
		appPortBkp = appPort + 1111;
		requestDest = new InetSocketAddress(lang.httpIp, appPort);
		requestDestBkp = new InetSocketAddress(lang.httpIp, appPortBkp);

		// 相关变量初始化
		appName = lang.appName;
		appTooltips = lang.appTooltips;
		httpHeader = lang.httpHeader;
		defaultFontName = lang.lanuageConfig.getValue("defaultFontName");
		defaultFontsize = Integer.parseInt(lang.lanuageConfig.getValue("defaultFontSize"));

		// 线程池
		threadPool = Executors.newCachedThreadPool();

		// checkOS();
		initFont();
		getScreenSize();

		// System.setProperty("awt.useSystemAAFontSettings", "on");

		initSystemTray();

		checkUpdate();

		if (displayFmCtrl) addDisplayFmListener();

		SwingUtilities.invokeLater(new Runnable() {
			public void run() {
				// Install WebLaF as application L&F
				initWebLaf();
				ctr = new controller();

				if (System.getProperty("java.version").indexOf("1.8") == -1) {
					controller.notificationtimeAbout(String.format("Detected current Java version %s. Java 1.8 is needed.", System.getProperty("java.version")) , 3000);
				}
			}
		});

	}

}