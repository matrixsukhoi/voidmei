package prog;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.InputStream;

import javax.sound.sampled.AudioFormat;
import javax.sound.sampled.AudioInputStream;
import javax.sound.sampled.AudioSystem;
import javax.sound.sampled.Clip;
import javax.sound.sampled.DataLine;
import javax.sound.sampled.FloatControl;
import javax.sound.sampled.LineUnavailableException;
import javax.sound.sampled.UnsupportedAudioFileException;

import parser.indicators;
import parser.state;
import sun.audio.AudioPlayer;
import sun.audio.AudioStream;
// 语音告警

public class voiceWarning implements Runnable {
	long GCCheckMili;
	service xS;
	state st;
	indicators indic;
	Boolean doit;
	private boolean playCompleted;

	public void playWav(String Path) {
		InputStream in = null;
		try {
			in = new FileInputStream(Path);
		} catch (FileNotFoundException e1) {
			// TODO Auto-generated catch block
			e1.printStackTrace();
		}

		// create an audiostream from the inputstream
		AudioStream audioStream = null;
		try {
			audioStream = new AudioStream(in);
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}

		// play the audio clip with the audioplayer class
		AudioPlayer.player.start(audioStream);
		AudioPlayer.player.start(audioStream);
	}

	public static final int maxEndVoiceNum = 4;
	public float aoaWarningLine;
	public float iasWarningLine;
	public float machWarningLine;

	public class audClip {

		Clip clip;
		int cnt = 0;
		Boolean isAct;
		long lastTimePlay;
		long coolDown;

		public audClip(String path, long coolDownSeconds) {
			File audioFile = new File(path);
			playCompleted = false;
			Clip audioClip = null;
			try {
				AudioInputStream audioStream = AudioSystem.getAudioInputStream(audioFile);

				AudioFormat format = audioStream.getFormat();

				DataLine.Info info = new DataLine.Info(Clip.class, format);

				audioClip = (Clip) AudioSystem.getLine(info);

				// audioClip.addLineListener(this);

				audioClip.open(audioStream);

				FloatControl gainControl = (FloatControl) audioClip.getControl(FloatControl.Type.MASTER_GAIN);
				float range = gainControl.getMaximum() - gainControl.getMinimum();
//				System.out.println(range);
				// 
				float rangen = 0 - gainControl.getMinimum();
				float rangep = gainControl.getMaximum() - 0;
				float val = 0.0f;
				if (app.voiceVolumn <= 100){
					val = gainControl.getMinimum() + (float)Math.log10(app.voiceVolumn) * rangen/2.0f;
					if (val < gainControl.getMinimum()) val = gainControl.getMinimum();
				}
				
				// 大于100属于增益
				if (app.voiceVolumn > 100){
					val = (app.voiceVolumn - 100) * rangep/100.0f;
					if (val > gainControl.getMaximum()) val = gainControl.getMaximum();
					
				}

//				System.out.println(val);
				if (gainControl != null)gainControl.setValue(val);
//				Math.log10(app.voiceVolumn)/2.0f;
				// 映射使用log方式
				// f(x) [0, 200] -> [0, ]; 越接近1的越密 
				
				
				// audioClip.start();

				// audioClip.close();

			} catch (UnsupportedAudioFileException ex) {
				System.out.println("The specified audio file is not supported.");
				ex.printStackTrace();
			} catch (LineUnavailableException ex) {
				System.out.println("Audio line for playing back is unavailable.");
				ex.printStackTrace();
			} catch (IOException ex) {
				System.out.println("Error playing the audio file.");
				ex.printStackTrace();
			}
			// 获得clip
			this.clip = audioClip;
			this.cnt = 0;
			this.isAct = false;
			this.coolDown = coolDownSeconds * 1000;
		}

		public void playOnce(long time) {
			if (!this.isPlaying(time)) {

				this.isAct = true;
				this.lastTimePlay = time;
				// this.clip.stop();

				if (cnt++ == 0)
					this.clip.start();
				else
					this.clip.loop(1);
				// this.clip.start();
				// cnt++;
				// System.out.println(cnt);
			}
		}

