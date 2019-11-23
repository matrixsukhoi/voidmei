package ui;
import java.awt.Color;
import java.awt.Font;
import java.awt.GraphicsConfiguration;
import java.awt.GraphicsDevice;
import java.awt.GraphicsEnvironment;
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

import prog.app;
import prog.controller;
import prog.language;
import prog.service;

import static javax.swing.JSplitPane.HORIZONTAL_SPLIT;
import static com.alee.laf.splitpane.WebSplitPane.VERTICAL_SPLIT;
public class engineInfo extends WebFrame implements Runnable {
	/**
	 * 
	 */
	
	int isDragging = 0;
	int xx=0;
	int yy=0;
	public volatile boolean doit=true;
	WebSplitPane splitPane;
	public int overheattime;
	private static final long serialVersionUID = 3063042782594625576L;
	public controller c;
	public service s;
	public String status;
	String NumFont;
	int freq;
	long MainCheckMili;
	int WIDTH;
	int HEIGHT;
	int lx;
	int ly;
	int OP;
	Color transParentWhite = new Color(255, 255, 255, 100);
	WebPanel panel;
	WebComponentPanel webComponentPanel;
	WebProgressBar progressBar1;
	WebSlider slider1;
	WebSlider slider2;
	WebSlider slider3;
	WebSlider slider4;
	WebStepProgress wsp1;
	WebStepProgress wsp2;
	WebLabel lblAta;
	WebPanel topPanel;
	WebPanel bottomPanel;
	WebPanel leftPanel;
	WebPanel rightPanel;
	WebLabel lefttitle;
	WebLabel bottomtitle;

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
	WebLabel lblFwd ;
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
	int fontadd=0;
	
