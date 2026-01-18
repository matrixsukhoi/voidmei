package ui.overlay.logic;

import java.awt.Color;

import parser.Blkx;
import prog.Application;
import prog.Service;
import prog.config.HUDSettings;
import ui.overlay.model.HUDData;
import ui.overlay.MinimalHUDContext;

/**
 * Pure logic calculator for HUD Data.
 * Extracts raw data from Service/Blkx and performs business logic calculations.
 */
public class HUDCalculator {

    public static HUDData calculate(Service service, parser.Blkx blkx, HUDSettings settings, MinimalHUDContext ctx) {
        HUDData.Builder b = new HUDData.Builder();

        if (service == null)
            return b.build();

        // --- Raw Flight Data ---
        b.ias = service.IASv;
        b.mach = service.sState.M;
        b.altitude = service.alt;
        b.radioAltitude = service.radioAlt;
        b.verticalSpeed = service.SEP; // m/s
        b.heading = service.dCompass; // double
        b.turnRate = 0; // Not readily available in Service?

        // --- Attitude ---
        // Conversion from Service units (often raw values) to Degrees/Pixels done here?
        // Service.sIndic.aviahorizon_pitch is raw int, requires conversion.
        // Existing MinimalHUD Logic:
        // pitch = (int) ((-aviahp * pitchLimit / 90.0f));
        double aviahp = service.sIndic.aviahorizon_pitch;
        double aviar = service.sIndic.aviahorizon_roll;

        if (aviahp != -65535) {
            // In pixels for now, or degrees? Let's store Degrees in Data, Component
            // converts to Pixels?
            // Or store Pixels if it's display logic?
            // Ideally Data is Generic (Degrees). But AttitudeIndicator expects Pixels.
            // Let's store Degrees here and let Component/Helper convert.
            // Wait, Service usually provides raw telemetry.
            // Let's stick to what MinimalHUD did for now to ensure consistency,
            // but cleaner if we put logic here.

            // MinimalHUD: roundHorizon = (int) Math.round(-aviahp);
            b.pitch = -aviahp;
        } else {
            b.pitch = 0;
        }

        if (aviar != -65535) {
            b.roll = -aviar;
        } else {
            b.roll = 0;
        }

        // --- AoS (Slip) ---
        // MinimalHUD: aosX = (int) (-service.sState.AoS * slideLimit / 30.0f);
        // We store RAW Angle of Slip in Degrees.
        if (service.sState.AoS != -65535) {
            b.slip = service.sState.AoS;
        }

        // --- System State ---
        b.throttle = service.sState.throttle;
        b.flaps = service.sState.flaps;
        b.gear = service.sState.gear;
        b.airbrake = service.sState.airbrake;

        // --- Derived Metrics ---
        b.aoa = service.sState.AoA;
        b.gLoad = service.sState.Ny;
        b.energyM = service.energyM;

        b.isMachMode = settings.drawHudMach();
        b.isGearDown = b.gear > 0; // Check logic from MinimalHUD (gear > 0)
        b.isFlapsDown = b.flaps > 0;
        b.isAirbrakeActive = b.airbrake > 0;

        // --- Complex Logic: Vne Warning ---
        // Ported from MinimalHUD.updateString
        boolean warnVne = false;
        if (b.isAirbrakeActive && b.airbrake == 100) {
            warnVne = true;
        }

        if (blkx != null && blkx.valid) {
            // Calculate Maneuver Index
            double nfweight = blkx.nofuelweight;
            double maneuverIndex = 1 - (nfweight / (nfweight + service.fTotalFuel));
            b.maneuverIndex = maneuverIndex;

            double vwing = 0;
            if (blkx.isVWing) {
                vwing = service.sIndic.wsweep_indicator;
            }

            // Dynamic Vne calculation
            if ((service.IASv >= blkx.getVNEVWing(vwing) * 0.95)
                    || (service.sState.M >= blkx.getMNEVWing(vwing) * 0.95f)) {
                warnVne = true;
            }

            // AoA Warnings
            double maxAvailableAoA = blkx.getAoAHighVWing(vwing, b.flaps > 0 ? (int) b.flaps : 0);
            double availableAoA = maxAvailableAoA - b.aoa;

            if (availableAoA < settings.getAoAWarningRatio() * maxAvailableAoA) {
                b.aoaColor = Application.colorWarning;
            } else {
                b.aoaColor = Application.colorNum;
            }

            // Stall Warning (Contextual)
            if (availableAoA <= 0) {
                b.warnStall = true;
            }

        } else {
            // Default logic if no Blkx
            b.maneuverIndex = 0;
            b.aoaColor = Application.colorNum;
        }
        b.warnVne = warnVne;

        // --- Warnings ---
        // Altitude Warning
        if (service.radioAltValid && service.radioAlt <= 500) {
            b.warnAltitude = true; // Use Radio Alt logic
        }

        // --- Strings Formatting ---
        // Pre-calculate strings for performance and consistency
        if (b.isMachMode) {
            b.speedStr = String.format("M%5s", service.M);
        } else {
            String spdPre = settings.isSpeedLabelDisabled() ? "" : "SPD";
            b.speedStr = String.format("%s%6s", spdPre, service.IAS);
        }

        String altPre = settings.isAltitudeLabelDisabled() ? "" : "ALT";
        if (b.warnAltitude) {
            b.altStr = altPre + String.format("R%5s", service.sRadioAlt);
        } else {
            b.altStr = altPre + String.format("%6s", service.salt);
        }

        // Map Grid
        char map_x = (char) ('A' + (service.loc[1] * service.mapinfo.mapStage) + service.mapinfo.inGameOffset);
        int map_y = (int) (service.loc[0] * service.mapinfo.mapStage + service.mapinfo.inGameOffset + 1);
        b.mapGrid = String.format("%c%d", map_x, map_y);

        // Time (Fuel or Mission)
        if (b.gLoad > 1.5f || b.gLoad < -0.5f) {
            // Show G-Load instead of Time logic handled by Component?
            // Or Calculator decides what "Main Text" is?
            // Let's populate specific fields and let HUDTextRow decide or create meaningful
            // field.
            // Current MinimalHUD logic puts G in lines[4] if high G.
            // Let's stick to providing raw data and formatting helpers.
        }
        // Calculate Time String (Fuel or Empty)
        String s = service.sfueltime;
        String compressor = "";
        switch (service.sState.compressorstage) {
            case 1:
                compressor = "C";
                break;
            case 2:
                compressor = "CC";
                break;
            case 3:
                compressor = "CCC";
                break;
            default:
                compressor = "";
        }
        if (b.gear <= 0) {
            b.timeStr = String.format("L%5s%s", s, compressor);
        } else {
            b.timeStr = String.format("E%5s", service.sTime);
        }

        return b.build();
    }
}
