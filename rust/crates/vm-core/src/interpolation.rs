//! Interpolation utilities for flight data calculations.
//! Provides zero-allocation, thread-safe interpolation methods.
//!
//! All methods are pure functions - they do not modify state and are safe
//! to call from any thread.
//!
//! 对应 Java: `src/prog/util/Interpolation.java` (一比一翻译)

// Prevent instantiation
// PORT: Java 私有构造器防实例化 → Rust 自由函数模块无实例化概念, 天然满足

/// Linear interpolation between two points.
/// Given a value x, interpolates y value on the line through (x0,y0) and (x1,y1).
///
/// - `x`  The x value to interpolate at
/// - `x0` First point x coordinate
/// - `y0` First point y coordinate
/// - `x1` Second point x coordinate
/// - `y1` Second point y coordinate
///
/// Returns the interpolated y value.
pub fn lerp(x: f64, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    if (x1 - x0).abs() < 1e-9 {
        return y0; // Avoid division by zero
    }
    let t = (x - x0) / (x1 - x0);
    y0 + t * (y1 - y0)
}

/// Calculate the slope between two points.
/// Replaces duplicated calcK() methods in Service and HUDCalculator.
///
/// Returns the slope (dy/dx), or 0 if x1 == x0.
pub fn slope(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    if (x1 - x0).abs() < 1e-9 {
        return 0.0;
    }
    (y1 - y0) / (x1 - x0)
}

/// 1D table interpolation with boundary clamping.
/// Interpolates y value for given x from parallel arrays of (x,y) pairs.
///
/// - `x`  The x value to look up
/// - `xs` Array of x values (must be monotonically increasing)
/// - `ys` Array of corresponding y values
///
/// Returns interpolated y value, clamped to boundary values if x is outside range.
pub fn interp1d(x: f64, xs: &[f64], ys: &[f64]) -> f64 {
    interp1d_extrapolate(x, xs, ys, false)
}

/// 1D table interpolation with optional extrapolation.
///
/// - `x`           The x value to look up
/// - `xs`          Array of x values (must be monotonically increasing)
/// - `ys`          Array of corresponding y values
/// - `extrapolate` If true, extrapolate beyond boundaries; if false, clamp
///
/// Returns the interpolated y value.
// PORT: Java 重载 interp1d(x, xs, ys, extrapolate) → Rust 无函数重载, 更名 interp1d_extrapolate
pub fn interp1d_extrapolate(x: f64, xs: &[f64], ys: &[f64], extrapolate: bool) -> f64 {
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return ys[0];
    }

    // Below minimum
    if x <= xs[0] {
        if extrapolate && n >= 2 {
            return lerp(x, xs[0], ys[0], xs[1], ys[1]);
        }
        return ys[0];
    }

    // Above maximum
    if x >= xs[n - 1] {
        if extrapolate && n >= 2 {
            return lerp(x, xs[n - 2], ys[n - 2], xs[n - 1], ys[n - 1]);
        }
        return ys[n - 1];
    }

    // Find interval using binary search
    let i = find_interval(x, xs);
    lerp(x, xs[i], ys[i], xs[i + 1], ys[i + 1])
}

