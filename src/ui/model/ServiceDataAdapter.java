package ui.model;

import prog.service;

/**
 * Adapter that wraps a service instance to provide FlightDataProvider
 * interface.
 * This allows flightInfo to depend on the interface rather than the concrete
 * service class.
 */
public class ServiceDataAdapter implements FlightDataProvider {

    private final service data;

    public ServiceDataAdapter(service data) {
        this.data = data;
    }

    @Override
    public String getIAS() {
        return data.IAS;
    }

    @Override
    public String getTAS() {
        return data.TAS;
    }

    @Override
    public String getMach() {
        return data.M;
    }

    @Override
    public String getCompass() {
        return data.compass;
    }

    @Override
    public String getAltitude() {
        return data.salt;
    }

    @Override
    public String getRadioAltitude() {
        return data.sRadioAlt;
    }

    @Override
    public String getVario() {
        return data.Vy;
    }

    @Override
    public String getSEP() {
        return data.sSEP;
    }

    @Override
    public String getAcceleration() {
        return data.sAcc;
    }

    @Override
    public String getWx() {
        return data.Wx;
    }

    @Override
    public String getGLoad() {
        return data.sN;
    }

    @Override
    public String getTurnRate() {
        return data.sTurnRate;
    }

    @Override
    public String getTurnRadius() {
        return data.sTurnRds;
    }

    @Override
    public String getAoA() {
        return data.AoA;
    }

    @Override
    public String getAoS() {
        return data.AoS;
    }

    @Override
    public String getWingSweep() {
        return data.sWingSweep;
    }

    @Override
    public String getNAString() {
        return service.nastring;
    }
}
