package parser;
import prog.Application;
import prog.util.StringHelper;

import prog.Service;

public class Indicators{
	public String valid;
	public String type;
	public boolean isdummyplane=false;
	public String stype;
	public boolean flag;
//	public boolean fuelpressure;
	public double speed;
	public double pedals;
	public double stick_elevator;
	public double stick_ailerons;
	public double altitude_hour;
	public double altitude_min;
	public double altitude_10k;
	public double bank;
	public double turn;
	public double compass;
	public double clock_hour;
	public double clock_min;
	public double clock_sec;
	public double manifold_pressure;
	public double rpm;
	public double oil_pressure;
	public double water_temperature;
	public double engine_temperature;
	public double mixture;
	public double fuel[];
	public double fuel_pressure;
	public double oxygen;
	public double gears_lamp;
	public double flaps;
	public double trimmer;
	public double throttle;
	public double weapon1;
	public double weapon2;
	public double weapon3;
	public double prop_pitch_hour;
	public double prop_pitch_min;
	public double ammo_counter1;
	public double ammo_counter2;
	public double ammo_counter3;
	public double oilTemp;
	public double waterTemp;
	public int fuelnum;
	public double vario;
	public double aviahorizon_pitch;
	public double aviahorizon_roll;
	public double wsweep_indicator;
	public double radio_altitude;
	private String army;


	public void init() {
		//Application.debugPrint("indicator初始化了");
		valid = Service.nastring;
		fuelnum=0;
		fuel = new double[5];
		flag = false;
//		fuelpressure=false;
	}
	
