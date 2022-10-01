package parser;



//http://127.0.0.1:8111/hudmsg?lastEvt=0&lastDmg=0
public class hudMsg {
	String s;
	public damage dmg;

	String getDmglastLine() {
		int bix;
		int eix;

		if (s.length() > 30) {
			eix = s.length() - 2;
			bix = eix - 1;
			while (s.charAt(bix) != '{') {
				bix--;
			}
			return s.substring(bix, eix);

		} else
			return "";

	}

	String getLine(String a) {
		int bix;
		int eix;
		bix = s.indexOf(a);
		if (bix != -1) {
			eix = bix + 1;
			while (s.charAt(eix) != '{') {
				eix++;
				if (s.charAt(eix) == ']')
					return "";
			}
			bix = eix;
			eix++;
			while (s.charAt(eix) != '}') {
				eix++;
			}
			eix++;
			return s.substring(bix, eix);
		} else
			return "";
	}

	public int parseObj(String buf) {
		// app.debugPrint(buf);
		int bix = 0;
		int eix = 0;
		// id
		if (buf.length() > 20) {
			while (buf.charAt(eix) != ':') {
				eix++;
			}
			eix++;
			eix++;
			bix = eix;
			while (buf.charAt(eix) != ',') {
				eix++;
			}
			dmg.id = Integer.parseInt(buf.substring(bix, eix));

			eix++;

			while (buf.charAt(eix) != ':') {
				eix++;
			}
			eix++;
			while (buf.charAt(eix) != '"') {
				eix++;
			}
			eix++;
			bix = eix;
			while (buf.charAt(eix) != '"') {
				eix++;
			}
			dmg.msg = buf.substring(bix, eix);
			return 1;
		} else
			return 0;
	}

	public class events {

	}

	public class damage {
		public int id;
		public String msg;
		public String sender;
		public boolean enemy;
		public String mode;
		public boolean updated;
	}

	public void init() {
		//app.debugPrint("hudMSG初始化了");
		dmg = new damage();
	}

	public int update(String S, int lastDmg) {
		s = S;
		// app.debugPrint(S);
		//String buf = getLine("damage");
		dmg.updated=false;
		String lastbuf = getDmglastLine();
		//app.debugPrint(lastbuf);
		if (parseObj(lastbuf) == 1) {
			//app.debugPrint(dmg.id + " " + dmg.msg);
			dmg.updated=true;
			return dmg.id;
		}

		else {
			return lastDmg;
		}

	}
}