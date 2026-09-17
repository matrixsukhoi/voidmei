import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;
import java.util.TreeSet;

import prog.config.ConfigLoader;
import prog.i18n.Lang;
import ui.window.comparison.logic.ComparisonRules;
import ui.window.comparison.logic.FmPropKeys;

/**
 * i18n 守护测试 —— 防翻译破坏功能/防中文逻辑键回归
 *
 * 五道闸门:
 * 1. 三语言包 key 集合一致(漏翻=红灯)
 * 2. ui_layout.cfg 的 @key 引用全部存在于 zh 基准包
 * 3. ComparisonRules 稳定键自洽: 全非中文 + 显示名可反查回自身(三语言)
 * 4. panel :id 唯一且非中文; Controller.getOverlaySettings 参数全为稳定 id
 * 5. fm.prop.* 显示名与 b* 格式串属性行逐字一致(三语言, 翻译漂移=对比规则静默失效)
 *
 * 运行前提: cwd = 项目根。运行方式: python script/build.py test i18n-guard
 */
public class TestI18nGuard {

    private static int passed = 0;
    private static int failed = 0;
    private static final List<String> PROBLEMS = new ArrayList<>();

    public static void main(String[] args) {
        System.out.println("=== i18n 守护测试 ===\n");

        try {
            gate1_KeySetConsistency();
            gate2_DslKeyReferences();
            gate3_ComparisonStableKeys();
            gate4_PanelIdsAndCallers();
            gate5_FmPropFormatSync();
        } catch (Exception e) {
            failed++;
            PROBLEMS.add("守护测试异常中断: " + e);
        }

        System.out.println("\n=== 测试结果 ===");
        for (String p : PROBLEMS)
            System.out.println("[问题] " + p);
        System.out.println("通过: " + passed);
        System.out.println("失败: " + failed);

        if (failed > 0) {
            System.exit(1);
        }
    }

    // ---------- 工具 ----------

    private static Set<String> propKeys(String path) throws Exception {
        Set<String> keys = new LinkedHashSet<>();
        File f = new File(path);
        if (!f.exists())
            return keys;
        try (BufferedReader br = new BufferedReader(
                new InputStreamReader(new FileInputStream(f), StandardCharsets.UTF_8))) {
            String line;
            while ((line = br.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty() || line.startsWith("#") || !line.contains("="))
                    continue;
                keys.add(line.substring(0, line.indexOf('=')).trim());
            }
        }
        return keys;
    }

    private static boolean hasChinese(String s) {
        for (char c : s.toCharArray())
            if (c >= 0x4E00 && c <= 0x9FFF)
                return true;
        return false;
    }

    private static void check(boolean ok, String gate, String problem) {
        if (ok) {
            passed++;
        } else {
            failed++;
            PROBLEMS.add("[" + gate + "] " + problem);
        }
    }

    // ---------- 闸门 1: 三包 key 集合一致 ----------

    private static void gate1_KeySetConsistency() throws Exception {
        System.out.println("-- 闸门1: 三包 key 集合一致 --");
        Set<String> zh = propKeys("lang/zh.properties");
        Set<String> en = propKeys("lang/en.properties");
        Set<String> ru = propKeys("lang/ru.properties");
        check(!zh.isEmpty(), "1", "zh 基准包为空(文件缺失?)");

        Set<String> enMissing = new TreeSet<>(zh);
        enMissing.removeAll(en);
        Set<String> ruMissing = new TreeSet<>(zh);
        ruMissing.removeAll(ru);
        Set<String> enExtra = new TreeSet<>(en);
        enExtra.removeAll(zh);
        check(enMissing.isEmpty(), "1", "en 缺 key: " + head(enMissing));
        check(ruMissing.isEmpty(), "1", "ru 缺 key: " + head(ruMissing));
        check(enExtra.isEmpty(), "1", "en 多余 key: " + head(enExtra));
        if (enMissing.isEmpty() && ruMissing.isEmpty() && enExtra.isEmpty())
            passed++; // 三包一致计数
    }

    private static String head(Set<String> s) {
        int i = 0;
        StringBuilder sb = new StringBuilder();
        for (String k : s) {
            sb.append(k).append(' ');
            if (++i >= 5) {
                sb.append("... 共").append(s.size());
                break;
            }
        }
        return sb.toString();
    }

    // ---------- 闸门 2: DSL @key 引用存在于 zh 包 ----------

    private static void gate2_DslKeyReferences() throws Exception {
        System.out.println("-- 闸门2: DSL @key 引用有效 --");
        Set<String> zh = propKeys("lang/zh.properties");
        Set<String> refs = new TreeSet<>();
        try (BufferedReader br = new BufferedReader(
                new InputStreamReader(new FileInputStream("ui_layout.cfg"), StandardCharsets.UTF_8))) {
            String line;
            while ((line = br.readLine()) != null) {
                int at = line.indexOf("\"@");
                while (at >= 0) {
                    int end = line.indexOf('"', at + 2);
                    if (end < 0)
                        break;
                    refs.add(line.substring(at + 2, end));
                    at = line.indexOf("\"@", end);
                }
            }
        }
        check(refs.size() > 100, "2", "@key 引用数异常偏少: " + refs.size() + "(抽取脚本没跑?)");
        Set<String> unknown = new TreeSet<>(refs);
        unknown.removeAll(zh);
        check(unknown.isEmpty(), "2", "@key 引用不在 zh 包: " + head(unknown));
    }

