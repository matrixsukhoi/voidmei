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

    // Add more event types as needed
}
