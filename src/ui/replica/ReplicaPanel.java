package ui.replica;

import com.alee.extended.layout.VerticalFlowLayout;
import com.alee.laf.panel.WebPanel;
import ui.layout.ResponsiveGridLayout;

/**
 * Main panel that reconstructs the User's Screenshot UI 1:1.
 */
public class ReplicaPanel extends WebPanel {

    public ReplicaPanel() {
        super();
        PinkStyle style = ReplicaBuilder.getStyle();
        style.decorateMainPanel(this);

        // Layout: Vertical stack of cards
        this.setLayout(new VerticalFlowLayout(0, 10)); // 10px spacing between cards
        this.setBorder(javax.swing.BorderFactory.createEmptyBorder(10, 10, 10, 10));

        // --- Card 1: Engine (Derived from "Engine" title, mixed Online/Offline
        // sections) ---
        // Screenshot shows "Engine ?" then a box called "Offline" and "Online" inside?
        // Actually it looks like the main header is "Engine ?" and inside there are
        // groups.
        // Group 1: "Offline"
        // Group 2: "Online"

        // Let's create a Card for "Offline" and "Online" separately or nested?
        // The screenshot shows "Offline" and "Online" as labels inside a big box
        // "Engine".
        // But the border surrounds "Offline" items.
        // Let's implement the specific structure:

        // -- Engine Card -- (Concepts)
        WebPanel engineCard = style.createContainer("引擎 ?");
        // Note: The screenshot has a pink "?" next to "Engine". We put it in title for
        // now.

        WebPanel engineContent = new WebPanel(new VerticalFlowLayout(0, 10));
        engineContent.setOpaque(false);
        engineCard.add(engineContent);

        // Sub-section: Offline
        WebPanel offlineGroup = createSectionPanel("离线");
        offlineGroup.add(ReplicaBuilder.createSwitchItem("WindowsTTS", false, false));
        offlineGroup.add(ReplicaBuilder.createSwitchItem("NeoSpeech", true, false));
        offlineGroup.add(ReplicaBuilder.createSwitchItem("VoiceRoid2", true, true)); // has gear
        offlineGroup.add(ReplicaBuilder.createSwitchItem("VOICEVOX", true, true));
        offlineGroup.add(ReplicaBuilder.createSwitchItem("GPT-SoVITS", true, true));
        offlineGroup.add(ReplicaBuilder.createSwitchItem("vits-simple-api", true, true));
        engineContent.add(offlineGroup);

        // Sub-section: Online
        WebPanel onlineGroup = createSectionPanel("在线");
        // "VolcTTS" (Pink ON)
        WebPanel volPanel = ReplicaBuilder.createSwitchItem("火山TTS", true, false);
        // Re-color specific switch manually if needed? No, built-in logic handles
        // checked=Pink
        onlineGroup.add(volPanel);

        onlineGroup.add(ReplicaBuilder.createSwitchItem("BCUT TTS", false, false));
        onlineGroup.add(ReplicaBuilder.createSwitchItem("大模型通用接口", true, true));
        onlineGroup.add(ReplicaBuilder.createSwitchItem("edgeTTS", true, false));
        onlineGroup.add(ReplicaBuilder.createSwitchItem("自定义", true, false)); // Has pencil icon?
        onlineGroup.add(ReplicaBuilder.createSwitchItem("GoogleTTS", true, false));
        engineContent.add(onlineGroup);

        this.add(engineCard);

        // --- Card 2: Sound ---
        WebPanel soundCard = style.createContainer("声音");
        WebPanel soundContent = new WebPanel(new VerticalFlowLayout(0, 10));
        soundContent.setOpaque(false);
        soundCard.add(soundContent);

        // Row 1: Dropdown "Select Voice"
        // Screenshot: "Select Voice" [ jp Japanese Female ]
        soundContent.add(ReplicaBuilder.createDropdownItem("选择声音", new String[] { "jp 日语女声" }));

        // Row 2: 3 Spinners [Vol] [Speed] [Pitch]
        WebPanel spinGrid = new WebPanel(new ResponsiveGridLayout(3, 10, 5));
        spinGrid.setOpaque(false);
        spinGrid.add(ReplicaBuilder.createSpinnerItem("音量 (0~100)", 100, 0, 100, 1));
        spinGrid.add(ReplicaBuilder.createSpinnerItem("语速 (-10~10)", 0.0, -10, 10, 0.1));
        spinGrid.add(ReplicaBuilder.createSpinnerItem("音高 (-10~10)", 0.0, -10, 10, 0.1));

        soundContent.add(spinGrid);

        this.add(soundCard);

        // --- Card 3: Behavior ---
        WebPanel behaviorCard = style.createContainer("行为");
        WebPanel behGrid = new WebPanel(new ResponsiveGridLayout(3, 10, 5)); // 3 cols
        behGrid.setOpaque(false);

        behGrid.add(ReplicaBuilder.createSwitchItem("自动朗读", false, false));
        behGrid.add(ReplicaBuilder.createSwitchItem("不被打断", true, false));
        behGrid.add(ReplicaBuilder.createSwitchItem("自动前进", true, false));

        behGrid.add(ReplicaBuilder.createSwitchItem("朗读原文", true, false)); // Pink
        behGrid.add(ReplicaBuilder.createSwitchItem("朗读翻译", true, false));
        // Empty slot? Or strict layout
        behGrid.add(new WebPanel()); // Filler

        // Bottom Row: "Language Assign" "Language Correct"
        behGrid.add(ReplicaBuilder.createSwitchItem("语音指定", true, true));
        behGrid.add(ReplicaBuilder.createSwitchItem("语音修正", true, true));

        behaviorCard.add(behGrid);

        this.add(behaviorCard);
    }

    private WebPanel createSectionPanel(String title) {
        // Screenshot shows a labelled group inside the card.
        // "Offline" is just bold text above the grid?
        // Or is it a nested group? Using a titled border is easiest to replicate visual
        // grouping.

        WebPanel panel = new WebPanel();
        panel.setOpaque(false);
        panel.setLayout(new ResponsiveGridLayout(3, 20, 10)); // 3 Columns, 20px gap

        javax.swing.border.TitledBorder border = javax.swing.BorderFactory.createTitledBorder(title);
        border.setTitleFont(PinkStyle.FONT_HEADER.deriveFont(java.awt.Font.PLAIN, 12f));
        // Actually screenshot title is "Offline" in bold/large.

        panel.setBorder(border);

        return panel;
    }
}
