package parser;

import java.awt.Color;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import prog.Application;

public class MapObj {

	public class Movobj {
		public String type;
		public String color;
		public Color colorg;
		public int blink;
		public double distance;
		public String icon;
		public String iconBg;
		public double x;
		public double y;
		public double dx;
		public double dy;

	}

	public class Staobj {
		public String type;
		public String color;
		public Color colorg;
		public int blink;
		public String icon;
		public String iconBg;
		public double x;
		public double y;
	}

	public class Plaobj {
		public String type;
		public String color;
		public Color colorg;
		public int blink;
		public String icon;
		public String iconBg;
		public double x;
		public double y;
		public double dx;
		public double dy;
	}

	public class Slcobj {
		public String type;
		public String color;
		public Color colorg;
		public int blink;
		public String icon;
		public String iconBg;
		public double x;
		public double y;
		public double dx;
		public double dy;
	}

	int num;
	public Movobj mov[];
	public int movcur;
	public Staobj sta[];
	public Plaobj pla;
	public Slcobj slc;
	public int stacur;
	public double aot;
	String s;

	String getLine() {
		int bix;
		int eix;
		String buf;
		bix = s.indexOf('{');
		if (bix != -1) {
			eix = s.indexOf('}');
			buf = s.substring(bix, eix + 1);
			// Application.debugPrint("切片值"+buf);
			s = s.substring(eix + 1, s.length());
			// Application.debugPrint("切片后"+s);
			return buf;
		} else
			return ("");
	}

	void parseObj(String t) {
		int bix;
		int eix;
		int quoteloc = 1;
		int flag = 0;
		String type = "";
		String color = "";
		Color colorg;
		int blink;
		String icon;
		String iconBg;
		double x;
		double y;
		double dx;
		double dy;

		boolean isPlayer = false;
		boolean isSelected = false;
		// 先找type
		bix = t.indexOf('"');
		if (bix != -1) {
			eix = bix + 5;
			while (t.charAt(eix) != ':') {
				eix++;
			}
			bix = eix + 1 + quoteloc;
			eix = bix;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			type = t.substring(bix, eix);
			// Application.debugPrint(type.charAt(3));
			// 继续向下搜索Color
			eix = eix + 2;
			// 找三个引号
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			bix = eix;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			color = t.substring(bix, eix);

			// 继续向下搜索Color[]
			eix = eix + 2;
			// 找2个引号
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '[') {
				eix++;
			}
			eix++;
			bix = eix;
			while (t.charAt(eix) != ',') {
				eix++;
			}
			int red = Integer.parseInt(t.substring(bix, eix));
			// Application.debugPrint(red);
			eix++;
			bix = eix;
			while (t.charAt(eix) != ',') {
				eix++;
			}
			int green = Integer.parseInt(t.substring(bix, eix));
			eix++;
			bix = eix;
			while (t.charAt(eix) != ']') {
				eix++;
			}
			int blue = Integer.parseInt(t.substring(bix, eix));
			colorg = new Color(red, green, blue);
			// Application.debugPrint(colorg);

			// 继续向下搜索blink
			eix = eix + 2;
			// 找2个引号
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != ':') {
				eix++;
			}
			eix++;
			bix = eix;
			while (t.charAt(eix) != ',') {
				eix++;
			}
			blink = Integer.parseInt(t.substring(bix, eix));
			// Application.debugPrint(blink);

			// 继续向下搜索icon
			eix = eix + 1;
			// 找三个引号
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			bix = eix;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			icon = t.substring(bix, eix);
			if (icon.equals("Player"))
				isPlayer = true;

			// Application.debugPrint(icon);

