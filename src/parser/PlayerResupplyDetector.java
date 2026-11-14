package parser;

import java.util.List;

/**
 * 玩家补给瞬移检测（跨 map poll 状态机，非线程安全，只在轮询线程使用）。
 *
 * 真机规律：补给至少需要约 10s；停在跑道上时机头坐标长时间不变，
 * 补给完成瞬间游戏把飞机瞬移到跑道端头（2026-11-14 抓包约 441m），
 * 同时 IAS 仍为 0、油门 0，8111 没有 repair/rearm 标志字段。
 *
 * 判据：连续静止 >=5s → 单次位移 >=100m → 落点在某条跑道端点 250m 内。
 * 不依赖 Player 图标消失（补给全程图标都存在）。
 */
public final class PlayerResupplyDetector {

    /** 瞬移前需要连续静止的时长。 */
    public static final long REQUIRED_STATIONARY_MS = 5000;
    /** 静止判定：IAS 低于该值（km/h）。 */
    public static final double STATIONARY_MAX_IAS_KMH = 5.0;
    /** 静止判定：单次 map poll 位移小于该值（米）。 */
    public static final double STATIONARY_MAX_STEP_METERS = 1.0;
    /** 单次 map poll 位移超过该值视为瞬移（米）。 */
    public static final double TELEPORT_MIN_METERS = 100.0;
    /** 落点距跑道端点的容差（米）。 */
    public static final double RUNWAY_END_TOLERANCE_METERS = 250.0;
    /** 用于补给判定的最短跑道长度（米），过滤过短/无效对象。 */
    public static final double MIN_RUNWAY_METERS = 100.0;

    private boolean hasPrev;
    private double prevX;
    private double prevY;
    private long stationarySinceMs = -1;

    /** 玩家离开地图/阵亡/换局时清空历史，避免把重生当成补给。 */
    public void reset() {
        hasPrev = false;
        stationarySinceMs = -1;
    }

    /**
     * 每轮 map poll 调用一次。
     *
     * @param x        玩家归一化地图 X
     * @param y        玩家归一化地图 Y
     * @param iasKmh   当前表速
     * @param nowMs    本轮时间戳
     * @param runways  本轮 map_obj 解析出的机场跑道
     * @param info     地图信息（提供 grid_size）
     * @return 本轮是否识别到补给完成瞬移
     */
    public boolean update(double x, double y, double iasKmh, long nowMs,
            List<MapObj.AirfieldRaw> runways, MapInfo info) {
        if (info == null || !(info.grid_sizeX > 0) || !(info.grid_sizeY > 0)
                || x < 0 || x > 1 || y < 0 || y > 1) {
            reset();
            return false;
        }

        if (!hasPrev) {
            hasPrev = true;
            prevX = x;
            prevY = y;
            stationarySinceMs = iasKmh < STATIONARY_MAX_IAS_KMH ? nowMs : -1;
            return false;
        }

        double step = distanceMeters(prevX, prevY, x, y, info);
        boolean teleported = step >= TELEPORT_MIN_METERS;
        boolean wasStationary = stationarySinceMs >= 0
                && nowMs - stationarySinceMs >= REQUIRED_STATIONARY_MS;
        boolean onRunwayEnd = nearRunwayEnd(x, y, runways, info);

        boolean resupply = teleported && wasStationary && onRunwayEnd;

        prevX = x;
        prevY = y;
        if (teleported) {
            // 瞬移后重新计时，下一轮从跑道端开始
            stationarySinceMs = iasKmh < STATIONARY_MAX_IAS_KMH ? nowMs : -1;
        } else if (iasKmh < STATIONARY_MAX_IAS_KMH && step < STATIONARY_MAX_STEP_METERS) {
            if (stationarySinceMs < 0)
                stationarySinceMs = nowMs;
        } else {
            stationarySinceMs = -1;
        }
        return resupply;
    }

    private static boolean nearRunwayEnd(double x, double y, List<MapObj.AirfieldRaw> runways,
            MapInfo info) {
        if (runways == null)
            return false;
        for (MapObj.AirfieldRaw r : runways) {
            double length = Math.hypot((r.ex - r.sx) * info.grid_sizeX,
                    (r.ey - r.sy) * info.grid_sizeY);
            if (length < MIN_RUNWAY_METERS)
                continue;
            if (distanceMeters(x, y, r.sx, r.sy, info) <= RUNWAY_END_TOLERANCE_METERS
                    || distanceMeters(x, y, r.ex, r.ey, info) <= RUNWAY_END_TOLERANCE_METERS)
                return true;
        }
        return false;
    }

    private static double distanceMeters(double x1, double y1, double x2, double y2, MapInfo info) {
        double dx = (x2 - x1) * info.grid_sizeX;
        double dy = (y2 - y1) * info.grid_sizeY;
        return Math.hypot(dx, dy);
    }
}
