package ui.base;

import java.awt.Color;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.util.List;

import com.alee.laf.panel.WebPanel;
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
 * Provides:
 * - Event-driven updates via FlightDataBus
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

    // Throttling to prevent EDT task accumulation
    private static final long REFRESH_INTERVAL_MS = 50;
    private long lastRefreshTime = 0;

    // Prevent duplicate event subscriptions
    private boolean subscribed = false;

    // Preview mode flag - when true, skip FlightDataBus subscription
    protected boolean isPreview = false;

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

    public void init(ConfigProvider config) {
        this.config = config;

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

    public void initPreview(ConfigProvider config) {
        this.isPreview = true;  // Set BEFORE init() to prevent event subscription
        init(config);
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
            fieldManager.addField(def.key, def.label, def.unit, def.configKey, def.hideWhenNA, def.hideWhenZero,
                    def.previewValue, def.format);
        }
    }

    protected void subscribeToEvents() {
        if (isPreview) return;  // Preview mode: skip subscription to preserve static preview values
        if (!subscribed) {
            FlightDataBus.getInstance().register(this);
            subscribed = true;
        }
    }

    @Override
    public void onFlightData(FlightDataEvent event) {
        // Throttling prevents EDT task accumulation
        long now = System.currentTimeMillis();
        if (now - lastRefreshTime < REFRESH_INTERVAL_MS) {
            return; // Skip this update, too soon
        }
        lastRefreshTime = now;

        javax.swing.SwingUtilities.invokeLater(() -> {
            Map<String, String> data = event.getData();
            for (DataField field : fieldManager.getFields()) {
                // Determine if we should process this field (Legacy vs Zero-GC)
                if (field.valueSupplier != null) {
                    // ZERO-GC PATH
                    // 1. Fetch value first (needed for visibilitySupplier expression evaluation)
                    double val = field.valueSupplier.getAsDouble();

                    // 2. Determine visibility from supplier
                    // visibilitySupplier 现在可能包含 :visible-when 表达式的求值逻辑，
                    // 会自动处理引擎类型判断、值比较等复杂条件
                    boolean visible = (field.visibilitySupplier == null) || field.visibilitySupplier.getAsBoolean();
                    field.visible = visible;

                    // 3. Dynamic Precision via Supplier
                    if (field.precisionSupplier != null) {
                        int newPrecision = field.precisionSupplier.getAsInt();
                        if (newPrecision != field.precision) {
                            field.precision = newPrecision;
                        }
                    }

                    // 4. Dynamic Unit via Supplier
                    if (field.unitSupplier != null) {
                        String newUnit = field.unitSupplier.get();
                        if (newUnit != null && !newUnit.equals(field.unit)) {
                            field.setUnit(newUnit);
                        }
                    }

                    // 5. Format if visible
                    if (field.visible) {
                        // 5a. 检查 :na-when 条件（条件满足时显示 "-" 而非数值）
                        if (field.naWhenEvaluator != null && field.naWhenEvaluator.evaluate(val)) {
                            // NA 条件满足，显示 "-"
                            field.buffer[0] = '-';
                            field.length = 1;
                        } else if ("TIME_MM_SS".equals(field.format)) {
                            field.length = ui.util.FastNumberFormatter.formatTime(val, field.buffer);
                        } else {
                            field.length = ui.util.FastNumberFormatter.format(val, field.buffer, field.precision);
                        }
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

    // --- Legacy Compatibility ---

    public void updateString() {
        // No-op: event-driven
    }

    @Override
    public void dispose() {
        if (subscribed) {
            FlightDataBus.getInstance().unregister(this);
            subscribed = false;
        }
        super.dispose();
    }
}
