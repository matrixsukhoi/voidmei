package ui.layout;

import com.alee.laf.panel.WebPanel;
import ui.mainform;

public class ExamplePage extends BasePage {

    public ExamplePage(mainform parent) {
        super(parent);
    }

    @Override
    protected void initContent(WebPanel content) {
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
    }
}
