package ui.overlay.logic;

import java.util.Map;

import parser.Blkx;
import prog.Application;
import prog.config.HUDSettings;
import ui.overlay.model.HUDData;
import ui.overlay.MinimalHUDContext;

/**
 * Pure logic calculator for HUD Data.
 * Extracts raw data from FlightDataEvent (Data/State) and performs business
 * logic calculations.
 */
public class HUDCalculator {

    public static HUDData calculate(prog.event.FlightDataEvent event, parser.Blkx blkx, HUDSettings settings,
            MinimalHUDContext ctx) {
        HUDData.Builder b = new HUDData.Builder();

        if (event == null)
            return b.build();

        Map<String, String> data = event.getData();
        parser.State sState = (parser.State) event.getState();
        parser.Indicators sIndic = (parser.Indicators) event.getIndicators();

        // --- Raw Flight Data ---
        b.ias = safeParseDouble(data.get("ias_val"), 0);
        b.mach = (sState != null) ? sState.M : 0;
        b.altitude = safeParseDouble(data.get("alt_val"), 0);
        b.radioAltitude = safeParseDouble(data.get("radioAlt_f"), 0);
        b.verticalSpeed = safeParseDouble(data.get("sep_val"), 0);
        b.heading = safeParseDouble(data.get("compass_val"), 0);
        b.turnRate = 0;

        b.mapGrid = data.get("mapGrid");
        if (b.mapGrid == null)
            b.mapGrid = "--";

        // --- Attitude ---
        double aviahp = 0;
        double aviar = 0;
        if (sIndic != null) {
            aviahp = sIndic.aviahorizon_pitch;
            aviar = sIndic.aviahorizon_roll;
        } else {
            aviahp = safeParseDouble(data.get("aviahorizon_pitch"), -65535);
            aviar = safeParseDouble(data.get("aviahorizon_roll"), -65535);
        }

        if (aviahp != -65535) {
            b.pitch = -aviahp;
        } else {
            b.pitch = 0;
        }

        if (aviar != -65535) {
            b.roll = -aviar;
        } else {
            b.roll = 0;
        }

        // --- AoS / System State ---
        if (sState != null) {
            if (sState.AoS != -65535)
                b.slip = sState.AoS;

            b.throttle = sState.throttle;
            b.flaps = sState.flaps;
            b.gear = sState.gear;
            b.airbrake = sState.airbrake;

            b.airbrake = sState.airbrake;

            if (b.throttle > 100) {
                b.throttleColor = java.awt.Color.RED;
            } else {
                b.throttleColor = java.awt.Color.WHITE; // Or default? HUDData defaults to GREEN, but white is standard
                                                        // text.
            }

            boolean isDowningFlap = Boolean.parseBoolean(data.get("isDowningFlap"));
            b.flapAllowAngle = getFlapAllowAngle(b.ias, isDowningFlap, blkx);

            b.aoa = sState.AoA;
            b.gLoad = sState.Ny;
        }

        b.energyM = safeParseDouble(data.get("energyM"), 0);

        b.isMachMode = settings.drawHudMach();
        b.isGearDown = b.gear > 0;
        b.isFlapsDown = b.flaps > 0;
        b.isAirbrakeActive = b.airbrake > 0;

        // --- Warning Logic ---
        boolean warnVne = false;
        if (b.isAirbrakeActive && b.airbrake == 100) {
            warnVne = true;
        }

        if (blkx != null && blkx.valid) {
            // User requested formula: 1 - (nfweight / (nfweight + fuel))
            double nfweight = blkx.nofuelweight;
            double currentFuel = (sState != null) ? sState.mfuel : 0;

            // Check for valid weights to avoid division by zero
            if (nfweight > 0 && (nfweight + currentFuel) > 0) {
                b.maneuverIndex = 1.0 - (nfweight / (nfweight + currentFuel));
            } else {
                b.maneuverIndex = 0;
            }

            double vwing = 0;
            if (blkx.isVWing && sIndic != null) {
                vwing = sIndic.wsweep_indicator;
            }

            // Dynamic Vne calculation
            if ((b.ias >= blkx.getVNEVWing(vwing) * 0.95)
                    || (b.mach >= blkx.getMNEVWing(vwing) * 0.95f)) {
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
            if (availableAoA < settings.getAoABarWarningRatio() * maxAvailableAoA) {
                b.aoaBarColor = Application.colorUnit;
            } else {
                b.aoaBarColor = Application.colorNum;
            }

            if (maxAvailableAoA > 0.001) {
                b.aoaRatio = availableAoA / maxAvailableAoA;
            } else {
                b.aoaRatio = 0;
            }

            if (availableAoA <= 0) {
                b.warnStall = true;
            }

        } else {
            b.maneuverIndex = 0;
            b.aoaColor = Application.colorNum;
            b.aoaBarColor = Application.colorNum;
            b.aoaRatio = b.aoa / 30.0;
        }
        b.warnVne = warnVne;

        // Warnings
        double radioAlt = b.radioAltitude;
        boolean radioAltValid = Boolean.parseBoolean(data.get("radioAltValid"));
        if (radioAltValid && radioAlt <= 500) {
            b.warnAltitude = true;
        }

        // --- Strings Formatting (using Data) ---
        if (b.isMachMode) {
            b.speedStr = String.format("M%5.2f", b.mach);
        } else {
            String spdPre = settings.isSpeedLabelDisabled() ? "" : "SPD";
            b.speedStr = String.format("%s%6d", spdPre, (int) b.ias);
        }

        String altPre = settings.isAltitudeLabelDisabled() ? "" : "ALT";
        if (b.warnAltitude) {
            b.altStr = altPre + String.format("R%5.0f", b.radioAltitude);
        } else {
            b.altStr = altPre + String.format("%6.0f", b.altitude);
        }

        if (settings.isAoADisabled()) {
            b.aoaStr = "";
            b.energyStr = "";
        } else {
            b.aoaStr = String.format("α%3.0f", b.aoa);
            b.energyStr = String.format("E%5.0f", b.energyM);
        }

        String sepPre = settings.isSEPLabelDisabled() ? "" : "SEP";
        if (b.verticalSpeed > 0) {
            b.sepStr = String.format("%s↑%4.0f", sepPre, b.verticalSpeed);
        } else {
            b.sepStr = String.format("%s↓%4.0f", sepPre, b.verticalSpeed);
        }

        // Maneuver / Time
        if (b.gLoad > 1.5f || b.gLoad < -0.5f) {
            b.maneuverRowStr = String.format("G%5.1f", b.gLoad);
        } else {
            String time = data.get("timeStr");
            b.maneuverRowStr = (time != null && !time.isEmpty()) ? "L" + time : "";
        }

        // Configuration
        String brk = "";
        String gear = "";
        boolean inAction = false;
        if (b.airbrake > 0) {
            brk = "BRK";
            if (b.airbrake != 100)
                inAction = true;
        }
        if (b.gear > 0) {
            gear = "GEA";
            if (b.gear != 100)
                inAction = true;
        }

        if (b.flaps > 0) {
            b.flapsStr = String.format("F%3.0f%s%s", b.flaps, brk, gear);
        } else {
            if (blkx != null && blkx.isVWing && sIndic != null) {
                b.flapsStr = String.format("W%3.0f%s%s", sIndic.wsweep_indicator * 100, brk, gear); // approx logic
            } else {
                b.flapsStr = String.format("%4s%s%s", "", brk, gear);
            }
        }

        b.warnConfiguration = inAction;

        return b.build();
    }

    private static double safeParseDouble(String val, double def) {
        if (val == null)
            return def;
        try {
            return Double.parseDouble(val);
        } catch (NumberFormatException e) {
            return def;
        }
    }

    private static double getFlapAllowAngle(double ias, boolean isDowningFlap, Blkx blkx) {
        if (ias == 0 || blkx == null || !blkx.valid)
            return 125;

        int i = 0;
        for (; i < blkx.FlapsDestructionNum - 1; i++) {
            if (ias > blkx.FlapsDestructionIndSpeed[i][1]) {
                break;
            }
        }

        double x0, x1, y0, y1, t;
        double k;

        if (i == 0) {
            x0 = blkx.FlapsDestructionIndSpeed[i][1];
            y0 = blkx.FlapsDestructionIndSpeed[i][0] * 100.0f;
            x1 = blkx.FlapsDestructionIndSpeed[i + 1][1];
            y1 = blkx.FlapsDestructionIndSpeed[i + 1][0] * 100.0f;
            k = calcK(x0, y0, x1, y1);
            t = y0 + (ias - x0) * k;
            return normFlapAngle(t);
        } else {
            if (ias == blkx.FlapsDestructionIndSpeed[i - 1][1]) {
                return blkx.FlapsDestructionIndSpeed[i - 1][0] * 100.0f;
            }
            x0 = blkx.FlapsDestructionIndSpeed[i - 1][1];
            y0 = blkx.FlapsDestructionIndSpeed[i - 1][0] * 100.0f;
            x1 = blkx.FlapsDestructionIndSpeed[i][1];
            y1 = blkx.FlapsDestructionIndSpeed[i][0] * 100.0f;
            k = calcK(x0, y0, x1, y1);
            t = y0 + (ias - x0) * k;
            return normFlapAngle(t);
        }
    }

    private static double calcK(double x0, double y0, double x1, double y1) {
        if (Math.abs(x1 - x0) < 0.0001)
            return 0;
        return (y1 - y0) / (x1 - x0);
    }

    private static double normFlapAngle(double t) {
        if (t < 0)
            return 0;
        if (t < 125)
            return t;
        return 125;
    }

    // --- Helper for Text Measurement ---
    private static final java.awt.image.BufferedImage MEASURE_IMG = new java.awt.image.BufferedImage(1, 1,
            java.awt.image.BufferedImage.TYPE_INT_ARGB);
    private static final java.awt.Graphics2D MEASURE_G = MEASURE_IMG.createGraphics();

    public static int getStringWidth(String text, java.awt.Font font) {
        if (text == null || text.isEmpty() || font == null) {
            return 0;
        }
        synchronized (MEASURE_G) {
            MEASURE_G.setFont(font);
            return MEASURE_G.getFontMetrics().stringWidth(text);
        }
    }
}
