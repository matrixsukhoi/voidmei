package ui.minihud;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;

/**
 * 紧凑布局串的字符串编解码。
 * 安全字符集固定为 A-Z a-z 0-9 . _ -，其余字节一律 %XX。
 * 编码后不含引号/反斜杠/换行/分隔符，可安全放进 S 表达式字符串值。
 */
public final class MiniHudTextCodec {

    private static final char[] HEX = "0123456789ABCDEF".toCharArray();

    private MiniHudTextCodec() {
    }

    private static boolean safe(int b) {
        return (b >= 'A' && b <= 'Z') || (b >= 'a' && b <= 'z') || (b >= '0' && b <= '9')
                || b == '.' || b == '_' || b == '-';
    }

    public static String encode(String s) {
        if (s == null || s.isEmpty())
            return "";
        byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
        StringBuilder sb = new StringBuilder(bytes.length + 8);
        for (byte raw : bytes) {
            int b = raw & 0xFF;
            if (safe(b)) {
                sb.append((char) b);
            } else {
                sb.append('%').append(HEX[b >> 4]).append(HEX[b & 0xF]);
            }
        }
        return sb.toString();
    }

    public static String decode(String s) {
        if (s == null || s.isEmpty())
            return "";
        ByteArrayOutputStream out = new ByteArrayOutputStream(s.length());
        int i = 0;
        while (i < s.length()) {
            char c = s.charAt(i);
            if (c == '%' && i + 2 < s.length()) {
                int hi = hex(s.charAt(i + 1));
                int lo = hex(s.charAt(i + 2));
                if (hi >= 0 && lo >= 0) {
                    out.write((hi << 4) | lo);
                    i += 3;
                    continue;
                }
            }
            // 非法 % 序列按原字符保留（容错，不让坏配置整串失效）
            out.write(c & 0xFF);
            i++;
        }
        return new String(out.toByteArray(), StandardCharsets.UTF_8);
    }

    private static int hex(char c) {
        if (c >= '0' && c <= '9')
            return c - '0';
        if (c >= 'A' && c <= 'F')
            return c - 'A' + 10;
        if (c >= 'a' && c <= 'f')
            return c - 'a' + 10;
        return -1;
    }
}
