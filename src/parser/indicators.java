package parser;

import prog.service;

public class indicators{
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


	public void init() {
		//app.debugPrint("indicator初始化了");
		valid = service.nastring;
		fuelnum=0;
		fuel = new double[5];
		flag = false;
//		fuelpressure=false;
	}
	
	public void update(String buf) {
		valid = stringHelper.getString(buf, "valid");
		if (valid.equals("true")){
			flag=true;
			type=stringHelper.getString(buf, "type").toUpperCase();
		
			if(type.length()>0){
				type=type.substring(1, type.length()-1);
				//判定是否dummyplane
				//if(type.equals("DUMMY_PLANE"))isdummyplane=true;
				//else isdummyplane=false;
				if(type.length()>9)stype=type.substring(0, 8);
				else stype=type;
				
			}
			
			speed=stringHelper.getDataFloat(stringHelper.getString(buf, "speed"));
			pedals=stringHelper.getDataFloat(stringHelper.getString(buf, "pedals"));
			stick_elevator=stringHelper.getDataFloat(stringHelper.getString(buf, "stick_elevator"));
			stick_ailerons=stringHelper.getDataFloat(stringHelper.getString(buf, "stick_ailerons"));
			altitude_hour=stringHelper.getDataFloat(stringHelper.getString(buf, "altitude_hour"));
			altitude_min=stringHelper.getDataFloat(stringHelper.getString(buf, "altitude_min"));;
			altitude_10k=stringHelper.getDataFloat(stringHelper.getString(buf, "altitude_10k"));
			bank=stringHelper.getDataFloat(stringHelper.getString(buf, "bank"));
			turn=stringHelper.getDataFloat(stringHelper.getString(buf, "turn"));
			compass=stringHelper.getDataFloat(stringHelper.getString(buf, "compass"));
			clock_hour=stringHelper.getDataFloat(stringHelper.getString(buf, "clock_hour"));
			clock_min=stringHelper.getDataFloat(stringHelper.getString(buf, "clock_min"));
			clock_sec=stringHelper.getDataFloat(stringHelper.getString(buf, "clock_sec"));
			manifold_pressure=stringHelper.getDataFloat(stringHelper.getString(buf, "manifold_pressure"));
			rpm=stringHelper.getDataFloat(stringHelper.getString(buf, "rpm"));
			wsweep_indicator = stringHelper.getDataFloat(stringHelper.getString(buf, "wing_sweep_indicator"));
//			app.debugPrint(wsweep_indicator);
			oil_pressure=stringHelper.getDataFloat(stringHelper.getString(buf, "oil_pressure"));
//			water_temperature=stringHelper.getDatadouble(stringHelper.getString(buf, "water_temperature"));
			engine_temperature=stringHelper.getDataFloat(stringHelper.getString(buf, "head_temperature"));
			mixture=stringHelper.getDataFloat(stringHelper.getString(buf, "mixture"));
			
			// 防止读到油压
			fuel[0]=stringHelper.getDataFloat(stringHelper.getString(buf, "\"fuel\""));
			fuelnum = 1;
			for (int i = 1 ; i < 5; i++){
				fuel[i] = stringHelper.getDataFloat(stringHelper.getString(buf, "fuel"+i));
				if(fuel[i] == -65535) fuel[i] = 0;
				else fuelnum += 1;
			}
//			fuel[0]=stringHelper.getDatadouble(stringHelper.getString(buf, "fuel1"));
//			if (fuel[0] == -65535){
//				fuel[0] = stringHelper.getDatadouble(stringHelper.getString(buf, "fuel"));
//				if
//			}
			aviahorizon_pitch = stringHelper.getDataFloat(stringHelper.getString(buf, "aviahorizon_pitch"));
			aviahorizon_roll = stringHelper.getDataFloat(stringHelper.getString(buf, "aviahorizon_roll"));
			radio_altitude = stringHelper.getDataFloat(stringHelper.getString(buf, "radio_altitude"));
			oilTemp = stringHelper.getDataFloat(stringHelper.getString(buf, "oil_temperature"));
			waterTemp = stringHelper.getDataFloat(stringHelper.getString(buf, "water_temperature"));
			if(fuel[0]==-65535){
				fuel[0] = 0;
//				fuel[0]=stringHelper.getDatadouble(stringHelper.getString(buf, "fuel_pressure"))*10;
//				fuelpressure=true;
			}
//			else{
//				fuelpressure=false;
//			}
//			fuelpressure=false;
//			fuel[1]=stringHelper.getDataFloat(stringHelper.getString(buf, "fuel2"));
//			fuel[2]=stringHelper.getDataFloat(stringHelper.getString(buf, "fuel3"));
//			fuel[3]=stringHelper.getDataFloat(stringHelper.getString(buf, "fuel4"));
//			if(fuelnum==0){
//				if(fuel[0]!=-65535)fuelnum=fuelnum+1;
//				if(fuel[1]!=-65535)fuelnum=fuelnum+1;
//				if(fuel[2]!=-65535)fuelnum=fuelnum+1;
//				if(fuel[3]!=-65535)fuelnum=fuelnum+1;
//
//			}
			
			fuel_pressure=stringHelper.getDataFloat(stringHelper.getString(buf, "fuel_pressure"));
			oxygen=stringHelper.getDataFloat(stringHelper.getString(buf, "oxygen"));
			gears_lamp=stringHelper.getDataFloat(stringHelper.getString(buf, "gears_lamp"));
			flaps=stringHelper.getDataFloat(stringHelper.getString(buf, "flaps"));
			vario = stringHelper.getDataFloat(stringHelper.getString(buf, "vario"));
			trimmer=stringHelper.getDataFloat(stringHelper.getString(buf, "trimmer"));
			throttle=stringHelper.getDataFloat(stringHelper.getString(buf, "throttle"));
			weapon1=stringHelper.getDataFloat(stringHelper.getString(buf, "weapon1"));
			weapon2=stringHelper.getDataFloat(stringHelper.getString(buf, "weapon2"));
			weapon3=stringHelper.getDataFloat(stringHelper.getString(buf, "weapon3"));
			prop_pitch_hour=stringHelper.getDataFloat(stringHelper.getString(buf, "prop_pitch_hour"));
			prop_pitch_min=stringHelper.getDataFloat(stringHelper.getString(buf, "prop_pitch_min"));
			ammo_counter1=stringHelper.getDataFloat(stringHelper.getString(buf, "ammo_counter1"));
			ammo_counter2=stringHelper.getDataFloat(stringHelper.getString(buf, "ammo_counter2"));
			ammo_counter3=stringHelper.getDataFloat(stringHelper.getString(buf, "ammo_counter3"));
			

		
		}
		else{
			type="No Cockpit";
			stype="NoCockpit";

			flag=false;
		}
	}
}