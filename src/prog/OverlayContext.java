package prog;

import parser.blkx;
import parser.AttributePool;

/**
 * Context object containing all data overlays might need.
 * Eliminates the need to pass multiple parameters to overlay methods.
 */
public class OverlayContext {

    public final controller tc;
    public final service S;
    public final blkx blkx;
    public final AttributePool pool;
    public final boolean isPreviewMode;

    private OverlayContext(Builder builder) {
        this.tc = builder.tc;
        this.S = builder.S;
        this.blkx = builder.blkx;
        this.pool = builder.pool;
        this.isPreviewMode = builder.isPreviewMode;
    }

    /**
     * Get a boolean config value.
     */
    public boolean getBool(String key) {
        return Boolean.parseBoolean(tc.getconfig(key));
    }

    /**
     * Get a string config value.
     */
    public String getString(String key) {
        return tc.getconfig(key);
    }

    /**
     * Check if this is a jet aircraft.
     */
    public boolean isJet() {
        return blkx != null && blkx.isJet;
    }

    /**
     * Check if debug mode is enabled.
     */
    public boolean isDebug() {
        return app.debug;
    }

    /**
     * Builder for OverlayContext.
     */
    public static class Builder {
        private controller tc;
        private service S;
        private blkx blkx;
        private AttributePool pool;
        private boolean isPreviewMode = false;

        public Builder controller(controller tc) {
            this.tc = tc;
            return this;
        }

        public Builder service(service S) {
            this.S = S;
            return this;
        }

        public Builder blkx(blkx blkx) {
            this.blkx = blkx;
            return this;
        }

        public Builder pool(AttributePool pool) {
            this.pool = pool;
            return this;
        }

        public Builder previewMode(boolean isPreviewMode) {
            this.isPreviewMode = isPreviewMode;
            return this;
        }

        public OverlayContext build() {
            return new OverlayContext(this);
        }
    }

    /**
     * Create a builder.
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Quick factory for game mode context.
     */
    public static OverlayContext forGameMode(controller tc) {
        return builder()
                .controller(tc)
                .service(tc.S)
                .blkx(tc.blkx)
                .pool(tc.globalPool)
                .previewMode(false)
                .build();
    }

    /**
     * Quick factory for preview mode context.
     */
    public static OverlayContext forPreviewMode(controller tc) {
        return builder()
                .controller(tc)
                .service(tc.S)
                .blkx(tc.blkx)
                .pool(tc.globalPool)
                .previewMode(true)
                .build();
    }
}
