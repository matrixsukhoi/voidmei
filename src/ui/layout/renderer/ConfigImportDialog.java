package ui.layout.renderer;

import java.awt.BasicStroke;
import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.FlowLayout;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.Window;
import java.awt.datatransfer.DataFlavor;
import java.awt.datatransfer.Transferable;
import java.awt.dnd.DnDConstants;
import java.awt.dnd.DropTarget;
import java.awt.dnd.DropTargetDragEvent;
import java.awt.dnd.DropTargetDropEvent;
import java.awt.dnd.DropTargetEvent;
import java.awt.dnd.DropTargetListener;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.KeyEvent;
import java.awt.event.WindowAdapter;
import java.awt.event.WindowEvent;
import java.io.File;
import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;

import javax.swing.BorderFactory;
import javax.swing.JComponent;
import javax.swing.JFileChooser;
import javax.swing.KeyStroke;
import javax.swing.SwingUtilities;
import javax.swing.filechooser.FileNameExtensionFilter;

import com.alee.extended.window.WebPopOver;
import com.alee.laf.button.WebButton;
import com.alee.laf.filechooser.WebFileChooser;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

import prog.AlwaysOnTopCoordinator;
import prog.i18n.Lang;
import ui.replica.PinkStyle;
import ui.replica.ReplicaBuilder;

/**
 * 现代化的配置导入对话框，支持拖放和点击选择文件。
 *
 * <p>Layout:</p>
 * <pre>
 * ┌─────────────────────────────────────────────────────────┐
 * │                    导入配置文件                           │
 * ├─────────────────────────────────────────────────────────┤
 * │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐    │
 * │  │                                                 │    │
 * │  │           [📁 文件图标]                          │    │
 * │  │                                                 │    │
 * │  │      拖放 .cfg 文件到此处                        │    │
 * │  │         或点击选择文件                           │    │
 * │  │                                                 │    │
 * │  │      支持的格式: *.cfg, *.bak                    │    │
 * │  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘    │
 * │                                                         │
 * │  已选择: [无] / [xxx.cfg ✓]                             │
 * │                                                         │
 * │                              [取消]  [导入配置]          │
 * └─────────────────────────────────────────────────────────┘
 * </pre>
 *
 * <p>视觉状态:</p>
 * <ul>
 *   <li>默认: 虚线边框 (COLOR_BORDER)，白色背景</li>
 *   <li>拖放悬停: 粉色边框 (COLOR_PRIMARY)，淡粉色背景</li>
 *   <li>已选择: 显示文件名，"导入配置"按钮启用</li>
 *   <li>无效文件: 红色边框，错误提示</li>
 * </ul>
 */
public class ConfigImportDialog {

    /**
     * 文件选择回调接口
     */
    public interface FileSelectedCallback {
        void onFileSelected(File file);
    }

    // 单例控制：防止重复打开对话框（volatile 保证多线程可见性）
    private static volatile ConfigImportDialog currentInstance = null;
    private static final AtomicBoolean dialogOpen = new AtomicBoolean(false);

    private final WebPopOver popover;
    private final JComponent ownerComponent;
    private final DropZonePanel dropZone;
    private final WebLabel statusLabel;
    private final WebButton importButton;

    private File selectedFile;
    private final FileSelectedCallback callback;

    // 支持的文件扩展名
    private static final String[] SUPPORTED_EXTENSIONS = {"cfg", "bak"};

    // Wine 环境检测（缓存结果）
    private static final boolean IS_WINE = detectWineEnvironment();

    /**
     * 检测是否运行在 Wine 环境下。
     * Wine 会设置特定的环境变量和注册表项。
     */
    private static boolean detectWineEnvironment() {
        // 方法1: 检查 Wine 特有的环境变量
        String winePrefix = System.getenv("WINEPREFIX");
        if (winePrefix != null && !winePrefix.isEmpty()) {
            prog.util.Logger.info("Wine", "检测到 WINEPREFIX: " + winePrefix);
            return true;
        }

        // 方法2: 检查 Wine loader 环境变量
        String wineLoader = System.getenv("WINELOADER");
        if (wineLoader != null && !wineLoader.isEmpty()) {
            prog.util.Logger.info("Wine", "检测到 WINELOADER: " + wineLoader);
            return true;
        }

        // 方法3: 检查 ntdll.dll 中的 wine_get_version 函数（通过类加载器名称推断）
        String javaHome = System.getProperty("java.home", "");
        if (javaHome.contains(".wine") || javaHome.contains("wine")) {
            prog.util.Logger.info("Wine", "检测到 java.home 包含 wine: " + javaHome);
            return true;
        }

        return false;
    }

