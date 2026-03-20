package ui.layout;

import prog.Application;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;

import com.alee.extended.layout.VerticalFlowLayout;
import com.alee.extended.panel.WebButtonGroup;
import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.scroll.WebScrollPane;

import ui.MainForm;
import prog.i18n.Lang;

/**
 * Base abstract class for ALL UI content pages.
 * Encapsulates standard styling (separator line), layout structure
 * (Scroll/Content),
 * and the standard Bottom Control Panel.
 * 
 * New pages should extend this and implement initContent().
 */
public abstract class BasePage extends WebPanel {

    protected MainForm parent;
    protected boolean isDetailedMode = false;

    public BasePage(MainForm parent) {
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

        this.add(createCenterComponent(content), BorderLayout.CENTER);

        // --- Bottom Control Panel ---
        createBottomPanel();
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
        WebButton btnPreview = UIBuilder.createFooterButton(Lang.mDisplayPreview);
        WebButton btnClosePreview = UIBuilder.createFooterButton(Lang.mClosePreview);

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
        WebButton btnExit = UIBuilder.createFooterButton(Lang.mCancel);
        WebButton btnStart = UIBuilder.createFooterButton(Lang.mStart);
        WebButtonGroup rightGroup = new WebButtonGroup(true, btnExit, btnStart);

        btnStart.setRound(10); // Start button specific styling from legacy

        rightGroup.setButtonsDrawSides(false, false, false, true);
        rightGroup.setButtonsForeground(new java.awt.Color(0, 0, 0, 200));
        rightGroup.setButtonsShadeWidth(3);

        btnExit.addActionListener(new ActionListener() {
            @Override
            public void actionPerformed(ActionEvent e) {
                parent.saveConfig();
                parent.tc.saveConfig();
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