		public Boolean isPlaying(long time) {
			if (this.isAct) {
				if (time - this.lastTimePlay <= this.coolDown) {
					return true;
				}
				this.isAct = this.clip.isRunning();
				return this.isAct;
			} else
				return this.isAct;
		}

		public void close() {
			this.clip.close();
			this.clip = null;

		}
	};

	Clip getClip(String audioFilePath) {
		File audioFile = new File(audioFilePath);
		playCompleted = false;
		Clip audioClip = null;
		try {
			AudioInputStream audioStream = AudioSystem.getAudioInputStream(audioFile);

			AudioFormat format = audioStream.getFormat();

			DataLine.Info info = new DataLine.Info(Clip.class, format);

			audioClip = (Clip) AudioSystem.getLine(info);

//			audioClip.getCon
			// audioClip.addLineListener(this);

			audioClip.open(audioStream);

			// audioClip.start();

			// audioClip.close();

		} catch (UnsupportedAudioFileException ex) {
			System.out.println("The specified audio file is not supported.");
			ex.printStackTrace();
		} catch (LineUnavailableException ex) {
			System.out.println("Audio line for playing back is unavailable.");
			ex.printStackTrace();
		} catch (IOException ex) {
			System.out.println("Error playing the audio file.");
			ex.printStackTrace();
		}

		return audioClip;

	}

	// 攻角提示
	audClip aoaCrit;
	private controller xc;
	private audClip iasWarn;
	private float gearWarningLine;
	private audClip gearWarn;
	private audClip engWarn;
	private audClip fuelWarn;
	private int lowfuelWarningLine;
	private audClip machWarn;
	private long fuelCheck;
	private audClip engFail;
	private audClip heightWarn;
	private audClip fuelPrsWarn;
	private int fuelPCheck;
	public float speedWarningLine;
	public float nyWarningLine0;
	public float nyWarningLine1;
	private audClip varioWarn;
	private audClip stallWarn;
	private audClip nyWarn;
	private audClip oofWarn;
	private audClip flapWarn;
	private audClip engFailInvert;
	private audClip rpmThrottleWarn;
	private audClip rpmLowWarn;
	private audClip rpmHighWarn;
	private audClip terrainWarn;
	private boolean isGearAlive;
	private boolean isFlapAlive;

	public void init(controller c, service S) {
		xS = S;
		xc = c;
		st = xS.sState;
		indic = xS.sIndic;
		// 加载其他
		aoaCrit = new audClip("./voice/aoalimit.wav", 2);
		aoaWarningLine = 15;
		if (xc.blkx != null && xc.blkx.valid)
			aoaWarningLine = xc.blkx.NoFlapsWing.AoACritHigh;

		//
		// rpmThrottleWarn = new audClip("./voice/warn_pitch.wav", 10);
		rpmLowWarn = new audClip("./voice/warn_lowrpm.wav", 10);
		rpmHighWarn = new audClip("./voice/warn_highrpm.wav", 10);

		// 襟翼
		flapWarn = new audClip("./voice/warn_flap.wav", 5);

		// 失速
		speedWarningLine = 0;
		if (xc.blkx != null && xc.blkx.valid)
			speedWarningLine = xc.blkx.CriticalSpeed * 3.6f;
		if (speedWarningLine == 0)
			speedWarningLine = 100;
		// System.out.println(speedWarningLine);
		stallWarn = new audClip("./voice/warn_stall.wav", 2);

		// 过载
		nyWarningLine0 = 0;
		nyWarningLine1 = 0;
		if (xc.blkx != null && xc.blkx.valid) {
			nyWarningLine0 = xc.blkx.maxAllowGload[0];
			nyWarningLine1 = xc.blkx.maxAllowGload[1];
		}
		if (nyWarningLine0 == 0)
			nyWarningLine0 = -4;
		if (nyWarningLine1 == 0)
			nyWarningLine1 = 10;
		// System.out.println(nyWarningLine0);
		// System.out.println(nyWarningLine1);
		nyWarn = new audClip("./voice/warn_loadfactor.wav", 2);

		// ias
		iasWarn = new audClip("./voice/warn_ias.wav", 10);
		iasWarningLine = 0;
		if (xc.blkx != null && xc.blkx.valid)
			iasWarningLine = xc.blkx.vne;
		if (iasWarningLine == 0)
			iasWarningLine = 2000;

		// mach
		machWarn = new audClip("./voice/warn_mach.wav", 10);
		machWarningLine = 0;
		if (xc.blkx != null && xc.blkx.valid)
			machWarningLine = xc.blkx.vneMach;
		if (machWarningLine == 0)
			machWarningLine = 3.0f;
		// System.out.println(machWarningLine);

		// gear
		gearWarningLine = 0;
		gearWarn = new audClip("./voice/warn_gear.wav", 7);
		if (xc.blkx != null && xc.blkx.valid)
			gearWarningLine = xc.blkx.GearDestructionIndSpeed;
		if (gearWarningLine == 0)
			gearWarningLine = 450;
		// System.out.println(gearWarningLine);

		// 温度
		engWarn = new audClip("./voice/warn_engineoverheat.wav", 60);

		// 引擎失效
		engFail = new audClip("./voice/fail_engine.wav", 60);
		engFailInvert = new audClip("./voice/fail_engine.wav", 5);

		// 高度
		heightWarn = new audClip("./voice/warn_altitude.wav", 5);

		// 雷达高度
		terrainWarn = new audClip("./voice/warn_terrain.wav", 5);

		// 燃油
		lowfuelWarningLine = 10;
		fuelWarn = new audClip("./voice/warn_lowfuel.wav", 60);

		fuelPrsWarn = new audClip("./voice/warn_lowpressure.wav", 30);
		oofWarn = new audClip("./voice/fail_nofuel.wav", 60);

		// 下降率
		varioWarn = new audClip("./voice/warn_highvario.wav", 5);

		// 初始化
		audClip aC = new audClip("./voice/start1.wav", 1);
		aC.playOnce(xS.SystemTime);
		// aC.close();
		aC = null;
		engDamage = false;
		isGearAlive = true;
		isFlapAlive = true;
		doit = Boolean.TRUE;
	}

