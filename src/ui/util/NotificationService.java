package ui.util;

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

import prog.app;
import prog.lang;

/**
 * Centralized notification service for showing WebNotification popups.
 * Extracted from controller.java for better separation of concerns.
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
        NotificationManager.showNotification(createCountdownNotification(time, lang.cEnginedmg));
    }

    /**
     * Show an error-style notification (e.g., engine bomb).
     */
    public static void showError(String text) {
        NotificationManager.showNotification(createErrorNotification(text));
    }

    // --- Internal factory methods ---

    private static WebNotification createNotification(String text) {
        WebNotification a = new WebNotification();
        WebLabel text1 = new WebLabel(text);
        text1.setFont(app.defaultFont);
        a.setFont(app.defaultFont);
        a.setIcon(NotificationIcon.information.getIcon());
        a.add(text1);
        a.setDisplayTime(5000);
        a.setFocusable(false);
        return a;
    }

    private static WebNotification createTimedNotification(String text, int time) {
        WebNotification a = new WebNotification();
        WebLabel text1 = new WebLabel(text);
        text1.setFont(app.defaultFont);
        a.setFont(app.defaultFont);
        a.setIcon(NotificationIcon.information.getIcon());
        a.add(text1);
        a.setDisplayTime(time);
        a.setFocusable(false);
        return a;
    }

    private static WebNotification createAboutNotification(String text, int time) {
        WebNotification a = new WebNotification();
        WebLabel text1 = new WebLabel(text);
        text1.setFont(new Font(app.defaultFontName, Font.PLAIN, 14));
        Image I = Toolkit.getDefaultToolkit().createImage("image/fubuki.jpg");
        ImageIcon icon = new ImageIcon(I);
        a.setFont(app.defaultFont);
        a.setIcon(icon);
        a.add(text1);
        a.setDisplayTime(time);
        a.setFocusable(false);
        return a;
    }

    private static WebNotification createCountdownNotification(long time, String pattern) {
        WebNotification a = new WebNotification();
        a.setFont(app.defaultFont);
        a.setIcon(NotificationIcon.clock.getIcon());
        a.setWindowOpacity(0.5f);

        WebClock clock = new WebClock();
        clock.setClockType(ClockType.timer);
        clock.setTimeLeft(time);
        clock.setFont(app.defaultFont);
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
        text1.setFont(app.defaultFont);
        a.setFont(app.defaultFont);
        a.setIcon(NotificationIcon.error);
        a.add(text1);
        a.setDisplayTime(3000);
        a.setFocusable(false);
        return a;
    }
}
