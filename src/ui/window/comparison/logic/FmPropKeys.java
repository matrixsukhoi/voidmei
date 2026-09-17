package ui.window.comparison.logic;

import java.util.Collections;
import java.util.HashMap;
import java.util.Map;

/**
 * FM 属性名稳定键空间。
 *
 * <p>对比规则等逻辑匹配必须用稳定 key, 而 FM 数据行的属性名(冒号前段)是显示文本,
 * 随语言变化 —— 翻译后按显示名匹配会静默失效(规则查不到 → 全部平局灰色)。
 * 本类维护 稳定key ↔ 显示名 的双向映射, 消费方拿显示名反查稳定 key。
 *
 * <p>P0: 显示名为内置中文常量, 必须与 lang/cur.properties 中 b* 格式串的属性行
 * 逐字一致(守护测试校验); i18n 化后切换为 Lang 供给并支持语言热切换 rebuild。
 */
public final class FmPropKeys {

    private FmPropKeys() {
    }

    /** 稳定 key → 显示名(属性名, 冒号前段)。与 cur.properties 格式串逐字一致 */
    private static final Map<String, String> KEY_TO_DISPLAY;
    /** 显示名 → 稳定 key(反向查找表, 只读快照) */
    private static volatile Map<String, String> displayToKey;

    static {
        Map<String, String> m = new HashMap<>();
        m.put("emptyWeight", "空重(kg)");
        m.put("maxFuelWeight", "最大燃油重量(kg)");
        m.put("critSpeed", "临界速度(km/h)");
        m.put("allowLoadFactor", "允许过载(满/半油)");
        m.put("avgHeatRecovery", "平均耐热条恢复速率");
        m.put("maxLiftLoad350", "千米最大升力过载");
        m.put("liftLoadFactor", "主升力面积因数载荷");
        m.put("oswaldEfficiency", "翼展效率");
        m.put("dragAreaFactor", "主阻力面积因数及加速度系数");
        m.put("inducedDragFactor", "诱导阻力因数及加速度系数");
        m.put("radiatorDragCoeff", "散热/油冷器阻力系数");
        KEY_TO_DISPLAY = Collections.unmodifiableMap(m);
        displayToKey = buildReverse();
    }

    private static Map<String, String> buildReverse() {
        Map<String, String> r = new HashMap<>();
        for (Map.Entry<String, String> e : KEY_TO_DISPLAY.entrySet()) {
            r.put(e.getValue(), e.getKey());
        }
        return r;
    }

    /**
     * 由显示名(当前语言的属性行冒号前段)反查稳定 key。
     * @return 稳定 key; 未注册的属性返回 null(调用方按"无规则"处理)
     */
    public static String keyOfDisplay(String display) {
        if (display == null)
            return null;
        return displayToKey.get(display.trim());
    }

    /** 稳定 key → 显示名(调试/测试用) */
    public static String displayOf(String key) {
        return KEY_TO_DISPLAY.get(key);
    }
}
