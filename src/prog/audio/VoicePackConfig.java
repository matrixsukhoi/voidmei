package prog.audio;

/**
 * 语音包配置值对象，封装 "packName|enabled" 格式的解析和序列化
 *
 * 行为契约：
 * - parse("jarvis|true") → VoicePackConfig("jarvis", true)
 * - parse("jarvis") → VoicePackConfig("jarvis", true)  // 默认启用
 * - parse(null) → VoicePackConfig("default", true)
 * - parse("") → VoicePackConfig("default", true)
 * - toConfigString() → "packName|enabled"
 *
 * 这是一个不可变的值对象，线程安全。
 */
public final class VoicePackConfig {
    /** 默认语音包名称 */
    public static final String DEFAULT_PACK = "default";
    /** 配置键前缀 */
    public static final String VOICE_PREFIX = "voice_";

    /** 语音包名称 */
    public final String packName;
    /** 是否启用 */
    public final boolean enabled;

    /**
     * 构造函数
     * @param packName 语音包名称，null 或空字符串会被替换为 "default"
     * @param enabled 是否启用
     */
    public VoicePackConfig(String packName, boolean enabled) {
        this.packName = (packName == null || packName.isEmpty()) ? DEFAULT_PACK : packName;
        this.enabled = enabled;
    }

    /**
     * 解析配置字符串
     * 格式: "packName|enabled" 或 "packName"
     *
     * @param configValue 配置值，可以为 null
     * @return 解析后的配置对象
     */
    public static VoicePackConfig parse(String configValue) {
        String packName = DEFAULT_PACK;
        boolean enabled = true;

        if (configValue != null && !configValue.isEmpty()) {
            if (configValue.contains("|")) {
                String[] parts = configValue.split("\\|", 2);
                packName = parts[0];
                if (parts.length > 1) {
                    enabled = Boolean.parseBoolean(parts[1]);
                }
            } else {
                packName = configValue;
            }
        }

        return new VoicePackConfig(packName, enabled);
    }

    /**
     * 序列化为配置字符串
     * @return 格式: "packName|enabled"
     */
    public String toConfigString() {
        return packName + "|" + enabled;
    }

    /**
     * 创建启用状态变更后的新实例
     * @param newEnabled 新的启用状态
     * @return 新实例
     */
    public VoicePackConfig withEnabled(boolean newEnabled) {
        return new VoicePackConfig(this.packName, newEnabled);
    }

    /**
     * 创建包名变更后的新实例
     * @param newPackName 新的包名
     * @return 新实例
     */
    public VoicePackConfig withPackName(String newPackName) {
        return new VoicePackConfig(newPackName, this.enabled);
    }

    /**
     * 剥离 voice_ 前缀
     * @param key 配置键
     * @return 剥离前缀后的键
     */
    public static String stripVoicePrefix(String key) {
        if (key != null && key.startsWith(VOICE_PREFIX)) {
            return key.substring(VOICE_PREFIX.length());
        }
        return key;
    }

    /**
     * 添加 voice_ 前缀
     * @param key 告警键
     * @return 带前缀的配置键
     */
    public static String withVoicePrefix(String key) {
        if (key != null && !key.startsWith(VOICE_PREFIX)) {
            return VOICE_PREFIX + key;
        }
        return key;
    }

    @Override
    public String toString() {
        return "VoicePackConfig{packName='" + packName + "', enabled=" + enabled + "}";
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null || getClass() != obj.getClass()) return false;
        VoicePackConfig that = (VoicePackConfig) obj;
        return enabled == that.enabled && packName.equals(that.packName);
    }

    @Override
    public int hashCode() {
        return 31 * packName.hashCode() + (enabled ? 1 : 0);
    }
}
