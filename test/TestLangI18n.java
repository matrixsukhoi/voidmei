import prog.i18n.Lang;

/**
 * Lang i18n 基建测试 —— 回退链 / 幂等重灌 / 偏好解析
 *
 * 运行前提: cwd = 项目根 (lang/zh.properties 存在; en/ru 为骨架, 天然测试缺 key 回退)
 * 运行方式: python script/build.py test i18n
 */
public class TestLangI18n {

	private static int passed = 0;
	private static int failed = 0;

	public static void main(String[] args) {
		System.out.println("=== Lang i18n 测试 ===\n");

		testFallbackChain();
		testIdempotentReload();
		testUiDynamicQuery();
		testLocaleOfPreference();

		System.out.println("\n=== 测试结果 ===");
		System.out.println("通过: " + passed);
		System.out.println("失败: " + failed);

		if (failed > 0) {
			System.exit(1);
		}
	}

	/** 回退链: 当前语言缺 key → zh 基准 → 内联默认(修复原"缺 key 变空串"缺陷) */
	private static void testFallbackChain() {
		System.out.println("-- 回退链测试 --");
		Lang.initLang("zh");
		String zhVal = Lang.updateLanguage("mCancel", "__dft__");
		assertNotNull(zhVal, "zh 包应命中 mCancel");
		assertEquals(false, zhVal.equals("__dft__"), "zh 包命中时不应返回内联默认");

		// en 为空骨架: 缺 key 必须回退到 zh 值, 而非空串
		Lang.initLang("en");
		String enFallback = Lang.updateLanguage("mCancel", "__dft__");
		assertEquals(zhVal, enFallback, "en 缺 key 应回退 zh 值");

		// 两边都缺的 key: 返回内联默认(原版缺陷是返回空串)
		String dft = Lang.updateLanguage("__no_such_key__", "内联默认值");
		assertEquals("内联默认值", dft, "全缺 key 应返回内联默认值");
	}

	/** 幂等重灌: initLang 可重复调用, currentLocale 正确, 静态字段随包变化 */
	private static void testIdempotentReload() {
		System.out.println("-- 幂等重灌测试 --");
		Lang.initLang("zh");
		assertEquals("zh", Lang.locale(), "locale 应为 zh");
		String zhBWeight = Lang.bWeight;
		assertNotNull(zhBWeight, "zh 下 bWeight 应非空");

		// ru 为空骨架 → bWeight 经 zh 回退仍非空, 且回退值与 zh 一致
		Lang.initLang("ru");
		assertEquals("ru", Lang.locale(), "热切换后 locale 应为 ru");
		assertEquals(zhBWeight, Lang.bWeight, "ru 缺 key 时静态字段应回退 zh 值");

		// 切回 zh(验证重复灌值不残留)
		Lang.initLang("zh");
		assertEquals(zhBWeight, Lang.bWeight, "切回 zh 后值应还原");
	}

	/** Lang.ui 动态查询口: DSL @key 用, 缺失回退原文 */
	private static void testUiDynamicQuery() {
		System.out.println("-- 动态查询测试 --");
		Lang.initLang("zh");
		assertEquals("原文直显", Lang.ui("__no_such_ui_key__", "原文直显"), "未知 key 应返回原文");
		String known = Lang.ui("mCancel", "原文直显");
		assertEquals(false, known.equals("原文直显"), "已知 key 不应返回原文");
	}

	/** 偏好值解析: 下拉值 → locale; Auto/非法 → 系统(仅断言三合法值之一) */
	private static void testLocaleOfPreference() {
		System.out.println("-- 偏好解析测试 --");
		assertEquals("zh", Lang.localeOfPreference("中文"), "中文→zh");
		assertEquals("en", Lang.localeOfPreference("English"), "English→en");
		assertEquals("ru", Lang.localeOfPreference("Русский"), "Русский→ru");
		for (String v : new String[] { "Auto", "", null, "garbage" }) {
			String loc = Lang.localeOfPreference(v);
			boolean legal = "zh".equals(loc) || "en".equals(loc) || "ru".equals(loc);
			assertEquals(true, legal, "Auto/非法值(" + v + ")应解析为合法 locale: " + loc);
		}
	}

	private static void assertEquals(Object expect, Object actual, String msg) {
		if (expect == null ? actual == null : expect.equals(actual)) {
			passed++;
			System.out.println("[PASS] " + msg);
		} else {
			failed++;
			System.out.println("[FAIL] " + msg + " — 期望: " + expect + ", 实际: " + actual);
		}
	}

	private static void assertNotNull(Object o, String msg) {
		assertEquals(false, o == null, msg);
	}
}
