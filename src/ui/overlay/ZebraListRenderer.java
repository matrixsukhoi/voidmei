package ui.overlay;

import java.awt.Color;
import java.util.List;
import java.util.function.Predicate;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

/**
 * Zebra-stripe list renderer with header highlighting.
 */
public class ZebraListRenderer implements OverlayRenderer {

    // Default header detection
    private Predicate<String> headerMatcher = line -> line.contains("fm器件") || line.contains("FM文件");

    @Override
    public void render(List<String> data, WebPanel panel, java.awt.Font font, int alpha) {
        panel.removeAll();

        int rowIndex = 0;
        for (int i = 0; i < data.size(); i++) {
            String line = data.get(i);
            WebLabel label = new WebLabel(line);
            label.setFont(font);
            label.setForeground(Color.WHITE);
            label.setOpaque(true);
            label.setMargin(2, 6, 2, 6);

            if (isHeader(line)) {
                // Header: Deep amber
                label.setBackground(new Color(80, 60, 0, alpha));
            } else {
                // Zebra stripe
                if (rowIndex % 2 == 0) {
                    label.setBackground(new Color(25, 25, 25, alpha));
                } else {
                    label.setBackground(new Color(40, 40, 40, alpha));
                }
                rowIndex++;
            }
            panel.add(label);
        }
        panel.revalidate();
    }

    @Override
    public boolean isHeader(String line) {
        return headerMatcher.test(line);
    }

    @Override
    public void setHeaderMatcher(Predicate<String> matcher) {
        if (matcher != null) {
            this.headerMatcher = matcher;
        }
    }
}
