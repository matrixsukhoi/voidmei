package ui.minihud;

import ui.component.HUDComponent;

/** 条目类型工厂：按条目属性创建组件实例（多实例各自独立）。 */
public interface HUDWidgetFactory {
    HUDComponent create(WidgetBuildContext ctx);
}
