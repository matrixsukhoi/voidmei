package ui.minihud;

/**
 * 自由画布条目的属性键与取值常量。
 * 键名会写入持久化布局，发布后必须保持稳定，不能复用或改名。
 */
public final class MiniHudProps {
    private MiniHudProps() {
    }

    // 组件类型（注册表 type，永不复用/改名）
    public static final String TYPE_TEXT = "text";
    public static final String TYPE_BAR = "bar";
    public static final String TYPE_COMPASS = "compass";
    public static final String TYPE_ATTITUDE = "attitude";
    public static final String TYPE_PANEL = "panel";

    // 通用
    public static final String SOURCE = "source";
    public static final String FIELD = "field";
    public static final String TEXT = "text";
    public static final String LABEL = "label";
    public static final String UNIT = "unit";
    public static final String PRECISION = "precision";
    public static final String ALIGN = "align"; // L / C / R
    public static final String TEMPLATE = "template";
    // 预览态显示值：每个组件自带，不依赖全局演示数据；宽度仍由 template 决定
    public static final String PREVIEW = "preview";
    public static final String WIDTH_UNITS = "widthUnits";
    public static final String FONT = "font"; // main / small / tiny
    public static final String BASELINE = "baseline"; // main / font
    public static final String COLOR = "color"; // normal / warning / auto / aoa
    public static final String ALT_SOURCE = "altSource"; // AUTO / BARO / RADAR
    public static final String ARROW = "arrow";

    // 条
    public static final String BAR_SOURCE = "barSource"; // flapAngle / speedRatio / throttle / aoa / maneuver / field
    public static final String MIN = "min";
    public static final String MAX = "max";
    public static final String ORIENTATION = "orientation"; // H / V
    public static final String LENGTH_UNITS = "lengthUnits";
    public static final String THICKNESS_UNITS = "thicknessUnits";

    // 仪表
    public static final String COORDINATE_MODE = "coordinateMode"; // BODY / INERTIAL

    // 背景板
    public static final String HEIGHT_UNITS = "heightUnits";
    public static final String RADIUS_UNITS = "radiusUnits";
    public static final String PANEL_COLOR = "panelColor"; // #RRGGBBAA

    // 文本来源
    public static final String SRC_FIELD = "field";
    public static final String SRC_STATIC = "static";
    public static final String SRC_IAS = "ias";
    public static final String SRC_MACH = "mach";
    public static final String SRC_ALTITUDE = "altitude";
    public static final String SRC_ENERGY = "energy";
    public static final String SRC_AOA = "aoa";
    public static final String SRC_SEP = "sep";
    public static final String SRC_GLOAD = "gload";
    public static final String SRC_FLAPS = "flaps";
    public static final String SRC_AIRBRAKE = "airbrake";
    public static final String SRC_GEAR = "gear";
    public static final String SRC_TIME = "time";
    public static final String SRC_FLAP_ANGLE = "flapAngle";
    public static final String SRC_THROTTLE = "throttle";

    // 条来源
    public static final String BAR_FLAP_ANGLE = "flapAngle";
    public static final String BAR_SPEED_RATIO = "speedRatio";
    public static final String BAR_THROTTLE = "throttle";
    public static final String BAR_AOA = "aoa";
    public static final String BAR_MANEUVER = "maneuver";
    public static final String BAR_FIELD = "field";

    // 取值
    public static final String ALT_AUTO = "AUTO";
    public static final String ALT_BARO = "BARO";
    public static final String ALT_RADAR = "RADAR";
    public static final String MODE_BODY = "BODY";
    public static final String MODE_INERTIAL = "INERTIAL";
    public static final String ALIGN_LEFT = "L";
    public static final String ALIGN_CENTER = "C";
    public static final String ALIGN_RIGHT = "R";
    public static final String FONT_MAIN = "main";
    public static final String FONT_SMALL = "small";
    public static final String FONT_TINY = "tiny";
    public static final String BASELINE_MAIN = "main";
    public static final String BASELINE_FONT = "font";
    public static final String COLOR_NORMAL = "normal";
    public static final String COLOR_WARNING = "warning";
    public static final String COLOR_AUTO = "auto";
    public static final String COLOR_AOA = "aoa";
    public static final String ORIENT_H = "H";
    public static final String ORIENT_V = "V";
}
