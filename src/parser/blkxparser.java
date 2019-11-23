package parser;
import java.io.BufferedReader;
import java.io.File;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.IOException;

public class blkxparser {
	public class XY{
		public double x[];
		public double y[];
		public int cur;
		XY(int num){
			x=new double[num];
			y=new double[num];
			cur=0;
		}
	}
	public String data;
	public XY loc;//WEP爬升
	public XY loc0;//NOM爬升
	public XY loc1;//WEP速度
	public XY loc2;//NOM速度
	public XY loc3;//滚转
	public XY plotdata[];
	
	public void getperformancedata(String t){

	}
	public String subSt(String t){
		String a=t.substring(1,t.length()-1);
		return a;
	}
	public void transUnit(){
		String unitSystem="";
		unitSystem=getone("PASSPORT.UNITSYSTEM");
		unitSystem=subSt(unitSystem);
		if(unitSystem.indexOf("Imperial")!=-1){
			//System.out.println("英制");
			for(int i=0;i<loc.cur;i++){
				loc.y[i]=loc.y[i]*0.3048;
			}
			for(int i=0;i<loc0.cur;i++){
				loc0.y[i]=loc0.y[i]*0.3048;
			}
			for(int i=0;i<loc1.cur;i++){
				loc1.y[i]=loc1.y[i]*0.3048;
				loc1.x[i]=loc1.x[i]* 1.609344;
			}
			for(int i=0;i<loc2.cur;i++){
				loc2.y[i]=loc2.y[i]*0.3048;
				loc2.x[i]=loc2.x[i]* 1.609344;
			}
			for(int i=0;i<loc3.cur;i++){
				loc3.y[i]=loc3.y[i]* 1.609344;
				System.out.println(loc3.x[i]+" "+loc3.y[i]);
			}
			
			
		}
	}
	public void getAllplotdata(){
		loc=getplotdata("PASSPORT.ALT.minClimbTimeWep");
		loc0=getplotdata("PASSPORT.ALT.minClimbTimeNom");
		loc1=getplotdata("PASSPORT.ALT.maxSpeedWep");
		loc2=getplotdata("PASSPORT.ALT.maxSpeedNom");
		loc3=getplotdata("PASSPORT.IAS.maxRollRateLeft");
		transUnit();
	}
	public XY getplotdata(String t){
		int line=0;
		t=getArray(t);
		for(int i=0;i<t.length();i++){
			if(t.charAt(i)=='\n')line++;
		}
		XY lo=new XY(line);
		int bix=0;
		for(int i=0;i<t.length();i++){
			if(t.charAt(i)=='\n'){
				String temp=t.substring(bix,i);
				String[] tmp=temp.split(", ");
				lo.y[lo.cur]=Double.parseDouble(tmp[0]);
				lo.x[lo.cur]=Double.parseDouble(tmp[1]);
				lo.cur++;
				bix=i+1;
			}
		}
		return lo;
	}
	public void init(String t){
		data=t;
		
	}
	public blkxparser(String filepath){
		File file = new File(filepath);
		StringBuilder sb = new StringBuilder();
		String s = "";
		BufferedReader br;
		try {
			br = new BufferedReader(new FileReader(file));
			while ((s = br.readLine()) != null) {
				sb.append(s + "\n");
			}
			br.close();
		} catch (FileNotFoundException e1) {
			// TODO Auto-generated catch block
			e1.printStackTrace();
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		data = sb.toString();
	}
	String cut(String t,String clslabel){
		String tmp=t;
		int i=0;
		int left=0;
		int right=0;
		int bix=tmp.toUpperCase().indexOf(clslabel.toUpperCase()+'{');
		if(bix==-1)return "null";
		int cutleft=bix;
		while(tmp.charAt(cutleft)!='{')cutleft++;
		cutleft++;
		for(i=bix;i<tmp.length();i++){
			if(tmp.charAt(i)=='{')left++;
			if(tmp.charAt(i)=='}')right++;
			if(left!=0&&right!=0&&left==right)break;
		}
		int cutright=i;
		return tmp.substring(cutleft, cutright);
	}
	public String getArray(String label){
		String value="";
		String text=data;
		//第一步处理
		int clsbix=0;
		for(int i=0;i<label.length();i++){
			if(label.charAt(i)=='.'){
				String cls=label.substring(clsbix, i);
				text=cut(text,cls);
				clsbix=i+1;
			}
		}
		label=label.substring(clsbix);
		//System.out.println(text);
		//第二步获得值
		int bix=0;
		int eix=0;
		bix=text.toUpperCase().indexOf(label.toUpperCase());
		while(bix!=-1){
			while(text.charAt(bix)!='=')bix++;
			bix++;
			eix=bix;
			while(text.charAt(eix)!='\n')eix++;
			value=value+text.substring(bix, eix+1);
			text=text.substring(eix+1);
			bix=text.toUpperCase().indexOf(label.toUpperCase());
		}
		return value;
	}
	public String getlastone(String label){
		String value="";
		String text=data;
		//第一步处理
		int clsbix=0;
		for(int i=0;i<label.length();i++){
			if(label.charAt(i)=='.'){
				String cls=label.substring(clsbix, i);
				text=cut(text,cls);
				clsbix=i+1;
			}
		}
		label=label.substring(clsbix);
		//第二步获得值
		int bix=0;
		int eix=0;
		bix=text.toUpperCase().lastIndexOf(label.toUpperCase());
		if(bix==-1)return "null";
		while(text.charAt(bix)!='=')bix++;
		bix++;
		eix=bix;
		while(text.charAt(eix)!='\n')eix++;
		value=text.substring(bix, eix);
		return value;
	}
	public String getone(String label){
		String value="";
		String text=data;
		//第一步处理
		int clsbix=0;
		for(int i=0;i<label.length();i++){
			if(label.charAt(i)=='.'){
				String cls=label.substring(clsbix, i);
				text=cut(text,cls);
				clsbix=i+1;
			}
		}
		label=label.substring(clsbix);
		//第二步获得值
		int bix=0;
		int eix=0;
		bix=text.toUpperCase().indexOf(label.toUpperCase());
		if(bix==-1)return "null";
		while(text.charAt(bix)!='=')bix++;
		bix++;
		eix=bix;
		while(text.charAt(eix)!='\n')eix++;
		value=text.substring(bix, eix);
		return value;
	}
	
}