package ui.minihud.editor;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Component;
import java.awt.Font;
import java.awt.Graphics2D;
import java.awt.Point;
import java.awt.Rectangle;
import java.awt.event.KeyAdapter;
import java.awt.event.KeyEvent;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Deque;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

import javax.swing.JCheckBox;
import javax.swing.JComboBox;
import javax.swing.JMenu;
import javax.swing.JMenuItem;
import javax.swing.JOptionPane;
import javax.swing.JPanel;
import javax.swing.JPopupMenu;
import javax.swing.JTextField;

import prog.config.MiniHudFieldCatalog;
import ui.minihud.HUDWidgetSpec;
import ui.minihud.HUDWidgetRegistry;
import ui.minihud.MiniHudDocument;
import ui.minihud.MiniHudGroup;
import ui.minihud.MiniHudPresets;
import ui.minihud.MiniHudProps;
import ui.minihud.MiniHudScene;
import ui.minihud.MiniHudWidget;

/**
 * 预览态自由画布编辑器。
 * 默认只读；进入编辑后支持多选、拖动、缩放、吸附加参考线、
 * 显式分组、图层、属性、增删与撤销/重做。游戏态不安装。
 */
public final class MiniHudEditor {

    private static final Color SELECT_COLOR = new Color(255, 200, 40, 230);
    private static final Color LOCK_COLOR = new Color(150, 150, 150, 200);
    private static final Color HIDDEN_COLOR = new Color(160, 160, 160, 120);
    private static final Color GUIDE_COLOR = new Color(80, 200, 255, 180);
    private static final Color MARQUEE_COLOR = new Color(80, 200, 255, 200);
    private static final Color TOOLBAR_BG = new Color(0, 0, 0, 160);
    private static final Color TOOLBAR_FG = new Color(220, 220, 220, 230);
    private static final BasicStroke SELECT_STROKE = new BasicStroke(1.5f, BasicStroke.CAP_BUTT,
            BasicStroke.JOIN_MITER, 10f, new float[] { 5f, 4f }, 0f);
    private static final BasicStroke HIDDEN_STROKE = new BasicStroke(1f, BasicStroke.CAP_BUTT,
            BasicStroke.JOIN_MITER, 10f, new float[] { 3f, 3f }, 0f);
    private static final BasicStroke GUIDE_STROKE = new BasicStroke(1f);
    private static final double SNAP = 0.1;
    private static final int HANDLE = 10;
    private static final double MIN_SCALE = MiniHudDocument.MIN_SCALE;
    private static final double MAX_SCALE = MiniHudDocument.MAX_SCALE;
    private static final int UNDO_LIMIT = 100;

    private final MiniHudEditorHost host;
    private Component target;
    private MouseAdapter mouseAdapter;
    private MouseMotionAdapter motionAdapter;
    private KeyAdapter keyAdapter;

    private final LinkedHashSet<String> selected = new LinkedHashSet<>();
    private String primaryId;
    private boolean readOnly = true;

    private boolean dragging;
    private boolean moved;
    private int pressX;
    private int pressY;
    private final java.util.Map<String, double[]> dragStart = new java.util.HashMap<>();
    private MiniHudDocument undoBeforeDrag;

    private boolean resizing;
    private String resizeId;
    private final java.util.Map<String, Double> resizeStart = new java.util.HashMap<>();
    private Rectangle resizePreview;

    private boolean marquee;
    private int marqueeX0;
    private int marqueeY0;
    private Rectangle marqueeRect;

    private boolean windowDrag;
    private int startWindowX;
    private int startWindowY;
    private int startScreenX;
    private int startScreenY;

    private boolean arrowDirty;
    private long lastMenuMs;

    private final Deque<MiniHudDocument> undoStack = new ArrayDeque<>();
    private final Deque<MiniHudDocument> redoStack = new ArrayDeque<>();

    private final Rectangle tbEdit = new Rectangle();
    private final Rectangle tbAdd = new Rectangle();
    private final Rectangle tbUndo = new Rectangle();
    private final Rectangle tbRedo = new Rectangle();
    private final Rectangle tbLayers = new Rectangle();
    private final Rectangle tbReset = new Rectangle();
    private Font toolbarFont;
    private int toolbarFontSize = -1;

    public MiniHudEditor(MiniHudEditorHost host) {
        this.host = host;
    }

    public void install(Component c) {
        this.target = c;
        c.setFocusable(true);
        this.mouseAdapter = new MouseAdapter() {
            @Override
            public void mousePressed(MouseEvent e) {
                onPressed(e);
            }

            @Override
            public void mouseReleased(MouseEvent e) {
                onReleased(e);
            }

            @Override
            public void mouseClicked(MouseEvent e) {
                if (e.getClickCount() == 2 && !readOnly)
                    onDoubleClick(e);
            }
        };
        this.motionAdapter = new MouseMotionAdapter() {
            @Override
            public void mouseDragged(MouseEvent e) {
                onDragged(e);
            }
        };
        this.keyAdapter = new KeyAdapter() {
            @Override
            public void keyPressed(KeyEvent e) {
                onKey(e);
            }

            @Override
            public void keyReleased(KeyEvent e) {
                if (arrowDirty) {
                    arrowDirty = false;
                    host.commitLayout();
                }
            }
        };
        c.addMouseListener(mouseAdapter);
        c.addMouseMotionListener(motionAdapter);
        c.addKeyListener(keyAdapter);
    }

    public void dispose() {
        if (target != null) {
            if (mouseAdapter != null)
                target.removeMouseListener(mouseAdapter);
            if (motionAdapter != null)
                target.removeMouseMotionListener(motionAdapter);
            if (keyAdapter != null)
                target.removeKeyListener(keyAdapter);
            target = null;
        }
        clearSelection();
        dragging = false;
        resizing = false;
        marquee = false;
        windowDrag = false;
        if (arrowDirty) {
            arrowDirty = false;
            host.commitLayout();
        }
    }

    public boolean isDragging() {
        return dragging || resizing;
    }

    public boolean isReadOnly() {
        return readOnly;
    }

    public Set<String> getSelectedIds() {
        return new LinkedHashSet<>(selected);
    }

    public String getPrimaryId() {
        return primaryId;
    }

    public void selectOnly(String id) {
        selected.clear();
        if (id != null)
            selected.add(id);
        primaryId = id;
        host.repaintEditor();
    }

    private void clearSelection() {
        selected.clear();
        primaryId = null;
    }

    private void setReadOnly(boolean v) {
        this.readOnly = v;
        host.setEditorEditing(!v);
        if (v) {
            clearSelection();
            tbAdd.setBounds(0, 0, 0, 0);
            tbUndo.setBounds(0, 0, 0, 0);
            tbRedo.setBounds(0, 0, 0, 0);
            tbLayers.setBounds(0, 0, 0, 0);
            tbReset.setBounds(0, 0, 0, 0);
            if (arrowDirty) {
                arrowDirty = false;
                host.commitLayout();
            }
        }
        host.repaintEditor();
    }

    // --- 鼠标 ---

