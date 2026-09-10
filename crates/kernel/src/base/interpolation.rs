//! Interpolation utilities for flight data calculations.
//! Provides zero-allocation, thread-safe interpolation methods.
//!
//! All methods are pure functions - they do not modify state and are safe
//! to call from any thread.
//!
//! 对应 Java: `src/prog/util/Interpolation.java` (一比一翻译)

// Prevent instantiation
// Java 私有构造器防实例化 → Rust 自由函数模块无实例化概念, 天然满足

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
// Java 重载 interp1d(x, xs, ys, extrapolate) → Rust 无函数重载, 更名 interp1d_extrapolate
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
// Java double[][] 锯齿数组 → &[&[f64]]; Java zz == null → Option::None;
//       内层数组过短时索引 panic, 对应 Java ArrayIndexOutOfBoundsException
// Java `int ix = 0; double tx = 0;` (及 iy/ty) 的初始 0 值为死值 (if/else 各分支均先赋值),
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
    // Java int 索引允许负值过渡 (nx==1 走上方分支时 ix = nx-2 = -1,
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
    let z01 = if iy + 1 < ny as isize {
        zz[ix as usize][(iy + 1) as usize]
    } else {
        z00
    };
    let z10 = if ix + 1 < nx as isize {
        zz[(ix + 1) as usize][iy as usize]
    } else {
        z00
    };
    let z11 = if ix + 1 < nx as isize && iy + 1 < ny as isize {
        zz[(ix + 1) as usize][(iy + 1) as usize]
    } else {
        z00
    };

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
// Java 泛型 List<T> → Option<&[T]> (null → None); ToDoubleFunction<T> → impl Fn(&T) -> f64
pub fn interp_sweep_level<T>(
    vwing: f64,
    levels: Option<&[T]>,
    extractor: impl Fn(&T) -> f64,
    sweep_extractor: impl Fn(&T) -> f64,
    default_value: f64,
) -> f64 {
    // Java `levels == null || levels.isEmpty()` 两分支同返 default, 合并为一个 match
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
// Java 包私有 static → Rust 模块私有 (同文件 tests 模块可直接测试)
fn find_interval(x: f64, xs: &[f64]) -> usize {
    // Java int 的 lo/hi 可为 -1 (空数组时 hi = -1), usize 会下溢 panic, 用 isize 复刻
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
mod tests;