	WebLabel createWebLabel(String text){
		WebLabel x=new WebLabel(text);
		x.setDrawShade(true);
		x.setForeground(new Color(245, 248, 250, 240));
		x.setShadeColor(Color.BLACK);
		return x;
	}
	void initLabel(WebLabel x, int font) {

		x.setHorizontalAlignment(SwingConstants.CENTER);
		x.setDrawShade(true);
		x.setForeground(new Color(245, 248, 250, 240));
		x.setShadeColor(Color.BLACK);
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
		bottomType.setForeground(new Color(245, 248, 250, 240));
		bottomType.setShadeColor(Color.BLACK);
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
	
		s.fill=GridBagConstraints.BASELINE;
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
		topPanel.setBackground(new Color(0, 0, 0, 0));
		topPanel.setBorderColor(new Color(0, 0, 0, 0));

		// Bottom part content
		bottomPanel = new WebPanel(true);
		bottomPanel.setWebColoredBackground(false);
		bottomPanel.setShadeWidth(0);
		bottomPanel.setBackground(new Color(0, 0, 0, 0));
		bottomPanel.setBorderColor(new Color(0, 0, 0, 0));

		// Left part content
		leftPanel = new WebPanel(true);
		leftPanel.setWebColoredBackground(false);
		leftPanel.setShadeWidth(0);
		leftPanel.setBackground(new Color(0, 0, 0, 0));
		leftPanel.setBorderColor(new Color(0, 0, 0, 0));

		// Right part content
		rightPanel = new WebPanel(true);

		rightPanel.setWebColoredBackground(false);
		rightPanel.setShadeWidth(0);
		rightPanel.setBackground(new Color(0, 0, 0, 0));
		rightPanel.setBorderColor(new Color(0, 0, 0, 0));

		// 左panel控件
		lefttitle = new WebLabel(language.eThrottle);
		lefttitle.setHorizontalAlignment(SwingConstants.CENTER);
		lefttitle.setDrawShade(true);
		lefttitle.setForeground(new Color(245, 248, 250, 240));
		lefttitle.setShadeColor(Color.BLACK);
		lefttitle.setFont(new Font(FontName, Font.PLAIN, font1));

		
		WebLabel lefttitle1 = new WebLabel(language.eProppitch);
		lefttitle1.setHorizontalAlignment(SwingConstants.CENTER);
		lefttitle1.setDrawShade(true);
		lefttitle1.setForeground(new Color(245, 248, 250, 240));
		lefttitle1.setShadeColor(Color.BLACK);
		lefttitle1.setFont(new Font(FontName, Font.PLAIN, font1));
		
		WebLabel lefttitle2 = new WebLabel(language.eMixture);
		lefttitle2.setHorizontalAlignment(SwingConstants.CENTER);
		lefttitle2.setDrawShade(true);
		lefttitle2.setForeground(new Color(245, 248, 250, 240));
		lefttitle2.setShadeColor(Color.BLACK);
		lefttitle2.setFont(new Font(FontName, Font.PLAIN, font1));


		
		slider1 = new WebSlider(WebSlider.VERTICAL);
		slider1.setMinimum(0);
		slider1.setMaximum(110);
		slider1.setDrawProgress(true);
		slider1.setMinorTickSpacing(5);
		slider1.setMajorTickSpacing(50);
		slider1.setPaintTicks(true);
		slider1.setPaintLabels(true);
		slider1.setPreferredHeight(120);
		// slider1.setSnapToTicks(true);
		slider1.setProgressShadeWidth(0);
		slider1.setTrackShadeWidth(1);
		// slider1.setDrawThumb(false);
		slider1.setThumbShadeWidth(2);
		slider1.setThumbBgBottom(new Color(0, 0, 0, 0));
		slider1.setThumbBgTop(new Color(0, 0, 0, 0));
		slider1.setTrackBgBottom(new Color(0, 0, 0, 0));
		slider1.setTrackBgTop(new Color(0, 0, 0, 0));
		slider1.setProgressBorderColor(new Color(0, 0, 0, 0));
		slider1.setProgressTrackBgBottom(transParentWhite);
		slider1.setProgressTrackBgTop(transParentWhite);
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
		slider2.setMinorTickSpacing(5);
		slider2.setMajorTickSpacing(50);
		slider2.setPaintTicks(true);
		slider2.setPreferredHeight(120);
		slider2.setPaintLabels(true);
		// slider1.setSnapToTicks(true);
		slider2.setProgressShadeWidth(0);
		slider2.setTrackShadeWidth(1);
		// slider1.setDrawThumb(false);
		slider2.setThumbShadeWidth(2);
		slider2.setThumbBgBottom(new Color(0, 0, 0, 0));
		slider2.setThumbBgTop(new Color(0, 0, 0, 0));
		slider2.setTrackBgBottom(new Color(0, 0, 0, 0));
		slider2.setTrackBgTop(new Color(0, 0, 0, 0));
		slider2.setProgressBorderColor(new Color(0, 0, 0, 0));
		slider2.setProgressTrackBgBottom(transParentWhite);
		slider2.setProgressTrackBgTop(transParentWhite);
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
		// slider1.setSnapToTicks(true);
		slider3.setProgressShadeWidth(0);
		slider3.setTrackShadeWidth(1);
		// slider1.setDrawThumb(false);
		slider3.setThumbShadeWidth(2);
		slider3.setThumbBgBottom(new Color(0, 0, 0, 0));
		slider3.setThumbBgTop(new Color(0, 0, 0, 0));
		slider3.setTrackBgBottom(new Color(0, 0, 0, 0));
		slider3.setTrackBgTop(new Color(0, 0, 0, 0));
		slider3.setProgressBorderColor(new Color(0, 0, 0, 0));
		slider3.setProgressTrackBgBottom(transParentWhite);
		slider3.setProgressTrackBgTop(transParentWhite);
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
		 * progressBar1.setForeground(new Color(0,0,0,0));
		 * progressBar1.setBgBottom(new Color(0, 0, 0, 0));
		 * progressBar1.setBgTop(new Color(0, 0, 0, 0)); //
		 * progressBar1.setBackground(new Color(0,0,0));
		 * progressBar1.setProgressBottomColor(new Color(255, 255, 255, 255));
		 * progressBar1.setProgressTopColor(new Color(255, 255, 255, 255));
		 * progressBar1.setForeground(new Color(245, 248, 250));
		 * progressBar1.setBorderPainted(true); progressBar1.setShadeWidth(1);
		 * // progressBar1.setHighlightDarkWhite(new Color(255,255,255,0));
		 * progressBar1.setFont(new Font(FontName, Font.PLAIN, 12));
		 * progressBar1.setHighlightDarkWhite(new Color(0, 0, 0));
		 * progressBar1.setHighlightWhite(new Color(0, 0, 0));
		 */
		// 下panel
		bottomtitle = new WebLabel(language.eCompressor);
		bottomtitle.setHorizontalAlignment(SwingConstants.CENTER);
		bottomtitle.setDrawShade(true);
		bottomtitle.setForeground(new Color(245, 248, 250, 240));
		bottomtitle.setShadeColor(Color.BLACK);
		bottomtitle.setFont(new Font(FontName, Font.PLAIN, font1));
		
		WebLabel bottomtitle1 = new WebLabel(language.eRadiator);
		bottomtitle1.setHorizontalAlignment(SwingConstants.CENTER);
		bottomtitle1.setDrawShade(true);
		bottomtitle1.setForeground(new Color(245, 248, 250, 240));
		bottomtitle1.setShadeColor(Color.BLACK);
		bottomtitle1.setFont(new Font(FontName, Font.PLAIN, font1));
		
		WebLabel bottomtitle2 = new WebLabel(language.eMagneto);
		bottomtitle2.setHorizontalAlignment(SwingConstants.CENTER);
		bottomtitle2.setDrawShade(true);
		bottomtitle2.setForeground(new Color(245, 248, 250, 240));
		bottomtitle2.setShadeColor(Color.BLACK);
		bottomtitle2.setFont(new Font(FontName, Font.PLAIN, font1));


		slider4 = new WebSlider(WebSlider.HORIZONTAL);
		slider4.setMinimum(0);
		slider4.setMaximum(100);
		slider4.setDrawProgress(true);
		slider4.setPaintTicks(false);
		slider4.setPaintLabels(false);

		wsp1 = new WebStepProgress(1);
		wsp1.setPreferredWidth(200);
		wsp1.setSelectedStepIndex(0);
		wsp1.setShadeWidth(0);
		wsp1.setPathWidth(0);
		wsp1.setStepControlWidth(0);
		wsp1.setDisplayLabels(false);
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

		//slider4.setSnapToTicks(true);

		slider4.setPreferredWidth(100);
		slider4.setProgressShadeWidth(0);
		slider4.setTrackShadeWidth(1);
		slider4.setDrawThumb(true);
		slider4.setSharpThumbAngle(true);
		slider4.setThumbShadeWidth(2);
		slider4.setThumbBgBottom(new Color(0, 0, 0, 0));
		slider4.setThumbBgTop(new Color(0, 0, 0, 0));
		slider4.setTrackBgBottom(new Color(0, 0, 0, 0));
		slider4.setTrackBgTop(new Color(0, 0, 0, 0));
		slider4.setProgressBorderColor(new Color(0, 0, 0, 0));
		slider4.setProgressTrackBgBottom(transParentWhite);
		slider4.setProgressTrackBgTop(transParentWhite);
		slider4.setFocusable(false); // 取消slider4响应
		MouseListener[] mls4 = slider4.getMouseListeners();
		MouseMotionListener[] mmls4 = slider4.getMouseMotionListeners();
		for (int t = 0; t < mls4.length; t++) {
			slider4.removeMouseListener(mls4[t]);

		}
		for (int t = 0; t < mmls4.length; t++) {
			slider4.removeMouseMotionListener(mmls4[t]);
		}

		// 布局

		// 左Panel
		GridBagLayout G=new GridBagLayout();
		GridBagConstraints s = new GridBagConstraints();// 定义一个GridBagConstraints，
		// 是用来控制添加进的组件的显示位置
	
		s.fill=GridBagConstraints.BELOW_BASELINE;
		
		s.gridwidth=1;
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

		bottomPanel.setLayout(new GridLayout(2,3));
		bottomPanel.add(bottomtitle);
		bottomPanel.add(bottomtitle1);
		bottomPanel.add(bottomtitle2);
		bottomPanel.add(wsp1);
		bottomPanel.add(slider4);
		bottomPanel.add(wsp2);

		initPanelText();//
	}

