package ui.layout;

import java.awt.Container;
import javax.swing.JComponent;
import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

/**
 * Interface for defining different UI styles (e.g., Classic, Modern/Card).
 */
public interface UIStyle {

    /**
     * Creates a container panel for a group of settings.
     * 
     * @param title The title of the group (if applicable).
     * @return A configured WebPanel.
     */
    WebPanel createContainer(String title);

    /**
     * Decorates a WebPanel to act as a wrapper for a single control (e.g., label +
     * switch).
     * 
     * @param panel The panel to decorate.
     */
    void decorateControlPanel(WebPanel panel);

    /**
     * Decorates a label.
     * 
     * @param label The label to decorate.
     */
    void decorateLabel(WebLabel label);

    /**
     * Decorates a switch to match the specific style.
     * 
     * @param webSwitch The switch to decorate.
     */
    void decorateSwitch(WebSwitch webSwitch);

    /**
     * Decorates a slider.
     * 
     * @param slider The slider to decorate.
     */
    void decorateSlider(WebSlider slider);

    /**
     * Decorates the main content panel (e.g. background color).
     * 
     * @param panel The main container panel.
     */
    void decorateMainPanel(WebPanel panel);
}
