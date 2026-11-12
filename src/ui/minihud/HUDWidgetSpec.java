package ui.minihud;

/**
 * 原子组件类型元信息。注册表只登记原子组件；
 * “行/预设”是编辑器宏，不是组件类型。
 */
public final class HUDWidgetSpec {
    public final String type;
    public final String category;
    public final String displayName;
    public final boolean dataField;
    public final boolean scalable;
    public final EdgeResize edgeResize;
    public final HUDWidgetFactory factory;

    public HUDWidgetSpec(String type, String category, String displayName, boolean dataField,
            boolean scalable, EdgeResize edgeResize, HUDWidgetFactory factory) {
        this.type = type;
        this.category = category;
        this.displayName = displayName;
        this.dataField = dataField;
        this.scalable = scalable;
        this.edgeResize = edgeResize;
        this.factory = factory;
    }
}
