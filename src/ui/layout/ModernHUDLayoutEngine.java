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
        // Keep current Origin
        this.canvasRect.setSize(width, height);
        this.dirty = true;
    }

    public void setCanvasOrigin(int x, int y) {
        this.canvasRect.setLocation(x, y);
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

    private boolean debug = false;
    private int renderOffsetX = 0;
    private int renderOffsetY = 0;

    public void setDebug(boolean debug) {
        this.debug = debug;
        this.dirty = true;
    }

    public void setRenderOffset(int x, int y) {
        this.renderOffsetX = x;
        this.renderOffsetY = y;
    }

    public void render(Graphics2D g) {
        // ... (existing render logic)
        for (HUDLayoutNode node : sortedNodes) {
            if (!node.component.isVisible())
                continue;

            Rectangle r = node.getPixelRect();
            // Apply Render Offset to shift logical layout into physical window space
            node.component.draw(g, r.x + renderOffsetX, r.y + renderOffsetY);

            if (debug) {
                drawDebug(g, node);
            }
        }
    }

    private void drawDebug(Graphics2D g, HUDLayoutNode node) {
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
        Rectangle r = node.getPixelRect();
        g.drawRect(r.x + renderOffsetX, r.y + renderOffsetY, r.width, r.height);
    }

    public void logTopology() {
        Logger.info("ModernLayout", "Topology Order: ");
        for (HUDLayoutNode node : sortedNodes) {
            Logger.info("ModernLayout",
                    " -> " + node.id + " (Parent: " + (node.getParent() == null ? "ROOT" : node.getParent().id) + ")");
        }
    }

    /**
     * Calculate the bounding rectangle of all VISIBLE components.
     * Used for dynamic window resizing.
     */
    /**
     * Calculate the bounding rectangle of all VISIBLE components.
     * Used for dynamic window resizing.
     */
    public Rectangle getContentBounds() {
        int minX = Integer.MAX_VALUE;
        int minY = Integer.MAX_VALUE;
        int maxX = Integer.MIN_VALUE;
        int maxY = Integer.MIN_VALUE;
        boolean hasContent = false;

        for (HUDLayoutNode node : sortedNodes) {
            if (node.component != null && node.component.isVisible()) {
                hasContent = true;
                Rectangle r = node.getPixelRect();
                int right = r.x + r.width;
                int bottom = r.y + r.height;

                if (r.x < minX)
                    minX = r.x;
                if (r.y < minY)
                    minY = r.y;
                if (right > maxX)
                    maxX = right;
                if (bottom > maxY)
                    maxY = bottom;
            }
        }

        // Return at least 1x1 to avoid invisible windows if empty
        if (!hasContent)
            return new Rectangle(0, 0, 1, 1);

        // Return full bounding box relative to current (0,0)
        // Width/Height must be positive dimensions
        return new Rectangle(minX, minY, maxX - minX, maxY - minY);
    }

    /**
     * Automatically resize the given window to fit the content, adding padding.
     * Also applies render offset to center the content within the padding.
     * 
     * @param window  The AWT Window or Component to resize (must support setSize)
     * @param padding The padding in pixels to apply around the content
     */
    public void applyAutoSizing(java.awt.Component window, int padding) {
        // 1. Ensure topology is resolved
        this.doLayout();

        // 2. Get actual content bounds
        Rectangle contentBounds = getContentBounds();

        // 3. Calculate Render Offset
        // Goal: Shift minX/minY to the padding position
        int offsetX = padding - contentBounds.x;
        int offsetY = padding - contentBounds.y;

        // 4. Calculate New Window Size
        // Width = Content Width + Left Padding + Right Padding
        int newWidth = contentBounds.width + (padding * 2);
        int newHeight = contentBounds.height + (padding * 2);

        // 5. Apply changes
        window.setSize(newWidth, newHeight);
        this.setRenderOffset(offsetX, offsetY);

        Logger.info("ModernLayout", "Auto-sized window: Content[" + contentBounds.x + "," + contentBounds.y +
                " " + contentBounds.width + "x" + contentBounds.height + "] -> Window[" +
                newWidth + "x" + newHeight + "] Offset[" + offsetX + "," + offsetY + "]");
    }
}
