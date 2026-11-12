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
import java.util.ArrayList;
import java.util.List;

import javax.swing.JCheckBox;
import javax.swing.JComboBox;
import javax.swing.JMenu;
import javax.swing.JMenuItem;
import javax.swing.JOptionPane;
import javax.swing.JPanel;
import javax.swing.JPopupMenu;
import javax.swing.JTextField;

import prog.config.MiniHudFieldCatalog;
import ui.minihud.HUDWidgetRegistry;
import ui.minihud.HUDWidgetSpec;
import ui.minihud.MiniHudCells;
import ui.minihud.MiniHudEntry;
import ui.minihud.MiniHudProps;
import ui.minihud.MiniHudScene;
import ui.minihud.MiniHudWidgets;

/**
 * 预览态画布编辑器：选择、拖动、缩放、增删、换字段、右键菜单、工具条、快捷键。
 * 默认只读（只显示「编辑」按钮），点「编辑」进入编辑态；游戏态绝不安装。
 */
public final class MiniHudEditor {

    private static final Color SELECT_COLOR = new Color(255, 200, 40, 230);
    private static final Color LOCK_COLOR = new Color(150, 150, 150, 200);
    private static final Color HIDDEN_COLOR = new Color(160, 160, 160, 120);
    private static final Color TOOLBAR_BG = new Color(0, 0, 0, 160);
    private static final Color TOOLBAR_FG = new Color(220, 220, 220, 230);
    private static final BasicStroke SELECT_STROKE = new BasicStroke(1.5f, BasicStroke.CAP_BUTT,
            BasicStroke.JOIN_MITER, 10f, new float[] { 5f, 4f }, 0f);
    private static final BasicStroke HIDDEN_STROKE = new BasicStroke(1f, BasicStroke.CAP_BUTT,
            BasicStroke.JOIN_MITER, 10f, new float[] { 3f, 3f }, 0f);
    private static final double SNAP = 0.25;
    private static final int HANDLE = 9;
    private static final double MIN_SCALE = 0.25;
    private static final double MAX_SCALE = 4.0;

    private final MiniHudEditorHost host;
    private Component target;
    private MouseAdapter mouseAdapter;
    private MouseMotionAdapter motionAdapter;
    private KeyAdapter keyAdapter;

    private String selectedId;
    // 默认只读（预览态）；点「编辑」才允许拖动/增删
    private boolean readOnly = true;

    private boolean dragging;
    private boolean moved;
    private int pressX, pressY;
    private double startUnitX, startUnitY;

    private boolean resizing;
    private String resizeId;
    private double startScale;
    private Rectangle resizePreview;

    private boolean windowDrag;
    private int startWindowX, startWindowY;
    private int startScreenX, startScreenY;
    private boolean arrowDirty; // 方向键微调尚未落盘
    private long lastMenuMs; // 防 press/release 双触发弹两次菜单

