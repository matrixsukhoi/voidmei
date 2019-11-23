package parser;
public class indicators{
	public volatile String valid;
	public volatile String type;
	public volatile boolean isdummyplane=false;
	public volatile String stype;
	public volatile boolean flag;
	public volatile boolean fuelpressure;
	public volatile float speed;
	public volatile float pedals;
	public volatile float stick_elevator;
	public volatile float stick_ailerons;
	public volatile float altitude_hour;
	public volatile float altitude_min;
	public volatile float altitude_10k;
	public volatile float bank;
	public volatile float turn;
	public volatile float compass;
	public volatile float clock_hour;
	public volatile float clock_min;
	public volatile float clock_sec;
	public volatile float manifold_pressure;
	public volatile float rpm;
	public volatile float oil_pressure;
	public volatile float water_temperature;
	public volatile float engine_temperature;
	public volatile float mixture;
	public volatile float fuel[];
	public volatile float fuel_pressure;
	public volatile float oxygen;
	public volatile float gears_lamp;
	public volatile float flaps;
	public volatile float trimmer;
	public volatile float throttle;
	public volatile float weapon1;
	public volatile float weapon2;
	public volatile float weapon3;
	public volatile float prop_pitch_hour;
	public volatile float prop_pitch_min;
	public volatile float ammo_counter1;
	public volatile float ammo_counter2;
	public volatile float ammo_counter3;
	public volatile int fuelnum;
	public String getString(String R, String S) {
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
	public float getDatafloat(String sdata){
		if(sdata!=null)return Float.parseFloat(sdata);
		else return -65535;
	}

	public void init() {
		//System.out.println("indicator初始化了");
		valid = "false";
		fuelnum=0;
		fuel=new float[5];
		fuelpressure=false;
	}
	
	public void update(String buf) {
		valid = getString(buf, "valid");
		flag=false;
		if (valid.equals("true")){
			flag=true;
			type=getString(buf, "type").toUpperCase();
		
			if(type!=""){
				type=type.substring(1, type.length()-1);
				//判定是否dummyplane
				//if(type.equals("DUMMY_PLANE"))isdummyplane=true;
				//else isdummyplane=false;
				if(type.length()>9)stype=type.substring(0, 8);
				else stype=type;
				
			}
			
			speed=getDatafloat(getString(buf, "speed"));
			pedals=getDatafloat(getString(buf, "pedals"));
			stick_elevator=getDatafloat(getString(buf, "stick_elevator"));
			stick_ailerons=getDatafloat(getString(buf, "stick_ailerons"));
			altitude_hour=getDatafloat(getString(buf, "altitude_hour"));
			altitude_min=getDatafloat(getString(buf, "altitude_min"));;
			altitude_10k=getDatafloat(getString(buf, "altitude_10k"));
			bank=getDatafloat(getString(buf, "bank"));
			turn=getDatafloat(getString(buf, "turn"));
			compass=getDatafloat(getString(buf, "compass"));
			clock_hour=getDatafloat(getString(buf, "clock_hour"));
			clock_min=getDatafloat(getString(buf, "clock_min"));
			clock_sec=getDatafloat(getString(buf, "clock_sec"));
			manifold_pressure=getDatafloat(getString(buf, "manifold_pressure"));
			rpm=getDatafloat(getString(buf, "rpm"));
			oil_pressure=getDatafloat(getString(buf, "oil_pressure"));
			water_temperature=getDatafloat(getString(buf, "water_temperature"));
			engine_temperature=getDatafloat(getString(buf, "head_temperature"));
			mixture=getDatafloat(getString(buf, "mixture"));
			fuel[0]=getDatafloat(getString(buf, "fuel1"));
			if(fuel[0]==-65535){
				fuel[0]=getDatafloat(getString(buf, "fuel_pressure"))*10;
				fuelpressure=true;
			}
			else{
				fuelpressure=false;
			}
			fuel[1]=getDatafloat(getString(buf, "fuel2"));
			fuel[2]=getDatafloat(getString(buf, "fuel3"));
			fuel[3]=getDatafloat(getString(buf, "fuel4"));
			fuel_pressure=getDatafloat(getString(buf, "mixture"));
			oxygen=getDatafloat(getString(buf, "oxygen"));
			gears_lamp=getDatafloat(getString(buf, "gears_lamp"));
			flaps=getDatafloat(getString(buf, "flaps"));
			trimmer=getDatafloat(getString(buf, "trimmer"));
			throttle=getDatafloat(getString(buf, "throttle"));
			weapon1=getDatafloat(getString(buf, "weapon1"));
			weapon2=getDatafloat(getString(buf, "weapon2"));
			weapon3=getDatafloat(getString(buf, "weapon3"));
			prop_pitch_hour=getDatafloat(getString(buf, "prop_pitch_hour"));
			prop_pitch_min=getDatafloat(getString(buf, "prop_pitch_min"));
			ammo_counter1=getDatafloat(getString(buf, "ammo_counter1"));
			ammo_counter2=getDatafloat(getString(buf, "ammo_counter2"));
			ammo_counter3=getDatafloat(getString(buf, "ammo_counter3"));
			
			if(fuelnum==0){
				if(fuel[0]!=-65535)fuelnum=fuelnum+1;
				if(fuel[1]!=-65535)fuelnum=fuelnum+1;
				if(fuel[2]!=-65535)fuelnum=fuelnum+1;
				if(fuel[3]!=-65535)fuelnum=fuelnum+1;

			}
		
		}
		else{
			type="No Cockpit";
			stype="NoCockpit";
		}
	}
}