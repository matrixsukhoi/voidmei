package prog;

/**
 * Represents the state of the controller lifecycle.
 */
public enum ControllerState {
    /**
     * Initial state - waiting for status bar initialization
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
        for (ControllerState state : values()) {
            if (state.legacyValue == value) {
                return state;
            }
        }
        return INIT;
    }
}
