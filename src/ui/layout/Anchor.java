package ui.layout;

/**
 * Defines anchor points for component alignment.
 * Used for both "Self Anchor" (which point of the component aligns to the
 * target)
 * and "Target Anchor" (which point of the parent/screen is the target).
 */
public enum Anchor {
    TOP_LEFT, TOP_CENTER, TOP_RIGHT,
    MIDDLE_LEFT, CENTER, MIDDLE_RIGHT,
    BOTTOM_LEFT, BOTTOM_CENTER, BOTTOM_RIGHT;

    public boolean isLeft() {
        return this == TOP_LEFT || this == MIDDLE_LEFT || this == BOTTOM_LEFT;
    }

    public boolean isRight() {
        return this == TOP_RIGHT || this == MIDDLE_RIGHT || this == BOTTOM_RIGHT;
    }

    public boolean isTop() {
        return this == TOP_LEFT || this == TOP_CENTER || this == TOP_RIGHT;
    }

    public boolean isBottom() {
        return this == BOTTOM_LEFT || this == BOTTOM_CENTER || this == BOTTOM_RIGHT;
    }

    public boolean isCenterHorizontal() {
        return this == TOP_CENTER || this == CENTER || this == BOTTOM_CENTER;
    }

    public boolean isCenterVertical() {
        return this == MIDDLE_LEFT || this == CENTER || this == MIDDLE_RIGHT;
    }
}
