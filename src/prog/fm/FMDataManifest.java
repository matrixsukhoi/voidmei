package prog.fm;

/**
 * data prerelease 的 data_manifest.json 值对象（FM 数据在线更新用）。
 *
 * <p>manifest 由 build.py 的 json.dumps(indent=2) 生成，键值独占一行、格式固定，
 * 故用最小手工解析（不引 JSON 库）。契约：任一 indexOf 判负即字段缺失，
 * 必需字段（wt_version/zip/sha256）缺失时 {@link #parse} 整体返回 null。
 */
public final class FMDataManifest {

	/** WT 四段版本号，如 "2.59.0.28"（与 data/aces/version 同源） */
	public final String wtVersion;
	/** zip 附件文件名（单一真相源，禁止按 glob 猜） */
	public final String zipName;
	/** 整包 zip 的 SHA-256（hex 小写），防传输损坏 */
	public final String sha256;
	/** 打包日期 yyyymmdd，仅展示用，可为 null */
	public final String date;

	public FMDataManifest(String wtVersion, String zipName, String sha256, String date) {
		this.wtVersion = wtVersion;
		this.zipName = zipName;
		this.sha256 = sha256;
		this.date = date;
	}

	/**
	 * 解析 manifest JSON。格式不符 / 必需字段缺失 / null 或空串输入 → 返回 null（调用方按检查失败处理）。
	 */
	public static FMDataManifest parse(String json) {
		if (json == null || json.isEmpty())
			return null;
		String wt = extractString(json, "wt_version");
		String zip = extractString(json, "zip");
		String sha = extractString(json, "sha256");
		if (wt == null || zip == null || sha == null)
			return null;
		return new FMDataManifest(wt, zip, sha, extractString(json, "date"));
	}

	/**
	 * 提取顶层 "key": "value" 的 value。值非字符串（null/数字）或键不存在 → null。
	 * 值（版本号/文件名/hex）无转义字符，无需处理 \"。
	 */
	static String extractString(String json, String key) {
		// 前导引号防子串误配（如 "my_zip" 不会匹配 "zip"）
		int i = json.indexOf("\"" + key + "\":");
		if (i < 0)
			return null;
		int v = i + key.length() + 3; // 跳过 "key":
		while (v < json.length() && Character.isWhitespace(json.charAt(v)))
			v++;
		if (v >= json.length() || json.charAt(v) != '"')
			return null;
		int e = json.indexOf('"', v + 1);
		return e < 0 ? null : json.substring(v + 1, e);
	}

	/**
	 * 四段点分版本号语义比较（"2.59.0.28" vs "2.59.0.7"：数值 28&gt;7，
	 * 字符串序会判错）。按 '.' 分段逐段比较：缺段按 0 补齐；某段非数字时该段
	 * 退化为字符串比较，整体不抛。null 视为最小（远端 null → 不更新）。
	 *
	 * @return 负数 a&lt;b / 0 相等 / 正数 a&gt;b
	 */
	public static int compareVersion(String a, String b) {
		if (a == null || b == null) {
			if (a == null && b == null)
				return 0;
			return a == null ? -1 : 1;
		}
		String[] pa = a.split("\\.");
		String[] pb = b.split("\\.");
		int n = Math.max(pa.length, pb.length);
		for (int i = 0; i < n; i++) {
			int r = compareSegment(seg(pa, i), seg(pb, i));
			if (r != 0)
				return r;
		}
		return 0;
	}

	private static String seg(String[] parts, int i) {
		return i < parts.length && !parts[i].isEmpty() ? parts[i] : "0";
	}

	private static int compareSegment(String x, String y) {
		try {
			return Integer.compare(Integer.parseInt(x), Integer.parseInt(y));
		} catch (NumberFormatException e) {
			return x.compareTo(y);
		}
	}
}
