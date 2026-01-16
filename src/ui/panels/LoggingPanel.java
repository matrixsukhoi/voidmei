package ui.panels;

import java.awt.Color;
import java.awt.FlowLayout;
import java.awt.Font;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import java.io.File;

import ui.util.FileUtils;

import com.alee.extended.button.WebSwitch;
import com.alee.extended.layout.VerticalFlowLayout;
import com.alee.extended.window.WebPopOver;
import com.alee.laf.button.WebButton;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.text.WebTextArea;
import com.github.kwhat.jnativehook.GlobalScreen;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;
import com.github.kwhat.jnativehook.keyboard.NativeKeyListener;

import parser.blkx;
import prog.ConfigurationService;
import prog.app;
import prog.lang;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;
import ui.layout.UIBuilder;

public class LoggingPanel extends WebPanel {

    public static void initDefaults(ConfigurationService cs) {
        cs.setConfig("enableLogging", Boolean.toString(Boolean.FALSE));
        cs.setConfig("enableAltInformation", Boolean.toString(Boolean.FALSE));
    }

    public WebSwitch bEnableLogging;
    public WebSwitch bEnableInformation;
    public WebComboBox bFMList0;
    public WebComboBox bFMList1;
    public WebSwitch bFMPrintLogSwitch;
    public WebButton bDisplayFmKey;

    private WebFrame parent;
    private Runnable onChangeCallback;
    private Runnable onSaveCallback;

    private int isDragging;
    private int xx;
    private int yy;

    public LoggingPanel(WebFrame parent) {
        super();
        this.parent = parent;
        this.setOpaque(false);
        this.setBackground(new Color(0, 0, 0, 0));
        initUI();
    }

    private void initUI() {
        this.setLayout(new FlowLayout(FlowLayout.LEFT));

        bEnableLogging = UIBuilder.addSwitch(this, lang.mP5LoggingAndCharting, false);
        UIBuilder.addVoidWebLabel(this, lang.mP5LoggingAndChartingBlank);

        bEnableInformation = UIBuilder.addSwitch(this, lang.mP5Information, false);
        UIBuilder.addVoidWebLabel(this, lang.mP5InformationBlank);

        bFMList0 = createFMList(this, lang.mP5FMChoose + " 0");
        UIBuilder.addVoidWebLabel(this, lang.mP5FMChooseBlank);
        bFMList0.addActionListener(new ActionListener() {
            private int t = 0;

            public void actionPerformed(ActionEvent e) {
                if (t++ != 0)
                    displayFM(bFMList0, 0);
            }
        });

        bFMList1 = createFMList(this, lang.mP5FMChoose + " 1");
        UIBuilder.addVoidWebLabel(this, lang.mP5FMChooseBlank);
        bFMList1.addActionListener(new ActionListener() {
            private int t = 0;

            public void actionPerformed(ActionEvent e) {
                if (t++ != 0)
                    displayFM(bFMList1, 1);
            }
        });

        bFMPrintLogSwitch = UIBuilder.addSwitch(this, lang.mP5FMPrintEnable, false);
        UIBuilder.addVoidWebLabel(this, lang.mP5FMPrintEnableBlank);

        UIBuilder.addVoidWebLabel(this, lang.mP5FMDisplayKey);
        bDisplayFmKey = new WebButton(NativeKeyEvent.getKeyText(app.displayFmKey));
        bDisplayFmKey.setFocusable(false);
        bDisplayFmKey.addActionListener(e -> {
            bDisplayFmKey.setText(lang.mP5FMDisplayKeyTip);
            GlobalScreen.addNativeKeyListener(new NativeKeyListener() {
                @Override
                public void nativeKeyPressed(NativeKeyEvent e) {
                    int code = e.getKeyCode();
                    if (code == NativeKeyEvent.VC_NUM_LOCK || code == NativeKeyEvent.VC_CAPS_LOCK
                            || code == NativeKeyEvent.VC_SCROLL_LOCK) {
                        return;
                    }
                    app.displayFmKey = code;
                    bDisplayFmKey.setText(NativeKeyEvent.getKeyText(app.displayFmKey));
                    if (onSaveCallback != null)
                        onSaveCallback.run();
                    GlobalScreen.removeNativeKeyListener(this);
                }
            });
        });
        this.add(bDisplayFmKey);

        setupListeners();
    }