/// 2D bilinear interpolation for table lookup.
/// Useful for thrust tables indexed by altitude and velocity.
///
/// - `x`  First dimension value (e.g., altitude)
/// - `y`  Second dimension value (e.g., velocity)
/// - `xs` Array of x values (must be monotonically increasing)
/// - `ys` Array of y values (must be monotonically increasing)
/// - `zz` 2D array of z values, indexed as zz[xIndex][yIndex]
///
/// Returns the bilinearly interpolated z value.
// PORT: Java double[][] 锯齿数组 → &[&[f64]]; Java zz == null → Option::None;
//       内层数组过短时索引 panic, 对应 Java ArrayIndexOutOfBoundsException
// PORT: Java `int ix = 0; double tx = 0;` (及 iy/ty) 的初始 0 值为死值 (if/else 各分支均先赋值),
//       保留字面初始化以对齐源码; allow 收窄到各 let 语句, 避免函数级 allow
//       掩盖未来新增代码的 unused_assignments 警告
pub fn interp2d(x: f64, y: f64, xs: &[f64], ys: &[f64], zz: Option<&[&[f64]]>) -> f64 {
    let nx = xs.len();
    let ny = ys.len();

    if nx == 0 || ny == 0 || zz.is_none() {
        return 0.0;
    }
    let zz = zz.unwrap();

    // Clamp x to bounds and find interval
    // PORT: Java int 索引允许负值过渡 (nx==1 走上方分支时 ix = nx-2 = -1,
    //       由下方单元素分支修正后才使用); usize 在 nx-2 处会下溢 panic, 故用 isize 中转
    #[allow(unused_assignments)]
    let mut ix: isize = 0;
    #[allow(unused_assignments)]
    let mut tx: f64 = 0.0;
    if x <= xs[0] {
        ix = 0;
        tx = 0.0;
    } else if x >= xs[nx - 1] {
        ix = nx as isize - 2;
        tx = 1.0;
    } else {
        ix = find_interval(x, xs) as isize;
        tx = (x - xs[ix as usize]) / (xs[(ix + 1) as usize] - xs[ix as usize]);
    }

    // Clamp y to bounds and find interval
    #[allow(unused_assignments)]
    let mut iy: isize = 0;
    #[allow(unused_assignments)]
    let mut ty: f64 = 0.0;
    if y <= ys[0] {
        iy = 0;
        ty = 0.0;
    } else if y >= ys[ny - 1] {
        iy = ny as isize - 2;
        ty = 1.0;
    } else {
        iy = find_interval(y, ys) as isize;
        ty = (y - ys[iy as usize]) / (ys[(iy + 1) as usize] - ys[iy as usize]);
    }

    // Handle edge case for single-element dimension
    if nx == 1 {
        ix = 0;
        tx = 0.0;
    }
    if ny == 1 {
        iy = 0;
        ty = 0.0;
    }

    // Bilinear interpolation
    let z00 = zz[ix as usize][iy as usize];
    let z01 = if iy + 1 < ny as isize { zz[ix as usize][(iy + 1) as usize] } else { z00 };
    let z10 = if ix + 1 < nx as isize { zz[(ix + 1) as usize][iy as usize] } else { z00 };
    let z11 =
        if ix + 1 < nx as isize && iy + 1 < ny as isize { zz[(ix + 1) as usize][(iy + 1) as usize] } else { z00 };

    let z0 = z00 + ty * (z01 - z00);
    let z1 = z10 + ty * (z11 - z10);
    z0 + tx * (z1 - z0)
}

/// Interpolate over SweepLevel list without allocating temporary arrays.
/// Used for variable-geometry wing aircraft (e.g., F-14) to interpolate
/// aerodynamic properties based on wing sweep position.
///
/// - `vwing`     Wing sweep ratio (0.0 = fully forward, 1.0 = fully swept)
/// - `levels`    List of sweep levels ordered by sweep ratio
/// - `extractor` Function to extract the numeric value from each level
/// - `default_value` Value to return if list is null or empty
///
/// Returns the interpolated value at the given sweep position.
// PORT: Java 泛型 List<T> → Option<&[T]> (null → None); ToDoubleFunction<T> → impl Fn(&T) -> f64
pub fn interp_sweep_level<T>(
    vwing: f64,
    levels: Option<&[T]>,
    extractor: impl Fn(&T) -> f64,
    sweep_extractor: impl Fn(&T) -> f64,
    default_value: f64,
) -> f64 {
    // PORT: Java `levels == null || levels.isEmpty()` 两分支同返 default, 合并为一个 match
    let levels = match levels {
        Some(l) if !l.is_empty() => l,
        _ => return default_value,
    };

    let n = levels.len();
    if n == 1 {
        return extractor(&levels[0]);
    }

    // Below minimum sweep
    let first_sweep = sweep_extractor(&levels[0]);
    if vwing <= first_sweep {
        return extractor(&levels[0]);
    }

    // Find the enclosing interval
    for i in 0..n - 1 {
        let s0 = sweep_extractor(&levels[i]);
        let s1 = sweep_extractor(&levels[i + 1]);
        if vwing >= s0 && vwing <= s1 {
            let v0 = extractor(&levels[i]);
            let v1 = extractor(&levels[i + 1]);
            return lerp(vwing, s0, v0, s1, v1);
        }
    }

    // Above maximum sweep
    extractor(&levels[n - 1])
}

