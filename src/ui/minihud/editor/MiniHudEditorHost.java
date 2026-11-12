package ui.minihud.editor;

import java.awt.Component;
import java.awt.Point;

import prog.config.MiniHudFieldCatalog;
import ui.minihud.MiniHudLayoutDoc;
import ui.minihud.MiniHudScene;
import ui.overlay.MinimalHUDContext;

/** 编辑器与宿主 overlay 的最小接口，便于离屏快照直接驱动交互。 */
public interface MiniHudEditorHost {
    MiniHudScene getScene();

    MiniHudLayoutDoc getDoc();

    MinimalHUDContext getCtx();

    boolean isPreviewMode();

    void repaintEditor();

    Point getEditorLocation();

    void setEditorLocation(int x, int y);

    void saveEditorWindowPosition();

    void commitLayout();

    /** 文档被编辑器直接修改后重建场景（不重新读配置）。 */
    void rebuildSceneFromDoc();

    /** 恢复出厂布局。 */
    void resetToDefaultLayout();

    MiniHudFieldCatalog getFieldCatalog();

    Component getEditorComponent();

    /** 编辑器进入/退出编辑态：宿主据此显示布局调试框等编辑辅助。 */
    void setEditorEditing(boolean editing);
}
