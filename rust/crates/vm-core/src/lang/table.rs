//! `lang/cur.properties` 的静态快照 — 键值 = Java `java.util.Properties` 加载后的实际值。
//!
//! 来源文件: 项目根 `lang/cur.properties` (UTF-8)。
//! 生成方法 (Java 8 oracle 实测, 非手工推算):
//! ```text
//! new Properties().load(new InputStreamReader(new FileInputStream("lang/cur.properties"), "utf-8"))
//! ```
//! 逐键 base64 dump 后转义为 Rust 字面量。已内化的 Properties 语义 (oracle 验证):
//! - 文件中的 \n 转义已展开为真实控制字符; \\n 为字面 反斜杠+n (mResetConfirmContent)
//! - 分隔符 `=` 后的前导 ASCII 空白被跳过 (noblkx 的前导空格不进值); 尾部空白保留
//! - 值全为 ASCII 空白的键 (mP4attitudeIndicatorPanelBlank) 加载后为空串
//! - 全角空格 U+3000 不是 Properties 空白, 原样保留 (表中写作 \u{3000})
//!
//! PORT: Java 迭代顺序为 Hashtable 无序; 此处按键排序存储 (getValue 语义与顺序无关)。
//! 漂移守护: `tests::table_matches_cur_properties_source` 与源文件逐键对拍,
//! 源 properties 改动后未再生本表即测试失败。

