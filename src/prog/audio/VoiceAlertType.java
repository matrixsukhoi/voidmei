package prog.audio;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * 语音告警类型枚举
 * 集中定义所有告警的 key 和默认冷却时间
 *
 * 告警分类：
 * - 攻角类 (2): aoaCrit, aoaHigh
 * - 速度类 (3): warn_ias, warn_mach, warn_stall
 * - 结构类 (4): warn_gear, warn_flap, warn_loadfactor, warn_brake
 * - 引擎类 (6): warn_engineoverheat, fail_engine, warn_lowrpm, warn_highrpm, warn_lowpressure, warn_compressor
 * - 燃油类 (2): warn_lowfuel, fail_nofuel
 * - 高度类 (3): warn_altitude, warn_terrain, warn_highvario
 * - 舵效类 (3): rudderEff, elevatorEff, aileronEff
 * - 启动音效 (1): start1
 */
public enum VoiceAlertType {
    // 攻角类
    AOA_CRIT("aoaCrit", 1),
    AOA_HIGH("aoaHigh", 8),

    // 速度类
    WARN_IAS("warn_ias", 10),
    WARN_MACH("warn_mach", 10),
    WARN_STALL("warn_stall", 2),

    // 结构类
    WARN_GEAR("warn_gear", 7),
    WARN_FLAP("warn_flap", 1),
    WARN_LOADFACTOR("warn_loadfactor", 2),
    WARN_BRAKE("warn_brake", 8),

    // 引擎类
    WARN_ENGINEOVERHEAT("warn_engineoverheat", 60),
    FAIL_ENGINE("fail_engine", 60),
    WARN_LOWRPM("warn_lowrpm", 10),
    WARN_HIGHRPM("warn_highrpm", 10),
    WARN_LOWPRESSURE("warn_lowpressure", 30),
    WARN_COMPRESSOR("warn_compressor", 0),  // 状态驱动，无冷却

    // 燃油类
    WARN_LOWFUEL("warn_lowfuel", 60),
    FAIL_NOFUEL("fail_nofuel", 60),

    // 高度类
    WARN_ALTITUDE("warn_altitude", 5),
    WARN_TERRAIN("warn_terrain", 5),
    WARN_HIGHVARIO("warn_highvario", 5),

    // 舵效类
    RUDDER_EFF("rudderEff", 10),
    ELEVATOR_EFF("elevatorEff", 10),
    AILERON_EFF("aileronEff", 10),

    // 启动音效
    START1("start1", 1);

    private final String key;
    private final int cooldownSeconds;

    // 缓存的 key 列表，避免重复计算
    private static final List<String> CONFIGURABLE_KEYS;

    static {
        // 静态初始化，Java 8 兼容
        List<String> keys = new ArrayList<>();
        for (VoiceAlertType type : values()) {
            if (!keys.contains(type.key)) {
                keys.add(type.key);
            }
        }
        CONFIGURABLE_KEYS = Collections.unmodifiableList(keys);
    }

    VoiceAlertType(String key, int cooldownSeconds) {
        this.key = key;
        this.cooldownSeconds = cooldownSeconds;
    }

    /**
     * 获取告警键名
     * @return 告警键名，如 "aoaCrit"
     */
    public String getKey() {
        return key;
    }

    /**
     * 获取冷却时间（秒）
     * @return 冷却时间秒数
     */
    public int getCooldownSeconds() {
        return cooldownSeconds;
    }

    /**
     * 获取冷却时间（毫秒）
     * @return 冷却时间毫秒数
     */
    public long getCooldownMs() {
        return cooldownSeconds * 1000L;
    }

    /**
     * 获取所有用于 UI 配置的告警 key 列表
     * 注意：每个 key 只出现一次（如 fail_engine 只列一次）
     *
     * @return 不可变的 key 列表
     */
    public static List<String> getConfigurableKeys() {
        return CONFIGURABLE_KEYS;
    }

    /**
     * 获取所有 key 列表（等同于 getConfigurableKeys，用于 VoiceGlobalRenderer）
     * @return 不可变的所有 key 列表
     */
    public static List<String> getAllKeys() {
        return CONFIGURABLE_KEYS;
    }

    /**
     * 根据 key 查找告警类型
     * @param key 告警键名
     * @return 对应的枚举值，找不到返回 null
     */
    public static VoiceAlertType fromKey(String key) {
        if (key == null) return null;
        for (VoiceAlertType type : values()) {
            if (type.key.equals(key)) {
                return type;
            }
        }
        return null;
    }

    /**
     * 获取告警总数
     * @return 可配置的告警数量
     */
    public static int getAlertCount() {
        return CONFIGURABLE_KEYS.size();
    }
}
