package prog;

/**
 * Strategy interface for determining if an overlay should be activated.
 * Replaces hardcoded conditions scattered throughout controller.
 */
@FunctionalInterface
public interface ActivationStrategy {

    /**
     * Determine if the overlay should be activated based on context.
     */
    boolean shouldActivate(OverlayContext ctx);

    /**
     * Combine this strategy with another using AND logic.
     */
    default ActivationStrategy and(ActivationStrategy other) {
        return ctx -> this.shouldActivate(ctx) && other.shouldActivate(ctx);
    }

    /**
     * Combine this strategy with another using OR logic.
     */
    default ActivationStrategy or(ActivationStrategy other) {
        return ctx -> this.shouldActivate(ctx) || other.shouldActivate(ctx);
    }

    /**
     * Negate this strategy.
     */
    default ActivationStrategy not() {
        return ctx -> !this.shouldActivate(ctx);
    }

    // ========== Preset Strategies ==========

    /**
     * Always activate.
     */
    static ActivationStrategy always() {
        return ctx -> true;
    }

    /**
     * Never activate.
     */
    static ActivationStrategy never() {
        return ctx -> false;
    }

    /**
     * Activate based on a boolean config key.
     */
    static ActivationStrategy config(String configKey) {
        return ctx -> ctx.getBool(configKey);
    }

    /**
     * Activate only in debug mode.
     */
    static ActivationStrategy debugOnly() {
        return ctx -> ctx.isDebug();
    }

    /**
     * Activate only for jet aircraft.
     */
    static ActivationStrategy jetOnly() {
        return ctx -> ctx.isJet();
    }

    /**
     * Activate only in preview mode.
     */
    static ActivationStrategy previewOnly() {
        return ctx -> ctx.isPreviewMode;
    }

    /**
     * Activate only in game mode (not preview).
     */
    static ActivationStrategy gameModeOnly() {
        return ctx -> !ctx.isPreviewMode;
    }

    /**
     * Activate when blkx data is available.
     */
    static ActivationStrategy blkxAvailable() {
        return ctx -> ctx.blkx != null;
    }
}
