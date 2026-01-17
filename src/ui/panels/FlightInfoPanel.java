package ui.panels;

import java.awt.Color;
import java.awt.FlowLayout;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.config.ConfigurationService;
import prog.Application;
import prog.i18n.Lang;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;
import ui.layout.UIBuilder;

public class FlightInfoPanel extends WebPanel {

    public static void initDefaults(ConfigurationService cs) {
        cs.setConfig("flightInfoSwitch", Boolean.toString(Boolean.TRUE));
        cs.setConfig("flightInfoEdge", Boolean.toString(Boolean.FALSE));
        cs.setConfig("flightInfoFontC", Application.defaultFontName);
        cs.setConfig("flightInfoFontaddC", Integer.toString(0));
    }

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

        bFlightInfoSwitch = UIBuilder.addSwitch(this, Lang.mP4FlightInfoPanel, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4FlightInfoBlank);

        bFlightInfoEdge = UIBuilder.addSwitch(this, Lang.mP4FlightInfoGlassEdge, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4FlightInfoGlassEdgeBlank);

        battitudeIndicatorSwitch = UIBuilder.addSwitch(this, Lang.mP4attitudeIndicatorPanel, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4attitudeIndicatorPanelBlank);

        bFMPrintSwitch = UIBuilder.addSwitch(this, Lang.mP4FMPanel, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4FMPanelBlank);

        bFlightInfoIAS = UIBuilder.addSwitch(this, Lang.mP4fiIAS, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiIASBlank);

        bFlightInfoTAS = UIBuilder.addSwitch(this, Lang.mP4fiTAS, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiIASBlank);

        bFlightInfoMach = UIBuilder.addSwitch(this, Lang.mP4fiMach, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiMachBlank);

        bFlightInfoCompass = UIBuilder.addSwitch(this, Lang.mP4fiCompass, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiCompassBlank);

        bFlightInfoHeight = UIBuilder.addSwitch(this, Lang.mP4fiHeight, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiHeightBlank);

        bFlightInfoVario = UIBuilder.addSwitch(this, Lang.mP4fiVario, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiVarioBlank);

        bFlightInfoSEP = UIBuilder.addSwitch(this, Lang.mP4fiSEP, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiSEPBlank);

        bFlightInfoAcc = UIBuilder.addSwitch(this, Lang.mP4fiAcc, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiAccBlank);

        bFlightInfoWx = UIBuilder.addSwitch(this, Lang.mP4fiWx, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiWxBlank);

        bFlightInfoNy = UIBuilder.addSwitch(this, Lang.mP4fiNy, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiNyBlank);

        bFlightInfoTurn = UIBuilder.addSwitch(this, Lang.mP4fiTurn, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiTurnBlank);

        bFlightInfoTurnRadius = UIBuilder.addSwitch(this, Lang.mP4fiTurnRadius, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiTurnRadiusBlank);

        bFlightInfoAoA = UIBuilder.addSwitch(this, Lang.mP4fiAoA, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiAoABlank);

        bFlightInfoAoS = UIBuilder.addSwitch(this, Lang.mP4fiAoS, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiAoSBlank);

        bFlightInfoWingSweep = UIBuilder.addSwitch(this, Lang.mP4fiWingSweep, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiWingSweepBlank);

        bFlightInfoRadioAlt = UIBuilder.addSwitch(this, Lang.mP4fiRadioAlt, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP4fiRadioAltBlank);

        sFlightInfoFont = UIBuilder.addFontComboBox(this, Lang.mP4PanelFont, Application.fonts);
        iFlightInfoFontSizeIncr = UIBuilder.addSlider(this, Lang.mP4FontAdjust, -6, 20, 0, 200, 1, 4);
        iflightInfoColumnNum = UIBuilder.addSlider(this, Lang.mP4ColumnAdjust, 1, 16, 2, 200, 1, 2);

        setupListeners();
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
        bFMPrintSwitch.addActionListener(e -> {
            fireChange();
            UIStateBus.getInstance().publish(
                    UIStateEvents.FM_PRINT_SWITCH_CHANGED,
                    bFMPrintSwitch.isSelected());
        });
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

        // Subscribe to FM Print switch changes from LoggingPanel
        UIStateBus.getInstance().subscribe(UIStateEvents.FM_PRINT_SWITCH_CHANGED, data -> {
            Boolean newState = (Boolean) data;
            if (bFMPrintSwitch.isSelected() != newState) {
                bFMPrintSwitch.setSelected(newState);
            }
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
