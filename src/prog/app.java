package prog;

import java.awt.AWTException;
import java.awt.Color;
import java.awt.Font;
import java.awt.GraphicsEnvironment;
import java.awt.Image;
import java.awt.MenuItem;
import java.awt.PopupMenu;
import java.awt.RenderingHints;
import java.awt.Robot;
import java.awt.SystemTray;
import java.awt.Toolkit;
import java.awt.TrayIcon;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.KeyEvent;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;

import javax.swing.SwingUtilities;

import com.alee.global.StyleConstants;
import com.alee.laf.WebLookAndFeel;

public class app {
	public static boolean debug = true;
	public static boolean ForeignLanguage = false;
	public static String appName = language.appName;
	public static String DefaultNumfontName = "Segoe UI";
	public static String appTooltips = language.appTooltips;
	public static String version = "1.11";
	public static String httpHeader = language.httpHeader;
	public static String DefaultFontName = "yahei consolas";
	public static Font DefaultFont = new Font(DefaultFontName, Font.PLAIN, 12);
	public static Font DefaultFontBig = new Font(DefaultFontName, Font.PLAIN, 14);
	public static Font DefaultFontBigBold = new Font(DefaultFontName, Font.BOLD, 16);
	public static Font DefaultFontsmall = new Font(DefaultFontName, Font.PLAIN, 10);
	public static Process plugin = null;
	public static Runtime r;
	public static Robot robot;
	// public static String NumfontPath = "fonts/DINPro-Regular.otf";
	// public static String NumfontPath2="fonts/DINCond-Medium.otf";
	public static GraphicsEnvironment environment;
	public static int ScreenWidth;
	public static int ScreenHeight;
	public static String[] fonts;
	public static Color WhiteColor = new Color(245, 248, 250, 240);

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
			tasklist1 = out.toString().indexOf("TaskBarHider.exe");// ������
		} catch (Exception e1) {
			e1.printStackTrace();
		}
		if (tasklist1 == -1) {
			// �����ڽ�����û�з���
			try {
				plugin = r.exec(System.getProperty("user.dir") + "\\TaskBarHider.exe");
			} catch (IOException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
		} else {
			System.out.println("TaskBarHider�����Ѿ���!");
		}
	
		// robot.keyPress(17);
		// robot.keyPress(192);
		// robot.keyRelease(17);
		// robot.keyRelease(192);
	}

	public void pluginoff() {
		plugin.destroy();
	}

	public void systemTray() {
		if (SystemTray.isSupported()) {
			SystemTray st = SystemTray.getSystemTray();
			Image image = Toolkit.getDefaultToolkit().getImage("image/16x16.png");
			TrayIcon ti = new TrayIcon(image);
			ti.setToolTip(appName);
			PopupMenu p = new PopupMenu("");
			MenuItem close = new MenuItem(language.close);
			MenuItem about = new MenuItem(language.about);
			close.addActionListener(new ActionListener() {

				public void actionPerformed(ActionEvent e) {
					st.remove(ti);
					System.exit(0);
				}
			});
			about.addActionListener(new ActionListener() {

				public void actionPerformed(ActionEvent e) {

					controller.notificationtimeAbout(language.aboutcontentsub2, 24000);
					controller.notificationtimeAbout(language.aboutcontentsub1, 16000);
					controller.notificationtimeAbout(language.aboutcontent, 8000);

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
				System.out.println(language.failaddtoTray);
			}
		}
	}

	/*
	 * boolean checkfonts(){ int i=0; int flag=0; for(i=0;i<fonts.length;i++){
	 * if(fonts[i].equals("DINPro-Regular"))flag++;
	 * if(fonts[i].equals("DINCond-Medium"))flag++; } if(flag==2)return true;
	 * else return false; }
	 */
	public static void main(String[] args) {
		if (ForeignLanguage) {
			DefaultFontName = DefaultNumfontName;
			language.initEng();
			appName = language.appName;
			appTooltips = language.appTooltips;
			httpHeader = language.httpHeader;
		}
//		if (Float.parseFloat(System.getProperty("os.version")) < 6.0) {
//			controller.notificationtime(language.Systemerror, 10000);
//			try {
//				Thread.sleep(10000);
//			} catch (InterruptedException e) {
//				// TODO Auto-generated catch block
//				e.printStackTrace();
//			}
//			// System.exit(0);
//		}
		environment = GraphicsEnvironment.getLocalGraphicsEnvironment();
		fonts = environment.getAvailableFontFamilyNames();// ���ϵͳ����
		// System.out.println(fonts.length);
		// DefaultFont=new Font("Microsoft YaHei", Font.BOLD, 12);

		ScreenWidth = Toolkit.getDefaultToolkit().getScreenSize().width;
		ScreenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
		// System.out.println(ScreenWidth);
		System.setProperty("awt.useSystemAAFontSettings", "on");

		SwingUtilities.invokeLater(new Runnable() {
			public void run() {
				// System.out.println("appִ����");
				// Install WebLaF as application L&F

				WebLookAndFeel.install();

				StyleConstants.textRenderingHints = new RenderingHints(RenderingHints.KEY_TEXT_ANTIALIASING,
						RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
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
				if(!debug)controller.notificationtime("���л�����Ϸ�����޷��������������밴CTRL+~����������",10000);
				// if(A.checkfonts()==false)controller.notificationtime("Ϊ��֤Ӣ�ġ����ֵı�ʶ�ȣ���������װfontsĿ¼�µĵ¹���ҵ��׼1451ϵ������DINPro-Regular��DINCond-Medium",10000);
				app A = new app();
				A.systemTray();
				try {
					robot = new Robot();
				} catch (AWTException e) {
					// TODO Auto-generated catch block
					e.printStackTrace();
				}

				//A.pluginopen();
				controller ctrl = new controller();

				// System.out.println(System.getProperty("user.dir"));
				ctrl.init();

			}
		});

	}
}