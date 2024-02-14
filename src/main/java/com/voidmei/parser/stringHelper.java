package com.voidmei.parser;

public class stringHelper {
	
	public static final int iInvalid = -65535;
	public static final double fInvalid = -65535;
	public static void getStringBuilder(StringBuilder R, String S, char buf[], int buflen) {
		int bix;
		int eix;
		bix = R.lastIndexOf(S);
		
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
			R.getChars(bix, eix, buf, buflen);
		}
		
	}
	

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
	public static double getDataFloatC(CharSequence cs){
		if(cs!=null)return Float.parseFloat(cs.toString());
		else return fInvalid;
	}
	public static double getDataIntC(CharSequence cs){
		if (cs != null)
			return Integer.parseInt(cs.toString());
		else
			return iInvalid;
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
