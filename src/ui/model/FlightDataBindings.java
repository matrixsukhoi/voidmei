package ui.model;

import java.util.HashMap;
import java.util.Map;
import java.util.function.Function;

/**
 * Registry for mapping field keys to FlightDataProvider accessor methods.
 * Replaces hardcoded switch statements with functional bindings.
 */
public class FlightDataBindings {

    private static final Map<String, Function<FlightDataProvider, String>> bindings = new HashMap<>();

    static {
        // Speed
        bindings.put("ias", FlightDataProvider::getIAS);
        bindings.put("tas", FlightDataProvider::getTAS);
        bindings.put("mach", FlightDataProvider::getMach);

        // Navigation
        bindings.put("dir", FlightDataProvider::getCompass);
        bindings.put("height", FlightDataProvider::getAltitude);
        bindings.put("rda", FlightDataProvider::getRadioAltitude);

        // Performance
        bindings.put("vario", FlightDataProvider::getVario);
        bindings.put("sep", FlightDataProvider::getSEP);
        bindings.put("acc", FlightDataProvider::getAcceleration);
        bindings.put("wx", FlightDataProvider::getWx);
        bindings.put("ny", FlightDataProvider::getGLoad);
        bindings.put("turn", FlightDataProvider::getTurnRate);
        bindings.put("rds", FlightDataProvider::getTurnRadius);

        // Attitude
        bindings.put("aoa", FlightDataProvider::getAoA);
        bindings.put("aos", FlightDataProvider::getAoS);
        bindings.put("ws", FlightDataProvider::getWingSweep);
    }

    /**
     * Get the data accessor function for a given key.
     */
    public static Function<FlightDataProvider, String> get(String key) {
        return bindings.get(key);
    }
}
