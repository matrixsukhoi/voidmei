package ui;

import static javax.swing.JSplitPane.VERTICAL_SPLIT;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Container;
import java.awt.FlowLayout;
import java.awt.Font;
import java.awt.Image;
import java.awt.Toolkit;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.io.File;

import javax.swing.ImageIcon;
import javax.swing.SwingConstants;

import com.alee.extended.button.WebSwitch;
import com.alee.extended.image.WebImage;
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
import com.alee.laf.text.WebTextField;
import com.alee.utils.ImageUtils;

import prog.app;
import prog.controller;
import prog.lang;

public class mainform extends WebFrame implements Runnable {
	/**
	 * 
	 */
	private static final long serialVersionUID = 5917570099029563038L;
	public volatile Boolean doit=true;
	public int width;
	public int height;
	public controller tc;
	int gcCount=0;
	Container root;
	WebPanel jp1;
	WebPanel jp2;
	WebPanel jp3;
	WebPanel jp4;
	WebPanel jp5;
	WebPanel jp6;
	// test
	
	WebSwitch bFlightInfoSwitch;
	WebSwitch bFlightInfoEdge;
	WebComboBox sFlightInfoFont;
	WebSlider iFlightInfoFontSizeIncr;
	
	WebSwitch bEngineInfoSwitch;
	WebSwitch bEngineInfoEdge;
	WebComboBox fEngineInfoFont;
	WebSlider iEngineInfoFontSizeIncr;

	WebSwitch bCrosshairSwitch;
	WebSlider iCrosshairScale;
	WebSwitch bTextureCrosshairSwitch;
	WebComboBox sCrosshairName;
	WebSwitch bDrawHudTextSwitch;

	WebSwitch bEnableLogging;
	WebSwitch bEnableInformation;
	
	WebSwitch bEnableAxis;
	WebSwitch bEnableAxisEdge;
	WebSwitch bEnablegearAndFlaps;
	WebSwitch bEnablegearAndFlapsEdge;
	
	Boolean moveCheckFlag;

	
	WebSwitch bTempInfoSwitch;
	WebSlider iInterval;
	WebComboBox sGlobalNumFont;
	Color whiteBg = new Color(255,255,255,255); 
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
		WebButton A = createButton(lang.mCancel);
		WebButton B = createButton(lang.mStart);

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
				if (moveCheckFlag) {
					controller.notification(lang.mPlsclosePreview);
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

		lb.setForeground(new Color(0, 0, 0, 230));
		lb.setShadeColor(Color.WHITE);
		lb.setFont(app.DefaultFont);
		return lb;
	}
	public WebButton displayPreview;
	public WebSwitch battitudeIndicatorSwitch;
	public WebSwitch bFMPrintSwitch;
	private WebSwitch bcrosshairdisplaySwitch;
	private WebSwitch bFlightInfoIAS;
	private WebSwitch bFlightInfoTAS;
	private WebSwitch bFlightInfoMach;
	private WebSwitch bFlightInfoHeight;
	private WebSwitch bFlightInfoCompass;
	private WebSwitch bFlightInfoVario;
	private WebSwitch bFlightInfoSEP;
	private WebSwitch bFlightInfoAcc;
	private WebSwitch bFlightInfoWx;
	private WebSwitch bFlightInfoNy;
	private WebSwitch bFlightInfoTurn;
	private WebSwitch bFlightInfoTurnRadius;
	private WebSwitch bFlightInfoAoA;
	private WebSwitch bFlightInfoAoS;
	private WebSwitch bFlightInfoWingSweep;
	private WebSlider iflightInfoColumnNum;
	private WebSlider iengineInfoColumnNum;
	private WebSwitch bvoiceWarningSwitch;
	private WebSwitch bFlightInfoRadioAlt;
	private WebSwitch benableEngineControl;
	private WebSwitch bdrawShadeSwitch;
	private WebTextField cNumColor;
	private WebTextField cLabelColor;
	private WebTextField cUnitColor;
	private WebTextField cWarnColor;
	private WebTextField cShadeColor;
	private WebSwitch bEngineControlRadiator;
	private WebSwitch bEngineControlMixture;
	private WebSwitch bEngineControlPitch;
	private WebSwitch bEngineControlCompressor;
	private WebSwitch bEngineControlLFuel;
	private WebSwitch bEngineControlThrottle;
	private WebSwitch bEngineInfoHorsePower;
	private WebSwitch bEngineInfoThrust;
	private WebSwitch bEngineInfoRPM;
	private WebSwitch bEngineInfoPropPitch;
	private WebSwitch bEngineInfoEffEta;
	private WebSwitch bEngineInfoEffHp;
	private WebSwitch bEngineInfoPressure;
	private WebSwitch bEngineInfoPowerPercent;
	private WebSwitch bEngineInfoFuelKg;
	private WebSwitch bEngineInfoFuelTime;
	private WebSwitch bEngineInfoWepKg;
	private WebSwitch bEngineInfoWepTime;
	private WebSwitch bEngineInfoTemp;
	private WebSwitch bEngineInfoOilTemp;
	private WebSwitch bEngineInfoHeatTolerance;
	private WebSwitch bEngineInfoEngResponse;
	private WebSwitch bstatusSwitch;
	private WebSlider ivoiceVolume;
	private WebSwitch bAAEnable;
	
    private String getColorText ( final Color color )
    {
        return color.getRed () + ", " + color.getGreen () + ", " + color.getBlue () + ", " + color.getAlpha();
    }
    
    private Color textToColor(String t){
    	int R,G,B,A;
    	t = t.replaceAll(" ", "");
        String []ts = t.split(",");
        if (ts.length < 4) return Color.BLACK;
        R = Integer.parseInt(ts[0]);
        G = Integer.parseInt(ts[1]);
        B = Integer.parseInt(ts[2]);
        A = Integer.parseInt(ts[3]);
        Color c = new Color(R, G, B, A);
//        System.out.println(getColorText(c));
        return c;
    }
    
