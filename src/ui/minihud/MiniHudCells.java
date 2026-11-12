package ui.minihud;

import java.util.ArrayList;
import java.util.List;

/**
 * text.row 的单元格编解码。
 * 单元格用 ~ 分隔，格内 7 个子字段用 : 分隔：
 * field : text : label : unit : precision : weight : align
 * 所有子字段值均百分号编码，因此分隔符不会出现在值里。
 */
public final class MiniHudCells {

    public static final class Cell {
        public String field = "";
        public String text = "";
        public String label = "";
        public String unit = "";
        public int precision = -1; // -1 = 未指定，沿用字段目录默认精度
        public double weight = 1.0;
        public char align = 'L';

        public boolean isData() {
            return field != null && !field.isEmpty();
        }
    }

    private MiniHudCells() {
    }

    public static List<Cell> parse(String raw) {
        List<Cell> out = new ArrayList<>();
        if (raw == null || raw.isEmpty())
            return out;
        for (String cellRaw : raw.split("~", -1)) {
            if (cellRaw.isEmpty())
                continue;
            String[] f = cellRaw.split(":", -1);
            if (f.length != 7)
                continue;
            Cell c = new Cell();
            c.field = MiniHudTextCodec.decode(f[0]);
            c.text = MiniHudTextCodec.decode(f[1]);
            c.label = MiniHudTextCodec.decode(f[2]);
            c.unit = MiniHudTextCodec.decode(f[3]);
            try {
                c.precision = Integer.parseInt(f[4]);
            } catch (Exception ignore) {
                c.precision = -1;
            }
            try {
                c.weight = Double.parseDouble(f[5]);
            } catch (Exception ignore) {
            }
            c.align = f[6].isEmpty() ? 'L' : f[6].charAt(0);
            out.add(c);
        }
        return out;
    }

    public static String encode(List<Cell> cells) {
        if (cells == null || cells.isEmpty())
            return "";
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < cells.size(); i++) {
            if (i > 0)
                sb.append('~');
            Cell c = cells.get(i);
            sb.append(MiniHudTextCodec.encode(c.field)).append(':')
                    .append(MiniHudTextCodec.encode(c.text)).append(':')
                    .append(MiniHudTextCodec.encode(c.label)).append(':')
                    .append(MiniHudTextCodec.encode(c.unit)).append(':')
                    .append(c.precision).append(':')
                    .append(c.weight).append(':')
                    .append(c.align);
        }
        return sb.toString();
    }
}