    // ---------- 闸门 3: 对比规则稳定键(禁中文逻辑键) ----------

    private static void gate3_ComparisonStableKeys() {
        System.out.println("-- 闸门3: 对比规则稳定键 --");
        // 规则必须真实存在(防止注册表被清空后测试空转)
        int ruleCount = 0;
        for (String k : FmPropKeys.stableKeys()) {
            if (ComparisonRules.hasRule(FmPropKeys.displayOf(k)))
                ruleCount++;
        }
        check(ruleCount >= 11, "3", "稳定键规则数不足: " + ruleCount + "/11");

        for (String key : FmPropKeys.stableKeys()) {
            check(!hasChinese(key), "3", "稳定键含中文: " + key);
        }
        // 三语言下显示名必须能反查回自身(注册表↔语言包同步)
        for (String loc : Arrays.asList("zh", "en", "ru")) {
            Lang.initLang(loc);
            for (String key : FmPropKeys.stableKeys()) {
                String display = FmPropKeys.displayOf(key);
                check(!display.isEmpty(), "3", "[" + loc + "] " + key + " 显示名为空");
                if (!key.equals(FmPropKeys.keyOfDisplay(display))) {
                    check(false, "3", "[" + loc + "] 显示名反查失配: " + key + " -> \"" + display + "\"");
                    return; // 反查已坏, 继续无意义
                }
            }
        }
        Lang.initLang("zh");
        passed++; // 三语言反查全过计数
    }

    // ---------- 闸门 4: panel id 唯一 + 调用方参数守卫 ----------

    private static void gate4_PanelIdsAndCallers() throws Exception {
        System.out.println("-- 闸门4: panel id 与调用方 --");
        List<ConfigLoader.GroupConfig> panels = ConfigLoader.loadConfig("ui_layout.cfg");
        Set<String> ids = new HashSet<>();
        int withId = 0;
        for (ConfigLoader.GroupConfig g : panels) {
            if (g.id != null) {
                withId++;
                check(!hasChinese(g.id), "4", "panel id 含中文: " + g.id);
                check(ids.add(g.id), "4", "panel id 重复: " + g.id);
            }
        }
        check(panels.size() >= 12 && withId == panels.size(), "4",
                "存在无 :id 的 panel(" + withId + "/" + panels.size() + ")");

        // Controller 源码里 getOverlaySettings("...") 参数必须走稳定 id(防中文键回归)
        Set<String> allowed = new HashSet<>(ids);
        allowed.add("StatusBar"); // 特例: 无 panel 匹配, 恒居中 fallback
        File src = new File("src/prog/Controller.java");
        if (src.exists()) {
            try (BufferedReader br = new BufferedReader(
                    new InputStreamReader(new FileInputStream(src), StandardCharsets.UTF_8))) {
                String line;
                while ((line = br.readLine()) != null) {
                    int i = line.indexOf("getOverlaySettings(\"");
                    while (i >= 0) {
                        int end = line.indexOf('"', i + 20);
                        if (end < 0)
                            break;
                        String arg = line.substring(i + 20, end);
                        check(allowed.contains(arg), "4", "getOverlaySettings 参数非法: \"" + arg + "\"");
                        i = line.indexOf("getOverlaySettings(\"", end);
                    }
                }
            }
        }
        passed++; // 扫描完成计数
    }

    // ---------- 闸门 5: fm.prop 显示名 ↔ b* 格式串属性行逐字一致 ----------

    private static void gate5_FmPropFormatSync() {
        System.out.println("-- 闸门5: fm.prop ↔ 格式串同步 --");
        // (稳定键, 格式串字段名, 属性行在格式串中的下标)
        String[][] pairs = {
                { "emptyWeight", "bWeight", "0" }, { "maxFuelWeight", "bWeight", "1" },
                { "critSpeed", "bCritSpeed", "0" }, { "allowLoadFactor", "bAllowLoadFactor", "0" },
                { "avgHeatRecovery", "bAverageHeatRecovery", "0" },
                { "maxLiftLoad350", "bMaxLiftLoad350", "0" },
                { "liftLoadFactor", "bLift", "1" }, { "oswaldEfficiency", "bLift", "2" },
                { "dragAreaFactor", "bDrag", "0" }, { "inducedDragFactor", "bDrag", "1" },
                { "radiatorDragCoeff", "bDrag", "2" },
        };
        for (String loc : Arrays.asList("zh", "en", "ru")) {
            Lang.initLang(loc);
            for (String[] p : pairs) {
                String fmt;
                try {
                    java.lang.reflect.Field f = Lang.class.getField(p[1]);
                    fmt = (String) f.get(null);
                } catch (Exception e) {
                    check(false, "5", "Lang." + p[1] + " 字段缺失");
                    return;
                }
                String[] lines = fmt.replace("\\n", "\n").split("\n");
                int idx = Integer.parseInt(p[2]);
                check(idx < lines.length, "5", "[" + loc + "] " + p[1] + " 行数不足");
                if (idx >= lines.length)
                    return;
                String propName = lines[idx].split(":")[0].trim();
                String expect = FmPropKeys.displayOf(p[0]);
                check(propName.equals(expect), "5",
                        "[" + loc + "] " + p[0] + ": 格式串=\"" + propName + "\" ≠ fm.prop=\"" + expect + "\"");
            }
        }
        Lang.initLang("zh");
        passed++; // 三语言同步全过计数
    }
}
