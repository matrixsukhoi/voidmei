package ui.layout.renderer;

import java.awt.BorderLayout;
import java.awt.Font;
import java.awt.Window;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import java.io.File;

import javax.swing.SwingUtilities;

import com.alee.extended.layout.VerticalFlowLayout;
import com.alee.extended.window.WebPopOver;
import com.alee.laf.button.WebButton;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.text.WebTextArea;

import parser.Blkx;
import prog.Application;
import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;
import prog.i18n.Lang;
import prog.util.FileUtils;
import ui.replica.ReplicaBuilder;
import javax.swing.event.PopupMenuListener;
import javax.swing.event.PopupMenuEvent;

public class FMListRowRenderer implements RowRenderer {

    // Drag state for popover
    private int isDragging;
    private int xx;
    private int yy;

    /**
     * FM 预览 popover 的加载代号（P5）：每次 displayFM 递增。
     * 后台线程加载完成 invokeLater 回来时比对，不符说明期间又触发过新的预览
     * （或 popover 已关闭），旧结果直接作废——参考 Controller.previewGeneration 模式。
     */
    private final java.util.concurrent.atomic.AtomicLong fmPreviewGeneration =
            new java.util.concurrent.atomic.AtomicLong();

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        // Parse directory path (standard FM path or from config)
        // P5: 默认路径收编到 FMDataPaths（fm/ 物理文件目录 = flightmodels 根下 "fm" 子目录）
        String dirPath = new File(prog.fm.FMDataPaths.fmDir(), "fm").getPath();
        // If format is provided, use it? original code hardcodes it, but let's be
        // flexible if cfg provides it
        if (row.format != null && !row.format.isEmpty()) {
            dirPath = row.format;
        }

        // Get file list
        File dir = new File(dirPath);
        String[] files = dir.list();
        if (files == null)
            files = new String[0];
        files = FileUtils.getFilelistNameNoEx(files);

