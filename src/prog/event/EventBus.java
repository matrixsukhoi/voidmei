package prog.event;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

/**
 * Event Bus for the Data Plane to communicate with the View Plane.
 * Decouples the Service (Producer) from Overlays (Consumers).
 */
public class EventBus {
    private static final EventBus INSTANCE = new EventBus();
    private final List<FlightDataListener> listeners = new CopyOnWriteArrayList<>();

    private EventBus() {
    }

    public static EventBus getInstance() {
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
                // Log but don't crash
                System.err.println("Error in event listener: " + e.getMessage());
                e.printStackTrace();
            }
        }
    }
}
