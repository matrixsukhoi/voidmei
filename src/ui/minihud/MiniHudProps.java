package ui.minihud;

/** 条目属性键与取值常量（序列化后必须保持字面稳定）。 */
public final class MiniHudProps {
    private MiniHudProps() {
    }

    // legacy.akb
    public static final String SPEED_FIELD = "speedField";
    public static final String SPEED_LABEL = "speedLabel";
    public static final String SHOW_SPEED = "showSpeed";
    public static final String SHOW_AOA = "showAoa";

    // legacy.energy
    public static final String ALT_SOURCE = "altSource"; // AUTO / BARO / RADAR
    public static final String ALT_LABEL = "altLabel";
    public static final String ENERGY_LABEL = "energyLabel";
    public static final String SHOW_ALTITUDE = "showAltitude";
    public static final String SHOW_ENERGY = "showEnergy";

    // legacy.mechanization
    public static final String FLAP_TEXT = "flapText";
    public static final String AIRBRAKE_TEXT = "airbrakeText";
    public static final String GEAR_TEXT = "gearText";

    // legacy.sep
    public static final String SEP_LABEL = "label";
    public static final String SEP_ARROW = "arrow";

    // legacy.maneuver
    public static final String G_LABEL = "gLabel";
    public static final String SHOW_GLOAD = "showGLoad";
    public static final String SHOW_BAR = "showBar";

    // gauge.compass / gauge.attitude
    public static final String COORDINATE_MODE = "coordinateMode"; // INERTIAL / BODY

    // 条/仪表边缩放
    public static final String LENGTH_UNITS = "lengthUnits";

    // text.row 单元格
    public static final String CELLS = "cells";

    // 取值
    public static final String ALT_AUTO = "AUTO";
    public static final String ALT_BARO = "BARO";
    public static final String ALT_RADAR = "RADAR";
    public static final String MODE_INERTIAL = "INERTIAL";
    public static final String MODE_BODY = "BODY";
    public static final String FIELD_IAS = "getIAS";
    public static final String FIELD_MACH = "getMach";
}
