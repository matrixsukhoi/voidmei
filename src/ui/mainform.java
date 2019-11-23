package ui;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Container;
import java.awt.FlowLayout;
import java.awt.Font;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;

import java.io.File;

import javax.swing.ImageIcon;
import javax.swing.SwingConstants;

import com.alee.extended.button.WebSwitch;
import com.alee.extended.panel.WebButtonGroup;
import com.alee.global.StyleConstants;
import com.alee.laf.WebLookAndFeel;
import com.alee.laf.button.WebButton;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.slider.WebSlider;
import com.alee.laf.splitpane.WebSplitPane;
import com.alee.laf.tabbedpane.TabbedPaneStyle;
import com.alee.laf.tabbedpane.WebTabbedPane;

import prog.app;
import prog.controller;
import prog.language;

import static com.alee.laf.splitpane.WebSplitPane.VERTICAL_SPLIT;

public class mainform extends WebFrame implements Runnable {
	/**
	 * 
	 */
	private static final long serialVersionUID = 5917570099029563038L;
	public volatile Boolean doit=true;
	public int WIDTH;
	public int HEIGHT;
	public controller tc;
	int GCcount=0;
	Container root;
	WebPanel Jp1;
	WebPanel Jp2;
	WebPanel Jp3;
	WebPanel Jp4;
	WebPanel Jp5;
	WebPanel Jp6;
	// test
	
	WebSwitch bflightInfoSwitch;
	WebSwitch bflightInfoEdge;
	WebComboBox sflightInfoFontC;
	WebSlider iflightInfoFontsizeaddC;
	
	WebSwitch bengineInfoSwitch;
	WebSwitch bengineInfoEdge;
	WebComboBox sengineInfoFont;
	WebSlider iengineInfoFontsizeadd;

	WebSwitch bcrosshairSwitch;
	WebSlider icrosshairScale;
	WebSwitch busetexturecrosshair;
	WebComboBox scrosshairName;
	WebSwitch bdrawHUDtext;

	WebSwitch benableLogging;
	WebSwitch benableInformation;
	
	WebSwitch benableAxis;
	WebSwitch benableAxisEdge;
	WebSwitch benablegearAndFlaps;
	WebSwitch benablegearAndFlapsEdge;
	
	Boolean movecheck;

	
	WebSwitch busetempInfoSwitch;
	WebSlider iInterval;
	WebComboBox sGlobalNumFont;
	
