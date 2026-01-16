package ui.panels;

import java.awt.Color;
import java.awt.Dimension;
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

public class FlightInfoPanel extends WebPanel {

    public WebSwitch bFlightInfoSwitch;
    public WebSwitch bFlightInfoEdge;
    public WebSwitch battitudeIndicatorSwitch;
    public WebSwitch bFMPrintSwitch;

    public WebSwitch bFlightInfoIAS;
    public WebSwitch bFlightInfoTAS;
    public WebSwitch bFlightInfoMach;
    public WebSwitch bFlightInfoCompass;
    public WebSwitch bFlightInfoHeight;
    public WebSwitch bFlightInfoVario;
    public WebSwitch bFlightInfoSEP;
    public WebSwitch bFlightInfoAcc;
    public WebSwitch bFlightInfoWx;
    public WebSwitch bFlightInfoNy;
    public WebSwitch bFlightInfoTurn;
    public WebSwitch bFlightInfoTurnRadius;
    public WebSwitch bFlightInfoAoA;
    public WebSwitch bFlightInfoAoS;
    public WebSwitch bFlightInfoWingSweep;
    public WebSwitch bFlightInfoRadioAlt;

    public WebComboBox sFlightInfoFont;
    public WebSlider iFlightInfoFontSizeIncr;
    public WebSlider iflightInfoColumnNum;

    private Runnable onChangeCallback;

    public FlightInfoPanel() {
        super();
        this.setOpaque(false);
        this.setBackground(new Color(0, 0, 0, 0));
        initUI();
    }

