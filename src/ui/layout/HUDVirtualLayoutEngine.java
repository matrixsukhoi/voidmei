package ui.layout;

import java.awt.Dimension;
import java.awt.Graphics2D;
import java.util.ArrayList;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;

/**
 * The core layout engine that calculates absolute pixel coordinates for HUD
 * components
 * based on their logical slot assignments and preferred sizes.
 */
public class HUDVirtualLayoutEngine {

    private int canvasWidth;
    private int canvasHeight;
    private int padding = 0;

    private final List<HUDComponentState> componentStates = new ArrayList<>();

    // Slotted components grouped for stacking
    private final Map<HUDLayoutSlot, List<HUDComponentState>> slotGroups = new EnumMap<>(HUDLayoutSlot.class);

    public HUDVirtualLayoutEngine(int width, int height) {
        this.canvasWidth = width;
        this.canvasHeight = height;
        for (HUDLayoutSlot slot : HUDLayoutSlot.values()) {
            slotGroups.put(slot, new ArrayList<>());
        }
    }

    public void setCanvasSize(int width, int height) {
        this.canvasWidth = width;
        this.canvasHeight = height;
    }

    public void addComponent(HUDComponentState state) {
        componentStates.add(state);
    }

    /**
     * Re-calculates coordinates for all components.
     * Call this when canvas size or component configurations change.
     */
    public void doLayout() {
        // Reset groups
        for (List<HUDComponentState> group : slotGroups.values()) {
            group.clear();
        }

        // Group by slot
        for (HUDComponentState state : componentStates) {
            slotGroups.get(state.getSlot()).add(state);
        }

        // Calculate positions for each slot
        for (HUDLayoutSlot slot : HUDLayoutSlot.values()) {
            calculateSlotPositions(slot, slotGroups.get(slot));
        }
    }

    private void calculateSlotPositions(HUDLayoutSlot slot, List<HUDComponentState> group) {
        if (group.isEmpty())
            return;

        int startX = 0;
        int startY = 0;
        boolean stackUp = false;

        // Determine base anchor point
        switch (slot) {
            case TOP_LEFT:
                startX = padding;
                startY = padding;
                break;
            case TOP_CENTER:
                startX = canvasWidth / 2;
                startY = padding;
                break;
            case TOP_RIGHT:
                startX = canvasWidth - padding;
                startY = padding;
                break;
            case MIDDLE_LEFT:
                startX = padding;
                startY = canvasHeight / 2;
                break;
            case MIDDLE_CENTER:
                startX = canvasWidth / 2;
                startY = canvasHeight / 2;
                break;
            case MIDDLE_RIGHT:
                startX = canvasWidth - padding;
                startY = canvasHeight / 2;
                break;
            case BOTTOM_LEFT:
                startX = padding;
                startY = canvasHeight - padding;
                stackUp = true;
                break;
            case BOTTOM_CENTER:
                startX = canvasWidth / 2;
                startY = canvasHeight - padding;
                stackUp = true;
                break;
            case BOTTOM_RIGHT:
                startX = canvasWidth - padding;
                startY = canvasHeight - padding;
                stackUp = true;
                break;
            case ABSOLUTE:
                for (HUDComponentState state : group) {
                    state.setCalculatedX(state.getXOffset());
                    state.setCalculatedY(state.getYOffset());
                }
                return;
        }

        // Stack logic
        int currentY = startY;
        for (HUDComponentState state : group) {
            Dimension size = state.getComponent().getPreferredSize();

            // Calculate X based on alignment
            int x = startX;
            if (slot.name().endsWith("_CENTER")) {
                x -= size.width / 2;
            } else if (slot.name().endsWith("_RIGHT")) {
                x -= size.width;
            }

            // Add user offset
            x += state.getXOffset();

            // Calculate Y
            int y = currentY;
            if (stackUp) {
                y -= size.height;
                currentY -= size.height + padding;
            } else if (slot.name().startsWith("MIDDLE_")) {
                // For middle slots, we might need a more complex strategy if there are
                // multiple.
                // Simple version: vertical center stack
                y -= size.height / 2;
                // Note: This simple logic only works well for single components in Middle
                // slots.
            } else {
                currentY += size.height + padding;
            }

            y += state.getYOffset();

            state.setCalculatedX(x);
            state.setCalculatedY(y);
        }
    }

    /**
     * Render all components to the provided graphics context.
     */
    public void render(Graphics2D g) {
        for (HUDComponentState state : componentStates) {
            if (state.isVisible()) {
                state.getComponent().draw(g, state.getCalculatedX(), state.getCalculatedY());
            }
        }
    }
}