	public static String[] getFilelistNameNoEx(String[] list){
		int i;
		String[] a=new String[list.length];
		for (i=0;i<list.length;i++){
			//System.out.println(list[i]);
			a[i]=getFileNameNoEx(list[i]);
			//System.out.println(list[i]);
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
	
	public void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(255, 255, 255, 255));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(255, 255, 255, 255));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(255, 255, 255, 255));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(255, 255, 255, 255));// 外描边透明
	}

	public void initJP(WebPanel JP) {
		JP.setWebColoredBackground(false);
		JP.setBackground(new Color(0, 0, 0, 0));
		JP.setUndecorated(false);
		// JP.setMargin ( 20 );
		//JP.setShadeTransparency((float) 0.1);

		JP.setShadeWidth(2);
		JP.setRound(StyleConstants.largeRound);
		JP.setBorderColor(new Color(0, 0, 0, 100));

		JP.setPaintBottom(false);
		JP.setPaintTop(false);
		//JP.setPaintLeft(false);
		JP.setPaintRight(false);
		// JP.setBorderColor(new Color(0,0, 0, 255));
	}

	public void initJPinside(WebPanel JP) {
		JP.setWebColoredBackground(false);
		JP.setBackground(new Color(0, 0, 0, 0));
		// JP.setUndecorated ( false);
		// JP.setMargin ( 20 );
		JP.setShadeTransparency((float) 0.1);
		JP.setShadeWidth(2);
		JP.setRound(StyleConstants.largeRound);
		JP.setBorderColor(new Color(0, 0, 0, 100));

	}

	// JP1布局
	public WebButton createButton(String text) {
		WebButton a = new WebButton(text);
		a.setShadeWidth(1);
		a.setDrawShade(true);
		// a.getWebUI().setInnerShadeColor(new Color(255,255,255,200));
		// a.getWebUI().setInnerShadeWidth(10);
		a.setFont(app.DefaultFontBig);
		a.setTopBgColor(new Color(0, 0, 0, 0));
		a.setBottomBgColor(new Color(0, 0, 0, 0));
		// a.setUndecorated(false);
		// a.setShadeWidth(1);
		a.setBorderPainted(false);

		return a;

	}

	public WebButtonGroup createbuttonGroup() {
		WebButton A = createButton(language.mCancel);
		WebButton B = createButton(language.mStart);

		WebButtonGroup G = new WebButtonGroup(true, A, B);
		// G.setBorderColor(new Color(0, 0, 0, 0));
		// G.setButtonsDrawSides(true, true, false,false);
		//A.setPreferredHeight(30);
		A.setPreferredWidth(120);
		//B.setPreferredHeight(30);
		
		B.setPreferredWidth(120);
		B.setRound(10);
		G.setButtonsDrawSides(false, false, false, true);
		G.setButtonsForeground(new Color(0, 0, 0, 200));
		// G.setButtonsInnerShadeColor(new Color(0,0,0));
		// G.setButtonsInnerShadeWidth(5);
		G.setButtonsShadeWidth(3);
		// G.setButtonsDrawFocus(false);
		A.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {

				System.exit(0);

			}
		});
		B.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				if (movecheck) {
					controller.notification(language.mPlsclosePreview);
				} else {
					confirm();
				}
			}
		});
		// G.setPaintSides(false, false, false, false);
		return G;
	}


	public WebComboBox createCrosshairList(WebPanel topPanel,String text) {

		WebLabel lb = createWebLabel(text);
		/*
		 * WebList editableList = new WebList(app.fonts); //
		 * System.out.println(app.fonts.length);
		 * editableList.setFont(app.DefaultFont);
		 * editableList.setSelectionShadeWidth(0);
		 * editableList.setSelectionBorderColor(new Color(0, 0, 0, 0));
		 * editableList.setVisibleRowCount(5); editableList.setBackground(new
		 * Color(0, 0, 0, 0)); //editableList.setSelectionBackgroundColor(new
		 * Color(0, 0, 0, 0));
		 * editableList.getWebUI().setWebColoredSelection(false);
		 * editableList.getWebUI().setSelectionShadeWidth(1);
		 * editableList.getWebUI().setSelectionBorderColor(new Color(0, 0, 0,
		 * 100)); editableList.getWebUI().setSelectionRound(5);
		 * editableList.getWebUI().setSelectionBackgroundColor(new Color(0, 0,
		 * 0, 0)); editableList.getWebUI().setHighlightRolloverCell(true); //
		 * editableList.setSelectedIndex ( 0 ); //
		 * editableList.setSelectedValue("", true); //
		 * editableList.getSelectedValue(); editableList.setEditable(false);
		 * editableList.setSelectionBackground(new Color(0, 0, 0, 0));
		 * //editableList.setSelectionForeground(new Color(0, 0, 0, 0)); //
		 * System.out.println(editableList.getScrollableTracksViewportWidth());
		 * // editableList.setPreferredSize(400, 200); WebScrollPane WSP = new
		 * WebScrollPane(editableList); WSP.setWheelScrollingEnabled(true);
		 * WSP.getWebVerticalScrollBar().setPaintButtons(false); //
		 * WSP.getWebUI().setDrawBackground(false); WSP.setBackground(new
		 * Color(0, 0, 0, 0));
		 * WSP.getWebVerticalScrollBar().setPaintTrack(false);
		 * WSP.setShadeWidth(0);
		 */
		File file = new File("image/gunsight");
		String[] filelist = file.list();
		//System.out.println(file.list());
		filelist=getFilelistNameNoEx(filelist);
		//System.out.println(filelist[0]);
		WebComboBox comboBox = new WebComboBox(filelist);
		comboBox.setWebColoredBackground(false);
		// comboBox.getWebUI().setDrawBorder(false);
		comboBox.setShadeWidth(1);
		comboBox.setDrawFocus(false);
		// comboBox.getWebUI().setWebColoredBackground(false);
		// comboBox.getComponent(0).setBackground(new Color(0, 0, 0, 0));
		comboBox.setFont(app.DefaultFont);

		// comboBox.getComponentPopupMenu().setBackground(new Color(0, 0, 0,
		// 0));
		// comboBox.getWebUI().setDrawBorder(false);
		comboBox.setExpandedBgColor(new Color(0, 0, 0, 0));
		// comboBox.getWebUI().setExpandedBgColor(new Color(0, 0, 0, 0));
		comboBox.setBackground(new Color(0, 0, 0, 0));

		topPanel.add(lb);
		topPanel.add(comboBox);
		return comboBox;
	}

	public WebComboBox createFontList(WebPanel topPanel,String text) {

		WebLabel lb = createWebLabel(text);
		/*
		 * WebList editableList = new WebList(app.fonts); //
		 * System.out.println(app.fonts.length);
		 * editableList.setFont(app.DefaultFont);
		 * editableList.setSelectionShadeWidth(0);
		 * editableList.setSelectionBorderColor(new Color(0, 0, 0, 0));
		 * editableList.setVisibleRowCount(5); editableList.setBackground(new
		 * Color(0, 0, 0, 0)); //editableList.setSelectionBackgroundColor(new
		 * Color(0, 0, 0, 0));
		 * editableList.getWebUI().setWebColoredSelection(false);
		 * editableList.getWebUI().setSelectionShadeWidth(1);
		 * editableList.getWebUI().setSelectionBorderColor(new Color(0, 0, 0,
		 * 100)); editableList.getWebUI().setSelectionRound(5);
		 * editableList.getWebUI().setSelectionBackgroundColor(new Color(0, 0,
		 * 0, 0)); editableList.getWebUI().setHighlightRolloverCell(true); //
		 * editableList.setSelectedIndex ( 0 ); //
		 * editableList.setSelectedValue("", true); //
		 * editableList.getSelectedValue(); editableList.setEditable(false);
		 * editableList.setSelectionBackground(new Color(0, 0, 0, 0));
		 * //editableList.setSelectionForeground(new Color(0, 0, 0, 0)); //
		 * System.out.println(editableList.getScrollableTracksViewportWidth());
		 * // editableList.setPreferredSize(400, 200); WebScrollPane WSP = new
		 * WebScrollPane(editableList); WSP.setWheelScrollingEnabled(true);
		 * WSP.getWebVerticalScrollBar().setPaintButtons(false); //
		 * WSP.getWebUI().setDrawBackground(false); WSP.setBackground(new
		 * Color(0, 0, 0, 0));
		 * WSP.getWebVerticalScrollBar().setPaintTrack(false);
		 * WSP.setShadeWidth(0);
		 */

		WebComboBox comboBox = new WebComboBox(app.fonts);
		comboBox.setWebColoredBackground(false);
		// comboBox.getWebUI().setDrawBorder(false);
		comboBox.setShadeWidth(1);
		comboBox.setDrawFocus(false);
		// comboBox.getWebUI().setWebColoredBackground(false);
		// comboBox.getComponent(0).setBackground(new Color(0, 0, 0, 0));
		comboBox.setFont(app.DefaultFont);

		// comboBox.getComponentPopupMenu().setBackground(new Color(0, 0, 0,
		// 0));
		// comboBox.getWebUI().setDrawBorder(false);
		comboBox.setExpandedBgColor(new Color(0, 0, 0, 0));
		// comboBox.getWebUI().setExpandedBgColor(new Color(0, 0, 0, 0));
		comboBox.setBackground(new Color(0, 0, 0, 0));

		topPanel.add(lb);
		topPanel.add(comboBox);
		return comboBox;
	}

	public WebLabel createWebLabel(String text) {
		WebLabel lb = new WebLabel();
		lb = new WebLabel(text);
		lb.setHorizontalAlignment(SwingConstants.CENTER);

		// lb.setDrawShade(true);

		lb.setForeground(new Color(0, 0, 0, 200));
		lb.setShadeColor(Color.WHITE);
		lb.setFont(app.DefaultFont);
		return lb;
	}

	public WebButtonGroup createLBGroup(WebPanel topPanel) {
		WebButton B = createButton(language.mDisplayPreview);
		WebButton C = createButton(language.mSavePosition);
		WebButtonGroup G = new WebButtonGroup(true, B, C);
		B.setPreferredWidth(120);
		
		C.setPreferredWidth(120);
		B.setFont(app.DefaultFont);
		C.setFont(app.DefaultFont);
		G.setButtonsShadeWidth(3);

		// WebLabel lb=createWebLabel("调整位置");
		B.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				if (movecheck == false) {

					controller.notification(language.mMovePanel);
					saveconfig();
					tc.Preview();

					movecheck = true;
				} else {
					controller.notification(language.mPreviewWarning);
				}
			}
		});
		C.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				if (movecheck) {
					controller.notification(language.mPositionSaved);
					tc.endPreviewengineInfo();
					movecheck = false;
				} else {

					controller.notification(language.mPreviewNotOpen);
				}
			}
		});
		G.setButtonsDrawSides(false, false, false, true);

		topPanel.add(G);
		return G;
	}

	public WebSlider createLSGroup(WebPanel topPanel, String text, int min, int max,int size, int tick1, int tick2) {
		WebLabel lb = createWebLabel(text);
		WebSlider ws = new WebSlider(WebSlider.HORIZONTAL);

		ws.setMinimum(min);
		ws.setMaximum(max);
		ws.setDrawProgress(true);
		ws.setPaintTicks(true);
		ws.setPaintLabels(true);
		ws.setMinorTickSpacing(tick1);
		ws.setMajorTickSpacing(tick2);
		ws.setPreferredWidth(size);
		ws.setProgressShadeWidth(0);
		ws.setTrackShadeWidth(1);
		// slider1.setDrawThumb(false);
		ws.setThumbShadeWidth(2);
		ws.setThumbBgBottom(new Color(0, 0, 0, 0));
		ws.setThumbBgTop(new Color(0, 0, 0, 0));
		ws.setTrackBgBottom(new Color(0, 0, 0, 0));
		ws.setTrackBgTop(new Color(0, 0, 0, 0));
		ws.setProgressBorderColor(new Color(0, 0, 0, 0));
		ws.setProgressTrackBgBottom(new Color(0, 0, 0, 0));
		ws.setProgressTrackBgTop(new Color(0, 0, 0, 0));

		topPanel.add(lb);
		topPanel.add(ws);
		return ws;
	}

	public WebSwitch createLCGroup(WebPanel topPanel,
			String text/* , GridBagConstraints s, GridBagLayout layout */) {

		WebLabel lb = createWebLabel(text);
		WebSwitch ws;
		ws = new WebSwitch();
		//ws.setShadeWidth(0);
		ws.getWebUI().setShadeWidth(0);
		ws.setWebColoredBackground(false);
		// System.out.println(ws.getComponent(0).getIgnoreRepaint());
		// ws.getComponent(0).getIgnoreRepaint();
		ws.setBackground(new Color(0, 0, 0, 0));
		ws.getWebUI().setPaintSides(true, false, true, false);
		//ws.getWebUI().setPaintSides(false, true, false, true);
		ws.setRound(5);
		ws.setShadeWidth(0);
		ws.getLeftComponent().setFont(new Font(app.DefaultNumfontName,Font.PLAIN,14));
		ws.getRightComponent().setFont(new Font(app.DefaultNumfontName,Font.PLAIN,14));
		ws.getLeftComponent().setDrawShade(false);
		ws.getRightComponent().setDrawShade(false);
		ws.getLeftComponent().setText("On");
		ws.getRightComponent().setText("Off");
		//ws.getFirstComponent().setEnabled(false);
		//ws.getWebUI().setPaintSideLines(false, false, false, false);
		//ws.getComponent(0).setBackground(new Color(0,0,0));
		/*
		 * layout.setConstraints(lb, s); s.gridx++; layout.setConstraints(ws,
		 * s);
		 */

		topPanel.add(lb);
		topPanel.add(ws);
		return ws;

	}

	public void createvoidWebLabel(WebPanel topPanel,String text){
		WebLabel A=createWebLabel(text);
		topPanel.add(A);
	}

	
	
	public void initJP1() {
		initJP(Jp1);
		WebPanel topPanel = new WebPanel();
		WebPanel bottomPanel = new WebPanel();
		initJPinside(topPanel);
		initJPinside(bottomPanel);
		WebSplitPane splitPane = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane.setOneTouchExpandable(true);
		// splitPane.setPreferredSize ( new Dimension ( 250, 200 ) );
		splitPane.setDividerLocation(320);
		splitPane.setContinuousLayout(false);
		splitPane.setDividerSize(0);
		splitPane.setDrawDividerBorder(false);
		splitPane.setOneTouchExpandable(false);
		splitPane.setEnabled(false);

		// topPanel
		if(app.debug)busetempInfoSwitch = createLCGroup(topPanel, language.mP1TempNotification);
		if(app.debug)createvoidWebLabel(topPanel,language.mP1TempNotificationBlank);
		sGlobalNumFont = createFontList(topPanel,language.mP1GlobalNumberFont);
		createvoidWebLabel(topPanel,language.mP1GlobalNumberFontBlank);
		iInterval=createLSGroup(topPanel,language.mP1Interval, 30, 300,500,20 , 50);
		/*
		 * GridBagLayout layout1 = new GridBagLayout(); GridBagConstraints s1 =
		 * new GridBagConstraints(); s1.fill = GridBagConstraints.BOTH;
		 * s1.gridwidth = 1; s1.weightx = 0; s1.weighty = 0; s1.gridx = 0;
		 * s1.gridy = 0;
		 */
		// createLCGroup(topPanel, "显示发动机面板 ");

		// topPanel.setLayout(layout1);
		topPanel.setLayout(new FlowLayout());
		FlowLayout layout = new FlowLayout();
		layout.setAlignment(FlowLayout.LEFT);
		topPanel.setLayout(layout);

		// bottomPanel

		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);

		Jp1.add(splitPane);

	}


	// JP2布局
	public void initJP2() {

		initJP(Jp2);

		WebPanel topPanel = new WebPanel();
		WebPanel bottomPanel = new WebPanel();
		initJPinside(topPanel);
		initJPinside(bottomPanel);
		WebSplitPane splitPane = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane.setOneTouchExpandable(true);
		splitPane.setDividerLocation(320);
		splitPane.setContinuousLayout(false);
		splitPane.setDividerSize(0);
		splitPane.setDrawDividerBorder(false);
		splitPane.setOneTouchExpandable(false);
		splitPane.setEnabled(false);

		bengineInfoSwitch = createLCGroup(topPanel, language.mP2EnginePanel);
		createvoidWebLabel(topPanel,language.mP2EnginePanelBlank);
		bengineInfoEdge = createLCGroup(topPanel, language.mP2EngineGlassEdge);
		createvoidWebLabel(topPanel,language.mP2EngineGlassEdgeBlank);

		// createLCGroup(topPanel, "面板透明度 ");
		// createLBGroup(topPanel);
		sengineInfoFont = createFontList(topPanel,language.mP2PanelFont);
		iengineInfoFontsizeadd = createLSGroup(topPanel, language.mP2FontAdjust, -5, 7,200, 1, 1);
		FlowLayout layout = new FlowLayout();
		layout.setAlignment(FlowLayout.LEFT);
		topPanel.setLayout(layout);

		// bottomPanel

		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);
		WebButtonGroup G1 = createLBGroup(bottomPanel);
		bottomPanel.add(G1, BorderLayout.LINE_START);
		Jp2.add(splitPane);

	}

	// JP3布局
	public void initJP3() {
		initJP(Jp3);

		WebPanel topPanel = new WebPanel();
		WebPanel bottomPanel = new WebPanel();
		initJPinside(topPanel);
		initJPinside(bottomPanel);
		WebSplitPane splitPane = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane.setOneTouchExpandable(true);
		splitPane.setDividerLocation(320);
		splitPane.setDividerSize(0);
		splitPane.setContinuousLayout(false);
		splitPane.setDrawDividerBorder(false);
		splitPane.setOneTouchExpandable(false);
		splitPane.setEnabled(false);

		bcrosshairSwitch = createLCGroup(topPanel, language.mP3Crosshair);
		createvoidWebLabel(topPanel,language.mP3CrosshairBlank);
		bdrawHUDtext=createLCGroup(topPanel, language.mP3Text);
		createvoidWebLabel(topPanel,language.mP3TextBlank);
		busetexturecrosshair = createLCGroup(topPanel, language.mP3CrosshairTexture);
		createvoidWebLabel(topPanel,language.mP3CrosshairTextureBlank);
		scrosshairName = createCrosshairList(topPanel,language.mP3ChooseTexture);
		createvoidWebLabel(topPanel,language.mP3ChooseTextureBlank);
		icrosshairScale = createLSGroup(topPanel, language.mP3CrosshairSize, 0, 150,500, 5, 20);
		
		// createLCGroup(topPanel, "面板透明度 ");
		// createLBGroup(topPanel);
		// sengineInfoFont = createFontList(topPanel);

		FlowLayout layout = new FlowLayout();
		layout.setAlignment(FlowLayout.LEFT);
		topPanel.setLayout(layout);

		// bottomPanel

		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);
		WebButtonGroup G1 = createLBGroup(bottomPanel);
		bottomPanel.add(G1, BorderLayout.LINE_START);
		Jp3.add(splitPane);
	}

	public void initJP4(){
		initJP(Jp4);

		WebPanel topPanel = new WebPanel();
		WebPanel bottomPanel = new WebPanel();
		initJPinside(topPanel);
		initJPinside(bottomPanel);
		WebSplitPane splitPane = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane.setOneTouchExpandable(true);
		splitPane.setDividerLocation(320);
		splitPane.setDividerSize(0);
		splitPane.setContinuousLayout(false);
		splitPane.setDrawDividerBorder(false);
		splitPane.setOneTouchExpandable(false);
		splitPane.setEnabled(false);

		bflightInfoSwitch = createLCGroup(topPanel, language.mP4FlightInfoPanel);
		createvoidWebLabel(topPanel,language.mP4FlightInfoBlank);
		bflightInfoEdge = createLCGroup(topPanel, language.mP4FlightInfoGlassEdge);
		createvoidWebLabel(topPanel,language.mP4FlightInfoGlassEdgeBlank);
		// createLCGroup(topPanel, "面板透明度 ");
		// createLBGroup(topPanel);
		sflightInfoFontC = createFontList(topPanel,language.mP4PanelFont);
		iflightInfoFontsizeaddC = createLSGroup(topPanel, language.mP4FontAdjust, -5, 7,200, 1, 1);
		FlowLayout layout = new FlowLayout();
		layout.setAlignment(FlowLayout.LEFT);
		topPanel.setLayout(layout);

		// bottomPanel

		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);
		WebButtonGroup G1 = createLBGroup(bottomPanel);
		bottomPanel.add(G1, BorderLayout.LINE_START);
		
		Jp4.add(splitPane);
	}
	
	public void initJP5() {
		initJP(Jp5);
		WebPanel topPanel = new WebPanel();
		WebPanel bottomPanel = new WebPanel();
		initJPinside(topPanel);
		initJPinside(bottomPanel);
		WebSplitPane splitPane = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane.setOneTouchExpandable(true);
		// splitPane.setPreferredSize ( new Dimension ( 250, 200 ) );
		splitPane.setDividerLocation(320);
		splitPane.setDividerSize(0);
		splitPane.setContinuousLayout(false);
		splitPane.setDrawDividerBorder(false);
		splitPane.setOneTouchExpandable(false);
		splitPane.setEnabled(false);

		// topPanel
		benableLogging = createLCGroup(topPanel, language.mP5LoggingAndCharting);
		createvoidWebLabel(topPanel,language.mP5LoggingAndChartingBlank);
		benableInformation= createLCGroup(topPanel, language.mP5Information);
		createvoidWebLabel(topPanel,language.mP5InformationBlank);
		/*
		 * GridBagLayout layout1 = new GridBagLayout(); GridBagConstraints s1 =
		 * new GridBagConstraints(); s1.fill = GridBagConstraints.BOTH;
		 * s1.gridwidth = 1; s1.weightx = 0; s1.weighty = 0; s1.gridx = 0;
		 * s1.gridy = 0;
		 */
		// createLCGroup(topPanel, "显示发动机面板 ");

		// topPanel.setLayout(layout1);
		topPanel.setLayout(new FlowLayout());
		FlowLayout layout = new FlowLayout();
		layout.setAlignment(FlowLayout.LEFT);
		topPanel.setLayout(layout);

		// bottomPanel
		WebButtonGroup G1 = createLBGroup(bottomPanel);
		bottomPanel.add(G1, BorderLayout.LINE_START);
		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);

		Jp5.add(splitPane);

	}
	public void initJP6() {
		initJP(Jp6);
		WebPanel topPanel = new WebPanel();
		WebPanel bottomPanel = new WebPanel();
		initJPinside(topPanel);
		initJPinside(bottomPanel);
		WebSplitPane splitPane = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane.setOneTouchExpandable(true);
		// splitPane.setPreferredSize ( new Dimension ( 250, 200 ) );
		splitPane.setDividerLocation(320);
		splitPane.setDividerSize(0);
		splitPane.setContinuousLayout(false);
		splitPane.setDrawDividerBorder(false);
		splitPane.setOneTouchExpandable(false);
		splitPane.setEnabled(false);

		// topPanel
		benableAxis = createLCGroup(topPanel, language.mP6AxisPanel);
		createvoidWebLabel(topPanel,language.mP6AxisPanelBlank);
		benableAxisEdge = createLCGroup(topPanel, language.mP6AxisEdge);
		createvoidWebLabel(topPanel,language.mP6AxisEdgeBlank);
		benablegearAndFlaps=createLCGroup(topPanel, language.mP6GearAndFlaps);
		benablegearAndFlapsEdge=createLCGroup(topPanel, language.mP6GearAndFlapsEdge);
		/*
		 * GridBagLayout layout1 = new GridBagLayout(); GridBagConstraints s1 =
		 * new GridBagConstraints(); s1.fill = GridBagConstraints.BOTH;
		 * s1.gridwidth = 1; s1.weightx = 0; s1.weighty = 0; s1.gridx = 0;
		 * s1.gridy = 0;
		 */
		// createLCGroup(topPanel, "显示发动机面板 ");

		// topPanel.setLayout(layout1);
		topPanel.setLayout(new FlowLayout());
		FlowLayout layout = new FlowLayout();
		layout.setAlignment(FlowLayout.LEFT);
		topPanel.setLayout(layout);

		// bottomPanel
		WebButtonGroup G1 = createLBGroup(bottomPanel);
		bottomPanel.add(G1, BorderLayout.LINE_START);
		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);

		Jp6.add(splitPane);

	}
	
	public void initPanel() {
		WebTabbedPane tabbedPane = new WebTabbedPane();
		// tabbedPane3.setPreferredSize ( new Dimension ( 150, 120 ) );
		tabbedPane.setTabPlacement(WebTabbedPane.LEFT);
		// tabbedPane.setTabLayoutPolicy(JTabbedPane.SCROLL_TAB_LAYOUT);

		Jp1 = new WebPanel();
		Jp2 = new WebPanel();
		Jp3 = new WebPanel();
		Jp4 = new WebPanel();
		Jp5 = new WebPanel();
		Jp6=new WebPanel();
		initJP1();
		initJP2();
		if(!app.ForeignLanguage)initJP3();
		initJP4();
		initJP5();
		initJP6();
		
		tabbedPane.addTab(language.mFlightInfo, Jp4);
		tabbedPane.addTab(language.mEngineInfo, Jp2);
		tabbedPane.addTab(language.mControlInfo, Jp6);
		tabbedPane.addTab(language.mLoggingAndAnalysis, Jp5);
		if(!app.ForeignLanguage)tabbedPane.addTab(language.mCrosshair, Jp3);
		tabbedPane.addTab(language.mAdvancedOption, Jp1);
		// tabbedPane.setTabBorderColor(new Color(0, 0, 0, 0));
		// tabbedPane.setContentBorderColor(new Color(0, 0, 0, 0));
		// tabbedPane.setShadeWidth(1);
		// tabbedPane.setSelectedBottomBg(new Color(0, 0, 0,20));
		// tabbedPane.setSelectedTopBg(new Color(0, 0, 0, 20));
		// tabbedPane.setBackgroundAt(1, new Color(0, 0, 0, 0));

		// tabbedPane.setSelectedForegroundAt(0,(new Color(0, 0, 0, 0));
		tabbedPane.setSelectedIndex(0);

		tabbedPane.setBackground(new Color(0, 0, 0, 0));
		// tabbedPane.getWebUI().setBackgroundColor(new Color(0,0,0,0));
		tabbedPane.setFont(app.DefaultFontBig);
		// tabbedPane.setShadeWidth(5);
		// tabbedPane.setForeground(new Color(255,255,255,255));
		// tabbedPane.setComponentOrientation(
		// ComponentOrientation.RIGHT_TO_LEFT);
		// tabbedPane.setBottomBg(new Color(255, 255, 255, 255));
		// tabbedPane.setTopBg(new Color(255, 255, 255, 255));
		tabbedPane.setPaintOnlyTopBorder(true);
		tabbedPane.setPaintBorderOnlyOnSelectedTab(true);
		tabbedPane.setTabbedPaneStyle(TabbedPaneStyle.attached);
		this.add(tabbedPane);
	}

	public void initConfig() {
		// tc.initconfig();
		// 从TC中取参数即可
		// System.out.println(Boolean.parseBoolean(tc.getconfig("engineInfoSwitch")));

		bflightInfoSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("flightInfoSwitch")));
		bflightInfoEdge.setSelected(Boolean.parseBoolean(tc.getconfig("flightInfoEdge")));
		sflightInfoFontC.setSelectedItem(tc.getconfig("flightInfoFontC"));
		iflightInfoFontsizeaddC.setValue(Integer.parseInt(tc.getconfig("flightInfoFontaddC")));
		
		bengineInfoSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("engineInfoSwitch")));
		bengineInfoEdge.setSelected(Boolean.parseBoolean(tc.getconfig("engineInfoEdge")));
		sengineInfoFont.setSelectedItem(tc.getconfig("engineInfoFont"));
		iengineInfoFontsizeadd.setValue(Integer.parseInt(tc.getconfig("engineInfoFontadd")));
		
		if(!app.ForeignLanguage)bcrosshairSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("crosshairSwitch")));
		if(!app.ForeignLanguage)icrosshairScale.setValue(Integer.parseInt(tc.getconfig("crosshairScale")));
		if(!app.ForeignLanguage)busetexturecrosshair.setSelected(Boolean.parseBoolean(tc.getconfig("usetexturecrosshair")));
		if(!app.ForeignLanguage)scrosshairName.setSelectedItem(tc.getconfig("crosshairName"));
		if(!app.ForeignLanguage)bdrawHUDtext.setSelected(Boolean.parseBoolean(tc.getconfig("drawHUDtext")));
		
		if(app.debug)busetempInfoSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("usetempInfoSwitch")));
		sGlobalNumFont.setSelectedItem(tc.getconfig("GlobalNumFont"));
		iInterval.setValue(Integer.parseInt(tc.getconfig("Interval")));
		
		benableLogging.setSelected(Boolean.parseBoolean(tc.getconfig("enableLogging")));
		benableInformation.setSelected(Boolean.parseBoolean(tc.getconfig("enableAltInformation")));
		
		benableAxis.setSelected(Boolean.parseBoolean(tc.getconfig("enableAxis")));
		benableAxisEdge.setSelected(Boolean.parseBoolean(tc.getconfig("enableAxisEdge")));
		benablegearAndFlaps.setSelected(Boolean.parseBoolean(tc.getconfig("enablegearAndFlaps")));
		benablegearAndFlapsEdge.setSelected(Boolean.parseBoolean(tc.getconfig("enablegearAndFlapsEdge")));
	}

	public void saveconfig() {
		// System.out.println(Boolean.toString(bengineInfoSwitch.isSelected()));
		
		tc.setconfig("flightInfoSwitch", Boolean.toString(bflightInfoSwitch.isSelected()));
		tc.setconfig("flightInfoEdge", Boolean.toString(bflightInfoEdge.isSelected()));
		//System.out.println(sengineInfoFont.getSelectedItem().toString());
		tc.setconfig("flightInfoFontC", sflightInfoFontC.getSelectedItem().toString());
		tc.setconfig("flightInfoFontaddC", Integer.toString(iflightInfoFontsizeaddC.getValue()));
		
		tc.setconfig("engineInfoSwitch", Boolean.toString(bengineInfoSwitch.isSelected()));
		tc.setconfig("engineInfoEdge", Boolean.toString(bengineInfoEdge.isSelected()));
		// System.out.println(sengineInfoFont.getSelectedValue());
		tc.setconfig("engineInfoFont", sengineInfoFont.getSelectedItem().toString());
		tc.setconfig("engineInfoFontadd", Integer.toString(iengineInfoFontsizeadd.getValue()));
		
		if(!app.ForeignLanguage)tc.setconfig("crosshairSwitch", Boolean.toString(bcrosshairSwitch.isSelected()));
		if(!app.ForeignLanguage)tc.setconfig("crosshairScale", Integer.toString(icrosshairScale.getValue()));
		if(!app.ForeignLanguage)tc.setconfig("usetexturecrosshair", Boolean.toString(busetexturecrosshair.isSelected()));
		if(!app.ForeignLanguage)tc.setconfig("crosshairName", scrosshairName.getSelectedItem().toString());
		if(!app.ForeignLanguage)tc.setconfig("drawHUDtext",  Boolean.toString(bdrawHUDtext.isSelected()));
		
		if(app.debug)tc.setconfig("usetempInfoSwitch", Boolean.toString(busetempInfoSwitch.isSelected()));
		tc.setconfig("GlobalNumFont",sGlobalNumFont.getSelectedItem().toString());
		tc.setconfig("Interval", Integer.toString(iInterval.getValue()));

		tc.setconfig("enableLogging", Boolean.toString(benableLogging.isSelected()));
		tc.setconfig("enableAltInformation", Boolean.toString(benableInformation.isSelected()));
		
		tc.setconfig("enableAxis", Boolean.toString(benableAxis.isSelected()));
		tc.setconfig("enableAxisEdge", Boolean.toString(benableAxisEdge.isSelected()));
		tc.setconfig("enablegearAndFlaps", Boolean.toString(benablegearAndFlaps.isSelected()));
		tc.setconfig("enablegearAndFlapsEdge", Boolean.toString(benablegearAndFlapsEdge.isSelected()));
		
		
	}

	public void confirm() {
		saveconfig();
		tc.saveconfig();
		tc.flag = 1;
		tc.start();
		// this.dispose();
	}

	public void init(controller c) {
		//System.setProperty("awt.useSystemAAFontSettings", "on");
		//System.out.println("mainForm初始化了");
		WebLookAndFeel.globalTitleFont = app.DefaultFontBigBold;
		WIDTH = 800;
		HEIGHT = 450;
		//Image I=Toolkit.getDefaultToolkit().getImage("image/form1.png");
		//I=I.getScaledInstance(32, 32,  Image.SCALE_SMOOTH);
		//this.setIconImage(I);
		tc = c;
		movecheck = false;
		this.setUndecorated(true);
		this.setLocation(app.ScreenWidth/2-WIDTH/2, app.ScreenHeight/2-HEIGHT/2);
		this.setFont(app.DefaultFont);
		this.setSize(WIDTH, HEIGHT);
		
		setFrameOpaque();// 窗口透明
		String ti=app.appName +" v"+ app.version;
		if(app.debug)ti=ti+"beta";
		//ti=ti+"　　————"+app.appTooltips;
		setTitle(ti);
		this.setShowMaximizeButton(false);
		this.getWebRootPaneUI().getTitleComponent().getComponent(1).setFont(new Font(app.DefaultFont.getName(),Font.PLAIN,14));// 设置title字体
		// this.getWebRootPaneUI().getWindowButtons().setButtonsInnerShadeWidth(0);
		// this.getWebRootPaneUI().getWindowButtons().setButtonsShadeWidth(0);
		this.getWebRootPaneUI().getWindowButtons().setBorderColor(new Color(0, 0, 0, 0));
		// this.getWebRootPaneUI().getWindowButtons().setButtonsDrawBottom(false);
		this.getWebRootPaneUI().getWindowButtons().setButtonsDrawSides(false, false, false, false);
		this.getWebRootPaneUI().getWindowButtons().getWebButton(0).setTopBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(0).setBottomBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(1).setTopBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(1).setBottomBgColor(new Color(0, 0, 0, 0));
		root=this.getContentPane();
		
	
		/*
		 * 如果有最大化 this.getWebRootPaneUI().getWindowButtons().getWebButton(2).
		 * setTopBgColor(new Color(0, 0, 0, 0));
		 * this.getWebRootPaneUI().getWindowButtons().getWebButton(2).
		 * setBottomBgColor(new Color(0, 0, 0, 0));
		 */
		setDrawWatermark(true);
		setWatermark(new ImageIcon("image/watermark.png"));
		
		// this.getWebRootPaneUI().getWindowButtons().getWebButton(2).setBorderPainted(false);
		initPanel();
		initConfig();// 读入Config

		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setVisible(true);
		this.setShadeWidth(10);
		//controller.notificationtime("1、本程序对游戏及游戏进程无任何修改，所有数据来自于官方开放的8111端口。\n\r2、本程序只是本人兴趣使然的创作，无任何营利性质。\n\r3、请勿将本程序扩散至百度WT相关贴吧，除此之外随意转载",10000);
	}
	

	@Override
	public void run() {
		// TODO Auto-generated method stub
		while (doit) {
			try {
				Thread.sleep(40);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			GCcount++;
			// System.out.println("As");
			//System.out.println("MainFrame执行了");
			this.repaint();
			if(GCcount==100){
				//System.out.println("MainFrameGC");
				System.gc();
				GCcount=0;
			}
		}
	}
}