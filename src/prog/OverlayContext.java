package prog;

import parser.Blkx;
import parser.AttributePool;

/**
 * Context object containing all data overlays might need.
 * Eliminates the need to pass multiple parameters to overlay methods.
 */
public class OverlayContext {

    public final Controller tc;
    public final Service S;
    public final Blkx Blkx;
    public final AttributePool pool;
    public final boolean isPreviewMode;

    private OverlayContext(Builder builder) {
        this.tc = builder.tc;
        this.S = builder.S;
        this.Blkx = builder.Blkx;
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
        return Blkx != null && Blkx.isJet;
    }

    /**
     * Check if debug mode is enabled.
     */
    public boolean isDebug() {
        return Application.debug;
    }

    /**
     * Builder for OverlayContext.
     */
    public static class Builder {
        private Controller tc;
        private Service S;
        private Blkx Blkx;
        private AttributePool pool;
        private boolean isPreviewMode = false;

        public Builder Controller(Controller tc) {
            this.tc = tc;
            return this;
        }

        public Builder Service(Service S) {
            this.S = S;
            return this;
        }

        public Builder Blkx(Blkx Blkx) {
            this.Blkx = Blkx;
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
    public static OverlayContext forGameMode(Controller tc) {
        return builder()
                .Controller(tc)
                .Service(tc.S)
                .Blkx(tc.Blkx)
                .pool(tc.globalPool)
                .previewMode(false)
                .build();
    }

    /**
     * Quick factory for preview mode context.
     */
    public static OverlayContext forPreviewMode(Controller tc) {
        return builder()
                .Controller(tc)
                .Service(tc.S)
                .Blkx(tc.Blkx)
                .pool(tc.globalPool)
                .previewMode(true)
                .build();
    }
}