    private void initUI() {
        this.setLayout(new FlowLayout(FlowLayout.LEFT));

        bFlightInfoSwitch = UIBuilder.addSwitch(this, lang.mP4FlightInfoPanel, false);
        createvoidWebLabel(this, lang.mP4FlightInfoBlank);

        bFlightInfoEdge = UIBuilder.addSwitch(this, lang.mP4FlightInfoGlassEdge, false);
        createvoidWebLabel(this, lang.mP4FlightInfoGlassEdgeBlank);

        battitudeIndicatorSwitch = UIBuilder.addSwitch(this, lang.mP4attitudeIndicatorPanel, false);
        createvoidWebLabel(this, lang.mP4attitudeIndicatorPanelBlank);

        bFMPrintSwitch = UIBuilder.addSwitch(this, lang.mP4FMPanel, false);
        createvoidWebLabel(this, lang.mP4FMPanelBlank);

        bFlightInfoIAS = UIBuilder.addSwitch(this, lang.mP4fiIAS, false);
        createvoidWebLabel(this, lang.mP4fiIASBlank);

        bFlightInfoTAS = UIBuilder.addSwitch(this, lang.mP4fiTAS, false);
        createvoidWebLabel(this, lang.mP4fiIASBlank);

        bFlightInfoMach = UIBuilder.addSwitch(this, lang.mP4fiMach, false);
        createvoidWebLabel(this, lang.mP4fiMachBlank);

        bFlightInfoCompass = UIBuilder.addSwitch(this, lang.mP4fiCompass, false);
        createvoidWebLabel(this, lang.mP4fiCompassBlank);

        bFlightInfoHeight = UIBuilder.addSwitch(this, lang.mP4fiHeight, false);
        createvoidWebLabel(this, lang.mP4fiHeightBlank);

        bFlightInfoVario = UIBuilder.addSwitch(this, lang.mP4fiVario, false);
        createvoidWebLabel(this, lang.mP4fiVarioBlank);

        bFlightInfoSEP = UIBuilder.addSwitch(this, lang.mP4fiSEP, false);
        createvoidWebLabel(this, lang.mP4fiSEPBlank);

        bFlightInfoAcc = UIBuilder.addSwitch(this, lang.mP4fiAcc, false);
        createvoidWebLabel(this, lang.mP4fiAccBlank);

        bFlightInfoWx = UIBuilder.addSwitch(this, lang.mP4fiWx, false);
        createvoidWebLabel(this, lang.mP4fiWxBlank);

        bFlightInfoNy = UIBuilder.addSwitch(this, lang.mP4fiNy, false);
        createvoidWebLabel(this, lang.mP4fiNyBlank);

        bFlightInfoTurn = UIBuilder.addSwitch(this, lang.mP4fiTurn, false);
        createvoidWebLabel(this, lang.mP4fiTurnBlank);

        bFlightInfoTurnRadius = UIBuilder.addSwitch(this, lang.mP4fiTurnRadius, false);
        createvoidWebLabel(this, lang.mP4fiTurnRadiusBlank);

        bFlightInfoAoA = UIBuilder.addSwitch(this, lang.mP4fiAoA, false);
        createvoidWebLabel(this, lang.mP4fiAoABlank);

        bFlightInfoAoS = UIBuilder.addSwitch(this, lang.mP4fiAoS, false);
        createvoidWebLabel(this, lang.mP4fiAoSBlank);

        bFlightInfoWingSweep = UIBuilder.addSwitch(this, lang.mP4fiWingSweep, false);
        createvoidWebLabel(this, lang.mP4fiWingSweepBlank);

        bFlightInfoRadioAlt = UIBuilder.addSwitch(this, lang.mP4fiRadioAlt, false);
        createvoidWebLabel(this, lang.mP4fiRadioAltBlank);

        sFlightInfoFont = UIBuilder.addFontComboBox(this, lang.mP4PanelFont, app.fonts);
        iFlightInfoFontSizeIncr = UIBuilder.addSlider(this, lang.mP4FontAdjust, -6, 20, 0, 200, 1, 4);
        iflightInfoColumnNum = UIBuilder.addSlider(this, lang.mP4ColumnAdjust, 1, 16, 2, 200, 1, 2);

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
        bFlightInfoSwitch.addActionListener(e -> fireChange());
        bFlightInfoEdge.addActionListener(e -> fireChange());
        battitudeIndicatorSwitch.addActionListener(e -> fireChange());
        bFMPrintSwitch.addActionListener(e -> fireChange());
        bFlightInfoIAS.addActionListener(e -> fireChange());
        bFlightInfoTAS.addActionListener(e -> fireChange());
        bFlightInfoMach.addActionListener(e -> fireChange());
        bFlightInfoCompass.addActionListener(e -> fireChange());
        bFlightInfoHeight.addActionListener(e -> fireChange());
        bFlightInfoVario.addActionListener(e -> fireChange());
        bFlightInfoSEP.addActionListener(e -> fireChange());
        bFlightInfoAcc.addActionListener(e -> fireChange());
        bFlightInfoWx.addActionListener(e -> fireChange());
        bFlightInfoNy.addActionListener(e -> fireChange());
        bFlightInfoTurn.addActionListener(e -> fireChange());
        bFlightInfoTurnRadius.addActionListener(e -> fireChange());
        bFlightInfoAoA.addActionListener(e -> fireChange());
        bFlightInfoAoS.addActionListener(e -> fireChange());
        bFlightInfoWingSweep.addActionListener(e -> fireChange());
        bFlightInfoRadioAlt.addActionListener(e -> fireChange());
        sFlightInfoFont.addActionListener(e -> fireChange());

        iFlightInfoFontSizeIncr.addChangeListener(e -> {
            if (!iFlightInfoFontSizeIncr.getValueIsAdjusting())
                fireChange();
        });
        iflightInfoColumnNum.addChangeListener(e -> {
            if (!iflightInfoColumnNum.getValueIsAdjusting())
                fireChange();
        });
    }

