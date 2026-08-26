package ui.debug;

import java.awt.Color;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.GraphicsEnvironment;
import java.awt.RenderingHints;
import java.awt.Toolkit;
import java.awt.image.BufferedImage;
import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.HashMap;
import java.util.Map;
import java.util.Scanner;

import javax.imageio.ImageIO;

import prog.Application;
import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigProvider;
import prog.config.OverlaySettings;
import ui.component.AttitudeIndicatorGauge;
import ui.component.CompassGauge;
import ui.component.LinearGauge;
import ui.model.DefaultFieldManager;
import ui.model.FieldDefinition;
import ui.overlay.model.HUDData;
import ui.renderer.BOSStyleRenderer;
import ui.renderer.RenderContext;
import ui.util.FastNumberFormatter;

/**
 * FlightInfoOverlay 离屏 PNG 导出器 (Rust 复现对拍基线).
 *
 * 复用生产渲染链 (RenderContext + DefaultFieldManager + BOSStyleRenderer),
 * 画到 BufferedImage(TYPE_INT_ARGB) 后写 PNG, 同时导出布局度量 meta JSON。
 * 不解析 ui_layout.cfg: 字段/颜色/默认值与 Rust 端常量表同源快照。
 *
 * 用法 (repo 根目录):
 *   java -classpath "bin;dep/*" ui.debug.OverlayPngExport --out <p.png> [--meta <p.json>]
 *        [--font-add N] [--column N] [--values values.txt] [--aa on|off]
 *   java -classpath "bin;dep/*" ui.debug.OverlayPngExport --gauge <linear|compass|attitude>
 *        --out <p.png> [--data data.txt] [--aa on|off]
 *   java -classpath "bin;dep/*" ui.debug.OverlayPngExport --minihud --out <p.png> [--aa on|off]
 *
 * values 文件: 每行 "getter名=数值", 注入动态数据走 FastNumberFormatter。
 * --data 文件: 每行 "key=value" (# 注释), gauge 数值参数见 exportGauge* 各默认。
 * --minihud: 默认配置完整 HUD 整帧 (preview 静态数据), 见 exportMiniHud。
 */
public class OverlayPngExport {

    // 与 ui_layout.cfg (panel "飞行信息") 当前默认值一致的快照 (同 rust/src/fields.rs)
    private static final Object[][] FIELD_DEFS = {
            // {key, label, unit, preview, precision}
            { "getIAS", "表  速", "Km/h", "500", 0 },
            { "getTAS", "真空速", "Km/h", "550", 0 },
            { "getMach", "马赫数", "Ma", "0.45", 2 },
            { "getCompass", "航  向", "Deg", "270", 0 },
            { "getAltitude", "高  度", "M", "1500", 0 },
            { "getVario", "爬升率", "M/s", "10", 1 },
            { "getSEP", "S E P", "M/s", "15", 0 },
            { "getAcceleration", "加速度", "M/s²", "1.2", 1 },
            { "getRollRate", "滚转率", "Deg/s", "5.0", 0 },
            { "getNy", "过  载", "G", "1.0", 1 },
            { "getTurnRate", "转弯率", "Deg/s", "2.5", 1 },
            { "getTurnRadius", "转半径", "M", "800", 0 },
            { "getAoA", "攻  角", "Deg", "4.2", 1 },
            { "getAoS", "侧滑角", "Deg", "0.5", 1 },
            { "getWingSweep", "可变翼", "%", "15", 0 },
            { "getRadioAltitude", "测距高", "M", "325", 0 },
    };

    // ui_layout.cfg 当前默认配色 #RRGGBBAA
    private static final int COLOR_NUM = 0xFFFFFFFF;
    private static final int COLOR_LABEL = 0xFFFFFFFF;
    private static final int COLOR_UNIT = 0xE89332FF;
    private static final int COLOR_SHADE = 0x000000FF;

