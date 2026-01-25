package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.text.WebTextField;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.PropertyBinder;
import ui.replica.ReplicaBuilder;

import java.awt.event.FocusAdapter;
import java.awt.event.FocusEvent;

/**
 * Renders TEXT or INPUT type rows as a text field.
 */
public class TextRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        String defaultVal = (row.value != null) ? row.getStr() : "";

        // Get current value
        String currentVal;
        if (row.property != null && PropertyBinder.hasField(groupConfig, row.property)) {
            currentVal = PropertyBinder.getString(groupConfig, row.property, defaultVal);
        } else if (row.property != null) {
            currentVal = context.getStringFromConfigService(row.property, defaultVal);
        } else {
            currentVal = defaultVal;
        }

        // Create the UI component
        WebPanel itemPanel = ReplicaBuilder.createTextItem(row.label, currentVal, 10, row.desc);
        WebTextField textField = ReplicaBuilder.getTextField(itemPanel);

        if (textField != null) {
            // Update on focus loss or enter key
            textField.addActionListener(e -> {
                updateValue(textField.getText(), row, groupConfig, context);
            });

            textField.addFocusListener(new FocusAdapter() {
                @Override
                public void focusLost(FocusEvent e) {
                    updateValue(textField.getText(), row, groupConfig, context);
                }
            });
        }

        return itemPanel;
    }

    private void updateValue(String newVal, RowConfig row, GroupConfig groupConfig, RenderContext context) {
        if (context.isUpdating())
            return;

        final String prop = row.property;

        // Update memory model
        row.value = newVal;

        // Sync with property binder
        PropertyBinder.setString(groupConfig, prop, newVal);

        // Sync to ConfigurationService
        if (prop != null) {
            context.syncStringToConfigService(prop, newVal);
        }

        context.onSave();
    }
}
