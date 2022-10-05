package parser;

public class stringHelper {
	
	public static final int iInvalid = -65535;
	public static final double fInvalid = -65535;
	public static String getString(String R, String S) {
		int bix;
		int eix;
		bix = R.indexOf(S);
		if (bix >= 0) {
			eix = bix;
			while (R.charAt(eix) != ':') {
				eix++;
			}
			eix++;
			bix = eix + 1;
			while (R.charAt(eix) != ',' && R.charAt(eix) != '}') {
				eix++;
				if (eix == R.length() + 1)
					break;
			}
			return R.substring(bix, eix);
		} else
			return null;
	}

	public static double getDataFloat(String sdata){
		if(sdata!=null)return Float.parseFloat(sdata);
		else return fInvalid;
	}

	public static int getDataInt(String sdata) {
		if (sdata != null)
			return Integer.parseInt(sdata);
		else
			return iInvalid;
	}

}
