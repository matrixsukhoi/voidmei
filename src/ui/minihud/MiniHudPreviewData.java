package ui.minihud;

import prog.Application;
import ui.overlay.MinimalHUDContext;
import ui.overlay.model.HUDData;

/** 预览态演示数据：与旧 refreshTemplates 的占位值一致，保证所见即所得。 */
public final class MiniHudPreviewData {

    private MiniHudPreviewData() {
    }

    public static HUDData create(MinimalHUDContext ctx) {
        HUDData.Builder b = new HUDData.Builder();
        b.ias = 360;
        b.mach = 0.85;
        b.altitude = 1024;
        b.radioAltitude = 510;
        b.radioAltitudeValid = true;
        b.verticalSpeed = 30;
        b.heading = 270;
        b.pitch = 5;
        b.roll = 10;
        b.slip = 1;
        b.aoa = 4.2;
        b.pitchValid = true;
        b.throttle = 100;
        b.flaps = 20;
        b.gear = 0;
        b.airbrake = 0;
        b.flapAllowAngle = 100;
        b.hasFlaps = true;
        b.energyM = 114514;
        b.gLoad = 2.0;
        b.maneuverIndex = 0.2;
        b.isMachMode = true;
        b.aoaColor = Application.colorNum;
        b.aoaBarColor = Application.colorNum;
        b.throttleColor = Application.colorNum;
        b.mapGrid = "C4";
        b.timeStr = "12:00";
        b.speedStr = String.format("M%5.2f", 0.85);
        b.altStr = "ALT" + String.format("%6s", "1024");
        b.aoaStr = String.format("α%3.0f", 20.0);
        b.energyStr = "E114514";
        b.flapsWingStr = "F 20";
        b.airbrakeStr = "";
        b.gearStr = "GEA";
        b.sepStr = "SEP↑30";
        b.maneuverStateStr = "G" + String.format("%5s", "2.0");
        b.aoaRatio = 0.2;
        b.speedBar_speedRatio = 0.6;
        b.speedBar_stallRatio = 0.25;
        b.speedBar_unitMachLimitRatio = 0.9;
        b.speedBar_aileronLockRatio = 0.7;
        b.speedBar_rudderLockRatio = 0.75;
        return b.build();
    }
}
