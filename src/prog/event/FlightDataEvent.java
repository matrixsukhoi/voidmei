package prog.event;

import java.util.Collections;
import java.util.Map;

/**
 * Data Plane object carrying a snapshot of the flight physics.
 * Immutable to ensure thread safety when passed between Service and UI threads.
 */
public class FlightDataEvent {
    private final Map<String, String> data;
    private final long timestamp;

    public FlightDataEvent(Map<String, String> data) {
        this.data = data != null ? Collections.unmodifiableMap(new java.util.HashMap<>(data)) : Collections.emptyMap();
        this.timestamp = System.currentTimeMillis();
    }

    public String get(String key) {
        return data.get(key);
    }

    public Map<String, String> getData() {
        return data;
    }

    public long getTimestamp() {
        return timestamp;
    }
}
