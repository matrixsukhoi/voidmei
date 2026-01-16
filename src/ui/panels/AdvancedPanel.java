package ui.panels;

import java.awt.Color;
import java.awt.FlowLayout;

import javax.swing.SwingConstants;

import com.alee.extended.button.WebSwitch;
import com.alee.extended.image.WebImage;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;
import com.alee.laf.text.WebTextField;
import com.alee.utils.ImageUtils;

import prog.ConfigurationService;
import prog.app;
import prog.lang;
import ui.layout.UIBuilder;

public class AdvancedPanel extends WebPanel {

    // Migrated Fields
    private WebSwitch bstatusSwitch;
    private WebSwitch bdrawShadeSwitch;
    private WebSwitch bAAEnable;
    private WebComboBox sGlobalNumFont;
    private WebTextField cNumColor;
    private WebTextField cLabelColor;
    private WebTextField cUnitColor;
    private WebTextField cWarnColor;
    private WebTextField cShadeColor;
    private WebSlider iInterval;
    private WebSwitch bvoiceWarningSwitch;
    private WebSlider ivoiceVolume;
    private WebSwitch bTempInfoSwitch; // Conditional debug

    private Runnable onChangeCallback;

    public AdvancedPanel() {
        super();
        this.setOpaque(false);
        this.setBackground(new Color(0, 0, 0, 0));
        initUI();
    }

    private void initUI() {
        this.setLayout(new FlowLayout(FlowLayout.LEFT));

        if (app.debug) {
            bTempInfoSwitch = UIBuilder.addSwitch(this, lang.mP1statusBar, false);
            createvoidWebLabel(this, lang.mP1statusBarBlank);
        }

        bstatusSwitch = UIBuilder.addSwitch(this, lang.mP1statusBar, false);
        createvoidWebLabel(this, lang.mP1statusBarBlank);

        bdrawShadeSwitch = UIBuilder.addSwitch(this, lang.mP1drawFontShape, false);
        createvoidWebLabel(this, lang.mP1drawFontShapeBlank);

        bAAEnable = UIBuilder.addSwitch(this, lang.mP1AAEnable, false);
        createvoidWebLabel(this, lang.mP1AAEnableBlank);

        sGlobalNumFont = UIBuilder.addFontComboBox(this, lang.mP1GlobalNumberFont, app.fonts);
        createvoidWebLabel(this, lang.mP1GlobalNumberFontBlank);

        cNumColor = createColorGroup(this, lang.mP1NumColor);
        createvoidWebLabel(this, lang.mP1NumColorBlank);

        cLabelColor = createColorGroup(this, lang.mP1LabelColor);
        createvoidWebLabel(this, lang.mP1LabelColorBlank);

        cUnitColor = createColorGroup(this, lang.mP1UnitColor);
        createvoidWebLabel(this, lang.mP1UnitColorBlank);

        cWarnColor = createColorGroup(this, lang.mP1WarnColor);
        createvoidWebLabel(this, lang.mP1WarnColorBlank);

        cShadeColor = createColorGroup(this, lang.mP1ShadeColor);
        createvoidWebLabel(this, lang.mP1ShadeColorBlank);

        iInterval = UIBuilder.addSlider(this, lang.mP1Interval, 10, 300, 70, 500, 5, 40);

        bvoiceWarningSwitch = UIBuilder.addSwitch(this, lang.mP1VoiceWarning, false);
        createvoidWebLabel(this, lang.mP1VoiceWarningBlank);

        ivoiceVolume = UIBuilder.addSlider(this, lang.mP1voiceVolume, 0, 200, 100, 300, 10, 50);
        createvoidWebLabel(this, lang.mP1voiceVolumeBlank);

        setupListeners();
    }

    private WebTextField createColorGroup(WebPanel panel, String text) {
        WebTextField field = UIBuilder.addColorField(panel, text, "255, 255, 255, 255", Color.WHITE);
        field.addActionListener(e -> {
            updateColorGroupColor(field);
            fireChange();
        });
        return field;
    }

    private void createvoidWebLabel(WebPanel panel, String text) {
        WebLabel lb = new WebLabel(text);
        lb.setHorizontalAlignment(SwingConstants.CENTER);
        lb.setForeground(new Color(0, 0, 0, 230));
        lb.setShadeColor(Color.WHITE);
        lb.setFont(app.defaultFont);
        panel.add(lb);
    }