    private void onPressed(MouseEvent e) {
        if (!host.isPreviewMode() || host.getScene() == null)
            return;
        focusWindow();
        if (onToolbar(e.getX(), e.getY()))
            return;

        if (e.isPopupTrigger()) {
            if (readOnly)
                return;
            MiniHudScene.Runtime hit = host.getScene().hitTest(e.getX(), e.getY(), true);
            if (hit != null && !selected.contains(hit.widget.id))
                selectOnly(hit.widget.id);
            else if (hit == null)
                clearSelection();
            host.repaintEditor();
            showMenu(e);
            return;
        }
        if (javax.swing.SwingUtilities.isRightMouseButton(e))
            return;

        pressX = e.getX();
        pressY = e.getY();
        moved = false;

        if (readOnly) {
            startWindowDrag(e);
            return;
        }

        // 编辑态：中键拖动 = 移动整个窗口；Alt+拖动空白也移窗
        if (javax.swing.SwingUtilities.isMiddleMouseButton(e)) {
            startWindowDrag(e);
            return;
        }

        MiniHudScene.Runtime hit = host.getScene().hitTest(e.getX(), e.getY(), true);
        if (hit == null) {
            if (e.isAltDown()) {
                startWindowDrag(e);
                return;
            }
            if (!e.isShiftDown())
                clearSelection();
            marquee = true;
            marqueeX0 = e.getX();
            marqueeY0 = e.getY();
            marqueeRect = new Rectangle(e.getX(), e.getY(), 0, 0);
            host.repaintEditor();
            return;
        }

        MiniHudWidget w = hit.widget;
        if (e.isShiftDown()) {
            if (!selected.add(w.id)) {
                selected.remove(w.id);
                if (w.id.equals(primaryId))
                    primaryId = selected.isEmpty() ? null : selected.iterator().next();
            } else {
                primaryId = w.id;
            }
            host.repaintEditor();
            return;
        }
        if (!selected.contains(w.id)) {
            selected.clear();
            selected.add(w.id);
            primaryId = w.id;
        } else {
            primaryId = w.id;
        }
        host.repaintEditor();

        MiniHudScene.Runtime primary = host.getScene().runtime(primaryId);
        if (primary != null && nearHandle(primary, e.getX(), e.getY()) && !w.locked && isScalable(w)) {
            startResize();
        } else if (!w.locked) {
            startDrag();
        }
    }

    private void focusWindow() {
        if (target == null)
            return;
        java.awt.Window win = javax.swing.SwingUtilities.getWindowAncestor(target);
        if (win != null) {
            win.setFocusableWindowState(true);
            win.setFocusable(true);
        }
        target.requestFocusInWindow();
    }

    private void onDoubleClick(MouseEvent e) {
        if (host.getScene() == null)
            return;
        MiniHudScene.Runtime hit = host.getScene().hitTest(e.getX(), e.getY(), true);
        if (hit == null || hit.widget.groupId == null)
            return;
        selected.clear();
        for (MiniHudWidget w : host.getDoc().membersOf(hit.widget.groupId))
            selected.add(w.id);
        primaryId = hit.widget.id;
        host.repaintEditor();
    }

