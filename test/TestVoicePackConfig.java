import prog.audio.VoicePackConfig;
import prog.audio.VoiceAlertType;

import java.util.List;

/**
 * Tests for VoicePackConfig and VoiceAlertType.
 *
 * Validates that the new classes match the original parsing behavior exactly.
 *
 * Run with: ./script/test.sh voicepack
 */
public class TestVoicePackConfig {

    private static int passed = 0;
    private static int failed = 0;

    public static void main(String[] args) {
        System.out.println("=== VoicePackConfig Tests ===\n");

        testBasicParsing();
        testNullAndEmptyParsing();
        testSerialization();
        testWithMethods();
        testPrefixMethods();
        testParsingConsistencyWithOriginal();

        System.out.println("\n=== VoiceAlertType Tests ===\n");

        testAlertTypeKeys();
        testAlertTypeCooldowns();
        testAlertTypeCount();

        System.out.println("\n=== Results ===");
        System.out.printf("Passed: %d, Failed: %d%n", passed, failed);

        if (failed > 0) {
            System.exit(1);
        }
    }

    // ==================== VoicePackConfig Tests ====================

    private static void testBasicParsing() {
        System.out.println("Testing basic parsing...");

        // Parse with enabled=true
        VoicePackConfig config = VoicePackConfig.parse("jarvis|true");
        assertEquals("parse jarvis|true packName", "jarvis", config.packName);
        assertTrue("parse jarvis|true enabled", config.enabled);

        // Parse with enabled=false
        config = VoicePackConfig.parse("jarvis|false");
        assertEquals("parse jarvis|false packName", "jarvis", config.packName);
        assertFalse("parse jarvis|false enabled", config.enabled);

        // Parse without enabled (defaults to true)
        config = VoicePackConfig.parse("jarvis");
        assertEquals("parse jarvis packName", "jarvis", config.packName);
        assertTrue("parse jarvis enabled (default)", config.enabled);
    }

    private static void testNullAndEmptyParsing() {
        System.out.println("Testing null and empty parsing...");

        // Parse null
        VoicePackConfig config = VoicePackConfig.parse(null);
        assertEquals("parse null packName", "default", config.packName);
        assertTrue("parse null enabled", config.enabled);

        // Parse empty string
        config = VoicePackConfig.parse("");
        assertEquals("parse empty packName", "default", config.packName);
        assertTrue("parse empty enabled", config.enabled);
    }

    private static void testSerialization() {
        System.out.println("Testing serialization...");

        VoicePackConfig config = new VoicePackConfig("jarvis", false);
        assertEquals("toConfigString", "jarvis|false", config.toConfigString());

        config = new VoicePackConfig("default", true);
        assertEquals("toConfigString default|true", "default|true", config.toConfigString());
    }

    private static void testWithMethods() {
        System.out.println("Testing with* methods...");

        VoicePackConfig original = new VoicePackConfig("jarvis", true);

        // withEnabled
        VoicePackConfig updated = original.withEnabled(false);
        assertEquals("withEnabled packName unchanged", "jarvis", updated.packName);
        assertFalse("withEnabled new value", updated.enabled);
        assertTrue("withEnabled original unchanged", original.enabled);

        // withPackName
        updated = original.withPackName("custom");
        assertEquals("withPackName new value", "custom", updated.packName);
        assertEquals("withPackName original unchanged", "jarvis", original.packName);
    }

    private static void testPrefixMethods() {
        System.out.println("Testing prefix methods...");

        // stripVoicePrefix
        assertEquals("strip voice_aoaCrit", "aoaCrit", VoicePackConfig.stripVoicePrefix("voice_aoaCrit"));
        assertEquals("strip aoaCrit (no prefix)", "aoaCrit", VoicePackConfig.stripVoicePrefix("aoaCrit"));
        assertEquals("strip null", null, VoicePackConfig.stripVoicePrefix(null));

        // withVoicePrefix
        assertEquals("with aoaCrit", "voice_aoaCrit", VoicePackConfig.withVoicePrefix("aoaCrit"));
        assertEquals("with voice_aoaCrit (already has)", "voice_aoaCrit", VoicePackConfig.withVoicePrefix("voice_aoaCrit"));
        assertEquals("with null", null, VoicePackConfig.withVoicePrefix(null));
    }