    // 拖放区域颜色常量（移到外层类以兼容 Java 8）
    private static final Color COLOR_HOVER_BG = new Color(255, 240, 245); // 淡粉色
    private static final Color COLOR_INVALID_BORDER = new Color(220, 53, 69); // 红色

    /**
     * 显示配置导入对话框的唯一入口（静态方法）。
     * 内置 CAS 防重复点击保护，确保同时只有一个对话框实例。
     *
     * @param owner    父组件，用于定位弹出窗口
     * @param callback 用户确认导入时的回调
     */
    public static void showDialog(JComponent owner, FileSelectedCallback callback) {
        // CAS 防止重复打开
        if (!dialogOpen.compareAndSet(false, true)) {
            // 已有对话框打开，聚焦到现有实例
            if (currentInstance != null) {
                currentInstance.bringToFront();
            }
            return;
        }

        // 创建并显示新实例
        currentInstance = new ConfigImportDialog(owner, callback);
        currentInstance.show();
    }

    /**
     * 将对话框置于最前。
     * WebPopOver 本身继承自 Window，直接调用 toFront() 即可。
     */
    private void bringToFront() {
        // WebPopOver 本身就是 Window，不需要通过 getWindowAncestor 获取
        popover.toFront();
    }

    /**
     * 创建配置导入对话框（私有构造函数，使用 showDialog 入口）
     *
     * @param owner    父组件，用于定位弹出窗口
     * @param callback 用户确认导入时的回调
     */
    private ConfigImportDialog(JComponent owner, FileSelectedCallback callback) {
        this.ownerComponent = owner;
        this.callback = callback;
        this.popover = new WebPopOver(owner);

        // 配置弹出窗口
        // Wine 环境下焦点检测不稳定，禁用失焦自动关闭
        // 原生环境下点击外部区域自动关闭
        popover.setCloseOnFocusLoss(!IS_WINE);
        popover.setMovable(true);
        popover.setMargin(5);

        // 注册到全局 popover 追踪，以便 disposeAllPopovers 能清理
        ReplicaBuilder.registerPopover(popover);

        // 监听窗口关闭事件（包括失焦自动关闭、取消按钮、导入完成）
        popover.addWindowListener(new WindowAdapter() {
            @Override
            public void windowClosed(WindowEvent e) {
                cleanup();
            }
        });

        // 主容器
        WebPanel mainPanel = new WebPanel(new BorderLayout(10, 10));
        mainPanel.setOpaque(true);
        mainPanel.setBackground(PinkStyle.COLOR_BG_PANEL);
        mainPanel.setBorder(BorderFactory.createCompoundBorder(
            BorderFactory.createLineBorder(PinkStyle.COLOR_PRIMARY, 1),
            BorderFactory.createEmptyBorder(15, 15, 15, 15)
        ));

        // 标题
        WebLabel titleLabel = new WebLabel(Lang.mImportConfigTitle);
        titleLabel.setFont(PinkStyle.FONT_HEADER);
        titleLabel.setForeground(PinkStyle.COLOR_TEXT);
        titleLabel.setHorizontalAlignment(WebLabel.CENTER);
        mainPanel.add(titleLabel, BorderLayout.NORTH);

        // 拖放区域
        dropZone = new DropZonePanel();
        mainPanel.add(dropZone, BorderLayout.CENTER);

        // 底部面板：状态 + 按钮
        WebPanel bottomPanel = new WebPanel(new BorderLayout(5, 10));
        bottomPanel.setOpaque(false);

        // 状态标签
        statusLabel = new WebLabel(Lang.mImportFileNone);
        statusLabel.setFont(PinkStyle.FONT_NORMAL);
        statusLabel.setForeground(PinkStyle.COLOR_TEXT);
        bottomPanel.add(statusLabel, BorderLayout.NORTH);

        // 按钮行
        WebPanel buttonRow = new WebPanel(new FlowLayout(FlowLayout.RIGHT, 10, 0));
        buttonRow.setOpaque(false);

        WebButton cancelButton = new WebButton(Lang.mCancel);
        cancelButton.setFont(PinkStyle.FONT_NORMAL);
        cancelButton.addActionListener(e -> popover.dispose());
        buttonRow.add(cancelButton);

        importButton = new WebButton(Lang.mImportButtonImport);
        importButton.setFont(PinkStyle.FONT_NORMAL);
        importButton.setEnabled(false); // 初始禁用，直到选择文件
        importButton.addActionListener(e -> {
            if (selectedFile != null && callback != null) {
                popover.dispose();
                callback.onFileSelected(selectedFile);
            }
        });
        buttonRow.add(importButton);

        bottomPanel.add(buttonRow, BorderLayout.SOUTH);
        mainPanel.add(bottomPanel, BorderLayout.SOUTH);

        popover.add(mainPanel);

        // 添加 ESC 键关闭支持
        popover.getRootPane().registerKeyboardAction(
            e -> popover.dispose(),
            KeyStroke.getKeyStroke(KeyEvent.VK_ESCAPE, 0),
            JComponent.WHEN_IN_FOCUSED_WINDOW
        );
    }

