package ui.panels;

import java.awt.Color;
import java.awt.FlowLayout;

import javax.swing.SwingConstants;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.ConfigurationService;
import prog.app;
import prog.lang;
import ui.layout.UIBuilder;

public class EngineInfoPanel extends WebPanel {

    public WebSwitch bEngineInfoSwitch;
    public WebSwitch bEngineInfoEdge;

    public WebSwitch bEngineInfoHorsePower;
    public WebSwitch bEngineInfoThrust;
    public WebSwitch bEngineInfoRPM;
    public WebSwitch bEngineInfoPropPitch;
    public WebSwitch bEngineInfoEffEta;
    public WebSwitch bEngineInfoEffHp;
    public WebSwitch bEngineInfoPressure;
    public WebSwitch bEngineInfoPowerPercent;
    public WebSwitch bEngineInfoFuelKg;
    public WebSwitch bEngineInfoFuelTime;
    public WebSwitch bEngineInfoWepKg;
    public WebSwitch bEngineInfoWepTime;
    public WebSwitch bEngineInfoTemp;
    public WebSwitch bEngineInfoOilTemp;
    public WebSwitch bEngineInfoHeatTolerance;
    public WebSwitch bEngineInfoEngResponse;

    public WebComboBox fEngineInfoFont;
    public WebSlider iEngineInfoFontSizeIncr;
    public WebSlider iengineInfoColumnNum;

    private Runnable onChangeCallback;

    public EngineInfoPanel() {
        super();
        this.setOpaque(false);
        this.setBackground(new Color(0, 0, 0, 0));
        initUI();
    }

    private void initUI() {
        this.setLayout(new FlowLayout(FlowLayout.LEFT));

        bEngineInfoSwitch = UIBuilder.addSwitch(this, lang.mP2EnginePanel, false);
        createvoidWebLabel(this, lang.mP2EnginePanelBlank);

        bEngineInfoEdge = UIBuilder.addSwitch(this, lang.mP2EngineGlassEdge, false);
        createvoidWebLabel(this, lang.mP2EngineGlassEdgeBlank);

        bEngineInfoHorsePower = UIBuilder.addSwitch(this, lang.mP2eiHorsePower, false);
        createvoidWebLabel(this, lang.mP2eiHorsePowerBlank);

        bEngineInfoThrust = UIBuilder.addSwitch(this, lang.mP2eiThrust, false);
        createvoidWebLabel(this, lang.mP2eiThrustBlank);

        bEngineInfoRPM = UIBuilder.addSwitch(this, lang.mP2eiRPM, false);
        createvoidWebLabel(this, lang.mP2eiRPMBlank);

        bEngineInfoPropPitch = UIBuilder.addSwitch(this, lang.mP2eiPropPitch, false);
        createvoidWebLabel(this, lang.mP2eiPropPitchBlank);

        bEngineInfoEffEta = UIBuilder.addSwitch(this, lang.mP2eiEffEta, false);
        createvoidWebLabel(this, lang.mP2eiEffEtaBlank);

        bEngineInfoEffHp = UIBuilder.addSwitch(this, lang.mP2eiEffHp, false);
        createvoidWebLabel(this, lang.mP2eiEffHpBlank);

        bEngineInfoPressure = UIBuilder.addSwitch(this, lang.mP2eiPressure, false);
        createvoidWebLabel(this, lang.mP2eiPressureBlank);

        bEngineInfoPowerPercent = UIBuilder.addSwitch(this, lang.mP2eiPowerPercent, false);
        createvoidWebLabel(this, lang.mP2eiPowerPercentBlank);

        bEngineInfoFuelKg = UIBuilder.addSwitch(this, lang.mP2eiFuelKg, false);
        createvoidWebLabel(this, lang.mP2eiFuelKgBlank);

        bEngineInfoFuelTime = UIBuilder.addSwitch(this, lang.mP2eiFuelTime, false);
        createvoidWebLabel(this, lang.mP2eiFuelTimeBlank);

        bEngineInfoWepKg = UIBuilder.addSwitch(this, lang.mP2eiWepKg, false);
        createvoidWebLabel(this, lang.mP2eiWepKgBlank);

        bEngineInfoWepTime = UIBuilder.addSwitch(this, lang.mP2eiWepTime, false);
        createvoidWebLabel(this, lang.mP2eiWepTimeBlank);

        bEngineInfoTemp = UIBuilder.addSwitch(this, lang.mP2eiTemp, false);
        createvoidWebLabel(this, lang.mP2eiTempBlank);

        bEngineInfoOilTemp = UIBuilder.addSwitch(this, lang.mP2eiOilTemp, false);
        createvoidWebLabel(this, lang.mP2eiOilTempBlank);

        bEngineInfoHeatTolerance = UIBuilder.addSwitch(this, lang.mP2eiHeatTolerance, false);
        createvoidWebLabel(this, lang.mP2eiHeatToleranceBlank);

        bEngineInfoEngResponse = UIBuilder.addSwitch(this, lang.mP2eiEngResponse, false);
        createvoidWebLabel(this, lang.mP2eiEngResponseBlank);

        fEngineInfoFont = UIBuilder.addFontComboBox(this, lang.mP2PanelFont, app.fonts);
        iEngineInfoFontSizeIncr = UIBuilder.addSlider(this, lang.mP2FontAdjust, -6, 20, 0, 200, 1, 4);
        iengineInfoColumnNum = UIBuilder.addSlider(this, lang.mP4ColumnAdjust, 1, 16, 2, 200, 1, 2);

        createvoidWebLabel(this, lang.mP2EngineBlank);

        setupListeners();
    }

