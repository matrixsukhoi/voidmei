package prog;

/**
 * Represents the State of the Controller lifecycle.
 */
public enum ControllerState {
    /**
     * Initial State - waiting for status bar initialization
     */
    INIT(1),

    /**
     * Connected to game server - waiting to enter game
     */
    CONNECTED(2),

    /**
     * In game - all overlays active
     */
    IN_GAME(3),

    /**
     * Preview mode
     */
    PREVIEW(4);

    private final int legacyValue;

    ControllerState(int legacyValue) {
        this.legacyValue = legacyValue;
    }

    /**
     * Get the legacy integer value for backwards compatibility.
     */
    public int getLegacyValue() {
        return legacyValue;
    }

    /**
     * Convert legacy integer flag to ControllerState.
     */
    public static ControllerState fromLegacyValue(int value) {
        for (ControllerState State : values()) {
            if (State.legacyValue == value) {
                return State;
            }
        }
        return INIT;
    }
}
