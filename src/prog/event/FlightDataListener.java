package prog.event;

/**
 * Interface for consuming Data Plane events.
 */
public interface FlightDataListener {
    void onFlightData(FlightDataEvent event);
}
