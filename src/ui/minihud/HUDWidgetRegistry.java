package ui.minihud;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** 条目类型注册表。type id 一经发布永不复用/改名（序列化兼容）。 */
public final class HUDWidgetRegistry {

    private static final Map<String, HUDWidgetSpec> registry = new LinkedHashMap<>();

    private HUDWidgetRegistry() {
    }

    public static void register(HUDWidgetSpec spec) {
        registry.put(spec.type, spec);
    }

    public static HUDWidgetSpec find(String type) {
        return registry.get(type);
    }

    public static List<HUDWidgetSpec> all() {
        return new ArrayList<>(registry.values());
    }

    public static List<HUDWidgetSpec> category(String category) {
        List<HUDWidgetSpec> out = new ArrayList<>();
        for (HUDWidgetSpec s : registry.values()) {
            if (category.equals(s.category))
                out.add(s);
        }
        return out;
    }
}
