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
 * An example usage of the new Layout Manager framework.
 */
public class ExamplePage extends WebPanel {

    private mainform parent;

    public ExamplePage(mainform parent) {
        super();
        this.parent = parent;
        this.setLayout(new BorderLayout());
        this.setOpaque(false);
        this.setBackground(new Color(0, 0, 0, 0));

        // --- Content Area ---
        WebPanel content = new WebPanel();
        content.setOpaque(false);
        content.setLayout(new VerticalFlowLayout(0, 0));
        // Add padding to satisfy "neat padding" requirement
        content.setBorder(javax.swing.BorderFactory.createEmptyBorder(10, 20, 10, 20));

        WebPanel titleGrid = UIBuilder.createGridContainer(2);
        UIBuilder.addSwitch(titleGrid, "飞行信息面板", true);
        UIBuilder.addSwitch(titleGrid, "玻璃边框", false);
        UIBuilder.addSwitch(titleGrid, "地平仪面板", true);
        UIBuilder.addSwitch(titleGrid, "拆包信息", false);
        content.add(titleGrid);

        // Grid Container for Engine Info Items (4 items per row)
        WebPanel engineGrid = UIBuilder.createGridContainer(4);

        UIBuilder.addSwitch(engineGrid, "显示示空速", true);
        UIBuilder.addSwitch(engineGrid, "显示真空速", false);
        UIBuilder.addSwitch(engineGrid, "显示马赫数", false);
        UIBuilder.addSwitch(engineGrid, "显示航向", true);

        UIBuilder.addSwitch(engineGrid, "显示高度", false);
        UIBuilder.addSwitch(engineGrid, "显示爬升率", true);
        UIBuilder.addSwitch(engineGrid, "显示SEP", true);
        UIBuilder.addSwitch(engineGrid, "显示加速度", false);

        UIBuilder.addSwitch(engineGrid, "显示滚转率", false);
        UIBuilder.addSwitch(engineGrid, "显示过载", false);
        UIBuilder.addSwitch(engineGrid, "显示转弯率", true);
        UIBuilder.addSwitch(engineGrid, "显示转半径", true);

        UIBuilder.addSwitch(engineGrid, "显示攻角", false);
        UIBuilder.addSwitch(engineGrid, "显示侧滑角", true);
        UIBuilder.addSwitch(engineGrid, "显示可变翼", true);
        UIBuilder.addSwitch(engineGrid, "显示测距高", true);

        content.add(engineGrid);

        // Font and slider controls section
        WebPanel controlsGrid = UIBuilder.createGridContainer(2);

        // Font selector
        UIBuilder.addComboBox(controlsGrid, "Panel Font", prog.app.fonts);

        // Font size slider
        UIBuilder.addSlider(controlsGrid, "Font Adjust", -6, 20, 0);

        // Column count slider
        UIBuilder.addSlider(controlsGrid, "Column Adjust", 1, 16, 4);

        content.add(controlsGrid);

        WebScrollPane scroll = new WebScrollPane(content);
        scroll.setOpaque(false);
        scroll.getViewport().setOpaque(false);
        scroll.setBorder(null);
        scroll.setViewportBorder(null);
        scroll.setDrawBorder(false); // Force removal of WebLaF specific border
        this.add(scroll, BorderLayout.CENTER);

        // --- Bottom Control Panel ---
        createBottomPanel();
    }

    private void createBottomPanel() {
        WebPanel bottomPanel = new WebPanel(new BorderLayout());
        bottomPanel.setOpaque(false);
        // Add some padding for the footer too
        bottomPanel.setBorder(javax.swing.BorderFactory.createEmptyBorder(5, 5, 5, 5));

        // Left Group (Preview Controls)
        WebButton btnPreview = parent.createButton(lang.mDisplayPreview);
        WebButton btnClosePreview = parent.createButton(lang.mClosePreview);
        WebButtonGroup leftGroup = new WebButtonGroup(true, btnPreview, btnClosePreview);
        btnPreview.setPreferredWidth(120);
        btnClosePreview.setPreferredWidth(120);
        // parent.createButton sets defaultFontBig, so these should match Start/Exit
        // height now
        // Removed manual font set to ensure consistency

        leftGroup.setButtonsShadeWidth(3);
        leftGroup.setButtonsDrawSides(false, false, false, true);

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
        WebButton btnExit = parent.createButton(lang.mCancel);
        WebButton btnStart = parent.createButton(lang.mStart);
        WebButtonGroup rightGroup = new WebButtonGroup(true, btnExit, btnStart);
        btnExit.setPreferredWidth(120);
        btnStart.setPreferredWidth(120);
        btnStart.setRound(10);
        rightGroup.setButtonsDrawSides(false, false, false, true);
        rightGroup.setButtonsForeground(new Color(0, 0, 0, 200));
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
