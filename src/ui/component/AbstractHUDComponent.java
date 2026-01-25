package ui.component;

import java.awt.Dimension;
import java.awt.Graphics2D;

/**
 * Abstract base class for HUD components.
 * Provides common functionality like visibility state management.
 */
public abstract class AbstractHUDComponent implements HUDComponent {

    protected boolean visible = true;

    @Override
    public boolean isVisible() {
        return visible;
    }

    @Override
    public void setVisible(boolean visible) {
        this.visible = visible;
    }

    // Default implementations for abstract interface methods if needed,
    // explicitly abstract if they must be implemented by subclasses.
    @Override
    public abstract String getId();

    @Override
    public abstract Dimension getPreferredSize();

    @Override
    public abstract void draw(Graphics2D g2d, int x, int y);
}
