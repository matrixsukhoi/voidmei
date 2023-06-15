package ui;

import java.awt.Color;
import java.awt.Font;
import java.awt.GridBagConstraints;
import java.awt.GridBagLayout;
import java.awt.GridLayout;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionAdapter;
import java.awt.event.MouseMotionListener;

import javax.swing.SwingConstants;

import com.alee.extended.panel.WebComponentPanel;
import com.alee.extended.progress.WebStepProgress;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.progressbar.WebProgressBar;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.separator.WebSeparator;
import com.alee.laf.slider.WebSlider;
import com.alee.laf.splitpane.WebSplitPane;

import parser.blkx;
import prog.app;
import prog.controller;
import prog.lang;
import prog.service;

import static javax.swing.JSplitPane.HORIZONTAL_SPLIT;
import static com.alee.laf.splitpane.WebSplitPane.VERTICAL_SPLIT;

public class engineInfo2 extends WebFrame implements Runnable {
	/**
	 * 
	 */

	int isDragging = 0;
	int xx = 0;
	int yy = 0;
	public volatile boolean doit = true;
	WebSplitPane splitPane;
	public int overheattime;
	private static final long serialVersionUID = 3063042782594625576L;
	public controller c;
	public service s;
	public blkx p;
	public int wtload1;
	public int oilload1;
	public String status;
	String NumFont;

	long freq;
	long MainCheckMili;
	int WIDTH;
	int HEIGHT;
	int lx;
	int ly;
	int OP;
	Color transParentWhite = app.colorNum;
	Color transParentWhitePlus = app.colorNum;

	Color transparent = new Color(0, 0, 0, 0);
	Color lblShadeColor = app.colorShade;

	WebPanel panel;
	WebComponentPanel webComponentPanel;
	WebProgressBar progressBar1;
	WebSlider slider1;
	WebSlider slider2;
	WebSlider slider3;
	WebSlider slider4;
	WebSlider slider5;
	WebStepProgress wsp1;
	WebStepProgress wsp2;
	WebLabel lblAta;
	WebLabel lblc;
	WebLabel lblc_1;
	WebPanel topPanel;
	WebPanel bottomPanel;
	WebPanel leftPanel;
	WebPanel rightPanel;
	WebLabel lefttitle;
	WebLabel bottomtitle;
	WebLabel lblBhp;
	WebLabel lblBhp_1;
	WebLabel bottomTypel;
	WebLabel bottomType;// 机型
	WebLabel bottomTyper;

	WebLabel bottomThrl;// 推力
	WebLabel bottomThr;// 推力
	WebLabel bottomThrr;// 推力

	WebLabel bottomRpml;// 转速
	WebLabel bottomRpm;// 转速
	WebLabel bottomRpmr;// 转速

	WebLabel bottomHpl;// 功率
	WebLabel bottomHp;// 功率
	WebLabel bottomHpr;// 功率

	WebLabel bottomEhpl;// 轴功率
	WebLabel bottomEhp;// 轴功率
	WebLabel bottomEhpr;// 轴功率

	WebLabel bottomEffl;// 轴功率
	WebLabel bottomEff;// 轴功率
	WebLabel bottomEffr;// 轴功率

	WebLabel bottomPrel;// 进气压
	WebLabel bottomPre;// 进气压
	WebLabel bottomPrer;// 进气压

	WebLabel bottomEngl;// 发动机温
	WebLabel bottomEng;// 发动机温
	WebLabel bottomEngr;// 发动机温

	WebLabel bottomWatl;// 水温
	WebLabel bottomWat;// 水温
	WebLabel bottomWatr;// 水温

	WebLabel bottomOill;// 油温
	WebLabel bottomOil;// 油温
	WebLabel bottomOilr;// 油温

	WebLabel bottomWeil;// 油重
	WebLabel bottomWei;// 油重
	WebLabel bottomWeir;// 油重

	WebLabel bottomTiml;// 预估时间
	WebLabel bottomTim;// 预估时间
	WebLabel bottomTimr;// 预估时间

	WebLabel bottomProl;// 浆距角
	WebLabel bottomPro;// 浆距角
	WebLabel bottomPror;// 浆距角

	WebLabel lblKg;
	WebLabel lblFwd;
	WebLabel lblMin;

	WebLabel label_25;

	WebLabel label_3;
	WebLabel label_6;
	WebLabel label_9;
	WebLabel label_12;
	WebLabel label_15;
	WebLabel label_18;
	WebLabel label_21;
	WebLabel label_24;
	WebLabel label_27;
	WebLabel label_30;
	WebLabel label_33;
	WebLabel label_13;
	public String FontName;
	int font1 = 12;
	int font2 = 14;
	int font3 = 10;
	int fontadd = 0;
	private WebLabel label_16;
	private WebLabel bottomtitle1;
	private WebLabel lefttitle1;
	private WebLabel lefttitle2;

	static Color lblColor = app.colorUnit;
	static Color lblNameColor = app.colorLabel;
	static Color lblNumColor = app.colorNum;

	WebLabel createWebLabel(String text) {
		WebLabel x = new WebLabel(text);
		x.setDrawShade(true);
		x.setForeground(lblNameColor);
		x.setShadeColor(lblShadeColor);

		return x;
	}

	void initLabel(WebLabel x, int font) {

		x.setHorizontalAlignment(SwingConstants.CENTER);
		x.setDrawShade(true);
		x.setForeground(lblNameColor);
		x.setShadeColor(lblShadeColor);
		x.setFont(new Font(FontName, Font.PLAIN, font));
		if (font == font3)
			x.setHorizontalAlignment(SwingConstants.LEFT);
		if (font == font2)
			x.setHorizontalAlignment(SwingConstants.CENTER);
		topPanel.add(x);
	}

