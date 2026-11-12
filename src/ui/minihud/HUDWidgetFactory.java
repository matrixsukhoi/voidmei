package ui.minihud;

import ui.component.HUDComponent;

/** 原子组件工厂：每次调用创建一个独立实例，多实例互不共享状态。 */
public interface HUDWidgetFactory {
    HUDComponent create(WidgetBuildContext ctx);
}
