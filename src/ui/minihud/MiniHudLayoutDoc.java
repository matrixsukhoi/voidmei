package ui.minihud;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * MiniHUD 自定义布局文档。
 * 只描述条目、父子关系、几何与条目属性；全局字体/颜色/DPI 仍由设置与 Context 负责。
 */
public final class MiniHudLayoutDoc {

    public static final int VERSION = 1;
    public static final int MAX_ENTRIES = 200;
    public static final double MIN_SCALE = 0.25;
    public static final double MAX_SCALE = 4.0;

    public int version = VERSION;
    public final List<MiniHudEntry> entries = new ArrayList<>();

    public MiniHudEntry find(String id) {
        if (id == null)
            return null;
        for (MiniHudEntry e : entries) {
            if (id.equals(e.id))
                return e;
        }
        return null;
    }

    public int indexOf(String id) {
        for (int i = 0; i < entries.size(); i++) {
            if (entries.get(i).id.equals(id))
                return i;
        }
        return -1;
    }

    public List<MiniHudEntry> childrenOf(String id) {
        List<MiniHudEntry> result = new ArrayList<>();
        for (MiniHudEntry e : entries) {
            if (id == null ? e.parentId == null : id.equals(e.parentId))
                result.add(e);
        }
        return result;
    }

    /** 删除条目及其整棵子树。 */
    public boolean remove(String id) {
        MiniHudEntry target = find(id);
        if (target == null)
            return false;
        Set<String> doomed = new HashSet<>();
        doomed.add(id);
        boolean grew = true;
        while (grew) {
            grew = false;
            for (MiniHudEntry e : entries) {
                if (e.parentId != null && doomed.contains(e.parentId) && doomed.add(e.id))
                    grew = true;
            }
        }
        entries.removeIf(e -> doomed.contains(e.id));
        return true;
    }

    public String nextId() {
        int i = 1;
        while (find("e" + i) != null)
            i++;
        return "e" + i;
    }

    /**
     * 修复坏数据：ID 去重、父级缺失/成环降根、scale 钳制、上限截断。
     * 必须在构建场景前调用，避免布局引擎 DFS 栈溢出。
     */
    public void validate() {
        version = VERSION;
        if (entries.size() > MAX_ENTRIES) {
            entries.subList(MAX_ENTRIES, entries.size()).clear();
        }
        Set<String> ids = new HashSet<>();
        int auto = 1;
        for (MiniHudEntry e : entries) {
            if (e.type == null || e.type.isEmpty())
                e.type = "text.row";
            if (e.id == null || e.id.isEmpty() || !ids.add(e.id)) {
                String nid;
                do {
                    nid = "e" + auto++;
                } while (ids.contains(nid));
                e.id = nid;
                ids.add(nid);
            }
            if (e.scale < MIN_SCALE)
                e.scale = MIN_SCALE;
            if (e.scale > MAX_SCALE)
                e.scale = MAX_SCALE;
            if (e.parentAnchor == null)
                e.parentAnchor = ui.layout.Anchor.TOP_LEFT;
            if (e.selfAnchor == null)
                e.selfAnchor = ui.layout.Anchor.TOP_LEFT;
        }
        // 父级不存在或指向自己 → 降为根
        for (MiniHudEntry e : entries) {
            if (e.parentId != null && (e.parentId.equals(e.id) || find(e.parentId) == null))
                e.parentId = null;
        }
        // 断环：父链走不到根就把当前节点降根
        for (MiniHudEntry e : entries) {
            Set<String> seen = new HashSet<>();
            seen.add(e.id);
            String p = e.parentId;
            while (p != null) {
                if (!seen.add(p)) {
                    e.parentId = null;
                    break;
                }
                MiniHudEntry pe = find(p);
                p = pe != null ? pe.parentId : null;
            }
        }
    }

    public MiniHudLayoutDoc copy() {
        MiniHudLayoutDoc d = new MiniHudLayoutDoc();
        d.version = version;
        for (MiniHudEntry e : entries)
            d.entries.add(e.copy(e.id));
        return d;
    }
}
