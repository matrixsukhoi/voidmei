package ui.layout.renderer;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.FlowLayout;
import java.awt.event.FocusAdapter;
import java.awt.event.FocusEvent;

import javax.swing.BorderFactory;
import javax.swing.JColorChooser;

import com.alee.extended.window.WebPopOver;
import com.alee.laf.button.WebButton;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;
import com.alee.laf.text.WebTextField;

import prog.util.ColorHelper;
import ui.replica.PinkStyle;

/**
 * Modern color picker popup with HSB palette, alpha slider, and hex input.
 *
 * <p>Layout:</p>
 * <pre>
 * ┌─────────────────────────┐
 * │  ┌─────────────────┐    │
 * │  │   HSB Palette   │    │
 * │  │                 │    │
 * │  └─────────────────┘    │
 * │  Alpha: [━━━━●━━━] 255  │
 * │  Hex:   [#FFFFFFFF   ]  │
 * │  Preview: [████████]    │
 * │         [取消] [确定]   │
 * └─────────────────────────┘
 * </pre>
 */
public class ColorPickerPopup {

    /**
     * Callback interface for color selection.
     */
    public interface ColorSelectedCallback {
        void onColorSelected(Color color);
    }

    private final WebPopOver popover;
    private final javax.swing.JComponent ownerComponent;
    private final JColorChooser colorChooser;
    private final WebSlider alphaSlider;
    private final WebTextField hexField;
    private final WebPanel previewPanel;
    private final WebLabel alphaValueLabel;

    private Color currentColor;
    private boolean updatingUI = false;

    /**
     * Creates a color picker popup.
     *
     * @param owner        The component to attach the popup to
     * @param initialColor The initial color to display
     * @param callback     Called when user confirms color selection
     */
    public ColorPickerPopup(javax.swing.JComponent owner, Color initialColor, ColorSelectedCallback callback) {
        this.currentColor = initialColor != null ? initialColor : Color.WHITE;
        this.ownerComponent = owner;
        this.popover = new WebPopOver(owner);

        // Configure popover
        popover.setCloseOnFocusLoss(false);
        popover.setMovable(true);
        popover.setMargin(5);

        // Main container
        WebPanel mainPanel = new WebPanel(new BorderLayout(5, 5));
        mainPanel.setOpaque(true);
        mainPanel.setBackground(PinkStyle.COLOR_BG_PANEL);
        mainPanel.setBorder(BorderFactory.createCompoundBorder(
            BorderFactory.createLineBorder(PinkStyle.COLOR_PRIMARY, 1),
            BorderFactory.createEmptyBorder(10, 10, 10, 10)
        ));

        // Initialize alpha slider first (referenced by color chooser listener)
        alphaSlider = new WebSlider(WebSlider.HORIZONTAL, 0, 255, currentColor.getAlpha());

        // Color chooser (HSB palette)
        colorChooser = new JColorChooser(new Color(currentColor.getRGB() & 0x00FFFFFF));
        colorChooser.setPreviewPanel(new WebPanel()); // Hide default preview
        // Try to keep only the HSB panel for a cleaner look (if available)
        javax.swing.colorchooser.AbstractColorChooserPanel[] panels = colorChooser.getChooserPanels();
        if (panels != null && panels.length > 0) {
            // Find and keep only the HSB panel
            for (javax.swing.colorchooser.AbstractColorChooserPanel panel : panels) {
                String name = panel.getDisplayName();
                if (name != null && (name.contains("HSV") || name.contains("HSB") || name.contains("HSL"))) {
                    colorChooser.setChooserPanels(new javax.swing.colorchooser.AbstractColorChooserPanel[] { panel });
                    break;
                }
            }
        }
        // Let the color chooser auto-size to fit its content
        colorChooser.getSelectionModel().addChangeListener(e -> {
            if (!updatingUI) {
                Color rgb = colorChooser.getColor();
                currentColor = new Color(rgb.getRed(), rgb.getGreen(), rgb.getBlue(), alphaSlider.getValue());
                updateUIFromColor();
            }
        });
        mainPanel.add(colorChooser, BorderLayout.CENTER);

        // Bottom controls panel
        WebPanel controlsPanel = new WebPanel(new BorderLayout(5, 5));
        controlsPanel.setOpaque(false);

        // Alpha slider row
        WebPanel alphaRow = new WebPanel(new BorderLayout(5, 0));
        alphaRow.setOpaque(false);

        WebLabel alphaLabel = new WebLabel("Alpha:");
        alphaLabel.setFont(PinkStyle.FONT_NORMAL);
        alphaLabel.setForeground(PinkStyle.COLOR_TEXT);
        alphaRow.add(alphaLabel, BorderLayout.WEST);
        alphaSlider.setPreferredSize(new Dimension(150, 30));
        alphaSlider.setOpaque(false);
        alphaSlider.addChangeListener(e -> {
            if (!updatingUI) {
                currentColor = new Color(
                    currentColor.getRed(),
                    currentColor.getGreen(),
                    currentColor.getBlue(),
                    alphaSlider.getValue()
                );
                updateUIFromColor();
            }
        });
        alphaRow.add(alphaSlider, BorderLayout.CENTER);

        alphaValueLabel = new WebLabel(String.valueOf(currentColor.getAlpha()));
        alphaValueLabel.setFont(PinkStyle.FONT_NORMAL);
        alphaValueLabel.setForeground(PinkStyle.COLOR_TEXT);
        alphaValueLabel.setPreferredSize(new Dimension(30, 20));
        alphaRow.add(alphaValueLabel, BorderLayout.EAST);

        controlsPanel.add(alphaRow, BorderLayout.NORTH);

        // Hex input row
        WebPanel hexRow = new WebPanel(new BorderLayout(5, 0));
        hexRow.setOpaque(false);
        hexRow.setBorder(BorderFactory.createEmptyBorder(5, 0, 5, 0));

        WebLabel hexLabel = new WebLabel("Hex:");
        hexLabel.setFont(PinkStyle.FONT_NORMAL);
        hexLabel.setForeground(PinkStyle.COLOR_TEXT);
        hexRow.add(hexLabel, BorderLayout.WEST);

        hexField = new WebTextField(ColorHelper.toHexString(currentColor, true), 10);
        hexField.setFont(PinkStyle.FONT_NORMAL);
        hexField.addActionListener(e -> parseHexInput());
        hexField.addFocusListener(new FocusAdapter() {
            @Override
            public void focusLost(FocusEvent e) {
                parseHexInput();
            }
        });
        hexRow.add(hexField, BorderLayout.CENTER);

        // Preview panel
        previewPanel = new WebPanel();
        previewPanel.setPreferredSize(new Dimension(40, 24));
        previewPanel.setBackground(currentColor);
        previewPanel.setBorder(BorderFactory.createLineBorder(Color.GRAY, 1));
        hexRow.add(previewPanel, BorderLayout.EAST);

        controlsPanel.add(hexRow, BorderLayout.CENTER);

        // Button row
        WebPanel buttonRow = new WebPanel(new FlowLayout(FlowLayout.RIGHT, 5, 0));
        buttonRow.setOpaque(false);

        WebButton cancelButton = new WebButton("取消");
        cancelButton.setFont(PinkStyle.FONT_NORMAL);
        cancelButton.addActionListener(e -> popover.dispose());
        buttonRow.add(cancelButton);

        WebButton confirmButton = new WebButton("确定");
        confirmButton.setFont(PinkStyle.FONT_NORMAL);
        confirmButton.addActionListener(e -> {
            if (callback != null) {
                callback.onColorSelected(currentColor);
            }
            popover.dispose();
        });
        buttonRow.add(confirmButton);

        controlsPanel.add(buttonRow, BorderLayout.SOUTH);

        mainPanel.add(controlsPanel, BorderLayout.SOUTH);

        popover.add(mainPanel);
    }