    /**
     * 显示对话框（在父窗口中央，避免闪烁）
     */
    private void show() {
        // 通知 z-order 协调器：对话框即将显示
        AlwaysOnTopCoordinator.getInstance().dialogWillShow();

        // 1. 先 pack 获取尺寸（不显示），避免先显示再移动的闪烁
        popover.pack();

        // 2. 计算居中位置
        Window parentWindow = SwingUtilities.getWindowAncestor(ownerComponent);
        if (parentWindow != null) {
            int centerX = parentWindow.getX() + (parentWindow.getWidth() - popover.getWidth()) / 2;
            int centerY = parentWindow.getY() + (parentWindow.getHeight() - popover.getHeight()) / 2;
            popover.setLocation(centerX, centerY);
        }

        // 3. 显示（已在正确位置）
        popover.setVisible(true);

        // 置于最前
        bringToFront();
    }

    /**
     * 统一清理逻辑 - 由 WindowListener.windowClosed 调用
     * 无论是点击外部关闭、取消按钮还是导入完成，都会触发此方法
     */
    private void cleanup() {
        // 从 ReplicaBuilder 移除追踪
        ReplicaBuilder.unregisterPopover(popover);
        // 重置单例状态
        currentInstance = null;
        dialogOpen.set(false);
        // 通知 z-order 协调器：对话框已关闭
        AlwaysOnTopCoordinator.getInstance().dialogDidDismiss();
    }

    /**
     * 关闭对话框
     */
    public void dispose() {
        popover.dispose();
        // cleanup() 会通过 WindowListener 自动调用
    }

    /**
     * 验证文件是否为支持的配置文件格式
     */
    private boolean isValidConfigFile(File file) {
        if (file == null || !file.isFile()) {
            return false;
        }
        String name = file.getName().toLowerCase();
        for (String ext : SUPPORTED_EXTENSIONS) {
            if (name.endsWith("." + ext)) {
                return true;
            }
        }
        return false;
    }

    /**
     * 设置选中的文件并更新 UI 状态
     */
    private void setSelectedFile(File file) {
        if (isValidConfigFile(file)) {
            this.selectedFile = file;
            statusLabel.setText(String.format(Lang.mImportFileSelected, file.getName()));
            statusLabel.setForeground(PinkStyle.COLOR_TEXT);
            importButton.setEnabled(true);
            dropZone.setFileSelected(true);
        } else {
            this.selectedFile = null;
            statusLabel.setText(Lang.mImportDropZoneInvalid);
            statusLabel.setForeground(Color.RED);
            importButton.setEnabled(false);
            dropZone.setFileSelected(false);
        }
    }

