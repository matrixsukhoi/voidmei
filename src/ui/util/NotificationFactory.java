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
import com.alee.managers.notification.WebNotification;

import prog.app;
import prog.lang;

/**
 * Factory for creating WebNotification objects.
 * Separates UI creation logic from Business Logic (Controller).
 */
public class NotificationFactory {

    public static WebNotification createSimple(String text) {
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

    public static WebNotification createWithDuration(String text, int time) {
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

    public static WebNotification createError(String text) {
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

    public static WebNotification createTimer(long time, String pattern) {
        WebNotification a = new WebNotification();
        a.setFont(app.defaultFont);
        a.setIcon(NotificationIcon.clock.getIcon());
        a.setWindowOpacity((float) (0.5));
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

    public static WebNotification createAbout(String text, int time) {
        WebNotification a = new WebNotification();
        WebLabel text1 = new WebLabel(text);
        text1.setFont(new Font(app.defaultFontName, Font.PLAIN, 14));
        Image I = Toolkit.getDefaultToolkit().createImage("image/fubuki.jpg");
        ImageIcon A = new ImageIcon(I);
        a.setFont(app.defaultFont);
        a.setIcon(A);
        a.add(text1);
        a.setDisplayTime(time);
        a.setFocusable(false);
        return a;
    }
}