/// Binary search to find the interval index where xs[i] <= x < xs[i+1].
///
/// - `x`  The value to locate
/// - `xs` Sorted array of x values
///
/// Returns index i such that xs[i] <= x < xs[i+1], or 0 if x < xs[0],
/// or n-2 if x >= xs[n-1].
// PORT: Java 包私有 static → Rust 模块私有 (同文件 tests 模块可直接测试)
fn find_interval(x: f64, xs: &[f64]) -> usize {
    // PORT: Java int 的 lo/hi 可为 -1 (空数组时 hi = -1), usize 会下溢 panic, 用 isize 复刻
    let mut lo: isize = 0;
    let mut hi: isize = xs.len() as isize - 1;

    // x is below the range
    if x <= xs[lo as usize] {
        return 0;
    }
    // x is above the range
    if x >= xs[hi as usize] {
        return Ord::max(0, hi - 1) as usize;
    }

    // Binary search
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if xs[mid as usize] <= x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- lerp ----------

    #[test]
    fn lerp_midpoint_and_extrapolation() {
        // 中点: t = 0.5 → 100 + 0.5*(200-100) = 150 (二进制精确值)
        assert_eq!(lerp(5.0, 0.0, 100.0, 10.0, 200.0), 150.0);
        // 区间外: t = 1.5 → 10 + 1.5*20 = 40
        assert_eq!(lerp(15.0, 0.0, 10.0, 10.0, 30.0), 40.0);
    }

    #[test]
    fn lerp_degenerate_interval_returns_y0() {
        // |x1-x0| < 1e-9 时避免除零, 直接返回 y0
        assert_eq!(lerp(5.0, 0.0, 7.0, 1e-10, 99.0), 7.0);
        assert_eq!(lerp(5.0, 3.0, 7.0, 3.0, 99.0), 7.0);
    }

    // ---------- slope ----------

    #[test]
    fn slope_basic_and_degenerate() {
        assert_eq!(slope(0.0, 10.0, 10.0, 30.0), 2.0);
        // x1 == x0 → 0
        assert_eq!(slope(5.0, 1.0, 5.0, 2.0), 0.0);
    }

    // ---------- interp1d (clamp) ----------

    #[test]
    fn interp1d_midpoint() {
        let xs = [0.0, 1000.0, 2000.0, 3000.0];
        let ys = [100.0, 95.0, 85.0, 70.0];
        // 1500 落在 [1000,2000]: t=0.5 → 95 + 0.5*(85-95) = 90
        assert_eq!(interp1d(1500.0, &xs, &ys), 90.0);
        // 恰好落在节点 xs[2]=2000: findInterval 返回 2, t=0 → 85
        assert_eq!(interp1d(2000.0, &xs, &ys), 85.0);
    }

    #[test]
    fn interp1d_clamps_at_boundaries() {
        let xs = [0.0, 1000.0, 2000.0, 3000.0];
        let ys = [100.0, 95.0, 85.0, 70.0];
        assert_eq!(interp1d(-500.0, &xs, &ys), 100.0); // 低于下界
        assert_eq!(interp1d(0.0, &xs, &ys), 100.0); // x <= xs[0] 边界
        assert_eq!(interp1d(3500.0, &xs, &ys), 70.0); // 高于上界
        assert_eq!(interp1d(3000.0, &xs, &ys), 70.0); // x >= xs[n-1] 边界
    }

    #[test]
    fn interp1d_empty_and_single_element() {
        assert_eq!(interp1d(1.0, &[], &[]), 0.0); // 空表 → 0
        assert_eq!(interp1d(1.0, &[5.0], &[42.0]), 42.0); // 单点 → ys[0]
        assert_eq!(interp1d(99.0, &[5.0], &[42.0]), 42.0);
    }

    // ---------- interp1d_extrapolate ----------

    #[test]
    fn interp1d_extrapolate_below_and_above() {
        let xs = [0.0, 1000.0, 2000.0, 3000.0];
        let ys = [100.0, 95.0, 85.0, 70.0];
        // 下方外推: t = -1 → 100 + (-1)*(95-100) = 105
        assert_eq!(interp1d_extrapolate(-1000.0, &xs, &ys, true), 105.0);
        // 上方外推: t = 2 → 85 + 2*(70-85) = 55
        assert_eq!(interp1d_extrapolate(4000.0, &xs, &ys, true), 55.0);
        // extrapolate=false 时仍钳位
        assert_eq!(interp1d_extrapolate(-1000.0, &xs, &ys, false), 100.0);
    }

    #[test]
    fn interp1d_extrapolate_single_element_ignores_extrapolation() {
        // n==1 提前返回 ys[0], extrapolate 分支 (n >= 2) 不生效
        assert_eq!(interp1d_extrapolate(99.0, &[5.0], &[42.0], true), 42.0);
    }

    #[test]
    fn interp1d_nan_propagates() {
        // NaN 不满足任何边界比较 (NaN 比较恒 false), find_interval 二分中
        // xs[mid] <= NaN 恒 false → 收敛到 0 区间, lerp 中 t = NaN → 返回 NaN
        // (Java IEEE 754 同路径同结果)
        let xs = [0.0, 1000.0, 2000.0, 3000.0];
        let ys = [100.0, 95.0, 85.0, 70.0];
        assert!(interp1d(f64::NAN, &xs, &ys).is_nan());
    }

    // ---------- find_interval ----------

    #[test]
    fn find_interval_positions() {
        let xs = [0.0, 10.0, 20.0, 30.0];
        assert_eq!(find_interval(5.0, &xs), 0);
        assert_eq!(find_interval(15.0, &xs), 1);
        assert_eq!(find_interval(25.0, &xs), 2);
        assert_eq!(find_interval(0.0, &xs), 0); // x <= xs[0] → 0
        assert_eq!(find_interval(30.0, &xs), 2); // x >= xs[n-1] → n-2
    }

    #[test]
    fn find_interval_single_element_returns_zero() {
        // 单元素: x >= xs[0] → Math.max(0, hi-1) = max(0, -1) = 0
        assert_eq!(find_interval(5.0, &[5.0]), 0);
        assert_eq!(find_interval(7.0, &[5.0]), 0);
    }

    #[test]
    #[should_panic] // Java: ArrayIndexOutOfBoundsException (xs[0] 越界)
    fn find_interval_empty_panics() {
        let _ = find_interval(1.0, &[]);
    }

    // ---------- interp2d ----------

    #[test]
    fn interp2d_bilinear_center() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let row0 = [0.0, 1.0];
        let row1 = [2.0, 3.0];
        let zz = [&row0[..], &row1[..]];
        // 中心: tx=ty=0.5 → 0.5*(0+2) + 0.5*(1+3) 的双线性值 = 1.5
        assert_eq!(interp2d(0.5, 0.5, &xs, &ys, Some(&zz)), 1.5);
    }

    #[test]
    fn interp2d_clamps_at_corners() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let row0 = [0.0, 1.0];
        let row1 = [2.0, 3.0];
        let zz = [&row0[..], &row1[..]];
        // x 低于下界 (tx=0), y 高于上界 (ty=1): z0=1, z1=3, 结果 = 1
        assert_eq!(interp2d(-5.0, 2.0, &xs, &ys, Some(&zz)), 1.0);
    }

    #[test]
    fn interp2d_single_element_x_dimension() {
        // 走 ix = nx-2 = -1 的 Java 负索引过渡, 由单元素分支修正为 ix=0, tx=0
        let xs = [5.0];
        let ys = [0.0, 1.0];
        let row = [10.0, 20.0];
        let zz = [&row[..]];
        // z00=10, z01=20, z10=z11=10 (ix+1 越界回落 z00); ty=0.5 → z0=15, tx=0 → 15
        assert_eq!(interp2d(7.0, 0.5, &xs, &ys, Some(&zz)), 15.0);
    }

    #[test]
    fn interp2d_single_element_y_dimension() {
        let xs = [0.0, 1.0];
        let ys = [3.0];
        let row0 = [5.0];
        let row1 = [7.0];
        let zz = [&row0[..], &row1[..]];
        // iy = ny-2 = -1 过渡修正为 iy=0, ty=0; z01=z11=z00=5; tx=0.5 → 5+0.5*(7-5)=6
        assert_eq!(interp2d(0.5, 99.0, &xs, &ys, Some(&zz)), 6.0);
    }

    #[test]
    fn interp2d_empty_or_none_returns_zero() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let row0 = [0.0, 1.0];
        let row1 = [2.0, 3.0];
        let zz = [&row0[..], &row1[..]];
        assert_eq!(interp2d(0.5, 0.5, &[], &ys, Some(&zz)), 0.0); // nx == 0
        assert_eq!(interp2d(0.5, 0.5, &xs, &[], Some(&zz)), 0.0); // ny == 0
        assert_eq!(interp2d(0.5, 0.5, &xs, &ys, None), 0.0); // zz == null
    }

    #[test]
    #[should_panic] // Java: ArrayIndexOutOfBoundsException (zz[0][1] 内层过短)
    fn interp2d_ragged_inner_panics() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let row = [1.0]; // 内层长度 1 < ny=2
        let zz = [&row[..]];
        let _ = interp2d(0.5, 0.5, &xs, &ys, Some(&zz));
    }

    #[test]
    #[should_panic] // Java: ArrayIndexOutOfBoundsException (zz[ix+1] 外层行数 < nx)
    fn interp2d_short_outer_panics() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let row = [1.0, 2.0]; // 内层长度 = ny, 但外层 1 行 < nx=2, zz[1] 越界
        let zz = [&row[..]];
        let _ = interp2d(0.5, 0.5, &xs, &ys, Some(&zz));
    }

    #[test]
    #[should_panic] // Java: NaN 不满足任何边界比较走 else 分支, xs[ix+1] 越界 → AIOOBE
    fn interp2d_nan_with_single_element_dimension_panics() {
        // 单元素维 + NaN: 跳过上方 clamp 分支进 find_interval, ix+1=1 处 xs[1] 越界
        let xs = [5.0];
        let ys = [0.0, 1.0];
        let row = [10.0, 20.0];
        let zz = [&row[..]];
        let _ = interp2d(f64::NAN, 0.5, &xs, &ys, Some(&zz));
    }

    // ---------- interp_sweep_level ----------

    struct SweepLevel {
        sweep: f64,
        vne: f64,
    }

    fn vne(l: &SweepLevel) -> f64 {
        l.vne
    }

    fn sweep(l: &SweepLevel) -> f64 {
        l.sweep
    }

    #[test]
    fn interp_sweep_level_none_or_empty_returns_default() {
        assert_eq!(interp_sweep_level(0.5, None, vne, sweep, 999.0), 999.0);
        let empty: [SweepLevel; 0] = [];
        assert_eq!(interp_sweep_level(0.5, Some(&empty), vne, sweep, 999.0), 999.0);
    }

    #[test]
    fn interp_sweep_level_single_element() {
        let levels = [SweepLevel { sweep: 0.3, vne: 700.0 }];
        assert_eq!(interp_sweep_level(0.9, Some(&levels), vne, sweep, 999.0), 700.0);
    }

    #[test]
    fn interp_sweep_level_below_minimum() {
        let levels = [
            SweepLevel { sweep: 0.2, vne: 800.0 },
            SweepLevel { sweep: 1.0, vne: 600.0 },
        ];
        // vwing <= firstSweep → 首元素值
        assert_eq!(interp_sweep_level(0.0, Some(&levels), vne, sweep, 999.0), 800.0);
        assert_eq!(interp_sweep_level(0.2, Some(&levels), vne, sweep, 999.0), 800.0);
    }

    #[test]
    fn interp_sweep_level_middle_intervals() {
        let two = [
            SweepLevel { sweep: 0.0, vne: 800.0 },
            SweepLevel { sweep: 1.0, vne: 600.0 },
        ];
        // t = 0.25 → 800 + 0.25*(600-800) = 750
        assert_eq!(interp_sweep_level(0.25, Some(&two), vne, sweep, 999.0), 750.0);
        // 区间边界 vwing == s1: 命中首个满足 s0<=v<=s1 的区间, t=1 → 700
        assert_eq!(interp_sweep_level(0.5, Some(&two), vne, sweep, 999.0), 700.0);

        let three = [
            SweepLevel { sweep: 0.0, vne: 800.0 },
            SweepLevel { sweep: 0.5, vne: 700.0 },
            SweepLevel { sweep: 1.0, vne: 600.0 },
        ];
        // 0.75 落在 [0.5, 1.0]: t = 0.5 → 700 + 0.5*(600-700) = 650
        assert_eq!(interp_sweep_level(0.75, Some(&three), vne, sweep, 999.0), 650.0);
    }

    #[test]
    fn interp_sweep_level_above_maximum() {
        let levels = [
            SweepLevel { sweep: 0.0, vne: 800.0 },
            SweepLevel { sweep: 1.0, vne: 600.0 },
        ];
        assert_eq!(interp_sweep_level(1.2, Some(&levels), vne, sweep, 999.0), 600.0);
    }
}