	public void update(String buf) {
		valid = StringHelper.getString(buf, "valid");
		army = StringHelper.getString(buf, "army");
		if (valid != null && valid.equals("true") && !army.equals("tank")){
			flag=true;
			type=StringHelper.getString(buf, "type").toUpperCase();
		
			if(type.length()>0){
				type=type.substring(1, type.length()-1);
				//判定是否dummyplane
				//if(type.equals("DUMMY_PLANE"))isdummyplane=true;
				//else isdummyplane=false;
				if(type.length()>9)stype=type.substring(0, 8);
				else stype=type;
				
			}
			
			speed=StringHelper.getDataFloat(StringHelper.getString(buf, "speed"));
			pedals=StringHelper.getDataFloat(StringHelper.getString(buf, "pedals"));
			stick_elevator=StringHelper.getDataFloat(StringHelper.getString(buf, "stick_elevator"));
			stick_ailerons=StringHelper.getDataFloat(StringHelper.getString(buf, "stick_ailerons"));
			altitude_hour=StringHelper.getDataFloat(StringHelper.getString(buf, "altitude_hour"));
			altitude_min=StringHelper.getDataFloat(StringHelper.getString(buf, "altitude_min"));;
			altitude_10k=StringHelper.getDataFloat(StringHelper.getString(buf, "altitude_10k"));
			bank=StringHelper.getDataFloat(StringHelper.getString(buf, "bank"));
			turn=StringHelper.getDataFloat(StringHelper.getString(buf, "turn"));
			compass=StringHelper.getDataFloat(StringHelper.getString(buf, "compass"));
			clock_hour=StringHelper.getDataFloat(StringHelper.getString(buf, "clock_hour"));
			clock_min=StringHelper.getDataFloat(StringHelper.getString(buf, "clock_min"));
			clock_sec=StringHelper.getDataFloat(StringHelper.getString(buf, "clock_sec"));
			manifold_pressure=StringHelper.getDataFloat(StringHelper.getString(buf, "manifold_pressure"));
			rpm=StringHelper.getDataFloat(StringHelper.getString(buf, "rpm"));
			wsweep_indicator = StringHelper.getDataFloat(StringHelper.getString(buf, "wing_sweep_indicator"));
//			Application.debugPrint(wsweep_indicator);
			oil_pressure=StringHelper.getDataFloat(StringHelper.getString(buf, "oil_pressure"));
//			water_temperature=StringHelper.getDatadouble(StringHelper.getString(buf, "water_temperature"));
			engine_temperature=StringHelper.getDataFloat(StringHelper.getString(buf, "head_temperature"));
			mixture=StringHelper.getDataFloat(StringHelper.getString(buf, "mixture"));
			
			// 防止读到油压
			fuel[0]=StringHelper.getDataFloat(StringHelper.getString(buf, "\"fuel\""));
			fuelnum = 1;
			for (int i = 1 ; i < 5; i++){
				fuel[i] = StringHelper.getDataFloat(StringHelper.getString(buf, "fuel"+i));
				if(fuel[i] == -65535) fuel[i] = 0;
				else fuelnum += 1;
			}
//			fuel[0]=StringHelper.getDatadouble(StringHelper.getString(buf, "fuel1"));
//			if (fuel[0] == -65535){
//				fuel[0] = StringHelper.getDatadouble(StringHelper.getString(buf, "fuel"));
//				if
//			}
			aviahorizon_pitch = StringHelper.getDataFloat(StringHelper.getString(buf, "aviahorizon_pitch"));
			aviahorizon_roll = StringHelper.getDataFloat(StringHelper.getString(buf, "aviahorizon_roll"));
			radio_altitude = StringHelper.getDataFloat(StringHelper.getString(buf, "radio_altitude"));
			oilTemp = StringHelper.getDataFloat(StringHelper.getString(buf, "oil_temperature"));
			waterTemp = StringHelper.getDataFloat(StringHelper.getString(buf, "water_temperature"));
			if(fuel[0]==-65535){
				fuel[0] = 0;
//				fuel[0]=StringHelper.getDatadouble(StringHelper.getString(buf, "fuel_pressure"))*10;
//				fuelpressure=true;
			}
//			else{
//				fuelpressure=false;
//			}
//			fuelpressure=false;
//			fuel[1]=StringHelper.getDataFloat(StringHelper.getString(buf, "fuel2"));
//			fuel[2]=StringHelper.getDataFloat(StringHelper.getString(buf, "fuel3"));
//			fuel[3]=StringHelper.getDataFloat(StringHelper.getString(buf, "fuel4"));
//			if(fuelnum==0){
//				if(fuel[0]!=-65535)fuelnum=fuelnum+1;
//				if(fuel[1]!=-65535)fuelnum=fuelnum+1;
//				if(fuel[2]!=-65535)fuelnum=fuelnum+1;
//				if(fuel[3]!=-65535)fuelnum=fuelnum+1;
//
//			}
			
			fuel_pressure=StringHelper.getDataFloat(StringHelper.getString(buf, "fuel_pressure"));
			oxygen=StringHelper.getDataFloat(StringHelper.getString(buf, "oxygen"));
			gears_lamp=StringHelper.getDataFloat(StringHelper.getString(buf, "gears_lamp"));
			flaps=StringHelper.getDataFloat(StringHelper.getString(buf, "flaps"));
			vario = StringHelper.getDataFloat(StringHelper.getString(buf, "vario"));
			trimmer=StringHelper.getDataFloat(StringHelper.getString(buf, "trimmer"));
			throttle=StringHelper.getDataFloat(StringHelper.getString(buf, "throttle"));
			weapon1=StringHelper.getDataFloat(StringHelper.getString(buf, "weapon1"));
			weapon2=StringHelper.getDataFloat(StringHelper.getString(buf, "weapon2"));
			weapon3=StringHelper.getDataFloat(StringHelper.getString(buf, "weapon3"));
			prop_pitch_hour=StringHelper.getDataFloat(StringHelper.getString(buf, "prop_pitch_hour"));
			prop_pitch_min=StringHelper.getDataFloat(StringHelper.getString(buf, "prop_pitch_min"));
			ammo_counter1=StringHelper.getDataFloat(StringHelper.getString(buf, "ammo_counter1"));
			ammo_counter2=StringHelper.getDataFloat(StringHelper.getString(buf, "ammo_counter2"));
			ammo_counter3=StringHelper.getDataFloat(StringHelper.getString(buf, "ammo_counter3"));
			

		
		}
		else{
			type="No Cockpit";
			stype="NoCockpit";

			flag=false;
		}
	}
}