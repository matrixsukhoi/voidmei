package ui.overlay;

import java.util.List;

import ui.base.FieldOverlay;
import ui.model.FieldDefinition;
import ui.model.FlightInfoConfig;
import ui.renderer.BOSStyleRenderer;
import ui.renderer.OverlayRenderer;

/**
 * FlightInfo overlay window displaying real-time flight data.
 * 
 * Extends FieldOverlay for event-driven updates.
 * Uses FlightInfoConfig for field definitions and BOSStyleRenderer for display.
 */
public class FlightInfoOverlay extends FieldOverlay {
	private static final long serialVersionUID = 6759127498151892589L;

	private FlightInfoConfig flightInfoConfig;
	private ui.model.TelemetrySource service;

	public FlightInfoOverlay() {
		super();
		setTitle("飞行信息");
	}

	@Override
	protected OverlayRenderer createRenderer() {
		return new BOSStyleRenderer();
	}

	@Override
	protected List<FieldDefinition> getFieldDefinitions() {
		if (flightInfoConfig == null)
			return null;
		return flightInfoConfig.getFieldDefinitions();
	}

	/**
	 * Standardized initialization.
	 */
	public void init(prog.Controller c, prog.Service s, prog.config.OverlaySettings settings) {
		this.service = s;
		// 使用 getConfigProvider() 获取配置接口，而不是直接使用 Controller
		prog.config.ConfigProvider configProvider = c.getConfigProvider();
		this.config = configProvider;
		this.flightInfoConfig = ui.model.FlightInfoConfig.createDefault(configProvider, settings.getGroupConfig());

		// Standardize style/font keys from Config object
		this.numFontKey = flightInfoConfig.numFontKey;
		this.labelFontKey = flightInfoConfig.labelFontKey;
		this.columnKey = flightInfoConfig.columnKey;
		this.edgeKey = flightInfoConfig.edgeKey;
		this.defaultShowEdge = flightInfoConfig.showEdge;
		this.title = flightInfoConfig.title;

		setOverlaySettings(settings);
		super.init(configProvider);

		if (s != null) {
			setVisible(true);
		}
	}

	public void onFlightData(prog.event.FlightDataEvent event) {
		super.onFlightData(event);
	}

	/**
	 * Initialize for preview mode using standardized signature.
	 */
	public void initPreview(prog.Controller c, prog.config.OverlaySettings settings) {
		this.isPreview = true;  // Set BEFORE init() to prevent FlightDataBus subscription
		init(c, null, settings);
		applyPreviewStyle();
		setupDragListeners();
		setVisible(true);
		reinitConfig();  // Required: refreshes preview layout/styling
	}

	@Override
	public void reinitConfig() {
		if (overlaySettings != null) {
			// Refresh Config object from latest data
			this.flightInfoConfig = ui.model.FlightInfoConfig.createDefault(this.config,
					overlaySettings.getGroupConfig());

			// Sync style configuration
			this.numFontKey = flightInfoConfig.numFontKey;
			this.labelFontKey = flightInfoConfig.labelFontKey;
			this.columnKey = flightInfoConfig.columnKey;
			this.edgeKey = flightInfoConfig.edgeKey;
			this.defaultShowEdge = flightInfoConfig.showEdge;
			this.title = flightInfoConfig.title;
		}

		if (flightInfoConfig == null)
			return;

		super.reinitConfig();

		// Bind fields based on current telemetry source
		prog.config.ConfigLoader.GroupConfig groupConfig = flightInfoConfig.groupConfig;
		if (groupConfig != null && this.service != null) {
			bindDynamicFields(this.service, groupConfig.rows);
			repaint();
		}
	}

	private void bindDynamicFields(ui.model.TelemetrySource s,
			java.util.List<prog.config.ConfigLoader.RowConfig> rows) {
		ui.model.FieldManager fm = this.fieldManager;
		for (prog.config.ConfigLoader.RowConfig row : rows) {
			if ("DATA".equals(row.type) && row.property != null && !row.property.isEmpty()) {
				java.util.function.DoubleSupplier valueSupplier = ui.util.ReflectBinder.resolveDouble(s, row.property);

				// 构建 visibilitySupplier
				java.util.function.BooleanSupplier visibilitySupplier = null;

				if (row.visibleWhen != null) {
					// 使用表达式求值器（visibleWhen 是预解析的 SExp 对象）
					final ui.util.VisibilityExpressionEvaluator evaluator =
						new ui.util.VisibilityExpressionEvaluator(row.visibleWhen, s);
					final java.util.function.DoubleSupplier vs = valueSupplier;
					visibilitySupplier = () -> evaluator.evaluate(vs.getAsDouble());
				} else {
					// 回退到自动推断（向后兼容）：getXXX -> isXXXValid
					String baseMethod = row.property.trim();
					if (baseMethod.contains("*")) {
						baseMethod = baseMethod.split("\\*")[0].trim();
					}
					if (baseMethod.startsWith("get")) {
						String validityMethod = "is" + baseMethod.substring(3) + "Valid";
						visibilitySupplier = ui.util.ReflectBinder.resolveBoolean(s, validityMethod);
					}
				}

				fm.bind(row.property, valueSupplier, visibilitySupplier, row.precision, row.format);
				fm.setFieldVisible(row.property, row.getBool());
			}
			if (row.children != null && !row.children.isEmpty()) {
				bindDynamicFields(s, row.children);
			}
		}
	}

	// Manual loadPosition and saveCurrentPosition overrides removed.
	// DraggableOverlay handles this via OverlaySettings automatically.
}