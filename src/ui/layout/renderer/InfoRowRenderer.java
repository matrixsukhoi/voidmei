package ui.layout.renderer;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import ui.replica.PinkStyle;

import javax.swing.BorderFactory;
import javax.swing.JEditorPane;
import javax.swing.event.HyperlinkEvent;
import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Container;
import java.awt.Desktop;
import java.awt.Dimension;
import java.awt.Font;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Renders INFO type rows as read-only text with rich text support.
 * Used for displaying static information text, such as welcome messages or instructions.
 *
 * Features:
 * - Auto-wrapping based on panel width
 * - Text selection and copying support
 * - Automatic URL detection and clickable hyperlinks
 */
public class InfoRowRenderer implements RowRenderer {

    private static final Color LABEL_COLOR = new Color(80, 80, 80);
    private static final Color VALUE_COLOR = new Color(60, 60, 60);
    /** preferred 高度计算用的固定 wrap 宽(保守值: 实际卡片更宽时留白不裁字) */
    private static final int INFO_WRAP_WIDTH = 560;

    /**
     * URL pattern for automatic hyperlink detection.
     * Matches http:// and https:// URLs with common URL characters.
     */
    private static final Pattern URL_PATTERN = Pattern.compile(
        "(https?://[\\w\\-._~:/?#\\[\\]@!$&'()*+,;=%]+)"
    );

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

        // Build HTML content with auto-wrapping and hyperlink support
        String html = buildHtmlContent(text);

        JEditorPane editorPane = new JEditorPane("text/html", html) {
            @Override
            public Dimension getPreferredSize() {
                // i18n 修复: HTMLEditorKit 的默认 preferred 宽 = 整段文本单行排开的全宽
                // (诊断实测欢迎页长句 1620/1188/1110px), 会把卡片→页面→窗口一路撑爆。
                // 父容器宽感知方案无效(父宽本身就是被子撑出的, 循环未切断)。
                // 彻底切断: 宽度恒 0 不参与容器协商; 高度按固定 wrap 宽计算,
                // 取保守值(实际卡片更宽时留白, 绝不裁字)
                setSize(INFO_WRAP_WIDTH, Integer.MAX_VALUE);
                Dimension d = super.getPreferredSize();
                d.width = 0;
                return d;
            }
        };
        editorPane.setEditable(false);
        editorPane.setOpaque(false);
        editorPane.setBackground(new Color(0, 0, 0, 0));
        editorPane.setBorder(null);

        // Hyperlink click handling - opens URLs in default browser
        editorPane.addHyperlinkListener(e -> {
            if (e.getEventType() == HyperlinkEvent.EventType.ACTIVATED) {
                try {
                    Desktop.getDesktop().browse(e.getURL().toURI());
                } catch (Exception ex) {
                    prog.util.Logger.warn("InfoRowRenderer", "Failed to open link: " + ex.getMessage());
                }
            }
        });

        panel.add(editorPane, BorderLayout.CENTER);

        return panel;
    }

    /**
     * Builds HTML content with proper styling and automatic URL detection.
     *
     * @param text Raw text content (may contain URLs)
     * @return HTML string ready for JEditorPane rendering
     */
    private String buildHtmlContent(String text) {
        // 1. Escape HTML special characters to prevent injection
        text = escapeHtml(text);

        // 2. Auto-detect URLs and convert to clickable hyperlinks
        Matcher matcher = URL_PATTERN.matcher(text);
        StringBuffer sb = new StringBuffer();
        while (matcher.find()) {
            String url = matcher.group(1);
            matcher.appendReplacement(sb, "<a href=\"" + url + "\">" + url + "</a>");
        }
        matcher.appendTail(sb);
        text = sb.toString();

        // 3. Wrap in HTML with inline CSS styling
        Font font = PinkStyle.FONT_NORMAL;
        String fontFamily = font.getFamily();
        int fontSize = font.getSize();
        String color = String.format("#%02x%02x%02x",
            VALUE_COLOR.getRed(), VALUE_COLOR.getGreen(), VALUE_COLOR.getBlue());

        return "<html><body style='font-family:" + fontFamily +
               ";font-size:" + fontSize + "pt;color:" + color +
               ";margin:0;padding:0;'>" + text + "</body></html>";
    }

    /**
     * Escapes HTML special characters to prevent XSS and rendering issues.
     */
    private String escapeHtml(String text) {
        return text.replace("&", "&amp;")
                   .replace("<", "&lt;")
                   .replace(">", "&gt;")
                   .replace("\"", "&quot;");
    }
}
