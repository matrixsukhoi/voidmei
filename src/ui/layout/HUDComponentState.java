package ui.layout;

import ui.component.HUDComponent;

/**
 * Data structure representing a component's assigned position and layout state.
 */
public class HUDComponentState {
    private final HUDComponent component;
    private HUDLayoutSlot slot;
    private int xOffset; // User-defined manual offset from slot anchor
    private int yOffset;
    private boolean visible = true;

    // Calculated coordinates (absolute pixels relative to canvas top-left)
    private int calculatedX;
    private int calculatedY;

    public HUDComponentState(HUDComponent component) {
        this.component = component;
        this.slot = HUDLayoutSlot.TOP_LEFT; // Default
    }

    public HUDComponent getComponent() {
        return component;
    }

    public HUDLayoutSlot getSlot() {
        return slot;
    }

    public void setSlot(HUDLayoutSlot slot) {
        this.slot = slot;
    }

    public int getXOffset() {
        return xOffset;
    }

    public void setXOffset(int xOffset) {
        this.xOffset = xOffset;
    }

    public int getYOffset() {
        return yOffset;
    }

    public void setYOffset(int yOffset) {
        this.yOffset = yOffset;
    }

    public int getCalculatedX() {
        return calculatedX;
    }

    public void setCalculatedX(int calculatedX) {
        this.calculatedX = calculatedX;
    }

    public int getCalculatedY() {
        return calculatedY;
    }

    public void setCalculatedY(int calculatedY) {
        this.calculatedY = calculatedY;
    }

    public boolean isVisible() {
        return visible;
    }

    public void setVisible(boolean visible) {
        this.visible = visible;
    }
}
