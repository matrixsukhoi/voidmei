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
import ui.model.DefaultFieldManager;
import ui.model.FieldDefinition;
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
 *
 * values 文件: 每行 "getter名=数值", 注入动态数据走 FastNumberFormatter。
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