    public static void main(String[] args) throws Exception {
        String out = opt(args, "--out");
        if (out == null) {
            System.err.println("缺少 --out <路径>");
            System.exit(1);
        }
        String meta = opt(args, "--meta");
        int fontAdd = optInt(args, "--font-add", 0);
        int column = optInt(args, "--column", 1);
        boolean aa = !"off".equals(opt(args, "--aa"));
        String valuesPath = opt(args, "--values");

        // 单字符实验模式: 指定字符画到 48x48 (基线 24), 用于光栅化差异分析
        String single = opt(args, "--single");
        if (single != null) {
            registerFonts();
            applyStaticRenderState(true);
            Font f = new Font("Sarasa Mono SC", Font.BOLD, 24);
            BufferedImage im = new BufferedImage(48, 48, BufferedImage.TYPE_INT_ARGB);
            Graphics2D g = im.createGraphics();
            g.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
            g.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
            g.setFont(f);
            g.setColor(new Color(255, 255, 255, 255));
            g.drawString(single, 6, 24);
            g.dispose();
            ImageIO.write(im, "png", new File(out));
            System.out.println("single '" + single + "' -> " + out);
            return;
        }

        // gauge 对拍模式 (D7 验收: 三 gauge 组件像素基线, 对端 = rust parity_gauges.rs)
        String gauge = opt(args, "--gauge");
        if (gauge != null) {
            exportGauge(gauge, args);
            return;
        }

        // MiniHUD 整帧对拍模式 (D7: 默认配置完整 HUD, 对端 = rust parity_minihud.rs)
        if (hasFlag(args, "--minihud")) {
            exportMiniHud(args);
            return;
        }

        registerFonts();
        applyStaticRenderState(aa);

        // RenderContext: 与 FieldOverlay.reinitConfig 相同入口 (component 传 null, 内部走 Toolkit)
        RenderContext ctx = RenderContext.fromSettings(new DefaultSettings(fontAdd), null,
                "flightInfoFontC", "flightInfoColumn", new DefaultLegacyConfig(column));

        DefaultFieldManager fm = new DefaultFieldManager(null);
        for (Object[] def : FIELD_DEFS) {
            fm.addField((String) def[0], (String) def[1], (String) def[2], null,
                    false, false, (String) def[3], null);
            setPrecision(fm.getField((String) def[0]), (Integer) def[4]);
        }
        dumpFontBinding(ctx);

        // 动态值模式: 常量注入 + FastNumberFormatter + visible-when/na-when 手工求值
        if (valuesPath != null) {
            applyValues(fm, valuesPath);
        }

        int visible = fm.visibleCount();
        int width = ctx.getTotalWidth();
        int height = ctx.getTotalHeight(visible);

        BufferedImage img = new BufferedImage(width, height, BufferedImage.TYPE_INT_ARGB);
        Graphics2D g2d = img.createGraphics();
        new BOSStyleRenderer().render(g2d, fm.getFields(), ctx, new int[2]);
        g2d.dispose();
        ImageIO.write(img, "png", new File(out));

        if (meta != null) {
            writeMeta(meta, ctx, visible, aa);
        }
        System.out.println("导出 " + out + " (" + width + "x" + height + ", visible=" + visible + ")");
    }

    /** 注册项目字体, 使 new Font("Sarasa Mono SC", ...) 可解析 */
    private static void registerFonts() throws Exception {
        GraphicsEnvironment ge = GraphicsEnvironment.getLocalGraphicsEnvironment();
        ge.registerFont(Font.createFont(Font.TRUETYPE_FONT, new File("fonts/sarasa-mono-sc-regular.ttf")));
        ge.registerFont(Font.createFont(Font.TRUETYPE_FONT, new File("fonts/sarasa-mono-sc-bold.ttf")));
    }

    /** 诊断: 确认 numFont 实际解析到的字体 (防静默回退) */
    private static void dumpFontBinding(RenderContext ctx) {
        Font f = ctx.numFont;
        System.out.println("[font] req=(\"Sarasa Mono SC\", BOLD, " + f.getSize() + ")"
                + " -> family=" + f.getFamily() + " name=" + f.getFontName()
                + " canDisplayCJK=" + f.canDisplay('航') + " numGlyphs=" + f.getNumGlyphs());
    }

    /** BOSStyleRenderer/TextGauge 读 Application 静态字段, 导出前对齐默认配置 */
    private static void applyStaticRenderState(boolean aa) {
        // 注意: cfg 颜色是 #RRGGBBAA, 而 Color(int, boolean) 按 0xAARRGGBB 解读, 必须拆通道
        Application.colorNum = rgbaColor(COLOR_NUM);
        Application.colorLabel = rgbaColor(COLOR_LABEL);
        Application.colorUnit = rgbaColor(COLOR_UNIT);
        Application.colorShadeShape = rgbaColor(COLOR_SHADE);
        Application.graphAASetting = aa ? RenderingHints.VALUE_ANTIALIAS_ON : RenderingHints.VALUE_ANTIALIAS_OFF;
        Application.textAASetting = aa ? RenderingHints.VALUE_TEXT_ANTIALIAS_ON
                : RenderingHints.VALUE_TEXT_ANTIALIAS_OFF;
    }

    /** #RRGGBBAA (cfg 语义) → Color */
    private static Color rgbaColor(int hex) {
        return new Color((hex >> 24) & 0xFF, (hex >> 16) & 0xFF, (hex >> 8) & 0xFF, hex & 0xFF);
    }

    // ==== gauge 对拍模式 (Rust parity_gauges.rs 同源快照, 改一处必须同步另一处) ====

    /** gauge 画布外扩留白: linear/compass 20, attitude 40 (容纳北三角/文本伸出 preferred 界) */
    private static final int PAD_LINEAR = 20;
    private static final int PAD_COMPASS = 20;
    private static final int PAD_ATTITUDE = 40;

