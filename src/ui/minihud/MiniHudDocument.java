package ui.minihud;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * MiniHUD 自由画布文档。
 * 所有组件都在同一个无限画布坐标系里；窗口只是内容包围盒的视口。
 */
public final class MiniHudDocument {

    public static final int VERSION = 3;
    public static final int MAX_WIDGETS = 300;
    public static final double MIN_SCALE = 0.25;
    public static final double MAX_SCALE = 4.0;
    private static final double COORD_LIMIT = 100000.0;

    public int version = VERSION;
    public final List<MiniHudWidget> widgets = new ArrayList<>();
    public final List<MiniHudGroup> groups = new ArrayList<>();

    public MiniHudWidget find(String id) {
        if (id == null)
            return null;
        for (MiniHudWidget w : widgets) {
            if (id.equals(w.id))
                return w;
        }
        return null;
    }

    public MiniHudGroup findGroup(String id) {
        if (id == null)
            return null;
        for (MiniHudGroup g : groups) {
            if (id.equals(g.id))
                return g;
        }
        return null;
    }

    public int indexOf(String id) {
        for (int i = 0; i < widgets.size(); i++) {
            if (widgets.get(i).id.equals(id))
                return i;
        }
        return -1;
    }

    public List<MiniHudWidget> membersOf(String groupId) {
        List<MiniHudWidget> out = new ArrayList<>();
        if (groupId == null)
            return out;
        for (MiniHudWidget w : widgets) {
            if (groupId.equals(w.groupId))
                out.add(w);
        }
        return out;
    }

    public boolean remove(String id) {
        int idx = indexOf(id);
        if (idx < 0)
            return false;
        widgets.remove(idx);
        return true;
    }

    public String nextWidgetId() {
        int i = 1;
        while (find("w" + i) != null)
            i++;
        return "w" + i;
    }

    public String nextGroupId() {
        int i = 1;
        while (findGroup("g" + i) != null)
            i++;
        return "g" + i;
    }

    /** 修复坏数据：ID 去重、未知类型回退文本、坐标/缩放钳制、组引用补齐、数量截断。 */
    public void validate() {
        if (version <= 0)
            version = VERSION;
        if (widgets.size() > MAX_WIDGETS)
            widgets.subList(MAX_WIDGETS, widgets.size()).clear();

        Set<String> ids = new HashSet<>();
        Set<String> groupIds = new HashSet<>();
        for (MiniHudGroup g : groups) {
            if (g.id == null || g.id.isEmpty() || !groupIds.add(g.id)) {
                String nid;
                int i = 1;
                do {
                    nid = "g" + i++;
                } while (groupIds.contains(nid));
                g.id = nid;
                groupIds.add(nid);
            }
            if (g.name == null)
                g.name = g.id;
        }

        int auto = 1;
        for (MiniHudWidget w : widgets) {
            if (w.type == null || w.type.isEmpty())
                w.type = MiniHudProps.TYPE_TEXT;
            if (w.id == null || w.id.isEmpty() || !ids.add(w.id)) {
                String nid;
                do {
                    nid = "w" + auto++;
                } while (ids.contains(nid));
                w.id = nid;
                ids.add(nid);
            }
            if (Double.isNaN(w.x) || Double.isInfinite(w.x))
                w.x = 0;
            if (Double.isNaN(w.y) || Double.isInfinite(w.y))
                w.y = 0;
            w.x = clamp(w.x, -COORD_LIMIT, COORD_LIMIT);
            w.y = clamp(w.y, -COORD_LIMIT, COORD_LIMIT);
            w.scale = clamp(w.scale, MIN_SCALE, MAX_SCALE);
            if (w.groupId != null && !groupIds.contains(w.groupId)) {
                groups.add(new MiniHudGroup(w.groupId, w.groupId));
                groupIds.add(w.groupId);
            }
        }
        // 去掉没有任何成员的组，避免图层面板出现空组
        Set<String> usedGroups = new HashSet<>();
        for (MiniHudWidget w : widgets) {
            if (w.groupId != null)
                usedGroups.add(w.groupId);
        }
        groups.removeIf(g -> !usedGroups.contains(g.id));
    }

    public MiniHudDocument copy() {
        MiniHudDocument d = new MiniHudDocument();
        d.version = version;
        for (MiniHudGroup g : groups)
            d.groups.add(g.copy());
        for (MiniHudWidget w : widgets)
            d.widgets.add(w.copy(w.id));
        return d;
    }

    private static double clamp(double v, double min, double max) {
        if (v < min)
            return min;
        if (v > max)
            return max;
        return v;
    }
}
