package ui.minihud;

/** 显式分组元信息。组不是坐标父级，组内组件仍保存绝对坐标。 */
public final class MiniHudGroup {
    public String id;
    public String name;

    public MiniHudGroup(String id, String name) {
        this.id = id;
        this.name = name;
    }

    public MiniHudGroup copy() {
        return new MiniHudGroup(id, name);
    }
}
