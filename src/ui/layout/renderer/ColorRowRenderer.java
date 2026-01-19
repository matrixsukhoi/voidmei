package ui.layout.renderer;

import java.awt.Color;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.text.WebTextField;
import com.alee.extended.image.WebImage;
import com.alee.utils.ImageUtils;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import ui.replica.ReplicaBuilder;

public class ColorRowRenderer implements RowRenderer {
    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        // Read color components
        String key = row.property;
        // ConfigurationService stores split keys (key + "R", etc.) for colors
        String r = context.getStringFromConfigService(key + "R", "255");
        String g = context.getStringFromConfigService(key + "G", "255");
        String b = context.getStringFromConfigService(key + "B", "255");
        String a = context.getStringFromConfigService(key + "A", "255");

        String colorText = r + ", " + g + ", " + b + ", " + a;
        Color initialColor = parseColor(colorText);

        WebPanel itemPanel = ReplicaBuilder.createColorField(row.label, colorText, initialColor);
        WebTextField field = ReplicaBuilder.getColorField(itemPanel);

        if (field != null) {
            field.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                Color c = parseColor(field.getText());
                // Update icon
                field.setLeadingComponent(new WebImage(ImageUtils.createColorIcon(c)));

                // Sync components
                context.syncStringToConfigService(key + "R", Integer.toString(c.getRed()));
                context.syncStringToConfigService(key + "G", Integer.toString(c.getGreen()));
                context.syncStringToConfigService(key + "B", Integer.toString(c.getBlue()));
                context.syncStringToConfigService(key + "A", Integer.toString(c.getAlpha()));

                // Update memory model
                row.value = c.getRed() + ", " + c.getGreen() + ", " + c.getBlue() + ", " + c.getAlpha();

                context.onSave();
            });
        }

        return itemPanel;
    }

    private Color parseColor(String t) {
        if (t == null || t.isEmpty())
            return Color.WHITE;
        t = t.replaceAll(" ", "");
        String[] ts = t.split(",");
        if (ts.length < 3)
            return Color.WHITE; // Accept RGB or RGBA
        try {
            int R = Integer.parseInt(ts[0]);
            int G = Integer.parseInt(ts[1]);
            int B = Integer.parseInt(ts[2]);
            int A = (ts.length > 3) ? Integer.parseInt(ts[3]) : 255;
            return new Color(R, G, B, A);
        } catch (Exception e) {
            return Color.WHITE;
        }
    }
}
