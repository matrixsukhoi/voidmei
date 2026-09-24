package ui.util;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.geom.RoundRectangle2D;

import javax.swing.BorderFactory;
import javax.swing.JPanel;
import javax.swing.JWindow;
import javax.swing.SwingUtilities;
import javax.swing.Timer;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

import com.alee.laf.label.WebLabel;

import prog.Application;

/**
 * 项目统一的右下角通知库（替代散落的自绘 toast / ProgressToast）。
 *
 * <p>视觉：per-pixel 透明窗 + 自绘抗锯齿圆角卡片 + 多层淡边假阴影 + 自绘细进度条
 * （不使用 WebLaF 控件，避免其老式观感；只借用全局字体）。多条通知自动垂直堆叠。
 *
 * <p>API：
 * <ul>
 *   <li>{@link #show(String, int)} —— 定时自动消失的结果通知</li>
 *   <li>{@link #showProgress(String)} —— 返回 {@link Progress} 句柄：update(确定进度)/
 *       stage(不确定进度+阶段文本)/close(销毁)。同时至多建议一个进度通知。</li>
 * </ul>
 *
 * <p>线程安全：任意线程可调（内部派发 EDT）。headless（CI 白盒测试）全链 no-op。
 * 已知边界：per-pixel 透明窗在 GPU 兼容模式（软件渲染）+ 部分老驱动下可能异常，
 * 需真机验证；异常时的回退 = 恢复不透明窗底（直角矩形）。
 */
public final class Toast {

	private Toast() {
	}

	// ---- 视觉常量 (集中类头, 调整风格只动这里) ----
	/** 卡片圆角半径 */
	private static final int ARC = 12;
	/** 假阴影层数 (每层外扩 2px, alpha 递减) */
	private static final int SHADOW_LAYERS = 3;
	/** 卡片四边内留白 */
	private static final int PAD_X = 16, PAD_TOP = 10, PAD_BOTTOM = 12;
	/** 卡片底色 (MainForm 白风格) */
	private static final Color CARD_BG = new Color(255, 255, 255, 242);
	private static final Color TEXT_FG = new Color(0, 0, 0, 220);
	private static final Color SHADOW_C = new Color(0, 0, 0, 26);
	private static final Color BAR_FG = new Color(80, 140, 255);
	private static final Color BAR_TRACK = new Color(230, 230, 230);
	/** 进度条高度 (细条) */
	private static final int BAR_H = 5;
	/** 屏幕右下角边距 */
	private static final int MARGIN = 16;
	/** 多条堆叠间距 */
	private static final int STACK_GAP = 8;
	/** 进度窗底部额外高度(容纳进度条) */
	private static final int PROGRESS_EXTRA = PAD_BOTTOM + BAR_H + 4;
	/** 文本区最大宽度 (HTML 折行用, 防长文案撑出超宽卡片; 380 太窄像被掐断, 560 约两行收纳 about) */
	private static final int MAX_TEXT_W = 560;
	/** URL 识别 (ASCII 白名单; 中文紧跟 URL 时自然截断) */
	private static final java.util.regex.Pattern URL_RE = java.util.regex.Pattern
			.compile("https?://[A-Za-z0-9./_#?&=%+\\-]+");
	/** 链接蓝 (与进度条/动作按钮同色系) */
	private static final String LINK_HEX = "#508CFF";

	/** 活动窗栈 (仅 EDT 上增删; CopyOnWriteArrayList 供 relayout 读) */
	private static final List<ToastWindow> stack = new CopyOnWriteArrayList<>();

	private static boolean headless() {
		return java.awt.GraphicsEnvironment.isHeadless();
	}

	/** 文本中的首个 URL; 无则 null。public 供白盒测试 */
	public static String firstUrl(String text) {
		if (text == null)
			return null;
		java.util.regex.Matcher m = URL_RE.matcher(text);
		return m.find() ? m.group() : null;
	}