        // Get current value
        String currentVal = context.getStringFromConfigService(row.property,
                row.property.contains("0") ? files.length > 0 ? files[0] : "" : files.length > 0 ? files[0] : "");

        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        ReplicaBuilder.getStyle().decorateControlPanel(panel);
        panel.setBorder(javax.swing.BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(row.label);
        if ((row.desc != null && !row.desc.isEmpty()) || (row.descImg != null && !row.descImg.isEmpty())) {
            ReplicaBuilder.applyStylizedTooltip(label, row.desc, row.descImg);
        }
        ReplicaBuilder.getStyle().decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        WebComboBox combo = new WebComboBox(files);
        combo.setEditable(false);
        // combo.setPreferredSize(new Dimension(150, 26));
        // Styling
        combo.setWebColoredBackground(false);
        combo.setShadeWidth(1);
        combo.setDrawFocus(false);
        combo.setFont(Application.defaultFont);
        combo.setExpandedBgColor(new java.awt.Color(0, 0, 0, 0));
        combo.setBackground(new java.awt.Color(0, 0, 0, 0));

        // 注册到全局追踪，以便弹出窗口互斥
        ReplicaBuilder.registerComboBox(combo);

        if (currentVal != null && !currentVal.isEmpty()) {
            combo.setSelectedItem(currentVal);
        }

        // 下拉菜单打开时，关闭其他弹出窗口
        combo.addPopupMenuListener(new PopupMenuListener() {
            @Override
            public void popupMenuWillBecomeVisible(PopupMenuEvent e) {
                ReplicaBuilder.dismissActivePopups();
            }
            @Override
            public void popupMenuWillBecomeInvisible(PopupMenuEvent e) {
            }
            @Override
            public void popupMenuCanceled(PopupMenuEvent e) {
            }
        });

        combo.addActionListener(e -> {
            if (context.isUpdating())
                return;
            Object selected = combo.getSelectedItem();
            if (selected != null) {
                context.syncStringToConfigService(row.property, selected.toString());
                context.onSave();
            }
        });

        panel.add(combo, BorderLayout.CENTER);

        // Add "View" Button
        WebButton viewBtn = new WebButton("View"); // Lang.mView? Using hardcoded for now or Lang if available
        // LoggingPanel used native interaction, here we add a button
        viewBtn.setMargin(0, 5, 0, 5);
        viewBtn.addActionListener(e -> {
            // Launch new Comparison UI
            Window parentWindow = SwingUtilities.getWindowAncestor(panel);
            Object selected = combo.getSelectedItem();
            String fmName = (selected != null) ? selected.toString() : "a_4h";

            // Use Application.ctr if available, or just null if CompactComparisonWindow
            // doesn't strictly need it for basic view
            // CompactComparisonWindow constructor matches: (Window, Controller, String,
            // String)
            ui.window.comparison.CompactComparisonWindow cf = new ui.window.comparison.CompactComparisonWindow(
                    parentWindow,
                    Application.ctr,
                    fmName,
                    null // Single view mode
            );
            cf.setVisible(true);
        });
        panel.add(viewBtn, BorderLayout.EAST);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    /**
     * 在 popover 中显示 FM 数据（P5 异步化）。
     *
     * <p>原实现 EDT 上同步 {@code new Blkx(...)} 全量解析，大 FM 文件会冻结设置页
     * 数百 ms。现改为：popover 先显示"加载中..."占位 → 后台 daemon 线程加载 →
     * {@code invokeLater} 回填文本；popover 已关闭或已有更新一次预览时结果作废
     * （generation 比对 + isDisplayable 双保险）。
     *
     * <p>注：当前无调用方（View 按钮直接打开对比窗口），保留供后续接线。
     */
    private void displayFM(WebPanel source, String planeName) {
        final long gen = fmPreviewGeneration.incrementAndGet();

        Window parentWindow = SwingUtilities.getWindowAncestor(source);

        WebPopOver popOver = new WebPopOver(parentWindow);
        popOver.setMargin(5);
        popOver.setLayout(new VerticalFlowLayout());

        WebButton closeButton = new WebButton(Lang.mCancel, e -> popOver.dispose());
        closeButton.setUndecorated(true);
        closeButton.setFont(Application.defaultFont);
        closeButton.setFontSize((int) (Application.defaultFontsize * 1.5f));
        closeButton.setFontStyle(Font.BOLD);

        // 先给占位文本，真实数据后台加载完成后回填
        WebTextArea textArea = new WebTextArea("加载中...");
        popOver.add(textArea);
        popOver.setFont(Application.defaultFont);
        textArea.setFont(Application.defaultFont);
        textArea.setFontSize((int) (Application.defaultFontsize * 1.2f));
        textArea.setEditable(false);
        popOver.add(closeButton);

        popOver.show(source); // Show relative to source panel

        // 后台线程加载 FM（EDT 零阻塞）
        Thread loader = new Thread(() -> {
            Blkx fmblk = loadFmBlkxForPreview(planeName);
            final String result = (fmblk != null && fmblk.fmdata != null)
                    ? fmblk.fmdata : "FM 加载失败: " + planeName;
            SwingUtilities.invokeLater(() -> {
                // 过期/已关闭作废：generation 不符（期间又开过新预览）或
                // popover 已销毁（用户点了关闭），都不再回填
                if (fmPreviewGeneration.get() != gen || !popOver.isDisplayable()) {
                    return;
                }
                textArea.setText(result);
                textArea.repaint();
            });
        }, "FM-Preview");
        loader.setDaemon(true);
        loader.start();

        // Drag logic
        textArea.addMouseListener(new MouseAdapter() {
            public void mousePressed(MouseEvent e) {
                isDragging = 1;
                xx = e.getX();
                yy = e.getY();
            }

            public void mouseReleased(MouseEvent e) {
                if (isDragging == 1)
                    isDragging = 0;
            }
        });
        textArea.addMouseMotionListener(new MouseMotionAdapter() {
            public void mouseDragged(MouseEvent e) {
                int left = popOver.getLocation().x;
                int top = popOver.getLocation().y;
                popOver.setLocation(left + e.getX() - xx, top + e.getY() - yy);
            }
        });
    }

    /**
     * displayFM 专用 FM 加载（P5 收编）：FMLoader 标准链路优先（机型名 → 中央文件 →
     * 物理文件），MISSING/CORRUPT 时回退按 fm/ 物理文件名直读。
     *
     * <p>名字空间差异：UI 下拉列表列出的是 fm/ 物理文件名（连字符命名，如
     * {@code a-10c}），与中央机型名（下划线命名，如 {@code a_10c}）在数据集中约
     * 84/1210 个机型不同名——对这部分机型 FMLoader 找不到同名中央文件，必须回退
     * 直读才能与收编前行为一致。与 PowerCurveWindow/CompactComparisonWindow 的
     * 回退策略保持统一。
     *
     * @return 加载失败的 Blkx 或 null（文件不存在/解析无效）
     */
    private static Blkx loadFmBlkxForPreview(String planeName) {
        prog.fm.FMHandle handle = prog.fm.FMLoader.load(planeName);
        if (handle.hasFM()) {
            return handle.blkx;
        }
        File f = new File(prog.fm.FMDataPaths.fmDir(), "fm/" + planeName + ".blkx");
        if (!f.exists()) {
            f = new File(prog.fm.FMDataPaths.fmDir(), "fm/" + planeName + ".blk");
        }
        Blkx b = new Blkx(f.getPath(), planeName);
        return b.valid ? b : null;
    }

}
