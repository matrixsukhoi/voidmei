package prog.event;

/**
 * Type-safe payload for flight data events.
 * Replaces the untyped Map&lt;String, String&gt; for compile-time safety
 * and zero unnecessary String boxing of boolean/numeric values.
 *
 * Immutable — safe for cross-thread passing between Service and EDT.
 */
public final class EventPayload {
    public final String mapGrid;
    public final boolean fatalWarn;
    public final boolean radioAltValid;
    public final boolean isDowningFlap;
    public final String timeStr;
    public final boolean isJet;
    public final boolean engineCheckDone;
    /** Optimal compressor stage index (0-based). -1 indicates invalid/jet/single-stage. */
    public final int optimalCompressorStage;
    /** True when actual compressor stage doesn't match optimal (at full throttle). */
    public final boolean compressorStageMismatch;

    public EventPayload(String mapGrid, boolean fatalWarn, boolean radioAltValid,
                        boolean isDowningFlap, String timeStr, boolean isJet,
                        boolean engineCheckDone, int optimalCompressorStage,
                        boolean compressorStageMismatch) {
        this.mapGrid = mapGrid;
        this.fatalWarn = fatalWarn;
        this.radioAltValid = radioAltValid;
        this.isDowningFlap = isDowningFlap;
        this.timeStr = timeStr;
        this.isJet = isJet;
        this.engineCheckDone = engineCheckDone;
        this.optimalCompressorStage = optimalCompressorStage;
        this.compressorStageMismatch = compressorStageMismatch;
    }

    public static class Builder {
        private String mapGrid = "--";
        private boolean fatalWarn = false;
        private boolean radioAltValid = false;
        private boolean isDowningFlap = false;
        private String timeStr = "--:--";
        private boolean isJet = false;
        private boolean engineCheckDone = false;
        private int optimalCompressorStage = -1;
        private boolean compressorStageMismatch = false;

        public Builder mapGrid(String v) { this.mapGrid = v; return this; }
        public Builder fatalWarn(boolean v) { this.fatalWarn = v; return this; }
        public Builder radioAltValid(boolean v) { this.radioAltValid = v; return this; }
        public Builder isDowningFlap(boolean v) { this.isDowningFlap = v; return this; }
        public Builder timeStr(String v) { this.timeStr = v; return this; }
        public Builder isJet(boolean v) { this.isJet = v; return this; }
        public Builder engineCheckDone(boolean v) { this.engineCheckDone = v; return this; }
        public Builder optimalCompressorStage(int v) { this.optimalCompressorStage = v; return this; }
        public Builder compressorStageMismatch(boolean v) { this.compressorStageMismatch = v; return this; }

        public EventPayload build() {
            return new EventPayload(mapGrid, fatalWarn, radioAltValid,
                                    isDowningFlap, timeStr, isJet, engineCheckDone,
                                    optimalCompressorStage, compressorStageMismatch);
        }
    }

    public static Builder builder() { return new Builder(); }
}
