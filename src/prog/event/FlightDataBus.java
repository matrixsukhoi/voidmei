package prog.event;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

/**
 * Event Bus for Flight Data communication.
 * Used by Service (producer) to push flight physics updates to Overlays
 * (consumers).
 * Thread-safe using CopyOnWriteArrayList.
 */
public class FlightDataBus {
    private static final FlightDataBus INSTANCE = new FlightDataBus();
    private final List<FlightDataListener> listeners = new CopyOnWriteArrayList<>();

    private FlightDataBus() {
    }

    public static FlightDataBus getInstance() {
        return INSTANCE;
    }

    public void register(FlightDataListener listener) {
        if (!listeners.contains(listener)) {
            listeners.add(listener);
        }
    }

    public void unregister(FlightDataListener listener) {
        listeners.remove(listener);
    }

    public void publish(FlightDataEvent event) {
        for (FlightDataListener listener : listeners) {
            try {
                listener.onFlightData(event);
            } catch (Exception e) {
                System.err.println("[FlightDataBus] Error in listener: " + e.getMessage());
                e.printStackTrace();
            }
        }
    }
}