    private void createvoidWebLabel(WebPanel panel, String text) {
        WebLabel lb = new WebLabel(text);
        lb.setHorizontalAlignment(SwingConstants.CENTER);
        lb.setForeground(new Color(0, 0, 0, 230));
        lb.setShadeColor(Color.WHITE);
        lb.setFont(app.defaultFont);
        panel.add(lb);
    }

    public void setOnChange(Runnable callback) {
        this.onChangeCallback = callback;
    }

    private void fireChange() {
        if (onChangeCallback != null) {
            onChangeCallback.run();
        }
    }

    private void setupListeners() {
        bEngineInfoSwitch.addActionListener(e -> fireChange());
        bEngineInfoEdge.addActionListener(e -> fireChange());
        bEngineInfoHorsePower.addActionListener(e -> fireChange());
        bEngineInfoThrust.addActionListener(e -> fireChange());
        bEngineInfoRPM.addActionListener(e -> fireChange());
        bEngineInfoPropPitch.addActionListener(e -> fireChange());
        bEngineInfoEffEta.addActionListener(e -> fireChange());
        bEngineInfoEffHp.addActionListener(e -> fireChange());
        bEngineInfoPressure.addActionListener(e -> fireChange());
        bEngineInfoPowerPercent.addActionListener(e -> fireChange());
        bEngineInfoFuelKg.addActionListener(e -> fireChange());
        bEngineInfoFuelTime.addActionListener(e -> fireChange());
        bEngineInfoWepKg.addActionListener(e -> fireChange());
        bEngineInfoWepTime.addActionListener(e -> fireChange());
        bEngineInfoTemp.addActionListener(e -> fireChange());
        bEngineInfoOilTemp.addActionListener(e -> fireChange());
        bEngineInfoHeatTolerance.addActionListener(e -> fireChange());
        bEngineInfoEngResponse.addActionListener(e -> fireChange());
        fEngineInfoFont.addActionListener(e -> fireChange());

        iEngineInfoFontSizeIncr.addChangeListener(e -> {
            if (!iEngineInfoFontSizeIncr.getValueIsAdjusting())
                fireChange();
        });
        iengineInfoColumnNum.addChangeListener(e -> {
            if (!iengineInfoColumnNum.getValueIsAdjusting())
                fireChange();
        });
    }