    /**
     * Shows the popup below the owner component.
     */
    public void show() {
        // Position below the owner, aligned to left edge
        popover.show(ownerComponent, 0, ownerComponent.getHeight() + 5);

        // Bring to front
        java.awt.Window win = javax.swing.SwingUtilities.getWindowAncestor(popover);
        if (win != null) {
            win.toFront();
        }
    }

    /**
     * Shows the popup at the specified location relative to owner.
     */
    public void show(int x, int y) {
        popover.show(ownerComponent, x, y);

        // Bring to front
        java.awt.Window win = javax.swing.SwingUtilities.getWindowAncestor(popover);
        if (win != null) {
            win.toFront();
        }
    }

    private void parseHexInput() {
        if (updatingUI) return;

        String text = hexField.getText().trim();
        Color parsed = ColorHelper.parseColor(text, currentColor);
        if (!parsed.equals(currentColor)) {
            currentColor = parsed;
            updateUIFromColor();
        }
    }

    private void updateUIFromColor() {
        updatingUI = true;
        try {
            // Update color chooser (RGB only, alpha handled separately)
            Color rgb = new Color(currentColor.getRed(), currentColor.getGreen(), currentColor.getBlue());
            if (!colorChooser.getColor().equals(rgb)) {
                colorChooser.setColor(rgb);
            }

            // Update alpha slider
            alphaSlider.setValue(currentColor.getAlpha());
            alphaValueLabel.setText(String.valueOf(currentColor.getAlpha()));

            // Update hex field
            hexField.setText(ColorHelper.toHexString(currentColor, true));

            // Update preview
            previewPanel.setBackground(currentColor);
        } finally {
            updatingUI = false;
        }
    }

    /**
     * Disposes the popup.
     */
    public void dispose() {
        popover.dispose();
    }
}
