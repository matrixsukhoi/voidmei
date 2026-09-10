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
#[should_panic]
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
#[should_panic]
fn interp2d_ragged_inner_panics() {
    let xs = [0.0, 1.0];
    let ys = [0.0, 1.0];
    let row = [1.0]; // 内层长度 1 < ny=2
    let zz = [&row[..]];
    let _ = interp2d(0.5, 0.5, &xs, &ys, Some(&zz));
}

#[test]
#[should_panic]
fn interp2d_short_outer_panics() {
    let xs = [0.0, 1.0];
    let ys = [0.0, 1.0];
    let row = [1.0, 2.0]; // 内层长度 = ny, 但外层 1 行 < nx=2, zz[1] 越界
    let zz = [&row[..]];
    let _ = interp2d(0.5, 0.5, &xs, &ys, Some(&zz));
}

#[test]
#[should_panic]
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
    assert_eq!(
        interp_sweep_level(0.5, Some(&empty), vne, sweep, 999.0),
        999.0
    );
}

#[test]
fn interp_sweep_level_single_element() {
    let levels = [SweepLevel {
        sweep: 0.3,
        vne: 700.0,
    }];
    assert_eq!(
        interp_sweep_level(0.9, Some(&levels), vne, sweep, 999.0),
        700.0
    );
}

#[test]
fn interp_sweep_level_below_minimum() {
    let levels = [
        SweepLevel {
            sweep: 0.2,
            vne: 800.0,
        },
        SweepLevel {
            sweep: 1.0,
            vne: 600.0,
        },
    ];
    // vwing <= firstSweep → 首元素值
    assert_eq!(
        interp_sweep_level(0.0, Some(&levels), vne, sweep, 999.0),
        800.0
    );
    assert_eq!(
        interp_sweep_level(0.2, Some(&levels), vne, sweep, 999.0),
        800.0
    );
}

#[test]
fn interp_sweep_level_middle_intervals() {
    let two = [
        SweepLevel {
            sweep: 0.0,
            vne: 800.0,
        },
        SweepLevel {
            sweep: 1.0,
            vne: 600.0,
        },
    ];
    // t = 0.25 → 800 + 0.25*(600-800) = 750
    assert_eq!(
        interp_sweep_level(0.25, Some(&two), vne, sweep, 999.0),
        750.0
    );
    // 区间边界 vwing == s1: 命中首个满足 s0<=v<=s1 的区间, t=1 → 700
    assert_eq!(
        interp_sweep_level(0.5, Some(&two), vne, sweep, 999.0),
        700.0
    );

    let three = [
        SweepLevel {
            sweep: 0.0,
            vne: 800.0,
        },
        SweepLevel {
            sweep: 0.5,
            vne: 700.0,
        },
        SweepLevel {
            sweep: 1.0,
            vne: 600.0,
        },
    ];
    // 0.75 落在 [0.5, 1.0]: t = 0.5 → 700 + 0.5*(600-700) = 650
    assert_eq!(
        interp_sweep_level(0.75, Some(&three), vne, sweep, 999.0),
        650.0
    );
}

#[test]
fn interp_sweep_level_above_maximum() {
    let levels = [
        SweepLevel {
            sweep: 0.0,
            vne: 800.0,
        },
        SweepLevel {
            sweep: 1.0,
            vne: 600.0,
        },
    ];
    assert_eq!(
        interp_sweep_level(1.2, Some(&levels), vne, sweep, 999.0),
        600.0
    );
}
