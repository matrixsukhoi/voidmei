package prog.util;

import java.util.List;
import java.util.function.ToDoubleFunction;

/**
 * Interpolation utilities for flight data calculations.
 * Provides zero-allocation, thread-safe interpolation methods.
 *
 * All methods are pure functions - they do not modify state and are safe
 * to call from any thread.
 */
public final class Interpolation {

    // Prevent instantiation
    private Interpolation() {}

    /**
     * Linear interpolation between two points.
     * Given a value x, interpolates y value on the line through (x0,y0) and (x1,y1).
     *
     * @param x  The x value to interpolate at
     * @param x0 First point x coordinate
     * @param y0 First point y coordinate
     * @param x1 Second point x coordinate
     * @param y1 Second point y coordinate
     * @return The interpolated y value
     */
    public static double lerp(double x, double x0, double y0, double x1, double y1) {
        if (Math.abs(x1 - x0) < 1e-9) {
            return y0;  // Avoid division by zero
        }
        double t = (x - x0) / (x1 - x0);
        return y0 + t * (y1 - y0);
    }

    /**
     * Calculate the slope between two points.
     * Replaces duplicated calcK() methods in Service and HUDCalculator.
     *
     * @param x0 First point x coordinate
     * @param y0 First point y coordinate
     * @param x1 Second point x coordinate
     * @param y1 Second point y coordinate
     * @return The slope (dy/dx), or 0 if x1 == x0
     */
    public static double slope(double x0, double y0, double x1, double y1) {
        if (Math.abs(x1 - x0) < 1e-9) {
            return 0;
        }
        return (y1 - y0) / (x1 - x0);
    }

    /**
     * 1D table interpolation with boundary clamping.
     * Interpolates y value for given x from parallel arrays of (x,y) pairs.
     *
     * @param x  The x value to look up
     * @param xs Array of x values (must be monotonically increasing)
     * @param ys Array of corresponding y values
     * @return Interpolated y value, clamped to boundary values if x is outside range
     */
    public static double interp1d(double x, double[] xs, double[] ys) {
        return interp1d(x, xs, ys, false);
    }

    /**
     * 1D table interpolation with optional extrapolation.
     *
     * @param x           The x value to look up
     * @param xs          Array of x values (must be monotonically increasing)
     * @param ys          Array of corresponding y values
     * @param extrapolate If true, extrapolate beyond boundaries; if false, clamp
     * @return Interpolated y value
     */
    public static double interp1d(double x, double[] xs, double[] ys, boolean extrapolate) {
        int n = xs.length;
        if (n == 0) {
            return 0;
        }
        if (n == 1) {
            return ys[0];
        }

        // Below minimum
        if (x <= xs[0]) {
            if (extrapolate && n >= 2) {
                return lerp(x, xs[0], ys[0], xs[1], ys[1]);
            }
            return ys[0];
        }

        // Above maximum
        if (x >= xs[n - 1]) {
            if (extrapolate && n >= 2) {
                return lerp(x, xs[n - 2], ys[n - 2], xs[n - 1], ys[n - 1]);
            }
            return ys[n - 1];
        }

        // Find interval using binary search
        int i = findInterval(x, xs);
        return lerp(x, xs[i], ys[i], xs[i + 1], ys[i + 1]);
    }