    /**
     * 验证新解析逻辑与原代码完全一致
     * 原代码位于 VoiceWarning.audClip.reload() 第 90-116 行
     */
    private static void testParsingConsistencyWithOriginal() {
        System.out.println("Testing parsing consistency with original code...");

        String[] testCases = {
            "jarvis|true",
            "jarvis|false",
            "jarvis",
            "default|true",
            "default|false",
            "custom_pack|true",
            "",
            null
        };

        for (String val : testCases) {
            // 原逻辑 (从 VoiceWarning.audClip.reload() 复制)
            String oldPackName = "default";
            boolean oldEnabled = true;
            if (val != null && !val.isEmpty()) {
                if (val.contains("|")) {
                    String[] parts = val.split("\\|");
                    oldPackName = parts[0];
                    if (parts.length > 1)
                        oldEnabled = Boolean.parseBoolean(parts[1]);
                } else {
                    oldPackName = val;
                }
            }

            // 新逻辑
            VoicePackConfig config = VoicePackConfig.parse(val);

            String testName = "consistency: " + (val == null ? "null" : "\"" + val + "\"");
            assertEquals(testName + " packName", oldPackName, config.packName);
            assertEquals(testName + " enabled", oldEnabled, config.enabled);
        }
    }

    // ==================== VoiceAlertType Tests ====================

    private static void testAlertTypeKeys() {
        System.out.println("Testing alert type keys...");

        // 验证关键的告警类型存在
        assertEquals("AOA_CRIT key", "aoaCrit", VoiceAlertType.AOA_CRIT.getKey());
        assertEquals("WARN_GEAR key", "warn_gear", VoiceAlertType.WARN_GEAR.getKey());
        assertEquals("FAIL_ENGINE key", "fail_engine", VoiceAlertType.FAIL_ENGINE.getKey());
        assertEquals("START1 key", "start1", VoiceAlertType.START1.getKey());

        // 验证 fromKey 查找
        assertEquals("fromKey aoaCrit", VoiceAlertType.AOA_CRIT, VoiceAlertType.fromKey("aoaCrit"));
        assertEquals("fromKey nonexistent", null, VoiceAlertType.fromKey("nonexistent"));
    }

    private static void testAlertTypeCooldowns() {
        System.out.println("Testing alert type cooldowns...");

        assertEquals("AOA_CRIT cooldown", 1, VoiceAlertType.AOA_CRIT.getCooldownSeconds());
        assertEquals("AOA_HIGH cooldown", 8, VoiceAlertType.AOA_HIGH.getCooldownSeconds());
        assertEquals("WARN_ENGINEOVERHEAT cooldown", 60, VoiceAlertType.WARN_ENGINEOVERHEAT.getCooldownSeconds());
        assertEquals("WARN_COMPRESSOR cooldown", 0, VoiceAlertType.WARN_COMPRESSOR.getCooldownSeconds());

        // 验证毫秒转换
        assertEquals("AOA_CRIT cooldown ms", 1000L, VoiceAlertType.AOA_CRIT.getCooldownMs());
        assertEquals("WARN_ENGINEOVERHEAT cooldown ms", 60000L, VoiceAlertType.WARN_ENGINEOVERHEAT.getCooldownMs());
    }

    private static void testAlertTypeCount() {
        System.out.println("Testing alert type count...");

        // 原硬编码列表 (从 VoiceGlobalRenderer 复制)
        String[] originalKeys = {
            "aoaCrit", "aoaHigh", "warn_stall", "warn_gear",
            "warn_engineoverheat", "warn_lowfuel", "warn_altitude",
            "warn_terrain", "warn_flap", "warn_loadfactor",
            "rudderEff", "elevatorEff", "aileronEff", "warn_lowrpm",
            "warn_highrpm", "warn_ias", "warn_mach", "fail_engine",
            "warn_lowpressure", "fail_nofuel", "warn_highvario",
            "warn_brake", "warn_compressor", "start1"
        };

        List<String> newKeys = VoiceAlertType.getAllKeys();

        assertEquals("key count", originalKeys.length, newKeys.size());

        // 验证所有原始 key 都存在
        for (String key : originalKeys) {
            assertTrue("contains key: " + key, newKeys.contains(key));
        }

        // 验证 getAlertCount
        assertEquals("getAlertCount", 24, VoiceAlertType.getAlertCount());
    }

    // ==================== Assertion Helpers ====================

    private static void assertTrue(String name, boolean condition) {
        if (condition) {
            System.out.println("  PASS: " + name);
            passed++;
        } else {
            System.out.println("  FAIL: " + name + " - expected true but got false");
            failed++;
        }
    }

    private static void assertFalse(String name, boolean condition) {
        if (!condition) {
            System.out.println("  PASS: " + name);
            passed++;
        } else {
            System.out.println("  FAIL: " + name + " - expected false but got true");
            failed++;
        }
    }

    private static void assertEquals(String name, Object expected, Object actual) {
        boolean equal = (expected == null && actual == null) ||
                        (expected != null && expected.equals(actual));
        if (equal) {
            System.out.println("  PASS: " + name);
            passed++;
        } else {
            System.out.println("  FAIL: " + name + " - expected " + expected + " but got " + actual);
            failed++;
        }
    }
}
