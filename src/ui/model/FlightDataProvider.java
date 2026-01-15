package ui.model;

/**
 * Interface for accessing flight data.
 * Abstracts away the specific data source (service, simulation, etc.).
 */
public interface FlightDataProvider {

    // Speed data
    String getIAS();

    String getTAS();

    String getMach();

    // Navigation data
    String getCompass();

    String getAltitude();

    String getRadioAltitude();

    // Performance data
    String getVario();

    String getSEP();

    String getAcceleration();

    String getWx();

    String getGLoad();

    String getTurnRate();

    String getTurnRadius();

    // Attitude data
    String getAoA();

    String getAoS();

    String getWingSweep();

    /**
     * Get the N/A string used to indicate unavailable data.
     */
    String getNAString();
}