    private void setupListeners() {
        bEnableLogging.addActionListener(e -> fireChange());
        bEnableInformation.addActionListener(e -> fireChange());
        bFMPrintLogSwitch.addActionListener(e -> {
            fireChange();
            // Also publish event so FlightInfoPanel can sync
            UIStateBus.getInstance().publish(
                    UIStateEvents.FM_PRINT_SWITCH_CHANGED,
                    bFMPrintLogSwitch.isSelected());
        });
        bFMList0.addActionListener(e -> fireChange());
        bFMList1.addActionListener(e -> fireChange());

        // Subscribe to FM Print switch changes from FlightInfoPanel
        UIStateBus.getInstance().subscribe(UIStateEvents.FM_PRINT_SWITCH_CHANGED, data -> {
            Boolean newState = (Boolean) data;
            if (bFMPrintLogSwitch.isSelected() != newState) {
                bFMPrintLogSwitch.setSelected(newState);
            }
        });
    }

    private void fireChange() {
        if (onChangeCallback != null)
            onChangeCallback.run();
    }

    public void setOnChange(Runnable callback) {
        this.onChangeCallback = callback;
    }

    public void setOnSave(Runnable callback) {
        this.onSaveCallback = callback;
    }

    private WebComboBox createFMList(WebPanel panel, String text) {
        UIBuilder.addVoidWebLabel(panel, text);
        File file = new File("data/aces/gamedata/flightmodels/fm");
        String[] filelist = file.list();
        if (filelist == null)
            filelist = new String[0];
        filelist = FileUtils.getFilelistNameNoEx(filelist);
        WebComboBox comboBox = new WebComboBox(filelist);
        comboBox.setWebColoredBackground(false);
        comboBox.setShadeWidth(1);
        comboBox.setDrawFocus(false);
        comboBox.setFont(app.defaultFont);
        comboBox.setExpandedBgColor(new Color(0, 0, 0, 0));
        panel.add(comboBox);
        return comboBox;
    }

    private void displayFM(WebComboBox bFMList, int idx) {
        String planeName = bFMList.getSelectedItem().toString();
        String path = "data/aces/gamedata/flightmodels/fm/" + planeName + ".blkx";
        blkx fmblk = new blkx(path, planeName);
        WebPopOver popOver = new WebPopOver(parent);
        popOver.setMargin(5);
        popOver.setLayout(new VerticalFlowLayout());
        WebButton closeButton = new WebButton(lang.mCancel, e -> popOver.dispose());
        closeButton.setUndecorated(true);
        closeButton.setFont(app.defaultFont);
        closeButton.setFontSize((int) (app.defaultFontsize * 1.5f));
        closeButton.setFontStyle(Font.BOLD);
        WebTextArea textArea = new WebTextArea(fmblk.fmdata);
        popOver.add(textArea);
        popOver.setFont(app.defaultFont);
        textArea.setFont(app.defaultFont);
        textArea.setFontSize((int) (app.defaultFontsize * 1.2f));
        popOver.add(closeButton);
        popOver.show(parent);

        textArea.addMouseListener(new MouseAdapter() {
            public void mousePressed(MouseEvent e) {
                isDragging = 1;
                xx = e.getX();
                yy = e.getY();
            }

            public void mouseReleased(MouseEvent e) {
                if (isDragging == 1)
                    isDragging = 0;
            }
        });
        textArea.addMouseMotionListener(new MouseMotionAdapter() {
            public void mouseDragged(MouseEvent e) {
                int left = popOver.getLocation().x;
                int top = popOver.getLocation().y;
                popOver.setLocation(left + e.getX() - xx, top + e.getY() - yy);
            }
        });
        popOver.setLocation(popOver.getLocation().x + idx * popOver.getSize().width, popOver.getLocation().y);
    }

    public void loadConfig(ConfigurationService cs) {
        bEnableLogging.setSelected(Boolean.parseBoolean(cs.getConfig("enableLogging")));
        bEnableInformation.setSelected(Boolean.parseBoolean(cs.getConfig("enableAltInformation")));
        String fm0 = cs.getConfig("selectedFM0");
        if (fm0 != null)
            bFMList0.setSelectedItem(fm0);
        String fm1 = cs.getConfig("selectedFM1");
        if (fm1 != null)
            bFMList1.setSelectedItem(fm1);
        bFMPrintLogSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("enableFMPrint")));
    }

    public void saveConfig(ConfigurationService cs) {
        cs.setConfig("enableLogging", Boolean.toString(bEnableLogging.isSelected()));
        cs.setConfig("enableAltInformation", Boolean.toString(bEnableInformation.isSelected()));
        if (bFMList0.getSelectedItem() != null)
            cs.setConfig("selectedFM0", bFMList0.getSelectedItem().toString());
        if (bFMList1.getSelectedItem() != null)
            cs.setConfig("selectedFM1", bFMList1.getSelectedItem().toString());
        cs.setConfig("enableFMPrint", Boolean.toString(bFMPrintLogSwitch.isSelected()));
    }
}
