package parser;

import prog.Application;

class zb {
	double x;
	double y;
}

public class MapInfo {
	private static final java.util.regex.Pattern GRID_SIZE_PATTERN = java.util.regex.Pattern
			.compile("\"grid_size\"\\s*:\\s*\\[\\s*(-?[\\d.eE+-]+)\\s*,\\s*(-?[\\d.eE+-]+)\\s*\\]");
	String s;
	public double grid_stepsX;
	public double grid_stepsY;
	public double grid_zeroX;
	public double grid_zeroY;
	// 地图网格覆盖的物理尺寸(米): map_obj 归一化坐标 × 本值 = 世界米数
	// 注意不能用 cmapmaxsize(map_max-map_min) 代替: 真机快照里二者不相等
	public double grid_sizeX;
	public double grid_sizeY;
	public int map_generation;
	public double map_maxX;
	public double map_maxY;
	public double map_minX;
	public double map_minY;
	public double cmapmaxsizeX;
	public double cmapmaxsizeY;

	public double inGameOffset; // 游戏内地图的偏移量
	zb tp;
	public double mapStage;

	double StringtoFloat(String a) {
		if (a.length() != 0)
			return Float.parseFloat(a);
		else
			return 0;

	}

	public zb getMapInfoParserArray(String t) {
		int bix;
		int eix;
		zb a = new zb();
		bix = s.indexOf(t);

		if (bix >= 0) {
			eix = bix;
			while (s.charAt(eix) != ':') {
				eix++;
			}
			eix++;
			bix = eix + 3;
			while (s.charAt(eix) != ',') {
				eix++;
				if (eix == s.length() + 1)
					break;
			}

			a.x = StringtoFloat(s.substring(bix, eix));

			bix = eix + 2;
			while (s.charAt(eix) != ']') {
				eix++;
				if (eix == s.length() + 1)
					break;
			}
			eix = eix - 1;

			a.y = StringtoFloat(s.substring(bix, eix));

		}
		return a;

	}

	public void init() {

	}

	public void update(String S) {
		s = S;
		// System.out.print(s);
		// grid_size 用容错正则单独解析: 距离换算依赖它, 不随原文空格格式变化
		java.util.regex.Matcher gs = GRID_SIZE_PATTERN.matcher(s);
		if (gs.find()) {
			grid_sizeX = Double.parseDouble(gs.group(1));
			grid_sizeY = Double.parseDouble(gs.group(2));
		}
		tp = getMapInfoParserArray("grid_steps");
		grid_stepsX = tp.x;
		grid_stepsY = tp.y;
		tp = getMapInfoParserArray("grid_zero");
		grid_zeroX = tp.x;
		grid_zeroY = tp.y;
		tp = getMapInfoParserArray("map_max");
		map_maxX = tp.x;
		map_maxY = tp.y;
		tp = getMapInfoParserArray("map_min");
		map_minX = tp.x;
		map_minY = tp.y;
		cmapmaxsizeX = map_maxX - map_minX;
		cmapmaxsizeY = map_maxY - map_minY;
		inGameOffset = ((grid_zeroY - grid_zeroX) - (map_maxX + map_maxY)) / (grid_stepsX + grid_stepsY);
		mapStage = (map_maxX + map_maxY) * 2 / (grid_stepsX + grid_stepsY);

		// Application.debugPrint("ingame mapinfo offset:" + inGameOffset + "map stage: " + mapStage);
	}
}