	public boolean engDamage;
	private int oofCheck;
	private long gearCheck;
	private long flapCheck;
	public static final long sleepTime = 100;

	// 问题:不能收起落架的飞机怎么办??

	@Override
	public void run() {
		try {
			Thread.sleep(1000);
		} catch (InterruptedException e1) {
			// TODO Auto-generated catch block
			e1.printStackTrace();
		}
		while (doit) {

			try {
				Thread.sleep(sleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			Boolean noRPM = false;
			long t = xS.SystemTime;

			// 攻角判断
			if (xS.playerLive && st.IAS > 80 && st.AoA >= aoaWarningLine - 1)
				aoaCrit.playOnce(t);

			// 速度判断
			if (st.IAS >= iasWarningLine - 40)
				iasWarn.playOnce(t);

			// 马赫数判断
			if (st.M >= machWarningLine - 0.05f)
				machWarn.playOnce(t);

			// 起落架判断
			if (isGearAlive && st.gear > 0 && st.IAS >= gearWarningLine - 25) {
				gearWarn.playOnce(t);
			}
			// 襟翼判断逻辑
			// 先得判断襟翼在哪个段内
			// 使用线性方式获得襟翼的限速
			if (isFlapAlive && st.IAS >= xS.flapAllowSpeed - 25) {
				flapWarn.playOnce(t);
			}

			// 下降率逻辑
			if (isGearAlive && st.gear >= 50 && st.Vy <= -8) {
				// 下降率高
				varioWarn.playOnce(t);
			}

			// 起落架完好性判断
			if (isGearAlive && st.IAS > gearWarningLine) {
				// 超速持续10秒
				gearCheck += sleepTime;
				if (gearCheck >= 10000) {
					gearCheck = 0;
					isGearAlive = false;
				}
			} else {
				gearCheck = 0;
			}
			if (isFlapAlive && st.IAS > xS.flapAllowSpeed) {
				// 超速持续10秒
				flapCheck += sleepTime;
				if (flapCheck >= 10000) {
					flapCheck = 0;
					isFlapAlive = false;
				}
			} else {
				flapCheck = 0;
			}

			// 恢复起落架逻辑
			if (!isGearAlive && st.gear == 0) {
				// 恢复起落架
				isGearAlive = true;
			}
			if (!isFlapAlive && st.flaps == 0) {
				isFlapAlive = true;
			}

			//

			if (xS.curLoadMinWorkTime < 300 * 1000) {
				engWarn.playOnce(t);
			}

			if (xS.fTotalFuel == 0) {
				if (oofCheck++ > 16) {
					oofCheck = 0;
					oofWarn.playOnce(t);
				}
			}
			if (xS.fuelPercent <= lowfuelWarningLine) {
				// 持续16tick以上,解决退出游戏时的油告警
				if (fuelCheck++ > 16) {
					fuelCheck = 0;
					fuelWarn.playOnce(t);
				}
			}

			// 下降率等于高度的10分之一会触发警告

			// TODO: 使用无线电高度

			if (st.gear <= 0 && xS.playerLive && st.Vy < -st.heightm / 10.0f) {
				heightWarn.playOnce(t);
			} else {
				// 触发高度警告，但触发地形警告
				if (st.gear <= 0 && xS.playerLive && xS.radioAlt > 0) {
					if (xS.dRadioAlt < -xS.radioAlt / 10.0f) {
						terrainWarn.playOnce(t);
					}
				}
			}

			if ((indic.fuel_pressure >= 0) && st.throttle - indic.fuel_pressure * 10 > 2.0f) {
				fuelPCheck += sleepTime;
				if (fuelPCheck >= 2000) {
					fuelPCheck = 0;
					fuelPrsWarn.playOnce(t);
					engDamage = true;
				}
			} else {
				fuelPCheck = 0;
			}

			if (engDamage && indic.fuel_pressure == 0) {
				fuelPCheck += sleepTime;
				if (fuelPCheck >= 1000) {
					engFail.playOnce(t);
					engDamage = false;
				}
			}

			// 断油逻辑

			if (st.Ny < 0 && st.throttle > 50) {
				if (st.thrust[0] < 50) {
					// System.out.println(xS.totalthr);
					noRPM = true;
					engFailInvert.playOnce(t);
				}
				// if (st.engineType == 0 && xS.totalhp == 0){
				// System.out.println("??");
				// engFailInvert.playOnce(t);
				// }
			}
			// 超过30
			// 桨距
			if (!noRPM) {
				// 定距桨特殊处理,即不是喷气但是桨距无效
				if (!(!xS.isEngJet() && st.RPMthrottle < 0)) {
					if (st.throttle - 30 > st.RPM * 100.0f / xS.maximumThrRPM) {
						// 转速低
						rpmLowWarn.playOnce(t);
					}
				}

				// 超过30
				// System.out.println(st.RPM * 100.0f/xS.maximumThrRPM -
				// st.throttle);
				if (xS.getMaximumRPM && xS.maximumThrRPM > 0 && st.RPM * 100.0f / xS.maximumThrRPM >= 105) {
					// 转速高
					rpmHighWarn.playOnce(t);
				}
			}

			// 失速逻辑
			// 有起落架
			if (xS.playerLive && st.gear == 0 && st.Vy != 0&& st.IAS <= speedWarningLine) {
				// 失速
				stallWarn.playOnce(t);
			}
//			// 无起落架不告失速
//			if(st.engineAlive && st.gear < 0 && st.Vy != 0 && st.IAS <= speedWarningLine && st.IAS > 70){
//				// 失速
//				stallWarn.playOnce(t);
//			}

			// 过载
			if (xS.playerLive && st.Ny > nyWarningLine1 || st.Ny < nyWarningLine0) {
				// 高过载
				nyWarn.playOnce(t);
			}

			// 桨距提示
			// if (xc.blkx != null && xc.blkx.valid && !xc.blkx.isJet &&
			// st.throttle - st.RPMthrottle > 50){
			// rpmThrottleWarn.playOnce(t);
			// }

			// 负G是否直接可以判断油压?

		}
		// try {
		// Thread.sleep(1000);
		// } catch (InterruptedException e1) {
		// // TODO Auto-generated catch block
		// e1.printStackTrace();
		// }
		// 随机播放
		int i = (int) (Math.random() * maxEndVoiceNum);
		audClip aC = new audClip("./voice/end" + i + ".wav", 1);
		aC.playOnce(xS.SystemTime);
		// aC.close();
		aC = null;
	}
}
