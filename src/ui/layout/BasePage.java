package ui.layout;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;

import com.alee.extended.layout.VerticalFlowLayout;
import com.alee.extended.panel.WebButtonGroup;
import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.scroll.WebScrollPane;

import ui.mainform;
import prog.lang;

/**
 * Base abstract class for ALL UI content pages.
 * Encapsulates standard styling (separator line), layout structure
 * (Scroll/Content),
 * and the standard Bottom Control Panel.
 * 
 * New pages should extend this and implement initContent().
 */
public abstract class BasePage extends WebPanel {

    protected mainform parent;
    protected boolean isDetailedMode = false;

    public BasePage(mainform parent) {
        super();
        this.parent = parent;
        this.setLayout(new BorderLayout());
        this.setOpaque(false);
        this.setBackground(new Color(0, 0, 0, 0));

        // Essential settings for WebPanel to respect custom shading/borders
        this.setWebColoredBackground(false);
        this.setUndecorated(false);

        // --- Standard Separator Styling ---
        // Mimics the legacy initJP code to create the seamless vertical separator line
        this.setShadeWidth(2);
        this.setRound(com.alee.global.StyleConstants.largeRound);
        this.setBorderColor(new java.awt.Color(0, 0, 0, 100));
        this.setPaintBottom(false);
        this.setPaintTop(false);
        this.setPaintRight(false);
        this.setPaintLeft(true); // Left border acts as the separator

        // --- Content Area Setup ---
        WebPanel content = new WebPanel();
        content.setOpaque(false);
        content.setLayout(new VerticalFlowLayout(0, 0));
        // Standard padding
        content.setBorder(javax.swing.BorderFactory.createEmptyBorder(10, 20, 10, 20));

        // Let subclass populate the content
        initContent(content);

        // --- Scroll Pane ---
        // --- Center Component ---
        // --- Center Component ---
        this.add(createCenterComponent(content), BorderLayout.CENTER);

        // --- Top Toolbar (optional, subclasses can override) ---
        WebPanel topToolbar = createTopToolbar();
        if (topToolbar != null) {
            this.add(topToolbar, BorderLayout.NORTH);
        }

        // --- Bottom Control Panel ---
        createBottomPanel();
    }

    /**
     * Creates an optional top toolbar for the page.
     * Default implementation returns a styled panel with a title.
     * Subclasses can override and call super to get the styled container,
     * then add their specific controls to it.
     */
    protected WebPanel createTopToolbar() {
        WebPanel toolbar = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 10, 5));

        // Keep transparent background
        toolbar.setOpaque(false);

        // Create rounded line border with title
        javax.swing.border.Border lineBorder = javax.swing.BorderFactory.createLineBorder(
                new java.awt.Color(0, 0, 0, 120), 2, true); // Black rounded border (lighter)

        javax.swing.border.TitledBorder titledBorder = javax.swing.BorderFactory.createTitledBorder(
                lineBorder,
                prog.lang.mBasicSettings, // Localized title
                javax.swing.border.TitledBorder.LEFT,
                javax.swing.border.TitledBorder.TOP,
                prog.app.defaultFont,
                new java.awt.Color(0, 0, 0, 150));

        // Add padding inside the border
        toolbar.setBorder(javax.swing.BorderFactory.createCompoundBorder(
                titledBorder,
                javax.swing.BorderFactory.createEmptyBorder(8, 12, 8, 12)));

        return toolbar;
    }

    /**
     * Creates the component to be added to the center of the page.
     * Default implementation wraps the content in a WebScrollPane.
     * Subclasses can override this to provide custom behavior (e.g. scaling without
     * scrolling).
     */
    protected java.awt.Component createCenterComponent(WebPanel content) {
        WebScrollPane scroll = new WebScrollPane(content);
        scroll.setOpaque(false);
        scroll.getViewport().setOpaque(false);
        scroll.setBorder(null);
        scroll.setViewportBorder(null);
        scroll.setDrawBorder(false);
        return scroll;
    }

    /**
     * Implementing classes should add their specific UI components to this panel.
     * 
     * @param contentPanel The container with VerticalFlowLayout and standard
     *                     padding.
     */
    protected abstract void initContent(WebPanel contentPanel);

    protected void createBottomPanel() {
        WebPanel bottomPanel = new WebPanel(new BorderLayout());
        bottomPanel.setOpaque(false);
        // Increase padding to prevent "squeezed" look
        // bottomPanel.setBorder(javax.swing.BorderFactory.createEmptyBorder(10, 15, 15,
        // 15));

        // Use UIBuilder.createFooterButton for standardizing ALL bottom buttons

        // Left Group (Preview Controls)
        WebButton btnPreview = UIBuilder.createFooterButton(lang.mDisplayPreview);
        WebButton btnClosePreview = UIBuilder.createFooterButton(lang.mClosePreview);

        WebButtonGroup leftGroup = new WebButtonGroup(true, btnPreview, btnClosePreview);

        leftGroup.setButtonsShadeWidth(3);
        leftGroup.setButtonsDrawSides(false, false, false, true);
        leftGroup.setButtonsForeground(new java.awt.Color(0, 0, 0, 200)); // Match right group style

        btnPreview.addActionListener(new ActionListener() {
            @Override
            public void actionPerformed(ActionEvent e) {
                parent.startPreview();
            }
        });

        btnClosePreview.addActionListener(new ActionListener() {
            @Override
            public void actionPerformed(ActionEvent e) {
                parent.stopPreview();
            }
        });

        bottomPanel.add(leftGroup, BorderLayout.LINE_START);

        // Right Group (Exit/Start)
        WebButton btnExit = UIBuilder.createFooterButton(lang.mCancel);
        WebButton btnStart = UIBuilder.createFooterButton(lang.mStart);
        WebButtonGroup rightGroup = new WebButtonGroup(true, btnExit, btnStart);

        btnStart.setRound(10); // Start button specific styling from legacy

        rightGroup.setButtonsDrawSides(false, false, false, true);
        rightGroup.setButtonsForeground(new java.awt.Color(0, 0, 0, 200));
        rightGroup.setButtonsShadeWidth(3);

        btnExit.addActionListener(new ActionListener() {
            @Override
            public void actionPerformed(ActionEvent e) {
                parent.saveconfig();
                parent.tc.saveconfig();
                System.exit(0);
            }
        });

        btnStart.addActionListener(new ActionListener() {
            @Override
            public void actionPerformed(ActionEvent e) {
                parent.confirm();
            }
        });

        bottomPanel.add(rightGroup, BorderLayout.LINE_END);

        this.add(bottomPanel, BorderLayout.SOUTH);
    }

}
