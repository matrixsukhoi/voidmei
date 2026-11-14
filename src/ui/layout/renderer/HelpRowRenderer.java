package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;

import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;
import ui.replica.ReplicaBuilder;

/**
 * HELP 行：只读功能条目，悬浮显示 desc/desc-img，不写配置。
 * 用于 MiniHUD「功能列表」这类纯说明列表。
 */
public class HelpRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        return ReplicaBuilder.createHelpItem(row.label, row.desc, row.descImg);
    }
}
