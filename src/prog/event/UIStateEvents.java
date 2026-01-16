package prog.event;

/**
 * Event type constants for UIStateBus.
 * Centralizes all UI state event identifiers for easy discovery and
 * refactoring.
 */
public final class UIStateEvents {

    private UIStateEvents() {
    }

    /**
     * Published when the FM Print switch state changes.
     * Payload: Boolean (new state)
     */
    public static final String FM_PRINT_SWITCH_CHANGED = "fmPrintSwitchChanged";

    // Add more event types as needed
}
