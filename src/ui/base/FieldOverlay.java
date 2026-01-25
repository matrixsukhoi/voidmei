package ui.base;

import java.awt.Color;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.util.List;

import com.alee.laf.panel.WebPanel;
import parser.AttributePool;
import prog.config.ConfigProvider;
import ui.model.DataField;
import ui.model.DefaultFieldManager;
import ui.model.FieldDefinition;
import ui.model.FieldManager;
import ui.renderer.OverlayRenderer;
import ui.renderer.RenderContext;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import java.util.Map;

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
public abstract class FieldOverlay extends DraggableOverlay implements FlightDataListener {
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
    }

    public void initPreview(ConfigProvider config, AttributePool pool) {
        init(config, pool);
        applyPreviewStyle();
        setupDragListeners();
        setVisible(true);
    }

    // --- Configuration ---

    public void reinitConfig() {
        renderContext = RenderContext.fromSettings(overlaySettings, this,
                numFontKey, columnKey, config);

        initFields();
        subscribeToEvents();

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
        int[] pos = loadPosition(width, height);
        this.setBounds(pos[0], pos[1], width, height);

        repaint();
    }

    protected void initFields() {
        fieldManager.clearAll();
        List<FieldDefinition> defs = getFieldDefinitions();
        if (defs == null)
            return;

        for (FieldDefinition def : defs) {
            fieldManager.addField(def.key, def.label, def.unit, def.configKey, def.hideWhenNA, def.exampleValue);
        }
    }

    protected void subscribeToEvents() {
        FlightDataBus.getInstance().register(this);
    }

    @Override
    public void onFlightData(FlightDataEvent event) {
        javax.swing.SwingUtilities.invokeLater(() -> {
            Map<String, String> data = event.getData();
            for (DataField field : fieldManager.getFields()) {
                // Determine if we should process this field (Legacy vs Zero-GC)
                if (field.valueSupplier != null) {
                    // ZERO-GC PATH
                    if (field.visibilitySupplier != null) {
                        field.visible = field.visibilitySupplier.getAsBoolean();
                    }
                    if (field.visible) {
                        double val = field.valueSupplier.getAsDouble();
                        field.length = ui.util.FastNumberFormatter.format(val, field.buffer, field.precision);
                    }
                } else {
                    // LEGACY PATH
                    // Legacy path visibility is handled inside updateField ->
                    // setValueWithVisibility
                    String key = field.key;
                    String val = naString;
                    if (data.containsKey(key)) {
                        val = data.get(key);
                    }
                    fieldManager.updateField(key, val, naString);

                    // Dynamic Unit Support
                    if (data.containsKey(key + "_unit")) {
                        fieldManager.updateFieldUnit(key, data.get(key + "_unit"));
                    }
                }
            }
            if (panel != null) {
                panel.repaint();
            }
        });

    }

    // --- PoolListener Implementation ---

    // --- Legacy Compatibility ---

    public void updateString() {
        // No-op: event-driven
    }

    @Override
    public void dispose() {
        FlightDataBus.getInstance().unregister(this);
        super.dispose();
    }
}
