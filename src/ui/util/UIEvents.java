package ui.util;

/**
 * Event type constants for UIEventBus.
 * Centralizes all event identifiers for easy discovery and refactoring.
 */
public final class UIEvents {

    private UIEvents() {
    }

    /**
     * Published when the FM Print switch state changes.
     * Payload: Boolean (new state)
     */
    public static final String FM_PRINT_SWITCH_CHANGED = "fmPrintSwitchChanged";

    // Add more event types as needed
}
