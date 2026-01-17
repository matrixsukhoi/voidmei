package prog.event;

/**
 * Event type constants for UIStateBus.
 * Centralizes all UI State event identifiers for easy discovery and
 * refactoring.
 */
public final class UIStateEvents {

    private UIStateEvents() {
    }

    /**
     * Published when the FM Print switch State changes.
     * Payload: Boolean (new State)
     */
    public static final String FM_PRINT_SWITCH_CHANGED = "fmPrintSwitchChanged";

    /**
     * Published when any configuration value is updated in memory.
     * Payload: String (the config key that changed)
     */
    public static final String CONFIG_CHANGED = "configChanged";

    /**
     * Published when FM data has been successfully parsed and loaded into the
     * global pool.
     * Payload: String (aircraft name)
     */
    public static final String FM_DATA_LOADED = "fmDataLoaded";

    // Add more event types as needed
}
