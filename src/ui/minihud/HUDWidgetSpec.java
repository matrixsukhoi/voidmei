package ui.minihud;

/** 条目类型元信息：右键菜单、属性编辑与缩放能力都从这里取。 */
public final class HUDWidgetSpec {
    public final String type;
    public final String category;
    public final String displayName;
    /** 是否支持“替换数据字段” */
    public final boolean dataField;
    /** 是否显示四角缩放柄 */
    public final boolean scalable;
    /** 条/仪表边缩放方向 */
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