    public Color updateColorGroupColor(WebTextField trailing){
    	Color c = textToColor(trailing.getText());
//    	System.out.println(getColorText(c));
    	trailing.setLeadingComponent ( new WebImage ( ImageUtils.createColorIcon ( c ) ) );	
    	return c;
    }
	public WebTextField createColorGroup(WebPanel topPanel, String text){
        // Initial color
        final Color initialColor = Color.WHITE;

        WebLabel lb = createWebLabel(text);
        String S = getColorText(initialColor);
        
//        textToColor(S);
        
        
        WebTextField trailing = new WebTextField ( getColorText(initialColor), 15 );
        trailing.setMargin ( 0, 0, 0, 2 );
        trailing.setLeadingComponent ( new WebImage ( ImageUtils.createColorIcon ( initialColor ) ) );
        trailing.setShadeWidth(2);
        
        trailing.addActionListener(new ActionListener(){

            public void actionPerformed(ActionEvent e){

            	Color c = updateColorGroupColor(trailing);
            }
        });
        
        topPanel.add(lb);
        topPanel.add(trailing);
        
        
        
        return trailing;
        
        
        // Simple color chooser
//        final WebButton colorChooserButton = new WebButton ( getColorText ( initialColor ), ImageUtils.createColorIcon ( initialColor ) );
//        colorChooserButton.setLeftRightSpacing ( 0 );
//        colorChooserButton.setMargin ( 0, 0, 0, 3 );
//        colorChooserButton.addActionListener ( new ActionListener ()
//        {
//            private WebColorChooserDialog colorChooser = null;
//            private Color lastColor = initialColor;
//
//            @Override
//            public void actionPerformed ( final ActionEvent e )
//            {
//                if ( colorChooser == null )
//                {
//                    colorChooser = new WebColorChooserDialog ( topPanel );
//                }
//                colorChooser.setColor ( lastColor );
//                colorChooser.setVisible ( true );
//
//                if ( colorChooser.getResult () == DialogOptions.OK_OPTION )
//                {
//                    final Color color = colorChooser.getColor ();
//                    lastColor = color;
//
//                    colorChooserButton.setIcon ( ImageUtils.createColorIcon ( color ) );
//                    colorChooserButton.setText ( getColorText ( color ) );
//                }
//            }
//        } );
//        GroupPanel t = new GroupPanel ( colorChooserButton );
//        topPanel.add(lb);
//        topPanel.add(t);
//        return t;

	}
	public WebButtonGroup createLBGroup(WebPanel topPanel) {
		displayPreview = createButton(lang.mDisplayPreview);
		WebButton C = createButton(lang.mSavePosition);
		WebButtonGroup G = new WebButtonGroup(true, displayPreview, C);
		displayPreview.setPreferredWidth(120);
		
		C.setPreferredWidth(120);
		displayPreview.setFont(app.DefaultFont);
		C.setFont(app.DefaultFont);
		G.setButtonsShadeWidth(3);

		// WebLabel lb=createWebLabel("调整位置");
		displayPreview.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				if (moveCheckFlag == false) {

					controller.notification(lang.mMovePanel);
					saveconfig();
					tc.Preview();

					moveCheckFlag = true;
				} else {
//					System.out.println("重置\n");
//					config_init();
					controller.notification(lang.mPreviewWarning);
				}
			}
		});
		C.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				if (moveCheckFlag) {
					controller.notification(lang.mPositionSaved);
					tc.endPreview();
					moveCheckFlag = false;
				} else {

					controller.notification(lang.mPreviewNotOpen);
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
		ws.setThumbShadeWidth(1);
		ws.setThumbBgBottom(whiteBg);
		ws.setThumbBgTop(whiteBg);
		ws.setTrackBgBottom(whiteBg);
		ws.setTrackBgTop(whiteBg);
		ws.setProgressBorderColor(whiteBg);
		ws.setProgressTrackBgBottom(whiteBg);
		ws.setProgressTrackBgTop(whiteBg);

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
		ws.setBackground(whiteBg);
		ws.getWebUI().setPaintSides(true, false, true, false);
		//ws.getWebUI().setPaintSides(false, true, false, true);
		ws.setRound(5);
//		ws.setAnimate(false);
		ws.setShadeWidth(1);
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
		initJP(jp1);
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
		
		if(app.debug)bTempInfoSwitch = createLCGroup(topPanel, lang.mP1statusBar);
		if(app.debug)createvoidWebLabel(topPanel,lang.mP1statusBarBlank);
		
		bstatusSwitch = createLCGroup(topPanel, lang.mP1statusBar);
		createvoidWebLabel(topPanel,lang.mP1statusBarBlank);
		
		bvoiceWarningSwitch = createLCGroup(topPanel, lang.mP1VoiceWarning);
		createvoidWebLabel(topPanel,lang.mP1VoiceWarningBlank);
		
		ivoiceVolume = createLSGroup(topPanel, lang.mP1voiceVolume, 0, 200, 300, 10, 50);
		createvoidWebLabel(topPanel,lang.mP1voiceVolumeBlank);
		
		
		
		bdrawShadeSwitch = createLCGroup(topPanel, lang.mP1drawFontShape);
		createvoidWebLabel(topPanel,lang.mP1drawFontShapeBlank);

		bAAEnable = createLCGroup(topPanel, lang.mP1AAEnable);
		createvoidWebLabel(topPanel,lang.mP1AAEnableBlank);
		

		sGlobalNumFont = createFontList(topPanel,lang.mP1GlobalNumberFont);
		createvoidWebLabel(topPanel,lang.mP1GlobalNumberFontBlank);
		
		cNumColor = createColorGroup(topPanel, lang.mP1NumColor);
		createvoidWebLabel(topPanel,lang.mP1NumColorBlank);
		cLabelColor = createColorGroup(topPanel, lang.mP1LabelColor);
		createvoidWebLabel(topPanel,lang.mP1LabelColorBlank);
		
		cUnitColor = createColorGroup(topPanel, lang.mP1UnitColor);
		createvoidWebLabel(topPanel,lang.mP1UnitColorBlank);
		
		cWarnColor = createColorGroup(topPanel, lang.mP1WarnColor);
		createvoidWebLabel(topPanel,lang.mP1WarnColorBlank);
		
		cShadeColor = createColorGroup(topPanel, lang.mP1ShadeColor);
		createvoidWebLabel(topPanel,lang.mP1ShadeColorBlank);
		
		iInterval=createLSGroup(topPanel,lang.mP1Interval, 10, 300,500,5 , 40);
		


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

//		WebButtonGroup G = createbuttonGroup();
//		bottomPanel.add(G, BorderLayout.LINE_END);
		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);
		WebButtonGroup G1 = createLBGroup(bottomPanel);
		bottomPanel.add(G1, BorderLayout.LINE_START);
		
		jp1.add(splitPane);

	}


	// JP2布局
	public void initJP2() {

		initJP(jp2);

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

		
		
		bEngineInfoSwitch = createLCGroup(topPanel, lang.mP2EnginePanel);
		createvoidWebLabel(topPanel,lang.mP2EnginePanelBlank);
		bEngineInfoEdge = createLCGroup(topPanel, lang.mP2EngineGlassEdge);
		createvoidWebLabel(topPanel,lang.mP2EngineGlassEdgeBlank);


		bEngineInfoHorsePower = createLCGroup(topPanel, lang.mP2eiHorsePower);
		createvoidWebLabel(topPanel,lang.mP2eiHorsePowerBlank);
		bEngineInfoThrust = createLCGroup(topPanel, lang.mP2eiThrust);
		createvoidWebLabel(topPanel,lang.mP2eiThrustBlank);
		bEngineInfoRPM = createLCGroup(topPanel, lang.mP2eiRPM);
		createvoidWebLabel(topPanel,lang.mP2eiRPMBlank);
		bEngineInfoPropPitch = createLCGroup(topPanel, lang.mP2eiPropPitch);
		createvoidWebLabel(topPanel,lang.mP2eiPropPitchBlank);
		bEngineInfoEffEta = createLCGroup(topPanel, lang.mP2eiEffEta);
		createvoidWebLabel(topPanel,lang.mP2eiEffEtaBlank);
		bEngineInfoEffHp = createLCGroup(topPanel, lang.mP2eiEffHp);
		createvoidWebLabel(topPanel,lang.mP2eiEffHpBlank);
		bEngineInfoPressure = createLCGroup(topPanel, lang.mP2eiPressure);
		createvoidWebLabel(topPanel,lang.mP2eiPressureBlank);
		bEngineInfoPowerPercent = createLCGroup(topPanel, lang.mP2eiPowerPercent);
		createvoidWebLabel(topPanel,lang.mP2eiPowerPercentBlank);
		bEngineInfoFuelKg = createLCGroup(topPanel, lang.mP2eiFuelKg);
		createvoidWebLabel(topPanel,lang.mP2eiFuelKgBlank);
		bEngineInfoFuelTime = createLCGroup(topPanel, lang.mP2eiFuelTime);
		createvoidWebLabel(topPanel,lang.mP2eiFuelTimeBlank);
		bEngineInfoWepKg = createLCGroup(topPanel, lang.mP2eiWepKg);
		createvoidWebLabel(topPanel,lang.mP2eiWepKgBlank);
		bEngineInfoWepTime = createLCGroup(topPanel, lang.mP2eiWepTime);
		createvoidWebLabel(topPanel,lang.mP2eiWepTimeBlank);
		bEngineInfoTemp = createLCGroup(topPanel, lang.mP2eiTemp);
		createvoidWebLabel(topPanel,lang.mP2eiTempBlank);
		bEngineInfoOilTemp = createLCGroup(topPanel, lang.mP2eiOilTemp);
		createvoidWebLabel(topPanel,lang.mP2eiOilTempBlank);
		bEngineInfoHeatTolerance = createLCGroup(topPanel, lang.mP2eiHeatTolerance);
		createvoidWebLabel(topPanel,lang.mP2eiHeatToleranceBlank);
		bEngineInfoEngResponse = createLCGroup(topPanel, lang.mP2eiEngResponse);
		createvoidWebLabel(topPanel,lang.mP2eiEngResponseBlank);
		
//		bEngineInfoHp = createLCGroup(topPanel, language.mP4fiIAS);
//		createvoidWebLabel(topPanel,language.mP4fiIASBlank);
//		
		
		// createLCGroup(topPanel, "面板透明度 ");
		// createLBGroup(topPanel);
		fEngineInfoFont = createFontList(topPanel,lang.mP2PanelFont);
		iEngineInfoFontSizeIncr = createLSGroup(topPanel, lang.mP2FontAdjust, -6, 20,200, 1, 4);
		iengineInfoColumnNum = createLSGroup(topPanel, lang.mP4ColumnAdjust, 1, 16,200, 1, 2);

		createvoidWebLabel(topPanel,lang.mP2EngineBlank);
		
		FlowLayout layout = new FlowLayout();
		layout.setAlignment(FlowLayout.LEFT);
		topPanel.setLayout(layout);

		// bottomPanel

		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);
		WebButtonGroup G1 = createLBGroup(bottomPanel);
		bottomPanel.add(G1, BorderLayout.LINE_START);
		jp2.add(splitPane);

	}

	// JP3布局
	public void initJP3() {
		initJP(jp3);

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

		bCrosshairSwitch = createLCGroup(topPanel, lang.mP3Crosshair);
		createvoidWebLabel(topPanel,lang.mP3CrosshairBlank);
		bcrosshairdisplaySwitch = createLCGroup(topPanel, lang.mP3CrosshairDisplay);
		createvoidWebLabel(topPanel,lang.mP3CrosshairDisplayBlank);
		createvoidWebLabel(topPanel,lang.mP3CrosshairBlank);
		bDrawHudTextSwitch=createLCGroup(topPanel, lang.mP3Text);
		createvoidWebLabel(topPanel,lang.mP3TextBlank);
		bTextureCrosshairSwitch = createLCGroup(topPanel, lang.mP3CrosshairTexture);
		createvoidWebLabel(topPanel,lang.mP3CrosshairTextureBlank);
		sCrosshairName = createCrosshairList(topPanel,lang.mP3ChooseTexture);
		createvoidWebLabel(topPanel,lang.mP3ChooseTextureBlank);
		iCrosshairScale = createLSGroup(topPanel, lang.mP3CrosshairSize, 0, 200,500, 5, 20);
		
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
		jp3.add(splitPane);
	}

	public void initJP4(){
		initJP(jp4);

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

		bFlightInfoSwitch = createLCGroup(topPanel, lang.mP4FlightInfoPanel);
		createvoidWebLabel(topPanel,lang.mP4FlightInfoBlank);
		bFlightInfoEdge = createLCGroup(topPanel, lang.mP4FlightInfoGlassEdge);
		createvoidWebLabel(topPanel,lang.mP4FlightInfoGlassEdgeBlank);
		
		battitudeIndicatorSwitch = createLCGroup(topPanel, lang.mP4attitudeIndicatorPanel);
		createvoidWebLabel(topPanel,lang.mP4attitudeIndicatorPanelBlank);
		
		bFMPrintSwitch = createLCGroup(topPanel, lang.mP4FMPanel);
		createvoidWebLabel(topPanel,lang.mP4FMPanelBlank);
		
		bFlightInfoIAS = createLCGroup(topPanel, lang.mP4fiIAS);
		createvoidWebLabel(topPanel,lang.mP4fiIASBlank);
		
		bFlightInfoTAS = createLCGroup(topPanel, lang.mP4fiTAS);
		createvoidWebLabel(topPanel,lang.mP4fiIASBlank);
		
		bFlightInfoMach = createLCGroup(topPanel, lang.mP4fiMach);
		createvoidWebLabel(topPanel,lang.mP4fiMachBlank);
		
		bFlightInfoCompass = createLCGroup(topPanel, lang.mP4fiCompass);
		createvoidWebLabel(topPanel,lang.mP4fiCompassBlank);
		
		bFlightInfoHeight = createLCGroup(topPanel, lang.mP4fiHeight);
		createvoidWebLabel(topPanel,lang.mP4fiHeightBlank);

		
		bFlightInfoVario = createLCGroup(topPanel, lang.mP4fiVario);
		createvoidWebLabel(topPanel,lang.mP4fiVarioBlank);
		
		bFlightInfoSEP = createLCGroup(topPanel, lang.mP4fiSEP);
		createvoidWebLabel(topPanel,lang.mP4fiSEPBlank);
		
		bFlightInfoAcc = createLCGroup(topPanel, lang.mP4fiAcc);
		createvoidWebLabel(topPanel,lang.mP4fiAccBlank);
		
		bFlightInfoWx = createLCGroup(topPanel, lang.mP4fiWx);
		createvoidWebLabel(topPanel,lang.mP4fiWxBlank);
		
		bFlightInfoNy = createLCGroup(topPanel, lang.mP4fiNy);
		createvoidWebLabel(topPanel,lang.mP4fiNyBlank);
		
		bFlightInfoTurn = createLCGroup(topPanel, lang.mP4fiTurn);
		createvoidWebLabel(topPanel,lang.mP4fiTurnBlank);
		
		bFlightInfoTurnRadius = createLCGroup(topPanel, lang.mP4fiTurnRadius);
		createvoidWebLabel(topPanel,lang.mP4fiTurnRadiusBlank);
		
		bFlightInfoAoA = createLCGroup(topPanel, lang.mP4fiAoA);
		createvoidWebLabel(topPanel,lang.mP4fiAoABlank);
		
		bFlightInfoAoS = createLCGroup(topPanel, lang.mP4fiAoS);
		createvoidWebLabel(topPanel,lang.mP4fiAoSBlank);
		
		bFlightInfoWingSweep = createLCGroup(topPanel, lang.mP4fiWingSweep);
		createvoidWebLabel(topPanel,lang.mP4fiWingSweepBlank);
		

		bFlightInfoRadioAlt = createLCGroup(topPanel, lang.mP4fiRadioAlt);
		createvoidWebLabel(topPanel,lang.mP4fiRadioAltBlank);
		
		// createLCGroup(topPanel, "面板透明度 ");
		// createLBGroup(topPanel);
		sFlightInfoFont = createFontList(topPanel,lang.mP4PanelFont);
		iFlightInfoFontSizeIncr = createLSGroup(topPanel, lang.mP4FontAdjust, -6, 20,200, 1, 4);
		iflightInfoColumnNum = createLSGroup(topPanel, lang.mP4ColumnAdjust, 1, 16,200, 1, 2);
		FlowLayout layout = new FlowLayout();
		layout.setAlignment(FlowLayout.LEFT);
		topPanel.setLayout(layout);

		// bottomPanel

		WebButtonGroup G = createbuttonGroup();
		bottomPanel.add(G, BorderLayout.LINE_END);
		WebButtonGroup G1 = createLBGroup(bottomPanel);
		bottomPanel.add(G1, BorderLayout.LINE_START);
		
		jp4.add(splitPane);
	}
	
	public void initJP5() {
		initJP(jp5);
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
		bEnableLogging = createLCGroup(topPanel, lang.mP5LoggingAndCharting);
		createvoidWebLabel(topPanel,lang.mP5LoggingAndChartingBlank);
		bEnableInformation= createLCGroup(topPanel, lang.mP5Information);
		createvoidWebLabel(topPanel,lang.mP5InformationBlank);
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

		jp5.add(splitPane);

	}
	public void initJP6() {
		initJP(jp6);
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
		bEnableAxis = createLCGroup(topPanel, lang.mP6AxisPanel);
		createvoidWebLabel(topPanel,lang.mP6AxisPanelBlank);
		bEnableAxisEdge = createLCGroup(topPanel, lang.mP6AxisEdge);
		createvoidWebLabel(topPanel,lang.mP6AxisEdgeBlank);
		bEnablegearAndFlaps=createLCGroup(topPanel, lang.mP6GearAndFlaps);
		bEnablegearAndFlapsEdge=createLCGroup(topPanel, lang.mP6GearAndFlapsEdge);
		createvoidWebLabel(topPanel,lang.mP6GearAndFlapsEdgeBlank);
		benableEngineControl = createLCGroup(topPanel, lang.mP6engineControl);
		createvoidWebLabel(topPanel,lang.mP6engineControlBlank);
		
		// 引擎控制


		// 油门
		bEngineControlThrottle = createLCGroup(topPanel, lang.mP6ecThrottle);
		createvoidWebLabel(topPanel,lang.mP6ecThrottleBlank);
		// 桨距
		bEngineControlPitch = createLCGroup(topPanel, lang.mP6ecPitch);
		createvoidWebLabel(topPanel,lang.mP6ecPitchBlank);
		// 混合比
		bEngineControlMixture = createLCGroup(topPanel, lang.mP6ecMixture);
		createvoidWebLabel(topPanel,lang.mP6ecMixtureBlank);
		// 散热器
		bEngineControlRadiator = createLCGroup(topPanel, lang.mP6ecRadiator);
		createvoidWebLabel(topPanel,lang.mP6ecRadiatorBlank);
		// 增压器
		bEngineControlCompressor = createLCGroup(topPanel, lang.mP6ecCompressor);
		createvoidWebLabel(topPanel,lang.mP6ecCompressorBlank);
		// 燃油量
		bEngineControlLFuel = createLCGroup(topPanel, lang.mP6ecLFuel);
		createvoidWebLabel(topPanel,lang.mP6ecLFuelBlank);
		
		
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

		jp6.add(splitPane);

	}
	
	public void initPanel() {
		WebTabbedPane tabbedPane = new WebTabbedPane();
		// tabbedPane3.setPreferredSize ( new Dimension ( 150, 120 ) );
		tabbedPane.setTabPlacement(WebTabbedPane.LEFT);
		// tabbedPane.setTabLayoutPolicy(JTabbedPane.SCROLL_TAB_LAYOUT);

		jp1 = new WebPanel();
		jp2 = new WebPanel();
		jp3 = new WebPanel();
		jp4 = new WebPanel();
		jp5 = new WebPanel();
		jp6 = new WebPanel();
		initJP1();
		initJP2();
		if(!app.foreignLanguage)initJP3();
		initJP4();
		initJP5();
		initJP6();
		
		tabbedPane.addTab(lang.mFlightInfo, jp4);
		tabbedPane.addTab(lang.mEngineInfo, jp2);
		tabbedPane.addTab(lang.mControlInfo, jp6);
		tabbedPane.addTab(lang.mLoggingAndAnalysis, jp5);
		if(!app.foreignLanguage)tabbedPane.addTab(lang.mCrosshair, jp3);
		tabbedPane.addTab(lang.mAdvancedOption, jp1);
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

		bFlightInfoSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("flightInfoSwitch")));
		bFlightInfoEdge.setSelected(Boolean.parseBoolean(tc.getconfig("flightInfoEdge")));
		sFlightInfoFont.setSelectedItem(tc.getconfig("flightInfoFontC"));
		iFlightInfoFontSizeIncr.setValue(Integer.parseInt(tc.getconfig("flightInfoFontaddC")));
		battitudeIndicatorSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("enableAttitudeIndicator")));
		bFMPrintSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("enableFMPrint")));
		
		bFlightInfoIAS.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoIAS")));
		bFlightInfoTAS.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoTAS")));
		bFlightInfoMach.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoMach")));
		bFlightInfoCompass.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoCompass")));
		bFlightInfoHeight.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoHeight")));
		bFlightInfoVario.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoVario")));
		bFlightInfoSEP.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoSEP")));
		bFlightInfoAcc.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoAcc")));
		bFlightInfoWx.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoWx")));
		bFlightInfoNy.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoNy")));
		bFlightInfoTurn.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoTurn")));
		bFlightInfoTurnRadius.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoTurnRadius")));
		bFlightInfoAoA.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoAoA")));
		bFlightInfoAoS.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoAoS")));
		bFlightInfoWingSweep.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoWingSweep")));
		bFlightInfoRadioAlt.setSelected(!Boolean.parseBoolean(tc.getconfig("disableFlightInfoRadioAlt")));
		
		iflightInfoColumnNum.setValue(Integer.parseInt(tc.getconfig("flightInfoColumn")));
		
		
		bEngineInfoSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("engineInfoSwitch")));
		bEngineInfoEdge.setSelected(Boolean.parseBoolean(tc.getconfig("engineInfoEdge")));
		fEngineInfoFont.setSelectedItem(tc.getconfig("engineInfoFont"));
		iEngineInfoFontSizeIncr.setValue(Integer.parseInt(tc.getconfig("engineInfoFontadd")));
		iengineInfoColumnNum.setValue(Integer.parseInt(tc.getconfig("engineInfoColumn")));
		
		bEngineInfoHorsePower.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoHorsePower")));
		bEngineInfoThrust.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoThrust")));
        bEngineInfoRPM.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoRPM")));
        bEngineInfoPropPitch.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoPropPitch")));
        bEngineInfoEffEta.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoEffEta")));
        bEngineInfoEffHp.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoEffHp")));
        bEngineInfoPressure.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoPressure")));
        bEngineInfoPowerPercent.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoPowerPercent")));
        bEngineInfoFuelKg.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoFuelKg")));
        bEngineInfoFuelTime.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoFuelTime")));
        bEngineInfoWepKg.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoWepKg")));
        bEngineInfoWepTime.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoWepTime")));
        bEngineInfoTemp.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoTemp")));
        bEngineInfoOilTemp.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoOilTemp")));
        bEngineInfoHeatTolerance.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoHeatTolerance")));
        bEngineInfoEngResponse.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoEngResponse")));
        

		
		
		if(!app.foreignLanguage)bCrosshairSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("crosshairSwitch")));
		if(!app.foreignLanguage)iCrosshairScale.setValue(Integer.parseInt(tc.getconfig("crosshairScale")));
		if(!app.foreignLanguage)bTextureCrosshairSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("usetexturecrosshair")));
		if(!app.foreignLanguage)sCrosshairName.setSelectedItem(tc.getconfig("crosshairName"));
		if(!app.foreignLanguage)bDrawHudTextSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("drawHUDtext")));
		if(!app.foreignLanguage)bcrosshairdisplaySwitch.setSelected(Boolean.parseBoolean(tc.getconfig("displayCrosshair")));

		bdrawShadeSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("simpleFont")));
		bAAEnable.setSelected(Boolean.parseBoolean(tc.getconfig("AAEnable")));
		bvoiceWarningSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("enableVoiceWarn")));
		if(app.debug)bTempInfoSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("usetempInfoSwitch")));
		sGlobalNumFont.setSelectedItem(tc.getconfig("GlobalNumFont"));
		iInterval.setValue(Integer.parseInt(tc.getconfig("Interval")));
	
		bstatusSwitch.setSelected(Boolean.parseBoolean(tc.getconfig("enableStatusBar")));
		ivoiceVolume.setValue(Integer.parseInt(tc.getconfig("voiceVolume")));
