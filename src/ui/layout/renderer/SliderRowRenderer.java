package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.PropertyBinder;
import ui.replica.ReplicaBuilder;

/**
 * Renders SLIDER type rows.
 */
public class SliderRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        // Get current value from GroupConfig property
        int defaultVal = 0;
        if (row.defaultValue != null) {
            try {
                defaultVal = Integer.parseInt(row.defaultValue);
            } catch (Exception e) {
            }
        }
        int currentVal = PropertyBinder.getInt(groupConfig, row.property, defaultVal);

        // Ensure min < max to avoid crash
        int min = row.minVal;
        int max = row.maxVal;
        if (min >= max) {
            max = min + 100; // Fallback
        }

        // Clamp value to range
        if (currentVal < min)
            currentVal = min;
        if (currentVal > max)
            currentVal = max;

        WebPanel itemPanel = ReplicaBuilder.createSliderItem(row.label, min, max, currentVal, 150);
        WebSlider slider = ReplicaBuilder.getSlider(itemPanel);

        if (slider != null) {
            final String prop = row.property;
            slider.addChangeListener(e -> {
                if (context.isUpdating())
                    return;
                if (!slider.getValueIsAdjusting()) {
                    PropertyBinder.setInt(groupConfig, prop, slider.getValue());
                    if ("panelColumns".equals(prop)) {
                        context.onRebuild();
                    }
                    context.onSave();
                }
            });
        }

        return itemPanel;
    }
}
