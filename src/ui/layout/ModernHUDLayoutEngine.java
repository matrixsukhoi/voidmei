package ui.layout;

import java.awt.Graphics2D;
import java.awt.Rectangle;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.Stack;

import prog.util.Logger;

/**
 * A modern, unit-based layout engine for HUDs.
 * Features:
 * 1. LineHeight based scaling (DPI independent).
 * 2. Topological dependency resolution.
 * 3. Anchor-based positioning.
 */
public class ModernHUDLayoutEngine {

    private final Map<String, HUDLayoutNode> nodes = new HashMap<>(); // ID -> Node
    private double lineHeight = 20.0;
    private int canvasWidth;
    private int canvasHeight;
    private Rectangle canvasRect = new Rectangle();

    // Sorted list of nodes for rendering/layout
    private List<HUDLayoutNode> sortedNodes = new ArrayList<>();
    private boolean dirty = true;

    public ModernHUDLayoutEngine(int width, int height) {
        setCanvasSize(width, height);
    }

    public void setCanvasSize(int width, int height) {
        this.canvasWidth = width;
        this.canvasHeight = height;
        this.canvasRect.setBounds(0, 0, width, height);
        this.dirty = true;
    }

    public void setLineHeight(double lineHeight) {
        if (Math.abs(this.lineHeight - lineHeight) > 0.001) {
            this.lineHeight = lineHeight;
            this.dirty = true;
        }
    }

    public void addNode(HUDLayoutNode node) {
        nodes.put(node.id, node);
        dirty = true;
    }

    public HUDLayoutNode getNode(String id) {
        return nodes.get(id);
    }

    public void clear() {
        nodes.clear();
        sortedNodes.clear();
        dirty = true;
    }

    /**
     * Perform layout calculation if needed.
     */
    public void doLayout() {
        if (dirty) {
            resolveTopology();
            calculateCoordinates();
            dirty = false;
        }

        // Always re-calculate if components changed size?
        // Ideally, we check if any component size changed.
        // For performance, we assume size changes trigger layouts externally or we
        // check hash?
        // Modern engine: check basic dirty flag or forced update.
        // In simple mode: always recalculate positions is cheap if node count < 100.
        calculateCoordinates();
    }

    /**
     * Sort nodes based on dependency (DFS).
     * Root nodes (no parent) come first.
     */
    private void resolveTopology() {
        sortedNodes.clear();
        Set<String> visited = new HashSet<>();
        Set<String> recursionStack = new HashSet<>();

        for (HUDLayoutNode node : nodes.values()) {
            if (node.getParent() == null) {
                visit(node, visited, recursionStack);
            }
        }
    }

    private void visit(HUDLayoutNode node, Set<String> visited, Set<String> stack) {
        if (visited.contains(node.id))
            return;
        if (stack.contains(node.id)) {
            Logger.info("ModernLayout", "Cycle detected in layout dependency: " + node.id);
            return;
        }

        stack.add(node.id);

        // Dependency: Parent must be layout BEFORE Child.
        // Wait, 'visit' logic for sorting?
        // If 'parent' is dependency, we should visit parent first.
        // My loop starts from Roots (parent==null).
        // Then I should traverse children.
        // Roots are calculated first relative to Canvas.
        // Children are calculated relative to Parent.

        sortedNodes.add(node);

        for (HUDLayoutNode child : node.getChildren()) {
            visit(child, visited, stack);
        }

        stack.remove(node.id);
        visited.add(node.id);
    }

    private void calculateCoordinates() {
        for (HUDLayoutNode node : sortedNodes) {
            Rectangle refRect = (node.getParent() == null) ? canvasRect : node.getParent().getPixelRect();
            node.solve(lineHeight, refRect);
        }
    }

    public void render(Graphics2D g) {
        // Render in order? Or z-index?
        // Usually dependency order is fine for painting (parents then children).
        for (HUDLayoutNode node : sortedNodes) {
            if (!node.component.isVisible())
                continue; // Assuming component has visible flag or we add it to Node
            // HUDComponent usually has internal draw logic, assuming it's visible.
            // We can check `node.getPixelRect()` for visibility culling if needed.

            Rectangle r = node.getPixelRect();
            node.component.draw(g, r.x, r.y);

            // Debug draw
            // Generate color from ID hash for consistency
            int hash = node.id.hashCode();
            int rCol = (hash & 0xFF0000) >> 16;
            int gCol = (hash & 0x00FF00) >> 8;
            int bCol = (hash & 0x0000FF);
            // Ensure high brightness for visibility on dark background
            if (rCol + gCol + bCol < 380) {
                rCol = Math.min(255, rCol + 100);
                gCol = Math.min(255, gCol + 100);
                bCol = Math.min(255, bCol + 100);
            }
            g.setColor(new java.awt.Color(rCol, gCol, bCol));
            g.drawRect(r.x, r.y, r.width, r.height);
        }
    }

    public void logTopology() {
        Logger.info("ModernLayout", "Topology Order: ");
        for (HUDLayoutNode node : sortedNodes) {
            Logger.info("ModernLayout",
                    " -> " + node.id + " (Parent: " + (node.getParent() == null ? "ROOT" : node.getParent().id) + ")");
        }
    }
}