    public void loadConfig(ConfigurationService cs) {
        bFlightInfoSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("flightInfoSwitch")));
        bFlightInfoEdge.setSelected(Boolean.parseBoolean(cs.getConfig("flightInfoEdge")));
        battitudeIndicatorSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("enableAttitudeIndicator")));
        bFMPrintSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("enableFMPrint")));

        bFlightInfoIAS.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoIAS")));
        bFlightInfoTAS.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoTAS")));
        bFlightInfoMach.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoMach")));
        bFlightInfoCompass.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoCompass")));
        bFlightInfoHeight.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoHeight")));
        bFlightInfoVario.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoVario")));
        bFlightInfoSEP.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoSEP")));
        bFlightInfoAcc.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoAcc")));
        bFlightInfoWx.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoWx")));
        bFlightInfoNy.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoNy")));
        bFlightInfoTurn.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoTurn")));
        bFlightInfoTurnRadius.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoTurnRadius")));
        bFlightInfoAoA.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoAoA")));
        bFlightInfoAoS.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoAoS")));
        bFlightInfoWingSweep.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoWingSweep")));
        bFlightInfoRadioAlt.setSelected(!Boolean.parseBoolean(cs.getConfig("disableFlightInfoRadioAlt")));

        String font = cs.getConfig("flightInfoFontC");
        if (font != null && !font.isEmpty())
            sFlightInfoFont.setSelectedItem(font);

        String fontAdd = cs.getConfig("flightInfoFontaddC");
        if (fontAdd != null && !fontAdd.isEmpty())
            iFlightInfoFontSizeIncr.setValue(Integer.parseInt(fontAdd));

        String col = cs.getConfig("flightInfoColumn");
        if (col != null && !col.isEmpty())
            iflightInfoColumnNum.setValue(Integer.parseInt(col));
    }

    public void saveConfig(ConfigurationService cs) {
        cs.setConfig("flightInfoSwitch", Boolean.toString(bFlightInfoSwitch.isSelected()));
        cs.setConfig("flightInfoEdge", Boolean.toString(bFlightInfoEdge.isSelected()));
        cs.setConfig("enableAttitudeIndicator", Boolean.toString(battitudeIndicatorSwitch.isSelected()));
        cs.setConfig("enableFMPrint", Boolean.toString(bFMPrintSwitch.isSelected()));

        cs.setConfig("disableFlightInfoIAS", Boolean.toString(!bFlightInfoIAS.isSelected()));
        cs.setConfig("disableFlightInfoTAS", Boolean.toString(!bFlightInfoTAS.isSelected()));
        cs.setConfig("disableFlightInfoMach", Boolean.toString(!bFlightInfoMach.isSelected()));
        cs.setConfig("disableFlightInfoCompass", Boolean.toString(!bFlightInfoCompass.isSelected()));
        cs.setConfig("disableFlightInfoHeight", Boolean.toString(!bFlightInfoHeight.isSelected()));
        cs.setConfig("disableFlightInfoVario", Boolean.toString(!bFlightInfoVario.isSelected()));
        cs.setConfig("disableFlightInfoSEP", Boolean.toString(!bFlightInfoSEP.isSelected()));
        cs.setConfig("disableFlightInfoAcc", Boolean.toString(!bFlightInfoAcc.isSelected()));
        cs.setConfig("disableFlightInfoWx", Boolean.toString(!bFlightInfoWx.isSelected()));
        cs.setConfig("disableFlightInfoNy", Boolean.toString(!bFlightInfoNy.isSelected()));
        cs.setConfig("disableFlightInfoTurn", Boolean.toString(!bFlightInfoTurn.isSelected()));
        cs.setConfig("disableFlightInfoTurnRadius", Boolean.toString(!bFlightInfoTurnRadius.isSelected()));
        cs.setConfig("disableFlightInfoAoA", Boolean.toString(!bFlightInfoAoA.isSelected()));
        cs.setConfig("disableFlightInfoAoS", Boolean.toString(!bFlightInfoAoS.isSelected()));
        cs.setConfig("disableFlightInfoWingSweep", Boolean.toString(!bFlightInfoWingSweep.isSelected()));
        cs.setConfig("disableFlightInfoRadioAlt", Boolean.toString(!bFlightInfoRadioAlt.isSelected()));

        cs.setConfig("flightInfoFontC", sFlightInfoFont.getSelectedItem().toString());
        cs.setConfig("flightInfoFontaddC", Integer.toString(iFlightInfoFontSizeIncr.getValue()));
        cs.setConfig("flightInfoColumn", Integer.toString(iflightInfoColumnNum.getValue()));
    }
}
