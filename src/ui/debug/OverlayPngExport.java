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
 *
 * values 文件: 每行 "getter名=数值", 注入动态数据走 FastNumberFormatter。
 * --data 文件: 每行 "key=value" (# 注释), gauge 数值参数见 exportGauge* 各默认。
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