	/** 转 HTML: 非 URL 段转义(换行→<br>), URL 段包蓝色下划线链接。public 供白盒测试 */
	public static String toHtml(String text) {
		StringBuilder sb = new StringBuilder(text.length() + 32);
		java.util.regex.Matcher m = URL_RE.matcher(text);
		int last = 0;
		while (m.find()) {
			appendEscaped(sb, text.substring(last, m.start()));
			String href = m.group().replace("&", "&amp;");
			sb.append("<a href='").append(href).append("'><font color='").append(LINK_HEX)
					.append("'><u>").append(href).append("</u></font></a>");
			last = m.end();
		}
		appendEscaped(sb, text.substring(last));
		return sb.toString();
	}

	private static void appendEscaped(StringBuilder sb, String s) {
		for (int i = 0; i < s.length(); i++) {
			char c = s.charAt(i);
			if (c == '<')
				sb.append("&lt;");
			else if (c == '>')
				sb.append("&gt;");
			else if (c == '&')
				sb.append("&amp;");
			else if (c == '\n' || c == '\r')
				sb.append("<br>");
			else
				sb.append(c);
		}
	}

	// ---- 门面 ----

	/** 显示一条定时通知, 默认 5 秒后自动销毁 (对齐旧 NotificationService.show 的默认时长) */
	public static void show(final String text) {
		show(text, 5000);
	}

	/** 显示一条定时通知, displayMs 后自动销毁 */
	public static void show(final String text, final int displayMs) {
		show(text, displayMs, null);
	}

	/** 同上, 带左侧图标 (about 等品牌通知用) */
	public static void show(final String text, final int displayMs, final java.awt.Image icon) {
		if (headless())
			return;
		SwingUtilities.invokeLater(() -> {
			PlainToast t = new PlainToast(text, icon);
			t.display();
			Timer timer = new Timer(displayMs, e -> t.close());
			timer.setRepeats(false);
			timer.start();
		});
	}

	/** 显示进度通知并返回控制句柄 (无动作按钮) */
	public static Progress showProgress(final String text) {
		return showProgress(text, (Action[]) null);
	}

	/** 显示进度通知（可带动作按钮, 如"转后台/取消"）并返回控制句柄 */
	public static Progress showProgress(final String text, final Action... actions) {
		if (headless())
			return NOOP_PROGRESS;
		ProgressToastCard card = new ProgressToastCard(actions);
		SwingUtilities.invokeLater(() -> {
			card.window().setLabel(text);
			// 修复: 构造时 label 为空, 尺寸按空文本算的极小; 显示前必须按真实文本重算, 否则溢出
			card.window().layoutAndPlace(PROGRESS_EXTRA);
			card.window().showWindow();
		});
		return card;
	}

	/** 进度句柄 (headless 用的空实现) */
	private static final Progress NOOP_PROGRESS = new Progress() {
		@Override
		public void update(String text, long done, long total) {
		}

		@Override
		public void stage(String text) {
		}

		@Override
		public void close() {
		}

		@Override
		public void dismiss() {
		}
	};

	/** 进度通知控制句柄: 文本+进度一起更新(内部节流), stage 切不确定模式, close 销毁 */
	public interface Progress {
		void update(String text, long done, long total);

		void stage(String text);

		void close();

		/** 收起通知窗（任务继续, 后续更新静默忽略, close 幂等）——"转后台"按钮用 */
		void dismiss();
	}

	/** 通知上的动作按钮（文字小按钮, 排在文本下方右对齐; onClick 在 EDT 执行） */
	public static final class Action {
		final String label;
		final Runnable onClick;

		public Action(String label, Runnable onClick) {
			this.label = label;
			this.onClick = onClick;
		}
	}

	// ---- 窗体与渲染 ----

	/** 自绘圆角卡片窗 (结果通知与进度通知共用骨架) */
	private static class ToastWindow extends JWindow {
		final WebLabel label = new WebLabel();
		/** 进度条数据: -2=无进度条(纯文本通知), -1=不确定模式, 0..100=确定进度 */
		volatile int progress = -2;
		/** 不确定模式滑块相位 0..1 (动画 Timer 驱动) */
		volatile float phase = 0f;
		Timer indeterminateTimer = null;
		/** 动作按钮衬板 (null=无动作行); 高度参与窗尺寸计算 */
		JPanel actionsPad = null;
		/** 左侧图标占宽 (0=无图标), 参与窗宽计算 */
		int iconWidth = 0;
		/** 窗已销毁标记: dismiss/close 后后续 update 静默忽略, 不再操作已 dispose 的窗 */
		volatile boolean closed = false;

