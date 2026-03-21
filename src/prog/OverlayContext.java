package prog;

import parser.Blkx;
import prog.config.ConfigProvider;

/**
 * Context object containing all data overlays might need.
 * Eliminates the need to pass multiple parameters to overlay methods.
 *
 * 配置访问通过 ConfigProvider 接口而不是 Controller，遵循依赖倒置原则。
 */
public class OverlayContext {

    public final Controller tc;
    public final Service S;
    public final Blkx Blkx;
    public final boolean isPreviewMode;
    /** 配置提供者，用于访问配置而不依赖 Controller */
    public final ConfigProvider configProvider;

    private OverlayContext(Builder builder) {
        this.tc = builder.tc;
        this.S = builder.S;
        this.Blkx = builder.Blkx;
        this.isPreviewMode = builder.isPreviewMode;
        this.configProvider = builder.configProvider;
    }

    /**
     * Get a boolean config value.
     * 通过 ConfigProvider 接口访问配置，而不是通过 Controller。
     */
    public boolean getBool(String key) {
        return Boolean.parseBoolean(configProvider.getConfig(key));
    }

    /**
     * Get a string config value.
     * 通过 ConfigProvider 接口访问配置，而不是通过 Controller。
     */
    public String getString(String key) {
        return configProvider.getConfig(key);
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
        private boolean isPreviewMode = false;
        private ConfigProvider configProvider;

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

        public Builder previewMode(boolean isPreviewMode) {
            this.isPreviewMode = isPreviewMode;
            return this;
        }

        /**
         * 设置配置提供者，用于访问配置而不依赖 Controller。
         */
        public Builder configProvider(ConfigProvider configProvider) {
            this.configProvider = configProvider;
            return this;
        }

        public OverlayContext build() {
            // 如果没有显式设置 configProvider，从 Controller 获取
            if (this.configProvider == null && this.tc != null) {
                this.configProvider = tc.getConfigService();
            }
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
     * configProvider 自动从 Controller 的 ConfigService 获取。
     */
    public static OverlayContext forGameMode(Controller tc) {
        return builder()
                .Controller(tc)
                .Service(tc.S)
                .Blkx(tc.getBlkx())
                .configProvider(tc.getConfigService())
                .previewMode(false)
                .build();
    }

    /**
     * Quick factory for preview mode context.
     * configProvider 自动从 Controller 的 ConfigService 获取。
     */
    public static OverlayContext forPreviewMode(Controller tc) {
        return builder()
                .Controller(tc)
                .Service(tc.S)
                .Blkx(tc.getBlkx())
                .configProvider(tc.getConfigService())
                .previewMode(true)
                .build();
    }
}
