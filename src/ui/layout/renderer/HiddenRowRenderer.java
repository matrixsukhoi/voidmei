package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;

import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;

/**
 * 内部隐藏行：只承载配置值（如 MiniHUD 布局串），不渲染任何控件。
 * DynamicDataPage.buildContainer 对 null 返回值已做保护。
 */
public class HiddenRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        return null;
    }
}