	public void initgroup() {

		GridBagLayout layout = new GridBagLayout();

		bottomTypel = new WebLabel("机　型");
		initLabel(bottomTypel, font1);
		bottomType = new WebLabel("");
		// initLabel(bottomType, 14);
		bottomType.setHorizontalAlignment(SwingConstants.CENTER);
		bottomType.setDrawShade(true);
		bottomType.setForeground(lblNameColor);
		bottomType.setShadeColor(lblShadeColor);
		bottomType.setFont(new Font(FontName, Font.PLAIN, 14));
		topPanel.add(bottomType);

		bottomTyper = new WebLabel("");
		initLabel(bottomTyper, font3);

		bottomThrl = new WebLabel("推　力");
		initLabel(bottomThrl, font1);
		bottomThr = new WebLabel();
		initLabel(bottomThr, font2);
		bottomThrr = new WebLabel("kg");
		initLabel(bottomThrr, font3);

		bottomRpml = new WebLabel("转　速");
		initLabel(bottomRpml, font1);
		bottomRpm = new WebLabel();
		initLabel(bottomRpm, font2);
		bottomRpmr = new WebLabel("转/分");
		initLabel(bottomRpmr, font3);

		bottomHpl = new WebLabel("功　率");
		initLabel(bottomHpl, font1);
		bottomHp = new WebLabel();
		initLabel(bottomHp, font2);
		bottomHpr = new WebLabel("bhp");
		initLabel(bottomHpr, font3);

		bottomEhpl = new WebLabel("轴功率");
		initLabel(bottomEhpl, font1);
		bottomEhp = new WebLabel();
		initLabel(bottomEhp, font2);
		bottomEhpr = new WebLabel("bhp");
		initLabel(bottomEhpr, font3);

		bottomEffl = new WebLabel("桨效率");
		initLabel(bottomEffl, font1);
		bottomEff = new WebLabel();
		initLabel(bottomEff, font2);
		bottomEffr = new WebLabel("%");
		initLabel(bottomEffr, font3);

		bottomPrel = new WebLabel("进气压");
		initLabel(bottomPrel, font1);
		bottomPre = new WebLabel();
		initLabel(bottomPre, font2);
		bottomPrer = new WebLabel("ATA");
		initLabel(bottomPrer, font3);

		bottomEngl = new WebLabel("温　度");
		initLabel(bottomEngl, font1);
		bottomEng = new WebLabel();
		initLabel(bottomEng, font2);
		bottomEngr = new WebLabel("℃");
		initLabel(bottomEngr, font3);

		bottomOill = new WebLabel("油　温");
		initLabel(bottomOill, font1);
		bottomOil = new WebLabel();
		initLabel(bottomOil, font2);
		bottomOilr = new WebLabel("℃");
		initLabel(bottomOilr, font3);

		bottomWeil = new WebLabel("油　重");
		initLabel(bottomWeil, font1);
		bottomWei = new WebLabel();
		initLabel(bottomWei, font2);
		bottomWeir = new WebLabel("kg");
		initLabel(bottomWeir, font3);

		bottomTiml = new WebLabel("时　间");
		initLabel(bottomTiml, font1);
		bottomTim = new WebLabel();
		initLabel(bottomTim, font2);
		bottomTimr = new WebLabel("分钟");
		initLabel(bottomTimr, font3);

		bottomProl = new WebLabel("过　热");
		initLabel(bottomProl, font1);
		bottomPro = new WebLabel("0");
		initLabel(bottomPro, font2);
		bottomPror = new WebLabel("秒");
		initLabel(bottomPror, font3);

		// bottomType.setText("asdafsasfas");
		GridBagConstraints s = new GridBagConstraints();// 定义一个GridBagConstraints，
		// 是用来控制添加进的组件的显示位置

		s.fill = GridBagConstraints.BASELINE;
		s.gridwidth = 1;
		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 0;
		s.gridy = 0;
		layout.setConstraints(bottomTypel, s);

		s.weightx = 2;
		s.gridwidth = 2;
		s.weighty = 1;
		s.gridx = 1;
		s.gridy = 0;
		layout.setConstraints(bottomType, s);

		s.gridwidth = 1;
		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 3;
		s.gridy = 0;
		layout.setConstraints(bottomHpl, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 4;
		s.gridy = 0;
		layout.setConstraints(bottomHp, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 5;
		s.gridy = 0;
		layout.setConstraints(bottomHpr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 6;
		s.gridy = 0;
		layout.setConstraints(bottomEngl, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 7;
		s.gridy = 0;
		layout.setConstraints(bottomEng, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 8;
		s.gridy = 0;
		layout.setConstraints(bottomEngr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 9;
		s.gridy = 0;
		layout.setConstraints(bottomWeil, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 10;
		s.gridy = 0;
		layout.setConstraints(bottomWei, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 11;
		s.gridy = 0;
		layout.setConstraints(bottomWeir, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 0;
		s.gridy = 1;
		layout.setConstraints(bottomThrl, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 1;
		s.gridy = 1;
		layout.setConstraints(bottomThr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 2;
		s.gridy = 1;
		layout.setConstraints(bottomThrr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 3;
		s.gridy = 1;
		layout.setConstraints(bottomEhpl, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 4;
		s.gridy = 1;
		layout.setConstraints(bottomEhp, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 5;
		s.gridy = 1;
		layout.setConstraints(bottomEhpr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 6;
		s.gridy = 1;
		layout.setConstraints(bottomOill, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 7;
		s.gridy = 1;
		layout.setConstraints(bottomOil, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 8;
		s.gridy = 1;
		layout.setConstraints(bottomOilr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 9;
		s.gridy = 1;
		layout.setConstraints(bottomTiml, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 10;
		s.gridy = 1;
		layout.setConstraints(bottomTim, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 11;
		s.gridy = 1;
		layout.setConstraints(bottomTimr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 0;
		s.gridy = 2;
		layout.setConstraints(bottomRpml, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 1;
		s.gridy = 2;
		layout.setConstraints(bottomRpm, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 2;
		s.gridy = 2;
		layout.setConstraints(bottomRpmr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 3;
		s.gridy = 2;
		layout.setConstraints(bottomEffl, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 4;
		s.gridy = 2;
		layout.setConstraints(bottomEff, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 5;
		s.gridy = 2;
		layout.setConstraints(bottomEffr, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 6;
		s.gridy = 2;
		layout.setConstraints(bottomPrel, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 7;
		s.gridy = 2;
		layout.setConstraints(bottomPre, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 8;
		s.gridy = 2;
		layout.setConstraints(bottomPrer, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 9;
		s.gridy = 2;
		layout.setConstraints(bottomProl, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 10;
		s.gridy = 2;
		layout.setConstraints(bottomPro, s);

		s.weightx = 1;
		s.weighty = 1;
		s.gridx = 11;
		s.gridy = 2;
		layout.setConstraints(bottomPror, s);

		topPanel.setLayout(layout);

	}

	public void initPanel() {
		// Top part content
		topPanel = new WebPanel(true);
		topPanel.setWebColoredBackground(false);
		topPanel.setShadeWidth(0);
		topPanel.setBackground(transparent);
		topPanel.setBorderColor(transparent);

		// Bottom part content
		bottomPanel = new WebPanel(true);
		bottomPanel.setWebColoredBackground(false);
		bottomPanel.setShadeWidth(0);
		bottomPanel.setBackground(transparent);
		bottomPanel.setBorderColor(transparent);

		// Left part content
		leftPanel = new WebPanel(true);
		leftPanel.setWebColoredBackground(false);
		leftPanel.setShadeWidth(0);
		leftPanel.setBackground(transparent);
		leftPanel.setBorderColor(transparent);

		// Right part content
		rightPanel = new WebPanel(true);

		rightPanel.setWebColoredBackground(false);
		rightPanel.setShadeWidth(0);
		rightPanel.setBackground(transparent);
		rightPanel.setBorderColor(transparent);

		// 左panel控件
		lefttitle = new WebLabel(lang.eThrottle);
		lefttitle.setHorizontalAlignment(SwingConstants.CENTER);
		lefttitle.setDrawShade(true);
		lefttitle.setForeground(lblNameColor);
		lefttitle.setShadeColor(lblShadeColor);
		lefttitle.setFont(new Font(FontName, Font.PLAIN, font1));

		lefttitle1 = new WebLabel(lang.eProppitch);
		lefttitle1.setHorizontalAlignment(SwingConstants.CENTER);
		lefttitle1.setDrawShade(true);
		lefttitle1.setForeground(lblNameColor);
		lefttitle1.setShadeColor(lblShadeColor);
		lefttitle1.setFont(new Font(FontName, Font.PLAIN, font1));

		lefttitle2 = new WebLabel(lang.eMixture);
		lefttitle2.setHorizontalAlignment(SwingConstants.CENTER);
		lefttitle2.setDrawShade(true);
		lefttitle2.setForeground(lblNameColor);
		lefttitle2.setShadeColor(lblShadeColor);
		lefttitle2.setFont(new Font(FontName, Font.PLAIN, font1));

		slider1 = new WebSlider(WebSlider.VERTICAL);

		slider1.setMinimum(0);
		slider1.setMaximum(110);
		slider1.setDrawProgress(true);
		slider1.setMinorTickSpacing(10);
		slider1.setMajorTickSpacing(50);
		slider1.setPaintTicks(true);
		slider1.setPaintLabels(true);
		slider1.setPreferredHeight(120);
		// slider1.setSnapToTicks(true);
		slider1.setProgressShadeWidth(0);
		slider1.setTrackShadeWidth(1);
		slider1.setDrawThumb(false);
		slider1.setThumbShadeWidth(1);
		slider1.setThumbBgBottom(transparent);
		slider1.setThumbBgTop(transparent);
		slider1.setTrackBgBottom(transparent);
		slider1.setTrackBgTop(transparent);
		slider1.setProgressBorderColor(transparent);
		slider1.setProgressTrackBgBottom(transParentWhite);
		slider1.setProgressTrackBgTop(transParentWhite);
		// slider1.settick

		// app.debugPrint(slider1.getComponents()[0]);
		slider1.setProgressRound(0);
		slider1.setTrackRound(1);
		slider1.setSharpThumbAngle(true);
		slider1.setPaintTicks(false);
		slider1.setPaintLabels(false);
		// slider1.setProgressRound(5);
		slider1.setAnimated(false);
		slider1.setAngledThumb(true);
		// slider1.get;
		slider1.setFocusable(false);
		// 取消slider1响应
		MouseListener[] mls = slider1.getMouseListeners();
		MouseMotionListener[] mmls = slider1.getMouseMotionListeners();
		for (int t = 0; t < mls.length; t++) {
			slider1.removeMouseListener(mls[t]);

		}
		for (int t = 0; t < mmls.length; t++) {
			slider1.removeMouseMotionListener(mmls[t]);
		}

		slider2 = new WebSlider(WebSlider.VERTICAL);
		slider2.setMinimum(0);
		slider2.setMaximum(100);
		slider2.setDrawProgress(true);
		slider2.setMinorTickSpacing(10);
		slider2.setMajorTickSpacing(50);
		slider2.setPaintTicks(true);
		slider2.setPreferredHeight(120);
		slider2.setPaintLabels(true);
		// slider1.setSnapToTicks(true);
		slider2.setProgressShadeWidth(0);
		slider2.setTrackShadeWidth(1);
		slider2.setDrawThumb(false);
		slider2.setThumbShadeWidth(1);
		slider2.setThumbBgBottom(transparent);
		slider2.setThumbBgTop(transparent);
		slider2.setTrackBgBottom(transparent);
		slider2.setTrackBgTop(transparent);
		slider2.setProgressBorderColor(transparent);
		slider2.setProgressTrackBgBottom(transParentWhite);
		slider2.setProgressTrackBgTop(transParentWhitePlus);
		slider2.setProgressRound(0);
		slider2.setTrackRound(1);
		slider2.setPaintTicks(false);
		slider2.setPaintLabels(false);
		slider2.setSharpThumbAngle(true);
		slider2.setFocusable(false);

		// 取消slider2响应
		MouseListener[] mls2 = slider2.getMouseListeners();
		MouseMotionListener[] mmls2 = slider2.getMouseMotionListeners();
		for (int t = 0; t < mls2.length; t++) {
			slider2.removeMouseListener(mls2[t]);

		}
		for (int t = 0; t < mmls2.length; t++) {
			slider2.removeMouseMotionListener(mmls2[t]);
		}

		slider3 = new WebSlider(WebSlider.VERTICAL);
		slider3.setMinimum(0);
		slider3.setMaximum(120);
		slider3.setPreferredHeight(120);
		slider3.setDrawProgress(true);
		slider3.setMinorTickSpacing(5);
		slider3.setMajorTickSpacing(50);
		slider3.setPaintTicks(true);
		slider3.setPaintLabels(true);
		slider3.setProgressRound(0);
		slider3.setTrackRound(1);

		// slider1.setSnapToTicks(true);
		slider3.setProgressShadeWidth(0);
		slider3.setSharpThumbAngle(true);
		slider3.setTrackShadeWidth(1);
		slider3.setDrawThumb(false);
		slider3.setThumbShadeWidth(1);
		slider3.setThumbBgBottom(transparent);
		slider3.setThumbBgTop(transparent);
		slider3.setTrackBgBottom(transparent);
		slider3.setTrackBgTop(transparent);
		slider3.setProgressBorderColor(transparent);
		slider3.setProgressTrackBgBottom(transParentWhite);
		slider3.setProgressTrackBgTop(transParentWhitePlus);
		// slider3.setProgressRound(5);
		slider3.setPaintTicks(false);
		slider3.setPaintLabels(false);

		slider3.setFocusable(false);
		// 取消slider1响应
		MouseListener[] mls3 = slider3.getMouseListeners();
		MouseMotionListener[] mmls3 = slider3.getMouseMotionListeners();
		for (int t = 0; t < mls3.length; t++) {
			slider3.removeMouseListener(mls3[t]);

		}
		for (int t = 0; t < mmls3.length; t++) {
			slider3.removeMouseMotionListener(mmls3[t]);
		}

		// 右panel控件

		/*
		 * progressBar1 = new WebProgressBar(WebProgressBar.VERTICAL, 0, 100);
		 * progressBar1.setValue(0); progressBar1.setIndeterminate(false);
		 * progressBar1.setStringPainted(true); //
		 * progressBar1.setForeground(transparent);
		 * progressBar1.setBgBottom(transparent);
		 * progressBar1.setBgTop(transparent); // progressBar1.setBackground(new
		 * Color(0,0,0)); progressBar1.setProgressBottomColor(new Color(255,
		 * 255, 255, 255)); progressBar1.setProgressTopColor(new Color(255, 255,
		 * 255, 255)); progressBar1.setForeground(new Color(245, 248, 250));
		 * progressBar1.setBorderPainted(true); progressBar1.setShadeWidth(1);
		 * // progressBar1.setHighlightDarkWhite(new Color(255,255,255,0));
		 * progressBar1.setFont(new Font(FontName, Font.PLAIN, 12));
		 * progressBar1.setHighlightDarkWhite(new Color(0, 0, 0));
		 * progressBar1.setHighlightWhite(new Color(0, 0, 0));
		 */
		// 下panel
		bottomtitle = new WebLabel(lang.eCompressor);
		bottomtitle.setHorizontalAlignment(SwingConstants.CENTER);
		bottomtitle.setDrawShade(true);
		bottomtitle.setForeground(lblNameColor);
		bottomtitle.setShadeColor(lblShadeColor);
		bottomtitle.setFont(new Font(FontName, Font.PLAIN, font1));

		bottomtitle1 = new WebLabel(lang.eRadiator);
		bottomtitle1.setHorizontalAlignment(SwingConstants.CENTER);
		bottomtitle1.setDrawShade(true);
		bottomtitle1.setForeground(lblNameColor);
		bottomtitle1.setShadeColor(lblShadeColor);
		bottomtitle1.setFont(new Font(FontName, Font.PLAIN, font1));

		// WebLabel bottomtitle2 = new WebLabel(language.eMagneto);
		WebLabel bottomtitle2 = new WebLabel("");
		bottomtitle2.setHorizontalAlignment(SwingConstants.CENTER);
		bottomtitle2.setDrawShade(true);
		bottomtitle2.setForeground(lblNameColor);
		bottomtitle2.setShadeColor(lblShadeColor);
		bottomtitle2.setFont(new Font(FontName, Font.PLAIN, font1));

		slider4 = new WebSlider(WebSlider.HORIZONTAL);
		slider4.setMinimum(0);
		slider4.setMaximum(100);
		slider4.setDrawProgress(true);
		slider4.setPaintTicks(false);
		slider4.setPaintLabels(false);

		slider5 = new WebSlider(WebSlider.HORIZONTAL);
		slider5.setMinimum(0);
		slider5.setMaximum(100);
		slider5.setDrawProgress(true);
		slider5.setPaintTicks(false);
		slider5.setPaintLabels(false);

		wsp1 = new WebStepProgress(1);
		wsp1.setPreferredWidth(200);
		wsp1.setSelectedStepIndex(0);
		wsp1.setShadeWidth(0);
		wsp1.setPathWidth(0);
		wsp1.setStepControlWidth(0);
		wsp1.setDisplayLabels(false);
		// wsp1.setSelectionEnabled(false);
		wsp1.setProgressColor(transParentWhite);
		// wsp1.setPathWidth(0);
		wsp1.setStepControlFillWidth(8);
		wsp1.setPathFillWidth(5);
		MouseListener[] mlsws1 = wsp1.getMouseListeners();
		MouseMotionListener[] mmlsws1 = wsp1.getMouseMotionListeners();
		for (int t = 0; t < mlsws1.length; t++) {
			wsp1.removeMouseListener(mlsws1[t]);

		}
		for (int t = 0; t < mmlsws1.length; t++) {
			wsp1.removeMouseMotionListener(mmlsws1[t]);
		}
		// wsp1.setForeground(transParentWhite);
		// wsp1.setBackground(transParentWhite);
		// wsp1.setDisabledProgressColor(transParentWhite);
		// wsp1.setPathFillWidth(0);

		wsp2 = new WebStepProgress(4);
		wsp2.setPreferredWidth(150);
		wsp2.setSelectedStepIndex(0);
		wsp2.setShadeWidth(0);
		wsp2.setPathWidth(0);
		wsp2.setStepControlWidth(0);
		wsp2.setDisplayLabels(false);
		MouseListener[] mlsws2 = wsp2.getMouseListeners();
		MouseMotionListener[] mmlsws2 = wsp2.getMouseMotionListeners();
		for (int t = 0; t < mlsws2.length; t++) {
			wsp2.removeMouseListener(mlsws2[t]);

		}
		for (int t = 0; t < mmlsws1.length; t++) {
			wsp2.removeMouseMotionListener(mmlsws2[t]);
		}
		// wsp1.setSize(100, 30);

		// slider4.setSnapToTicks(true);

		slider4.setPreferredWidth(100);
		slider4.setProgressShadeWidth(0);
		slider4.setTrackShadeWidth(1);
		// slider4.setDrawThumb(true);
		slider4.setSharpThumbAngle(true);
		slider4.setThumbShadeWidth(1);
		slider4.setThumbBgBottom(transparent);
		slider4.setThumbBgTop(transparent);
		slider4.setTrackBgBottom(transparent);
		slider4.setTrackBgTop(transparent);
		slider4.setProgressBorderColor(transparent);
		slider4.setProgressTrackBgBottom(transParentWhite);
		slider4.setProgressTrackBgTop(transParentWhitePlus);

		slider4.setProgressRound(1);
		slider4.setTrackRound(1);

		// slider4.setProgressRound(5);
		slider4.setDrawThumb(false);
		slider4.setPaintTicks(false);

		slider4.setFocusable(false); // 取消slider4响应
		MouseListener[] mls4 = slider4.getMouseListeners();
		MouseMotionListener[] mmls4 = slider4.getMouseMotionListeners();
		for (int t = 0; t < mls4.length; t++) {
			slider4.removeMouseListener(mls4[t]);

		}
		for (int t = 0; t < mmls4.length; t++) {
			slider4.removeMouseMotionListener(mmls4[t]);
		}

		slider5.setPreferredWidth(100);
		slider5.setProgressShadeWidth(0);
		slider5.setTrackShadeWidth(1);
		slider5.setDrawThumb(true);
		slider5.setSharpThumbAngle(true);
		slider5.setThumbShadeWidth(2);
		slider5.setThumbBgBottom(transparent);
		slider5.setThumbBgTop(transparent);
		slider5.setTrackBgBottom(transparent);
		slider5.setTrackBgTop(transparent);
		slider5.setProgressBorderColor(transparent);
		slider5.setProgressTrackBgBottom(transParentWhite);
		slider5.setProgressTrackBgTop(transParentWhite);
		slider5.setDrawThumb(false);
		slider5.setPaintTicks(false);

		slider5.setFocusable(false); // 取消slider4响应
		MouseListener[] mls5 = slider5.getMouseListeners();
		MouseMotionListener[] mmls5 = slider5.getMouseMotionListeners();
		for (int t = 0; t < mls5.length; t++) {
			slider5.removeMouseListener(mls5[t]);

		}
		for (int t = 0; t < mmls5.length; t++) {
			slider5.removeMouseMotionListener(mmls5[t]);
		}

		// 布局

		// 左Panel
		GridBagLayout G = new GridBagLayout();
		GridBagConstraints s = new GridBagConstraints();// 定义一个GridBagConstraints，
		// 是用来控制添加进的组件的显示位置

		s.fill = GridBagConstraints.BELOW_BASELINE;

		s.gridwidth = 1;
		s.gridheight = 1;
		s.weightx = 1;
		s.weighty = 0;
		s.gridx = 0;
		s.gridy = 0;
		G.setConstraints(lefttitle, s);
		s.gridheight = 1;
		s.weightx = 1;
		s.weighty = 0;
		s.gridx = 1;
		s.gridy = 0;
		G.setConstraints(lefttitle1, s);
		s.gridheight = 1;
		s.weightx = 1;
		s.weighty = 0;
		s.gridx = 2;
		s.gridy = 0;
		G.setConstraints(lefttitle2, s);
		s.gridheight = 3;
		s.weightx = 1;
		s.weighty = 0;
		s.gridx = 0;
		s.gridy = 1;
		G.setConstraints(slider1, s);
		s.gridheight = 3;
		s.weightx = 1;
		s.weighty = 0;
		s.gridx = 1;
		s.gridy = 1;
		G.setConstraints(slider2, s);
		s.gridheight = 3;
		s.weightx = 1;
		s.weighty = 0;
		s.gridx = 2;
		s.gridy = 1;
		G.setConstraints(slider3, s);

		leftPanel.setLayout(G);
		leftPanel.add(lefttitle);
		leftPanel.add(lefttitle1);
		leftPanel.add(lefttitle2);
		leftPanel.add(slider1);
		leftPanel.add(slider2);
		leftPanel.add(slider3);
		// final TableLayout groupLayout = new TableLayout ( new double[][]{
		// columns, rows } );
		// groupLayout.setHGap ( 4 );
		// groupLayout.setVGap ( 4 );
		// panel.setLayout ( groupLayout );

		// 右Panel

		bottomPanel.setLayout(new GridLayout(2, 3));
		bottomPanel.add(bottomtitle);
		bottomPanel.add(bottomtitle1);
		bottomPanel.add(bottomtitle2);
		// bottomPanel.add(" 1");
		bottomPanel.add(wsp1);
		bottomPanel.add(slider4);
		// bottomPanel.add(slider5);
		// bottomPanel.add(wsp2);

		initPanelText();//
		// initPanelText22();
	}

	public void fclose() {
		// this.setVisible(false);

		this.dispose();
		// this.close();
	}

	public void initPreview(controller c) {

		init(c, null, null);
		// setShadeWidth(10);
		this.setVisible(false);
		// this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 50));
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 1));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 1));// 顶部透明

		leftPanel.addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
				/*
				 * if(A.tag==0){ if(f.mode==1){ A.setVisible(false);
				 * A.visibletag=0; } }
				 */
			}

			public void mousePressed(MouseEvent e) {
				isDragging = 1;
				xx = e.getX();
				yy = e.getY();

			}

			public void mouseReleased(MouseEvent e) {
				if (isDragging == 1) {
					isDragging = 0;
				}
				/*
				 * if(A.tag==0){ A.setVisible(false); }
				 */
			}
			/*
			 * public void mouseReleased(MouseEvent e){ if(A.tag==0){
			 * A.setVisible(true); } }
			 */
		});
		leftPanel.addMouseMotionListener(new MouseMotionAdapter() {
			public void mouseDragged(MouseEvent e) {
				if (isDragging == 1) {
					int left = getLocation().x;
					int top = getLocation().y;
					setLocation(left + e.getX() - xx, top + e.getY() - yy);
					setVisible(true);
					repaint();
				}
			}
		});

		topPanel.addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
				/*
				 * if(A.tag==0){ if(f.mode==1){ A.setVisible(false);
				 * A.visibletag=0; } }
				 */
			}

			public void mousePressed(MouseEvent e) {
				isDragging = 1;
				xx = e.getX();
				yy = e.getY();

			}

			public void mouseReleased(MouseEvent e) {
				if (isDragging == 1) {
					isDragging = 0;
				}
				/*
				 * if(A.tag==0){ A.setVisible(false); }
				 */
			}
			/*
			 * public void mouseReleased(MouseEvent e){ if(A.tag==0){
			 * A.setVisible(true); } }
			 */
		});
		topPanel.addMouseMotionListener(new MouseMotionAdapter() {
			public void mouseDragged(MouseEvent e) {
				if (isDragging == 1) {
					int left = getLocation().x;
					int top = getLocation().y;
					setLocation(left + e.getX() - xx, top + e.getY() - yy);
					setVisible(true);
					repaint();
				}
			}
		});

		bottomPanel.addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
				/*
				 * if(A.tag==0){ if(f.mode==1){ A.setVisible(false);
				 * A.visibletag=0; } }
				 */
			}

			public void mousePressed(MouseEvent e) {
				isDragging = 1;
				xx = e.getX();
				yy = e.getY();

			}

			public void mouseReleased(MouseEvent e) {
				if (isDragging == 1) {
					isDragging = 0;
				}
				/*
				 * if(A.tag==0){ A.setVisible(false); }
				 */
			}
			/*
			 * public void mouseReleased(MouseEvent e){ if(A.tag==0){
			 * A.setVisible(true); } }
			 */
		});
		bottomPanel.addMouseMotionListener(new MouseMotionAdapter() {
			public void mouseDragged(MouseEvent e) {
				if (isDragging == 1) {
					int left = getLocation().x;
					int top = getLocation().y;
					setLocation(left + e.getX() - xx, top + e.getY() - yy);
					setVisible(true);
					repaint();
				}
			}
		});
		this.setCursor(null);
		setVisible(true);
		// setFocusable(true);
		// setFocusableWindowState(true);

	}

	public void initPanelText() {
		topPanel.setLayout(null);

		WebPanel panel_1 = new WebPanel();
		panel_1.setLayout(null);
		panel_1.setWebColoredBackground(false);
		panel_1.setBackground(transparent);
		panel_1.setBounds(0, 0, 110, 36);
		topPanel.add(panel_1);

		lblFwd = createWebLabel("Fw-190D");
		lblFwd.setHorizontalAlignment(SwingConstants.CENTER);
		lblFwd.setForeground(lblNumColor);
		lblFwd.setFont(new Font(NumFont, Font.PLAIN, 16 + fontadd));
		lblFwd.setBounds(0, 0, 110, 36);
		panel_1.add(lblFwd);

		// WebLabel label = createWebLabel(language.eType);
		// label.setBounds(72, 0, 36, 18);
		// panel_1.add(label);
		// label.setVerticalAlignment(SwingConstants.BOTTOM);
		// label.setHorizontalAlignment(SwingConstants.LEFT);
		// label.setForeground(lblNumColor);
		// label.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		//
		WebLabel label_2 = createWebLabel("");
		label_2.setBounds(72, 18, 36, 18);
		panel_1.add(label_2);
		label_2.setVerticalAlignment(SwingConstants.TOP);
		label_2.setHorizontalAlignment(SwingConstants.LEFT);
		label_2.setForeground(lblColor);
		label_2.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));

		// 功率
		WebPanel panel = new WebPanel();
		panel.setLayout(null);
		panel.setWebColoredBackground(false);
		panel.setBackground(transparent);
		panel.setBounds(110, 0, 110, 36);
		topPanel.add(panel);

		label_3 = createWebLabel("2050");
		label_3.setHorizontalAlignment(SwingConstants.CENTER);
		label_3.setForeground(lblNumColor);
		label_3.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_3.setBounds(0, 0, 72, 36);
		panel.add(label_3);

		WebLabel label_4 = createWebLabel(lang.ePower);
		label_4.setVerticalAlignment(SwingConstants.BOTTOM);
		label_4.setHorizontalAlignment(SwingConstants.LEFT);
		label_4.setForeground(lblNameColor);
		label_4.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_4.setBounds(72, 0, 36, 18);
		panel.add(label_4);

		lblBhp = createWebLabel("Hp");
		lblBhp.setVerticalAlignment(SwingConstants.TOP);
		lblBhp.setHorizontalAlignment(SwingConstants.LEFT);
		lblBhp.setForeground(lblColor);
		lblBhp.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblBhp.setBounds(72, 18, 36, 18);
		panel.add(lblBhp);

		// 推力
		WebPanel panel_2 = new WebPanel();
		panel_2.setLayout(null);
		panel_2.setWebColoredBackground(false);
		panel_2.setBackground(transparent);
		panel_2.setBounds(220, 0, 110, 36);
		topPanel.add(panel_2);

		label_6 = createWebLabel("1200");
		label_6.setHorizontalAlignment(SwingConstants.CENTER);
		label_6.setForeground(lblNumColor);
		label_6.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_6.setBounds(0, 0, 72, 36);
		panel_2.add(label_6);

		WebLabel label_7 = createWebLabel(lang.eThurst);
		label_7.setVerticalAlignment(SwingConstants.BOTTOM);
		label_7.setHorizontalAlignment(SwingConstants.LEFT);
		label_7.setForeground(lblNameColor);
		label_7.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_7.setBounds(72, 0, 36, 18);
		panel_2.add(label_7);

		WebLabel lblKgf = createWebLabel("Kgf");
		lblKgf.setVerticalAlignment(SwingConstants.TOP);
		lblKgf.setHorizontalAlignment(SwingConstants.LEFT);
		lblKgf.setForeground(lblColor);
		lblKgf.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblKgf.setBounds(72, 18, 36, 18);
		panel_2.add(lblKgf);

		// 轴功率
		WebPanel panel_3 = new WebPanel();
		panel_3.setLayout(null);
		panel_3.setWebColoredBackground(false);
		panel_3.setBackground(transparent);
		panel_3.setBounds(330, 0, 110, 36);
		topPanel.add(panel_3);

		label_9 = createWebLabel("1860");
		label_9.setHorizontalAlignment(SwingConstants.CENTER);
		label_9.setForeground(lblNumColor);
		label_9.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_9.setBounds(0, 0, 72, 36);
		panel_3.add(label_9);

		WebLabel label_10 = createWebLabel(lang.eEffPower);
		label_10.setVerticalAlignment(SwingConstants.BOTTOM);
		label_10.setHorizontalAlignment(SwingConstants.LEFT);
		label_10.setForeground(lblNameColor);
		label_10.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_10.setBounds(72, 0, 36, 18);
		panel_3.add(label_10);

		lblBhp_1 = createWebLabel("Hp");
		lblBhp_1.setVerticalAlignment(SwingConstants.TOP);
		lblBhp_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblBhp_1.setForeground(lblColor);
		lblBhp_1.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblBhp_1.setBounds(72, 18, 36, 18);
		panel_3.add(lblBhp_1);

		// 油量
		WebPanel panel_4 = new WebPanel();
		panel_4.setLayout(null);
		panel_4.setWebColoredBackground(false);
		panel_4.setBackground(transparent);
		panel_4.setBounds(0, 36, 110, 36);
		topPanel.add(panel_4);

		label_12 = createWebLabel("165");
		label_12.setHorizontalAlignment(SwingConstants.CENTER);
		label_12.setForeground(lblNumColor);
		label_12.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_12.setBounds(0, 0, 72, 36);
		panel_4.add(label_12);

		label_13 = createWebLabel(lang.eFuel);
		label_13.setVerticalAlignment(SwingConstants.BOTTOM);
		label_13.setHorizontalAlignment(SwingConstants.LEFT);
		label_13.setForeground(lblNameColor);
		label_13.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_13.setBounds(72, 0, 36, 18);
		panel_4.add(label_13);

		lblKg = createWebLabel("Kg");
		lblKg.setVerticalAlignment(SwingConstants.TOP);
		lblKg.setHorizontalAlignment(SwingConstants.LEFT);
		lblKg.setForeground(lblColor);
		lblKg.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblKg.setBounds(72, 18, 36, 18);
		panel_4.add(lblKg);

		// 转速
		WebPanel panel_5 = new WebPanel();
		panel_5.setLayout(null);
		panel_5.setWebColoredBackground(false);
		panel_5.setBackground(transparent);
		panel_5.setBounds(110, 36, 110, 36);
		topPanel.add(panel_5);

		label_15 = createWebLabel("3000");
		label_15.setHorizontalAlignment(SwingConstants.CENTER);
		label_15.setForeground(lblNumColor);
		label_15.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_15.setBounds(0, 0, 72, 36);
		panel_5.add(label_15);

		label_16 = createWebLabel(lang.eRPM);
		label_16.setVerticalAlignment(SwingConstants.BOTTOM);
		label_16.setHorizontalAlignment(SwingConstants.LEFT);
		label_16.setForeground(lblNameColor);
		label_16.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_16.setBounds(72, 0, 36, 18);
		panel_5.add(label_16);

		WebLabel lblRpm = createWebLabel("RPM");
		lblRpm.setVerticalAlignment(SwingConstants.TOP);
		lblRpm.setHorizontalAlignment(SwingConstants.LEFT);
		lblRpm.setForeground(lblColor);
		lblRpm.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblRpm.setBounds(72, 18, 36, 18);
		panel_5.add(lblRpm);

		WebPanel panel_6 = new WebPanel();
		panel_6.setLayout(null);
		panel_6.setWebColoredBackground(false);
		panel_6.setBackground(transparent);
		panel_6.setBounds(220, 36, 110, 36);
		topPanel.add(panel_6);

		label_18 = createWebLabel("110");
		label_18.setHorizontalAlignment(SwingConstants.CENTER);
		label_18.setForeground(lblNumColor);
		label_18.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_18.setBounds(0, 0, 72, 36);
		panel_6.add(label_18);

		WebLabel label_19 = createWebLabel(lang.eTemp);
		label_19.setVerticalAlignment(SwingConstants.BOTTOM);
		label_19.setHorizontalAlignment(SwingConstants.LEFT);
		label_19.setForeground(lblNameColor);
		label_19.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_19.setBounds(72, 0, 36, 18);
		panel_6.add(label_19);

		lblc_1 = createWebLabel("\u00B0C");
		lblc_1.setVerticalAlignment(SwingConstants.TOP);
		lblc_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblc_1.setForeground(lblColor);
		lblc_1.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblc_1.setBounds(72, 18, 36, 18);
		panel_6.add(lblc_1);

		WebPanel panel_7 = new WebPanel();
		panel_7.setLayout(null);
		panel_7.setWebColoredBackground(false);
		panel_7.setBackground(transparent);
		panel_7.setBounds(330, 36, 110, 36);
		topPanel.add(panel_7);

		label_21 = createWebLabel("90");
		label_21.setHorizontalAlignment(SwingConstants.CENTER);
		label_21.setForeground(lblNumColor);
		label_21.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_21.setBounds(0, 0, 72, 36);
		panel_7.add(label_21);

		WebLabel label_22 = createWebLabel(lang.eEff);
		label_22.setVerticalAlignment(SwingConstants.BOTTOM);
		label_22.setHorizontalAlignment(SwingConstants.LEFT);
		label_22.setForeground(lblNameColor);
		label_22.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_22.setBounds(72, 0, 36, 18);
		panel_7.add(label_22);

		WebLabel label_23 = createWebLabel("%");
		label_23.setVerticalAlignment(SwingConstants.TOP);
		label_23.setHorizontalAlignment(SwingConstants.LEFT);
		label_23.setForeground(lblColor);
		label_23.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		label_23.setBounds(72, 18, 36, 18);
		panel_7.add(label_23);

		WebPanel panel_8 = new WebPanel();
		panel_8.setLayout(null);
		panel_8.setWebColoredBackground(false);
		panel_8.setBackground(transparent);
		panel_8.setBounds(0, 72, 110, 36);
		topPanel.add(panel_8);

		label_24 = createWebLabel("20");
		label_24.setHorizontalAlignment(SwingConstants.CENTER);
		label_24.setForeground(lblNumColor);
		label_24.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_24.setBounds(0, 0, 72, 36);
		panel_8.add(label_24);

		label_25 = createWebLabel(lang.eFueltime);
		label_25.setVerticalAlignment(SwingConstants.BOTTOM);
		label_25.setHorizontalAlignment(SwingConstants.LEFT);
		label_25.setForeground(lblNameColor);
		label_25.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_25.setBounds(72, 0, 36, 18);
		panel_8.add(label_25);

		lblMin = createWebLabel("Min");
		lblMin.setVerticalAlignment(SwingConstants.TOP);
		lblMin.setHorizontalAlignment(SwingConstants.LEFT);
		lblMin.setForeground(lblColor);
		lblMin.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblMin.setBounds(72, 18, 36, 18);
		panel_8.add(lblMin);

		WebPanel panel_9 = new WebPanel();
		panel_9.setLayout(null);
		panel_9.setWebColoredBackground(false);
		panel_9.setBackground(transparent);
		panel_9.setBounds(110, 72, 110, 36);
		topPanel.add(panel_9);

		label_27 = createWebLabel("2.1");
		label_27.setHorizontalAlignment(SwingConstants.CENTER);
		label_27.setForeground(lblNumColor);
		label_27.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_27.setBounds(0, 0, 72, 36);
		panel_9.add(label_27);

		WebLabel label_28 = createWebLabel(lang.eATM);
		label_28.setVerticalAlignment(SwingConstants.BOTTOM);
		label_28.setHorizontalAlignment(SwingConstants.LEFT);
		label_28.setForeground(lblNameColor);
		label_28.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_28.setBounds(72, 0, 36, 18);
		panel_9.add(label_28);

		lblAta = createWebLabel("Ata");
		lblAta.setVerticalAlignment(SwingConstants.TOP);
		lblAta.setHorizontalAlignment(SwingConstants.LEFT);
		lblAta.setForeground(lblColor);
		lblAta.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblAta.setBounds(72, 18, 36, 18);
		panel_9.add(lblAta);

		WebPanel panel_10 = new WebPanel();
		panel_10.setLayout(null);
		panel_10.setWebColoredBackground(false);
		panel_10.setBackground(transparent);
		panel_10.setBounds(220, 72, 110, 36);
		topPanel.add(panel_10);

		label_30 = createWebLabel("100");
		label_30.setHorizontalAlignment(SwingConstants.CENTER);
		label_30.setForeground(lblNumColor);
		label_30.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_30.setBounds(0, 0, 72, 36);
		panel_10.add(label_30);

		WebLabel label_31 = createWebLabel(lang.eOil);
		label_31.setVerticalAlignment(SwingConstants.BOTTOM);
		label_31.setHorizontalAlignment(SwingConstants.LEFT);
		label_31.setForeground(lblNameColor);
		label_31.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_31.setBounds(72, 0, 36, 18);
		panel_10.add(label_31);

		lblc = createWebLabel("\u00B0C");
		lblc.setVerticalAlignment(SwingConstants.TOP);
		lblc.setHorizontalAlignment(SwingConstants.LEFT);
		lblc.setForeground(lblColor);
		lblc.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblc.setBounds(72, 18, 36, 18);
		panel_10.add(lblc);

		WebPanel panel_11 = new WebPanel();
		panel_11.setLayout(null);
		panel_11.setWebColoredBackground(false);
		panel_11.setBackground(transparent);
		panel_11.setBounds(330, 72, 110, 36);
		topPanel.add(panel_11);

		label_33 = createWebLabel("0");
		label_33.setHorizontalAlignment(SwingConstants.CENTER);
		label_33.setForeground(lblNumColor);
		label_33.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_33.setBounds(0, 0, 72, 36);
		panel_11.add(label_33);

		WebLabel label_34 = createWebLabel(lang.eOverheat);
		label_34.setVerticalAlignment(SwingConstants.BOTTOM);
		label_34.setHorizontalAlignment(SwingConstants.LEFT);
		label_34.setForeground(lblNameColor);
		label_34.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_34.setBounds(72, 0, 36, 18);
		panel_11.add(label_34);

		WebLabel lblS = createWebLabel("Sec");
		lblS.setVerticalAlignment(SwingConstants.TOP);
		lblS.setHorizontalAlignment(SwingConstants.LEFT);
		lblS.setForeground(lblColor);
		lblS.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblS.setBounds(72, 18, 36, 18);
		panel_11.add(lblS);

		// WebSeparator separator1 = new WebSeparator();
		// separator1.setBounds(0, 0, 1024, 1);
		// topPanel.add(separator1);

		// WebSeparator separator2 = new WebSeparator();
		// separator2.setOrientation(SwingConstants.VERTICAL);
		// separator2.setBounds(471, -50, 1, 256);
		// topPanel.add(separator2);

		// WebSeparator separator3 = new WebSeparator();
		// separator3.setBounds(0, 100, 1024, 1);
		// topPanel.add(separator3);
		this.setCursor(app.blankCursor);
	}

	public void initPanelText22() {
		topPanel.setLayout(null);

		WebPanel panel_1 = new WebPanel();
		panel_1.setLayout(null);
		panel_1.setWebColoredBackground(false);
		panel_1.setBackground(transparent);
		panel_1.setBounds(0, 0, 110, 54);
		topPanel.add(panel_1);

		lblFwd = createWebLabel("Fw-190D");
		lblFwd.setHorizontalAlignment(SwingConstants.CENTER);
		lblFwd.setForeground(lblNameColor);
		lblFwd.setFont(new Font(NumFont, Font.PLAIN, 16 + fontadd));
		lblFwd.setBounds(0, 0, 110, 54);
		panel_1.add(lblFwd);

		// WebLabel label = createWebLabel(language.eType);
		// label.setBounds(72, 0, 36, 18);
		// panel_1.add(label);
		// label.setVerticalAlignment(SwingConstants.BOTTOM);
		// label.setHorizontalAlignment(SwingConstants.LEFT);
		// label.setForeground(lblNumColor);
		// label.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		//
		WebLabel label_2 = createWebLabel("");
		label_2.setBounds(72, 18, 36, 27);
		panel_1.add(label_2);
		label_2.setVerticalAlignment(SwingConstants.TOP);
		label_2.setHorizontalAlignment(SwingConstants.LEFT);
		label_2.setForeground(lblColor);
		label_2.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));

		// 功率
		WebPanel panel = new WebPanel();
		panel.setLayout(null);
		panel.setWebColoredBackground(false);
		panel.setBackground(transparent);
		panel.setBounds(110, 0, 110, 54);
		topPanel.add(panel);

		label_3 = createWebLabel("2050");
		label_3.setHorizontalAlignment(SwingConstants.CENTER);
		label_3.setForeground(lblNumColor);
		label_3.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_3.setBounds(0, 0, 72, 54);
		panel.add(label_3);

		WebLabel label_4 = createWebLabel(lang.ePower);
		label_4.setVerticalAlignment(SwingConstants.BOTTOM);
		label_4.setHorizontalAlignment(SwingConstants.LEFT);
		label_4.setForeground(lblNameColor);
		label_4.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_4.setBounds(72, 0, 36, 27);
		panel.add(label_4);

		lblBhp = createWebLabel("Hp");
		lblBhp.setVerticalAlignment(SwingConstants.TOP);
		lblBhp.setHorizontalAlignment(SwingConstants.LEFT);
		lblBhp.setForeground(lblColor);
		lblBhp.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblBhp.setBounds(72, 27, 36, 27);
		panel.add(lblBhp);

		// 推力
		WebPanel panel_2 = new WebPanel();
		panel_2.setLayout(null);
		panel_2.setWebColoredBackground(false);
		panel_2.setBackground(transparent);
		panel_2.setBounds(220, 0, 110, 36);
		topPanel.add(panel_2);

		label_6 = createWebLabel("1200");
		label_6.setHorizontalAlignment(SwingConstants.CENTER);
		label_6.setForeground(lblNumColor);
		label_6.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_6.setBounds(0, 0, 72, 36);
		panel_2.add(label_6);

		WebLabel label_7 = createWebLabel(lang.eThurst);
		label_7.setVerticalAlignment(SwingConstants.BOTTOM);
		label_7.setHorizontalAlignment(SwingConstants.LEFT);
		label_7.setForeground(lblNameColor);
		label_7.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_7.setBounds(72, 0, 36, 18);
		panel_2.add(label_7);

		WebLabel lblKgf = createWebLabel("Kgf");
		lblKgf.setVerticalAlignment(SwingConstants.TOP);
		lblKgf.setHorizontalAlignment(SwingConstants.LEFT);
		lblKgf.setForeground(lblColor);
		lblKgf.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblKgf.setBounds(72, 18, 36, 18);
		panel_2.add(lblKgf);

		// 轴功率
		WebPanel panel_3 = new WebPanel();
		panel_3.setLayout(null);
		panel_3.setWebColoredBackground(false);
		panel_3.setBackground(transparent);
		panel_3.setBounds(330, 0, 110, 36);
		topPanel.add(panel_3);

		label_9 = createWebLabel("1860");
		label_9.setHorizontalAlignment(SwingConstants.CENTER);
		label_9.setForeground(lblNumColor);
		label_9.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_9.setBounds(0, 0, 72, 36);
		panel_3.add(label_9);

		WebLabel label_10 = createWebLabel(lang.eEffPower);
		label_10.setVerticalAlignment(SwingConstants.BOTTOM);
		label_10.setHorizontalAlignment(SwingConstants.LEFT);
		label_10.setForeground(lblNameColor);
		label_10.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_10.setBounds(72, 0, 36, 18);
		panel_3.add(label_10);

		lblBhp_1 = createWebLabel("Hp");
		lblBhp_1.setVerticalAlignment(SwingConstants.TOP);
		lblBhp_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblBhp_1.setForeground(lblColor);
		lblBhp_1.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblBhp_1.setBounds(72, 18, 36, 18);
		panel_3.add(lblBhp_1);

		// 油量
		WebPanel panel_4 = new WebPanel();
		panel_4.setLayout(null);
		panel_4.setWebColoredBackground(false);
		panel_4.setBackground(transparent);
		panel_4.setBounds(0, 54, 110, 54);
		topPanel.add(panel_4);

		label_12 = createWebLabel("165");
		label_12.setHorizontalAlignment(SwingConstants.CENTER);
		label_12.setForeground(lblNumColor);
		label_12.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_12.setBounds(0, 0, 72, 54);
		panel_4.add(label_12);

		label_13 = createWebLabel(lang.eFuel);
		label_13.setVerticalAlignment(SwingConstants.BOTTOM);
		label_13.setHorizontalAlignment(SwingConstants.LEFT);
		label_13.setForeground(lblNameColor);
		label_13.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_13.setBounds(72, 0, 36, 27);
		panel_4.add(label_13);

		lblKg = createWebLabel("Kg");
		lblKg.setVerticalAlignment(SwingConstants.TOP);
		lblKg.setHorizontalAlignment(SwingConstants.LEFT);
		lblKg.setForeground(lblColor);
		lblKg.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblKg.setBounds(72, 27, 36, 27);
		panel_4.add(lblKg);

		// 转速
		WebPanel panel_5 = new WebPanel();
		panel_5.setLayout(null);
		panel_5.setWebColoredBackground(false);
		panel_5.setBackground(transparent);
		panel_5.setBounds(110, 36, 110, 36);
		topPanel.add(panel_5);

		label_15 = createWebLabel("3000");
		label_15.setHorizontalAlignment(SwingConstants.CENTER);
		label_15.setForeground(lblNumColor);
		label_15.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_15.setBounds(0, 0, 72, 36);
		panel_5.add(label_15);

		label_16 = createWebLabel(lang.eRPM);
		label_16.setVerticalAlignment(SwingConstants.BOTTOM);
		label_16.setHorizontalAlignment(SwingConstants.LEFT);
		label_16.setForeground(lblNameColor);
		label_16.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_16.setBounds(72, 0, 36, 18);
		panel_5.add(label_16);

		WebLabel lblRpm = createWebLabel("RPM");
		lblRpm.setVerticalAlignment(SwingConstants.TOP);
		lblRpm.setHorizontalAlignment(SwingConstants.LEFT);
		lblRpm.setForeground(lblColor);
		lblRpm.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblRpm.setBounds(72, 18, 36, 18);
		panel_5.add(lblRpm);

		WebPanel panel_6 = new WebPanel();
		panel_6.setLayout(null);
		panel_6.setWebColoredBackground(false);
		panel_6.setBackground(transparent);
		panel_6.setBounds(220, 36, 110, 36);
		topPanel.add(panel_6);

		label_18 = createWebLabel("110");
		label_18.setHorizontalAlignment(SwingConstants.CENTER);
		label_18.setForeground(lblNumColor);
		label_18.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_18.setBounds(0, 0, 72, 36);
		panel_6.add(label_18);

		WebLabel label_19 = createWebLabel(lang.eTemp);
		label_19.setVerticalAlignment(SwingConstants.BOTTOM);
		label_19.setHorizontalAlignment(SwingConstants.LEFT);
		label_19.setForeground(lblNameColor);
		label_19.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_19.setBounds(72, 0, 36, 18);
		panel_6.add(label_19);

		lblc_1 = createWebLabel("\u00B0C");
		lblc_1.setVerticalAlignment(SwingConstants.TOP);
		lblc_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblc_1.setForeground(lblColor);
		lblc_1.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblc_1.setBounds(72, 18, 36, 18);
		panel_6.add(lblc_1);

		WebPanel panel_7 = new WebPanel();
		panel_7.setLayout(null);
		panel_7.setWebColoredBackground(false);
		panel_7.setBackground(transparent);
		panel_7.setBounds(330, 36, 110, 36);
		topPanel.add(panel_7);

		label_21 = createWebLabel("90");
		label_21.setHorizontalAlignment(SwingConstants.CENTER);
		label_21.setForeground(lblNumColor);
		label_21.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_21.setBounds(0, 0, 72, 36);
		panel_7.add(label_21);

		WebLabel label_22 = createWebLabel(lang.eEff);
		label_22.setVerticalAlignment(SwingConstants.BOTTOM);
		label_22.setHorizontalAlignment(SwingConstants.LEFT);
		label_22.setForeground(lblNameColor);
		label_22.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_22.setBounds(72, 0, 36, 18);
		panel_7.add(label_22);

		WebLabel label_23 = createWebLabel("%");
		label_23.setVerticalAlignment(SwingConstants.TOP);
		label_23.setHorizontalAlignment(SwingConstants.LEFT);
		label_23.setForeground(lblColor);
		label_23.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		label_23.setBounds(72, 18, 36, 18);
		panel_7.add(label_23);

		WebPanel panel_8 = new WebPanel();
		panel_8.setLayout(null);
		panel_8.setWebColoredBackground(false);
		panel_8.setBackground(transparent);
		panel_8.setBounds(0, 72, 110, 36);
		topPanel.add(panel_8);

		label_24 = createWebLabel("20");
		label_24.setHorizontalAlignment(SwingConstants.CENTER);
		label_24.setForeground(lblNumColor);
		label_24.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_24.setBounds(0, 0, 72, 36);
		panel_8.add(label_24);

		label_25 = createWebLabel(lang.eFueltime);
		label_25.setVerticalAlignment(SwingConstants.BOTTOM);
		label_25.setHorizontalAlignment(SwingConstants.LEFT);
		label_25.setForeground(lblNameColor);
		label_25.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_25.setBounds(72, 0, 36, 18);
		panel_8.add(label_25);

		lblMin = createWebLabel("Min");
		lblMin.setVerticalAlignment(SwingConstants.TOP);
		lblMin.setHorizontalAlignment(SwingConstants.LEFT);
		lblMin.setForeground(lblColor);
		lblMin.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblMin.setBounds(72, 18, 36, 18);
		panel_8.add(lblMin);

		WebPanel panel_9 = new WebPanel();
		panel_9.setLayout(null);
		panel_9.setWebColoredBackground(false);
		panel_9.setBackground(transparent);
		panel_9.setBounds(110, 72, 110, 36);
		topPanel.add(panel_9);

		label_27 = createWebLabel("2.1");
		label_27.setHorizontalAlignment(SwingConstants.CENTER);
		label_27.setForeground(lblNumColor);
		label_27.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_27.setBounds(0, 0, 72, 36);
		panel_9.add(label_27);

		WebLabel label_28 = createWebLabel(lang.eATM);
		label_28.setVerticalAlignment(SwingConstants.BOTTOM);
		label_28.setHorizontalAlignment(SwingConstants.LEFT);
		label_28.setForeground(lblNameColor);
		label_28.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_28.setBounds(72, 0, 36, 18);
		panel_9.add(label_28);

		lblAta = createWebLabel("Ata");
		lblAta.setVerticalAlignment(SwingConstants.TOP);
		lblAta.setHorizontalAlignment(SwingConstants.LEFT);
		lblAta.setForeground(lblColor);
		lblAta.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblAta.setBounds(72, 18, 36, 18);
		panel_9.add(lblAta);

		WebPanel panel_10 = new WebPanel();
		panel_10.setLayout(null);
		panel_10.setWebColoredBackground(false);
		panel_10.setBackground(transparent);
		panel_10.setBounds(220, 72, 110, 36);
		topPanel.add(panel_10);

		label_30 = createWebLabel("100");
		label_30.setHorizontalAlignment(SwingConstants.CENTER);
		label_30.setForeground(lblNumColor);
		label_30.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_30.setBounds(0, 0, 72, 36);
		panel_10.add(label_30);

		WebLabel label_31 = createWebLabel(lang.eOil);
		label_31.setVerticalAlignment(SwingConstants.BOTTOM);
		label_31.setHorizontalAlignment(SwingConstants.LEFT);
		label_31.setForeground(lblNameColor);
		label_31.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_31.setBounds(72, 0, 36, 18);
		panel_10.add(label_31);

		lblc = createWebLabel("\u00B0C");
		lblc.setVerticalAlignment(SwingConstants.TOP);
		lblc.setHorizontalAlignment(SwingConstants.LEFT);
		lblc.setForeground(lblColor);
		lblc.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblc.setBounds(72, 18, 36, 18);
		panel_10.add(lblc);

		WebPanel panel_11 = new WebPanel();
		panel_11.setLayout(null);
		panel_11.setWebColoredBackground(false);
		panel_11.setBackground(transparent);
		panel_11.setBounds(330, 72, 110, 36);
		topPanel.add(panel_11);

		label_33 = createWebLabel("0");
		label_33.setHorizontalAlignment(SwingConstants.CENTER);
		label_33.setForeground(lblNumColor);
		label_33.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_33.setBounds(0, 0, 72, 36);
		panel_11.add(label_33);

		WebLabel label_34 = createWebLabel(lang.eOverheat);
		label_34.setVerticalAlignment(SwingConstants.BOTTOM);
		label_34.setHorizontalAlignment(SwingConstants.LEFT);
		label_34.setForeground(lblNameColor);
		label_34.setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		label_34.setBounds(72, 0, 36, 18);
		panel_11.add(label_34);

		WebLabel lblS = createWebLabel("Sec");
		lblS.setVerticalAlignment(SwingConstants.TOP);
		lblS.setHorizontalAlignment(SwingConstants.LEFT);
		lblS.setForeground(lblColor);
		lblS.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblS.setBounds(72, 18, 36, 18);
		panel_11.add(lblS);

		// WebSeparator separator1 = new WebSeparator();
		// separator1.setBounds(0, 0, 1024, 1);
		// topPanel.add(separator1);

		// WebSeparator separator2 = new WebSeparator();
		// separator2.setOrientation(SwingConstants.VERTICAL);
		// separator2.setBounds(471, -50, 1, 256);
		// topPanel.add(separator2);

		// WebSeparator separator3 = new WebSeparator();
		// separator3.setBounds(0, 100, 1024, 1);
		// topPanel.add(separator3);
		this.setCursor(app.blankCursor);
	}

	public void init(controller xc, service ts, blkx tp) {
		this.c = xc;
		this.s = ts;
		this.p = tp;

		overheattime = 0;
		freq = xc.freqEngineInfo;

		if (xc.getconfig("GlobalNumFont") != "")
			NumFont = xc.getconfig("GlobalNumFont");
		else
			NumFont = app.defaultNumfontName;

		if (xc.getconfig("engineInfoFont") != "")
			FontName = xc.getconfig("engineInfoFont");
		else
			FontName = app.defaultFont.getFontName();
		if (xc.getconfig("engineInfoFontadd") != "")
			fontadd = Integer.parseInt(xc.getconfig("engineInfoFontadd"));
		else
			fontadd = 0;
		// app.debugPrint(fontadd);
		font1 = font1 + fontadd;
		font2 = font2 + fontadd;
		font3 = font3 + fontadd;
		if (xc.getconfig("engineInfoX") != "")
			lx = Integer.parseInt(xc.getconfig("engineInfoX"));
		else
			lx = 0;
		if (xc.getconfig("engineInfoY") != "")
			ly = Integer.parseInt(xc.getconfig("engineInfoY"));
		else
			ly = 860;

		WIDTH = 640;
		HEIGHT = 210;
		// OP = 100;

		setSize(WIDTH, HEIGHT);
		setLocation(lx, ly);
		// setIconImage(Toolkit.getDefaultToolkit().createImage("image/form1.jpg"));

		WebPanel panel = new WebPanel();

		this.getWebRootPaneUI().setMiddleBg(transparent);// 中部透明
		this.getWebRootPaneUI().setTopBg(transparent);// 顶部透明
		this.getWebRootPaneUI().setBorderColor(transparent);// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(transparent);// 外描边透明

		// this.setUndecorated(true);

		panel.setBorderColor(transparent);

		// 初始化Panel
		initPanel();

		// Split
		WebSplitPane splitPane2 = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane2.setOneTouchExpandable(false);
		// splitPane.setPreferredSize ( new Dimension ( 250, 200 ) );
		splitPane2.setDividerLocation(120);
		splitPane2.setContinuousLayout(false);
		splitPane2.setDividerSize(0);
		splitPane2.setEnabled(false);
		splitPane2.setDrawDividerBorder(false);

		// Split
		splitPane = new WebSplitPane(HORIZONTAL_SPLIT, leftPanel, rightPanel);
		splitPane.setOneTouchExpandable(false);
		splitPane.setDrawDividerBorder(false);
		splitPane.setDividerSize(0);
		splitPane.setEnabled(false);
		// splitPane.setPreferredSize ( new Dimension ( 250, 200 ) );
		splitPane.setDividerLocation(130);
		splitPane.setContinuousLayout(true);
		splitPane.setDividerBorderColor(transparent);

		panel.setLayout(new GridLayout(1, 2));
		// 标签

		// panel透明
		panel.setWebColoredBackground(false);
		panel.setBackground(transparent);

		// 添加绘制物件至panel
		rightPanel.add(splitPane2);
		panel.add(splitPane);
		this.add(panel);

		WebSeparator separator1 = new WebSeparator();
		separator1.setSeparatorLightColor(lblNumColor);
		// separator1.setBounds(200, 116, 800, 1);
		separator1.setBounds(200, 5, 800, 1);
		// this.getRootPane().add(separator1);

		WebSeparator separator2 = new WebSeparator();
		separator2.setOrientation(SwingConstants.VERTICAL);
		separator2.setSeparatorLightColor(lblNumColor);
		separator2.setSeparatorLightUpperColor(lblNumColor);
		// separator2.setSeparatorColor(Color.RED);
		separator2.setBounds(660, -64, 1, 200);
		// separator2.setBounds(678, -30, 1, 256);

		// this.getRootPane().add(separator2);

		// this.setOpacity((double) (OP / 100.0f));

		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);

		setDefaultCloseOperation(3);
		setTitle(lang.eTitle);
		getWebRootPaneUI().getTitleComponent().getComponent(1).setFont(app.defaultFont);// 设置title字体
		setAlwaysOnTop(true);
		setFocusableWindowState(false);// 取消窗口焦点
		setFocusable(false);
		setVisible(true);
		jetChecked = false;
		// this.setCloseOnFocusLoss(true);

		if (xc.getconfig("engineInfoEdge").equals("true"))
			setShadeWidth(10);// 玻璃效果边框
		else {
			setShadeWidth(0);
			// this.getRootPane().add(separator1);
			// this.getRootPane().add(separator2);
		}
	}

	public void wspsetStep(WebStepProgress wsp, int step) {
		if (step > wsp.getStepsAmount()) {
			wsp.addSteps("");
			// app.debugPrint("成功");
			// wsp.setPathWidth(5);
		}

		wsp1.setSelectedStepIndex(step - 1);
	}

	public void updateOverheatTime(int Time) {
		overheattime = Time;
		label_33.setText(Integer.toString(overheattime));
	}

	void setValueold() {

		bottomType.setText(s.sIndic.stype);
		bottomThr.setText(s.sTotalThr);
		bottomRpm.setText(s.rpm);

		bottomHp.setText(s.sTotalHp);
		bottomEhp.setText(s.sTotalHpEff);
		bottomEff.setText(s.efficiency[0]);

		bottomEng.setText(s.watertemp + "/" + p.wtload1);
		bottomOil.setText(s.oiltemp + "/" + p.oilload1);

		bottomPre.setText(s.manifoldpressure);

		bottomWei.setText(s.sTotalFuel);
		bottomTim.setText(s.sfueltime);
	}

	long engineCheckMili;
	private boolean isJet;
	private boolean jetChecked;

	@Override
	public void run() {
		while (doit) {

			try {
				Thread.sleep(app.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}

			if (s.SystemTime - engineCheckMili > c.freqService) {
				engineCheckMili = s.SystemTime;
				if (s.sState != null) {
					if (jetChecked == false) {
						if (c.blkx != null && c.blkx.valid == true) {
							if (c.blkx.isJet) {
								isJet = true;
								slider1.setVisible(false);
								slider2.setVisible(false);
								slider4.setVisible(false);
								wsp1.setVisible(false);
								// label_16.setVisible(false);
								lefttitle1.setVisible(false);
								lefttitle.setVisible(false);
								bottomtitle1.setVisible(false);
								bottomtitle.setVisible(false);

								// 油门设置
								lefttitle2.setText(lang.eThrottle);
								slider3.setMaximum(110);
							}
							jetChecked = true;
						}
					}
					if (isJet) {
						slider3.setValue(s.sState.throttle);
					} else {
						slider1.setValue(s.sState.throttle);
						slider2.setValue(s.sState.RPMthrottle);
						slider3.setValue(s.sState.mixture);
						slider4.setValue(s.sState.radiator);

						// slider5.setValue(s.sState.mfuel1);
						wspsetStep(wsp1, s.sState.compressorstage);
					}
					// wsp2.setSelectedStepIndex(s.sState.magenato);

					lblFwd.setText(s.sIndic.stype);
					label_3.setText(s.sTotalHp);
					label_6.setText(s.sTotalThr);

					label_9.setText(s.sTotalHpEff);

					// if(!s.efficiency[0].equals("0"))
					// lblBhp_1.setText("/"+s.efficiency[0]+"%");

					label_15.setText(s.rpm);

					label_18.setText(s.watertemp);
					label_30.setText(s.oiltemp);

					label_21.setText(s.efficiency[0]);
					// label_33.setText(String.valueOf((int) c.weppft));

					// else{
					//
					// label_12.setText(s.stotalfuel);
					// label_24.setText(s.sfueltime);
					// }
					label_12.setText(s.sTotalFuel);

					// if(c.blkx.nitro != 0){
					//// label_25.setText(language.eFueltime)
					// label_24.setText(s.sfueltime+"/"+s.sWepTime);
					// }
					// else
					label_24.setText(s.sfueltime);
					if (c.blkx.maxEngLoad != 0) {
						lblc_1.setText("/" + (int) c.blkx.engLoad[s.curLoad].WaterLimit);
						lblc.setText("/" + (int) c.blkx.engLoad[s.curLoad].OilLimit);
						label_33.setText(s.sEngWorkTime);
					} else {
						lblc_1.setText("\u00B0C");
						lblc.setText("\u00B0C");
					}
					// if (s.isFuelpressure) {
					// label_13.setText(lang.eFuelPrs);
					// lblKg.setText("%");
					// } else {
					if (c.blkx.nitro != 0) {
						label_13.setText(lang.eFuelP);
						lblKg.setText("/" + s.sNitro + "Kg");
						lblMin.setText("/" + s.sWepTime + "Min");
					} else {
						label_13.setText(lang.eFuel);
						lblKg.setText("Kg");
						lblMin.setText("Min");
					}
					// }

					if (s.iCheckAlt > 0) {
						label_27.setText(s.pressurePounds);
						lblAta.setText("P " + s.pressureInchHg + "''");
					} else {
						label_27.setText(s.manifoldpressure);
						lblAta.setText("Ata ");
					}

					// bottomPro.setText(s.pitch[0]);
					// app.debugPrint("engineInfo执行了");
					// repaint();
					this.getContentPane().repaint();
					// this.getContentPane().getGraphics().dispose();
					// this.status="infolist就绪";
					// app.debugPrint(this.status);
					// 绘图

					// this.status="infolist等待";
					// app.debugPrint(this.status);

				}
			}
		}
	}
}