		ToastWindow(Action[] actions, java.awt.Image icon) {
			JPanel panel = new JPanel() {
				@Override
				protected void paintComponent(java.awt.Graphics g) {
					Graphics2D g2 = (Graphics2D) g.create();
					paintCard(g2, getWidth(), getHeight());
					g2.dispose();
				}
			};
			panel.setOpaque(false);
			panel.setBackground(new Color(0, 0, 0, 0));
			label.setForeground(TEXT_FG);
			label.setFont(Application.defaultFontBig); // 14pt: 通知要醒目一档, 与 MainForm 按钮同级
			label.setBorder(BorderFactory.createEmptyBorder(PAD_TOP, icon != null ? 8 : PAD_X, 2, PAD_X));
			// 文本行: 左侧可选图标 + 文字; 整行放 NORTH (CENTER 会垂直拉伸压到底部进度条区)
			JPanel textRow = new JPanel(new java.awt.BorderLayout());
			textRow.setOpaque(false);
			if (icon != null) {
				WebLabel iconLabel = new WebLabel(new javax.swing.ImageIcon(icon));
				iconLabel.setBorder(BorderFactory.createEmptyBorder(0, PAD_X, 0, 8));
				textRow.add(iconLabel, java.awt.BorderLayout.WEST);
				iconWidth = iconLabel.getPreferredSize().width;
			}
			textRow.add(label, java.awt.BorderLayout.CENTER);
			panel.add(textRow, java.awt.BorderLayout.NORTH);
			// 动作按钮行: 文本下方右对齐, 平面文字按钮 (蓝字与进度条同色)
			if (actions != null && actions.length > 0) {
				JPanel actionsRow = new JPanel(new java.awt.FlowLayout(java.awt.FlowLayout.RIGHT, 8, 0));
				actionsRow.setOpaque(false);
				for (Action a : actions) {
					javax.swing.JButton btn = new javax.swing.JButton(a.label);
					btn.setFont(Application.defaultFontSmall);
					btn.setContentAreaFilled(false);
					btn.setBorderPainted(false);
					btn.setFocusable(false);
					btn.setForeground(BAR_FG);
					btn.setCursor(java.awt.Cursor.getPredefinedCursor(java.awt.Cursor.HAND_CURSOR));
					btn.setBorder(BorderFactory.createEmptyBorder(2, 6, 2, 6));
					btn.addActionListener(e -> a.onClick.run());
					actionsRow.add(btn);
				}
				actionsPad = new JPanel(new java.awt.BorderLayout());
				actionsPad.setOpaque(false);
				actionsPad.setBorder(BorderFactory.createEmptyBorder(0, PAD_X, 4, PAD_X - 6));
				actionsPad.add(actionsRow, java.awt.BorderLayout.EAST);
				panel.add(actionsPad, java.awt.BorderLayout.CENTER);
			}

			setContentPane(panel);
			// per-pixel 透明窗: 窗底全透明, 圆角卡片由 paintComponent 自绘
			setBackground(new Color(0, 0, 0, 0));
			setAlwaysOnTop(true);
			setFocusable(false);
			// 含 URL 的文本整卡可点 (通知瞬态, 不做精确命中)
			label.addMouseListener(new java.awt.event.MouseAdapter() {
				@Override
				public void mouseClicked(java.awt.event.MouseEvent e) {
					String url = hyperlink;
					if (url != null) {
						try {
							java.awt.Desktop.getDesktop().browse(new java.net.URI(url));
						} catch (Exception ex) {
							prog.util.Logger.warn("Toast", "打开链接失败: " + url);
						}
					}
				}
			});
		}

		/** 文本中的 URL (无则 null); 点击 label 开浏览器 */
		volatile String hyperlink = null;

