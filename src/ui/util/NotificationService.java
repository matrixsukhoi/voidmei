package ui.util;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Image;
import java.awt.Toolkit;

import javax.swing.BorderFactory;
import javax.swing.ImageIcon;
import javax.swing.JPanel;
import javax.swing.JWindow;
import javax.swing.SwingUtilities;
import javax.swing.Timer;

import com.alee.extended.panel.GroupPanel;
import com.alee.extended.time.ClockType;
import com.alee.extended.time.WebClock;
import com.alee.laf.label.WebLabel;
import com.alee.managers.notification.NotificationIcon;
import com.alee.managers.notification.NotificationManager;
import com.alee.managers.notification.WebNotification;

import prog.Application;
import prog.i18n.Lang;

/**
 * Centralized notification Service for showing WebNotification popups.
 * Extracted from Controller.java for better separation of concerns.
 */
public class NotificationService {

    /**
     * Show a simple notification with default 5 second display time.
     */
    public static void show(String text) {
        NotificationManager.showNotification(createNotification(text));
    }

    /**
     * Show a notification with custom display time.
     */
    public static void showTimed(String text, int time) {
        NotificationManager.showNotification(createTimedNotification(text, time));
    }

    /**
     * Show an "about" style notification with custom icon.
     */
    public static void showAbout(String text, int time) {
        NotificationManager.showNotification(createAboutNotification(text, time));
    }

    /**
     * Show a countdown timer notification.
     */
    public static void showCountdown(long time, String pattern) {
        NotificationManager.showNotification(createCountdownNotification(time, pattern));
    }

    /**
     * Show an engine damage countdown notification.
     */
    public static void showEngineDamageCountdown(long time) {
        NotificationManager.showNotification(createCountdownNotification(time, Lang.cEnginedmg));
    }

    /**
     * Show an error-style notification (e.g., engine bomb).
     */
    public static void showError(String text) {
        NotificationManager.showNotification(createErrorNotification(text));
    }

    /**
     * 在屏幕右下角显示一条自绘 toast，displayMs 后自动销毁。
     *
     * <p>刻意不用 WebLaF NotificationManager —— 其显示位置是全局设置，
     * 会牵连引擎倒计时等既有通知的位置；此处自绘 JWindow 完全独立。
     * 线程安全：内部派发 EDT，任意线程可直接调用。
     */
    public static void showBottomRight(String text, int displayMs) {
        SwingUtilities.invokeLater(() -> {
            JPanel panel = new JPanel(new BorderLayout());
            panel.setBackground(new Color(20, 20, 20, 220));
            WebLabel label = new WebLabel(text);
            label.setForeground(Color.WHITE);
            label.setFont(Application.defaultFont);
            label.setBorder(BorderFactory.createEmptyBorder(12, 16, 12, 16));
            panel.add(label, BorderLayout.CENTER);

            JWindow toast = new JWindow();
            toast.setContentPane(panel);
            toast.setAlwaysOnTop(true);
            toast.setFocusable(false);
            // 右下角定位（逻辑屏幕尺寸, 与 MainForm 定位惯例一致; 留 16px 边距）
            Dimension pref = panel.getPreferredSize();
            toast.setBounds(Application.logicalWidth - pref.width - 16,
                    Application.logicalHeight - pref.height - 16, pref.width, pref.height);
            toast.setVisible(true);
            // 到时自动销毁（单次 Timer, dispose 释放原生资源）
            Timer timer = new Timer(displayMs, e -> {
                toast.setVisible(false);
                toast.dispose();
            });
            timer.setRepeats(false);
            timer.start();
        });
    }

    // --- Internal factory methods ---

    private static WebNotification createNotification(String text) {
        WebNotification a = new WebNotification();
        WebLabel text1 = new WebLabel(text);
        text1.setFont(Application.defaultFont);
        a.setFont(Application.defaultFont);
        a.setIcon(NotificationIcon.information.getIcon());
        a.add(text1);
        a.setDisplayTime(5000);
        a.setFocusable(false);
        return a;
    }

    private static WebNotification createTimedNotification(String text, int time) {
        WebNotification a = new WebNotification();
        WebLabel text1 = new WebLabel(text);
        text1.setFont(Application.defaultFont);
        a.setFont(Application.defaultFont);
        a.setIcon(NotificationIcon.information.getIcon());
        a.add(text1);
        a.setDisplayTime(time);
        a.setFocusable(false);
        return a;
    }

    private static WebNotification createAboutNotification(String text, int time) {
        WebNotification a = new WebNotification();
        WebLabel text1 = new WebLabel(text);
        text1.setFont(new Font(Application.defaultFontName, Font.PLAIN, 14));
        Image I = Toolkit.getDefaultToolkit().createImage("image/fubuki.jpg");
        ImageIcon icon = new ImageIcon(I);
        a.setFont(Application.defaultFont);
        a.setIcon(icon);
        a.add(text1);
        a.setDisplayTime(time);
        a.setFocusable(false);
        return a;
    }

    private static WebNotification createCountdownNotification(long time, String pattern) {
        WebNotification a = new WebNotification();
        a.setFont(Application.defaultFont);
        a.setIcon(NotificationIcon.clock.getIcon());
        a.setWindowOpacity(0.5f);

        WebClock clock = new WebClock();
        clock.setClockType(ClockType.timer);
        clock.setTimeLeft(time);
        clock.setFont(Application.defaultFont);
        clock.setTimePattern(pattern);
        a.setContent(new GroupPanel(clock));
        clock.start();

        a.setDisplayTime(time);
        a.setFocusable(false);
        return a;
    }

    private static WebNotification createErrorNotification(String text) {
        WebNotification a = new WebNotification();
        WebLabel text1 = new WebLabel(text);
        text1.setFont(Application.defaultFont);
        a.setFont(Application.defaultFont);
        a.setIcon(NotificationIcon.error);
        a.add(text1);
        a.setDisplayTime(3000);
        a.setFocusable(false);
        return a;
    }
}
