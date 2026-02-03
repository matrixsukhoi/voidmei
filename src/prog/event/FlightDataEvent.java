package prog.event;

import java.util.Collections;
import java.util.Map;
import java.util.HashMap;

/**
 * Immutable event carrying a snapshot of flight telemetry data.
 * Thread-safe for passing between Service thread and EDT.
 *
 * Primary access is via {@link #getPayload()} for type-safe fields.
 * Legacy {@link #getData()} / {@link #get(String)} are retained for
 * un-migrated consumers (e.g. FieldOverlay legacy path).
 */
public class FlightDataEvent {
    private final EventPayload payload;
    private final Object state;       // parser.State
    private final Object indicators;  // parser.Indicators
    private final long timestamp;

    public FlightDataEvent(EventPayload payload, Object state, Object indicators) {
        this.payload = payload;
        this.state = state;
        this.indicators = indicators;
        this.timestamp = System.currentTimeMillis();
    }

    /** @deprecated Use {@code FlightDataEvent(EventPayload, Object, Object)} */
    @Deprecated
    public FlightDataEvent(Map<String, String> data) {
        this(data, null, null);
    }

    /** @deprecated Use {@code FlightDataEvent(EventPayload, Object, Object)} */
    @Deprecated
    public FlightDataEvent(Map<String, String> data, Object state, Object indicators) {
        this(mapToPayload(data), state, indicators);
    }

    public EventPayload getPayload() { return payload; }
    public Object getState() { return state; }
    public Object getIndicators() { return indicators; }
    public long getTimestamp() { return timestamp; }

    /**
     * @deprecated Use {@link #getPayload()} for type-safe access.
     */
    @Deprecated
    public Map<String, String> getData() {
        Map<String, String> map = new HashMap<>();
        map.put("mapGrid", payload.mapGrid);
        map.put("fatalWarn", String.valueOf(payload.fatalWarn));
        map.put("radioAltValid", String.valueOf(payload.radioAltValid));
        map.put("isDowningFlap", String.valueOf(payload.isDowningFlap));
        map.put("timeStr", payload.timeStr);
        map.put("is_jet", String.valueOf(payload.isJet));
        map.put("engine_check_done", String.valueOf(payload.engineCheckDone));
        return map;
    }

    /**
     * @deprecated Use {@code getPayload().fieldName} instead.
     */
    @Deprecated
    public String get(String key) {
        return getData().get(key);
    }

    /** Convert legacy Map constructor arg into EventPayload. */
    private static EventPayload mapToPayload(Map<String, String> data) {
        if (data == null || data.isEmpty()) {
            return EventPayload.builder().build();
        }
        return EventPayload.builder()
            .mapGrid(data.containsKey("mapGrid") ? data.get("mapGrid") : "--")
            .fatalWarn(Boolean.parseBoolean(data.get("fatalWarn")))
            .radioAltValid(Boolean.parseBoolean(data.get("radioAltValid")))
            .isDowningFlap(Boolean.parseBoolean(data.get("isDowningFlap")))
            .timeStr(data.containsKey("timeStr") ? data.get("timeStr") : "--:--")
            .isJet(Boolean.parseBoolean(data.get("is_jet")))
            .engineCheckDone(Boolean.parseBoolean(data.get("engine_check_done")))
            .build();
    }
}