    public void loadConfig(ConfigurationService cs) {
        bEngineInfoSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("engineInfoSwitch")));
        bEngineInfoEdge.setSelected(Boolean.parseBoolean(cs.getConfig("engineInfoEdge")));

        bEngineInfoHorsePower.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoHorsePower")));
        bEngineInfoThrust.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoThrust")));
        bEngineInfoRPM.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoRPM")));
        bEngineInfoPropPitch.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoPropPitch")));
        bEngineInfoEffEta.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoEffEta")));
        bEngineInfoEffHp.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoEffHp")));
        bEngineInfoPressure.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoPressure")));
        bEngineInfoPowerPercent.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoPowerPercent")));
        bEngineInfoFuelKg.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoFuelKg")));
        bEngineInfoFuelTime.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoFuelTime")));
        bEngineInfoWepKg.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoWepKg")));
        bEngineInfoWepTime.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoWepTime")));
        bEngineInfoTemp.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoTemp")));
        bEngineInfoOilTemp.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoOilTemp")));
        bEngineInfoHeatTolerance.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoHeatTolerance")));
        bEngineInfoEngResponse.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoEngResponse")));

        String font = cs.getConfig("engineInfoFont");
        if (font != null && !font.isEmpty())
            fEngineInfoFont.setSelectedItem(font);

        String fontAdd = cs.getConfig("engineInfoFontadd");
        if (fontAdd != null && !fontAdd.isEmpty())
            iEngineInfoFontSizeIncr.setValue(Integer.parseInt(fontAdd));

        String col = cs.getConfig("engineInfoColumn");
        if (col != null && !col.isEmpty())
            iengineInfoColumnNum.setValue(Integer.parseInt(col));
    }

    public void saveConfig(ConfigurationService cs) {
        cs.setConfig("engineInfoSwitch", Boolean.toString(bEngineInfoSwitch.isSelected()));
        cs.setConfig("engineInfoEdge", Boolean.toString(bEngineInfoEdge.isSelected()));

        cs.setConfig("disableEngineInfoHorsePower", Boolean.toString(!bEngineInfoHorsePower.isSelected()));
        cs.setConfig("disableEngineInfoThrust", Boolean.toString(!bEngineInfoThrust.isSelected()));
        cs.setConfig("disableEngineInfoRPM", Boolean.toString(!bEngineInfoRPM.isSelected()));
        cs.setConfig("disableEngineInfoPropPitch", Boolean.toString(!bEngineInfoPropPitch.isSelected()));
        cs.setConfig("disableEngineInfoEffEta", Boolean.toString(!bEngineInfoEffEta.isSelected()));
        cs.setConfig("disableEngineInfoEffHp", Boolean.toString(!bEngineInfoEffHp.isSelected()));
        cs.setConfig("disableEngineInfoPressure", Boolean.toString(!bEngineInfoPressure.isSelected()));
        cs.setConfig("disableEngineInfoPowerPercent", Boolean.toString(!bEngineInfoPowerPercent.isSelected()));
        cs.setConfig("disableEngineInfoFuelKg", Boolean.toString(!bEngineInfoFuelKg.isSelected()));
        cs.setConfig("disableEngineInfoFuelTime", Boolean.toString(!bEngineInfoFuelTime.isSelected()));
        cs.setConfig("disableEngineInfoWepKg", Boolean.toString(!bEngineInfoWepKg.isSelected()));
        cs.setConfig("disableEngineInfoWepTime", Boolean.toString(!bEngineInfoWepTime.isSelected()));
        cs.setConfig("disableEngineInfoTemp", Boolean.toString(!bEngineInfoTemp.isSelected()));
        cs.setConfig("disableEngineInfoOilTemp", Boolean.toString(!bEngineInfoOilTemp.isSelected()));
        cs.setConfig("disableEngineInfoHeatTolerance", Boolean.toString(!bEngineInfoHeatTolerance.isSelected()));
        cs.setConfig("disableEngineInfoEngResponse", Boolean.toString(!bEngineInfoEngResponse.isSelected()));

        cs.setConfig("engineInfoFont", fEngineInfoFont.getSelectedItem().toString());
        cs.setConfig("engineInfoFontadd", Integer.toString(iEngineInfoFontSizeIncr.getValue()));
        cs.setConfig("engineInfoColumn", Integer.toString(iengineInfoColumnNum.getValue()));
    }
}
