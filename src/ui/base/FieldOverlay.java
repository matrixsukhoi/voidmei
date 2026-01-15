package ui.base;

import java.awt.Color;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.util.List;
import java.util.Set;

import com.alee.laf.panel.WebPanel;
import parser.AttributePool;
import ui.model.ConfigProvider;
import ui.model.DataField;
import ui.model.DefaultFieldManager;
import ui.model.FieldDefinition;
import ui.model.FieldManager;
import ui.renderer.OverlayRenderer;
import ui.renderer.RenderContext;

/**
 * Abstract base class for event-driven data overlay windows.
 * 
 * Provides: - Event-driven updates via AttributePool.PoolListener
 * - Automatic subscription to relevant data keys
 * - Pluggable rendering and configuration
 * 
 * Subclasses must implement:
 * - getConfig(): Return overlay-specific configuration
 * - createRenderer(): Return the renderer to use
 * - getFieldDefinitions(): Return the fields to display
 */
public abstract class FieldOverlay extends DraggableOverlay implements AttributePool.PoolListener {
    private static final long serialVersionUID = 1L;

    protected AttributePool poolSource;
    protected FieldManager fieldManager;
    protected OverlayRenderer renderer;
    protected RenderContext renderContext;
    protected WebPanel panel;
    protected String naString = "-";
    protected int[] renderOffset;

    // Configuration keys - set by subclass
    protected String numFontKey;
    protected String labelFontKey;
    protected String fontAddKey;
    protected String columnKey;
    protected String edgeKey;
    protected boolean defaultShowEdge = false;
    protected String title = "Overlay";

    public FieldOverlay() {
        super();
        this.doit = false; // Disable polling
        this.renderer = createRenderer();
    }

    // --- Abstract Methods (Subclass must implement) ---

    /**
     * Create the renderer for this overlay.
     */
    protected abstract OverlayRenderer createRenderer();

    /**
     * Get the field definitions for this overlay.
     */
    protected abstract List<FieldDefinition> getFieldDefinitions();

    // --- Initialization ---

    public void init(ConfigProvider config, AttributePool pool) {
        this.config = config;
        this.poolSource = pool;

        fieldManager = new DefaultFieldManager(config);
        setupTransparentWindow();
        renderOffset = new int[2];

        panel = new WebPanel() {
            private static final long serialVersionUID = 1L;

            @Override
            public void paintComponent(Graphics g) {
                Graphics2D g2d = (Graphics2D) g;
                if (renderer != null && renderContext != null) {
                    renderer.render(g2d, fieldManager.getFields(), renderContext, renderOffset);
                }
            }
        };
        panel.setOpaque(false);
        panel.setWebColoredBackground(false);
        panel.setBackground(new Color(0, 0, 0, 0));
        this.add(panel);

        setTitle(title);
        reinitConfig();
        setVisible(true);
        reinitConfig();
    }

    public void initPreview(ConfigProvider config, AttributePool pool) {
        init(config, pool);
        applyPreviewStyle();
        setupDragListeners();
        setVisible(true);
        reinitConfig();
    }

    // --- Configuration ---

    public void reinitConfig() {
        renderContext = RenderContext.fromConfig(config, this,
                numFontKey, labelFontKey, fontAddKey, columnKey);

        initFields();
        subscribeToPool();

        boolean showEdge = defaultShowEdge;
        if (config != null && edgeKey != null) {
            String edgeVal = config.getConfig(edgeKey);
            if (edgeVal != null && !edgeVal.isEmpty()) {
                showEdge = "true".equals(edgeVal);
            }
        }
        setShadeWidth(showEdge ? 10 : 0);

        int width = renderContext.getTotalWidth();
        int height = renderContext.getTotalHeight(fieldManager.size());
        int[] pos = loadPosition(0, 0);
        this.setBounds(pos[0], pos[1], width, height);

        repaint();
    }

    protected void initFields() {
        fieldManager.clearAll();
        List<FieldDefinition> defs = getFieldDefinitions();
        if (defs == null)
            return;

        for (FieldDefinition def : defs) {
            fieldManager.addField(def.key, def.label, def.unit, def.configKey, def.hideWhenNA);
        }
    }

    protected void subscribeToPool() {
        if (poolSource == null || fieldManager == null)
            return;

        java.util.ArrayList<String> keys = new java.util.ArrayList<>();
        for (DataField field : fieldManager.getFields()) {
            if (field.visible) {
                keys.add(field.key);
            }
        }

        if (!keys.isEmpty()) {
            poolSource.subscribe(this, keys.toArray(new String[0]));
        }
    }

    // --- PoolListener Implementation ---

    @Override
    public void onPoolUpdated(Set<String> changedKeys) {
        javax.swing.SwingUtilities.invokeLater(() -> {
            for (String key : changedKeys) {
                DataField field = fieldManager.getField(key);
                if (field != null) {
                    String val = naString;
                    if (poolSource != null) {
                        Object obj = poolSource.getValue(key);
                        if (obj != null && !"N/A".equals(obj)) {
                            val = obj.toString();
                        }
                    }
                    fieldManager.updateField(key, val, naString);
                }
            }
            if (panel != null) {
                panel.repaint();
            }
        });
    }

    // --- Legacy Compatibility ---

    public void updateString() {
        // No-op: event-driven
    }

    @Override
    public void dispose() {
        if (poolSource != null) {
            poolSource.unsubscribe(this);
        }
        super.dispose();
    }
}
