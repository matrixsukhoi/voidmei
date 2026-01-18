package ui.layout;

/**
 * Logical positions on the HUD screen where components can be anchored.
 */
public enum HUDLayoutSlot {
    TOP_LEFT,
    TOP_CENTER,
    TOP_RIGHT,

    MIDDLE_LEFT,
    MIDDLE_CENTER, // Used for crosshair etc.
    MIDDLE_RIGHT,

    BOTTOM_LEFT,
    BOTTOM_CENTER,
    BOTTOM_RIGHT,

    /**
     * Component has a fixed, user-defined absolute position.
     */
    ABSOLUTE
}
