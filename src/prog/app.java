package prog;

import java.awt.AWTException;
import java.awt.Color;
import java.awt.Cursor;
import java.awt.Font;
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
import java.awt.image.BufferedImage;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.nio.charset.Charset;

import javax.swing.SwingUtilities;

import com.alee.global.StyleConstants;
import com.alee.laf.WebLookAndFeel;

public class app {

	// 一些全局配置
	public static final boolean debug = false;
	public static final boolean foreignLanguage = false;

	public static final long gcSeconds = 10;
	public static final Color previewColor = new Color(0, 0, 0, 10);

	public static String appName = lang.appName;
	public static String DefaultNumfontName = "Roboto";
	public static String appTooltips = lang.appTooltips;
	public static String version = "1.46";
	public static String httpHeader = lang.httpHeader;
	public static int voiceVolumn = 100;
	public static String DefaultFontName = "Microsoft YaHei UI";
	public static Font DefaultFont;
	public static Font DefaultFontBig;
	public static Font DefaultFontBigBold;
	public static Font DefaultFontsmall;
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

	// 线程睡眠时间

	public static long threadSleepTime = 33;
	// 图形环境
	public static GraphicsEnvironment environment;
	public static int ScreenWidth;
	public static int ScreenHeight;
	public static String[] fonts;
	
	public static controller ctr;

	// 空鼠标指针
	public static BufferedImage cursorImg = new BufferedImage(16, 16, BufferedImage.TYPE_INT_ARGB);
	public static Cursor blankCursor = Toolkit.getDefaultToolkit().createCustomCursor(cursorImg, new Point(0, 0),
			"blank cursor");

	// 是否开启
	public static boolean drawFontShape = false;

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
				System.out.println("TaskBarHider程序已经打开!");
		}

		// robot.keyRelease(17);
		// robot.keyRelease(192);
	}

	public void pluginoff() {
		plugin.destroy();
	}

	public static void initSystemTray() {
		if (SystemTray.isSupported()) {
			SystemTray st = SystemTray.getSystemTray();
			Image image = Toolkit.getDefaultToolkit().getImage("image/16x16.png");
			TrayIcon ti = new TrayIcon(image);
			ti.setToolTip(appName);
			PopupMenu p = new PopupMenu("");
			MenuItem close = new MenuItem(lang.close);
			MenuItem about = new MenuItem(lang.about);
			close.setFont(DefaultFont);
			about.setFont(DefaultFont);
			p.setFont(DefaultFont);
			close.addActionListener(new ActionListener() {

				public void actionPerformed(ActionEvent e) {
					st.remove(ti);
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
			ti.setPopupMenu(p);
			try {
				st.add(ti);
			} catch (AWTException e1) {
				// TODO Auto-generated catch block
				// e1.printStackTrace();
				System.out.println(lang.failaddtoTray);
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
	
	public static void initFont(){
		environment = GraphicsEnvironment.getLocalGraphicsEnvironment();
		fonts = environment.getAvailableFontFamilyNames();// 获得系统字体
		Boolean findFont = false;
		for (int i = 0; i < fonts.length; i++) {
			// System.out.println(fonts[i]);
			if (fonts[i].equals(DefaultFontName)) {
				findFont = true;
			}
		}
		if (!findFont) {
			System.out.println("font can not find\n");
			if (DefaultFontName.equals("Microsoft YaHei UI")){
				DefaultFontName = "宋体";
			}
			else
				DefaultFontName = "Arial";
		}
		DefaultFont = new Font(DefaultFontName, Font.PLAIN, defaultFontsize);
		DefaultFontBig = new Font(DefaultFontName, Font.PLAIN, defaultFontsize + 2);
		DefaultFontBigBold = new Font(DefaultFontName, Font.BOLD, defaultFontsize + 4);
		DefaultFontsmall = new Font(DefaultFontName, Font.PLAIN, defaultFontsize - 2);
		environment = null;
//		fonts = null;
	}

	public static void getScreenSize(){
		ScreenWidth = Toolkit.getDefaultToolkit().getScreenSize().width;
		ScreenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
	}
	
	public static void setDebugLog(String path){
		PrintStream out = null;
		try {
			out = new PrintStream(path);
		} catch (FileNotFoundException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		System.setOut(out);
	}
	
	public static void setErrLog(String path){
		PrintStream out = null;
		try {
			out = new PrintStream(path);
		} catch (FileNotFoundException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		System.setErr(out);
	}
	public static void setUTF8(){
		System.setProperty("file.encoding","UTF-8");
		Field charset;
		try {
			charset = Charset.class.getDeclaredField("defaultCharset");

			charset.setAccessible(true);
			charset.set(null,null);
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
	public static void initWebLaf(){
		WebLookAndFeel.install();

		StyleConstants.textRenderingHints = new RenderingHints(RenderingHints.KEY_TEXT_ANTIALIASING,
				RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
		
		// WebLookAndFeel.set
		WebLookAndFeel.globalControlFont = DefaultFont;
		WebLookAndFeel.globalTooltipFont = DefaultFont;
		WebLookAndFeel.globalAlertFont = DefaultFont;
		WebLookAndFeel.globalMenuFont = DefaultFont;
		WebLookAndFeel.globalAcceleratorFont = DefaultFont;
		WebLookAndFeel.globalTitleFont = DefaultFont;
		WebLookAndFeel.globalTextFont = DefaultFont;
		WebLookAndFeel.setDecorateFrames(true);
		WebLookAndFeel.setDecorateAllWindows(true);

		WebLookAndFeel.setAllowLinuxTransparency(true);
	}
	public static void main(String[] args) {

		// set output stream
		setUTF8();

		setDebugLog("./output.log");
		setErrLog("./error.log");
		
		lang.initLang();
		appName = lang.appName;
		appTooltips = lang.appTooltips;
		httpHeader = lang.httpHeader;
		DefaultFontName = lang.lanuageConfig.getValue("defaultFontName");
		defaultFontsize = Integer.parseInt(lang.lanuageConfig.getValue("defaultFontSize"));

//		checkOS();;
		initFont();
		getScreenSize();

		// System.setProperty("awt.useSystemAAFontSettings", "on");

		initSystemTray();
		SwingUtilities.invokeLater(new Runnable() {
			public void run() {
				// Install WebLaF as application L&F
				initWebLaf();
				

				ctr = new controller();
			}
		});

	}


}