		void setLabel(String text) {
			hyperlink = firstUrl(text);
			// HTML 化的两种动因: 含 URL(链接化) 或 纯文本单行超宽(HTML 才有折行机制,
			// 纯文本 JLabel 不折行, 曾致 about 无链接的两条单行溢出卡片)
			label.setText(text);
			boolean tooWide = label.getPreferredSize().width > MAX_TEXT_W;
			if (hyperlink != null || tooWide) {
				label.setText("<html>" + toHtml(text) + "</html>");
			}
			// 手型光标只在真有链接时 (仅超宽折行的纯文本不该显示可点)
			label.setCursor(hyperlink != null
					? java.awt.Cursor.getPredefinedCursor(java.awt.Cursor.HAND_CURSOR) : null);
		}

		/** 画假阴影 + 圆角卡片底 */
		private void paintCard(Graphics2D g2, int w, int h) {
			g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
			int inset = SHADOW_LAYERS * 2;
			// 假阴影: 由内向外逐层扩大、alpha 递减的圆角描边
			for (int i = SHADOW_LAYERS; i >= 1; i--) {
				int grow = SHADOW_LAYERS * 2 - i * 2 + 1;
				g2.setColor(new Color(SHADOW_C.getRed(), SHADOW_C.getGreen(), SHADOW_C.getBlue(),
						SHADOW_C.getAlpha() * i / SHADOW_LAYERS));
				g2.fill(new RoundRectangle2D.Float(inset - grow, inset - grow,
						w - inset * 2 + grow * 2, h - inset * 2 + grow * 2, ARC, ARC));
			}
			// 卡片
			g2.setColor(CARD_BG);
			g2.fillRoundRect(inset, inset, w - inset * 2, h - inset * 2, ARC, ARC);
			// 进度条画在卡片底边内侧 (仅进度窗: progress != -2)
			int p = progress;
			if (p != -2) {
				int bw = w - inset * 2 - PAD_X * 2;
				int by = h - inset - PAD_BOTTOM + (PAD_BOTTOM - BAR_H) / 2;
				g2.setColor(BAR_TRACK);
				g2.fillRoundRect(inset + PAD_X, by, bw, BAR_H, BAR_H, BAR_H);
				g2.setColor(BAR_FG);
				if (p < 0) {
					// 不确定模式: 折返式扫描 (iOS/桌面惯例): 滑块到端点减速折返, 全程无跳变;
					// 三角波相位 + smoothstep 缓动, 宽度取业界常规 25% (Material 起始段同量级)
					float ping = phase < 0.5f ? phase * 2f : 2f - phase * 2f;
					float ease = ping * ping * (3f - 2f * ping);
					int sw = (int) (bw * 0.25f);
					int sx = inset + PAD_X + (int) (ease * (bw - sw));
					g2.fillRoundRect(sx, by, Math.max(sw, BAR_H * 2), BAR_H, BAR_H, BAR_H);
				} else {
					int fw = Math.max(BAR_H, bw * p / 100);
					g2.fillRoundRect(inset + PAD_X, by, fw, BAR_H, BAR_H, BAR_H);
				}
			}
		}

		/**
		 * 文本区 preferred 尺寸。HTML 文案必须用 View 量折行后的真实高度——
		 * setSize trick 对 WebLaF 的 label UI 无效, 曾致宽度 cap 了而高度仍按单行算,
		 * 多行文字从卡片底部溢出。纯文本(未超宽未链接)无 BasicHTML view, 走普通 pref。
		 */
		Dimension textPref(int maxW) {
			Object v = label.getClientProperty(javax.swing.plaf.basic.BasicHTML.propertyKey);
			if (v instanceof javax.swing.text.View) {
				javax.swing.text.View view = (javax.swing.text.View) v;
				java.awt.Insets bi = label.getInsets();
				view.setSize(maxW - bi.left - bi.right, 0);
				int w = (int) Math.ceil(view.getPreferredSpan(javax.swing.text.View.X_AXIS));
				int h = (int) Math.ceil(view.getPreferredSpan(javax.swing.text.View.Y_AXIS));
				return new Dimension(w + bi.left + bi.right, h + bi.top + bi.bottom);
			}
			return label.getPreferredSize();
		}

