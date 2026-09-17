package ui.window.comparison.logic;

import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

import prog.i18n.Lang;

/**
 * FM 属性名稳定键空间。
 *
 * <p>对比规则等逻辑匹配必须用稳定 key, 而 FM 数据行的属性名(冒号前段)是显示文本,
 * 随语言变化 —— 翻译后按显示名匹配会静默失效(规则查不到 → 全部平局灰色)。
 * 本类维护 稳定key ↔ 显示名 的双向映射, 消费方拿显示名反查稳定 key。
 *
 * <p>显示名经 Lang("fm.prop.<稳定key>")取得, 与 b* 格式串的属性行逐字一致
 * (守护测试校验); 反查表随语言热切换懒重建。
 */
public final class FmPropKeys {

    private FmPropKeys() {
    }

    /** 已注册的稳定 key 清单 */
    private static final List<String> STABLE_KEYS = Arrays.asList(
            "emptyWeight", "maxFuelWeight", "critSpeed", "allowLoadFactor",
            "avgHeatRecovery", "maxLiftLoad350", "liftLoadFactor", "oswaldEfficiency",
            "dragAreaFactor", "inducedDragFactor", "radiatorDragCoeff");

    /** 语言文件缺失时的中文兜底(与 zh.properties 的 fm.prop.* 逐字一致)。
     *  注意: 声明必须先于下方使用它的 static 块(静态初始化按源码顺序执行) */
    private static final Map<String, String> FALLBACK_ZH;
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
        FALLBACK_ZH = m;
    }

    /** 显示名 → 稳定 key(当前语言, 语言切换后由 ensureReverse 懒重建) */
    private static volatile Map<String, String> displayToKey = new HashMap<>();
    private static volatile String builtForLocale = "";

    static {
        for (String k : STABLE_KEYS) {
            displayToKey.put(displayOf(k), k);
        }
        builtForLocale = Lang.locale();
    }

    /** 语言热切换后首次调用时重建反向表(volatile 写, 线程安全) */
    private static void ensureReverse() {
        String loc = Lang.locale();
        if (!loc.equals(builtForLocale)) {
            Map<String, String> r = new HashMap<>();
            for (String k : STABLE_KEYS) {
                r.put(displayOf(k), k);
            }
            displayToKey = r;
            builtForLocale = loc;
        }
    }

    /**
     * 由显示名(当前语言的属性行冒号前段)反查稳定 key。
     * @return 稳定 key; 未注册的属性返回 null(调用方按"无规则"处理)
     */
    public static String keyOfDisplay(String display) {
        if (display == null)
            return null;
        ensureReverse();
        return displayToKey.get(display.trim());
    }

    /** 稳定 key → 显示名(经 Lang, 语言包缺失时回退内置中文) */
    public static String displayOf(String key) {
        return Lang.ui("fm.prop." + key, FALLBACK_ZH.get(key));
    }

    /** 全部稳定 key(守护测试用) */
    public static Set<String> stableKeys() {
        return new HashSet<>(STABLE_KEYS);
    }
}