/// 键值表 (按键排序)。共 362 条, 与源文件一一对应, 无重复键。
pub static LANGUAGE_PROPERTIES: &[(&str, &str)] = &[
    ("Systemerror", "该程序在VISTA/WIN 7以下的操作系统上运行会造成游戏丢帧或卡顿现象,建议您更新系统"),
    ("about", "About"),
    ("aboutcontent", "1.本程序对游戏程序及进程无任何修改,所有信息通过离线拆包数据和利用HTTP/GET请求读取WT官方提供的8111端口获得.\n\r"),
    ("aboutcontentsub1", "2.本程序只是兴趣使然的创作.程序代码遵循GPL-V3协议开源,访问https://github.com/matrixsukhoi/voidmei可获得最新源码和发布版.\n\r"),
    ("aboutcontentsub2", "3.本程序设计目标是帮助WT玩家更好理解飞行与空战.请注意:拆包违反EULA,勿在官方人员面前跳脸.\n\r"),
    ("appName", "VoidMei"),
    ("appTooltips", "WT8111端口信息分析、显示、记录工具"),
    ("bAllowLoadFactor", "允许过载(满/半油): [%.1f, %.1f], [%.1f, %.1f]\n"),
    ("bAoACrit", "临界攻角: [%.1f, %.1f]\n"),
    ("bAoACritCl", "临界攻角升力系数: [%.2f, %.2f]\n"),
    ("bAverageHeatRecovery", "平均耐热条恢复速率: %.1f\n"),
    ("bCdMin", "零升阻力系数: %.3f\n"),
    ("bCl0", "零攻角升力: %.3f\n"),
    ("bCritSpeed", "临界速度(km/h): [%.0f, %.0f]\n"),
    ("bDrag", "主阻力面积因数及加速度系数: %.2f / %.3f\n诱导阻力因数及加速度系数: %.3f / %.0f\n散热/油冷器阻力系数: %.3f / %.3f\n"),
    ("bEffSpeedAndPowerLoss", "三舵有效速度(km/h): [ 升降%.0f, 副翼%.0f, 方向%.0f ]\n三舵锁舵因数: [ 升降%.1f, 副翼%.1f, 方向%.1f ]\n"),
    ("bFlapRestrict", "襟翼限速(km/h)%d: %.0f%% / %.0f\n"),
    ("bFmParts", "------fm器件 %s------\n"),
    ("bFmVersion", "FM文件: %s - %s"),
    ("bInertia", "三轴转动惯量: [ P: %.0f, R: %.0f, Y: %.0f ]\n"),
    ("bLift", "主升力面积: %.1f机翼, %.1f机身\n主升力面积因数载荷: %.2f / %.2f(襟)\n翼展效率: %.2f 展弦比: %.1f 后掠角: %.1f\n"),
    ("bMaxLiftLoad350", "千米最大升力过载: %.1f / %.1f(襟) @ 350IAS\n"),
    ("bNitro", "加力(kg)/时限(分钟): %.1f / %.1f\n"),
    ("bWeight", "空重(kg): %.1f\n最大燃油重量(kg): %.1f\n"),
    ("cOpenpad", "'您已加入游戏，面板将在' s '秒内打开'"),
    ("cPlsopen", ",请用EXCEL打开"),
    ("cSavelog", "端口信息保存至"),
    ("cStartlog", "开始记录端口信息"),
    ("close", "Close"),
    ("dFTitle1", "时间-高度曲线"),
    ("dFTitle1X", "时间"),
    ("dFTitle1Y", "高度"),
    ("dFTitle2", "功率-高度包线"),
    ("dFTitle2X", "功率"),
    ("dFTitle2Y", "高度"),
    ("dFTitle3", "推力-高度包线"),
    ("dFTitle3X", "推力"),
    ("dFTitle3Y", "高度"),
    ("dFTitle4", "实功率-高度包线"),
    ("dFTitle4X", "实功率"),
    ("dFTitle4Y", "高度"),
    ("dFTitle5", "SEP-高度包线"),
    ("dFTitle5X", "SEP"),
    ("dFTitle5Y", "高度"),
    ("dFTitleHZ", "性能曲线生成"),
    ("dFnext", "下一个"),
    ("dFprev", "上一个"),
    ("defaultFontName", "Sarasa Mono SC"),
    ("defaultFontSize", "12"),
    ("eATM", "进气压"),
    ("eCompressor", "增"),
    ("eEff", "桨效率"),
    ("eEffPower", "实功率"),
    ("eEngRes", "响应速"),
    ("eFuel", "燃油量"),
    ("eFuelP", "燃加力"),
    ("eFuelPer", "油"),
    ("eFuelPrs", "油\u{3000}压"),
    ("eFueltime", "燃油时"),
    ("eMagneto", ""),
    ("eMixture", "混"),
    ("eOil", "油\u{3000}温"),
    ("eOverheat", "耐热时"),
    ("ePitchDeg", "桨距角"),
    ("ePower", "功\u{3000}率"),
    ("ePowerPercent", "动"),
    ("eProppitch", "桨"),
    ("eRPM", "转\u{3000}速"),
    ("eRadiator", "散"),
    ("eTemp", "温\u{3000}度"),
    ("eThrottle", "节"),
    ("eThurst", "推\u{3000}力"),
    ("eThurstP", "推"),
    ("eTitle", "发动机面板"),
    ("eType", "机\u{3000}型"),
    ("eWep", "加力量"),
    ("eWeptime", "加力时"),
    ("fA1", "到达 "),
    ("fA2", "米，用时 "),
    ("fA3", "秒，平均爬升率 "),
    ("fA4", "米/秒，记录完成"),
    ("fA_roll1", "速度  "),
    ("fA_roll2", "km/h下的最大滚转率: "),
    ("fA_roll3", "度/秒,记录完成"),
    ("fA_turn1", "速度  "),
    ("fA_turn2", "km/h下的最大法向过载: "),
    ("fA_turn3", "G, 此时SEP为: "),
    ("fA_turn4", "m/s, 记录完成"),
    ("fAcc", "加速度"),
    ("fAlt", "高\u{3000}度"),
    ("fAoA", "攻\u{3000}角"),
    ("fAoS", "侧滑角"),
    ("fCompass", "航\u{3000}向"),
    ("fGL", "过\u{3000}载"),
    ("fIAS", "表\u{3000}速"),
    ("fMach", "马赫数"),
    ("fRa", "测距高"),
    ("fSEP", "ＳＥＰ"),
    ("fTAS", "真空速"),
    ("fTR", "转半径"),
    ("fTRr", "转弯率"),
    ("fTitle", "飞行信息面板"),
    ("fVario", "爬升率"),
    ("fWs", "可变翼"),
    ("fWx", "滚转率"),
    ("failaddtoTray", "托盘加入失败"),
    ("fmCorruptToast", "FM 数据文件解析失败 (文件损坏)\n请重新解包更新 FM 数据"),
    ("fmMissingToast", "没有对应的 FM 数据文件\n可能是新出的飞机, FM 数据尚未更新"),
    ("gBrake", "减速板"),
    ("gFlaps", "襟\u{3000}翼"),
    ("gGear", "起落架"),
    ("gGearDown", "收起落"),
    ("gTitle", "飞行状态"),
    ("httpHeader", "\n"),
    ("httpIp", "127.0.0.1"),
    ("l1", "时间/s,"),
    ("l10", "ＳＥＰ*/m/s,"),
    ("l11", "过\u{3000}载/G,"),
    ("l12", "滚转率/deg/s,"),
    ("l13", "功\u{3000}率/hp,"),
    ("l14", "桨效率/%,"),
    ("l15", "实功率*/hp,"),
    ("l16", "转\u{3000}速/rpm,"),
    ("l17", "推\u{3000}力/kg,"),
    ("l18", "加速度*/m/s^2,"),
    ("l19", "桨\u{3000}距/%,"),
    ("l2", "节流阀/%,"),
    ("l20", "桨距角/deg,"),
    ("l21", "散热器/%,"),
    ("l22", "混合比/%,"),
    ("l23", "增压器/档,"),
    ("l24", "磁电机/档,"),
    ("l25", "进气压/ata,"),
    ("l26", "襟\u{3000}翼/%,"),
    ("l27", "升降舵/%,"),
    ("l28", "滚转舵/%,"),
    ("l29", "方向舵/%,"),
    ("l3", "表\u{3000}速/kph,"),
    ("l30", "攻\u{3000}角/deg,"),
    ("l31", "侧滑角/deg,"),
    ("l4", "真空速/kph,"),
    ("l5", "马赫数/Ma,"),
    ("l6", "高\u{3000}度/m,"),
    ("l7", "温\u{3000}度/℃,"),
    ("l8", "油\u{3000}温/℃,"),
    ("l9", "爬升率/m/s,"),
    ("lfailCreate", "记录文件创建失败"),
    ("lfailWrite", "记录文件写入失败"),
    ("mAdvancedOption", "\u{3000}\u{3000}高级设置"),
    ("mBasicSettings", "基本设定"),
    ("mCancel", "退\u{3000}出"),
    ("mClosePreview", "关闭预览"),
    ("mConfigErrorContent", "用户配置文件解析失败，将临时使用默认配置。\n请检查 ui_layout.user.cfg 文件是否损坏。"),
    ("mConfigErrorTitle", "配置错误"),
    ("mConfigMergedTitle", "配置已更新"),
    ("mControlInfo", "\u{3000}\u{3000}飞行控制"),
    ("mCrosshair", "\u{3000}自定义HUD"),
    ("mDetailedMode", "详细模式"),
    ("mDisplayOverlay", "显示Overlay: "),
    ("mDisplayPreview", "显示预览"),
    ("mEngineInfo", "\u{3000}发动机状态"),
    ("mFactoryResetConfirmContent", "确定要恢复出厂设置吗？\n所有自定义配置将被清除，当前配置将被备份。"),
    ("mFactoryResetConfirmTitle", "确认恢复出厂设置"),
    ("mFactoryResetFailContent", "恢复出厂设置失败，请检查模板文件是否存在。"),
    ("mFactoryResetFailTitle", "恢复失败"),
    ("mFactoryResetSuccessContent", "配置已恢复为出厂设置，请重启程序以应用所有更改。"),
    ("mFactoryResetSuccessTitle", "恢复成功"),
    ("mFlightInfo", "\u{3000}\u{3000}飞行状态"),
    ("mHotkeyToggle", "按键切换: "),
    ("mImportButtonImport", "导入配置"),
    ("mImportConfigTitle", "选择配置文件"),
    ("mImportConfirmContent", "确定要导入此配置文件吗？\n当前配置将被备份到 ui_layout.user.cfg.bak"),
    ("mImportConfirmTitle", "确认导入"),
    ("mImportDropZoneFormat", "支持的格式: *.cfg, *.bak"),
    ("mImportDropZoneInvalid", "不支持的文件格式，请选择 .cfg 或 .bak 文件"),
    ("mImportDropZoneRelease", "松开以导入"),
    ("mImportDropZoneSubtitle", "或点击选择文件"),
    ("mImportDropZoneTitle", "拖放配置文件到此处"),
    ("mImportFailContent", "配置文件导入失败，请检查文件格式是否正确。"),
    ("mImportFailTitle", "导入失败"),
    ("mImportFileNone", "未选择文件"),
    ("mImportFileSelected", "已选择: %s"),
    ("mImportSuccessContent", "配置文件已成功导入，请重启程序以应用所有更改。"),
    ("mImportSuccessTitle", "导入成功"),
    ("mLoggingAndAnalysis", "\u{3000}记录与分析"),
    ("mMergeAddedItems", "新增配置项:"),
    ("mMergeAddedPanels", "新增面板:"),
    ("mMergeUpdatedItems", "更新配置项:"),
    ("mMovePanel", "请拖动面板进行位置调整"),
    ("mP1AAEnable", "\u{3000}\u{3000}图形抗锯齿"),
    ("mP1AAEnableBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP1GlobalNumberFont", "\u{3000}\u{3000}全局数字字体\u{3000}"),
    ("mP1GlobalNumberFontBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP1Interval", "数据帧间隔(毫秒)"),
    ("mP1LabelColor", "\u{3000}\u{3000}\u{3000}\u{3000}标签色\u{3000}\u{3000}"),
    ("mP1LabelColorBlank", "\u{3000}"),
    ("mP1NumColor", "\u{3000}\u{3000}\u{3000}\u{3000}数字色\u{3000}\u{3000}"),
    ("mP1NumColorBlank", "\u{3000}"),
    ("mP1ShadeColor", "\u{3000}\u{3000}\u{3000}\u{3000}描边色\u{3000}\u{3000}"),
    ("mP1ShadeColorBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP1TempNotification", "\u{3000}\u{3000}\u{3000}\u{3000}温度通知\u{3000}"),
    ("mP1TempNotificationBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP1UnitColor", "\u{3000}\u{3000}\u{3000}\u{3000}单位色\u{3000}\u{3000}"),
    ("mP1UnitColorBlank", "\u{3000}"),
    ("mP1VoiceWarning", "\u{3000}\u{3000}\u{3000}\u{3000}语音告警\u{3000}"),
    ("mP1VoiceWarningBlank", ""),
    ("mP1WarnColor", "\u{3000}\u{3000}\u{3000}\u{3000}告警色\u{3000}\u{3000}"),
    ("mP1WarnColorBlank", "\u{3000}"),
    ("mP1drawFontShape", "\u{3000}\u{3000}简化字体描边\u{3000}"),
    ("mP1drawFontShapeBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP1statusBar", "\u{3000}\u{3000}\u{3000}等待状态条\u{3000}"),
    ("mP1statusBarBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP1voiceVolume", "\u{3000}\u{3000}语音告警音量"),
    ("mP1voiceVolumeBlank", ""),
    ("mP2EngineBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP2EngineGlassEdge", "\u{3000}玻璃边框"),
    ("mP2EngineGlassEdgeBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP2EnginePanel", "发动机面板"),
    ("mP2EnginePanelBlank", "\u{3000}\u{3000}"),
    ("mP2FontAdjust", "字体大小调整\u{3000}\u{3000}"),
    ("mP2PanelFont", "面板显示字体"),
    ("mP2eiEffEta", "显示桨效率"),
    ("mP2eiEffEtaBlank", "\u{3000}\u{3000}"),
    ("mP2eiEffHp", "显示实功率"),
    ("mP2eiEffHpBlank", "\u{3000}\u{3000}"),
    ("mP2eiEngResponse", "显示响应速"),
    ("mP2eiEngResponseBlank", "\u{3000}\u{3000}"),
    ("mP2eiFuelKg", "显示燃油量"),
    ("mP2eiFuelKgBlank", "\u{3000}\u{3000}"),
    ("mP2eiFuelTime", "显示燃油时"),
    ("mP2eiFuelTimeBlank", "\u{3000}\u{3000}"),
    ("mP2eiHeatTolerance", "显示耐热时"),
    ("mP2eiHeatToleranceBlank", "\u{3000}\u{3000}"),
    ("mP2eiHorsePower", "显示功\u{3000}率"),
    ("mP2eiHorsePowerBlank", "\u{3000}\u{3000}"),
    ("mP2eiOilTemp", "显示油\u{3000}温"),
    ("mP2eiOilTempBlank", "\u{3000}\u{3000}"),
    ("mP2eiPowerPercent", "显示动力量"),
    ("mP2eiPowerPercentBlank", "\u{3000}\u{3000}"),
    ("mP2eiPressure", "显示进气压"),
    ("mP2eiPressureBlank", "\u{3000}\u{3000}"),
    ("mP2eiPropPitch", "显示桨距角"),
    ("mP2eiPropPitchBlank", "\u{3000}\u{3000}"),
    ("mP2eiRPM", "显示转\u{3000}速"),
    ("mP2eiRPMBlank", "\u{3000}\u{3000}"),
    ("mP2eiTemp", "显示温\u{3000}度"),
    ("mP2eiTempBlank", "\u{3000}\u{3000}"),
    ("mP2eiThrust", "显示推\u{3000}力"),
    ("mP2eiThrustBlank", "\u{3000}\u{3000}"),
    ("mP2eiWepKg", "显示加力量"),
    ("mP2eiWepKgBlank", "\u{3000}\u{3000}"),
    ("mP2eiWepTime", "显示加力时"),
    ("mP2eiWepTimeBlank", "\u{3000}\u{3000}"),
    ("mP3ChooseTexture", "\u{3000}选择准星贴图\u{3000}"),
    ("mP3ChooseTextureBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP3Crosshair", "\u{3000} 自定义HUD"),
    ("mP3CrosshairBlank", "\u{3000}\u{3000}"),
    ("mP3CrosshairDisplay", "\u{3000}显示准星\u{3000}"),
    ("mP3CrosshairDisplayBlank", "\u{3000}\u{3000}"),
    ("mP3CrosshairSize", "\u{3000}\u{3000}自定义HUD大小"),
    ("mP3CrosshairTexture", "\u{3000}\u{3000}准星贴图"),
    ("mP3CrosshairTextureBlank", ""),
    ("mP3FlapAngleBar", "\u{3000} 显示襟翼指示条"),
    ("mP3FlapAngleBarBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP3MonoBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP3MonoFont", "\u{3000}\u{3000} HUD等距字体\u{3000}"),
    ("mP3Text", "最小HUD"),
    ("mP3TextBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP4ColumnAdjust", "面板每行个数\u{3000}                                     \u{3000}"),
    ("mP4FMPanel", "\u{3000}拆包信息\u{3000}"),
    ("mP4FMPanelBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}                    "),
    ("mP4FlightInfoBlank", ""),
    ("mP4FlightInfoGlassEdge", "\u{3000}玻璃边框\u{3000}"),
    ("mP4FlightInfoGlassEdgeBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP4FlightInfoPanel", "飞行信息面板"),
    ("mP4FontAdjust", "字体大小调整\u{3000}                                     \u{3000}"),
    ("mP4PanelFont", "面板显示字体                                            "),
    ("mP4attitudeIndicatorPanel", "地平仪面板\u{3000}"),
    ("mP4attitudeIndicatorPanelBlank", ""),
    ("mP4fiAcc", "显示加速度"),
    ("mP4fiAccBlank", "\u{3000}\u{3000}"),
    ("mP4fiAoA", "显示攻\u{3000}角"),
    ("mP4fiAoABlank", "\u{3000}\u{3000}"),
    ("mP4fiAoS", "显示侧滑角"),
    ("mP4fiAoSBlank", "\u{3000}\u{3000}"),
    ("mP4fiCompass", "显示航\u{3000}向"),
    ("mP4fiCompassBlank", "\u{3000}\u{3000}"),
    ("mP4fiHeight", "显示高\u{3000}度"),
    ("mP4fiHeightBlank", "\u{3000}\u{3000}"),
    ("mP4fiIAS", "显示示空速"),
    ("mP4fiIASBlank", "\u{3000}\u{3000}"),
    ("mP4fiMach", "显示马赫数"),
    ("mP4fiMachBlank", "\u{3000}\u{3000}"),
    ("mP4fiNy", "显示过\u{3000}载"),
    ("mP4fiNyBlank", "\u{3000}\u{3000}"),
    ("mP4fiRadioAlt", "显示测距高"),
    ("mP4fiRadioAltBlank", "\u{3000}\u{3000}"),
    ("mP4fiSEP", "显示ＳＥＰ"),
    ("mP4fiSEPBlank", "\u{3000}\u{3000}"),
    ("mP4fiTAS", "显示真空速"),
    ("mP4fiTASBlank", "\u{3000}\u{3000}"),
    ("mP4fiTurn", "显示转弯率"),
    ("mP4fiTurnBlank", "\u{3000}\u{3000}"),
    ("mP4fiTurnRadius", "显示转半径"),
    ("mP4fiTurnRadiusBlank", "\u{3000}\u{3000}"),
    ("mP4fiVario", "显示爬升率"),
    ("mP4fiVarioBlank", "\u{3000}\u{3000}"),
    ("mP4fiWingSweep", "显示可变翼"),
    ("mP4fiWingSweepBlank", "\u{3000}\u{3000}"),
    ("mP4fiWx", "显示滚转率"),
    ("mP4fiWxBlank", "\u{3000}\u{3000}"),
    ("mP5FMChoose", "比较飞行模型"),
    ("mP5FMChooseBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP5FMDisplayKey", "FM显示触发键"),
    ("mP5FMDisplayKeyTip", "按下新按钮..."),
    ("mP5FMPrintEnable", "显示FM文件详细数据"),
    ("mP5FMPrintEnableBlank", "\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP5Information", "通知记录信息"),
    ("mP5InformationBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP5LoggingAndCharting", "飞行记录和图表生成"),
    ("mP5LoggingAndChartingBlank", "\u{3000}\u{3000}"),
    ("mP6AxisEdge", "舵面值边框\u{3000} "),
    ("mP6AxisEdgeBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP6AxisPanel", "\u{3000}\u{3000}\u{3000}舵面值面板"),
    ("mP6AxisPanelBlank", "\u{3000}\u{3000}"),
    ("mP6GearAndFlaps", "起落架与襟翼面板"),
    ("mP6GearAndFlapsEdge", "起落架与襟翼边框\u{3000}"),
    ("mP6GearAndFlapsEdgeBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mP6ecCompressor", "显示增压器"),
    ("mP6ecCompressorBlank", "\u{3000}\u{3000}"),
    ("mP6ecLFuel", "显示燃油量"),
    ("mP6ecLFuelBlank", "\u{3000}\u{3000}"),
    ("mP6ecMixture", "显示混合比"),
    ("mP6ecMixtureBlank", "\u{3000}\u{3000}"),
    ("mP6ecPitch", "显示桨\u{3000}距"),
    ("mP6ecPitchBlank", "\u{3000}\u{3000}"),
    ("mP6ecRadiator", "显示散热器"),
    ("mP6ecRadiatorBlank", "\u{3000}\u{3000}"),
    ("mP6ecThrottle", "显示节流阀"),
    ("mP6ecThrottleBlank", "\u{3000}\u{3000}"),
    ("mP6engineControl", "\u{3000}发动机控制面板"),
    ("mP6engineControlBlank", "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}"),
    ("mResetConfirmContent", "确定要重置所有配置项吗？\\n此操作不可撤销。"),
    ("mResetConfirmTitle", "确认重置"),
    ("mSimpleMode", "简单模式"),
    ("mStart", "开\u{3000}始"),
    ("mUpdateAvailableContent", "GitHub上已发布新版本: %s<br>当前版本: %s<br>请点击下方链接下载更新。"),
    ("mUpdateAvailableLinkText", "前往下载页面"),
    ("mUpdateAvailableTitle", "发现新版本"),
    ("mWaitHotkey", "等待按键..."),
    ("noblkx", "找不到blkx文件\n请使用最新WT拆包aces.vromfs.bin"),
    ("oSkeyWord1", "热"),
    ("oSkeyWord2", "温"),
    ("sCheck", "检测到飞机启动"),
    ("sEnter", "等待飞机启动.."),
    ("sTitle", "状态条"),
    ("sWait", "等待建立连接"),
    ("vAileron", "副\u{3000}翼"),
    ("vElevator", "升降舵"),
    ("vRudder", "方向舵"),
    ("vTitle", "操纵面面板"),
    ("vVarioW", "可变翼"),
];