//		tc.setconfig("enableStatusBar", Boolean.toString(bstatusSwitch.isSelected()));
//		tc.setconfig("voiceVolumn", Integer.toString(ivoiceVolume.getValue()));
		
		
		// 颜色
		cNumColor.setText(getColorText(tc.getColorConfig("fontNum")));
		updateColorGroupColor(cNumColor);

		cLabelColor.setText(getColorText(tc.getColorConfig("fontLabel")));
		updateColorGroupColor(cLabelColor);

		cUnitColor.setText(getColorText(tc.getColorConfig("fontUnit")));
		updateColorGroupColor(cUnitColor);

		cWarnColor.setText(getColorText(tc.getColorConfig("fontWarn")));
		updateColorGroupColor(cWarnColor);

		cShadeColor.setText(getColorText(tc.getColorConfig("fontShade")));
		updateColorGroupColor(cShadeColor);
		
		
		bEnableLogging.setSelected(Boolean.parseBoolean(tc.getconfig("enableLogging")));
		bEnableInformation.setSelected(Boolean.parseBoolean(tc.getconfig("enableAltInformation")));
		
		bEnableAxis.setSelected(Boolean.parseBoolean(tc.getconfig("enableAxis")));
		bEnableAxisEdge.setSelected(Boolean.parseBoolean(tc.getconfig("enableAxisEdge")));
		bEnablegearAndFlaps.setSelected(Boolean.parseBoolean(tc.getconfig("enablegearAndFlaps")));
		bEnablegearAndFlapsEdge.setSelected(Boolean.parseBoolean(tc.getconfig("enablegearAndFlapsEdge")));
		benableEngineControl.setSelected(Boolean.parseBoolean(tc.getconfig("enableEngineControl")));
		

		bEngineControlThrottle.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoThrottle")));
		bEngineControlPitch.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoPitch")));
		bEngineControlMixture.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoMixture")));
		bEngineControlRadiator.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoRadiator")));
		bEngineControlCompressor.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoCompressor")));
		bEngineControlLFuel.setSelected(!Boolean.parseBoolean(tc.getconfig("disableEngineInfoLFuel")));
	}
	
	public void config_init() {
		tc.setconfig("flightInfoSwitch", Boolean.toString(Boolean.TRUE));
		tc.setconfig("flightInfoEdge", Boolean.toString(Boolean.FALSE));
		tc.setconfig("flightInfoFontC", app.DefaultFontName);
		tc.setconfig("flightInfoFontaddC", Integer.toString(0));
		
		
		tc.setconfig("engineInfoSwitch", Boolean.toString(bEngineInfoSwitch.isSelected()));
		tc.setconfig("engineInfoEdge", Boolean.toString(Boolean.FALSE));
		tc.setconfig("engineInfoFont", fEngineInfoFont.getSelectedItem().toString());
		tc.setconfig("engineInfoFontadd", Integer.toString(iEngineInfoFontSizeIncr.getValue()));
		
		if(!app.foreignLanguage)tc.setconfig("crosshairSwitch", Boolean.toString(Boolean.FALSE));
		if(!app.foreignLanguage)tc.setconfig("crosshairScale", Integer.toString(10));
		if(!app.foreignLanguage)tc.setconfig("usetexturecrosshair", Boolean.toString(Boolean.FALSE));
		if(!app.foreignLanguage)tc.setconfig("crosshairName", sCrosshairName.getSelectedItem().toString());
		if(!app.foreignLanguage)tc.setconfig("drawHUDtext",  Boolean.toString(Boolean.FALSE));
		if(!app.foreignLanguage)tc.setconfig("displayCrossharir", Boolean.toString(Boolean.FALSE));
		
		if(app.debug)tc.setconfig("usetempInfoSwitch", Boolean.toString(Boolean.FALSE));
//		tc.setconfig(", value);
		tc.setconfig("GlobalNumFont", app.DefaultNumfontName);
		tc.setconfig("Interval", Integer.toString(80));

		tc.setconfig("enableLogging", Boolean.toString(Boolean.FALSE));
		tc.setconfig("enableAltInformation", Boolean.toString(Boolean.FALSE));
		
		tc.setconfig("enableAxis", Boolean.toString(Boolean.FALSE));
		tc.setconfig("enableAxisEdge", Boolean.toString(Boolean.FALSE));
		tc.setconfig("enablegearAndFlaps", Boolean.toString(Boolean.FALSE));
		tc.setconfig("enablegearAndFlapsEdge", Boolean.toString(Boolean.FALSE));
		
	}
	
	public void saveconfig() {
		// System.out.println(Boolean.toString(bengineInfoSwitch.isSelected()));
		
		tc.setconfig("flightInfoSwitch", Boolean.toString(bFlightInfoSwitch.isSelected()));
		tc.setconfig("flightInfoEdge", Boolean.toString(bFlightInfoEdge.isSelected()));
		tc.setconfig("enableAttitudeIndicator", Boolean.toString(battitudeIndicatorSwitch.isSelected()));
		
		tc.setconfig("enableFMPrint", Boolean.toString(bFMPrintSwitch.isSelected()));
		
		tc.setconfig("disableFlightInfoIAS", Boolean.toString(!bFlightInfoIAS.isSelected()));
		tc.setconfig("disableFlightInfoTAS", Boolean.toString(!bFlightInfoTAS.isSelected()));
		tc.setconfig("disableFlightInfoMach", Boolean.toString(!bFlightInfoMach.isSelected()));
		tc.setconfig("disableFlightInfoCompass", Boolean.toString(!bFlightInfoCompass.isSelected()));
		tc.setconfig("disableFlightInfoHeight", Boolean.toString(!bFlightInfoHeight.isSelected()));
		tc.setconfig("disableFlightInfoVario", Boolean.toString(!bFlightInfoVario.isSelected()));
		
		tc.setconfig("disableFlightInfoSEP", Boolean.toString(!bFlightInfoSEP.isSelected()));
		
		tc.setconfig("disableFlightInfoAcc", Boolean.toString(!bFlightInfoAcc.isSelected()));
		
		tc.setconfig("disableFlightInfoVario", Boolean.toString(!bFlightInfoVario.isSelected()));
		tc.setconfig("disableFlightInfoWx", Boolean.toString(!bFlightInfoWx.isSelected()));
		tc.setconfig("disableFlightInfoNy", Boolean.toString(!bFlightInfoNy.isSelected()));
		tc.setconfig("disableFlightInfoTurn", Boolean.toString(!bFlightInfoTurn.isSelected()));
		tc.setconfig("disableFlightInfoTurnRadius", Boolean.toString(!bFlightInfoTurnRadius.isSelected()));
		
		tc.setconfig("disableFlightInfoAoA", Boolean.toString(!bFlightInfoAoA.isSelected()));
		tc.setconfig("disableFlightInfoAoS", Boolean.toString(!bFlightInfoAoS.isSelected()));
		tc.setconfig("disableFlightInfoWingSweep", Boolean.toString(!bFlightInfoWingSweep.isSelected()));
		tc.setconfig("disableFlightInfoRadioAlt", Boolean.toString(!bFlightInfoRadioAlt.isSelected()));
		
		
		tc.setconfig("flightInfoColumn", Integer.toString(iflightInfoColumnNum.getValue()));
		//System.out.println(sengineInfoFont.getSelectedItem().toString());
		tc.setconfig("flightInfoFontC", sFlightInfoFont.getSelectedItem().toString());
		tc.setconfig("flightInfoFontaddC", Integer.toString(iFlightInfoFontSizeIncr.getValue()));
		
		tc.setconfig("engineInfoSwitch", Boolean.toString(bEngineInfoSwitch.isSelected()));
		tc.setconfig("engineInfoEdge", Boolean.toString(bEngineInfoEdge.isSelected()));
		// System.out.println(sengineInfoFont.getSelectedValue());
		tc.setconfig("engineInfoFont", fEngineInfoFont.getSelectedItem().toString());
		tc.setconfig("engineInfoFontadd", Integer.toString(iEngineInfoFontSizeIncr.getValue()));
		tc.setconfig("engineInfoColumn", Integer.toString(iengineInfoColumnNum.getValue()));
		
		
		tc.setconfig("disableEngineInfoHorsePower", Boolean.toString(!bEngineInfoHorsePower.isSelected()));
		tc.setconfig("disableEngineInfoThrust", Boolean.toString(!bEngineInfoThrust.isSelected()));
		tc.setconfig("disableEngineInfoRPM", Boolean.toString(!bEngineInfoRPM.isSelected()));
		tc.setconfig("disableEngineInfoPropPitch", Boolean.toString(!bEngineInfoPropPitch.isSelected()));
		tc.setconfig("disableEngineInfoEffEta", Boolean.toString(!bEngineInfoEffEta.isSelected()));
		tc.setconfig("disableEngineInfoEffHp", Boolean.toString(!bEngineInfoEffHp.isSelected()));
		tc.setconfig("disableEngineInfoPressure", Boolean.toString(!bEngineInfoPressure.isSelected()));
		tc.setconfig("disableEngineInfoPowerPercent", Boolean.toString(!bEngineInfoPowerPercent.isSelected()));
		tc.setconfig("disableEngineInfoFuelKg", Boolean.toString(!bEngineInfoFuelKg.isSelected()));
		tc.setconfig("disableEngineInfoFuelTime", Boolean.toString(!bEngineInfoFuelTime.isSelected()));
		tc.setconfig("disableEngineInfoWepKg", Boolean.toString(!bEngineInfoWepKg.isSelected()));
		tc.setconfig("disableEngineInfoWepTime", Boolean.toString(!bEngineInfoWepTime.isSelected()));
		tc.setconfig("disableEngineInfoTemp", Boolean.toString(!bEngineInfoTemp.isSelected()));
		tc.setconfig("disableEngineInfoOilTemp", Boolean.toString(!bEngineInfoOilTemp.isSelected()));
		tc.setconfig("disableEngineInfoHeatTolerance", Boolean.toString(!bEngineInfoHeatTolerance.isSelected()));
		tc.setconfig("disableEngineInfoEngResponse", Boolean.toString(!bEngineInfoEngResponse.isSelected()));
		
		if(!app.foreignLanguage)tc.setconfig("crosshairSwitch", Boolean.toString(bCrosshairSwitch.isSelected()));
		if(!app.foreignLanguage)tc.setconfig("crosshairScale", Integer.toString(iCrosshairScale.getValue()));
		if(!app.foreignLanguage)tc.setconfig("usetexturecrosshair", Boolean.toString(bTextureCrosshairSwitch.isSelected()));
		if(!app.foreignLanguage)tc.setconfig("crosshairName", sCrosshairName.getSelectedItem().toString());
		if(!app.foreignLanguage)tc.setconfig("drawHUDtext",  Boolean.toString(bDrawHudTextSwitch.isSelected()));
		if(!app.foreignLanguage)tc.setconfig("displayCrosshair", Boolean.toString(bcrosshairdisplaySwitch.isSelected()));
		
		
		
		tc.setconfig("simpleFont", Boolean.toString(bdrawShadeSwitch.isSelected()));
		tc.setconfig("AAEnable", Boolean.toString(bAAEnable.isSelected()));
		
		tc.setconfig("enableVoiceWarn", Boolean.toString(bvoiceWarningSwitch.isSelected()));
		if(app.debug)tc.setconfig("usetempInfoSwitch", Boolean.toString(bTempInfoSwitch.isSelected()));
		tc.setconfig("GlobalNumFont",sGlobalNumFont.getSelectedItem().toString());
		tc.setconfig("Interval", Integer.toString(iInterval.getValue()));
		
		tc.setconfig("enableStatusBar", Boolean.toString(bstatusSwitch.isSelected()));
		tc.setconfig("voiceVolume", Integer.toString(ivoiceVolume.getValue()));
		
		
		// 颜色
		tc.setColorConfig("fontNum" ,textToColor(cNumColor.getText()));
		tc.setColorConfig("fontLabel" ,textToColor(cLabelColor.getText()));
		tc.setColorConfig("fontUnit" ,textToColor(cUnitColor.getText()));
		tc.setColorConfig("fontWarn" ,textToColor(cWarnColor.getText()));
		tc.setColorConfig("fontShade" ,textToColor(cShadeColor.getText()));
		
		
		
		tc.setconfig("enableLogging", Boolean.toString(bEnableLogging.isSelected()));
		tc.setconfig("enableAltInformation", Boolean.toString(bEnableInformation.isSelected()));
		
		tc.setconfig("enableAxis", Boolean.toString(bEnableAxis.isSelected()));
		tc.setconfig("enableAxisEdge", Boolean.toString(bEnableAxisEdge.isSelected()));
		tc.setconfig("enablegearAndFlaps", Boolean.toString(bEnablegearAndFlaps.isSelected()));
		tc.setconfig("enablegearAndFlapsEdge", Boolean.toString(bEnablegearAndFlapsEdge.isSelected()));
		tc.setconfig("enableEngineControl", Boolean.toString(benableEngineControl.isSelected()));
		

		tc.setconfig("disableEngineInfoThrottle", Boolean.toString(!bEngineControlThrottle.isSelected()));
		tc.setconfig("disableEngineInfoPitch", Boolean.toString(!bEngineControlPitch.isSelected()));
		tc.setconfig("disableEngineInfoMixture", Boolean.toString(!bEngineControlMixture.isSelected()));
		tc.setconfig("disableEngineInfoRadiator", Boolean.toString(!bEngineControlRadiator.isSelected()));
		tc.setconfig("disableEngineInfoCompressor", Boolean.toString(!bEngineControlCompressor.isSelected()));
		tc.setconfig("disableEngineInfoLFuel", Boolean.toString(!bEngineControlLFuel.isSelected()));
		
	}

	public void confirm() {
		saveconfig();
		tc.saveconfig();
		tc.loadFromConfig();
		
		this.setVisible(false);
		tc.flag = 1;
		tc.start();
//		this.dispose();
	}

	public mainform(controller c) {
		//System.setProperty("awt.useSystemAAFontSettings", "on");
		//System.out.println("mainForm初始化了");
		width = 800;
		height = 480;
		doit = true;
		Image I=Toolkit.getDefaultToolkit().getImage("image/form1.png");
		I=I.getScaledInstance(32, 32,  Image.SCALE_SMOOTH);
		this.setIconImage(I);
		
		tc = c;
		moveCheckFlag = false;
		
		
		
		this.setUndecorated(true);
		this.setLocation(app.ScreenWidth/2-width/2, app.ScreenHeight/2-height/2);
		this.setFont(app.DefaultFont);
		this.setSize(width, height);
		
//		setFrameOpaque();// 窗口透明
		String ti=app.appName +" v"+ app.version;
		if(app.debug)ti=ti+"beta";
//		ti=ti+"　　————"+app.appTooltips;
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
		
		root = this.getContentPane();
		
		setDrawWatermark(true);
		setWatermark(new ImageIcon("image/watermark.png"));
		
//		this.getTitleComponent().setForeground(new Color(0,0,0,255));
		// this.getWebRootPaneUI().getWindowButtons().getWebButton(2).setBorderPainted(false);
		initPanel();
		initConfig();// 读入Config
		
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setVisible(true);
		this.setShadeWidth(10);
		
	}
	

	@Override
	public void run() {
		// TODO Auto-generated method stub
		while (doit) {
			try {
				Thread.sleep(33);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			gcCount++;
			// System.out.println("As");
			//System.out.println("MainFrame执行了");
			root.repaint();
			if(gcCount==100){
				//System.out.println("MainFrameGC");
				System.gc();
				gcCount=0;
			}
			
		}
//		tc.loadFromConfig();
	}
}