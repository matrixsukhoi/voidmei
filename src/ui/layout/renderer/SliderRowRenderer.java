package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;
import com.alee.laf.spinner.WebSpinner;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import ui.replica.ReplicaBuilder;

/**
 * Renders SLIDER type rows with linked Spinner for numeric input.
 * Layout: [Label] ... [Slider] [Spinner] [Unit]
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
                // 使用默认值 0
            }
        }

        // 使用统一的配置读取助手
        // 优先级: PropertyBinder → ConfigurationService → 默认值
        int currentVal = RendererConfigHelper.readInt(context, groupConfig, row, defaultVal);

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

        // Pass unit to createSliderItem for Spinner + Unit label display
        WebPanel itemPanel = ReplicaBuilder.createSliderItem(row.label, row.minVal, row.maxVal, currentVal, 150,
                row.desc, row.descImg, row.unit);
        WebSlider slider = ReplicaBuilder.getSlider(itemPanel);
        WebSpinner spinner = ReplicaBuilder.getSpinner(itemPanel);

        // 统一的持久化逻辑
        final String prop = row.property;
        Runnable persistValue = () -> {
            if (context.isUpdating()) return;
            int newVal = slider.getValue();
            // Update memory model so it saves to ui_layout.cfg
            row.value = newVal;

            // 使用统一的配置写入助手
            RendererConfigHelper.writeInt(context, groupConfig, prop, newVal);

            if ("panelColumns".equals(prop)) {
                context.onRebuild();
            }
            context.onSave();
        };

        if (slider != null) {
            slider.addChangeListener(e -> {
                // Only persist when drag ends (valueIsAdjusting == false)
                if (!slider.getValueIsAdjusting()) {
                    persistValue.run();
                }
            });
        }

        if (spinner != null) {
            // Auto-save on focus lost (no need to press Enter)
            javax.swing.JComponent editor = spinner.getEditor();
            if (editor instanceof javax.swing.JSpinner.NumberEditor) {
                javax.swing.JFormattedTextField textField =
                    ((javax.swing.JSpinner.NumberEditor) editor).getTextField();
                textField.addFocusListener(new java.awt.event.FocusAdapter() {
                    @Override
                    public void focusLost(java.awt.event.FocusEvent e) {
                        persistValue.run();
                    }
                });
            }

            // Existing ChangeListener for arrow buttons and Enter key
            spinner.addChangeListener(e -> {
                persistValue.run();
            });
        }

        return itemPanel;
    }
}