/// 对应 `prog.config.Config.getValue(String)`: 键存在返回值 (可为空串), 缺失返回 `""`。
pub fn config_get_value(key: &str) -> &'static str {
    // Java Hashtable 点查; 362 条静态表线性扫即可, 行为一致
    for (k, v) in LANGUAGE_PROPERTIES {
        if *k == key {
            return v;
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Properties 空白字符集: 空格/制表/换页 (全角空格 U+3000 不算, 原样进值)
    fn is_props_ws(c: char) -> bool {
        c == ' ' || c == '\t' || c == '\u{c}'
    }

    /// `java.util.Properties.load(Reader)` 最小兼容实现 (快照对拍专用):
    /// 注释(#/!)与空行跳过、尾反斜杠续行、键分隔符(`=`/`:`/空白)两侧空白跳过、
    /// 值尾空白保留、`\t \n \r \f \\ \uXXXX` 转义、重复键后者覆盖 (Hashtable.put)。
    /// 局限: 行终止符按 `\n`/`\r\n` 识别 (源文件为 CRLF, 无裸 `\r` 行)。
    fn load_java_properties(text: &str) -> BTreeMap<String, String> {
        // 1) 物理行 → 逻辑行: 尾随奇数个反斜杠 = 续行 (丢弃该反斜杠与行终止符, 续行段前导空白丢弃)
        let mut logical: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut in_entry = false;
        for raw in text.split('\n') {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if in_entry {
                cur.push_str(line.trim_start_matches(is_props_ws));
            } else {
                let t = line.trim_start_matches(is_props_ws);
                if t.is_empty() || t.starts_with('#') || t.starts_with('!') {
                    continue; // 空行/注释行 (注释不可续行)
                }
                cur.clear();
                cur.push_str(t);
                in_entry = true;
            }
            let trailing_bs = cur.len() - cur.trim_end_matches('\\').len();
            if trailing_bs % 2 == 1 {
                cur.pop(); // 转义行终止符的反斜杠丢弃
            } else {
                logical.push(std::mem::take(&mut cur));
                in_entry = false;
            }
        }

        // 2) 逻辑行拆 (key, value) 再解转义
        let mut map = BTreeMap::new();
        for line in &logical {
            let chars: Vec<char> = line.chars().collect();
            let mut i = 0;
            let mut key = String::new();
            // 键到第一个未转义的 空白/'='/':' 为止
            while i < chars.len() {
                let c = chars[i];
                if c == '\\' && i + 1 < chars.len() {
                    key.push(c);
                    key.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '=' || c == ':' || is_props_ws(c) {
                    break;
                }
                key.push(c);
                i += 1;
            }
            while i < chars.len() && is_props_ws(chars[i]) {
                i += 1;
            }
            if i < chars.len() && (chars[i] == '=' || chars[i] == ':') {
                i += 1; // 分隔符
            }
            while i < chars.len() && is_props_ws(chars[i]) {
                i += 1;
            }
            let value: String = chars[i..].iter().collect(); // 值尾空白保留
            map.insert(unescape(&key), unescape(&value));
        }
        map
    }

    /// Properties 单遍解转义; 未知转义取字符本身 (Java 规范行为)
    fn unescape(s: &str) -> String {
        let mut out = String::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match it.next() {
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('n') => out.push('\n'),
                Some('f') => out.push('\u{c}'),
                Some('u') => {
                    let hex: String = it.by_ref().take(4).collect();
                    let cp = u32::from_str_radix(&hex, 16)
                        .unwrap_or_else(|_| panic!("Malformed \\uXXXX 转义: \\u{hex}"));
                    out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                }
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        }
        out
    }

    #[test]
    fn get_value_hit_and_miss() {
        assert_eq!(config_get_value("appName"), "VoidMei");
        assert_eq!(config_get_value("httpHeader"), "\n"); // 文件值 \n → 真实换行 (oracle)
        assert_eq!(config_get_value("eMagneto"), ""); // 存在但值为空
        assert_eq!(config_get_value("__no_such_key__"), ""); // 缺失 → ""
    }

    #[test]
    fn table_size_and_unique_sorted() {
        // Java 8 oracle 实测: cur.properties 加载后共 362 键, 无重复
        // (源文件改动需重新生成本表, 由下方对拍测试强制)
        assert_eq!(LANGUAGE_PROPERTIES.len(), 362);
        let mut seen = std::collections::HashSet::new();
        for (k, _) in LANGUAGE_PROPERTIES {
            assert!(seen.insert(*k), "重复键: {k}");
        }
        assert!(LANGUAGE_PROPERTIES.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn ideographic_space_alignment_preserved() {
        // oracle: mP1TempNotificationBlank = 36 个 U+3000 (对齐占位串)
        let v = config_get_value("mP1TempNotificationBlank");
        assert_eq!(v.chars().count(), 36);
        assert!(v.chars().all(|c| c == '\u{3000}'));
        // oracle: mP4FMPanelBlank = 19 个 U+3000 + 20 个 ASCII 空格 (尾部空白保留)
        let v = config_get_value("mP4FMPanelBlank");
        assert_eq!(v.chars().count(), 39);
        assert!(v.starts_with("\u{3000}\u{3000}\u{3000}"));
        assert!(v.ends_with("                    "));
    }

    #[test]
    fn properties_escape_semantics() {
        // oracle: 文件值 `...？\\n此操作...` → 字面 反斜杠+n, 不是换行
        assert_eq!(config_get_value("mResetConfirmContent"), "确定要重置所有配置项吗？\\n此操作不可撤销。");
        // oracle: aboutcontent 以 \n\r 转义结尾 → 真实 CR LF
        assert!(config_get_value("aboutcontent").ends_with("\n\r"));
        // oracle: noblkx 分隔符后的前导空格被 Properties 跳过
        assert_eq!(config_get_value("noblkx"), "找不到blkx文件\n请使用最新WT拆包aces.vromfs.bin");
    }

    /// 快照对拍 (两轮审查共同警告的漂移守护): 本表必须等于源文件
    /// `lang/cur.properties` 经 Properties 加载后的键值集 —— 源文件改动而
    /// 未再生快照时, 此处按键给出差异并失败, 而非静默与 Java 行为漂移。
    #[test]
    fn table_matches_cur_properties_source() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("lang")
            .join("cur.properties");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("读取 {} 失败: {e} — 对拍需要仓库内源文件", path.display())
        });
        let parsed = load_java_properties(&text);

        let table: BTreeMap<&str, &str> = LANGUAGE_PROPERTIES.iter().copied().collect();
        assert_eq!(table.len(), LANGUAGE_PROPERTIES.len(), "快照存在重复键");

        let mut drift: Vec<String> = Vec::new();
        for (k, tv) in &table {
            match parsed.get(*k) {
                None => drift.push(format!("快照多出键 {k:?} (源文件已无此键)")),
                Some(fv) if fv.as_str() != *tv => {
                    drift.push(format!("键 {k:?} 值不一致: 文件={fv:?} 快照={tv:?}"));
                }
                _ => {}
            }
        }
        for k in parsed.keys() {
            if !table.contains_key(k.as_str()) {
                drift.push(format!("源文件多出键 {k:?} (快照未再生)"));
            }
        }
        assert!(
            drift.is_empty(),
            "table.rs 与 lang/cur.properties 漂移:\n{}",
            drift.join("\n")
        );
    }
}