			// 继续向下搜索icon_bg
			eix = eix + 2;
			// 找三个引号
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			bix = eix;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			iconBg = t.substring(bix, eix);
			if (iconBg.equals("none") != true)
				isSelected = true;
			// Application.debugPrint(iconBg);
			// 继续向下搜索x
			eix = eix + 2;
			// 找2个引号
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != ':') {
				eix++;
			}
			eix++;
			bix = eix;
			while (t.charAt(eix) != ',') {
				eix++;
			}
			x = Float.parseFloat(t.substring(bix, eix));
			// Application.debugPrint(x);
			// 继续向下搜索y
			eix = eix + 1;
			// 找2个引号
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			while (t.charAt(eix) != ':') {
				eix++;
			}
			eix++;
			bix = eix;
			while (t.charAt(eix) != ',' && t.charAt(eix) != '}') {
				eix++;
			}
			y = Float.parseFloat(t.substring(bix, eix));

			if (t.charAt(eix) == '}')
				flag = 0;
			else
				flag = 1;

			// Application.debugPrint(t.substring(bix,eix));

			// 再根据type判断是否取dx、dy
			// Application.debugPrint(flag);

			if (flag == 0) {
				// 进入staobj写值
				if (isSelected) {
					slc.type = type;
					slc.color = color;
					slc.colorg = colorg;
					slc.blink = blink;
					slc.icon = icon;
					slc.iconBg = iconBg;
					slc.x = x;
					slc.y = y;
					slc.dx = 0;
					slc.dy = 0;
				} else {
					sta[stacur].type = type;
					sta[stacur].color = color;
					sta[stacur].colorg = colorg;
					sta[stacur].blink = blink;
					sta[stacur].icon = icon;
					sta[stacur].iconBg = iconBg;
					sta[stacur].x = x;
					sta[stacur].y = y;
					// Application.debugPrint("s写值成功" + sta[stacur].toString());
					stacur++;
				}
			}
			if (flag == 1) {
				// 进入movobj判断
				// 继续向下搜索y
				// Application.debugPrint(t);
				eix = eix + 1;
				// 找2个引号
				// Application.debugPrint(t);
				// Application.debugPrint("sad");
				while (t.charAt(eix) != '"') {
					eix++;
				}
				eix++;
				while (t.charAt(eix) != '"') {
					eix++;
				}
				eix++;
				while (t.charAt(eix) != ':') {
					eix++;
				}
				eix++;
				bix = eix;
				while (t.charAt(eix) != ',' && t.charAt(eix) != '}') {
					eix++;
				}
				// Application.debugPrint(t.substring(bix,eix));
				dx = Float.parseFloat(t.substring(bix, eix));

				// 继续向下搜索y
				eix = eix + 1;
				// 找2个引号
				while (t.charAt(eix) != '"') {
					eix++;
				}
				eix++;
				while (t.charAt(eix) != '"') {
					eix++;
				}
				eix++;
				while (t.charAt(eix) != ':') {
					eix++;
				}
				eix++;
				bix = eix;
				while (t.charAt(eix) != ',' && t.charAt(eix) != '}') {
					eix++;
				}
				// Application.debugPrint(t.substring(bix,eix));
				dy = Float.parseFloat(t.substring(bix, eix));
				if (!isPlayer) {
					if (isSelected) {
						slc.type = type;

						slc.color = color;
						slc.colorg = colorg;
						slc.blink = blink;
						slc.icon = icon;
						slc.iconBg = iconBg;
						slc.x = x;
						slc.y = y;
						slc.dx = dx;
						slc.dy = dy;
					} else {

						mov[movcur].type = type;

						mov[movcur].color = color;
						mov[movcur].colorg = colorg;
						mov[movcur].blink = blink;
						mov[movcur].icon = icon;
						mov[movcur].iconBg = iconBg;
						mov[movcur].x = x;
						mov[movcur].y = y;
						// Application.debugPrint("m写值成功" + mov[movcur].toString());
						movcur++;
					}
				} else {
					pla.type = type;
					pla.color = color;
					pla.colorg = colorg;
					pla.blink = blink;
					pla.icon = icon;
					pla.iconBg = iconBg;
					pla.x = x;
					pla.y = y;
					pla.dx = dx;
					pla.dy = dy;
					// Application.debugPrint("玩家写值成功" + pla.toString());
				}
			}

		}

	}

	void processObj() {
		String sobj;
		sobj = getLine();
		while (sobj != "") {
			parseObj(sobj);
			sobj = getLine();
		}
		// testmov();//测试用
		// Application.debugPrint(mov[movcur-1].x);
		//Application.debugPrint("切片完成");

	}

	void testmov() {
		int i;
		for (i = 0; i < num; i++) {
			System.out.print(mov[i].x + " ");

		}
		Application.debugPrint(String.format("%d",i));
	}

	void initMobj() {
		int i;
		for (i = 0; i < num; i++) {
			mov[i] = new Movobj();
			sta[i] = new Staobj();
		}
	}

	public void init() {
		num = 500;
		//Application.debugPrint("mapObj初始化了");
		mov = new Movobj[num];
		sta = new Staobj[num];
		initMobj();
		pla = new Plaobj();
		slc = new Slcobj();
		s = "";

	}
	public void calculate(){
		aot = Math.abs(Math.atan(slc.dy/slc.dx) - Math.atan(pla.dy/pla.dx));
	}
	
	/**
	 * 根据 map_obj 判断玩家是否已阵亡/不在战场上。
	 * 阵亡后游戏仍返回机场列表，但 Player 图标消失（真机抓包 2026-11-14）；坐标越界也视为无效。
	 *
	 * @param loc 可选输出缓冲（至少 2 个元素），成功时写入玩家归一化坐标
	 * @return true=阵亡/观战/过渡帧，玩家坐标不可用
	 */
	public static boolean isPlayerDead(String jsonText, double[] loc) {
		if (!getPlayerLoc(jsonText, loc))
			return true;
		return loc[0] < 0 || loc[0] > 1 || loc[1] < 0 || loc[1] > 1;
	}

	/** 重载：不需要玩家坐标时使用。 */
	public static boolean isPlayerDead(String jsonText) {
		return isPlayerDead(jsonText, new double[2]);
	}

	/**
	 * 从 map_obj 提取玩家归一化坐标。
	 *
	 * @return 本轮是否找到玩家图标；阵亡/观战/过渡帧可能没有 Player 对象
	 */
	public static boolean getPlayerLoc(String jsonText, double[] loc){
		if (jsonText == null || jsonText.isEmpty() || loc == null || loc.length < 2)
			return false;
		// 正则表达式用于匹配整个JSON对象，并捕获icon为"Player"的x和y坐标
		String pattern = "\\{[^{}]*\\\"icon\\\"\\s*:\\s*\\\"Player\\\"[^{}]*,[^{}]*\\\"x\\\"\\s*:\\s*(-?\\d+(\\.\\d+)?),[^{}]*\\\"y\\\"\\s*:\\s*(-?\\d+(\\.\\d+)?)[^{}]*\\}";
		Pattern r = Pattern.compile(pattern);
		Matcher m = r.matcher(jsonText);
		boolean found = false;

		while (m.find()) {
			double x = Double.parseDouble(m.group(1));
			double y = Double.parseDouble(m.group(3));
			// System.out.println("Player coordinates: x = " + x + ", y = " + y);
			loc[0] = x;
			loc[1] = y;
			found = true;
        }
		return found;
	}

	public static void getPlayerDir(String jsonText, double[] dir){
		// 正则表达式用于匹配整个JSON对象，并捕获icon为"Player"的x和y坐标
		String pattern = "\\{[^{}]*\\\"icon\\\"\\s*:\\s*\\\"Player\\\"[^{}]*,[^{}]*\\\"dx\\\"\\s*:\\s*(-?\\d+(\\.\\d+)?),[^{}]*\\\"dy\\\"\\s*:\\s*(-?\\d+(\\.\\d+)?)[^{}]*\\}";
		Pattern r = Pattern.compile(pattern);
		Matcher m = r.matcher(jsonText);

		while (m.find()) {
			double dx = Double.parseDouble(m.group(1));
			double dy = Double.parseDouble(m.group(3));
			// System.out.println("Player direction: dx = " + dx + ", dy = " + dy);
			dir[0] = dx;
			dir[1] = dy;
        }
	}
	public static void getAirfieldLoc(String jsonText, double[][] loc){
		// 正则表达式用于匹配整个JSON对象，并捕获icon为"Player"的x和y坐标
		String pattern = "\\{[^{}]*\\\"type\\\"\\s*:\\s*\\\"airfield\\\"[^{}]*,[^{}]*\\\"sx\\\"\\s*:\\s*(-?\\d+(\\.\\d+)?),[^{}]*\\\"sy\\\"\\s*:\\s*(-?\\d+(\\.\\d+)?)[^{}]*\\}";
		Pattern r = Pattern.compile(pattern);
		Matcher m = r.matcher(jsonText);

		while (m.find()) {
			double x = Double.parseDouble(m.group(1));
			double y = Double.parseDouble(m.group(3));
			// System.out.println("Airfield coordinates: x = " + x + ", y = " + y);
			// loc[0] = x;
			// loc[1] = y;
		}
	}

	/**
	 * map_obj.json 里的机场原始对象：跑道两端点(归一化地图坐标)、颜色与 blink。
	 * blink != 0 或红色系表示敌方，与旧版罗盘机场指示的判定保持一致。
	 */
	public static final class AirfieldRaw {
		public final double sx;
		public final double sy;
		public final double ex;
		public final double ey;
		public final boolean enemy;

		AirfieldRaw(double sx, double sy, double ex, double ey, boolean enemy) {
			this.sx = sx;
			this.sy = sy;
			this.ex = ex;
			this.ey = ey;
			this.enemy = enemy;
		}
	}

	private static final Pattern AIRFIELD_OBJ_PATTERN =
			Pattern.compile("\\{[^{}]*\\\"type\\\"\\s*:\\s*\\\"airfield\\\"[^{}]*\\}");
	private static final Pattern AIRFIELD_COORD_PATTERN = Pattern.compile(
			"\\\"(sx|sy|ex|ey)\\\"\\s*:\\s*(-?\\d+(?:\\.\\d+)?(?:[eE][+-]?\\d+)?)");
	private static final Pattern AIRFIELD_RGB_PATTERN = Pattern.compile(
			"\\\"color\\[\\]\\\"\\s*:\\s*\\[\\s*(\\d+)\\s*,\\s*(\\d+)\\s*,\\s*(\\d+)\\s*\\]");
	private static final Pattern AIRFIELD_HEX_PATTERN = Pattern.compile("\\\"color\\\"\\s*:\\s*\\\"#?([0-9A-Fa-f]{6})\\\"");
	private static final Pattern AIRFIELD_BLINK_PATTERN = Pattern.compile("\\\"blink\\\"\\s*:\\s*(-?\\d+)");

	/** 解析 map_obj.json 中的全部机场对象；解析失败的对象直接跳过。 */
	public static List<AirfieldRaw> parseAirfields(String jsonText) {
		if (jsonText == null || jsonText.isEmpty())
			return Collections.emptyList();
		List<AirfieldRaw> out = new ArrayList<>();
		Matcher objM = AIRFIELD_OBJ_PATTERN.matcher(jsonText);
		while (objM.find()) {
			String obj = objM.group();
			Double sx = extractDouble(AIRFIELD_COORD_PATTERN, obj, "sx");
			Double sy = extractDouble(AIRFIELD_COORD_PATTERN, obj, "sy");
			Double ex = extractDouble(AIRFIELD_COORD_PATTERN, obj, "ex");
			Double ey = extractDouble(AIRFIELD_COORD_PATTERN, obj, "ey");
			if (sx == null || sy == null || ex == null || ey == null)
				continue;

			int red = 0, green = 0, blue = 0;
			Matcher rgbM = AIRFIELD_RGB_PATTERN.matcher(obj);
			if (rgbM.find()) {
				red = Integer.parseInt(rgbM.group(1));
				green = Integer.parseInt(rgbM.group(2));
				blue = Integer.parseInt(rgbM.group(3));
			} else {
				Matcher hexM = AIRFIELD_HEX_PATTERN.matcher(obj);
				if (hexM.find()) {
					String hex = hexM.group(1);
					red = Integer.parseInt(hex.substring(0, 2), 16);
					green = Integer.parseInt(hex.substring(2, 4), 16);
					blue = Integer.parseInt(hex.substring(4, 6), 16);
				}
			}
			int blink = 0;
			Matcher blinkM = AIRFIELD_BLINK_PATTERN.matcher(obj);
			if (blinkM.find()) {
				blink = Integer.parseInt(blinkM.group(1));
			}
			boolean enemy = blink != 0 || (red > 100 && red > blue);
			out.add(new AirfieldRaw(sx, sy, ex, ey, enemy));
		}
		return out;
	}

	/**
	 * 归一化坐标 → 米制距离/跑道长度。玩家位置或 grid_size 未就绪时返回空列表。
	 */
	public static List<Airfield> resolveAirfields(List<AirfieldRaw> raws, double[] loc, MapInfo info) {
		if (raws == null || raws.isEmpty() || loc == null || info == null)
			return Collections.emptyList();
		double spanX = info.grid_sizeX;
		double spanY = info.grid_sizeY;
		if (!(spanX > 0) || !(spanY > 0))
			return Collections.emptyList();
		// 归一化地图坐标应落在 0..1；阵亡/过渡帧的异常坐标直接丢弃
		if (loc[0] < 0 || loc[0] > 1 || loc[1] < 0 || loc[1] > 1)
			return Collections.emptyList();
		if (loc[0] == 0 && loc[1] == 0)
			return Collections.emptyList();

		List<Airfield> out = new ArrayList<>(raws.size());
		for (AirfieldRaw raw : raws) {
			double cx = (raw.sx + raw.ex) * 0.5;
			double cy = (raw.sy + raw.ey) * 0.5;
			if (cx < 0 || cx > 1 || cy < 0 || cy > 1)
				continue;
			double dx = (cx - loc[0]) * spanX;
			double dy = (cy - loc[1]) * spanY;
			double length = Math.hypot((raw.ex - raw.sx) * spanX, (raw.ey - raw.sy) * spanY);
			out.add(new Airfield(Math.hypot(dx, dy), length, !raw.enemy,
					bearingDeg(dx, dy), mapGrid(cx, cy, info)));
		}
		return out;
	}

	/**
	 * 世界向量 → 0..360 罗盘方位；与 Service 无仪表航向回退公式一致。
	 * map_obj 的 y 轴向下（南），因此 dy>0 为南。
	 */
	private static double bearingDeg(double dx, double dy) {
		if (dy == 0 && dx == 0)
			return 0;
		if (dy < 0)
			return (360 - Math.toDegrees(Math.atan(dx / dy))) % 360;
		return 180 - Math.toDegrees(Math.atan(dx / dy));
	}

	/** 机场中心的地图网格，与玩家 mapGrid 使用同一套换算。 */
	private static String mapGrid(double x, double y, MapInfo info) {
		if (info == null || !(info.mapStage > 0))
			return "";
		char mapX = (char) ('A' + (y * info.mapStage) + info.inGameOffset);
		int mapY = (int) (x * info.mapStage + info.inGameOffset + 1);
		return String.format("%c%d", mapX, mapY);
	}

	private static Double extractDouble(Pattern p, String text, String key) {
		Matcher m = p.matcher(text);
		while (m.find()) {
			if (key.equals(m.group(1))) {
				try {
					return Double.parseDouble(m.group(2));
				} catch (NumberFormatException e) {
					return null;
				}
			}
		}
		return null;
	}

	public void update(String S) {
		s = S;
		// Application.debugPrint("初始值"+s);
		movcur = 0;
		stacur = 0;
		slc.type="";
		processObj();
		calculate();
	}
}