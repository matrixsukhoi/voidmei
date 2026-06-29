import parser.Blkx;
import prog.i18n.Lang;
import java.io.File;
import java.util.*;

/**
 * FM 数据全量验证器（白盒）。
 *
 * 直接走生产代码路径：遍历 fm/ 下所有 .blkx 文件，调用 Blkx 构造、getAllplotdata、finalizeLoading。
 * 跟 Controller.loadFMData() 走同一条路，不另起炉灶搞 FM 发现逻辑。
 *
 * 用法: java FMDataValidator <dataDir>
 *   默认: ~/Downloads/voidmei/data/aces/gamedata/flightmodels
 *
 * 退出码: 0 = 全成功, 1 = 有失败, 2 = 环境错误
 */
public class FMDataValidator {

    static class Stats {
        int total = 0;
        int ok = 0;
        int fail = 0;
        int exception = 0;
        List<String> details = new ArrayList<>();
        int lastPercent = -1;
        int exceptionPrinted = 0;
        static final int MAX_STACK_PRINT = 3;

        int totalFail() { return fail + exception; }
    }

    public static void main(String[] args) {
        Lang.initLang();

        String dataDirArg = args.length > 0 ? args[0]
            : System.getProperty("user.home") + "/Downloads/voidmei/data/aces/gamedata/flightmodels";
        File dataDir = new File(dataDirArg).getAbsoluteFile();

        // fm 子目录 — 生产代码中 FM 物理文件都在这里
        File fmDir = new File(dataDir, "fm");
        if (!fmDir.isDirectory()) {
            System.err.println("[错误] FM 目录不存在: " + fmDir.getAbsolutePath());
            System.err.println("用法: java FMDataValidator <dataDir>");
            System.err.println("  dataDir 为 flightmodels 目录，其下应有 fm/ 子目录");
            System.exit(2);
        }

        // 收集所有 .blkx FM 文件（排除 .blk，游戏新版本统一用 .blkx）
        File[] fmFiles = fmDir.listFiles((dir, name) ->
            name.toLowerCase().endsWith(".blkx"));
        if (fmFiles == null || fmFiles.length == 0) {
            System.err.println("[错误] fm/ 目录下未找到任何 .blkx 文件: " + fmDir);
            System.exit(2);
        }

        Stats s = new Stats();
        s.total = fmFiles.length;
        long start = System.currentTimeMillis();

        System.out.println("FM 目录: " + fmDir.getAbsolutePath());
        System.out.println("FM 文件: " + s.total + " 个");
        System.out.println();

        for (int i = 0; i < fmFiles.length; i++) {
            File f = fmFiles[i];
            String name = f.getName();

            // 进度报告
            int pct = i * 100 / s.total;
            if (pct >= s.lastPercent + 10) {
                s.lastPercent = pct;
                long elap = System.currentTimeMillis() - start;
                System.out.printf("进度: %d%% (%d/%d) | 耗时 %.1fs | OK:%d 失败:%d 异常:%d%n",
                    pct, i, s.total, elap / 1000.0, s.ok, s.fail, s.exception);
            }

            try {
                // === 生产代码路径: new Blkx(path, name, true) ===
                // Controller.loadFMData() 调的就是这行，第三个参数 true 触发 getload()
                Blkx fm = new Blkx(f.getAbsolutePath(), name, true);

                if (!fm.valid) {
                    s.fail++;
                    s.details.add("[无效] " + name);
                    continue;
                }

                // === 生产代码路径: getAllplotdata + finalizeLoading ===
                fm.getAllplotdata();
                fm.finalizeLoading();
                s.ok++;

            } catch (Exception e) {
                s.exception++;
                String loc = e.getStackTrace().length > 0 ? e.getStackTrace()[0].toString() : "unknown";
                s.details.add("[异常] " + name + ": " + e.getClass().getSimpleName() + " at " + loc);
                if (s.exceptionPrinted < Stats.MAX_STACK_PRINT) {
                    s.exceptionPrinted++;
                    System.err.println("--- 异常 #" + s.exceptionPrinted + ": " + name + " ---");
                    e.printStackTrace(System.err);
                }
            }
        }

        // === 最终报告 ===
        long elapsed = System.currentTimeMillis() - start;
        System.out.println();
        System.out.println("================================================");
        System.out.println("           FM 数据全量验证报告");
        System.out.println("================================================");
        System.out.printf("FM 目录:       %s%n", fmDir.getAbsolutePath());
        System.out.printf("FM 文件总数:   %d%n", s.total);
        System.out.printf("解析成功:      %d%n", s.ok);
        System.out.printf("解析失败:      %d%n", s.fail);
        System.out.printf("异常:          %d%n", s.exception);
        System.out.println("------------------------------------------------");
        System.out.printf("总失败:        %d%n", s.totalFail());
        System.out.printf("总耗时:        %.1fs (%dms)%n", elapsed / 1000.0, elapsed);

        if (s.totalFail() > 0) {
            int limit = Math.min(s.details.size(), 80);
            System.out.println("------------------------------------------------");
            System.out.printf("失败详情 (%d/%d):%n", limit, s.details.size());
            for (int i = 0; i < limit; i++) {
                System.out.printf("  [%d] %s%n", i + 1, s.details.get(i));
            }
            if (s.details.size() > limit) {
                System.out.printf("  ... 还有 %d 条%n", s.details.size() - limit);
            }
            System.out.println("================================================");
            System.out.println("结果: FAILED (" + s.totalFail() + " errors)");
        } else {
            System.out.println("================================================");
            System.out.println("结果: PASSED");
        }

        System.exit(s.totalFail() > 0 ? 1 : 0);
    }
}