	public void fclose() {
		// this.setVisible(false);

		this.dispose();
		// this.close();
	}

	public void initPreview(controller c){

		init(c,null);
		setShadeWidth(10);
		this.setVisible(false);
		//this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 50));
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
					int left =getLocation().x;
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
					int left =getLocation().x;
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
					int left =getLocation().x;
					int top = getLocation().y;
					setLocation(left + e.getX() - xx, top + e.getY() - yy);
					setVisible(true);
					repaint();
				}
			}
		});
		setVisible(true);
		//setFocusable(true);
		//setFocusableWindowState(true);
		
	}
	public void initPanelText(){
		topPanel.setLayout(null);
		
		WebPanel panel_1 = new WebPanel();
		panel_1.setLayout(null);
		panel_1.setWebColoredBackground(false);
		panel_1.setBackground(new Color(0,0,0,0));
		panel_1.setBounds(0, 0, 110, 36);
		topPanel.add(panel_1);
		
		lblFwd = createWebLabel("Fw-190D");
		lblFwd.setHorizontalAlignment(SwingConstants.CENTER);
		lblFwd.setForeground(Color.WHITE);
		lblFwd.setFont(new Font(NumFont, Font.PLAIN, 20+fontadd));
		lblFwd.setBounds(0, 0, 72, 36);
		panel_1.add(lblFwd);
		
		WebLabel label = createWebLabel(language.eType);
		label.setBounds(72, 0, 36, 18);
		panel_1.add(label);
		label.setVerticalAlignment(SwingConstants.BOTTOM);
		label.setHorizontalAlignment(SwingConstants.LEFT);
		label.setForeground(Color.WHITE);
		label.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		
		WebLabel label_2 = createWebLabel("");
		label_2.setBounds(72, 18, 36, 18);
		panel_1.add(label_2);
		label_2.setVerticalAlignment(SwingConstants.TOP);
		label_2.setHorizontalAlignment(SwingConstants.LEFT);
		label_2.setForeground(Color.LIGHT_GRAY);
		label_2.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		
		WebPanel panel = new WebPanel();
		panel.setLayout(null);
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0,0,0,0));
		panel.setBounds(110, 0, 110, 36);
		topPanel.add(panel);
		
		label_3 = createWebLabel("2050");
		label_3.setHorizontalAlignment(SwingConstants.CENTER);
		label_3.setForeground(Color.WHITE);
		label_3.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_3.setBounds(0, 0, 72, 36);
		panel.add(label_3);
		
		WebLabel label_4 = createWebLabel(language.ePower);
		label_4.setVerticalAlignment(SwingConstants.BOTTOM);
		label_4.setHorizontalAlignment(SwingConstants.LEFT);
		label_4.setForeground(Color.WHITE);
		label_4.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_4.setBounds(72, 0, 36, 18);
		panel.add(label_4);
		
		WebLabel lblBhp = createWebLabel("Bhp");
		lblBhp.setVerticalAlignment(SwingConstants.TOP);
		lblBhp.setHorizontalAlignment(SwingConstants.LEFT);
		lblBhp.setForeground(Color.LIGHT_GRAY);
		lblBhp.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		lblBhp.setBounds(72, 18, 36, 18);
		panel.add(lblBhp);
		
		WebPanel panel_2 = new WebPanel();
		panel_2.setLayout(null);
		panel_2.setWebColoredBackground(false);
		panel_2.setBackground(new Color(0,0,0,0));
		panel_2.setBounds(220, 0, 110, 36);
		topPanel.add(panel_2);
		
		label_6 = createWebLabel("1200");
		label_6.setHorizontalAlignment(SwingConstants.CENTER);
		label_6.setForeground(Color.WHITE);
		label_6.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_6.setBounds(0, 0, 72, 36);
		panel_2.add(label_6);
		
		WebLabel label_7 = createWebLabel(language.eThurst);
		label_7.setVerticalAlignment(SwingConstants.BOTTOM);
		label_7.setHorizontalAlignment(SwingConstants.LEFT);
		label_7.setForeground(Color.WHITE);
		label_7.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_7.setBounds(72, 0, 36, 18);
		panel_2.add(label_7);
		
		WebLabel lblKgf = createWebLabel("Kgf");
		lblKgf.setVerticalAlignment(SwingConstants.TOP);
		lblKgf.setHorizontalAlignment(SwingConstants.LEFT);
		lblKgf.setForeground(Color.LIGHT_GRAY);
		lblKgf.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		lblKgf.setBounds(72, 18, 36, 18);
		panel_2.add(lblKgf);
		
		WebPanel panel_3 = new WebPanel();
		panel_3.setLayout(null);
		panel_3.setWebColoredBackground(false);
		panel_3.setBackground(new Color(0,0,0,0));
		panel_3.setBounds(330, 0, 110, 36);
		topPanel.add(panel_3);
		
		label_9 = createWebLabel("1860");
		label_9.setHorizontalAlignment(SwingConstants.CENTER);
		label_9.setForeground(Color.WHITE);
		label_9.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_9.setBounds(0, 0, 72, 36);
		panel_3.add(label_9);
		
		WebLabel label_10 = createWebLabel(language.eEffPower);
		label_10.setVerticalAlignment(SwingConstants.BOTTOM);
		label_10.setHorizontalAlignment(SwingConstants.LEFT);
		label_10.setForeground(Color.WHITE);
		label_10.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_10.setBounds(72, 0, 36, 18);
		panel_3.add(label_10);
		
		WebLabel lblBhp_1 = createWebLabel("Bhp");
		lblBhp_1.setVerticalAlignment(SwingConstants.TOP);
		lblBhp_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblBhp_1.setForeground(Color.LIGHT_GRAY);
		lblBhp_1.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		lblBhp_1.setBounds(72, 18, 36, 18);
		panel_3.add(lblBhp_1);
		
		WebPanel panel_4 = new WebPanel();
		panel_4.setLayout(null);
		panel_4.setWebColoredBackground(false);
		panel_4.setBackground(new Color(0,0,0,0));
		panel_4.setBounds(0, 36, 110, 36);
		topPanel.add(panel_4);
		
		label_12 = createWebLabel("165");
		label_12.setHorizontalAlignment(SwingConstants.CENTER);
		label_12.setForeground(Color.WHITE);
		label_12.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_12.setBounds(0, 0, 72, 36);
		panel_4.add(label_12);
		
		label_13 = createWebLabel(language.eFuel);
		label_13.setVerticalAlignment(SwingConstants.BOTTOM);
		label_13.setHorizontalAlignment(SwingConstants.LEFT);
		label_13.setForeground(Color.WHITE);
		label_13.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_13.setBounds(72, 0, 36, 18);
		panel_4.add(label_13);
		
		lblKg = createWebLabel("Kg");
		lblKg.setVerticalAlignment(SwingConstants.TOP);
		lblKg.setHorizontalAlignment(SwingConstants.LEFT);
		lblKg.setForeground(Color.LIGHT_GRAY);
		lblKg.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		lblKg.setBounds(72, 18, 36, 18);
		panel_4.add(lblKg);
		
		WebPanel panel_5 = new WebPanel();
		panel_5.setLayout(null);
		panel_5.setWebColoredBackground(false);
		panel_5.setBackground(new Color(0,0,0,0));
		panel_5.setBounds(110, 36, 110, 36);
		topPanel.add(panel_5);
		
		label_15 = createWebLabel("3000");
		label_15.setHorizontalAlignment(SwingConstants.CENTER);
		label_15.setForeground(Color.WHITE);
		label_15.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_15.setBounds(0, 0, 72, 36);
		panel_5.add(label_15);
		
		WebLabel label_16 = createWebLabel(language.eRPM);
		label_16.setVerticalAlignment(SwingConstants.BOTTOM);
		label_16.setHorizontalAlignment(SwingConstants.LEFT);
		label_16.setForeground(Color.WHITE);
		label_16.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_16.setBounds(72, 0, 36, 18);
		panel_5.add(label_16);
		
		WebLabel lblRpm = createWebLabel("RPM");
		lblRpm.setVerticalAlignment(SwingConstants.TOP);
		lblRpm.setHorizontalAlignment(SwingConstants.LEFT);
		lblRpm.setForeground(Color.LIGHT_GRAY);
		lblRpm.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		lblRpm.setBounds(72, 18, 36, 18);
		panel_5.add(lblRpm);
		
		WebPanel panel_6 = new WebPanel();
		panel_6.setLayout(null);
		panel_6.setWebColoredBackground(false);
		panel_6.setBackground(new Color(0,0,0,0));
		panel_6.setBounds(220, 36, 110, 36);
		topPanel.add(panel_6);
		
		label_18 = createWebLabel("110");
		label_18.setHorizontalAlignment(SwingConstants.CENTER);
		label_18.setForeground(Color.WHITE);
		label_18.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_18.setBounds(0, 0, 72, 36);
		panel_6.add(label_18);
		
		WebLabel label_19 = createWebLabel(language.eTemp);
		label_19.setVerticalAlignment(SwingConstants.BOTTOM);
		label_19.setHorizontalAlignment(SwingConstants.LEFT);
		label_19.setForeground(Color.WHITE);
		label_19.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_19.setBounds(72, 0, 36, 18);
		panel_6.add(label_19);
		
		WebLabel lblc_1 = createWebLabel("\u00B0C");
		lblc_1.setVerticalAlignment(SwingConstants.TOP);
		lblc_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblc_1.setForeground(Color.LIGHT_GRAY);
		lblc_1.setFont(new Font(NumFont, Font.PLAIN, 12+fontadd));
		lblc_1.setBounds(72, 18, 36, 18);
		panel_6.add(lblc_1);
		
		WebPanel panel_7 = new WebPanel();
		panel_7.setLayout(null);
		panel_7.setWebColoredBackground(false);
		panel_7.setBackground(new Color(0,0,0,0));
		panel_7.setBounds(330, 36, 110, 36);
		topPanel.add(panel_7);
		
		label_21 = createWebLabel("90");
		label_21.setHorizontalAlignment(SwingConstants.CENTER);
		label_21.setForeground(Color.WHITE);
		label_21.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd+fontadd));
		label_21.setBounds(0, 0, 72, 36);
		panel_7.add(label_21);
		
		WebLabel label_22 = createWebLabel(language.eEff);
		label_22.setVerticalAlignment(SwingConstants.BOTTOM);
		label_22.setHorizontalAlignment(SwingConstants.LEFT);
		label_22.setForeground(Color.WHITE);
		label_22.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_22.setBounds(72, 0, 36, 18);
		panel_7.add(label_22);
		
		WebLabel label_23 = createWebLabel("%");
		label_23.setVerticalAlignment(SwingConstants.TOP);
		label_23.setHorizontalAlignment(SwingConstants.LEFT);
		label_23.setForeground(Color.LIGHT_GRAY);
		label_23.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		label_23.setBounds(72, 18, 36, 18);
		panel_7.add(label_23);
		
		WebPanel panel_8 = new WebPanel();
		panel_8.setLayout(null);
		panel_8.setWebColoredBackground(false);
		panel_8.setBackground(new Color(0,0,0,0));
		panel_8.setBounds(0, 72, 110, 36);
		topPanel.add(panel_8);
		
		label_24 = createWebLabel("20");
		label_24.setHorizontalAlignment(SwingConstants.CENTER);
		label_24.setForeground(Color.WHITE);
		label_24.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_24.setBounds(0, 0, 72, 36);
		panel_8.add(label_24);
		
		WebLabel label_25 = createWebLabel(language.eFueltime);
		label_25.setVerticalAlignment(SwingConstants.BOTTOM);
		label_25.setHorizontalAlignment(SwingConstants.LEFT);
		label_25.setForeground(Color.WHITE);
		label_25.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_25.setBounds(72, 0, 36, 18);
		panel_8.add(label_25);
		
		WebLabel lblMin = createWebLabel("Min");
		lblMin.setVerticalAlignment(SwingConstants.TOP);
		lblMin.setHorizontalAlignment(SwingConstants.LEFT);
		lblMin.setForeground(Color.LIGHT_GRAY);
		lblMin.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		lblMin.setBounds(72, 18, 36, 18);
		panel_8.add(lblMin);
		
		WebPanel panel_9 = new WebPanel();
		panel_9.setLayout(null);
		panel_9.setWebColoredBackground(false);
		panel_9.setBackground(new Color(0,0,0,0));
		panel_9.setBounds(110, 72, 110, 36);
		topPanel.add(panel_9);
		
		label_27 = createWebLabel("2.1");
		label_27.setHorizontalAlignment(SwingConstants.CENTER);
		label_27.setForeground(Color.WHITE);
		label_27.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_27.setBounds(0, 0, 72, 36);
		panel_9.add(label_27);
		
		WebLabel label_28 = createWebLabel(language.eATM);
		label_28.setVerticalAlignment(SwingConstants.BOTTOM);
		label_28.setHorizontalAlignment(SwingConstants.LEFT);
		label_28.setForeground(Color.WHITE);
		label_28.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_28.setBounds(72, 0, 36, 18);
		panel_9.add(label_28);
		
		lblAta = createWebLabel("Ata");
		lblAta.setVerticalAlignment(SwingConstants.TOP);
		lblAta.setHorizontalAlignment(SwingConstants.LEFT);
		lblAta.setForeground(Color.LIGHT_GRAY);
		lblAta.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		lblAta.setBounds(72, 18, 36, 18);
		panel_9.add(lblAta);
		
		WebPanel panel_10 = new WebPanel();
		panel_10.setLayout(null);
		panel_10.setWebColoredBackground(false);
		panel_10.setBackground(new Color(0,0,0,0));
		panel_10.setBounds(220, 72, 110, 36);
		topPanel.add(panel_10);
		
		label_30 = createWebLabel("100");
		label_30.setHorizontalAlignment(SwingConstants.CENTER);
		label_30.setForeground(Color.WHITE);
		label_30.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_30.setBounds(0, 0, 72, 36);
		panel_10.add(label_30);
		
		WebLabel label_31 = createWebLabel(language.eOil);
		label_31.setVerticalAlignment(SwingConstants.BOTTOM);
		label_31.setHorizontalAlignment(SwingConstants.LEFT);
		label_31.setForeground(Color.WHITE);
		label_31.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_31.setBounds(72, 0, 36, 18);
		panel_10.add(label_31);
		
		WebLabel lblc = createWebLabel("\u00B0C");
		lblc.setVerticalAlignment(SwingConstants.TOP);
		lblc.setHorizontalAlignment(SwingConstants.LEFT);
		lblc.setForeground(Color.LIGHT_GRAY);
		lblc.setFont(new Font(NumFont, Font.PLAIN, 12+fontadd));
		lblc.setBounds(72, 18, 36, 18);
		panel_10.add(lblc);
		
		WebPanel panel_11 = new WebPanel();
		panel_11.setLayout(null);
		panel_11.setWebColoredBackground(false);
		panel_11.setBackground(new Color(0,0,0,0));
		panel_11.setBounds(330, 72, 110, 36);
		topPanel.add(panel_11);
		
		label_33 = createWebLabel("0");
		label_33.setHorizontalAlignment(SwingConstants.CENTER);
		label_33.setForeground(Color.WHITE);
		label_33.setFont(new Font(NumFont, Font.PLAIN, 26+fontadd));
		label_33.setBounds(0, 0, 72, 36);
		panel_11.add(label_33);
		
		WebLabel label_34 = createWebLabel(language.eOverheat);
		label_34.setVerticalAlignment(SwingConstants.BOTTOM);
		label_34.setHorizontalAlignment(SwingConstants.LEFT);
		label_34.setForeground(Color.WHITE);
		label_34.setFont(new Font(FontName, Font.PLAIN, 12+fontadd));
		label_34.setBounds(72, 0, 36, 18);
		panel_11.add(label_34);
		
		WebLabel lblS = createWebLabel("S");
		lblS.setVerticalAlignment(SwingConstants.TOP);
		lblS.setHorizontalAlignment(SwingConstants.LEFT);
		lblS.setForeground(Color.LIGHT_GRAY);
		lblS.setFont(new Font(NumFont, Font.PLAIN, 10+fontadd));
		lblS.setBounds(72, 18, 36, 18);
		panel_11.add(lblS);
		
		//WebSeparator separator1 = new WebSeparator();
		//separator1.setBounds(0, 0, 1024, 1);
		//topPanel.add(separator1);
		
		//WebSeparator separator2 = new WebSeparator();
		//separator2.setOrientation(SwingConstants.VERTICAL);
		//separator2.setBounds(471, -50, 1, 256);
		//topPanel.add(separator2);
		
		//WebSeparator separator3 = new WebSeparator();
		//separator3.setBounds(0, 100, 1024, 1);
		//topPanel.add(separator3);

	}
	public void init(controller xc, service ts) {
		this.c = xc;
		this.s = ts;
		overheattime=0;
		freq = controller.freqengineInfo;

		if(xc.getconfig("GlobalNumFont")!="")NumFont=xc.getconfig("GlobalNumFont");
		else NumFont=app.DefaultNumfontName;
		
		if(xc.getconfig("engineInfoFont")!="")FontName=xc.getconfig("engineInfoFont");
		else FontName=app.DefaultFont.getFontName();
		if(xc.getconfig("engineInfoFontadd")!="")fontadd=Integer.parseInt(xc.getconfig("engineInfoFontadd"));
		else fontadd=0;
		//System.out.println(fontadd);
		font1=font1+fontadd;
		font2=font2+fontadd;
		font3=font3+fontadd;
		if(xc.getconfig("engineInfoX")!="")lx=Integer.parseInt(xc.getconfig("engineInfoX"));
		else lx=0;
		if(xc.getconfig("engineInfoY")!="")ly=Integer.parseInt(xc.getconfig("engineInfoY"));
		else ly=860;
		
		WIDTH = 730;
		HEIGHT = 210;
		//OP = 100;

		setSize(WIDTH, HEIGHT);
		setLocation(lx, ly);
		//setIconImage(Toolkit.getDefaultToolkit().createImage("image/form1.jpg"));

		WebPanel panel = new WebPanel();

		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		//this.setUndecorated(true);

		panel.setBorderColor(new Color(0, 0, 0, 0));

		// 初始化Panel
		initPanel();

		// Split
		WebSplitPane splitPane2 = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane2.setOneTouchExpandable(false);
		// splitPane.setPreferredSize ( new Dimension ( 250, 200 ) );
		splitPane2.setDividerLocation(110);
		splitPane2.setContinuousLayout(false);
		splitPane2.setDividerSize(0);
		splitPane2.setEnabled(false);
		

		// Split
		splitPane = new WebSplitPane(HORIZONTAL_SPLIT, leftPanel, rightPanel);
		splitPane.setOneTouchExpandable(false);
		splitPane.setDrawDividerBorder(false);
		splitPane.setDividerSize(1);
		splitPane.setEnabled(false);
		// splitPane.setPreferredSize ( new Dimension ( 250, 200 ) );
		splitPane.setDividerLocation(200);
		splitPane.setContinuousLayout(true);
		splitPane.setDividerBorderColor(new Color(0, 0, 0, 0));

		panel.setLayout(new GridLayout(1, 2));
		// 标签

		// panel透明
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));

		
		
		// 添加绘制物件至panel
		rightPanel.add(splitPane2);
		panel.add(splitPane);
		this.add(panel);
		
		WebSeparator separator1 = new WebSeparator();
		separator1.setBounds(200, 162, 800, 1);
		//this.getRootPane().add(separator1);
		
		WebSeparator separator2 = new WebSeparator();
		separator2.setOrientation(SwingConstants.VERTICAL);
		separator2.setBounds(678, -30, 1, 256);
	
		//this.getRootPane().add(separator2);
		
		// this.setOpacity((float) (OP / 100.0f));

		
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);

		
	
		setDefaultCloseOperation(3);
		setTitle(language.eTitle);
		getWebRootPaneUI().getTitleComponent().getComponent(1).setFont(app.DefaultFont);//设置title字体
		setAlwaysOnTop(true);
		setFocusableWindowState(false);// 取消窗口焦点
		setFocusable(false);
		setVisible(true);

		
		
		//this.setCloseOnFocusLoss(true);
	
		if(xc.getconfig("engineInfoEdge").equals("true"))setShadeWidth(10);//玻璃效果边框
		else {
			setShadeWidth(0);
			//this.getRootPane().add(separator1);
			this.getRootPane().add(separator2);
		}
	}


	public void wspsetStep(WebStepProgress wsp, int step) {
		if (step > wsp.getStepsAmount()) {
			wsp.addSteps("");
			// System.out.println("成功");
			//wsp.setPathWidth(5);
		}
		
		wsp1.setSelectedStepIndex(step - 1);
	}
	public void updateOverheatTime(int Time){
		overheattime=Time;
		label_33.setText(Integer.toString(overheattime));
	}

	
	void setValueold(){


		bottomType.setText(s.iIndic.stype);
		bottomThr.setText(s.stotalthr);
		bottomRpm.setText(s.rpm);

		bottomHp.setText(s.stotalhp);
		bottomEhp.setText(s.stotalhpeff);
		bottomEff.setText(s.efficiency[0]);

		bottomEng.setText(s.watertemp);
		bottomOil.setText(s.oiltemp);

		bottomPre.setText(s.manifoldpressure);

		bottomWei.setText(s.stotalfuel);
		bottomTim.setText(s.sfueltime);
	}
	@Override
	public void run() {
		while (doit) {

			try {
				Thread.sleep(freq);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}

			slider1.setValue(s.sState.throttle);
			slider2.setValue(s.sState.RPMthrottle);
			slider3.setValue(s.sState.mixture);
			slider4.setValue(s.sState.radiator);
			wspsetStep(wsp1, s.sState.compressorstage);
			wsp2.setSelectedStepIndex(s.sState.magenato);
			
			lblFwd.setText(s.iIndic.stype);
			label_3.setText(s.stotalhp);
			label_6.setText(s.stotalthr);

			label_9.setText(s.stotalhpeff);
			label_12.setText(s.stotalfuel);
			label_15.setText(s.rpm);

			label_18.setText(s.watertemp);
			label_21.setText(s.efficiency[0]);

			label_24.setText(s.sfueltime);
			if(s.isFuelpressure){
				label_13.setText(language.eFuelPrs);
				lblKg.setText("%");
			}
			else{
				label_13.setText(language.eFuel);
				lblKg.setText("Kg");
			}

			if(s.checkAlt>0) {
				label_27.setText(s.pressurePounds);
				lblAta.setText("Psi "+s.pressureInchHg+"''");
			}
			else {
				label_27.setText(s.manifoldpressure);
				lblAta.setText("Ata ");
			}
			label_30.setText(s.oiltemp);
			
			//bottomPro.setText(s.pitch[0]);
			//System.out.println("engineInfo执行了");
			repaint();
			// this.status="infolist就绪";
			// System.out.println(this.status);
			// 绘图

			// this.status="infolist等待";
			// System.out.println(this.status);

		}
	}
}