    /**
     * 打开文件选择器。
     * 临时禁用 closeOnFocusLoss，防止文件选择器打开时 popover 被自动关闭。
     */
    private void openFileChooser() {
        // 临时禁用失焦关闭，防止文件选择器打开时 popover 关闭
        popover.setCloseOnFocusLoss(false);

        try {
            WebFileChooser fileChooser = new WebFileChooser();
            fileChooser.setDialogTitle(Lang.mImportConfigTitle);
            // 添加两种文件过滤器：.cfg 和 .bak
            fileChooser.setFileFilter(new FileNameExtensionFilter(
                "Config files (*.cfg, *.bak)", SUPPORTED_EXTENSIONS));
            fileChooser.setMultiSelectionEnabled(false);

            Window parentWindow = SwingUtilities.getWindowAncestor(ownerComponent);
            int result = fileChooser.showOpenDialog(parentWindow);
            if (result == JFileChooser.APPROVE_OPTION) {
                setSelectedFile(fileChooser.getSelectedFile());
            }
        } finally {
            // 恢复失焦关闭（仅原生环境）
            popover.setCloseOnFocusLoss(!IS_WINE);
        }
    }

    /**
     * 拖放区域面板 - 支持拖放和点击选择
     */
    private class DropZonePanel extends WebPanel implements DropTargetListener {

        private boolean dragOver = false;
        private boolean fileSelected = false;
        private boolean invalidFile = false;

        public DropZonePanel() {
            setOpaque(false);
            setPreferredSize(new Dimension(320, 150));

            // Wine 环境下禁用 DND，因为 Wine 的 X11 DND 实现与 Java AWT 不兼容
            // 会导致 dragEnter/drop 回调被阻塞或根本不触发
            if (!IS_WINE) {
                new DropTarget(this, DnDConstants.ACTION_COPY, this, true);
                prog.util.Logger.debug("DND 已启用（原生环境）");
            } else {
                prog.util.Logger.info("Wine", "DND 已禁用（Wine 环境不兼容）");
            }

            // 点击事件 - 打开文件选择器（Wine 环境下这是唯一的选择方式）
            addMouseListener(new MouseAdapter() {
                @Override
                public void mouseClicked(MouseEvent e) {
                    openFileChooser();
                }

                @Override
                public void mouseEntered(MouseEvent e) {
                    setCursor(java.awt.Cursor.getPredefinedCursor(java.awt.Cursor.HAND_CURSOR));
                }

                @Override
                public void mouseExited(MouseEvent e) {
                    setCursor(java.awt.Cursor.getDefaultCursor());
                }
            });
        }

        public void setFileSelected(boolean selected) {
            this.fileSelected = selected;
            this.invalidFile = false;
            repaint();
        }

        public void setInvalidFile(boolean invalid) {
            this.invalidFile = invalid;
            repaint();
        }

        @Override
        protected void paintComponent(Graphics g) {
            super.paintComponent(g);

            Graphics2D g2d = (Graphics2D) g.create();
            g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);

            int w = getWidth();
            int h = getHeight();
            int margin = 10;
            int cornerRadius = 8;

            // 背景
            if (dragOver) {
                g2d.setColor(COLOR_HOVER_BG);
            } else {
                g2d.setColor(Color.WHITE);
            }
            g2d.fillRoundRect(margin, margin, w - 2 * margin, h - 2 * margin, cornerRadius, cornerRadius);

            // 边框
            Color borderColor;
            if (invalidFile) {
                borderColor = COLOR_INVALID_BORDER;
            } else if (dragOver || fileSelected) {
                borderColor = PinkStyle.COLOR_PRIMARY;
            } else {
                borderColor = PinkStyle.COLOR_BORDER;
            }

            g2d.setColor(borderColor);
            if (dragOver || fileSelected) {
                // 实线边框
                g2d.setStroke(new BasicStroke(2f));
            } else {
                // 虚线边框
                float[] dash = {8f, 4f};
                g2d.setStroke(new BasicStroke(2f, BasicStroke.CAP_BUTT, BasicStroke.JOIN_MITER, 10f, dash, 0f));
            }
            g2d.drawRoundRect(margin, margin, w - 2 * margin - 1, h - 2 * margin - 1, cornerRadius, cornerRadius);

            // 绘制文字
            g2d.setColor(PinkStyle.COLOR_TEXT);
            Font iconFont = PinkStyle.FONT_NORMAL.deriveFont(Font.PLAIN, 32f);
            Font mainFont = PinkStyle.FONT_NORMAL.deriveFont(Font.BOLD, 14f);
            Font subFont = PinkStyle.FONT_NORMAL.deriveFont(Font.PLAIN, 12f);

