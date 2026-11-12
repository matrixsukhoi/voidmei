package prog.event;

/**
 * 单个飞机场的罗盘标记数据。由 Service 在 map_obj 轮询时生成,
 * 同一实例被多个 FlightDataEvent 复用, 因此必须保持不可变。
 */
public final class AirfieldMark {
    public final boolean enemy;
    public final double distKm;
    /** 绝对方位, 0 = 北, 顺时针. */
    public final double bearingDeg;

    public AirfieldMark(boolean enemy, double distKm, double bearingDeg) {
        this.enemy = enemy;
        this.distKm = distKm;
        this.bearingDeg = bearingDeg;
    }
}
