package parser;

import java.awt.Color;
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
	
	public static void getPlayerLoc(String jsonText, double[] loc){
		// 正则表达式用于匹配整个JSON对象，并捕获icon为"Player"的x和y坐标
		String pattern = "\\{[^{}]*\\\"icon\\\"\\s*:\\s*\\\"Player\\\"[^{}]*,[^{}]*\\\"x\\\"\\s*:\\s*(-?\\d+(\\.\\d+)?),[^{}]*\\\"y\\\"\\s*:\\s*(-?\\d+(\\.\\d+)?)[^{}]*\\}";
		Pattern r = Pattern.compile(pattern);
		Matcher m = r.matcher(jsonText);

		while (m.find()) {
			double x = Double.parseDouble(m.group(1));
			double y = Double.parseDouble(m.group(3));
			// System.out.println("Player coordinates: x = " + x + ", y = " + y);
			loc[0] = x;
			loc[1] = y;
        }
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