            int centerX = w / 2;
            int y = h / 2 - 30;

            // 文件图标 (使用通用 Unicode 字符，避免 emoji 跨平台兼容性问题)
            g2d.setFont(iconFont);
            String icon = "\u21E9"; // ⇩ 下箭头，基本拉丁字符，广泛支持
            int iconWidth = g2d.getFontMetrics().stringWidth(icon);
            g2d.drawString(icon, centerX - iconWidth / 2, y);

            // 主提示文字
            y += 35;
            g2d.setFont(mainFont);
            String mainText;
            if (IS_WINE) {
                // Wine 环境下不支持拖放，只提示点击
                mainText = "点击选择文件";
            } else {
                mainText = dragOver ? Lang.mImportDropZoneRelease : Lang.mImportDropZoneTitle;
            }
            int mainWidth = g2d.getFontMetrics().stringWidth(mainText);
            g2d.drawString(mainText, centerX - mainWidth / 2, y);

            // 副提示文字
            if (!dragOver) {
                y += 20;
                g2d.setFont(subFont);
                g2d.setColor(new Color(120, 120, 120));
                String subText;
                if (IS_WINE) {
                    subText = "(Wine 环境不支持拖放)";
                } else {
                    subText = Lang.mImportDropZoneSubtitle;
                }
                int subWidth = g2d.getFontMetrics().stringWidth(subText);
                g2d.drawString(subText, centerX - subWidth / 2, y);

                // 格式说明
                y += 18;
                String formatText = Lang.mImportDropZoneFormat;
                int formatWidth = g2d.getFontMetrics().stringWidth(formatText);
                g2d.drawString(formatText, centerX - formatWidth / 2, y);
            }

            g2d.dispose();
        }

        // DropTargetListener 实现

        @Override
        public void dragEnter(DropTargetDragEvent dtde) {
            prog.util.Logger.info("DND", "dragEnter 开始");
            try {
                prog.util.Logger.info("DND", "dragEnter: 检查 flavor...");
                if (isFileListFlavor(dtde)) {
                    prog.util.Logger.info("DND", "dragEnter: flavor 匹配，调用 acceptDrag...");
                    dtde.acceptDrag(DnDConstants.ACTION_COPY);
                    prog.util.Logger.info("DND", "dragEnter: acceptDrag 完成");
                    dragOver = true;
                    invalidFile = false;
                    repaint();
                } else {
                    prog.util.Logger.info("DND", "dragEnter: flavor 不匹配，rejectDrag");
                    dtde.rejectDrag();
                }
            } catch (Exception e) {
                prog.util.Logger.warn("dragEnter 异常: " + e.getMessage());
            }
            prog.util.Logger.info("DND", "dragEnter 结束");
        }

        @Override
        public void dragOver(DropTargetDragEvent dtde) {
            // 保持拖放状态（不打日志，避免刷屏）
        }

        @Override
        public void dropActionChanged(DropTargetDragEvent dtde) {
            prog.util.Logger.info("DND", "dropActionChanged");
        }

        @Override
        public void dragExit(DropTargetEvent dte) {
            prog.util.Logger.info("DND", "dragExit");
            dragOver = false;
            repaint();
        }

