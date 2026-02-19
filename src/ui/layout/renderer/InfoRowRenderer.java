package ui.layout.renderer;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import ui.replica.PinkStyle;

import javax.swing.BorderFactory;
import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Font;

/**
 * Renders INFO type rows as read-only text labels.
 * Used for displaying static information text, such as welcome messages or instructions.
 * Supports multiline text via \n in the value.
 */
public class InfoRowRenderer implements RowRenderer {

    private static final Color LABEL_COLOR = new Color(80, 80, 80);
    private static final Color VALUE_COLOR = new Color(60, 60, 60);

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        WebPanel panel = new WebPanel(new BorderLayout(8, 0));
        panel.setOpaque(false);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        // Label (left side) - optional, used for step numbers or categories
        if (row.label != null && !row.label.isEmpty()) {
            WebLabel labelComp = new WebLabel(row.label);
            labelComp.setForeground(LABEL_COLOR);
            labelComp.setFont(labelComp.getFont().deriveFont(Font.BOLD));
            panel.add(labelComp, BorderLayout.WEST);

            // Enable ResponsiveGrid alignment (consistent with other renderers)
            panel.putClientProperty("alignLabel", labelComp);
        }

        // Value text (right side or full width if no label)
        String text = row.value != null ? row.getStr() : "";
        // Convert \n to HTML line breaks for multiline support
        if (text.contains("\\n")) {
            text = "<html>" + text.replace("\\n", "<br>") + "</html>";
        }

        WebLabel valueLabel = new WebLabel(text);
        valueLabel.setForeground(VALUE_COLOR);
        valueLabel.setFont(PinkStyle.FONT_NORMAL);
        panel.add(valueLabel, BorderLayout.CENTER);

        return panel;
    }
}