    /**
     * --gauge 模式入口: 三 gauge 组件以最小参数实例化, 画到 preferred size + 2×pad
     * 的 TYPE_INT_ARGB 离屏画布。
     * 风格参数是与 Rust parity_gauges.rs 同源的固定对拍基线, 非 MinimalHUDContext
     * 生产推导值 — 生产侧 applyStyleToComponents 按 hudFontSize 动态换算 (默认
     * crosshairScale=113 → hudFontSize=28, 实为 linear(134,7,f14) /
     * compass(22,2,28,21,f21) / attitude(35,18,22,2,1,f21)); 仅 attitude 基线
     * 恰与 hudFontSize=24 推导巧合一致 (cd=round(2·24·0.618)=30)。
     */
    private static void exportGauge(String name, String[] args) throws Exception {
        String out = opt(args, "--out");
        if (out == null) {
            System.err.println("缺少 --out <路径>");
            System.exit(1);
        }
        boolean aa = !"off".equals(opt(args, "--aa"));
        Map<String, String> data = readPairs(opt(args, "--data"));

        registerFonts();
        applyGaugeStaticState(aa);

        BufferedImage img;
        switch (name) {
            case "linear":
                img = exportLinearGauge(data);
                break;
            case "compass":
                img = exportCompassGauge(data);
                break;
            case "attitude":
                img = exportAttitudeGauge(data);
                break;
            default:
                throw new IllegalArgumentException("未知 gauge: " + name + " (linear|compass|attitude)");
        }
        ImageIO.write(img, "png", new File(out));
        System.out.println("gauge " + name + " -> " + out + " (" + img.getWidth() + "x" + img.getHeight() + ")");
    }

    /** gauge 模式静态态: 只覆盖 AA 开关, 颜色保持 Application.java:106-111 静态默认
     *  (MiniHUD 色系 = Rust gauges_bars/gauge_* 的 COLOR_* 常量同源;
     *  FlightInfo 的 cfg 覆盖色不适用于 gauge 组件) */
    private static void applyGaugeStaticState(boolean aa) {
        Application.graphAASetting = aa ? RenderingHints.VALUE_ANTIALIAS_ON : RenderingHints.VALUE_ANTIALIAS_OFF;
        Application.textAASetting = aa ? RenderingHints.VALUE_TEXT_ANTIALIAS_ON
                : RenderingHints.VALUE_TEXT_ANTIALIAS_OFF;
    }

