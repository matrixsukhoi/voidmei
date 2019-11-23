package parser;

import prog.controller;
import prog.language;
import prog.service;

public class flightAnalyzer {
	    public int engineType;
	    public String type;
		public double[] time = new double[185];// 从第零层开始
		public int power[] = new int[184];// 从第一层开始
		public int thrust[]=new int[184];
		public int eff[] = new int[184];
		public double sep[] = new double[184];
		public int initaltStage;
		public int curaltStage;
		boolean isInformation;
		
		service xs;
		int count;
		void init(int stage,service st){
			//System.out.println("analyzer初始化了");
			xs=st;
			count=1;
			isInformation=Boolean.parseBoolean(st.c.getconfig("enableAltInformation"));
			type=xs.iIndic.type;
			engineType=xs.sState.engineType;
			initaltStage = stage;
			curaltStage = initaltStage;
			time[curaltStage] = (xs.time / 1000f);
			power[curaltStage] = xs.totalhp;
			thrust[curaltStage]=xs.totalthr;
			eff[curaltStage] = xs.totalhpeff;
			sep[curaltStage] = xs.SEP;
			//System.out.println("已经记录stage"+curaltStage+"时间戳"+time[curaltStage]+"功率"+power[curaltStage]+"轴功率"+eff[curaltStage]+"SEP"+sep[curaltStage]);
		}
		void analyze(int stage){
			if (stage == curaltStage + 1) {
				eff[curaltStage]=eff[curaltStage]/count;
				sep[curaltStage]=sep[curaltStage]/(count*9.78f);
				//System.out.println("已经记录stage"+curaltStage+"时间戳"+time[curaltStage]+"功率"+power[curaltStage]+"推力"+thrust[curaltStage]+"轴功率"+eff[curaltStage]+"SEP"+sep[curaltStage]);
				curaltStage++;
				
				
				time[curaltStage] = (xs.time / 1000f);
				power[curaltStage] = xs.totalhp;
				thrust[curaltStage]=xs.totalthr;
				eff[curaltStage] = xs.totalhpeff;
				sep[curaltStage] = xs.SEP;
				count=1;
				if(isInformation)controller.notification(language.fA1+stage*100+language.fA2+(int)time[curaltStage]+language.fA3+(int)((stage-initaltStage)*1000/time[curaltStage])/10.0f+language.fA4);
			}
			else{
				eff[curaltStage] =(eff[curaltStage]+xs.totalhpeff);
				sep[curaltStage]=(sep[curaltStage]+xs.SEP);
				count++;
			}
		}
	}