		/** 计算内容尺寸并贴右下角 (调用前 label 文本已设好) */
		void layoutAndPlace(int extraBottom) {
			// label 的 preferred size 已含 empty border (上 PAD_TOP/左右 PAD_X/下 2),
			// 此处只补阴影 inset 与额外底部区, 再加一次 padding 会重复算宽高
			Dimension pref = textPref(MAX_TEXT_W);
			Dimension ap = actionsPad != null ? actionsPad.getPreferredSize() : new Dimension(0, 0);
			int w = Math.max(Math.min(pref.width, MAX_TEXT_W) + iconWidth, ap.width)
					+ SHADOW_LAYERS * 2 * 2;
			int h = pref.height + ap.height + extraBottom + SHADOW_LAYERS * 2 * 2;
			setSize(w, h);
		}

		void showWindow() {
			stack.add(this);
			relayout();
			setVisible(true);
		}

		void closeWindow() {
			if (closed)
				return;
			closed = true;
			stack.remove(this);
			setVisible(false);
			dispose();
			if (indeterminateTimer != null) {
				indeterminateTimer.stop();
				indeterminateTimer = null;
			}
			relayout();
		}

		/** 堆叠重排: 栈底贴屏幕右下角, 后来的窗向上叠 */
		static void relayout() {
			int y = Application.logicalHeight - MARGIN;
			for (int i = 0; i < stack.size(); i++) {
				ToastWindow w = stack.get(i);
				y -= w.getHeight();
				w.setLocation(Application.logicalWidth - w.getWidth() - MARGIN, y);
				if (i + 1 < stack.size())
					y -= STACK_GAP;
			}
		}
	}

	/** 纯文本结果通知 */
	private static class PlainToast {
		final ToastWindow window;

		PlainToast(String text, java.awt.Image icon) {
			window = new ToastWindow(null, icon);
			window.setLabel(text);
			window.layoutAndPlace(PAD_BOTTOM / 2);
		}

		void display() {
			window.showWindow();
		}

		void close() {
			window.closeWindow();
		}
	}

	/** 进度通知卡片 (Progress 句柄实现; 节流 + 不确定动画) */
	private static class ProgressToastCard implements Progress {
		final ToastWindow window;
		volatile long lastUpdateMs = 0;
		/** update 节流间隔 */
		private static final long THROTTLE_MS = 100;

		ProgressToastCard(Action[] actions) {
			window = new ToastWindow(actions, null);
			window.progress = -1; // 初始不确定模式
			window.layoutAndPlace(PROGRESS_EXTRA);
			// 不确定动画: 16ms 步进(60fps; 旧 40ms 一顿一顿), 仅 indeterminate 时重绘
			window.indeterminateTimer = new Timer(16, e -> {
				if (window.progress == -1) {
					window.phase = (window.phase + 0.012f) % 1f; // 全程约 1.4s
					window.repaint();
				}
			});
			window.indeterminateTimer.start();
		}

		ToastWindow window() {
			return window;
		}

		@Override
		public void update(final String text, final long done, final long total) {
			long now = System.currentTimeMillis();
			if (now - lastUpdateMs < THROTTLE_MS)
				return;
			lastUpdateMs = now;
			final int pct = total > 0 ? (int) Math.min(100, done * 100 / total) : -1;
			SwingUtilities.invokeLater(() -> {
				if (window.closed)
					return; // 已 dismiss/close: 后台任务继续, UI 静默
				window.setLabel(text);
				int old = window.progress;
				window.progress = pct;
				if (old != pct)
					window.repaint();
				window.layoutAndPlace(PROGRESS_EXTRA);
				ToastWindow.relayout();
			});
		}

		@Override
		public void stage(final String text) {
			SwingUtilities.invokeLater(() -> {
				if (window.closed)
					return;
				window.setLabel(text);
				window.progress = -1;
				window.layoutAndPlace(PROGRESS_EXTRA);
				ToastWindow.relayout();
			});
		}

		@Override
		public void close() {
			SwingUtilities.invokeLater(window::closeWindow);
		}

		@Override
		public void dismiss() {
			// 收起通知窗, 任务继续; closed 标记使后续 update 静默忽略
			SwingUtilities.invokeLater(window::closeWindow);
		}
	}
}
