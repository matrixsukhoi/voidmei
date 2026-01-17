package ui.panels;

import java.awt.Color;
import java.awt.FlowLayout;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.panel.WebPanel;

import prog.config.ConfigurationService;
import prog.i18n.Lang;
import ui.layout.UIBuilder;

public class EngineControlPanel extends WebPanel {

    public WebSwitch bEnableAxis;
    public WebSwitch bEnableAxisEdge;
    public WebSwitch bEnablegearAndFlaps;
    public WebSwitch bEnablegearAndFlapsEdge;
    public WebSwitch benableEngineControl;

    public WebSwitch bEngineControlThrottle;
    public WebSwitch bEngineControlPitch;
    public WebSwitch bEngineControlMixture;
    public WebSwitch bEngineControlRadiator;
    public WebSwitch bEngineControlCompressor;
    public WebSwitch bEngineControlLFuel;

    private Runnable onChangeCallback;

    public EngineControlPanel() {
        super();
        this.setOpaque(false);
        this.setBackground(new Color(0, 0, 0, 0));
        initUI();
    }

    private void initUI() {
        this.setLayout(new FlowLayout(FlowLayout.LEFT));

        bEnableAxis = UIBuilder.addSwitch(this, Lang.mP6AxisPanel, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6ecLFuelBlank);

        bEnableAxisEdge = UIBuilder.addSwitch(this, Lang.mP6AxisEdge, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6AxisEdgeBlank);

        bEnablegearAndFlaps = UIBuilder.addSwitch(this, Lang.mP6GearAndFlaps, false);
        bEnablegearAndFlapsEdge = UIBuilder.addSwitch(this, Lang.mP6GearAndFlapsEdge, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6GearAndFlapsEdgeBlank);

        benableEngineControl = UIBuilder.addSwitch(this, Lang.mP6engineControl, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6engineControlBlank);

        bEngineControlThrottle = UIBuilder.addSwitch(this, Lang.mP6ecThrottle, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6ecThrottleBlank);

        bEngineControlPitch = UIBuilder.addSwitch(this, Lang.mP6ecPitch, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6ecPitchBlank);

        bEngineControlMixture = UIBuilder.addSwitch(this, Lang.mP6ecMixture, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6ecMixtureBlank);

        bEngineControlRadiator = UIBuilder.addSwitch(this, Lang.mP6ecRadiator, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6ecRadiatorBlank);

        bEngineControlCompressor = UIBuilder.addSwitch(this, Lang.mP6ecCompressor, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6ecCompressorBlank);

        bEngineControlLFuel = UIBuilder.addSwitch(this, Lang.mP6ecLFuel, false);
        UIBuilder.addVoidWebLabel(this, Lang.mP6ecLFuelBlank);

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
        bEnableAxis.addActionListener(e -> fireChange());
        bEnableAxisEdge.addActionListener(e -> fireChange());
        bEnablegearAndFlaps.addActionListener(e -> fireChange());
        bEnablegearAndFlapsEdge.addActionListener(e -> fireChange());
        benableEngineControl.addActionListener(e -> fireChange());
        bEngineControlThrottle.addActionListener(e -> fireChange());
        bEngineControlPitch.addActionListener(e -> fireChange());
        bEngineControlMixture.addActionListener(e -> fireChange());
        bEngineControlRadiator.addActionListener(e -> fireChange());
        bEngineControlCompressor.addActionListener(e -> fireChange());
        bEngineControlLFuel.addActionListener(e -> fireChange());
    }

    public void loadConfig(ConfigurationService cs) {
        bEnableAxis.setSelected(Boolean.parseBoolean(cs.getConfig("enableAxis")));
        bEnableAxisEdge.setSelected(Boolean.parseBoolean(cs.getConfig("enableAxisEdge")));
        bEnablegearAndFlaps.setSelected(Boolean.parseBoolean(cs.getConfig("enablegearAndFlaps")));
        bEnablegearAndFlapsEdge.setSelected(Boolean.parseBoolean(cs.getConfig("enablegearAndFlapsEdge")));
        benableEngineControl.setSelected(Boolean.parseBoolean(cs.getConfig("enableEngineControl")));

        bEngineControlThrottle.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoThrottle")));
        bEngineControlPitch.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoPitch")));
        bEngineControlMixture.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoMixture")));
        bEngineControlRadiator.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoRadiator")));
        bEngineControlCompressor.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoCompressor")));
        bEngineControlLFuel.setSelected(!Boolean.parseBoolean(cs.getConfig("disableEngineInfoLFuel")));
    }

    public void saveConfig(ConfigurationService cs) {
        cs.setConfig("enableAxis", Boolean.toString(bEnableAxis.isSelected()));
        cs.setConfig("enableAxisEdge", Boolean.toString(bEnableAxisEdge.isSelected()));
        cs.setConfig("enablegearAndFlaps", Boolean.toString(bEnablegearAndFlaps.isSelected()));
        cs.setConfig("enablegearAndFlapsEdge", Boolean.toString(bEnablegearAndFlapsEdge.isSelected()));
        cs.setConfig("enableEngineControl", Boolean.toString(benableEngineControl.isSelected()));

        cs.setConfig("disableEngineInfoThrottle", Boolean.toString(!bEngineControlThrottle.isSelected()));
        cs.setConfig("disableEngineInfoPitch", Boolean.toString(!bEngineControlPitch.isSelected()));
        cs.setConfig("disableEngineInfoMixture", Boolean.toString(!bEngineControlMixture.isSelected()));
        cs.setConfig("disableEngineInfoRadiator", Boolean.toString(!bEngineControlRadiator.isSelected()));
        cs.setConfig("disableEngineInfoCompressor", Boolean.toString(!bEngineControlCompressor.isSelected()));
        cs.setConfig("disableEngineInfoLFuel", Boolean.toString(!bEngineControlLFuel.isSelected()));
    }
}