    /** 生产 paint 链的 RenderingHints (MiniHUDOverlay.paintComponent L243-248 同集) */
    private static void applyGaugeHints(Graphics2D g2d) {
        g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
        g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
        g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
                RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
        g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);
    }

    /** LinearGauge 竖向 (默认 tick 左) — 油门条典型形态 */
    private static BufferedImage exportLinearGauge(Map<String, String> data) {
        final int length = 120, thickness = 8, pad = PAD_LINEAR;
        Font fontNum = new Font("Sarasa Mono SC", Font.BOLD, 24);
        LinearGauge g = new LinearGauge("THR", 110, true);
        g.setStyleContext(length, thickness, fontNum, fontNum);
        int value = (int) dval(data, "value", 55.0);
        g.update(value, sval(data, "display", String.valueOf(value)));

        // preferred size 公式复刻 (LinearGauge.getPreferredSize L61-77: textMetric=(int)(size*2.0))
        int textMetric = (int) (fontNum.getSize() * 2.0);
        BufferedImage img = new BufferedImage(textMetric + thickness + 2 * pad, length + 2 * pad,
                BufferedImage.TYPE_INT_ARGB);
        Graphics2D g2d = img.createGraphics();
        applyGaugeHints(g2d);
        g.draw(g2d, pad, pad, length, thickness, fontNum, fontNum);
        g2d.dispose();
        return img;
    }

    /** CompassGauge — heading=123.4 非基数角覆盖指针旋转 + 三字航向文本 */
    private static BufferedImage exportCompassGauge(Map<String, String> data) {
        final int r = 25, lineWidth = 3, big = 24, small = 12, pad = PAD_COMPASS;
        CompassGauge g = new CompassGauge(r);
        g.setStyleContext(r, lineWidth, big, small, new Font("Sarasa Mono SC", Font.BOLD, small));

        HUDData.Builder b = new HUDData.Builder();
        b.heading = dval(data, "heading", 123.4);
        b.mapGrid = sval(data, "loc", "C4");
        g.onDataUpdate(b.build()); // 派生 compassDx/Dy 即时重算 (无平滑)

        // preferred size = 2r × 2r (CompassGauge.getPreferredSize L57-60)
        BufferedImage img = new BufferedImage(r * 2 + 2 * pad, r * 2 + 2 * pad, BufferedImage.TYPE_INT_ARGB);
        Graphics2D g2d = img.createGraphics();
        applyGaugeHints(g2d);
        g.draw(g2d, pad, pad);
        g2d.dispose();
        return img;
    }

    /** AttitudeIndicatorGauge — 恰为 MiniHUDContext hudFontSize=24 推导 (cd=round(2·24·0.618)=30 / cr=15 / inner=19) */
    private static BufferedImage exportAttitudeGauge(Map<String, String> data) {
        final int cd = 30, cr = 15, inner = 19, lw = 2, half = 1, pad = PAD_ATTITUDE;
        AttitudeIndicatorGauge g = new AttitudeIndicatorGauge();
        Font font = new Font("Sarasa Mono SC", Font.BOLD, 18); // hudFontSizeSmall = 24·0.75
        g.setStyleContext(cd, cr, inner, lw, half, font);

        // aosX 换算依赖 setStyleContext 的 font size — 先 style 后 data (生产同序)
        HUDData.Builder b = new HUDData.Builder();
        b.pitch = dval(data, "pitch", 12.5);
        b.roll = dval(data, "roll", 25.0);
        b.slip = dval(data, "slip", -3.4);
        b.pitchValid = dval(data, "valid", 1.0) != 0.0;
        g.onDataUpdate(b.build());

        // preferred size = cd × cd (AttitudeIndicatorGauge.getPreferredSize L63-66)
        BufferedImage img = new BufferedImage(cd + 2 * pad, cd + 2 * pad, BufferedImage.TYPE_INT_ARGB);
        Graphics2D g2d = img.createGraphics();
        applyGaugeHints(g2d);
        g.draw(g2d, pad, pad);
        g2d.dispose();
        return img;
    }

    /** --data 文件: 每行 "key=value" (# 注释), 值保持字符串由各 gauge 按需解析 */
    private static Map<String, String> readPairs(String path) throws Exception {
        Map<String, String> map = new HashMap<>();
        if (path == null)
            return map;
        try (Scanner sc = new Scanner(new File(path), "UTF-8")) {
            while (sc.hasNextLine()) {
                String line = sc.nextLine().trim();
                if (line.isEmpty() || line.startsWith("#"))
                    continue;
                int eq = line.indexOf('=');
                if (eq > 0)
                    map.put(line.substring(0, eq), line.substring(eq + 1));
            }
        }
        return map;
    }

    private static double dval(Map<String, String> m, String key, double def) {
        String v = m.get(key);
        return v == null ? def : Double.parseDouble(v.trim());
    }

    private static String sval(Map<String, String> m, String key, String def) {
        String v = m.get(key);
        return v == null ? def : v;
    }

    // ==== MiniHUD 整帧对拍模式 (Rust parity_minihud.rs 同源快照, 改一处必须同步另一处) ====

    /** MiniHUDOverlay.java:765 LAYOUT_PADDING 同值 */
    private static final int MINIHUD_LAYOUT_PADDING = 45;

    /**
     * --minihud 模式入口: 默认配置 (ui_layout.cfg panel "MiniHUD" L45-94 :default 快照)
     * 的完整 HUD 离屏渲染; 数据走 preview 静态注入 (service == null → refreshTemplates
     * 的 lines[] 预览串, 同 FieldOverlay POC 模式)。
     *
     * 组装链是 MiniHUDOverlay.init(controller, null, settings) 私有编排的同源快照
     * (reinitConfig→refreshTemplates / initComponentsLayout / applyStyleToComponents /
     * updateComponents 预览分支 / initModernLayout 拓扑 + applyAutoSizing), 组件/ctx/
     * 布局引擎全为生产类; 不实例化 MiniHUDOverlay 本身 (WebLaF WebFrame + Controller
     * 依赖, FieldOverlay POC 先例同为手抄组装)。dpiScale = Application 静态默认 1.0
     * (导出链不跑 DPIHelper.getScale)。
     *
     * 已知端口差异 (rust rows.rs 头部备案): Rust 侧 Row2 以 HUDFlapsRow (合并串文本行,
     * 模板宽 w("    BRKGEAR")=11 格) 占位 HUDMechanizationRow (三段模板占位,
     * w("F100 ")+w("BRK ")+w("GEA")=12 格) → 对拍时 row2 文字位与挂其右缘的
     * attitude/compass 横向 ~1 字符格偏移为已知结构性差异, 其余应逐像素一致 (AA 口径同
     * gauge 对拍)。
     */
    private static void exportMiniHud(String[] args) throws Exception {
        String out = opt(args, "--out");
        if (out == null) {
            System.err.println("缺少 --out <路径>");
            System.exit(1);
        }
        boolean aa = !"off".equals(opt(args, "--aa"));

        registerFonts();
        applyGaugeStaticState(aa); // MiniHUD 色系 = Application 静态默认 (同 gauge 模式, FlightInfo cfg 覆盖色不适用)

        MiniHudSettings settings = new MiniHudSettings();
        ui.overlay.MinimalHUDContext ctx = ui.overlay.MinimalHUDContext.create(settings);

        // --- refreshTemplates() 快照 (MiniHUDOverlay.java L161-208; hudRows 尚未创建,
        //     尾部模板推送段无操作) ---
        String[] lines = new String[6];
        lines[0] = String.format("M%5.2f", 0.85);        // drawHudMach=true (标签开关只影响非 mach 分支)
        lines[1] = "ALT" + String.format("%6s", "1024"); // alwaysShowRadarAltitude=false
        lines[3] = "SEP" + String.format("↑%-4s", "30");
        lines[4] = "G" + String.format("%5s", "2.0");
        lines[2] = String.format("%4s", "");             // enableFlapAngleBar=true
        lines[2] += "BRK";
        lines[2] += "GEAR";
        String lineAoA = String.format("α%3.0f", 20.0);
        String relEnergy = "E114514";
        int aoaY = 10; // init 尾钳制: 10 <= rightDraw(154) 不变
        java.awt.Color aoaColor = Application.colorNum;
        java.awt.Color aoaBarColor = Application.colorNum;

        // --- initComponentsLayout() 快照 (L524-589) ---
        ui.component.FlapAngleBar flapAngleBar = new ui.component.FlapAngleBar();
        ui.component.SpeedRatioBar speedRatioBar = new ui.component.SpeedRatioBar();
        ui.component.CompassGauge compassGauge = new ui.component.CompassGauge(ctx.roundCompass);
        ui.component.AttitudeIndicatorGauge attitudeGauge = new ui.component.AttitudeIndicatorGauge();
        ui.component.CrosshairGauge crosshairGauge = new ui.component.CrosshairGauge();

        ui.component.row.HUDAkbRow row0 = new ui.component.row.HUDAkbRow(0, ctx.drawFont,
                ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw, ctx.lineWidth);
        row0.setTemplate(lines[0], lineAoA);
        ui.component.row.HUDEnergyRow row1 = new ui.component.row.HUDEnergyRow(1, ctx.drawFont,
                ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw);
        row1.setTemplate(lines[1], relEnergy);
        ui.component.row.HUDMechanizationRow row2 = new ui.component.row.HUDMechanizationRow(2,
                ctx.drawFont, ctx.hudFontSize);
        row2.setTemplate(lines[2]); // 使用旧格式模板，内部自动解析 (Java 注释原文)
        ui.component.row.HUDTextRow row3 = new ui.component.row.HUDTextRow(3, ctx.drawFont,
                ctx.hudFontSize);
        row3.setTemplate(lines[3]);
        ui.component.row.HUDManeuverRow row4 = new ui.component.row.HUDManeuverRow(4, ctx.drawFont,
                ctx.hudFontSize, ctx.rightDraw, ctx.halfLine, ctx.lineWidth,
                ctx.strokeThick, ctx.strokeThin);
        row4.setTemplate(lines[4]);
        ui.component.LinearGauge throttleBar = new ui.component.LinearGauge("ThrottleBar", 110, true, false);

        ui.component.row.HUDRow[] hudRows = { row0, row1, row2, row3, row4 };

        // --- applyStyleToComponents() 快照 (L591-647; useTextureCrosshair=false 软件准星) ---
        int w = (int) (ctx.hudFontSize * 0.25);
        int h = (int) (ctx.hudFontSize * 5.5);
        if (w < 6)
            w = 6;
        speedRatioBar.setStyleContext(w, h, ctx.drawFontSSmall);
        crosshairGauge.setStyleContext(settings.getCrosshairScale());
        int responsiveWidth = (int) (ctx.hudFontSize * 6);
        flapAngleBar.setStyleContext(responsiveWidth, ctx.lineWidth + 2, ctx.drawFontSmall);
        compassGauge.setStyleContext(ctx.roundCompass, ctx.lineWidth, ctx.hudFontSize,
                ctx.hudFontSizeSmall, ctx.drawFontSmall);
        compassGauge.setInertialMode(false);
        attitudeGauge.setStyleContext(ctx.compassDiameter, ctx.compassRadius,
                ctx.compassInnerMarkRadius, ctx.lineWidth, ctx.halfLine, ctx.drawFontSmall);
        attitudeGauge.setInertialMode(false);
        row0.setStyle(ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw,
                ctx.lineWidth, (int) ctx.aoaLength);
        row1.setStyle(ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw);
        row2.setStyle(ctx.drawFont, ctx.hudFontSize);
        row3.setStyle(ctx.drawFont, ctx.hudFontSize);
        row4.setStyle(ctx.drawFont, ctx.hudFontSize, ctx.rightDraw, ctx.halfLine, ctx.lineWidth,
                ctx.strokeThick, ctx.strokeThin);
        int responsiveHeight = (int) (ctx.hudFontSize * 4.8);
        throttleBar.setStyleContext(responsiveHeight, ctx.barWidth, ctx.drawFontSSmall, ctx.drawFontSSmall);

        // --- updateComponents() 快照 (L309-402; service==null 预览分支) ---
        flapAngleBar.setVisible(true);  // drawHUDtext && enableFlapAngleBar
        compassGauge.setVisible(false); // showAttitudeGauge=true → 罗盘/姿态互斥
        attitudeGauge.setVisible(true);
        crosshairGauge.setVisible(true); // displayCrosshair (不受 drawHUDtext 管)
        speedRatioBar.setVisible(true);  // showSpeedBar=true
        throttleBar.setVisible(false);
        row0.setVisible(true);
        row0.setShowSpeed(true);
        row0.setShowAoa(true);
        row1.setVisible(true);
        row1.setShowAltitude(true);
        row1.setShowEnergy(true);
        row2.setVisible(true); // 三开关之或 (全开)
        row2.setShowFlaps(true);
        row2.setShowAirbrake(true);
        row2.setShowGear(true);
        row3.setVisible(true);
        row4.setVisible(true);
        row4.setShowGLoad(true);
        row4.setShowManeuverBar(true);
        row0.update(lines[0], false, lineAoA, aoaY, aoaColor, aoaBarColor);
        row1.update(lines[1], false, relEnergy);
        row2.update(lines[2], false); // inAction=false
        row3.update(lines[3], false);
        row4.update(lines[4], false, 0, 0, 0, 0, 0, 0, 0); // maneuverIndex/len 族全 0
        throttleBar.update(0, String.format("%3d", 0));   // service==null → throttleValue=0

        // --- initModernLayout() 快照 (L652-763; displayCrosshair=true → layoutWidth=width*2) ---
        ui.layout.ModernHUDLayoutEngine engine = new ui.layout.ModernHUDLayoutEngine(
                ctx.width * 2, ctx.height);
        engine.setLineHeight(ctx.hudFontSize);
        ui.layout.HUDLayoutNode row0Node = new ui.layout.HUDLayoutNode("row0", row0);
        row0Node.setRelativePosition(2.1, 3.5)
                .setAnchors(ui.layout.Anchor.TOP_LEFT, ui.layout.Anchor.TOP_LEFT);
        engine.addNode(row0Node);
        ui.layout.HUDLayoutNode flapNode = new ui.layout.HUDLayoutNode("flap", flapAngleBar);
        flapNode.setParent(row0Node)
                .setRelativePosition(0, -0.1)
                .setAnchors(ui.layout.Anchor.TOP_LEFT, ui.layout.Anchor.BOTTOM_LEFT);
        engine.addNode(flapNode);
        ui.layout.HUDLayoutNode prevRow = row0Node;
        ui.layout.HUDLayoutNode row2Node = null, row4Node = null;
        for (int i = 1; i < hudRows.length; i++) {
            ui.layout.HUDLayoutNode rowNode = new ui.layout.HUDLayoutNode("row" + i, hudRows[i]);
            rowNode.setParent(prevRow)
                    .setRelativePosition(0, 0.1)
                    .setAnchors(ui.layout.Anchor.BOTTOM_LEFT, ui.layout.Anchor.TOP_LEFT);
            engine.addNode(rowNode);
            prevRow = rowNode;
            if (i == 2)
                row2Node = rowNode;
            else if (i == 4)
                row4Node = rowNode;
        }
        ui.layout.HUDLayoutNode attitudeNode = new ui.layout.HUDLayoutNode("attitude", attitudeGauge);
        attitudeNode.setParent(row2Node)
                .setRelativePosition(0, 0.5)
                .setAnchors(ui.layout.Anchor.BOTTOM_RIGHT, ui.layout.Anchor.TOP_RIGHT);
        engine.addNode(attitudeNode);
        ui.layout.HUDLayoutNode compassNode = new ui.layout.HUDLayoutNode("compass", compassGauge);
        compassNode.setParent(row2Node)
                .setRelativePosition(0, 0.1)
                .setAnchors(ui.layout.Anchor.BOTTOM_RIGHT, ui.layout.Anchor.TOP_RIGHT);
        engine.addNode(compassNode);
        ui.layout.HUDLayoutNode speedBarNode = new ui.layout.HUDLayoutNode("speedBar", speedRatioBar);
        speedBarNode.setParent(row4Node)
                .setRelativePosition(-0.3, 0)
                .setAnchors(ui.layout.Anchor.BOTTOM_LEFT, ui.layout.Anchor.BOTTOM_RIGHT);
        engine.addNode(speedBarNode);
        ui.layout.HUDLayoutNode throttleNode = new ui.layout.HUDLayoutNode("throttle", throttleBar);
        throttleNode.setParent(row4Node)
                .setRelativePosition(-0.3, 0)
                .setAnchors(ui.layout.Anchor.BOTTOM_LEFT, ui.layout.Anchor.BOTTOM_RIGHT);
        engine.addNode(throttleNode);
        ui.layout.HUDLayoutNode crosshairNode = new ui.layout.HUDLayoutNode("crosshair", crosshairGauge);
        crosshairNode.setRelativePosition(0, 0)
                .setAnchors(ui.layout.Anchor.MIDDLE_RIGHT, ui.layout.Anchor.MIDDLE_RIGHT);
        engine.addNode(crosshairNode);

        engine.doLayout();
        java.awt.Container win = new java.awt.Container(); // applyAutoSizing 的 setSize 目标 (离屏替身)
        engine.applyAutoSizing(win, MINIHUD_LAYOUT_PADDING);

        // --- paintComponent 快照 (L241-256): setPaintMode + 4 hints + doLayout + render;
        //     drawBlinkX 的 blinkX 预览恒 false 无输出, 不复刻 ---
        BufferedImage img = new BufferedImage(win.getWidth(), win.getHeight(),
                BufferedImage.TYPE_INT_ARGB);
        Graphics2D g2d = img.createGraphics();
        g2d.setPaintMode();
        applyGaugeHints(g2d);
        engine.doLayout();
        engine.render(g2d);
        g2d.dispose();
        ImageIO.write(img, "png", new File(out));
        System.out.println("minihud -> " + out + " (" + img.getWidth() + "x" + img.getHeight() + ")");
    }

    /** MiniHUD 对拍设置: ui_layout.cfg (panel "MiniHUD" L45-94) :default 快照
     *  (同 rust parity_minihud.rs ParitySettings; 改一处必须同步另一处) */
    private static class MiniHudSettings implements prog.config.HUDSettings {
        public String getNumFont() { return "Sarasa Mono SC"; }
        public int getWindowX(int width) { return 0; }
        public int getWindowY(int height) { return 0; }
        public void saveWindowPosition(double x, double y) { }
        public String getFontName() { return "Sarasa Mono SC"; }
        public String getNumFontName() { return "Sarasa Mono SC"; }
        public int getFontSizeAdd() { return 0; }
        public boolean getBool(String key, boolean def) { return def; } // enableLayoutDebug → false
        public int getInt(String key, int def) { return def; }
        public String getString(String key, String def) { return def; }
        public GroupConfig getGroupConfig() { return null; }
        public boolean autoHideOnFocusLoss() { return false; }
        public int getCrosshairScale() { return 113; } // "minihud大小" :default
        public String getCrosshairName() { return "软件渲染准星"; } // :default → 软件矢量路径
        public boolean isDisplayCrosshair() { return true; }
        public boolean useTextureCrosshair() { return false; }
        public boolean drawHUDText() { return true; }
        public boolean showAttitudeGauge() { return true; }
        public double getAoAWarningRatio() { return 0.2; }   // :default 20 (%)
        public double getAoABarWarningRatio() { return 0.25; } // :default 25 (%)
        public boolean enableFlapAngleBar() { return true; }
        public boolean showSpeedBar() { return true; }
        public boolean drawHudMach() { return true; }
        public boolean isSpeedLabelDisabled() { return false; }
        public boolean isAltitudeLabelDisabled() { return false; }
        public boolean isSEPLabelDisabled() { return false; }
        public boolean showHUDSpeed() { return true; }
        public boolean showHUDAoA() { return true; }
        public boolean showHUDAltitude() { return true; }
        public boolean showHUDEnergy() { return true; }
        public boolean showHUDMechanization() { return true; }
        public boolean showHUDFlaps() { return true; }
        public boolean showHUDAirbrake() { return true; }
        public boolean showHUDGear() { return true; }
        public boolean showHUDSep() { return true; }
        public boolean showHUDGLoad() { return true; }
        public boolean showHUDManeuverBar() { return true; }
        public boolean isAttitudeIndicatorInertialMode() { return false; }
        public boolean isGPUCompatibilityMode() { return false; }
        public boolean alwaysShowRadarAltitude() { return false; }
    }

    /** 无值开关节测 (--minihud) */
    private static boolean hasFlag(String[] args, String key) {
        for (String a : args) {
            if (key.equals(a))
                return true;
        }
        return false;
    }

    private static void setPrecision(ui.model.DataField f, int precision) {
        f.precision = precision;
    }

    /** 注入动态值: 格式化到零 GC buffer, 并执行 visible-when / na-when */
    private static void applyValues(DefaultFieldManager fm, String path) throws Exception {
        Map<String, Double> values = new HashMap<>();
        try (Scanner sc = new Scanner(new File(path), "UTF-8")) {
            while (sc.hasNextLine()) {
                String line = sc.nextLine().trim();
                if (line.isEmpty() || line.startsWith("#"))
                    continue;
                int eq = line.indexOf('=');
                if (eq > 0) {
                    values.put(line.substring(0, eq), Double.parseDouble(line.substring(eq + 1)));
                }
            }
        }
        for (ui.model.DataField f : fm.getFields()) {
            Double v = values.get(f.key);
            if (v == null)
                continue;
            // visible-when (与 ui_layout.cfg 表达式一致)
            if ("getWingSweep".equals(f.key) && Math.abs(v - 0) < 0.0001) {
                f.visible = false;
                continue;
            }
            if ("getRadioAltitude".equals(f.key) && v < 0) {
                f.visible = false;
                continue;
            }
            // na-when: 转半径 > 9999 显示 "-"
            if ("getTurnRadius".equals(f.key) && v > 9999) {
                f.buffer[0] = '-';
                f.length = 1;
                continue;
            }
            // 可变翼显示 ×100 后的值
            double val = "getWingSweep".equals(f.key) ? v * 100 : v;
            f.length = FastNumberFormatter.format(val, f.buffer, f.precision);
        }
    }

    private static void writeMeta(String path, RenderContext ctx, int visible, boolean aa) throws Exception {
        FontMetrics numM = Toolkit.getDefaultToolkit().getFontMetrics(ctx.numFont);
        FontMetrics labelM = Toolkit.getDefaultToolkit().getFontMetrics(ctx.labelFont);
        FontMetrics unitM = Toolkit.getDefaultToolkit().getFontMetrics(ctx.unitFont);
        try (PrintWriter pw = new PrintWriter(new FileWriter(path))) {
            pw.println("{");
            pw.println("  \"font_size\": " + ctx.fontSize + ",");
            pw.println("  \"label_font_size\": " + ctx.labelFont.getSize() + ",");
            pw.println("  \"unit_font_size\": " + ctx.unitFont.getSize() + ",");
            pw.println("  \"column_num\": " + ctx.columnNum + ",");
            pw.println("  \"num_height\": " + ctx.numHeight + ",");
            pw.println("  \"total_width\": " + ctx.getTotalWidth() + ",");
            pw.println("  \"total_height\": " + ctx.getTotalHeight(visible) + ",");
            pw.println("  \"visible_fields\": " + visible + ",");
            pw.println("  \"aa\": " + aa + ",");
            pw.println("  \"num_metrics\": {\"ascent\": " + numM.getAscent() + ", \"descent\": " + numM.getDescent()
                    + ", \"leading\": " + numM.getLeading() + ", \"height\": " + numM.getHeight() + "},");
            pw.println("  \"label_metrics\": {\"ascent\": " + labelM.getAscent() + ", \"descent\": "
                    + labelM.getDescent() + ", \"leading\": " + labelM.getLeading() + ", \"height\": "
                    + labelM.getHeight() + "},");
            pw.println("  \"unit_metrics\": {\"ascent\": " + unitM.getAscent() + ", \"descent\": "
                    + unitM.getDescent() + ", \"leading\": " + unitM.getLeading() + ", \"height\": "
                    + unitM.getHeight() + "},");
            pw.println("  \"colors\": {\"num\": \"#FFFFFFFF\", \"label\": \"#FFFFFFFF\", \"unit\": \"#E89332FF\", \"shade\": \"#000000FF\"}");
            pw.println("}");
        }
    }

    private static String opt(String[] args, String key) {
        for (int i = 0; i < args.length - 1; i++) {
            if (key.equals(args[i]))
                return args[i + 1];
        }
        return null;
    }

    private static int optInt(String[] args, String key, int def) {
        String v = opt(args, key);
        return v == null ? def : Integer.parseInt(v);
    }

    /** 最小 OverlaySettings 实现: 默认字体/字号, 其余空操作 */
    private static class DefaultSettings implements OverlaySettings {
        private final int fontAdd;

        DefaultSettings(int fontAdd) {
            this.fontAdd = fontAdd;
        }

        public int getWindowX(int width) { return 0; }
        public int getWindowY(int height) { return 0; }
        public void saveWindowPosition(double x, double y) { }
        public String getFontName() { return "Sarasa Mono SC"; }
        public String getNumFontName() { return "Sarasa Mono SC"; }
        public int getFontSizeAdd() { return fontAdd; }
        public boolean getBool(String key, boolean def) { return def; }
        public int getInt(String key, int def) { return def; }
        public String getString(String key, String def) { return def; }
        public GroupConfig getGroupConfig() { return null; }
        public boolean autoHideOnFocusLoss() { return false; }
    }

    /** 最小 legacy ConfigProvider: 只提供列数 (ui_layout.cfg 默认 1) */
    private static class DefaultLegacyConfig implements ConfigProvider {
        private final int column;

        DefaultLegacyConfig(int column) {
            this.column = column;
        }

        public String getConfig(String key) {
            return "flightInfoColumn".equals(key) ? String.valueOf(column) : null;
        }

        public void setConfig(String key, String value) { }
        public boolean isFieldDisabled(String key) { return false; }
    }
}
