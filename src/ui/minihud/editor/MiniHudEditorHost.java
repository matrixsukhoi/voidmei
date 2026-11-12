package ui.minihud.editor;

import java.awt.Component;
import java.awt.Point;

import prog.config.MiniHudFieldCatalog;
import ui.minihud.MiniHudDocument;
import ui.minihud.MiniHudScene;
import ui.overlay.MinimalHUDContext;

/** 预览编辑器与 MiniHUDOverlay 的协作接口。 */
public interface MiniHudEditorHost {
    MiniHudScene getScene();

    MiniHudDocument getDoc();

    void setDoc(MiniHudDocument doc);

    MinimalHUDContext getCtx();

    boolean isPreviewMode();

    void repaintEditor();

    Point getEditorLocation();

    void setEditorLocation(int x, int y);

    void saveEditorWindowPosition();

    void commitLayout();

    void rebuildSceneFromDoc();

    /** 拖动中只重算矩形，不改窗口尺寸。 */
    void refreshLayout();

    /** 拖动/缩放期间冻结窗口尺寸，松手后再按内容适配。 */
    void setWindowFrozen(boolean frozen);

    void resetToDefaultLayout();

    MiniHudFieldCatalog getFieldCatalog();

    Component getEditorComponent();

    void setEditorEditing(boolean editing);
}
