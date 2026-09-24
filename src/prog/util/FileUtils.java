package prog.util;

import java.io.BufferedOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.security.MessageDigest;
import java.util.Enumeration;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

public class FileUtils {

    /** 单 zip 条目大小上限(实测最大 blkx ≈0.27MB, 放宽百倍防恶意大文件 OOM) */
    public static final long UNZIP_MAX_ENTRY_BYTES = 32L * 1024 * 1024;
    /** 解压条目总数上限(合法包 3801, 放宽一个数量级) */
    public static final int UNZIP_MAX_ENTRIES = 50000;
    /** 解压累计字节上限(合法包 ≈135MB, 放宽一个数量级, 防磁盘被 zip 炸弹塞满) */
    public static final long UNZIP_MAX_TOTAL_BYTES = 1536L * 1024 * 1024;

    public static String[] getFilelistNameNoEx(String[] list) {
        if (list == null)
            return new String[0];
        String[] a = new String[list.length];
        for (int i = 0; i < list.length; i++) {
            a[i] = getFileNameNoEx(list[i]);
        }
        return a;
    }

    public static String getFileNameNoEx(String filename) {
        if ((filename != null) && (filename.length() > 0)) {
            int dot = filename.lastIndexOf('.');
            if ((dot > -1) && (dot < (filename.length()))) {
                return filename.substring(0, dot);
            }
        }
        return filename;
    }

    /**
     * 计算文件 SHA-256(hex 小写)。失败(IO 错/算法不可用)返回 null 不抛,调用方按校验失败处理。
     */
    public static String sha256Hex(File file) {
        try {
            MessageDigest md = MessageDigest.getInstance("SHA-256");
            try (InputStream in = new FileInputStream(file)) {
                byte[] buf = new byte[65536];
                int n;
                while ((n = in.read(buf)) > 0) {
                    md.update(buf, 0, n);
                }
            }
            StringBuilder sb = new StringBuilder(64);
            for (byte b : md.digest()) {
                sb.append(Character.forDigit((b >> 4) & 0xF, 16));
                sb.append(Character.forDigit(b & 0xF, 16));
            }
            return sb.toString();
        } catch (Exception e) {
            Logger.warn("FileUtils", "sha256 计算失败: " + file + " (" + e + ")");
            return null;
        }
    }

    /**
     * 递归删除(尽力而为,单个删除失败不中断其余)。返回 true=目标已不存在。
     */
    public static boolean deleteRecursively(File f) {
        if (f == null || !f.exists())
            return true;
        if (f.isDirectory()) {
            File[] children = f.listFiles();
            if (children != null) {
                for (File c : children)
                    deleteRecursively(c);
            }
        }
        return f.delete();
    }

    /**
     * renameTo 带重试:Windows 下源目录内有文件被瞬时打开(如杀软扫描/FMLoader 读文件)
     * 会失败,重试窗口内通常已释放。
     */
    public static boolean renameWithRetry(File src, File dest, int attempts, long delayMs) {
        for (int i = 0; i < attempts; i++) {
            if (src.renameTo(dest))
                return true;
            if (i < attempts - 1)
                ExceptionHelper.sleepQuietly(delayMs);
        }
        return false;
    }

    /**
     * 解压 zip 到 destDir,保留条目相对目录结构(fmdata 包顶层为 data/)。
     *
     * <p>安全防护:zip-slip(条目 canonical 路径不得逃逸 destDir)、单条目/总量/条目数
     * 上限(防 zip 炸弹,上限常量见上)。任一防护触发或 IO 失败抛 IOException;
     * 已解出的部分留给调用方清理(整个 staging 目录可安全重删)。
     */
    public static void unzip(File zip, File destDir) throws IOException {
        long totalBytes = 0;
        int entries = 0;
        String destPrefix = destDir.getCanonicalPath() + File.separator;
        try (ZipFile zf = new ZipFile(zip)) {
            Enumeration<? extends ZipEntry> e = zf.entries();
            while (e.hasMoreElements()) {
                ZipEntry entry = e.nextElement();
                if (entry.isDirectory())
                    continue; // 目录条目跳过,由文件条目的 mkdirs 补齐
                if (++entries > UNZIP_MAX_ENTRIES)
                    throw new IOException("zip 条目数超限(>" + UNZIP_MAX_ENTRIES + ")");
                File out = new File(destDir, entry.getName());
                if (!out.getCanonicalPath().startsWith(destPrefix))
                    throw new IOException("zip 条目路径越界: " + entry.getName());
                if (entry.getSize() > UNZIP_MAX_ENTRY_BYTES)
                    throw new IOException("zip 条目过大: " + entry.getName());
                File parent = out.getParentFile();
                if (parent != null)
                    parent.mkdirs();
                long written = 0;
                try (InputStream in = zf.getInputStream(entry);
                        OutputStream os = new BufferedOutputStream(new FileOutputStream(out), 65536)) {
                    byte[] buf = new byte[65536];
                    int n;
                    while ((n = in.read(buf)) > 0) {
                        written += n;
                        if (written > UNZIP_MAX_ENTRY_BYTES)
                            throw new IOException("zip 条目过大: " + entry.getName());
                        os.write(buf, 0, n);
                    }
                }
                totalBytes += written;
                if (totalBytes > UNZIP_MAX_TOTAL_BYTES)
                    throw new IOException("zip 解压总量超限(>" + UNZIP_MAX_TOTAL_BYTES + ")");
            }
        }
    }
}
