package parser;


class zb{
	float x;
	float y;
}
public class mapInfo {
	String s;
	public float grid_stepsX;
	public float grid_stepsY;
	public float grid_zeroX;
	public float grid_zeroY;
	public int map_generation;
	public float map_maxX;
	public float map_maxY;
	public float map_minX;
	public float map_minY;
	public float cmapmaxsizeX;
	public float cmapmaxsizeY;
	zb tp;
	float StringtoFloat(String a) {
		if (a.length() != 0)return Float.parseFloat(a);
		else return 0;

	}

	public zb getMapInfoParserArray(String t) {
		int bix;
		int eix;
		zb a=new zb();
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
		//System.out.print(s);
		tp=getMapInfoParserArray("grid_steps");
		grid_stepsX=tp.x;
		grid_stepsY=tp.y;
		tp=getMapInfoParserArray("grid_zero");
		grid_zeroX=tp.x;
		grid_zeroY=tp.y;
		tp=getMapInfoParserArray("map_max");
		map_maxX=tp.x;
		map_maxY=tp.y;
		tp=getMapInfoParserArray("map_min");
		map_minX=tp.x;
		map_minY=tp.y;
		cmapmaxsizeX = map_maxX - map_minX;
		cmapmaxsizeY = map_maxY - map_minY;
	}
}