    /**
     * 2D bilinear interpolation for table lookup.
     * Useful for thrust tables indexed by altitude and velocity.
     *
     * @param x  First dimension value (e.g., altitude)
     * @param y  Second dimension value (e.g., velocity)
     * @param xs Array of x values (must be monotonically increasing)
     * @param ys Array of y values (must be monotonically increasing)
     * @param zz 2D array of z values, indexed as zz[xIndex][yIndex]
     * @return Bilinearly interpolated z value
     */
    public static double interp2d(double x, double y, double[] xs, double[] ys, double[][] zz) {
        int nx = xs.length;
        int ny = ys.length;

        if (nx == 0 || ny == 0 || zz == null) {
            return 0;
        }

        // Clamp x to bounds and find interval
        int ix = 0;
        double tx = 0;
        if (x <= xs[0]) {
            ix = 0;
            tx = 0;
        } else if (x >= xs[nx - 1]) {
            ix = nx - 2;
            tx = 1;
        } else {
            ix = findInterval(x, xs);
            tx = (x - xs[ix]) / (xs[ix + 1] - xs[ix]);
        }

        // Clamp y to bounds and find interval
        int iy = 0;
        double ty = 0;
        if (y <= ys[0]) {
            iy = 0;
            ty = 0;
        } else if (y >= ys[ny - 1]) {
            iy = ny - 2;
            ty = 1;
        } else {
            iy = findInterval(y, ys);
            ty = (y - ys[iy]) / (ys[iy + 1] - ys[iy]);
        }

        // Handle edge case for single-element dimension
        if (nx == 1) {
            ix = 0;
            tx = 0;
        }
        if (ny == 1) {
            iy = 0;
            ty = 0;
        }

        // Bilinear interpolation
        double z00 = zz[ix][iy];
        double z01 = (iy + 1 < ny) ? zz[ix][iy + 1] : z00;
        double z10 = (ix + 1 < nx) ? zz[ix + 1][iy] : z00;
        double z11 = (ix + 1 < nx && iy + 1 < ny) ? zz[ix + 1][iy + 1] : z00;

        double z0 = z00 + ty * (z01 - z00);
        double z1 = z10 + ty * (z11 - z10);
        return z0 + tx * (z1 - z0);
    }

    /**
     * Interpolate over SweepLevel list without allocating temporary arrays.
     * Used for variable-geometry wing aircraft (e.g., F-14) to interpolate
     * aerodynamic properties based on wing sweep position.
     *
     * @param <T>          The sweep level type (typically Blkx.SweepLevel)
     * @param vwing        Wing sweep ratio (0.0 = fully forward, 1.0 = fully swept)
     * @param levels       List of sweep levels ordered by sweep ratio
     * @param extractor    Function to extract the numeric value from each level
     * @param defaultValue Value to return if list is null or empty
     * @return Interpolated value at the given sweep position
     */
    public static <T> double interpSweepLevel(
            double vwing,
            List<T> levels,
            ToDoubleFunction<T> extractor,
            ToDoubleFunction<T> sweepExtractor,
            double defaultValue) {

        if (levels == null || levels.isEmpty()) {
            return defaultValue;
        }

        int n = levels.size();
        if (n == 1) {
            return extractor.applyAsDouble(levels.get(0));
        }

        // Below minimum sweep
        double firstSweep = sweepExtractor.applyAsDouble(levels.get(0));
        if (vwing <= firstSweep) {
            return extractor.applyAsDouble(levels.get(0));
        }

        // Find the enclosing interval
        for (int i = 0; i < n - 1; i++) {
            double s0 = sweepExtractor.applyAsDouble(levels.get(i));
            double s1 = sweepExtractor.applyAsDouble(levels.get(i + 1));
            if (vwing >= s0 && vwing <= s1) {
                double v0 = extractor.applyAsDouble(levels.get(i));
                double v1 = extractor.applyAsDouble(levels.get(i + 1));
                return lerp(vwing, s0, v0, s1, v1);
            }
        }

        // Above maximum sweep
        return extractor.applyAsDouble(levels.get(n - 1));
    }

    /**
     * Binary search to find the interval index where xs[i] <= x < xs[i+1].
     *
     * @param x  The value to locate
     * @param xs Sorted array of x values
     * @return Index i such that xs[i] <= x < xs[i+1], or 0 if x < xs[0],
     *         or n-2 if x >= xs[n-1]
     */
    static int findInterval(double x, double[] xs) {
        int lo = 0;
        int hi = xs.length - 1;

        // x is below the range
        if (x <= xs[lo]) {
            return 0;
        }
        // x is above the range
        if (x >= xs[hi]) {
            return Math.max(0, hi - 1);
        }

        // Binary search
        while (hi - lo > 1) {
            int mid = (lo + hi) / 2;
            if (xs[mid] <= x) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        return lo;
    }
}