    private final Rectangle tbEdit = new Rectangle();
    private final Rectangle tbAdd = new Rectangle();
    private final Rectangle tbReset = new Rectangle();
    private final Rectangle tbFit = new Rectangle();
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
        selectedId = null;
        dragging = false;
        resizing = false;
        windowDrag = false;
        if (arrowDirty) {
            arrowDirty = false;
            host.commitLayout();
        }
    }

    public boolean isDragging() {
        return dragging || resizing;
    }

    public String getSelectedId() {
        return selectedId;
    }

    public boolean isReadOnly() {
        return readOnly;
    }

    public void setReadOnly(boolean v) {
        this.readOnly = v;
        host.setEditorEditing(!v);
        if (v) {
            selectedId = null;
            // 只读时只保留「编辑」按钮，其余按钮不可见/不可点
            tbAdd.setBounds(0, 0, 0, 0);
            tbReset.setBounds(0, 0, 0, 0);
            tbFit.setBounds(0, 0, 0, 0);
            if (arrowDirty) {
                arrowDirty = false;
                host.commitLayout();
            }
        }
        host.repaintEditor();
    }

    private void onPressed(MouseEvent e) {
        if (!host.isPreviewMode() || host.getScene() == null)
            return;
        if (target != null) {
            // 首次点击时才允许窗口获得焦点（避免预览窗创建时抢主窗焦点）
            java.awt.Window w = javax.swing.SwingUtilities.getWindowAncestor(target);
            if (w != null) {
                w.setFocusableWindowState(true);
                w.setFocusable(true);
            }
            target.requestFocusInWindow();
        }

        // 工具条优先
        if (onToolbar(e.getX(), e.getY()))
            return;

        // 右键：Windows 的 popup trigger 在 release，press 时只等待，避免弹两次菜单
        if (e.isPopupTrigger()) {
            if (readOnly)
                return;
            ui.layout.HUDLayoutNode hit = host.getScene().engine.hitTest(e.getX(), e.getY(), true);
            selectedId = hit != null ? hit.id : null;
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

        MiniHudScene scene = host.getScene();
        ui.layout.HUDLayoutNode hit = scene.engine.hitTest(e.getX(), e.getY(), true);
        if (hit != null) {
            // 先选中、再在已选中条目的角柄上拖动才缩放，避免小条目一点就进缩放
            boolean wasSelected = hit.id.equals(selectedId);
            selectedId = hit.id;
            MiniHudEntry entry = host.getDoc().find(hit.id);
            if (entry != null && wasSelected && !entry.locked && isNearHandle(hit, e.getX(), e.getY())
                    && isScalable(entry)) {
                resizing = true;
                resizeId = entry.id;
                startScale = entry.scale;
                resizePreview = new Rectangle(hit.getPixelRect());
            } else if (entry != null && !entry.locked) {
                dragging = true;
                startUnitX = entry.unitX;
                startUnitY = entry.unitY;
            }
            host.repaintEditor();
        } else {
            selectedId = null;
            startWindowDrag(e);
            host.repaintEditor();
        }
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
        if (resizing && resizeId != null) {
            int dx = e.getX() - pressX;
            int dy = e.getY() - pressY;
            Rectangle r = host.getScene().node(resizeId).getPixelRect();
            resizePreview = new Rectangle(r.x, r.y, Math.max(6, r.width + dx), Math.max(6, r.height + dy));
            host.repaintEditor();
            return;
        }
        if (dragging && selectedId != null) {
            int dx = e.getX() - pressX;
            int dy = e.getY() - pressY;
            if (!moved && Math.abs(dx) + Math.abs(dy) < 2)
                return;
            moved = true;
            MiniHudEntry entry = host.getDoc().find(selectedId);
            MiniHudScene.EntryRuntime rt = host.getScene().byId.get(selectedId);
            if (entry == null || rt == null)
                return;
            double lh = Math.max(1, host.getCtx().hudFontSize);
            double ux = startUnitX + dx / lh;
            double uy = startUnitY + dy / lh;
            if (!e.isAltDown()) {
                ux = Math.round(ux / SNAP) * SNAP;
                uy = Math.round(uy / SNAP) * SNAP;
            }
            entry.unitX = ux;
            entry.unitY = uy;
            rt.node.setRelativePosition(ux, uy);
            host.getScene().engine.doLayout();
            host.repaintEditor();
        } else if (windowDrag) {
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
            ui.layout.HUDLayoutNode hit = host.getScene().engine.hitTest(e.getX(), e.getY(), true);
            selectedId = hit != null ? hit.id : null;
            host.repaintEditor();
            showMenu(e);
            return;
        }
        if (resizing) {
            applyResize();
            resizing = false;
            resizeId = null;
            resizePreview = null;
            afterDocChange();
            return;
        }
        if (windowDrag) {
            windowDrag = false;
            host.saveEditorWindowPosition();
            return;
        }
        if (dragging) {
            dragging = false;
            if (moved && selectedId != null)
                host.commitLayout();
        }
    }

    private void applyResize() {
        MiniHudEntry entry = host.getDoc().find(resizeId);
        MiniHudScene.EntryRuntime rt = host.getScene().byId.get(resizeId);
        if (entry == null || rt == null || resizePreview == null)
            return;
        int dw = resizePreview.width - rt.node.getPixelRect().width;
        int dh = resizePreview.height - rt.node.getPixelRect().height;
        double lh = Math.max(1, host.getCtx().hudFontSize);
        double scale = startScale + (dw + dh) / (2.0 * lh);
        if (scale < MIN_SCALE)
            scale = MIN_SCALE;
        if (scale > MAX_SCALE)
            scale = MAX_SCALE;
        // 双精度取 0.05 档，避免存一堆长小数
        entry.scale = Math.round(scale * 20.0) / 20.0;
    }

    private void onKey(KeyEvent e) {
        if (!host.isPreviewMode() || readOnly)
            return;
        MiniHudEntry sel = host.getDoc().find(selectedId);
        int code = e.getKeyCode();
        if (sel != null && sel.locked) {
            if (code == KeyEvent.VK_ESCAPE)
                selectedId = null;
            return;
        }
        if (code == KeyEvent.VK_ESCAPE) {
            selectedId = null;
            host.repaintEditor();
            return;
        }
        if (sel == null)
            return;
        if (code == KeyEvent.VK_DELETE) {
            deleteSelected();
            e.consume();
            return;
        }
        if (e.isControlDown() && code == KeyEvent.VK_D) {
            duplicateSelected();
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
            sel.unitX += dx;
            sel.unitY += dy;
            MiniHudScene.EntryRuntime rt = host.getScene().byId.get(sel.id);
            if (rt != null)
                rt.node.setRelativePosition(sel.unitX, sel.unitY);
            host.getScene().engine.doLayout();
            arrowDirty = true; // 键盘连按只在松键时落盘一次
            host.repaintEditor();
            e.consume();
        }
    }

    private boolean onToolbar(int x, int y) {
        if (tbEdit.contains(x, y)) {
            setReadOnly(!readOnly);
            return true;
        }
        if (tbAdd.contains(x, y)) {
            showAddOnlyMenu(x, y);
            return true;
        }
        if (tbReset.contains(x, y)) {
            host.resetToDefaultLayout();
            return true;
        }
        if (tbFit.contains(x, y)) {
            host.rebuildSceneFromDoc();
            return true;
        }
        return false;
    }

    private boolean isNearHandle(ui.layout.HUDLayoutNode node, int x, int y) {
        Rectangle r = node.getPixelRect();
        int ox = host.getScene().engine.getRenderOffsetX();
        int oy = host.getScene().engine.getRenderOffsetY();
        int rx = r.x + ox;
        int ry = r.y + oy;
        return (Math.abs(x - (rx + r.width)) <= HANDLE && Math.abs(y - (ry + r.height)) <= HANDLE)
                || (Math.abs(x - rx) <= HANDLE && Math.abs(y - (ry + r.height)) <= HANDLE)
                || (Math.abs(x - (rx + r.width)) <= HANDLE && Math.abs(y - ry) <= HANDLE)
                || (Math.abs(x - rx) <= HANDLE && Math.abs(y - ry) <= HANDLE);
    }

    private boolean isScalable(MiniHudEntry e) {
        HUDWidgetSpec spec = HUDWidgetRegistry.find(e.type);
        return spec == null || spec.scalable;
    }

    // --- 编辑操作（同时供右键菜单与离屏交互测试调用） ---

    public void addEntry(String type) {
        MiniHudEntry e = newEntry(type);
        selectedId = e.id;
        afterDocChange();
    }

    public void addDataField(String fieldKey) {
        if (fieldKey == null || fieldKey.isEmpty())
            return;
        MiniHudEntry e = newEntry("text.row");
        MiniHudCells.Cell c = new MiniHudCells.Cell();
        c.field = fieldKey;
        MiniHudFieldCatalog.FieldDef def = host.getFieldCatalog().get(fieldKey);
        if (def != null) {
            c.label = def.label != null ? def.label : "";
            c.unit = def.unit != null ? def.unit : "";
            c.precision = def.precision;
        }
        List<MiniHudCells.Cell> cells = new ArrayList<>();
        cells.add(c);
        e.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        selectedId = e.id;
        afterDocChange();
    }

    public void addStaticText(String text) {
        MiniHudEntry e = newEntry("text.row");
        MiniHudCells.Cell c = new MiniHudCells.Cell();
        c.text = text != null ? text : "";
        List<MiniHudCells.Cell> cells = new ArrayList<>();
        cells.add(c);
        e.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        selectedId = e.id;
        afterDocChange();
    }

    public void deleteSelected() {
        if (selectedId == null)
            return;
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel != null && sel.locked)
            return; // 锁定条目禁止删除
        int count = countSubtree(selectedId);
        Component c = host.getEditorComponent();
        if (count > 1 && c != null && !java.awt.GraphicsEnvironment.isHeadless()) {
            int r = JOptionPane.showConfirmDialog(c, "将删除该条目及其 " + (count - 1) + " 个子条目，继续？",
                    "删除条目", JOptionPane.OK_CANCEL_OPTION);
            if (r != JOptionPane.OK_OPTION)
                return;
        }
        host.getDoc().remove(selectedId);
        selectedId = null;
        afterDocChange();
    }

    public void duplicateSelected() {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null)
            return;
        String newId = host.getDoc().nextId();
        MiniHudEntry copy = sel.copy(newId);
        copy.unitX += 1.0;
        host.getDoc().entries.add(copy);
        selectedId = newId;
        afterDocChange();
    }

    public void replaceSelectedField(String fieldKey) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null || !"text.row".equals(sel.type))
            return;
        List<MiniHudCells.Cell> cells = MiniHudCells.parse(sel.get(MiniHudProps.CELLS, ""));
        if (cells.isEmpty())
            cells.add(new MiniHudCells.Cell());
        MiniHudCells.Cell c = cells.get(0);
        c.field = fieldKey;
        MiniHudFieldCatalog.FieldDef def = host.getFieldCatalog().get(fieldKey);
        if (def != null) {
            c.label = def.label != null ? def.label : c.label;
            c.unit = def.unit != null ? def.unit : c.unit;
            c.precision = def.precision;
        }
        sel.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        afterDocChange();
    }

    public void setSelectedLabel(String label) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null)
            return;
        if ("text.row".equals(sel.type)) {
            List<MiniHudCells.Cell> cells = MiniHudCells.parse(sel.get(MiniHudProps.CELLS, ""));
            if (cells.isEmpty())
                cells.add(new MiniHudCells.Cell());
            cells.get(0).label = label != null ? label : "";
            sel.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        } else if ("legacy.akb".equals(sel.type)) {
            sel.set(MiniHudProps.SPEED_LABEL, label != null ? label : "");
        } else if ("legacy.energy".equals(sel.type)) {
            sel.set(MiniHudProps.ALT_LABEL, label != null ? label : "");
        } else if ("legacy.sep".equals(sel.type)) {
            sel.set(MiniHudProps.SEP_LABEL, label != null ? label : "");
        } else if ("legacy.maneuver".equals(sel.type)) {
            sel.set(MiniHudProps.G_LABEL, label != null ? label : "");
        }
        afterDocChange();
    }

    /** 在选中的 text.row 末尾追加一个数据格（grid 行）。 */
    public void appendDataField(String fieldKey) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null || !"text.row".equals(sel.type))
            return;
        List<MiniHudCells.Cell> cells = MiniHudCells.parse(sel.get(MiniHudProps.CELLS, ""));
        MiniHudCells.Cell c = new MiniHudCells.Cell();
        c.field = fieldKey;
        MiniHudFieldCatalog.FieldDef def = host.getFieldCatalog().get(fieldKey);
        if (def != null) {
            c.label = def.label != null ? def.label : "";
            c.unit = def.unit != null ? def.unit : "";
            c.precision = def.precision;
        }
        cells.add(c);
        sel.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        afterDocChange();
    }

    public void appendStaticText(String text) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null || !"text.row".equals(sel.type))
            return;
        List<MiniHudCells.Cell> cells = MiniHudCells.parse(sel.get(MiniHudProps.CELLS, ""));
        MiniHudCells.Cell c = new MiniHudCells.Cell();
        c.text = text != null ? text : "";
        cells.add(c);
        sel.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        afterDocChange();
    }

    public void setSelectedUnit(String unit) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null || !"text.row".equals(sel.type))
            return;
        List<MiniHudCells.Cell> cells = MiniHudCells.parse(sel.get(MiniHudProps.CELLS, ""));
        if (cells.isEmpty())
            return;
        cells.get(0).unit = unit != null ? unit : "";
        sel.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        afterDocChange();
    }

    public void setSelectedPrecision(int precision) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null || !"text.row".equals(sel.type))
            return;
        List<MiniHudCells.Cell> cells = MiniHudCells.parse(sel.get(MiniHudProps.CELLS, ""));
        if (cells.isEmpty())
            return;
        cells.get(0).precision = Math.max(0, Math.min(6, precision));
        sel.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        afterDocChange();
    }

    public void setSelectedAlign(char align) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null || !"text.row".equals(sel.type))
            return;
        List<MiniHudCells.Cell> cells = MiniHudCells.parse(sel.get(MiniHudProps.CELLS, ""));
        for (MiniHudCells.Cell c : cells)
            c.align = align;
        sel.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        afterDocChange();
    }

    public void setSelectedText(String text) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null || !"text.row".equals(sel.type))
            return;
        List<MiniHudCells.Cell> cells = MiniHudCells.parse(sel.get(MiniHudProps.CELLS, ""));
        if (cells.isEmpty())
            cells.add(new MiniHudCells.Cell());
        cells.get(0).text = text != null ? text : "";
        sel.set(MiniHudProps.CELLS, MiniHudCells.encode(cells));
        afterDocChange();
    }

    public void toggleSelectedVisible() {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null)
            return;
        sel.visible = !sel.visible;
        afterDocChange();
    }

    public void toggleSelectedLocked() {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null)
            return;
        sel.locked = !sel.locked;
        afterDocChange();
    }

    public void bringToFront() {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null)
            return;
        int max = 0;
        for (MiniHudEntry e : host.getDoc().entries)
            max = Math.max(max, e.z);
        sel.z = max + 1;
        afterDocChange();
    }

    public void sendToBack() {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null)
            return;
        int min = 0;
        for (MiniHudEntry e : host.getDoc().entries)
            min = Math.min(min, e.z);
        sel.z = min - 1;
        afterDocChange();
    }

    public void setSelectedScale(double scale) {
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null)
            return;
        sel.scale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, scale));
        afterDocChange();
    }

    /** 复合行/仪表的条目属性：显示方式全部在这里按条目设置，没有全局开关。 */
    private void editSelectedProperties() {
        final MiniHudEntry e = host.getDoc().find(selectedId);
        if (e == null)
            return;
        Component parent = host.getEditorComponent();
        JPanel form = new JPanel();
        form.setLayout(new javax.swing.BoxLayout(form, javax.swing.BoxLayout.Y_AXIS));
        final Runnable[] apply = new Runnable[1];

        if ("legacy.akb".equals(e.type)) {
            final JComboBox<String> speed = new JComboBox<>(new String[] { "马赫数 (Ma)", "表速 (IAS)" });
            speed.setSelectedIndex(MiniHudProps.FIELD_IAS.equals(
                    e.get(MiniHudProps.SPEED_FIELD, MiniHudProps.FIELD_MACH)) ? 1 : 0);
            final JTextField label = new JTextField(e.get(MiniHudProps.SPEED_LABEL, "SPD"), 8);
            final JCheckBox showSpeed = new JCheckBox("显示速度", e.getBool(MiniHudProps.SHOW_SPEED, true));
            final JCheckBox showAoa = new JCheckBox("显示攻角", e.getBool(MiniHudProps.SHOW_AOA, true));
            row(form, "速度字段", speed);
            row(form, "速度标签", label);
            row(form, "", showSpeed);
            row(form, "", showAoa);
            apply[0] = () -> {
                e.set(MiniHudProps.SPEED_FIELD,
                        speed.getSelectedIndex() == 1 ? MiniHudProps.FIELD_IAS : MiniHudProps.FIELD_MACH);
                e.set(MiniHudProps.SPEED_LABEL, label.getText());
                e.set(MiniHudProps.SHOW_SPEED, showSpeed.isSelected() ? "1" : "0");
                e.set(MiniHudProps.SHOW_AOA, showAoa.isSelected() ? "1" : "0");
            };
        } else if ("legacy.energy".equals(e.type)) {
            final JComboBox<String> alt = new JComboBox<>(
                    new String[] { "自动 (500m 以下雷达)", "气压高度", "始终雷达高度" });
            String src = e.get(MiniHudProps.ALT_SOURCE, MiniHudProps.ALT_AUTO);
            alt.setSelectedIndex(MiniHudProps.ALT_BARO.equals(src) ? 1
                    : MiniHudProps.ALT_RADAR.equals(src) ? 2 : 0);
            final JTextField altLabel = new JTextField(e.get(MiniHudProps.ALT_LABEL, "ALT"), 8);
            final JTextField energyLabel = new JTextField(e.get(MiniHudProps.ENERGY_LABEL, "E"), 8);
            final JCheckBox showAlt = new JCheckBox("显示高度", e.getBool(MiniHudProps.SHOW_ALTITUDE, true));
            final JCheckBox showEnergy = new JCheckBox("显示能量", e.getBool(MiniHudProps.SHOW_ENERGY, true));
            row(form, "高度来源", alt);
            row(form, "高度标签", altLabel);
            row(form, "能量标签", energyLabel);
            row(form, "", showAlt);
            row(form, "", showEnergy);
            apply[0] = () -> {
                e.set(MiniHudProps.ALT_SOURCE,
                        alt.getSelectedIndex() == 1 ? MiniHudProps.ALT_BARO
                                : alt.getSelectedIndex() == 2 ? MiniHudProps.ALT_RADAR : MiniHudProps.ALT_AUTO);
                e.set(MiniHudProps.ALT_LABEL, altLabel.getText());
                e.set(MiniHudProps.ENERGY_LABEL, energyLabel.getText());
                e.set(MiniHudProps.SHOW_ALTITUDE, showAlt.isSelected() ? "1" : "0");
                e.set(MiniHudProps.SHOW_ENERGY, showEnergy.isSelected() ? "1" : "0");
            };
        } else if ("legacy.mechanization".equals(e.type)) {
            final JCheckBox flap = new JCheckBox("襟翼/可变翼", e.getBool(MiniHudProps.FLAP_TEXT, true));
            final JCheckBox brk = new JCheckBox("减速板", e.getBool(MiniHudProps.AIRBRAKE_TEXT, true));
            final JCheckBox gear = new JCheckBox("起落架", e.getBool(MiniHudProps.GEAR_TEXT, true));
            row(form, "", flap);
            row(form, "", brk);
            row(form, "", gear);
            apply[0] = () -> {
                e.set(MiniHudProps.FLAP_TEXT, flap.isSelected() ? "1" : "0");
                e.set(MiniHudProps.AIRBRAKE_TEXT, brk.isSelected() ? "1" : "0");
                e.set(MiniHudProps.GEAR_TEXT, gear.isSelected() ? "1" : "0");
            };
        } else if ("legacy.sep".equals(e.type)) {
            final JTextField label = new JTextField(e.get(MiniHudProps.SEP_LABEL, "SEP"), 8);
            final JCheckBox arrow = new JCheckBox("显示升降箭头", e.getBool(MiniHudProps.SEP_ARROW, true));
            row(form, "标签", label);
            row(form, "", arrow);
            apply[0] = () -> {
                e.set(MiniHudProps.SEP_LABEL, label.getText());
                e.set(MiniHudProps.SEP_ARROW, arrow.isSelected() ? "1" : "0");
            };
        } else if ("legacy.maneuver".equals(e.type)) {
            final JTextField gLabel = new JTextField(e.get(MiniHudProps.G_LABEL, "G"), 8);
            final JCheckBox g = new JCheckBox("显示过载读数", e.getBool(MiniHudProps.SHOW_GLOAD, true));
            final JCheckBox bar = new JCheckBox("显示机动条", e.getBool(MiniHudProps.SHOW_BAR, true));
            row(form, "过载标签", gLabel);
            row(form, "", g);
            row(form, "", bar);
            apply[0] = () -> {
                e.set(MiniHudProps.G_LABEL, gLabel.getText());
                e.set(MiniHudProps.SHOW_GLOAD, g.isSelected() ? "1" : "0");
                e.set(MiniHudProps.SHOW_BAR, bar.isSelected() ? "1" : "0");
            };
        } else if ("gauge.compass".equals(e.type) || "gauge.attitude".equals(e.type)) {
            final JComboBox<String> mode = new JComboBox<>(new String[] { "随体", "离体" });
            mode.setSelectedIndex(MiniHudProps.MODE_INERTIAL.equals(
                    e.get(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY)) ? 1 : 0);
            row(form, "坐标系", mode);
            apply[0] = () -> e.set(MiniHudProps.COORDINATE_MODE,
                    mode.getSelectedIndex() == 1 ? MiniHudProps.MODE_INERTIAL : MiniHudProps.MODE_BODY);
        } else {
            JOptionPane.showMessageDialog(parent, "该条目没有可调属性", "条目属性", JOptionPane.INFORMATION_MESSAGE);
            return;
        }

        int ok = JOptionPane.showConfirmDialog(parent, form, "条目属性", JOptionPane.OK_CANCEL_OPTION,
                JOptionPane.PLAIN_MESSAGE);
        if (ok != JOptionPane.OK_OPTION)
            return;
        apply[0].run();
        afterDocChange();
    }

    private static void row(JPanel form, String label, Component c) {
        JPanel r = new JPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 4, 2));
        r.setOpaque(false);
        if (!label.isEmpty())
            r.add(new javax.swing.JLabel(label));
        r.add(c);
        form.add(r);
    }

    private int countSubtree(String id) {
        int count = 1;
        for (MiniHudEntry e : host.getDoc().entries) {
            if (id.equals(e.parentId))
                count += countSubtree(e.id);
        }
        return count;
    }

    private MiniHudEntry newEntry(String type) {
        MiniHudEntry e = new MiniHudEntry(host.getDoc().nextId(), type);
        MiniHudWidgets.applyDefaults(e);
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel != null) {
            e.parentId = sel.parentId;
            e.parentAnchor = sel.parentAnchor;
            e.selfAnchor = sel.selfAnchor;
            e.unitX = sel.unitX;
            e.unitY = sel.unitY + 1.0;
            e.z = sel.z + 1;
        } else {
            // 无选中时放到默认行组下方，避免直接盖在速度行上
            e.parentId = null;
            e.unitX = 2.1;
            e.unitY = 8.5;
            e.z = 0;
        }
        host.getDoc().entries.add(e);
        return e;
    }

    private void afterDocChange() {
        host.rebuildSceneFromDoc();
        host.commitLayout();
        host.repaintEditor();
    }

    private void showMenu(MouseEvent e) {
        long now = System.currentTimeMillis();
        if (now - lastMenuMs < 200)
            return;
        lastMenuMs = now;
        JPopupMenu menu = new JPopupMenu();
        MiniHudEntry sel = host.getDoc().find(selectedId);
        if (sel == null) {
            menu.add(buildAddMenu());
            menu.addSeparator();
            JMenuItem reset = new JMenuItem("恢复默认布局");
            reset.addActionListener(ev -> host.resetToDefaultLayout());
            menu.add(reset);
        } else {
            if ("text.row".equals(sel.type)) {
                menu.add(buildFieldMenu("替换字段", this::replaceSelectedField));
                menu.add(buildFieldMenu("追加字段", this::appendDataField));
                JMenuItem text = new JMenuItem("编辑文本…");
                text.addActionListener(ev -> {
                    String v = JOptionPane.showInputDialog(host.getEditorComponent(), "文本内容", "");
                    if (v != null)
                        setSelectedText(v);
                });
                menu.add(text);
                JMenuItem addText = new JMenuItem("追加文本…");
                addText.addActionListener(ev -> {
                    String v = JOptionPane.showInputDialog(host.getEditorComponent(), "文本内容", "");
                    if (v != null)
                        appendStaticText(v);
                });
                menu.add(addText);
                JMenuItem unit = new JMenuItem("编辑单位…");
                unit.addActionListener(ev -> {
                    String v = JOptionPane.showInputDialog(host.getEditorComponent(), "单位", "");
                    if (v != null)
                        setSelectedUnit(v);
                });
                menu.add(unit);
                JMenuItem prec = new JMenuItem("编辑精度…");
                prec.addActionListener(ev -> {
                    String v = JOptionPane.showInputDialog(host.getEditorComponent(), "小数位 0-6", "0");
                    if (v != null) {
                        try {
                            setSelectedPrecision(Integer.parseInt(v.trim()));
                        } catch (NumberFormatException ignore) {
                        }
                    }
                });
                menu.add(prec);
                JMenu align = new JMenu("对齐");
                JMenuItem left = new JMenuItem("左对齐");
                left.addActionListener(ev -> setSelectedAlign('L'));
                align.add(left);
                JMenuItem center = new JMenuItem("居中");
                center.addActionListener(ev -> setSelectedAlign('C'));
                align.add(center);
                JMenuItem right = new JMenuItem("右对齐");
                right.addActionListener(ev -> setSelectedAlign('R'));
                align.add(right);
                menu.add(align);
            }
            if (hasLabel(sel.type)) {
                JMenuItem label = new JMenuItem("编辑标签…");
                label.addActionListener(ev -> {
                    String v = JOptionPane.showInputDialog(host.getEditorComponent(), "标签文字", "");
                    if (v != null)
                        setSelectedLabel(v);
                });
                menu.add(label);
            }
            JMenuItem props = new JMenuItem("条目属性…");
            props.addActionListener(ev -> editSelectedProperties());
            menu.add(props);
            menu.addSeparator();
            JMenuItem dup = new JMenuItem("复制");
            dup.addActionListener(ev -> duplicateSelected());
            menu.add(dup);
            JMenuItem del = new JMenuItem("删除");
            del.addActionListener(ev -> deleteSelected());
            menu.add(del);
            JMenuItem vis = new JMenuItem(sel.visible ? "隐藏" : "显示");
            vis.addActionListener(ev -> toggleSelectedVisible());
            menu.add(vis);
            JMenuItem lock = new JMenuItem(sel.locked ? "解锁" : "锁定");
            lock.addActionListener(ev -> toggleSelectedLocked());
            menu.add(lock);
            menu.addSeparator();
            JMenuItem front = new JMenuItem("置顶");
            front.addActionListener(ev -> bringToFront());
            menu.add(front);
            JMenuItem back = new JMenuItem("置底");
            back.addActionListener(ev -> sendToBack());
            menu.add(back);
        }
        java.awt.Component c = host.getEditorComponent();
        if (c != null)
            menu.show(c, e.getX(), e.getY());
    }

    private void showAddOnlyMenu(int x, int y) {
        JPopupMenu menu = new JPopupMenu();
        menu.add(buildAddMenu());
        java.awt.Component c = host.getEditorComponent();
        if (c != null)
            menu.show(c, x, y);
    }

    private JMenu buildAddMenu() {
        JMenu add = new JMenu("添加条目");

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
            item.addActionListener(ev -> addDataField(def.key));
            group.add(item);
        }
        if (data.getItemCount() == 0) {
            JMenuItem none = new JMenuItem("（无可用字段）");
            none.setEnabled(false);
            data.add(none);
        }
        add.add(data);

        JMenuItem text = new JMenuItem("文本…");
        text.addActionListener(ev -> {
            String v = JOptionPane.showInputDialog(host.getEditorComponent(), "文本内容", "");
            if (v != null)
                addStaticText(v);
        });
        add.add(text);

        JMenu compound = new JMenu("复合组件");
        JMenu gauges = new JMenu("条与仪表");
        for (HUDWidgetSpec spec : HUDWidgetRegistry.all()) {
            if ("text.row".equals(spec.type))
                continue;
            JMenuItem item = new JMenuItem(spec.displayName);
            item.addActionListener(ev -> addEntry(spec.type));
            if ("复合行".equals(spec.category))
                compound.add(item);
            else
                gauges.add(item);
        }
        add.add(compound);
        add.add(gauges);
        return add;
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

    private static boolean hasLabel(String type) {
        return "text.row".equals(type) || "legacy.akb".equals(type) || "legacy.energy".equals(type)
                || "legacy.sep".equals(type) || "legacy.maneuver".equals(type);
    }

    /** 绘制选中框、隐藏占位、工具条（在场景绘制之后调用）。 */
    public void paint(Graphics2D g2d) {
        paintHiddenPlaceholders(g2d);
        if (selectedId != null && host.getScene() != null) {
            MiniHudScene.EntryRuntime rt = host.getScene().byId.get(selectedId);
            if (rt != null) {
                Rectangle r = rt.node.getPixelRect();
                int x = r.x + host.getScene().engine.getRenderOffsetX();
                int y = r.y + host.getScene().engine.getRenderOffsetY();
                g2d.setColor(rt.entry.locked ? LOCK_COLOR : SELECT_COLOR);
                g2d.setStroke(SELECT_STROKE);
                g2d.drawRect(x - 1, y - 1, r.width + 2, r.height + 2);
                // 四角缩放手柄
                g2d.fillRect(x - 3, y - 3, 6, 6);
                g2d.fillRect(x + r.width - 3, y - 3, 6, 6);
                g2d.fillRect(x - 3, y + r.height - 3, 6, 6);
                g2d.fillRect(x + r.width - 3, y + r.height - 3, 6, 6);
            }
        }
        if (resizing && resizePreview != null) {
            int ox = host.getScene().engine.getRenderOffsetX();
            int oy = host.getScene().engine.getRenderOffsetY();
            g2d.setColor(SELECT_COLOR);
            g2d.setStroke(SELECT_STROKE);
            g2d.drawRect(resizePreview.x + ox, resizePreview.y + oy, resizePreview.width, resizePreview.height);
        }
        paintToolbar(g2d);
    }

    private void paintHiddenPlaceholders(Graphics2D g2d) {
        if (host.getScene() == null || readOnly)
            return;
        int ox = host.getScene().engine.getRenderOffsetX();
        int oy = host.getScene().engine.getRenderOffsetY();
        g2d.setColor(HIDDEN_COLOR);
        g2d.setStroke(HIDDEN_STROKE);
        for (MiniHudScene.EntryRuntime rt : host.getScene().byId.values()) {
            if (!rt.entry.visible) {
                Rectangle r = rt.node.getPixelRect();
                g2d.drawRect(r.x + ox, r.y + oy, Math.max(4, r.width), Math.max(4, r.height));
            }
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
            // 只读模式只留「编辑」，添加/默认/适配隐藏且不可点
            tbAdd.setBounds(0, 0, 0, 0);
            tbReset.setBounds(0, 0, 0, 0);
            tbFit.setBounds(0, 0, 0, 0);
            return;
        }
        x = placeButton(g2d, tbAdd, x, y, h, "+ 添加");
        x = placeButton(g2d, tbReset, x, y, h, "默认");
        placeButton(g2d, tbFit, x, y, h, "适配");
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