    private Color textToColor(String t) {
        if (t == null || t.isEmpty())
            return Color.BLACK;
        t = t.replaceAll(" ", "");
        String[] ts = t.split(",");
        if (ts.length < 4)
            return Color.BLACK;
        try {
            int R = Integer.parseInt(ts[0]);
            int G = Integer.parseInt(ts[1]);
            int B = Integer.parseInt(ts[2]);
            int A = Integer.parseInt(ts[3]);
            return new Color(R, G, B, A);
        } catch (Exception e) {
            return Color.BLACK;
        }
    }

    public void updateColorGroupColor(WebTextField trailing) {
        Color c = textToColor(trailing.getText());
        trailing.setLeadingComponent(new WebImage(ImageUtils.createColorIcon(c)));
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
        bstatusSwitch.addActionListener(e -> fireChange());
        bdrawShadeSwitch.addActionListener(e -> fireChange());
        bAAEnable.addActionListener(e -> fireChange());
        sGlobalNumFont.addActionListener(e -> fireChange());
        iInterval.addChangeListener(e -> {
            if (!iInterval.getValueIsAdjusting())
                fireChange();
        });
        bvoiceWarningSwitch.addActionListener(e -> fireChange());
        ivoiceVolume.addChangeListener(e -> {
            if (!ivoiceVolume.getValueIsAdjusting())
                fireChange();
        });
        if (bTempInfoSwitch != null)
            bTempInfoSwitch.addActionListener(e -> fireChange());
    }

    public void loadConfig(ConfigurationService cs) {
        if (app.debug && bTempInfoSwitch != null)
            bTempInfoSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("usetempInfoSwitch")));

        bstatusSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("enableStatusBar")));
        bdrawShadeSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("simpleFont")));
        bAAEnable.setSelected(Boolean.parseBoolean(cs.getConfig("AAEnable")));

        String globalFont = cs.getConfig("GlobalNumFont");
        if (globalFont != null && !globalFont.isEmpty()) {
            sGlobalNumFont.setSelectedItem(globalFont);
        }

        String interval = cs.getConfig("Interval");
        if (interval != null && !interval.isEmpty())
            iInterval.setValue(Integer.parseInt(interval));

        bvoiceWarningSwitch.setSelected(Boolean.parseBoolean(cs.getConfig("enableVoiceWarn")));
        String volume = cs.getConfig("voiceVolume");
        if (volume != null && !volume.isEmpty())
            ivoiceVolume.setValue(Integer.parseInt(volume));

        updateColorField(cNumColor, cs.getColorConfig("fontNum"));
        updateColorField(cLabelColor, cs.getColorConfig("fontLabel"));
        updateColorField(cUnitColor, cs.getColorConfig("fontUnit"));
        updateColorField(cWarnColor, cs.getColorConfig("fontWarn"));
        updateColorField(cShadeColor, cs.getColorConfig("fontShade"));
    }

    private void updateColorField(WebTextField field, Color c) {
        field.setText(getColorText(c));
        updateColorGroupColor(field);
    }

    private String getColorText(final Color color) {
        return color.getRed() + ", " + color.getGreen() + ", " + color.getBlue() + ", " + color.getAlpha();
    }

    public void saveConfig(ConfigurationService cs) {
        if (app.debug && bTempInfoSwitch != null)
            cs.setConfig("usetempInfoSwitch", Boolean.toString(bTempInfoSwitch.isSelected()));

        cs.setConfig("enableStatusBar", Boolean.toString(bstatusSwitch.isSelected()));
        cs.setConfig("simpleFont", Boolean.toString(bdrawShadeSwitch.isSelected()));
        cs.setConfig("AAEnable", Boolean.toString(bAAEnable.isSelected()));
        cs.setConfig("GlobalNumFont", sGlobalNumFont.getSelectedItem().toString());
        cs.setConfig("Interval", Integer.toString(iInterval.getValue()));
        cs.setConfig("enableVoiceWarn", Boolean.toString(bvoiceWarningSwitch.isSelected()));
        cs.setConfig("voiceVolume", Integer.toString(ivoiceVolume.getValue()));

        cs.setColorConfig("fontNum", textToColor(cNumColor.getText()));
        cs.setColorConfig("fontLabel", textToColor(cLabelColor.getText()));
        cs.setColorConfig("fontUnit", textToColor(cUnitColor.getText()));
        cs.setColorConfig("fontWarn", textToColor(cWarnColor.getText()));
        cs.setColorConfig("fontShade", textToColor(cShadeColor.getText()));
    }
}
