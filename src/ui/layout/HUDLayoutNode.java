package ui.layout;

import java.awt.Dimension;
import java.awt.Rectangle;
import java.util.ArrayList;
import java.util.List;

import ui.component.HUDComponent;

/**
 * A node in the Modern HUD Layout graph.
 * Wraps a HUDComponent and defines its dependency-based positioning.
 */
public class HUDLayoutNode {
    // Identity
    public final String id;
    public final HUDComponent component;

    // Topology
    private HUDLayoutNode parent;
    private final List<HUDLayoutNode> children = new ArrayList<>();

    // Unit-based Layout Specs (Scaling Invariant)
    private double unitX; // Relative to parent anchor
    private double unitY;
    private Anchor parentAnchor = Anchor.TOP_LEFT;
    private Anchor selfAnchor = Anchor.TOP_LEFT;

    // Flags
    private boolean ignoreBounds = false; // If true, doesn't affect parent's bound calculation (Overlay)

    // Runtime State (Calculated)
    private Rectangle pixelRect = new Rectangle();
    private boolean dirty = true;

    public HUDLayoutNode(String id, HUDComponent component) {
        this.id = id;
        this.component = component;
    }

    public HUDLayoutNode setParent(HUDLayoutNode parent) {
        if (this.parent != null) {
            this.parent.children.remove(this);
        }
        this.parent = parent;
        if (parent != null) {
            parent.children.add(this);
        }
        return this;
    }

    public HUDLayoutNode setRelativePosition(double unitX, double unitY) {
        this.unitX = unitX;
        this.unitY = unitY;
        return this;
    }

    public HUDLayoutNode setAnchors(Anchor parentAnchor, Anchor selfAnchor) {
        this.parentAnchor = parentAnchor;
        this.selfAnchor = selfAnchor;
        return this;
    }

    public HUDLayoutNode setIgnoreBounds(boolean ignore) {
        this.ignoreBounds = ignore;
        return this;
    }

    public HUDLayoutNode getParent() {
        return parent;
    }

    public List<HUDLayoutNode> getChildren() {
        return children;
    }

    // --- Calculation Logic (Called by Engine) ---

    /**
     * Solve relative position to absolute pixel coordinates.
     * 
     * @param lineHeight Global scaling factor
     * @param parentRect Absolute rect of parent (or Canvas rect if root)
     */
    public void solve(double lineHeight, Rectangle parentRect) {
        Dimension size = component.getPreferredSize(); // Assuming component has valid size

        // 1. Determine Target Point (on Parent)
        int targetX = getAnchorX(parentRect, parentAnchor);
        int targetY = getAnchorY(parentRect, parentAnchor);

        // 2. Add Unit Offset
        targetX += (int) (unitX * lineHeight);
        targetY += (int) (unitY * lineHeight);

        // 3. Determine Self Origin (Top-Left) based on Self Anchor aligning to Target
        // Point
        // If SelfAnchor is CENTER, then TopLeft = Target - Width/2, Target - Height/2
        int selfX = targetX;
        int selfY = targetY;

        if (selfAnchor.isCenterHorizontal()) {
            selfX -= size.width / 2;
        } else if (selfAnchor.isRight()) {
            selfX -= size.width;
        }

        if (selfAnchor.isCenterVertical()) {
            selfY -= size.height / 2;
        } else if (selfAnchor.isBottom()) {
            selfY -= size.height;
        }

        this.pixelRect.setBounds(selfX, selfY, size.width, size.height);
        // prog.util.Logger.info("LayoutDebug", String.format("Node %s: Anchor(%.1f,
        // %.1f) -> Target(%d, %d) -> Self(%d, %d) [Size: %dx%d]",
        // id, unitX, unitY, targetX, targetY, selfX, selfY, size.width, size.height));
        this.dirty = false;
    }

    public Rectangle getPixelRect() {
        return pixelRect;
    }

    private int getAnchorX(Rectangle rect, Anchor anchor) {
        if (anchor.isLeft())
            return rect.x;
        if (anchor.isRight())
            return rect.x + rect.width;
        return rect.x + rect.width / 2;
    }

    private int getAnchorY(Rectangle rect, Anchor anchor) {
        if (anchor.isTop())
            return rect.y;
        if (anchor.isBottom())
            return rect.y + rect.height;
        return rect.y + rect.height / 2;
    }
}
