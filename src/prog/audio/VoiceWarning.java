package prog.audio;

import prog.Application;
import prog.Controller;
import prog.Service;
import prog.config.ConfigProvider;
import prog.util.Logger;

import java.io.File;
import java.io.IOException;
import java.util.concurrent.ConcurrentHashMap;

import javax.sound.sampled.AudioFormat;
import javax.sound.sampled.AudioInputStream;
import javax.sound.sampled.AudioSystem;
import javax.sound.sampled.Clip;
import javax.sound.sampled.DataLine;
import javax.sound.sampled.LineUnavailableException;
import javax.sound.sampled.UnsupportedAudioFileException;

import parser.Indicators;
import parser.State;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import prog.event.EventPayload;

/**
 * 语音告警系统
 *
 * 线程模型：
 * - Service 线程 → run() → VoiceAlert.playOnce()
 * - UI 线程 → configHandler → VoiceAlert.reload()
 * - FlightDataBus → flightDataListener → currentMismatch (volatile)
 *
 * 重构说明：
 * - 使用 ConcurrentHashMap 替代 HashMap 保证线程安全
 * - VoiceAlert 类使用 volatile 字段和 synchronized reload() 保证线程安全
 * - run() 方法拆分为独立的 check* 方法，每个方法负责一种告警
 */
public class VoiceWarning implements Runnable {
    long GCCheckMili;
    Service xS;

    State st;
    Indicators indic;
    volatile Boolean doit;  // volatile 保证可见性
    private boolean playCompleted;

