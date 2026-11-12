package ui.overlay;

import java.awt.BorderLayout;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Image;
import java.awt.RenderingHints;
import java.awt.image.BufferedImage;
import java.io.File;

import javax.imageio.ImageIO;

import com.alee.laf.panel.WebPanel;

import prog.Application;
import prog.Controller;
import prog.Service;
import prog.config.OverlaySettings;
import prog.util.Logger;
import ui.WebLafSettings;
import ui.base.DraggableOverlay;
import ui.component.CrosshairGauge;
import ui.util.GraphicsUtil;
import ui.util.OverlayStyleHelper;

/**
 * 独立准星 overlay，与 MiniHUD 解耦。
 *
 * 纯静态绘制，不订阅 FlightDataBus。配置面板「准星」的 x/y 表示窗口中心
 * （而非左上角），保证默认落在屏幕中心，且调整大小时围绕中心缩放。
 */
public class CrosshairOverlay extends DraggableOverlay {

    private static final long serialVersionUID = 1L;
    private static final String SOFTWARE_CROSSHAIR = "软件渲染准星";

    private OverlaySettings settings;
    private CrosshairGauge gauge;
    private WebPanel content;
    private boolean preview;

    private int scale = 1;
    private int pad = 2;
    private int winSize = 4;
    private boolean textureMode;

    // 贴图缓存：避免 reinitConfig 反复读盘 + 缩放
    private Image cachedTexture;
    private String cachedTextureName;
    private int cachedTextureScale = -1;

    public CrosshairOverlay() {
        super();
        setTitle("crosshair");
    }

    public void setFrameOpaque() {
        OverlayStyleHelper.applyTransparentStyle(this);
    }

    public void init(Controller c, Service s, OverlaySettings overlaySettings) {
        this.settings = overlaySettings;
        setOverlaySettings(overlaySettings);
        setLayout(new BorderLayout());
        setFrameOpaque();

        gauge = new CrosshairGauge();
        content = new WebPanel() {
            private static final long serialVersionUID = -9061280572815010060L;

            @Override
            protected void paintComponent(Graphics g) {
                Graphics2D g2d = (Graphics2D) g;
                GraphicsUtil.configureOverlayRendering(g2d);
                g2d.setPaintMode();
                if (gauge == null)
                    return;
                // 矢量准星内部以 (x + width/2) 为中心，臂向两侧各延伸 width，
                // 因此绘制原点要再偏移 scale/2 才能让外接框铺满窗口
                if (textureMode) {
                    gauge.draw(g2d, pad, pad);
                } else {
                    gauge.draw(g2d, pad + scale / 2, pad + scale / 2);
                }
            }
        };
        content.setOpaque(false);
        content.setWebColoredBackground(false);
        setContentPane(content);

        // 透明 + 不抢焦点 + 置顶注册 + 空白光标（必须在设置 layout 之后）
        WebLafSettings.setWindowOpaque(this);

        reinitConfig();

        if (s != null) {
            setVisible(true);
        }
    }

    public void initPreview(Controller c, OverlaySettings overlaySettings) {
        preview = true;
        init(c, null, overlaySettings);
        applyPreviewStyle();
        setupDragListeners();
        setVisible(true);
    }

    public void reinitConfig() {
        if (settings == null || gauge == null) {
            return;
        }

        int base = settings.getInt("crosshairSize", 113);
        if (base < 1) {
            base = 1;
        }
        scale = (int) Math.round(base * Application.dpiScale);
        if (scale < 1) {
            scale = 1;
        }

        // 矢量准星自然外接 2*scale（圆只有 scale），贴图同样是 2*scale
        pad = Math.max(2, scale / 30);
        winSize = scale * 2 + pad * 2;

        String name = settings.getString("crosshairName", SOFTWARE_CROSSHAIR);
        if (name == null) {
            name = "";
        }
        name = name.trim();
        Image texture = null;
        if (!name.isEmpty() && !SOFTWARE_CROSSHAIR.equalsIgnoreCase(name)) {
            texture = loadTexture(name, scale * 2);
        }
        if (texture != null) {
            textureMode = true;
            gauge.setTextureStyle(true, texture, scale);
        } else {
            // 贴图缺失或选软件准星：显式回到矢量样式。
            // （旧路径 texture=null 时 width 仍为 0，会什么都画不出来）
            textureMode = false;
            gauge.setStyleContext(scale);
        }

        // 面板 x/y 表示窗口中心
        int cx = settings.getWindowX(winSize);
        int cy = settings.getWindowY(winSize);
        setBounds(cx - winSize / 2, cy - winSize / 2, winSize, winSize);

        if (preview) {
            applyPreviewStyle();
        } else {
            setFrameOpaque();
        }
        repaint();
    }

    @Override
    public void saveCurrentPosition() {
        if (settings == null) {
            return;
        }
        // 保存中心点，与 reinitConfig 的中心语义一致
        settings.saveWindowPosition(getX() + winSize / 2.0, getY() + winSize / 2.0);
    }

    /**
     * 加载并缩放准星贴图，按 (名称, 目标尺寸) 缓存。
     */
    private Image loadTexture(String name, int targetSize) {
        if (cachedTexture != null && name.equals(cachedTextureName) && targetSize == cachedTextureScale) {
            return cachedTexture;
        }
        File imgFile = new File("image/gunsight/" + name + ".png");
        if (!imgFile.exists()) {
            return null;
        }
        try {
            Image raw = ImageIO.read(imgFile);
            if (raw == null) {
                return null;
            }
            BufferedImage scaledImg = new BufferedImage(targetSize, targetSize, BufferedImage.TYPE_INT_ARGB);
            Graphics2D g2 = scaledImg.createGraphics();
            g2.setRenderingHint(RenderingHints.KEY_INTERPOLATION, RenderingHints.VALUE_INTERPOLATION_BILINEAR);
            g2.drawImage(raw, 0, 0, targetSize, targetSize, null);
            g2.dispose();

            cachedTexture = scaledImg;
            cachedTextureName = name;
            cachedTextureScale = targetSize;
            return scaledImg;
        } catch (Exception e) {
            Logger.info("Crosshair", "Failed to load crosshair: " + e.getMessage());
            return null;
        }
    }

    @Override
    public void run() {
        // 静态绘制，无轮询
    }
}
