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
public class FlightInfo extends FieldOverlay {
	private static final long serialVersionUID = 6759127498151892589L;

	private FlightInfoConfig flightInfoConfig;

	public FlightInfo() {
		super();
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
		this.config = c;
		this.flightInfoConfig = ui.model.FlightInfoConfig.createDefault(c, settings.getGroupConfig());

		// Standardize style/font keys from Config object
		this.numFontKey = flightInfoConfig.numFontKey;
		this.labelFontKey = flightInfoConfig.labelFontKey;
		this.columnKey = flightInfoConfig.columnKey;
		this.edgeKey = flightInfoConfig.edgeKey;
		this.defaultShowEdge = flightInfoConfig.showEdge;
		this.title = flightInfoConfig.title;

		setOverlaySettings(settings);
		super.init(c, c.globalPool);

		if (s != null) {
			bindHighFrequencyFields(s);
			setVisible(true);
		}
	}

	// --- Zero-GC Binding ---
	private void bindHighFrequencyFields(ui.model.TelemetrySource s) {
		ui.model.FieldManager fm = this.fieldManager;
		// Bind fields to service methods
		fm.bind("ias", s::getIAS, 0);
		fm.bind("tas", s::getTAS, 0);
		fm.bind("mach", s::getMach, 2);
		fm.bind("dir", s::getCompass, 0);
		fm.bind("height", s::getAltitude, 0);
		fm.bind("rda", s::getRadioAltitude, s::isRadioAltitudeValid, 0);
		fm.bind("vario", s::getVario, 1);
		fm.bind("sep", s::getSEP, 0);
		fm.bind("acc", s::getAcceleration, 1);
		fm.bind("wx", s::getRollRate, 0);
		fm.bind("ny", s::getNy, 1);

		fm.bind("turn", s::getTurnRate, 1);
		fm.bind("rds", s::getTurnRadius, 0);

		fm.bind("aoa", s::getAoA, 1);
		fm.bind("aos", s::getAoS, 1);
		fm.bind("ws", () -> s.getWingSweep() * 100.0, s::isWingSweepValid, 0);
	}

	/**
	 * Initialize for preview mode using standardized signature.
	 */
	public void initPreview(prog.Controller c, prog.config.OverlaySettings settings) {
		init(c, null, settings);
		applyPreviewStyle();
		setupDragListeners();
		setVisible(true);
		reinitConfig();
	}

	/**
	 * Reinitialize with current config.
	 */
	@Override
	public void reinitConfig() {
		if (flightInfoConfig == null)
			return;
		super.reinitConfig();
	}

	// Manual loadPosition and saveCurrentPosition overrides removed.
	// DraggableOverlay handles this via OverlaySettings automatically.
}