    // Legacy tool for direct playWav
    public void playWav(String Path) {
        try {
            File audioFile = new File(Path);
            AudioInputStream audioStream = AudioSystem.getAudioInputStream(audioFile);
            AudioFormat format = audioStream.getFormat();
            DataLine.Info info = new DataLine.Info(Clip.class, format);
            Clip audioClip = (Clip) AudioSystem.getLine(info);
            audioClip.open(audioStream);
            audioClip.start();
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    public static final int maxEndVoiceNum = 4;
    public double aoaWarningLine;
    public double iasWarningLine;
    public double machWarningLine;

    // 使用 ConcurrentHashMap 保证线程安全（UI 线程热重载，Service 线程播放）
    private ConcurrentHashMap<String, VoiceAlert> alerts = new ConcurrentHashMap<>();

    // Config listener
    private java.util.function.Consumer<Object> configHandler;

    /**
     * 线程安全的语音告警封装类
     *
     * 线程安全保证：
     * - clip 字段使用 volatile
     * - reload() 方法使用 synchronized
     * - playOnce() 不需要同步（只读 clip 引用，播放操作由底层 Clip 保证）
     */
    public class VoiceAlert {
        private volatile Clip clip;           // volatile 确保可见性
        private volatile boolean available;   // volatile 确保可见性
        volatile boolean isAct;               // volatile 确保可见性, package-private for aoaHigh special case
        volatile long lastTimePlay;           // volatile 确保可见性, package-private for aoaHigh special case
        private final long coolDownMs;
        private final String key;

        public VoiceAlert(String key, long coolDownSeconds) {
            this.key = key;
            this.coolDownMs = coolDownSeconds * 1000;
            this.isAct = false;

            // 注册到 alerts map
            alerts.put(key, this);

            reload();
        }

        /**
         * 重新加载音频资源（线程安全）
         * 必须同步以防止多个线程同时 reload
         */
        public synchronized void reload() {
            // 先关闭旧资源
            Clip oldClip = this.clip;
            if (oldClip != null) {
                try {
                    if (oldClip.isRunning()) {
                        oldClip.stop();
                    }
                    oldClip.close();
                } catch (Exception e) {
                    Logger.warn("VoiceAlert", "关闭旧 Clip 失败: " + key);
                }
            }

            // 解析配置
            String configKey = VoicePackConfig.withVoicePrefix(key);
            String val = configProvider.getConfig(configKey);
            VoicePackConfig config = VoicePackConfig.parse(val);

            if (!config.enabled) {
                this.available = false;
                this.clip = null;
                return;
            }

            // 加载新 Clip
            this.clip = VoiceResourceManager.getInstance().loadClip(key, config.packName);
            this.available = (this.clip != null);
        }

        /**
         * 播放一次（带冷却时间检查）
         * 不需要同步：volatile 保证读取最新值，播放操作是原子的
         */
        public void playOnce(long time) {
            if (isPlaying(time)) {
                return;
            }

            this.isAct = true;
            this.lastTimePlay = time;

            Clip c = this.clip;  // 本地引用，避免中途被 reload 改变
            if (!this.available || c == null) {
                return;
            }

            try {
                c.setFramePosition(0);
                c.start();
            } catch (Exception e) {
                Logger.debug("VoiceAlert", "播放失败: " + key + " - " + e.getMessage());
            }
        }

        /**
         * 检查是否正在播放（或在冷却期内）
         */
        public boolean isPlaying(long time) {
            if (!this.available || this.clip == null) {
                return true;  // 不可用时假装在播放，防止重试循环
            }

            if (this.isAct) {
                if (time - this.lastTimePlay <= this.coolDownMs) {
                    return true;  // 在冷却期内
                }
                Clip c = this.clip;
                this.isAct = (c != null && c.isRunning());
                return this.isAct;
            }
            return false;
        }

        /**
         * 关闭资源
         */
        public void close() {
            Clip c = this.clip;
            if (c != null) {
                try {
                    if (c.isRunning()) {
                        c.stop();
                    }
                    c.close();
                } catch (Exception e) {
                    // ignore
                }
            }
            this.clip = null;
        }

        public String getKey() {
            return key;
        }
    }

    // Legacy method for backward compatibility
    Clip getClip(String audioFilePath) {
        File audioFile = new File(audioFilePath);
        playCompleted = false;
        Clip audioClip = null;
        try {
            AudioInputStream audioStream = AudioSystem.getAudioInputStream(audioFile);
            AudioFormat format = audioStream.getFormat();
            DataLine.Info info = new DataLine.Info(Clip.class, format);
            audioClip = (Clip) AudioSystem.getLine(info);
            audioClip.open(audioStream);
        } catch (UnsupportedAudioFileException ex) {
            Application.debugPrint("The specified audio file is not supported.");
            ex.printStackTrace();
        } catch (LineUnavailableException ex) {
            Application.debugPrint("Audio line for playing back is unavailable.");
            ex.printStackTrace();
        } catch (IOException ex) {
            Application.debugPrint("Error playing the audio file.");
            ex.printStackTrace();
        }
        return audioClip;
    }

    // 攻角提示
    private VoiceAlert aoaCrit;
    private VoiceAlert aoaHigh;
    private Controller xc;
    /** 配置提供者，用于访问配置而不依赖 Controller */
    private ConfigProvider configProvider;

    // 速度相关
    private VoiceAlert iasWarn;
    private VoiceAlert machWarn;
    private VoiceAlert stallWarn;

    // 起落架/襟翼/减速板
    private double gearWarningLine;
    private VoiceAlert gearWarn;
    private VoiceAlert flapWarn;
    private VoiceAlert brakeWarn;
    private boolean isGearAlive;
    private boolean isFlapAlive;
    private long gearCheck;
    private long flapCheck;

    // 过载
    public double nyWarningLine0;
    public double nyWarningLine1;
    private VoiceAlert nyWarn;
    private parser.Blkx blkx;
    private double nofuelweight;

    // 引擎相关
    private VoiceAlert engWarn;
    private VoiceAlert engFail;
    private VoiceAlert engFailInvert;
    private VoiceAlert rpmLowWarn;
    private VoiceAlert rpmHighWarn;
    public boolean engDamage;

    // 燃油相关
    private int lowfuelWarningLine;
    private VoiceAlert fuelWarn;
    private VoiceAlert fuelPrsWarn;
    private VoiceAlert oofWarn;
    private long fuelCheck;
    private int fuelPCheck;
    private int oofCheck;

    // 高度相关
    private VoiceAlert heightWarn;
    private VoiceAlert terrainWarn;
    private VoiceAlert varioWarn;

    // 舵效相关
    private int rudderEffIAS;
    private int elevatorEffIAS;
    private int aileronEffIAS;
    private VoiceAlert rudderEff;
    private VoiceAlert elevatorEff;
    private VoiceAlert aileronEff;
    private boolean elevatorEffCheck = false;
    private boolean aileronEffCheck = false;
    private boolean rudderEffCheck = false;

    // 增压器档位告警
    private VoiceAlert compressorStageWarn;
    private volatile boolean currentMismatch = false;  // volatile for thread safety (from FlightDataBus)
    private boolean lastMismatch = false;              // For detecting state change (false→true, true→false)
    private long pendingCompressorWarnTime = 0;        // 0 = no pending warning, >0 = scheduled warning time
    private static final long COMPRESSOR_WARN_DELAY = 3000;  // 3-second delay before warning
    private FlightDataListener flightDataListener;

    // 常量
    public static final long sleepTime = 100;
    private static final long GEAR_DAMAGE_THRESHOLD_MS = 10000;  // 起落架超速 10 秒后标记为损坏
    private static final long FLAP_DAMAGE_THRESHOLD_MS = 15000;  // 襟翼超速 15 秒后标记为损坏

    public void init(Controller c, Service S) {
        if (S == null) {
            doit = false;
            return;
        }
        xS = S;
        xc = c;
        // 使用 ConfigurationService 作为 ConfigProvider，避免依赖 Controller 的委托方法
        this.configProvider = c.getConfigService();
        st = xS.sState;
        indic = xS.sIndic;

        // Config Listener - 使用 VoicePackConfig 工具方法
        if (configHandler == null) {
            configHandler = key -> {
                if (key instanceof String) {
                    String strKey = (String) key;
                    if (strKey.startsWith(VoicePackConfig.VOICE_PREFIX)) {
                        String alertKey = VoicePackConfig.stripVoicePrefix(strKey);
                        VoiceAlert alert = alerts.get(alertKey);  // ConcurrentHashMap 线程安全
                        if (alert != null) {
                            alert.reload();  // synchronized 方法
                            Logger.info("VoiceWarning", "Reloaded voice clip: " + alertKey);
                        }
                    }
                }
            };
            prog.event.UIStateBus.getInstance().subscribe(prog.event.UIStateEvents.CONFIG_CHANGED, configHandler);
        }

        // 初始化告警 - 使用 VoiceAlertType 获取默认冷却时间
        initAoAWarnings(c);
        initSpeedWarnings(c);
        initStructureWarnings(c);
        initEngineWarnings();
        initFuelWarnings();
        initAltitudeWarnings();
        initControlEffectivenessWarnings(c);
        initCompressorWarning();

        // 播放启动音效
        VoiceAlert startSound = new VoiceAlert("start1", 1);
        startSound.playOnce(xS.currentTimeMs);

        // 初始化状态
        engDamage = false;
        isGearAlive = true;
        isFlapAlive = true;
        lastMismatch = false;
        pendingCompressorWarnTime = 0;
        doit = Boolean.TRUE;
    }

    /**
     * 初始化攻角告警
     */
    private void initAoAWarnings(Controller c) {
        aoaCrit = new VoiceAlert("aoaCrit", VoiceAlertType.AOA_CRIT.getCooldownSeconds());
        aoaHigh = new VoiceAlert("aoaHigh", VoiceAlertType.AOA_HIGH.getCooldownSeconds());
        aoaWarningLine = 15;

        parser.Blkx b = c.getBlkx();
        if (b != null && b.valid) {
            aoaWarningLine = b.NoFlapsWing.AoACritHigh;
        }
    }

    /**
     * 初始化速度告警
     */
    private void initSpeedWarnings(Controller c) {
        iasWarn = new VoiceAlert("warn_ias", VoiceAlertType.WARN_IAS.getCooldownSeconds());
        machWarn = new VoiceAlert("warn_mach", VoiceAlertType.WARN_MACH.getCooldownSeconds());
        stallWarn = new VoiceAlert("warn_stall", VoiceAlertType.WARN_STALL.getCooldownSeconds());

        parser.Blkx b = c.getBlkx();
        iasWarningLine = 0;
        machWarningLine = 0;
        if (b != null && b.valid) {
            iasWarningLine = b.vne * 0.95f;
            machWarningLine = b.vneMach * 0.95f;
        }
        if (iasWarningLine == 0) iasWarningLine = Float.MAX_VALUE;
        if (machWarningLine == 0) machWarningLine = Float.MAX_VALUE;
    }

    /**
     * 初始化结构告警（起落架、襟翼、过载、减速板）
     */
    private void initStructureWarnings(Controller c) {
        gearWarn = new VoiceAlert("warn_gear", VoiceAlertType.WARN_GEAR.getCooldownSeconds());
        flapWarn = new VoiceAlert("warn_flap", VoiceAlertType.WARN_FLAP.getCooldownSeconds());
        nyWarn = new VoiceAlert("warn_loadfactor", VoiceAlertType.WARN_LOADFACTOR.getCooldownSeconds());
        brakeWarn = new VoiceAlert("warn_brake", VoiceAlertType.WARN_BRAKE.getCooldownSeconds());

        parser.Blkx b = c.getBlkx();

        // 起落架速度限制
        gearWarningLine = 0;
        if (b != null && b.valid) {
            gearWarningLine = b.GearDestructionIndSpeed;
        }
        if (gearWarningLine == 0) gearWarningLine = 450;

        // 过载限制
        this.blkx = b;
        this.nofuelweight = (b != null && b.valid) ? b.nofuelweight : 0;
        nyWarningLine0 = 0;
        nyWarningLine1 = 0;
        if (b != null && b.valid) {
            nyWarningLine0 = b.maxAllowGload[0];
            nyWarningLine1 = b.maxAllowGload[1];
        }
        if (nyWarningLine0 == 0) nyWarningLine0 = -4;
        if (nyWarningLine1 == 0) nyWarningLine1 = 10;
    }

    /**
     * 初始化引擎告警
     */
    private void initEngineWarnings() {
        engWarn = new VoiceAlert("warn_engineoverheat", VoiceAlertType.WARN_ENGINEOVERHEAT.getCooldownSeconds());
        engFail = new VoiceAlert("fail_engine", VoiceAlertType.FAIL_ENGINE.getCooldownSeconds());
        // engFailInvert 使用不同的冷却时间（5秒），用于倒飞断油检测
        engFailInvert = new VoiceAlert("fail_engine", 5);
        rpmLowWarn = new VoiceAlert("warn_lowrpm", VoiceAlertType.WARN_LOWRPM.getCooldownSeconds());
        rpmHighWarn = new VoiceAlert("warn_highrpm", VoiceAlertType.WARN_HIGHRPM.getCooldownSeconds());
    }

    /**
     * 初始化燃油告警
     */
    private void initFuelWarnings() {
        lowfuelWarningLine = 10;
        fuelWarn = new VoiceAlert("warn_lowfuel", VoiceAlertType.WARN_LOWFUEL.getCooldownSeconds());
        fuelPrsWarn = new VoiceAlert("warn_lowpressure", VoiceAlertType.WARN_LOWPRESSURE.getCooldownSeconds());
        oofWarn = new VoiceAlert("fail_nofuel", VoiceAlertType.FAIL_NOFUEL.getCooldownSeconds());
    }

    /**
     * 初始化高度告警
     */
    private void initAltitudeWarnings() {
        heightWarn = new VoiceAlert("warn_altitude", VoiceAlertType.WARN_ALTITUDE.getCooldownSeconds());
        terrainWarn = new VoiceAlert("warn_terrain", VoiceAlertType.WARN_TERRAIN.getCooldownSeconds());
        varioWarn = new VoiceAlert("warn_highvario", VoiceAlertType.WARN_HIGHVARIO.getCooldownSeconds());
    }

    /**
     * 初始化舵效告警
     */
    private void initControlEffectivenessWarnings(Controller c) {
        rudderEff = new VoiceAlert("rudderEff", VoiceAlertType.RUDDER_EFF.getCooldownSeconds());
        elevatorEff = new VoiceAlert("elevatorEff", VoiceAlertType.ELEVATOR_EFF.getCooldownSeconds());
        aileronEff = new VoiceAlert("aileronEff", VoiceAlertType.AILERON_EFF.getCooldownSeconds());

        rudderEffIAS = 65535;
        elevatorEffIAS = 65535;
        aileronEffIAS = 65535;

        parser.Blkx b = c.getBlkx();
        if (b != null && b.valid) {
            rudderEffIAS = (int) b.rudderEff;
            elevatorEffIAS = (int) b.elavEff;
            aileronEffIAS = (int) b.aileronEff;
        }
    }

    /**
     * 初始化增压器档位告警
     */
    private void initCompressorWarning() {
        // 增压器档位告警由状态变化驱动，无冷却时间
        compressorStageWarn = new VoiceAlert("warn_compressor", VoiceAlertType.WARN_COMPRESSOR.getCooldownSeconds());

        // 订阅 FlightDataBus 获取增压器档位不匹配事件
        flightDataListener = new FlightDataListener() {
            @Override
            public void onFlightData(FlightDataEvent event) {
                EventPayload payload = event.getPayload();
                currentMismatch = payload.compressorStageMismatch;
            }
        };
        FlightDataBus.getInstance().register(flightDataListener);
    }

    /**
     * Cleans up resources when VoiceWarning is disposed.
     */
    public void dispose() {
        doit = false;
        if (flightDataListener != null) {
            FlightDataBus.getInstance().unregister(flightDataListener);
            flightDataListener = null;
        }
    }

    // ==================== 主循环 ====================

    @Override
    public void run() {
        // 启动延迟，使用统一异常处理
        prog.util.ExceptionHelper.sleepQuietly(1000);

        while (doit) {
            try {
                Thread.sleep(sleepTime);
            } catch (InterruptedException e) {
                doit = false;
                break;
            }

            boolean fatal = false;
            long t = xS.currentTimeMs;

            // 更新动态参数（可变翼等）
            updateDynamicParameters();

            // 执行所有告警检测，收集 fatal 状态
            fatal |= checkAoAWarning(t);
            fatal |= checkSpeedWarning(t);
            fatal |= checkGearWarning(t);
            checkBrakeWarning(t);
            fatal |= checkFlapWarning(t);
            checkVarioWarning(t);
            checkEngineOverheatWarning(t);
            checkFuelWarning(t);
            fatal |= checkAltitudeWarning(t);
            checkFuelPressureWarning(t);
            checkInvertedFlightWarning(t);
            checkRPMWarning(t);
            checkStallWarning(t);
            fatal |= checkLoadFactorWarning(t);
            checkControlEffectivenessWarning(t);
            checkCompressorWarning(t);

            // 更新起落架/襟翼完好性状态
            updateStructureHealth();

            xS.fatalWarn = fatal;
        }
    }

    // ==================== 动态参数更新 ====================

    /**
     * 更新可变翼相关的动态参数
     * 原位置：run() 第 439-455 行
     */
    private void updateDynamicParameters() {
        double vwing = 0;
        int flaps = st.flaps > 0 ? st.flaps : 0;
        parser.Blkx b = xc.getBlkx();
        if (b != null && b.valid) {
            if (b.isVWing) {
                vwing = indic.wsweep_indicator;
            }
            aoaWarningLine = b.getAoAHighVWing(vwing, flaps);
            iasWarningLine = b.getVNEVWing(vwing) * 0.95f;
            machWarningLine = b.getMNEVWing(vwing) * 0.95f;
        }
    }

    // ==================== 告警检测方法 ====================

    /**
     * 攻角告警检测
     * 原位置：run() 第 458-468 行
     *
     * @return true 如果是致命告警
     */
    private boolean checkAoAWarning(long t) {
        if (!xS.playerLive || st.IAS <= 80) {
            return false;
        }

        if (st.AoA > aoaWarningLine - 1) {
            // 临界攻角告警
            aoaCrit.playOnce(t);
            // 同时标记 aoaHigh 为已播放，防止同时触发两个告警
            aoaHigh.lastTimePlay = t;
            aoaHigh.isAct = true;
            return true;
        } else if (st.AoA > (aoaWarningLine * 0.75f)) {
            // 高攻角预警
            aoaHigh.playOnce(t);
        }
        return false;
    }

    /**
     * 速度告警检测（IAS 和 Mach）
     * 原位置：run() 第 472-480 行
     *
     * @return true 如果是致命告警
     */
    private boolean checkSpeedWarning(long t) {
        boolean fatal = false;

        if (st.IAS >= iasWarningLine) {
            iasWarn.playOnce(t);
            fatal = true;
        }

        if (st.M >= machWarningLine) {
            machWarn.playOnce(t);
            fatal = true;
        }

        return fatal;
    }

    /**
     * 起落架告警检测
     * 原位置：run() 第 484-487 行
     *
     * @return true 如果是致命告警
     */
    private boolean checkGearWarning(long t) {
        if (isGearAlive && (st.gear > 0) && st.IAS >= gearWarningLine) {
            gearWarn.playOnce(t);
            return true;
        }
        return false;
    }

    /**
     * 减速板告警检测
     * 原位置：run() 第 490-492 行
     */
    private void checkBrakeWarning(long t) {
        // 起落架未放下但减速板已展开
        if (st.gear < 100 && st.airbrake >= 90) {
            brakeWarn.playOnce(t);
        }
    }

    /**
     * 襟翼告警检测
     * 原位置：run() 第 495-505 行
     *
     * @return true 如果是致命告警
     */
    private boolean checkFlapWarning(long t) {
        // 条件1: 不是正在下襟翼的状态
        boolean cond1 = isFlapAlive && !xS.isDowningFlap &&
                        (xS.flapAllowAngle - st.flaps < 2) && (st.flaps != 0);
        // 条件2: 正在下襟翼的状态
        boolean cond2 = isFlapAlive && xS.isDowningFlap &&
                        (xS.flapAllowAngle - st.flaps < 8);

        if (cond1 || cond2) {
            flapWarn.playOnce(t);
            return true;
        }
        return false;
    }

    /**
     * 下降率告警检测
     * 原位置：run() 第 508-512 行
     */
    private void checkVarioWarning(long t) {
        // 起落架放下且下降率过高
        if (isGearAlive && st.gear >= 50 && st.Vy <= -8) {
            varioWarn.playOnce(t);
        }
    }

    /**
     * 引擎过热告警检测
     * 原位置：run() 第 547-549 行
     */
    private void checkEngineOverheatWarning(long t) {
        // curLoadMinWorkTime < 300 秒表示引擎即将过热
        if (xS.curLoadMinWorkTime < 300 * 1000) {
            engWarn.playOnce(t);
        }
    }

    /**
     * 燃油告警检测（低油量和无油）
     * 原位置：run() 第 551-563 行
     */
    private void checkFuelWarning(long t) {
        // 无油告警
        if (xS.totalFuel == 0) {
            if (oofCheck++ > 16) {
                oofCheck = 0;
                oofWarn.playOnce(t);
            }
        }

        // 低油量告警
        if (xS.fuelPercent <= lowfuelWarningLine) {
            // 持续 16 tick 以上，解决退出游戏时的误告警
            if (fuelCheck++ > 16) {
                fuelCheck = 0;
                fuelWarn.playOnce(t);
            }
        }
    }

    /**
     * 高度告警检测（含地形告警）
     * 原位置：run() 第 568-581 行
     *
     * @return true 如果是致命告警
     */
    private boolean checkAltitudeWarning(long t) {
        // 起落架未放下且玩家存活
        if (st.gear > 0 || !xS.playerLive) {
            return false;
        }

        // 下降率等于高度的 10 分之一会触发警告
        if (st.Vy < -st.heightm / 10.0f) {
            heightWarn.playOnce(t);
            return true;
        } else {
            // 触发高度警告优先，其次是触发地形警告
            if (xS.radioAlt > 0) {
                if (xS.dRadioAlt < -xS.radioAlt / 10.0f) {
                    terrainWarn.playOnce(t);
                    return true;
                }
            }
        }
        return false;
    }

    /**
     * 燃油压力告警检测
     * 原位置：run() 第 583-600 行
     */
    private void checkFuelPressureWarning(long t) {
        // 油压过低检测
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

        // 引擎损坏后油压为0的告警
        if (engDamage && indic.fuel_pressure == 0) {
            fuelPCheck += sleepTime;
            if (fuelPCheck >= 1000) {
                engFail.playOnce(t);
                engDamage = false;
            }
        }
    }

    /**
     * 倒飞断油告警检测
     * 原位置：run() 第 604-609 行
     */
    private void checkInvertedFlightWarning(long t) {
        // 倒飞时油门大但推力低
        if (st.Ny < 0 && st.throttle > 50) {
            if (st.thrust[0] < 50) {
                engFailInvert.playOnce(t);
            }
        }
    }

    /**
     * 转速告警检测（低转速和高转速）
     * 原位置：run() 第 612-628 行
     */
    private void checkRPMWarning(long t) {
        // 倒飞时不检测转速
        if (st.Ny < 0 && st.throttle > 50 && st.thrust[0] < 50) {
            return;
        }

        // 定距桨特殊处理：不是喷气但桨距无效
        if (!(!xS.isEngJet() && st.RPMthrottle < 0)) {
            if (st.throttle - 30 > st.RPM * 100.0f / xS.maximumThrRPM) {
                // 转速低
                rpmLowWarn.playOnce(t);
            }
        }

        // 高转速告警
        if (xS.getMaximumRPM && xS.maximumThrRPM > 0 && st.RPM * 100.0f / xS.maximumThrRPM >= 105) {
            rpmHighWarn.playOnce(t);
        }
    }

    /**
     * 失速告警检测
     * 原位置：run() 第 632-634 行
     */
    private void checkStallWarning(long t) {
        // 没放下起落架、有下降率、速度低于失速速度
        if (xS.playerLive && st.gear == 0 && st.Vy != 0 &&
            xS.getStallSpeed() != 0 && st.IAS <= xS.getStallSpeed()) {
            stallWarn.playOnce(t);
        }
    }

    /**
     * 过载告警检测
     * 原位置：run() 第 637-651 行
     *
     * @return true 如果是致命告警
     */
    private boolean checkLoadFactorWarning(long t) {
        // 使用动态阈值
        double currentNyMin = nyWarningLine0;
        double currentNyMax = nyWarningLine1;

        if (blkx != null && blkx.rawWingCritOverload != null && nofuelweight > 0) {
            double currentWeight = nofuelweight + st.mfuel;
            double[] dynamicLimits = blkx.getMaxAllowGloadForWeight(currentWeight);
            currentNyMin = dynamicLimits[0];
            currentNyMax = dynamicLimits[1];
        }

        if (xS.playerLive && (st.Ny > currentNyMax || st.Ny < currentNyMin)) {
            nyWarn.playOnce(t);
            return true;
        }
        return false;
    }

    /**
     * 舵效告警检测（副翼和方向舵）
     * 原位置：run() 第 653-669 行
     */
    private void checkControlEffectivenessWarning(long t) {
        // 副翼舵效
        if (st.IAS >= aileronEffIAS) {
            if (!aileronEffCheck) {
                aileronEff.playOnce(t);
            }
            aileronEffCheck = true;
        } else {
            aileronEffCheck = false;
        }

        // 方向舵舵效
        if (st.IAS >= rudderEffIAS) {
            if (!rudderEffCheck) {
                rudderEff.playOnce(t);
            }
            rudderEffCheck = true;
        } else {
            rudderEffCheck = false;
        }
    }

    /**
     * 增压器档位不匹配告警检测
     * 原位置：run() 第 671-690 行
     */
    private void checkCompressorWarning(long t) {
        boolean isMismatch = currentMismatch;  // Read volatile once

        // Detect state change (false→true or true→false)
        if (isMismatch != lastMismatch) {
            if (isMismatch) {
                // false → true: 不一致了，启动 3 秒定时器
                pendingCompressorWarnTime = t + COMPRESSOR_WARN_DELAY;
            } else {
                // true → false: 一致了，取消定时器
                pendingCompressorWarnTime = 0;
            }
            lastMismatch = isMismatch;
        }

        // Check if it's time to play the warning
        if (pendingCompressorWarnTime > 0 && t >= pendingCompressorWarnTime) {
            compressorStageWarn.playOnce(t);
            // 每 3 秒重复告警直到不匹配状态解除
            pendingCompressorWarnTime = t + COMPRESSOR_WARN_DELAY;
        }
    }

    // ==================== 结构完好性更新 ====================

    /**
     * 更新起落架和襟翼的完好性状态
     * 原位置：run() 第 514-543 行
     */
    private void updateStructureHealth() {
        // 起落架完好性判断
        if (isGearAlive && st.IAS > gearWarningLine) {
            // 超速持续 10 秒后标记为损坏
            gearCheck += sleepTime;
            if (gearCheck >= GEAR_DAMAGE_THRESHOLD_MS) {
                gearCheck = 0;
                isGearAlive = false;
            }
        } else {
            gearCheck = 0;
        }

        // 襟翼完好性判断
        if (isFlapAlive && st.IAS > xS.flapAllowSpeed) {
            // 超速持续 15 秒后标记为损坏
            flapCheck += sleepTime;
            if (flapCheck >= FLAP_DAMAGE_THRESHOLD_MS) {
                flapCheck = 0;
                isFlapAlive = false;
            }
        } else {
            flapCheck = 0;
        }

        // 收起后恢复
        if (!isGearAlive && st.gear == 0) {
            isGearAlive = true;
        }
        if (!isFlapAlive && st.flaps == 0) {
            isFlapAlive = true;
        }
    }
}