    private void startDrag() {
        dragging = true;
        dragStart.clear();
        undoBeforeDrag = host.getDoc().copy();
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null && !w.locked)
                dragStart.put(id, new double[] { w.x, w.y });
        }
        host.setWindowFrozen(true);
    }

    private void startResize() {
        resizing = true;
        resizeId = primaryId;
        resizeStart.clear();
        undoBeforeDrag = host.getDoc().copy();
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null && !w.locked && isScalable(w))
                resizeStart.put(id, w.scale);
        }
        MiniHudScene.Runtime rt = host.getScene().runtime(resizeId);
        if (rt != null)
            resizePreview = new Rectangle(rt.rect);
        host.setWindowFrozen(true);
    }

    private void startWindowDrag(MouseEvent e) {
        windowDrag = true;
        Point loc = host.getEditorLocation();
        startWindowX = loc.x;
        startWindowY = loc.y;
        startScreenX = e.getXOnScreen();
        startScreenY = e.getYOnScreen();
    }

    private void onDragged(MouseEvent e) {
        if (!host.isPreviewMode())
            return;
        if (resizing) {
            int dx = e.getX() - pressX;
            int dy = e.getY() - pressY;
            MiniHudScene.Runtime rt = host.getScene().runtime(resizeId);
            if (rt != null) {
                resizePreview = new Rectangle(rt.rect.x, rt.rect.y, Math.max(4, rt.rect.width + dx),
                        Math.max(4, rt.rect.height + dy));
            }
            host.repaintEditor();
            return;
        }
        if (dragging) {
            int dx = e.getX() - pressX;
            int dy = e.getY() - pressY;
            if (!moved && Math.abs(dx) + Math.abs(dy) < 2)
                return;
            moved = true;
            double unit = Math.max(1, host.getCtx().hudFontSize);
            double dux = dx / unit;
            double duy = dy / unit;
            if (!e.isAltDown()) {
                dux = Math.round(dux / SNAP) * SNAP;
                duy = Math.round(duy / SNAP) * SNAP;
            }
            for (java.util.Map.Entry<String, double[]> en : dragStart.entrySet()) {
                MiniHudWidget w = host.getDoc().find(en.getKey());
                if (w != null) {
                    w.x = en.getValue()[0] + dux;
                    w.y = en.getValue()[1] + duy;
                }
            }
            host.refreshLayout();
            return;
        }
        if (marquee) {
            marqueeRect = new Rectangle(Math.min(marqueeX0, e.getX()), Math.min(marqueeY0, e.getY()),
                    Math.abs(e.getX() - marqueeX0), Math.abs(e.getY() - marqueeY0));
            host.repaintEditor();
            return;
        }
        if (windowDrag) {
            int nx = startWindowX + (e.getXOnScreen() - startScreenX);
            int ny = startWindowY + (e.getYOnScreen() - startScreenY);
            host.setEditorLocation(nx, ny);
            host.repaintEditor();
        }
    }

    private void onReleased(MouseEvent e) {
        if (e.isPopupTrigger()) {
            if (readOnly)
                return;
            MiniHudScene.Runtime hit = host.getScene().hitTest(e.getX(), e.getY(), true);
            if (hit != null)
                selectOnly(hit.widget.id);
            else
                clearSelection();
            host.repaintEditor();
            showMenu(e);
            return;
        }
        if (resizing) {
            applyResize();
            resizing = false;
            resizeId = null;
            resizePreview = null;
            if (moved) {
                finishOperation(true);
            } else {
                undoBeforeDrag = null;
                host.setWindowFrozen(false);
            }
            return;
        }
        if (dragging) {
            dragging = false;
            if (moved) {
                finishOperation(false);
            } else {
                undoBeforeDrag = null;
                host.setWindowFrozen(false);
            }
            return;
        }
        if (marquee) {
            marquee = false;
            if (marqueeRect != null && marqueeRect.width + marqueeRect.height > 3) {
                if (!e.isShiftDown())
                    selected.clear();
                for (MiniHudScene.Runtime rt : host.getScene().runtimes) {
                    if (marqueeRect.intersects(hitRect(rt)))
                        selected.add(rt.widget.id);
                }
                primaryId = selected.isEmpty() ? null : selected.iterator().next();
            }
            marqueeRect = null;
            host.repaintEditor();
            return;
        }
        if (windowDrag) {
            windowDrag = false;
            host.saveEditorWindowPosition();
        }
    }

    private void applyResize() {
        if (resizePreview == null || resizeId == null)
            return;
        MiniHudScene.Runtime rt = host.getScene().runtime(resizeId);
        if (rt == null)
            return;
        int dw = resizePreview.width - rt.rect.width;
        int dh = resizePreview.height - rt.rect.height;
        double unit = Math.max(1, host.getCtx().hudFontSize);
        double delta = (dw + dh) / (2.0 * unit);
        boolean changed = false;
        for (java.util.Map.Entry<String, Double> en : resizeStart.entrySet()) {
            MiniHudWidget w = host.getDoc().find(en.getKey());
            if (w == null)
                continue;
            double s = en.getValue() + delta;
            s = Math.max(MIN_SCALE, Math.min(MAX_SCALE, s));
            s = Math.round(s * 20.0) / 20.0;
            if (Math.abs(s - w.scale) > 1e-6) {
                w.scale = s;
                changed = true;
            }
        }
        moved = changed;
    }

    // --- 键盘 ---

    private void onKey(KeyEvent e) {
        if (!host.isPreviewMode() || readOnly)
            return;
        int code = e.getKeyCode();
        if (e.isControlDown() && code == KeyEvent.VK_Z) {
            if (e.isShiftDown())
                redo();
            else
                undo();
            e.consume();
            return;
        }
        if (e.isControlDown() && code == KeyEvent.VK_Y) {
            redo();
            e.consume();
            return;
        }
        if (e.isControlDown() && code == KeyEvent.VK_A) {
            selected.clear();
            for (MiniHudWidget w : host.getDoc().widgets)
                selected.add(w.id);
            primaryId = selected.isEmpty() ? null : selected.iterator().next();
            host.repaintEditor();
            e.consume();
            return;
        }
        if (e.isControlDown() && code == KeyEvent.VK_G) {
            if (e.isShiftDown())
                ungroupSelected();
            else
                groupSelected();
            e.consume();
            return;
        }
        if (e.isControlDown() && code == KeyEvent.VK_D) {
            duplicateSelected();
            e.consume();
            return;
        }
        if (code == KeyEvent.VK_ESCAPE) {
            clearSelection();
            host.repaintEditor();
            return;
        }
        if (code == KeyEvent.VK_DELETE) {
            deleteSelected();
            e.consume();
            return;
        }
        double step = e.isShiftDown() ? 1.0 : 0.1;
        double dx = 0, dy = 0;
        if (code == KeyEvent.VK_LEFT)
            dx = -step;
        else if (code == KeyEvent.VK_RIGHT)
            dx = step;
        else if (code == KeyEvent.VK_UP)
            dy = -step;
        else if (code == KeyEvent.VK_DOWN)
            dy = step;
        if (dx != 0 || dy != 0) {
            boolean any = false;
            for (String id : selected) {
                MiniHudWidget w = host.getDoc().find(id);
                if (w != null && !w.locked) {
                    w.x += dx;
                    w.y += dy;
                    any = true;
                }
            }
            if (any) {
                host.refreshLayout();
                arrowDirty = true;
                host.repaintEditor();
            }
            e.consume();
        }
    }

    // --- 工具条 ---

    private boolean onToolbar(int x, int y) {
        if (tbEdit.contains(x, y)) {
            setReadOnly(!readOnly);
            return true;
        }
        if (tbAdd.contains(x, y)) {
            showAddMenu(x, y, null);
            return true;
        }
        if (tbUndo.contains(x, y)) {
            undo();
            return true;
        }
        if (tbRedo.contains(x, y)) {
            redo();
            return true;
        }
        if (tbLayers.contains(x, y)) {
            showLayersMenu(x, y);
            return true;
        }
        if (tbReset.contains(x, y)) {
            pushUndo();
            host.resetToDefaultLayout();
            clearSelection();
            host.repaintEditor();
            return true;
        }
        return false;
    }

    // --- 选择/几何 ---

    private Rectangle screenRect(MiniHudScene.Runtime rt) {
        return new Rectangle(rt.rect.x + host.getScene().getOffsetX(),
                rt.rect.y + host.getScene().getOffsetY(), rt.rect.width, rt.rect.height);
    }

    private Rectangle hitRect(MiniHudScene.Runtime rt) {
        Rectangle r = screenRect(rt);
        if (rt.component instanceof ui.minihud.widget.BarWidget) {
            int pad = ((ui.minihud.widget.BarWidget) rt.component).getHitPadLeft();
            r.x -= pad;
            r.width += pad;
        }
        return r;
    }

    private boolean nearHandle(MiniHudScene.Runtime rt, int x, int y) {
        Rectangle r = screenRect(rt);
        return Math.abs(x - (r.x + r.width)) <= HANDLE && Math.abs(y - (r.y + r.height)) <= HANDLE;
    }

    private boolean isScalable(MiniHudWidget w) {
        HUDWidgetSpec spec = HUDWidgetRegistry.find(w.type);
        return spec == null || spec.scalable;
    }

    private double viewCenterXUnits() {
        Rectangle b = host.getScene().bounds(false);
        int unit = Math.max(1, host.getCtx().hudFontSize);
        return (b.x + b.width / 2.0) / unit - 2;
    }

    private double viewCenterYUnits() {
        Rectangle b = host.getScene().bounds(false);
        int unit = Math.max(1, host.getCtx().hudFontSize);
        return (b.y + b.height / 2.0) / unit;
    }

    // --- 编辑操作 ---

    private void pushUndo() {
        undoStack.push(host.getDoc().copy());
        if (undoStack.size() > UNDO_LIMIT)
            undoStack.removeLast();
        redoStack.clear();
    }

    public void undo() {
        if (undoStack.isEmpty())
            return;
        redoStack.push(host.getDoc().copy());
        host.setDoc(undoStack.pop());
        clearSelection();
        host.rebuildSceneFromDoc();
        host.commitLayout();
        host.repaintEditor();
    }

    public void redo() {
        if (redoStack.isEmpty())
            return;
        undoStack.push(host.getDoc().copy());
        host.setDoc(redoStack.pop());
        clearSelection();
        host.rebuildSceneFromDoc();
        host.commitLayout();
        host.repaintEditor();
    }

    /** 一次操作结束：记撤销、重建场景、落盘。 */
    private void finishOperation(boolean rebuild) {
        if (undoBeforeDrag != null) {
            undoStack.push(undoBeforeDrag);
            if (undoStack.size() > UNDO_LIMIT)
                undoStack.removeLast();
            redoStack.clear();
            undoBeforeDrag = null;
        }
        host.setWindowFrozen(false);
        if (rebuild)
            host.rebuildSceneFromDoc();
        host.commitLayout();
        host.repaintEditor();
    }

    private void afterSimpleChange() {
        host.rebuildSceneFromDoc();
        host.commitLayout();
        host.repaintEditor();
    }

    public void addPreset(MiniHudPresets.Preset preset, Double clickX, Double clickY) {
        pushUndo();
        double x = clickX != null ? clickX : viewCenterXUnits();
        double y = clickY != null ? clickY : viewCenterYUnits();
        MiniHudGroup g = MiniHudPresets.insert(host.getDoc(), preset, x, y);
        selected.clear();
        primaryId = null;
        for (MiniHudWidget w : host.getDoc().membersOf(g.id)) {
            selected.add(w.id);
            if (primaryId == null)
                primaryId = w.id;
        }
        afterSimpleChange();
    }

    public void addStaticText(String text, Double clickX, Double clickY) {
        pushUndo();
        MiniHudWidget w = MiniHudPresets.addStaticText(host.getDoc(), text,
                clickX != null ? clickX : viewCenterXUnits(),
                clickY != null ? clickY : viewCenterYUnits());
        selectOnly(w.id);
        afterSimpleChange();
    }

    public void addDataField(String fieldKey, Double clickX, Double clickY) {
        if (fieldKey == null || fieldKey.isEmpty())
            return;
        pushUndo();
        MiniHudWidget w = MiniHudPresets.addTextField(host.getDoc(), fieldKey,
                clickX != null ? clickX : viewCenterXUnits(),
                clickY != null ? clickY : viewCenterYUnits(), host.getFieldCatalog());
        selectOnly(w.id);
        afterSimpleChange();
    }

    public void addBar(String barSource, Double clickX, Double clickY) {
        pushUndo();
        MiniHudWidget w = MiniHudPresets.addBar(host.getDoc(), barSource,
                clickX != null ? clickX : viewCenterXUnits(),
                clickY != null ? clickY : viewCenterYUnits());
        selectOnly(w.id);
        afterSimpleChange();
    }

    public void addCompass(Double clickX, Double clickY) {
        pushUndo();
        MiniHudWidget w = MiniHudPresets.addCompass(host.getDoc(),
                clickX != null ? clickX : viewCenterXUnits(),
                clickY != null ? clickY : viewCenterYUnits());
        selectOnly(w.id);
        afterSimpleChange();
    }

    public void addAttitude(Double clickX, Double clickY) {
        pushUndo();
        MiniHudWidget w = MiniHudPresets.addAttitude(host.getDoc(),
                clickX != null ? clickX : viewCenterXUnits(),
                clickY != null ? clickY : viewCenterYUnits());
        selectOnly(w.id);
        afterSimpleChange();
    }

    public void addPanel(Double clickX, Double clickY) {
        pushUndo();
        MiniHudWidget w = MiniHudPresets.addPanel(host.getDoc(),
                clickX != null ? clickX : viewCenterXUnits(),
                clickY != null ? clickY : viewCenterYUnits());
        selectOnly(w.id);
        afterSimpleChange();
    }

    public void deleteSelected() {
        if (selected.isEmpty())
            return;
        boolean any = false;
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null && !w.locked) {
                any = true;
                break;
            }
        }
        if (!any)
            return;
        if (selected.size() > 1) {
            Component c = host.getEditorComponent();
            if (c != null && !java.awt.GraphicsEnvironment.isHeadless()) {
                int r = JOptionPane.showConfirmDialog(c, "删除选中的 " + selected.size() + " 个组件？",
                        "删除组件", JOptionPane.OK_CANCEL_OPTION);
                if (r != JOptionPane.OK_OPTION)
                    return;
            }
        }
        pushUndo();
        for (String id : new ArrayList<>(selected)) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null && !w.locked)
                host.getDoc().remove(id);
        }
        clearSelection();
        afterSimpleChange();
    }

    public void duplicateSelected() {
        if (selected.isEmpty())
            return;
        pushUndo();
        List<String> newIds = new ArrayList<>();
        boolean anyGroup = false;
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null && w.groupId != null) {
                anyGroup = true;
                break;
            }
        }
        MiniHudGroup copyGroup = anyGroup
                ? new MiniHudGroup(host.getDoc().nextGroupId(), "副本")
                : null;
        if (copyGroup != null)
            host.getDoc().groups.add(copyGroup);
        for (String id : new ArrayList<>(selected)) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w == null)
                continue;
            MiniHudWidget copy = w.copy(host.getDoc().nextWidgetId());
            copy.x += 1.0;
            copy.y += 1.0;
            copy.z = w.z + 1;
            if (copyGroup != null)
                copy.groupId = copyGroup.id;
            host.getDoc().widgets.add(copy);
            newIds.add(copy.id);
        }
        selected.clear();
        selected.addAll(newIds);
        primaryId = newIds.isEmpty() ? null : newIds.get(0);
        afterSimpleChange();
    }

    public void groupSelected() {
        if (selected.size() < 2)
            return;
        pushUndo();
        MiniHudGroup g = new MiniHudGroup(host.getDoc().nextGroupId(), "组 " + selected.size());
        host.getDoc().groups.add(g);
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null)
                w.groupId = g.id;
        }
        afterSimpleChange();
    }

    public void ungroupSelected() {
        boolean any = false;
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null && w.groupId != null) {
                any = true;
                break;
            }
        }
        if (!any)
            return;
        pushUndo();
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null)
                w.groupId = null;
        }
        host.getDoc().validate();
        afterSimpleChange();
    }

    public void toggleSelectedVisible() {
        if (primaryId == null)
            return;
        MiniHudWidget w = host.getDoc().find(primaryId);
        if (w == null)
            return;
        pushUndo();
        w.visible = !w.visible;
        afterSimpleChange();
    }

    public void toggleSelectedLocked() {
        if (primaryId == null)
            return;
        MiniHudWidget w = host.getDoc().find(primaryId);
        if (w == null)
            return;
        pushUndo();
        w.locked = !w.locked;
        afterSimpleChange();
    }

    public void bringToFront() {
        if (selected.isEmpty())
            return;
        pushUndo();
        int z = maxZ() + 1;
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null)
                w.z = z++;
        }
        afterSimpleChange();
    }

    public void sendToBack() {
        if (selected.isEmpty())
            return;
        pushUndo();
        int z = minZ() - selected.size();
        for (String id : selected) {
            MiniHudWidget w = host.getDoc().find(id);
            if (w != null)
                w.z = z++;
        }
        afterSimpleChange();
    }

    private int maxZ() {
        int max = 0;
        for (MiniHudWidget w : host.getDoc().widgets)
            max = Math.max(max, w.z);
        return max;
    }

    private int minZ() {
        int min = 0;
        for (MiniHudWidget w : host.getDoc().widgets)
            min = Math.min(min, w.z);
        return min;
    }

    public void alignSelected(String mode) {
        if (selected.size() < 2)
            return;
        List<MiniHudScene.Runtime> rts = new ArrayList<>();
        double minX = Double.MAX_VALUE, minY = Double.MAX_VALUE, maxX = -Double.MAX_VALUE, maxY = -Double.MAX_VALUE;
        int unit = Math.max(1, host.getCtx().hudFontSize);
        for (String id : selected) {
            MiniHudScene.Runtime rt = host.getScene().runtime(id);
            if (rt == null)
                continue;
            rts.add(rt);
            minX = Math.min(minX, rt.rect.x);
            minY = Math.min(minY, rt.rect.y);
            maxX = Math.max(maxX, rt.rect.x + rt.rect.width);
            maxY = Math.max(maxY, rt.rect.y + rt.rect.height);
        }
        if (rts.isEmpty())
            return;
        pushUndo();
        for (MiniHudScene.Runtime rt : rts) {
            MiniHudWidget w = rt.widget;
            if (w.locked)
                continue;
            switch (mode) {
                case "left":
                    w.x = minX / unit;
                    break;
                case "center":
                    w.x = (minX + maxX - rt.rect.width) / 2.0 / unit;
                    break;
                case "right":
                    w.x = (maxX - rt.rect.width) / (double) unit;
                    break;
                case "top":
                    w.y = minY / unit;
                    break;
                case "middle":
                    w.y = (minY + maxY - rt.rect.height) / 2.0 / unit;
                    break;
                case "bottom":
                    w.y = (maxY - rt.rect.height) / (double) unit;
                    break;
                default:
                    break;
            }
        }
        afterSimpleChange();
    }

    // --- 菜单 ---

    private void showMenu(MouseEvent e) {
        long now = System.currentTimeMillis();
        if (now - lastMenuMs < 150)
            return;
        lastMenuMs = now;
        JPopupMenu menu = new JPopupMenu();
        if (selected.isEmpty()) {
            int unit = Math.max(1, host.getCtx().hudFontSize);
            double ux = (e.getX() - host.getScene().getOffsetX()) / (double) unit;
            double uy = (e.getY() - host.getScene().getOffsetY()) / (double) unit;
            menu.add(buildAddMenu(ux, uy));
            menu.addSeparator();
            JMenuItem all = new JMenuItem("全选");
            all.addActionListener(ev -> {
                selected.clear();
                for (MiniHudWidget w : host.getDoc().widgets)
                    selected.add(w.id);
                primaryId = selected.isEmpty() ? null : selected.iterator().next();
                host.repaintEditor();
            });
            menu.add(all);
        } else {
            MiniHudWidget sel = host.getDoc().find(primaryId);
            JMenuItem props = new JMenuItem("组件属性…");
            props.addActionListener(ev -> editSelectedProperties());
            menu.add(props);
            if (sel != null && MiniHudProps.TYPE_TEXT.equals(sel.type)) {
                menu.add(buildFieldMenu("替换为数据字段", this::replaceSelectedField));
            }
            menu.addSeparator();
            JMenuItem dup = new JMenuItem("复制");
            dup.addActionListener(ev -> duplicateSelected());
            menu.add(dup);
            JMenuItem del = new JMenuItem("删除");
            del.addActionListener(ev -> deleteSelected());
            menu.add(del);
            JMenuItem group = new JMenuItem("成组");
            group.addActionListener(ev -> groupSelected());
            menu.add(group);
            JMenuItem ungroup = new JMenuItem("解组");
            ungroup.addActionListener(ev -> ungroupSelected());
            menu.add(ungroup);
            JMenuItem selectGroup = new JMenuItem("选择同组");
            selectGroup.addActionListener(ev -> selectSameGroup());
            menu.add(selectGroup);
            menu.addSeparator();
            JMenuItem vis = new JMenuItem(sel != null && sel.visible ? "隐藏" : "显示");
            vis.addActionListener(ev -> toggleSelectedVisible());
            menu.add(vis);
            JMenuItem lock = new JMenuItem(sel != null && sel.locked ? "解锁" : "锁定");
            lock.addActionListener(ev -> toggleSelectedLocked());
            menu.add(lock);
            menu.addSeparator();
            JMenuItem front = new JMenuItem("置顶");
            front.addActionListener(ev -> bringToFront());
            menu.add(front);
            JMenuItem back = new JMenuItem("置底");
            back.addActionListener(ev -> sendToBack());
            menu.add(back);
            if (selected.size() > 1) {
                JMenu align = new JMenu("对齐");
                addAlignItem(align, "左对齐", "left");
                addAlignItem(align, "水平居中", "center");
                addAlignItem(align, "右对齐", "right");
                addAlignItem(align, "顶对齐", "top");
                addAlignItem(align, "垂直居中", "middle");
                addAlignItem(align, "底对齐", "bottom");
                menu.add(align);
            }
            menu.addSeparator();
            menu.add(buildLayersMenu());
        }
        Component c = host.getEditorComponent();
        if (c != null)
            menu.show(c, e.getX(), e.getY());
    }

    private void addAlignItem(JMenu menu, String label, String mode) {
        JMenuItem item = new JMenuItem(label);
        item.addActionListener(ev -> alignSelected(mode));
        menu.add(item);
    }

    private void selectSameGroup() {
        MiniHudWidget w = host.getDoc().find(primaryId);
        if (w == null || w.groupId == null)
            return;
        selected.clear();
        for (MiniHudWidget m : host.getDoc().membersOf(w.groupId))
            selected.add(m.id);
        primaryId = w.id;
        host.repaintEditor();
    }

    private void showAddMenu(int x, int y, Double clickX) {
        JPopupMenu menu = new JPopupMenu();
        menu.add(buildAddMenu(clickX, null));
        Component c = host.getEditorComponent();
        if (c != null)
            menu.show(c, x, y);
    }

    private JMenu buildAddMenu(Double clickX, Double clickY) {
        JMenu add = new JMenu("添加组件");

        JMenu data = new JMenu("数据字段");
        String lastGroup = null;
        JMenu group = null;
        for (MiniHudFieldCatalog.FieldDef def : host.getFieldCatalog().all()) {
            String g = def.group != null ? def.group : "其他";
            if (!g.equals(lastGroup)) {
                group = new JMenu(g);
                data.add(group);
                lastGroup = g;
            }
            JMenuItem item = new JMenuItem(def.label + "  " + def.key);
            item.addActionListener(ev -> addDataField(def.key, clickX, clickY));
            group.add(item);
        }
        if (data.getItemCount() == 0) {
            JMenuItem none = new JMenuItem("（无可用字段）");
            none.setEnabled(false);
            data.add(none);
        }
        add.add(data);

        JMenuItem text = new JMenuItem("静态文本…");
        text.addActionListener(ev -> {
            String v = JOptionPane.showInputDialog(host.getEditorComponent(), "文本内容", "");
            if (v != null)
                addStaticText(v, clickX, clickY);
        });
        add.add(text);

        JMenu gauges = new JMenu("条与仪表");
        addBarItem(gauges, "襟翼角条", MiniHudProps.BAR_FLAP_ANGLE, clickX, clickY);
        addBarItem(gauges, "速度条", MiniHudProps.BAR_SPEED_RATIO, clickX, clickY);
        addBarItem(gauges, "油门条", MiniHudProps.BAR_THROTTLE, clickX, clickY);
        addBarItem(gauges, "AoA 条", MiniHudProps.BAR_AOA, clickX, clickY);
        addBarItem(gauges, "机动条", MiniHudProps.BAR_MANEUVER, clickX, clickY);
        add.add(gauges);

        JMenuItem compass = new JMenuItem("罗盘");
        compass.addActionListener(ev -> addCompass(clickX, clickY));
        add.add(compass);
        JMenuItem attitude = new JMenuItem("姿态仪");
        attitude.addActionListener(ev -> addAttitude(clickX, clickY));
        add.add(attitude);
        JMenuItem panel = new JMenuItem("背景板");
        panel.addActionListener(ev -> addPanel(clickX, clickY));
        add.add(panel);

        add.addSeparator();
        JMenu presets = new JMenu("预设（原子组件组）");
        for (MiniHudPresets.Preset p : MiniHudPresets.Preset.values()) {
            JMenuItem item = new JMenuItem(p.label);
            item.addActionListener(ev -> addPreset(p, clickX, clickY));
            presets.add(item);
        }
        add.add(presets);
        return add;
    }

    private void addBarItem(JMenu menu, String label, String source, Double clickX, Double clickY) {
        JMenuItem item = new JMenuItem(label);
        item.addActionListener(ev -> addBar(source, clickX, clickY));
        menu.add(item);
    }

    private JMenu buildFieldMenu(String title, java.util.function.Consumer<String> action) {
        JMenu replace = new JMenu(title);
        String lastGroup = null;
        JMenu group = null;
        for (MiniHudFieldCatalog.FieldDef def : host.getFieldCatalog().all()) {
            String g = def.group != null ? def.group : "其他";
            if (!g.equals(lastGroup)) {
                group = new JMenu(g);
                replace.add(group);
                lastGroup = g;
            }
            JMenuItem item = new JMenuItem(def.label + "  " + def.key);
            item.addActionListener(ev -> action.accept(def.key));
            group.add(item);
        }
        if (replace.getItemCount() == 0) {
            JMenuItem none = new JMenuItem("（无可用字段）");
            none.setEnabled(false);
            replace.add(none);
        }
        return replace;
    }

    private void replaceSelectedField(String fieldKey) {
        MiniHudWidget w = host.getDoc().find(primaryId);
        if (w == null || !MiniHudProps.TYPE_TEXT.equals(w.type))
            return;
        MiniHudFieldCatalog.FieldDef def = host.getFieldCatalog().get(fieldKey);
        pushUndo();
        w.set(MiniHudProps.SOURCE, MiniHudProps.SRC_FIELD);
        w.set(MiniHudProps.FIELD, fieldKey);
        if (def != null) {
            w.set(MiniHudProps.LABEL, def.label != null ? def.label : "");
            w.set(MiniHudProps.UNIT, def.unit != null ? def.unit : "");
            w.set(MiniHudProps.PRECISION, Integer.toString(def.precision));
            String sample = def.previewValue != null && !def.previewValue.isEmpty() ? def.previewValue : "8888";
            w.set(MiniHudProps.TEMPLATE, (def.label != null ? def.label : "") + sample
                    + (def.unit != null ? def.unit : ""));
        }
        afterSimpleChange();
    }

    private void showLayersMenu(int x, int y) {
        JMenu layers = buildLayersMenu();
        JPopupMenu popup = new JPopupMenu();
        while (layers.getItemCount() > 0)
            popup.add(layers.getItem(0));
        Component c = host.getEditorComponent();
        if (c != null)
            popup.show(c, x, y);
    }

    private JMenu buildLayersMenu() {
        JMenu layers = new JMenu("图层");
        List<MiniHudWidget> sorted = new ArrayList<>(host.getDoc().widgets);
        sorted.sort((a, b) -> Integer.compare(b.z, a.z));
        for (MiniHudWidget w : sorted) {
            MiniHudGroup g = host.getDoc().findGroup(w.groupId);
            String name = (g != null ? "[" + g.name + "] " : "") + typeName(w.type) + " " + w.id
                    + (w.visible ? "" : "（隐藏）") + (w.locked ? "（锁定）" : "");
            JMenu item = new JMenu(name);
            JMenuItem select = new JMenuItem("选中");
            select.addActionListener(ev -> selectOnly(w.id));
            item.add(select);
            JMenuItem vis = new JMenuItem(w.visible ? "隐藏" : "显示");
            vis.addActionListener(ev -> {
                pushUndo();
                w.visible = !w.visible;
                afterSimpleChange();
            });
            item.add(vis);
            JMenuItem lock = new JMenuItem(w.locked ? "解锁" : "锁定");
            lock.addActionListener(ev -> {
                pushUndo();
                w.locked = !w.locked;
                afterSimpleChange();
            });
            item.add(lock);
            JMenuItem front = new JMenuItem("置顶");
            front.addActionListener(ev -> {
                pushUndo();
                w.z = maxZ() + 1;
                afterSimpleChange();
            });
            item.add(front);
            JMenuItem back = new JMenuItem("置底");
            back.addActionListener(ev -> {
                pushUndo();
                w.z = minZ() - 1;
                afterSimpleChange();
            });
            item.add(back);
            layers.add(item);
        }
        if (sorted.isEmpty()) {
            JMenuItem none = new JMenuItem("（空画布）");
            none.setEnabled(false);
            layers.add(none);
        }
        return layers;
    }

    private static String typeName(String type) {
        if (MiniHudProps.TYPE_TEXT.equals(type))
            return "文本";
        if (MiniHudProps.TYPE_BAR.equals(type))
            return "条";
        if (MiniHudProps.TYPE_COMPASS.equals(type))
            return "罗盘";
        if (MiniHudProps.TYPE_ATTITUDE.equals(type))
            return "姿态仪";
        if (MiniHudProps.TYPE_PANEL.equals(type))
            return "背景";
        return type;
    }

    // --- 属性 ---

    private void editSelectedProperties() {
        final MiniHudWidget w = host.getDoc().find(primaryId);
        if (w == null)
            return;
        Component parent = host.getEditorComponent();
        JPanel form = new JPanel();
        form.setLayout(new javax.swing.BoxLayout(form, javax.swing.BoxLayout.Y_AXIS));
        final Runnable[] apply = new Runnable[1];

        if (MiniHudProps.TYPE_TEXT.equals(w.type)) {
            final JComboBox<String> source = new JComboBox<>(new String[] {
                    "数据字段", "静态文本", "表速 IAS", "马赫 Ma", "高度", "能量", "攻角", "SEP",
                    "过载 G", "襟翼/可变翼", "减速板", "起落架", "时间", "襟翼角", "油门" });
            source.setSelectedIndex(sourceIndex(w.get(MiniHudProps.SOURCE, MiniHudProps.SRC_STATIC)));
            final JComboBox<String> field = new JComboBox<>();
            for (MiniHudFieldCatalog.FieldDef def : host.getFieldCatalog().all())
                field.addItem(def.key);
            String curField = w.get(MiniHudProps.FIELD, "");
            if (!curField.isEmpty())
                field.setSelectedItem(curField);
            final JTextField text = new JTextField(w.get(MiniHudProps.TEXT, ""), 12);
            final JTextField label = new JTextField(w.get(MiniHudProps.LABEL, ""), 8);
            final JTextField unit = new JTextField(w.get(MiniHudProps.UNIT, ""), 6);
            final JTextField precision = new JTextField(w.get(MiniHudProps.PRECISION, ""), 4);
            final JTextField template = new JTextField(w.get(MiniHudProps.TEMPLATE, ""), 12);
            final JTextField preview = new JTextField(w.get(MiniHudProps.PREVIEW, ""), 12);
            final JComboBox<String> align = new JComboBox<>(new String[] { "左对齐", "居中", "右对齐" });
            align.setSelectedIndex("C".equals(w.get(MiniHudProps.ALIGN, "L")) ? 1
                    : "R".equals(w.get(MiniHudProps.ALIGN, "L")) ? 2 : 0);
            final JComboBox<String> font = new JComboBox<>(new String[] { "主字体", "小字体", "更小字体" });
            String fm = w.get(MiniHudProps.FONT, MiniHudProps.FONT_MAIN);
            font.setSelectedIndex(MiniHudProps.FONT_SMALL.equals(fm) ? 1
                    : MiniHudProps.FONT_TINY.equals(fm) ? 2 : 0);
            final JComboBox<String> color = new JComboBox<>(new String[] { "正常", "自动告警", "告警色", "攻角色" });
            color.setSelectedIndex(colorIndex(w.get(MiniHudProps.COLOR, MiniHudProps.COLOR_NORMAL)));
            final JComboBox<String> alt = new JComboBox<>(new String[] { "自动 (500m 以下雷达)", "气压", "雷达" });
            String as = w.get(MiniHudProps.ALT_SOURCE, MiniHudProps.ALT_AUTO);
            alt.setSelectedIndex(MiniHudProps.ALT_BARO.equals(as) ? 1 : MiniHudProps.ALT_RADAR.equals(as) ? 2 : 0);
            final JCheckBox arrow = new JCheckBox("SEP 升降箭头", w.getBool(MiniHudProps.ARROW, true));

            row(form, "来源", source);
            row(form, "字段", field);
            row(form, "静态文本", text);
            row(form, "标签", label);
            row(form, "单位", unit);
            row(form, "小数位", precision);
            row(form, "宽度模板", template);
            row(form, "预览值", preview);
            row(form, "对齐", align);
            row(form, "字体", font);
            row(form, "颜色", color);
            row(form, "高度来源", alt);
            row(form, "", arrow);
            apply[0] = () -> {
                w.set(MiniHudProps.SOURCE, sourceKey(source.getSelectedIndex()));
                if (field.getSelectedItem() != null)
                    w.set(MiniHudProps.FIELD, field.getSelectedItem().toString());
                w.set(MiniHudProps.TEXT, text.getText());
                w.set(MiniHudProps.LABEL, label.getText());
                w.set(MiniHudProps.UNIT, unit.getText());
                if (!precision.getText().trim().isEmpty()) {
                    try {
                        w.set(MiniHudProps.PRECISION, Integer.toString(Math.max(0,
                                Math.min(6, Integer.parseInt(precision.getText().trim())))));
                    } catch (NumberFormatException ignore) {
                    }
                }
                w.set(MiniHudProps.TEMPLATE, template.getText());
                w.set(MiniHudProps.PREVIEW, preview.getText());
                w.set(MiniHudProps.ALIGN, align.getSelectedIndex() == 1 ? MiniHudProps.ALIGN_CENTER
                        : align.getSelectedIndex() == 2 ? MiniHudProps.ALIGN_RIGHT : MiniHudProps.ALIGN_LEFT);
                w.set(MiniHudProps.FONT, font.getSelectedIndex() == 1 ? MiniHudProps.FONT_SMALL
                        : font.getSelectedIndex() == 2 ? MiniHudProps.FONT_TINY : MiniHudProps.FONT_MAIN);
                w.set(MiniHudProps.COLOR, colorKey(color.getSelectedIndex()));
                w.set(MiniHudProps.ALT_SOURCE, alt.getSelectedIndex() == 1 ? MiniHudProps.ALT_BARO
                        : alt.getSelectedIndex() == 2 ? MiniHudProps.ALT_RADAR : MiniHudProps.ALT_AUTO);
                w.set(MiniHudProps.ARROW, arrow.isSelected() ? "1" : "0");
            };
        } else if (MiniHudProps.TYPE_BAR.equals(w.type)) {
            final JComboBox<String> source = new JComboBox<>(new String[] {
                    "数据字段", "襟翼角", "速度比", "油门", "AoA", "机动" });
            source.setSelectedIndex(barSourceIndex(w.get(MiniHudProps.BAR_SOURCE, MiniHudProps.BAR_FIELD)));
            final JComboBox<String> field = new JComboBox<>();
            for (MiniHudFieldCatalog.FieldDef def : host.getFieldCatalog().all())
                field.addItem(def.key);
            String curField = w.get(MiniHudProps.FIELD, "");
            if (!curField.isEmpty())
                field.setSelectedItem(curField);
            final JTextField min = new JTextField(w.get(MiniHudProps.MIN, "0"), 4);
            final JTextField max = new JTextField(w.get(MiniHudProps.MAX, "1"), 4);
            final JTextField length = new JTextField(w.get(MiniHudProps.LENGTH_UNITS, "4"), 4);
            final JComboBox<String> orient = new JComboBox<>(new String[] { "水平", "垂直" });
            orient.setSelectedIndex("V".equals(w.get(MiniHudProps.ORIENTATION, "H")) ? 1 : 0);
            row(form, "来源", source);
            row(form, "字段", field);
            row(form, "最小值", min);
            row(form, "最大值", max);
            row(form, "长度(行高)", length);
            row(form, "方向", orient);
            apply[0] = () -> {
                w.set(MiniHudProps.BAR_SOURCE, barSourceKey(source.getSelectedIndex()));
                if (field.getSelectedItem() != null)
                    w.set(MiniHudProps.FIELD, field.getSelectedItem().toString());
                w.set(MiniHudProps.MIN, min.getText());
                w.set(MiniHudProps.MAX, max.getText());
                w.set(MiniHudProps.LENGTH_UNITS, length.getText());
                w.set(MiniHudProps.ORIENTATION,
                        orient.getSelectedIndex() == 1 ? MiniHudProps.ORIENT_V : MiniHudProps.ORIENT_H);
            };
        } else if (MiniHudProps.TYPE_COMPASS.equals(w.type) || MiniHudProps.TYPE_ATTITUDE.equals(w.type)) {
            final JComboBox<String> mode = new JComboBox<>(new String[] { "随体", "离体" });
            mode.setSelectedIndex(MiniHudProps.MODE_INERTIAL.equals(
                    w.get(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY)) ? 1 : 0);
            row(form, "坐标系", mode);
            apply[0] = () -> w.set(MiniHudProps.COORDINATE_MODE,
                    mode.getSelectedIndex() == 1 ? MiniHudProps.MODE_INERTIAL : MiniHudProps.MODE_BODY);
        } else if (MiniHudProps.TYPE_PANEL.equals(w.type)) {
            final JTextField width = new JTextField(w.get(MiniHudProps.WIDTH_UNITS, "6"), 4);
            final JTextField height = new JTextField(w.get(MiniHudProps.HEIGHT_UNITS, "3"), 4);
            final JTextField radius = new JTextField(w.get(MiniHudProps.RADIUS_UNITS, "0.25"), 4);
            final JTextField color = new JTextField(w.get(MiniHudProps.PANEL_COLOR, "#60202020"), 10);
            row(form, "宽(行高)", width);
            row(form, "高(行高)", height);
            row(form, "圆角(行高)", radius);
            row(form, "颜色(#RRGGBBAA)", color);
            apply[0] = () -> {
                w.set(MiniHudProps.WIDTH_UNITS, width.getText());
                w.set(MiniHudProps.HEIGHT_UNITS, height.getText());
                w.set(MiniHudProps.RADIUS_UNITS, radius.getText());
                w.set(MiniHudProps.PANEL_COLOR, color.getText());
            };
        } else {
            JOptionPane.showMessageDialog(parent, "该组件没有可调属性", "组件属性",
                    JOptionPane.INFORMATION_MESSAGE);
            return;
        }

        final JTextField scale = new JTextField(Double.toString(w.scale), 5);
        final JTextField z = new JTextField(Integer.toString(w.z), 5);
        row(form, "缩放", scale);
        row(form, "层级 z", z);

        int ok = JOptionPane.showConfirmDialog(parent, form, "组件属性", JOptionPane.OK_CANCEL_OPTION,
                JOptionPane.PLAIN_MESSAGE);
        if (ok != JOptionPane.OK_OPTION)
            return;
        pushUndo();
        apply[0].run();
        try {
            w.scale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, Double.parseDouble(scale.getText().trim())));
        } catch (Exception ignore) {
        }
        try {
            w.z = Integer.parseInt(z.getText().trim());
        } catch (Exception ignore) {
        }
        afterSimpleChange();
    }

    private static void row(JPanel form, String label, Component c) {
        JPanel r = new JPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 4, 2));
        r.setOpaque(false);
        if (!label.isEmpty())
            r.add(new javax.swing.JLabel(label));
        r.add(c);
        form.add(r);
    }

    private static int sourceIndex(String source) {
        switch (source) {
            case MiniHudProps.SRC_FIELD: return 0;
            case MiniHudProps.SRC_STATIC: return 1;
            case MiniHudProps.SRC_IAS: return 2;
            case MiniHudProps.SRC_MACH: return 3;
            case MiniHudProps.SRC_ALTITUDE: return 4;
            case MiniHudProps.SRC_ENERGY: return 5;
            case MiniHudProps.SRC_AOA: return 6;
            case MiniHudProps.SRC_SEP: return 7;
            case MiniHudProps.SRC_GLOAD: return 8;
            case MiniHudProps.SRC_FLAPS: return 9;
            case MiniHudProps.SRC_AIRBRAKE: return 10;
            case MiniHudProps.SRC_GEAR: return 11;
            case MiniHudProps.SRC_TIME: return 12;
            case MiniHudProps.SRC_FLAP_ANGLE: return 13;
            case MiniHudProps.SRC_THROTTLE: return 14;
            default: return 1;
        }
    }

    private static String sourceKey(int idx) {
        switch (idx) {
            case 0: return MiniHudProps.SRC_FIELD;
            case 1: return MiniHudProps.SRC_STATIC;
            case 2: return MiniHudProps.SRC_IAS;
            case 3: return MiniHudProps.SRC_MACH;
            case 4: return MiniHudProps.SRC_ALTITUDE;
            case 5: return MiniHudProps.SRC_ENERGY;
            case 6: return MiniHudProps.SRC_AOA;
            case 7: return MiniHudProps.SRC_SEP;
            case 8: return MiniHudProps.SRC_GLOAD;
            case 9: return MiniHudProps.SRC_FLAPS;
            case 10: return MiniHudProps.SRC_AIRBRAKE;
            case 11: return MiniHudProps.SRC_GEAR;
            case 12: return MiniHudProps.SRC_TIME;
            case 13: return MiniHudProps.SRC_FLAP_ANGLE;
            case 14: return MiniHudProps.SRC_THROTTLE;
            default: return MiniHudProps.SRC_STATIC;
        }
    }

    private static int colorIndex(String color) {
        switch (color) {
            case MiniHudProps.COLOR_AUTO: return 1;
            case MiniHudProps.COLOR_WARNING: return 2;
            case MiniHudProps.COLOR_AOA: return 3;
            default: return 0;
        }
    }

    private static String colorKey(int idx) {
        switch (idx) {
            case 1: return MiniHudProps.COLOR_AUTO;
            case 2: return MiniHudProps.COLOR_WARNING;
            case 3: return MiniHudProps.COLOR_AOA;
            default: return MiniHudProps.COLOR_NORMAL;
        }
    }

    private static int barSourceIndex(String source) {
        switch (source) {
            case MiniHudProps.BAR_FLAP_ANGLE: return 1;
            case MiniHudProps.BAR_SPEED_RATIO: return 2;
            case MiniHudProps.BAR_THROTTLE: return 3;
            case MiniHudProps.BAR_AOA: return 4;
            case MiniHudProps.BAR_MANEUVER: return 5;
            default: return 0;
        }
    }

    private static String barSourceKey(int idx) {
        switch (idx) {
            case 1: return MiniHudProps.BAR_FLAP_ANGLE;
            case 2: return MiniHudProps.BAR_SPEED_RATIO;
            case 3: return MiniHudProps.BAR_THROTTLE;
            case 4: return MiniHudProps.BAR_AOA;
            case 5: return MiniHudProps.BAR_MANEUVER;
            default: return MiniHudProps.BAR_FIELD;
        }
    }

    // --- 绘制 ---

    public void paint(Graphics2D g2d) {
        paintHiddenPlaceholders(g2d);
        paintSelection(g2d);
        if (marqueeRect != null) {
            g2d.setColor(MARQUEE_COLOR);
            g2d.setStroke(GUIDE_STROKE);
            g2d.drawRect(marqueeRect.x, marqueeRect.y, marqueeRect.width, marqueeRect.height);
        }
        if (resizePreview != null) {
            int ox = host.getScene().getOffsetX();
            int oy = host.getScene().getOffsetY();
            g2d.setColor(SELECT_COLOR);
            g2d.setStroke(SELECT_STROKE);
            g2d.drawRect(resizePreview.x + ox, resizePreview.y + oy, resizePreview.width, resizePreview.height);
        }
        paintToolbar(g2d);
    }

    private void paintHiddenPlaceholders(Graphics2D g2d) {
        if (host.getScene() == null || readOnly)
            return;
        g2d.setColor(HIDDEN_COLOR);
        g2d.setStroke(HIDDEN_STROKE);
        for (MiniHudScene.Runtime rt : host.getScene().runtimes) {
            if (!rt.widget.visible || !rt.component.isVisible()) {
                Rectangle r = screenRect(rt);
                g2d.drawRect(r.x, r.y, Math.max(4, r.width), Math.max(4, r.height));
            }
        }
    }

    private void paintSelection(Graphics2D g2d) {
        if (host.getScene() == null || selected.isEmpty())
            return;
        g2d.setStroke(SELECT_STROKE);
        for (String id : selected) {
            MiniHudScene.Runtime rt = host.getScene().runtime(id);
            if (rt == null)
                continue;
            Rectangle r = screenRect(rt);
            g2d.setColor(rt.widget.locked ? LOCK_COLOR : SELECT_COLOR);
            g2d.drawRect(r.x - 1, r.y - 1, r.width + 2, r.height + 2);
        }
        MiniHudScene.Runtime primary = host.getScene().runtime(primaryId);
        if (primary != null) {
            Rectangle r = screenRect(primary);
            g2d.setColor(primary.widget.locked ? LOCK_COLOR : SELECT_COLOR);
            g2d.fillRect(r.x + r.width - 3, r.y + r.height - 3, 6, 6);
        }
    }

    private void paintToolbar(Graphics2D g2d) {
        int size = Math.max(11, host.getCtx().hudFontSize / 2);
        if (toolbarFont == null || toolbarFontSize != size) {
            toolbarFont = new Font("SansSerif", Font.PLAIN, size);
            toolbarFontSize = size;
        }
        g2d.setFont(toolbarFont);
        int x = 6;
        int y = 6;
        int h = size + 12;
        x = placeButton(g2d, tbEdit, x, y, h, readOnly ? "编辑" : "预览");
        if (readOnly) {
            tbAdd.setBounds(0, 0, 0, 0);
            tbUndo.setBounds(0, 0, 0, 0);
            tbRedo.setBounds(0, 0, 0, 0);
            tbLayers.setBounds(0, 0, 0, 0);
            tbReset.setBounds(0, 0, 0, 0);
            return;
        }
        x = placeButton(g2d, tbAdd, x, y, h, "+ 添加");
        x = placeButton(g2d, tbUndo, x, y, h, "撤销");
        x = placeButton(g2d, tbRedo, x, y, h, "重做");
        x = placeButton(g2d, tbLayers, x, y, h, "图层");
        placeButton(g2d, tbReset, x, y, h, "默认");
    }

    private int placeButton(Graphics2D g2d, Rectangle rect, int x, int y, int h, String label) {
        int w = g2d.getFontMetrics().stringWidth(label) + 16;
        rect.setBounds(x, y, w, h);
        g2d.setColor(TOOLBAR_BG);
        g2d.fillRect(x, y, w, h);
        g2d.setColor(TOOLBAR_FG);
        g2d.drawString(label, x + 8, y + h - 8);
        return x + w + 4;
    }
}