        /**
         * 处理拖放事件。
         *
         * <p>Wine 环境兼容性修复：</p>
         * <ul>
         *   <li>getTransferData() 是同步阻塞调用，Wine 的 X11 DND 协议实现不完整，
         *       可能导致 Java AWT DND 线程与 Wine 后端通信时死锁</li>
         *   <li>解决方案：将 getTransferData() 移到后台线程执行，添加 2 秒超时保护</li>
         *   <li>UI 更新必须在 EDT 执行，防止竞态条件</li>
         * </ul>
         */
        @Override
        public void drop(DropTargetDropEvent dtde) {
            prog.util.Logger.info("DND", "drop 开始");
            dragOver = false;

            try {
                prog.util.Logger.info("DND", "drop: 检查 isFileListFlavor...");
                boolean isFlavor = isFileListFlavor(dtde);
                prog.util.Logger.info("DND", "drop: isFileListFlavor = " + isFlavor);

                if (isFlavor) {
                    prog.util.Logger.info("DND", "drop: 调用 acceptDrop...");
                    dtde.acceptDrop(DnDConstants.ACTION_COPY);
                    prog.util.Logger.info("DND", "drop: acceptDrop 完成");

                    prog.util.Logger.info("DND", "drop: 调用 getTransferable...");
                    Transferable transferable = dtde.getTransferable();
                    prog.util.Logger.info("DND", "drop: getTransferable 完成, transferable=" + transferable);

                    // 在后台线程执行可能阻塞的 getTransferData() 操作，带超时保护
                    // Wine 环境下此调用可能无限阻塞
                    prog.util.Logger.info("DND", "drop: 创建 executor...");
                    ExecutorService executor = Executors.newSingleThreadExecutor();
                    prog.util.Logger.info("DND", "drop: 提交 getTransferData 任务...");
                    Future<List<File>> future = executor.submit(() -> {
                        prog.util.Logger.info("DND", "drop[后台线程]: 开始 getTransferData...");
                        @SuppressWarnings("unchecked")
                        List<File> files = (List<File>) transferable.getTransferData(
                            DataFlavor.javaFileListFlavor
                        );
                        prog.util.Logger.info("DND", "drop[后台线程]: getTransferData 完成, files=" + files);
                        return files;
                    });
                    prog.util.Logger.info("DND", "drop: 任务已提交，等待结果 (2秒超时)...");

                    try {
                        // 2秒超时，防止Wine环境下无限阻塞
                        List<File> files = future.get(2, TimeUnit.SECONDS);
                        prog.util.Logger.info("DND", "drop: future.get 返回, files=" + files);
                        final boolean success = !files.isEmpty() && isValidConfigFile(files.get(0));
                        final File file = success ? files.get(0) : null;
                        prog.util.Logger.info("DND", "drop: success=" + success + ", file=" + file);

                        // UI更新必须在EDT执行，DND回调可能在非EDT线程
                        SwingUtilities.invokeLater(() -> {
                            if (success) {
                                setSelectedFile(file);
                            } else {
                                setInvalidFile(true);
                                statusLabel.setText(Lang.mImportDropZoneInvalid);
                                statusLabel.setForeground(Color.RED);
                            }
                            repaint();
                        });

                        prog.util.Logger.info("DND", "drop: 调用 dropComplete(" + success + ")...");
                        dtde.dropComplete(success);
                        prog.util.Logger.info("DND", "drop: dropComplete 完成");
                    } catch (TimeoutException e) {
                        // Wine 环境下 getTransferData() 超时
                        prog.util.Logger.warn("drop: getTransferData 超时！");
                        future.cancel(true);
                        prog.util.Logger.info("DND", "drop: 调用 dropComplete(false)...");
                        dtde.dropComplete(false);
                        prog.util.Logger.info("DND", "drop: dropComplete 完成");
                        SwingUtilities.invokeLater(() -> {
                            setInvalidFile(true);
                            statusLabel.setText("拖放操作超时");
                            statusLabel.setForeground(Color.RED);
                            repaint();
                        });
                        prog.util.Logger.warn("拖放操作超时（Wine兼容性问题）");
                    } catch (Exception e) {
                        prog.util.Logger.warn("drop: future.get 异常: " + e.getClass().getName() + ": " + e.getMessage());
                        dtde.dropComplete(false);
                    } finally {
                        prog.util.Logger.info("DND", "drop: shutdownNow executor...");
                        executor.shutdownNow();
                    }
                } else {
                    prog.util.Logger.info("DND", "drop: rejectDrop");
                    dtde.rejectDrop();
                }
            } catch (Exception e) {
                prog.util.Logger.warn("drop 异常: " + e.getClass().getName() + ": " + e.getMessage());
                e.printStackTrace();
                dtde.rejectDrop();
            }
            prog.util.Logger.info("DND", "drop 结束");
        }

        /**
         * 检查拖放事件是否包含文件列表
         */
        private boolean isFileListFlavor(DropTargetDragEvent dtde) {
            return dtde.isDataFlavorSupported(DataFlavor.javaFileListFlavor);
        }

        private boolean isFileListFlavor(DropTargetDropEvent dtde) {
            return dtde.isDataFlavorSupported(DataFlavor.javaFileListFlavor);
        }
    }
}
