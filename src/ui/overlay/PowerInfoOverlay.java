package ui.overlay;

import java.util.List;

import ui.base.FieldOverlay;
import ui.layout.renderer.RowRendererRegistry;
import ui.model.FieldDefinition;
import ui.model.EngineInfoConfig;
import ui.renderer.BOSStyleRenderer;
import ui.renderer.OverlayRenderer;

/**
 * EngineInfo overlay window displaying real-time engine data.
 *
 * Refactored to extend FieldOverlay for consistent event-driven updates.
 * Uses EngineInfoConfig for configuration and field definitions.
 */
public class PowerInfoOverlay extends FieldOverlay {
	private RowRendererRegistry registry;
	private static final long serialVersionUID = 1L;

	private EngineInfoConfig engineInfoConfig;
	private ui.model.TelemetrySource service;

	public PowerInfoOverlay() {
		super();
		this.registry = new RowRendererRegistry();
		setTitle("动力信息");
	}

	@Override
	protected OverlayRenderer createRenderer() {
		return new BOSStyleRenderer();
	}

	@Override
	protected List<FieldDefinition> getFieldDefinitions() {
		if (engineInfoConfig == null)
			return null;
		return engineInfoConfig.getFieldDefinitions();
	}

	/**
	 * Standardized initialization.
	 */
	public void init(prog.Controller c, prog.Service s, prog.config.OverlaySettings settings) {
		this.service = s; // Assign service FIRST so reinitConfig can use it
		// 使用 getConfigProvider() 获取配置接口，而不是直接使用 Controller
		prog.config.ConfigProvider configProvider = c.getConfigProvider();
		this.config = configProvider;
		this.engineInfoConfig = ui.model.EngineInfoConfig.createDefault(configProvider, settings.getGroupConfig());

		// Standardize style/font keys from Config object
		this.numFontKey = engineInfoConfig.numFontKey;
		this.labelFontKey = engineInfoConfig.labelFontKey;
		this.fontAddKey = engineInfoConfig.fontAddKey;
		this.columnKey = engineInfoConfig.columnKey;
		this.edgeKey = engineInfoConfig.edgeKey;
		this.defaultShowEdge = engineInfoConfig.showEdge;
		this.title = engineInfoConfig.title;

		setOverlaySettings(settings);
		super.init(configProvider);

		if (s != null) {
			setVisible(true);
		}
	}

	// Note: Manifold pressure unit/precision switching is now handled by FieldOverlay
	// via unitSupplier/precisionSupplier bound from :unit-source/:precision-source in config

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

	/**
	 * Custom reinitConfig to handle specific EngineInfo visibility logic.
	 */
	@Override
	public void reinitConfig() {
		if (overlaySettings != null) {
			// Refresh Config object from latest data
			this.engineInfoConfig = ui.model.EngineInfoConfig.createDefault(this.config,
					overlaySettings.getGroupConfig());

			// Sync style configuration
			this.numFontKey = engineInfoConfig.numFontKey;
			this.labelFontKey = engineInfoConfig.labelFontKey;
			this.fontAddKey = engineInfoConfig.fontAddKey;
			this.columnKey = engineInfoConfig.columnKey;
			this.edgeKey = engineInfoConfig.edgeKey;
			this.defaultShowEdge = engineInfoConfig.showEdge;
			this.title = engineInfoConfig.title;
		}

		if (engineInfoConfig == null)
			return;

		// 1. Standard FieldOverlay reinit (Fonts, Layout, Window Size)
		super.reinitConfig();

		// 2. Bind the fields to data sources
		prog.config.ConfigLoader.GroupConfig groupConfig = engineInfoConfig.groupConfig;
		if (groupConfig != null && service != null) {
			bindDynamicFields(service, groupConfig.rows);
			repaint();
		}
	}

	// Manual loadPosition and saveCurrentPosition overrides removed.

	/**
	 * Dynamically binds fields based on ConfigLoader.RowConfig items.
	 * 支持 :visible-when 表达式进行精细的可见性控制
	 */
	private void bindDynamicFields(ui.model.TelemetrySource s,
			java.util.List<prog.config.ConfigLoader.RowConfig> rows) {
		ui.model.FieldManager fm = this.fieldManager;

		for (prog.config.ConfigLoader.RowConfig row : rows) {
			// Bind if it's a DATA Item (and has a target)
			if ("DATA".equals(row.type) && row.property != null && !row.property.isEmpty()) {
				// Use ReflectBinder to create a zero-GC supplier
				java.util.function.DoubleSupplier valueSupplier = ui.util.ReflectBinder.resolveDouble(s, row.property);

				// 构建 visibilitySupplier
				// 仅使用 :visible-when 表达式控制可见性，移除自动推断 isXXXValid 机制
				// 原因：自动推断会导致意外的字段隐藏（如转半径 > 9999m 时被隐藏）
				java.util.function.BooleanSupplier visibilitySupplier = null;

				if (row.visibleWhen != null) {
					// 使用表达式求值器（visibleWhen 是预解析的 SExp 对象）
					final ui.util.VisibilityExpressionEvaluator evaluator =
						new ui.util.VisibilityExpressionEvaluator(row.visibleWhen, s);
					final java.util.function.DoubleSupplier vs = valueSupplier;
					visibilitySupplier = () -> evaluator.evaluate(vs.getAsDouble());
				}
				// 没有 :visible-when 的字段默认始终可见

				// Resolve dynamic unit source if specified
				java.util.function.Supplier<String> unitSupplier = null;
				if (row.unitSource != null && !row.unitSource.isEmpty()) {
					unitSupplier = ui.util.ReflectBinder.resolveString(s, row.unitSource);
				}

				// Resolve dynamic precision source if specified
				java.util.function.IntSupplier precisionSupplier = null;
				if (row.precisionSource != null && !row.precisionSource.isEmpty()) {
					precisionSupplier = ui.util.ReflectBinder.resolveInt(s, row.precisionSource);
				}

				// Use extended bind if we have dynamic suppliers, otherwise use standard bind
				if (unitSupplier != null || precisionSupplier != null) {
					fm.bind(row.property, valueSupplier, visibilitySupplier, row.precision, row.format,
							unitSupplier, precisionSupplier);
				} else {
					fm.bind(row.property, valueSupplier, visibilitySupplier, row.precision, row.format);
				}

				// 绑定 :na-when 表达式（条件性显示 "-"）
				if (row.naWhen != null) {
					ui.model.DataField df = fm.getField(row.property);
					if (df != null) {
						df.naWhenEvaluator = new ui.util.VisibilityExpressionEvaluator(row.naWhen, s);
					}
				}

				// Apply visibility immediately based on config value
				fm.setFieldVisible(row.property, row.getBool());
			}

			// Recurse for groups
			if (row.children != null && !row.children.isEmpty()) {
				bindDynamicFields(s, row.children);
			}
		}
	}
}
