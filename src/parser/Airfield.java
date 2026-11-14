package parser;

/**
 * 已换算成米制的机场快照（不可变）：
 * 与玩家当前位置的距离、跑道长度、敌我属性、世界方位与地图网格。
 */
public final class Airfield {

    public final double distanceMeters;
    public final double lengthMeters;
    public final boolean friendly;
    /** 相对世界坐标的机场方位（0=北，顺时针，0..360）。 */
    public final double bearingDeg;
    /** 机场中心所在的地图网格（如 "C4"）；无法计算时为空串。 */
    public final String mapGrid;

    public Airfield(double distanceMeters, double lengthMeters, boolean friendly,
            double bearingDeg, String mapGrid) {
        this.distanceMeters = distanceMeters;
        this.lengthMeters = lengthMeters;
        this.friendly = friendly;
        this.bearingDeg = bearingDeg;
        this.mapGrid = mapGrid != null ? mapGrid : "";
    }
}
