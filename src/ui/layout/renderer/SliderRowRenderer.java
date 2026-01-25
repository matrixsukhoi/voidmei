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
        // Get base default from row definition
        int defaultVal = 0;
        if (row.value != null) {
            try {
                defaultVal = row.getInt();
            } catch (Exception e) {
            }
        }

        // Priority for initial value:
        // 1. If property exists in GroupConfig, use PropertyBinder
        // 2. Otherwise try ConfigurationService
        // 3. Fallback to defaultVal (from row.value)
        int currentVal;
        if (row.property != null && PropertyBinder.hasField(groupConfig, row.property)) {
            currentVal = PropertyBinder.getInt(groupConfig, row.property, defaultVal);
        } else if (row.property != null) {
            String val = context.getStringFromConfigService(row.property, Integer.toString(defaultVal));
            try {
                currentVal = Integer.parseInt(val);
            } catch (Exception e) {
                currentVal = defaultVal;
            }
        } else {
            currentVal = defaultVal;
        }

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

        WebPanel itemPanel = ReplicaBuilder.createSliderItem(row.label, row.minVal, row.maxVal, currentVal, 150,
                row.desc,
                row.descImg);
        WebSlider slider = ReplicaBuilder.getSlider(itemPanel);

        if (slider != null) {
            final String prop = row.property;
            slider.addChangeListener(e -> {
                if (context.isUpdating())
                    return;
                if (!slider.getValueIsAdjusting()) {
                    int newVal = slider.getValue();
                    // Update memory model so it saves to ui_layout.cfg
                    row.value = newVal;

                    // Try property binding first
                    if (!PropertyBinder.setInt(groupConfig, prop, newVal)) {
                        // Fallback
                    }
                    // Always sync to ConfigurationService
                    if (prop != null) {
                        context.syncStringToConfigService(prop, Integer.toString(newVal));
                    }

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
