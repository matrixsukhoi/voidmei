package ui.panels;

import java.awt.FlowLayout;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.config.ConfigurationService;
import prog.Application;
import prog.i18n.Lang;
import ui.MainForm;
import ui.layout.UIBuilder;

/**
 * MiniHUDPanel encapsulates the UI and logic for the "MiniHUD" tab.
 * It includes crosshair switches, scaling, texture selection, and mono font
 * settings.
 */
public class MiniHUDPanel extends WebPanel {
    private static final long serialVersionUID = 1L;

    public static void initDefaults(ConfigurationService cs) {
        cs.setConfig("crosshairSwitch", Boolean.toString(Boolean.FALSE));
        cs.setConfig("crosshairScale", Integer.toString(10));
        cs.setConfig("usetexturecrosshair", Boolean.toString(Boolean.FALSE));
        cs.setConfig("drawHUDtext", Boolean.toString(Boolean.FALSE));
        cs.setConfig("enableFlapAngleBar", Boolean.toString(Boolean.TRUE));
        cs.setConfig("displayCrosshair", Boolean.toString(Boolean.FALSE));
    }

    private final MainForm parent;

    public WebSwitch bCrosshairSwitch;
    public WebSwitch bcrosshairdisplaySwitch;
    public WebSwitch bDrawHudTextSwitch;
    public WebSwitch bFlapBarSwitch;
    public WebSwitch bTextureCrosshairSwitch;
    public WebComboBox sCrosshairName;
    public WebSlider iCrosshairScale;
    public WebComboBox sMonoFont;

    private Runnable onChange;
    private Runnable onSave;

    public MiniHUDPanel(MainForm parent) {
        this.parent = parent;
        setLayout(new FlowLayout(FlowLayout.LEFT));
        setOpaque(false);
        initUI();
    }

    public void setOnChange(Runnable onChange) {
        this.onChange = onChange;
    }

    public void setOnSave(Runnable onSave) {
        this.onSave = onSave;
    }

    private void fireChange() {
        if (onChange != null) {
            onChange.run();
        }
    }

    private void fireSave() {
        if (onSave != null) {
            onSave.run();
        }
    }

    private void initUI() {
        bCrosshairSwitch = UIBuilder.addLCGroup(this, Lang.mP3Crosshair);
        UIBuilder.addVoidWebLabel(this, Lang.mP3CrosshairBlank);

        bcrosshairdisplaySwitch = UIBuilder.addLCGroup(this, Lang.mP3CrosshairDisplay);
        UIBuilder.addVoidWebLabel(this, Lang.mP3CrosshairDisplayBlank);
        UIBuilder.addVoidWebLabel(this, Lang.mP3CrosshairBlank);

        bDrawHudTextSwitch = UIBuilder.addLCGroup(this, Lang.mP3Text);
        UIBuilder.addVoidWebLabel(this, Lang.mP3TextBlank);

        bFlapBarSwitch = UIBuilder.addLCGroup(this, Lang.mP3FlapAngleBar);
        UIBuilder.addVoidWebLabel(this, Lang.mP3FlapAngleBarBlank);

        bTextureCrosshairSwitch = UIBuilder.addLCGroup(this, Lang.mP3CrosshairTexture);
        UIBuilder.addVoidWebLabel(this, Lang.mP3CrosshairTextureBlank);

        sCrosshairName = UIBuilder.addCrosshairList(this, Lang.mP3ChooseTexture, parent.isInitializing, () -> {
            fireSave();
            parent.tc.refreshPreviews();
        });
        UIBuilder.addVoidWebLabel(this, Lang.mP3ChooseTextureBlank);

        iCrosshairScale = UIBuilder.addSlider(this, Lang.mP3CrosshairSize, 0, 200, 10, 500, 5, 20);

        sMonoFont = UIBuilder.addFontComboBox(this, Lang.mP3MonoFont, Application.fonts);
        UIBuilder.addVoidWebLabel(this, Lang.mP3MonoFontBlank);

        setupListeners();
    }

    private void setupListeners() {
        bCrosshairSwitch.addActionListener(e -> fireChange());
        bcrosshairdisplaySwitch.addActionListener(e -> fireChange());
        bDrawHudTextSwitch.addActionListener(e -> fireChange());
        bFlapBarSwitch.addActionListener(e -> fireChange());
        bTextureCrosshairSwitch.addActionListener(e -> fireChange());

        iCrosshairScale.addChangeListener(e -> {
            if (!iCrosshairScale.getValueIsAdjusting()) {
                fireChange();
            }
        });

        sMonoFont.addActionListener(e -> fireChange());
    }

    public void loadConfig(ConfigurationService config) {
        bCrosshairSwitch.setSelected(Boolean.parseBoolean(config.getConfig("crosshairSwitch")));
        String scale = config.getConfig("crosshairScale");
        iCrosshairScale.setValue(Integer.parseInt(scale != null ? scale : "10"));
        bTextureCrosshairSwitch.setSelected(Boolean.parseBoolean(config.getConfig("usetexturecrosshair")));
        sCrosshairName.setSelectedItem(config.getConfig("crosshairName"));
        bDrawHudTextSwitch.setSelected(Boolean.parseBoolean(config.getConfig("drawHUDtext")));
        bFlapBarSwitch.setSelected(Boolean.parseBoolean(config.getConfig("enableFlapAngleBar")));
        bcrosshairdisplaySwitch.setSelected(Boolean.parseBoolean(config.getConfig("displayCrosshair")));
        sMonoFont.setSelectedItem(config.getConfig("MonoNumFont"));
    }

    public void saveConfig(ConfigurationService config) {
        config.setConfig("crosshairSwitch", Boolean.toString(bCrosshairSwitch.isSelected()));
        config.setConfig("crosshairScale", Integer.toString(iCrosshairScale.getValue()));
        config.setConfig("usetexturecrosshair", Boolean.toString(bTextureCrosshairSwitch.isSelected()));
        config.setConfig("crosshairName", sCrosshairName.getSelectedItem().toString());
        config.setConfig("drawHUDtext", Boolean.toString(bDrawHudTextSwitch.isSelected()));
        config.setConfig("enableFlapAngleBar", Boolean.toString(bFlapBarSwitch.isSelected()));
        config.setConfig("displayCrosshair", Boolean.toString(bcrosshairdisplaySwitch.isSelected()));
        config.setConfig("MonoNumFont", sMonoFont.getSelectedItem().toString());
    }
}
