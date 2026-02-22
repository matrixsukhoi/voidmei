package ui.layout.renderer;

import java.awt.Color;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.text.WebTextField;

import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.ColorHelper;
import ui.replica.ReplicaBuilder;

/**
 * Renders color configuration items with modern color picker.
 *
 * <p>Features:</p>
 * <ul>
 *   <li>Text field displays hex format (#RRGGBBAA)</li>
 *   <li>Input accepts both hex (#RRGGBB, #RRGGBBAA) and decimal (R, G, B, A) formats</li>
 *   <li>Clickable color swatch opens graphical color picker</li>
 *   <li>Configuration stored in decimal format for backward compatibility</li>
 * </ul>
 */
public class ColorRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        String key = row.property;

        // Read color from config (stored in decimal format)
        String colorText = context.getStringFromConfigService(key, "255, 255, 255, 255");
        Color initialColor = ColorHelper.parseColor(colorText, Color.WHITE);

        // Create panel with hex display format
        String hexDisplay = ColorHelper.toHexString(initialColor, true);
        WebPanel itemPanel = ReplicaBuilder.createColorField(row.label, hexDisplay, initialColor, row.desc, row.descImg);

        WebTextField field = ReplicaBuilder.getColorField(itemPanel);
        WebPanel swatch = ReplicaBuilder.getColorSwatch(itemPanel);
        WebLabel dropdown = ReplicaBuilder.getColorDropdown(itemPanel);

        // Shared update logic
        Runnable updateFromColor = () -> {
            if (context.isUpdating()) return;

            Color c = ColorHelper.parseColor(field.getText(), initialColor);
            applyColorChange(c, field, swatch, key, row, context);
        };

        // Text field input handler (supports both hex and decimal input)
        if (field != null) {
            field.addActionListener(e -> updateFromColor.run());

            // Auto-apply on focus lost (no need to press Enter)
            field.addFocusListener(new java.awt.event.FocusAdapter() {
                @Override
                public void focusLost(java.awt.event.FocusEvent e) {
                    updateFromColor.run();
                }
            });
        }

        // Track active picker to prevent duplicate popups
        // Using array wrapper to allow modification from lambda
        final ColorPickerPopup[] activePicker = { null };

        // Color picker popup handler - creates popup anchored to the clicked component
        java.util.function.Consumer<javax.swing.JComponent> showColorPicker = (anchor) -> {
            if (context.isUpdating()) return;

            // If a picker is already open, close it first
            if (activePicker[0] != null) {
                activePicker[0].dispose();
                activePicker[0] = null;
            }

            // Parse current color from field
            Color current = ColorHelper.parseColor(field.getText(), Color.WHITE);

            // Show color picker popup anchored to the clicked component
            activePicker[0] = new ColorPickerPopup(anchor, current, selectedColor -> {
                applyColorChange(selectedColor, field, swatch, key, row, context);
                activePicker[0] = null;  // Clear reference after selection
            });
            activePicker[0].show();
        };

        // Bind click events to swatch and dropdown button
        if (swatch != null) {
            swatch.addMouseListener(new MouseAdapter() {
                @Override
                public void mouseClicked(MouseEvent e) {
                    showColorPicker.accept(swatch);
                }
            });
        }
        if (dropdown != null) {
            dropdown.addMouseListener(new MouseAdapter() {
                @Override
                public void mouseClicked(MouseEvent e) {
                    showColorPicker.accept(dropdown);
                }
            });
        }

        return itemPanel;
    }

    /**
     * Applies a color change to all UI components and persists to config.
     */
    private void applyColorChange(Color color, WebTextField field, WebPanel swatch,
                                  String key, RowConfig row, RenderContext context) {
        // Update swatch background
        if (swatch != null) {
            swatch.setBackground(color);
        }

        // Update text field with hex format
        if (field != null) {
            field.setText(ColorHelper.toHexString(color, true));
        }

        // Store in decimal format for backward compatibility
        String unified = ColorHelper.toDecimalString(color);
        context.syncStringToConfigService(key, unified);

        // Legacy sync for backward compatibility with split keys
        context.syncStringToConfigService(key + "R", Integer.toString(color.getRed()));
        context.syncStringToConfigService(key + "G", Integer.toString(color.getGreen()));
        context.syncStringToConfigService(key + "B", Integer.toString(color.getBlue()));
        context.syncStringToConfigService(key + "A", Integer.toString(color.getAlpha()));

        // Update memory model
        row.value = unified;

        context.onSave();
    }
}
