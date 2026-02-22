package ui.util;

import java.awt.Color;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionListener;

import com.alee.laf.slider.WebSlider;

/**
 * Utility class for configuring read-only display sliders.
 *
 * <p>Consolidates the ~25-line initslider() pattern repeated in:
 * - GearFlapsOverlay.java
 * - AttitudeOverlay.java
 * - ControlSurfacesOverlay.java
 *
 * <p>These sliders are used for visual display only (not user input),
 * so mouse listeners are removed and thumb is hidden.
 */
public final class SliderHelper {

    private static final Color TRANSPARENT = new Color(0, 0, 0, 0);

    private SliderHelper() {
        // Prevent instantiation
    }

    /**
     * Configure a WebSlider as a read-only vertical progress bar.
     *
     * Used for gear/flaps indicators where the slider shows a value
     * but should not be interactable.
     *
     * @param slider The slider to configure
     * @param min Minimum value
     * @param max Maximum value
     * @param progressTopColor Top gradient color for progress track
     * @param progressBottomColor Bottom gradient color for progress track
     */
    public static void configureVerticalProgress(WebSlider slider, int min, int max,
            Color progressTopColor, Color progressBottomColor) {
        slider.setMinimum(min);
        slider.setMaximum(max);
        slider.setValue(0);
        slider.setDrawProgress(true);
        slider.setMinorTickSpacing(25);
        slider.setMajorTickSpacing(50);
        slider.setOrientation(javax.swing.SwingConstants.VERTICAL);
        slider.setPaintTicks(true);
        slider.setPaintLabels(false);
        slider.setDrawThumb(false);

        slider.setProgressRound(0);
        slider.setTrackRound(1);
        slider.setProgressShadeWidth(0);
        slider.setTrackShadeWidth(0);
        slider.setThumbShadeWidth(0);

        slider.setThumbBgBottom(TRANSPARENT);
        slider.setThumbBgTop(TRANSPARENT);
        slider.setTrackBgBottom(TRANSPARENT);
        slider.setTrackBgTop(TRANSPARENT);
        slider.setProgressBorderColor(TRANSPARENT);
        slider.setProgressTrackBgBottom(progressBottomColor);
        slider.setProgressTrackBgTop(progressTopColor);
        slider.setFocusable(false);

        removeAllListeners(slider);
    }

    /**
     * Configure a WebSlider as a read-only horizontal slider for attitude display.
     *
     * Used for attitude overlay where the thumb position indicates a value
     * but should not be interactable.
     *
     * @param slider The slider to configure
     * @param min Minimum value
     * @param max Maximum value
     * @param thumbColor Color for the thumb
     */
    public static void configureAttitudeSlider(WebSlider slider, int min, int max, Color thumbColor) {
        slider.setMinimum(min);
        slider.setMaximum(max);
        slider.setValue(0);
        slider.setDrawProgress(true);
        slider.setMinorTickSpacing(25);
        slider.setMajorTickSpacing(50);
        slider.setPaintTicks(true);
        slider.setPaintLabels(true);
        slider.setProgressShadeWidth(0);
        slider.setTrackShadeWidth(1);
        slider.setThumbShadeWidth(2);

        slider.setThumbBgBottom(thumbColor);
        slider.setThumbBgTop(thumbColor);
        slider.setTrackBgBottom(TRANSPARENT);
        slider.setTrackBgTop(TRANSPARENT);
        slider.setProgressBorderColor(TRANSPARENT);
        slider.setProgressTrackBgBottom(TRANSPARENT);
        slider.setProgressTrackBgTop(TRANSPARENT);
        slider.setFocusable(false);

        removeAllListeners(slider);
    }

    /**
     * Remove all mouse listeners from a slider to make it non-interactive.
     *
     * @param slider The slider to configure
     */
    public static void removeAllListeners(WebSlider slider) {
        for (MouseListener ml : slider.getMouseListeners()) {
            slider.removeMouseListener(ml);
        }
        for (MouseMotionListener mml : slider.getMouseMotionListeners()) {
            slider.removeMouseMotionListener(mml);
        }
    }
}
