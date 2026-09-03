//! 公式函数库注册表: 名字 → FnId → Rust 纯函数薄封装。
//! 全部封装既有实现 (interpolation/atmosphere_model/physics_constants), 不重复造轮。
//! 状态原语 (sma/prev/blend/deriv/vote/stable/learn_max) 的求值在 eval.rs (需 StateStore)。
//! 设计: doc/formula_system_design.md §3.3/§3.5

use crate::base::atmosphere_model;
use crate::base::interpolation;
use std::sync::Arc;

/// 求值期的值类型: 数值 (bool 以 0.0/1.0 编码) 或命名表 (不透明, 仅插值函数实参)
#[derive(Debug, Clone)]
pub enum Value {
    Num(f64),
    Table(Arc<Vec<f64>>),
}

impl Value {
    /// 取数值; Table 进数值上下文 = 类型错误 → NaN (设计 §3.6 隔离传播)
    pub fn num(self) -> f64 {
        match self {
            Value::Num(v) => v,
            Value::Table(_) => f64::NAN,
        }
    }

    pub fn truthy(v: f64) -> bool {
        // NaN 参与:
        v != 0.0
    }
}

/// 函数编号 (编译期 resolve, 求值期 match 分派)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnId {
    // 数学族
    Abs,
    Min,
    Max,
    Sqrt,
    Sin,
    Cos,
    Atan2,
    Exp,
    Ln,
    Floor,
    Ceil,
    Round,
    Clamp,
    // 哨兵族 (F_INVALID 约定)
    IsValid,
    Na,
    IsNan,
    // 插值族
    Lerp,
    Interp1d,
    Interp1dEx,
    Interp2d,
    // 大气族 (ISA)
    IsaPressure,
    IsaDensity,
    IsaTemp,
    IasToTas,
    TasToIas,
    IasPerMach,
    /// invalid(): 显式 NaN — 接管公式的"此帧不接管"表达 (write_back NaN 守卫)
    Invalid,
    // FM 查表族 (需求值上下文的当前 blkx, eval.rs 分派)
    FmVne,
    FmMne,
    FmAoaHigh,
    FmFlapAllowSpeed,
    FmFlapAllowAngle,
    // 状态原语 (eval.rs 分派)
    Sma,
    Prev,
    Blend,
    Deriv,
    Vote,
    Stable,
    LearnMax,
    /// latch(cond, x): cond 真 → x (并记忆); 假 → 上帧输出 (x **不求值**, 状态不污染)
    Latch,
}

/// 名字 → FnId (精确 match, 注册表面)
pub fn resolve_fn(name: &str) -> Option<FnId> {
    Some(match name {
        "abs" => FnId::Abs,
        "min" => FnId::Min,
        "max" => FnId::Max,
        "sqrt" => FnId::Sqrt,
        "sin" => FnId::Sin,
        "cos" => FnId::Cos,
        "atan2" => FnId::Atan2,
        "exp" => FnId::Exp,
        "ln" => FnId::Ln,
        "floor" => FnId::Floor,
        "ceil" => FnId::Ceil,
        "round" => FnId::Round,
        "clamp" => FnId::Clamp,
        "is_valid" => FnId::IsValid,
        "na" => FnId::Na,
        "is_nan" => FnId::IsNan,
        "lerp" => FnId::Lerp,
        "interp1d" => FnId::Interp1d,
        "interp1d_ex" => FnId::Interp1dEx,
        "interp2d" => FnId::Interp2d,
        "isa_pressure" => FnId::IsaPressure,
        "isa_density" => FnId::IsaDensity,
        "isa_temp" => FnId::IsaTemp,
        "ias_to_tas" => FnId::IasToTas,
        "tas_to_ias" => FnId::TasToIas,
        "ias_per_mach" => FnId::IasPerMach,
        "invalid" => FnId::Invalid,
        "fm_vne" => FnId::FmVne,
        "fm_mne" => FnId::FmMne,
        "fm_aoa_high" => FnId::FmAoaHigh,
        "fm_flap_allow_speed" => FnId::FmFlapAllowSpeed,
        "fm_flap_allow_angle" => FnId::FmFlapAllowAngle,
        "sma" => FnId::Sma,
        "prev" => FnId::Prev,
        "blend" => FnId::Blend,
        "deriv" => FnId::Deriv,
        "vote" => FnId::Vote,
        "stable" => FnId::Stable,
        "learn_max" => FnId::LearnMax,
        "latch" => FnId::Latch,
        _ => return None,
    })
}

/// FnId → 注册名 (编辑器目录/诊断用)
pub fn fn_name(fid: FnId) -> &'static str {
    match fid {
        FnId::Abs => "abs",
        FnId::Min => "min",
        FnId::Max => "max",
        FnId::Sqrt => "sqrt",
        FnId::Sin => "sin",
        FnId::Cos => "cos",
        FnId::Atan2 => "atan2",
        FnId::Exp => "exp",
        FnId::Ln => "ln",
        FnId::Floor => "floor",
        FnId::Ceil => "ceil",
        FnId::Round => "round",
        FnId::Clamp => "clamp",
        FnId::IsValid => "is_valid",
        FnId::Na => "na",
        FnId::IsNan => "is_nan",
        FnId::Lerp => "lerp",
        FnId::Interp1d => "interp1d",
        FnId::Interp1dEx => "interp1d_ex",
        FnId::Interp2d => "interp2d",
        FnId::IsaPressure => "isa_pressure",
        FnId::IsaDensity => "isa_density",
        FnId::IsaTemp => "isa_temp",
        FnId::IasToTas => "ias_to_tas",
        FnId::TasToIas => "tas_to_ias",
        FnId::IasPerMach => "ias_per_mach",
        FnId::Invalid => "invalid",
        FnId::FmVne => "fm_vne",
        FnId::FmMne => "fm_mne",
        FnId::FmAoaHigh => "fm_aoa_high",
        FnId::FmFlapAllowSpeed => "fm_flap_allow_speed",
        FnId::FmFlapAllowAngle => "fm_flap_allow_angle",
        FnId::Sma => "sma",
        FnId::Prev => "prev",
        FnId::Blend => "blend",
        FnId::Deriv => "deriv",
        FnId::Vote => "vote",
        FnId::Stable => "stable",
        FnId::LearnMax => "learn_max",
        FnId::Latch => "latch",
    }
}

/// 参数个数 (min, max); max = usize::MAX 表示变长 (min/max 二元起步)
pub fn arity(fid: FnId) -> (usize, usize) {
    match fid {
        FnId::Min | FnId::Max => (2, usize::MAX),
        FnId::Abs | FnId::Sqrt | FnId::Sin | FnId::Cos | FnId::Exp | FnId::Ln | FnId::Floor
        | FnId::Ceil | FnId::IsValid | FnId::IsNan => (1, 1),
        FnId::Na => (0, 0),
        FnId::Atan2 | FnId::Round => (2, 2),
        FnId::Clamp => (3, 3),
        FnId::Lerp => (5, 5),
        FnId::Interp1d | FnId::Interp1dEx => (3, 3),
        FnId::Interp2d => (5, 5),
        FnId::IsaPressure | FnId::IsaDensity | FnId::IsaTemp | FnId::IasPerMach => (1, 1),
        FnId::IasToTas | FnId::TasToIas => (2, 2),
        FnId::Invalid => (0, 0),
        FnId::FmVne | FnId::FmMne => (1, 1),
        FnId::FmAoaHigh => (2, 2),
        FnId::FmFlapAllowSpeed | FnId::FmFlapAllowAngle => (2, 2),
        FnId::Sma => (2, 2),
        FnId::Prev => (1, 1),
        FnId::Blend => (2, 2),
        FnId::Deriv => (1, 1),
        FnId::Vote => (3, 3),
        FnId::Stable => (2, 2),
        FnId::LearnMax => (3, 3),
        FnId::Latch => (2, 2),
    }
}

/// 是否 FM 查表函数 (求值需上下文携带当前 blkx)
pub fn is_ctx_fn(fid: FnId) -> bool {
    matches!(
        fid,
        FnId::FmVne | FnId::FmMne | FnId::FmAoaHigh | FnId::FmFlapAllowSpeed | FnId::FmFlapAllowAngle
    )
}

/// 是否状态原语 (求值需 &mut StateStore)
pub fn is_stateful(fid: FnId) -> bool {
    matches!(
        fid,
        FnId::Sma | FnId::Prev | FnId::Blend | FnId::Deriv | FnId::Vote | FnId::Stable | FnId::LearnMax
            | FnId::Latch
    )
}

/// 求值期 FnId ↔ u16 双向映射 — 宏以声明序生成, 与枚举判别值恒一致
/// (手写映射曾在插入 FM 查表族后判别值移位, 分派错乱, 此处根治)
macro_rules! fn_id_codec {
    ($($v:ident),* $(,)?) => {
        pub fn fid_to_u16(fid: FnId) -> u16 {
            match fid {
                $(FnId::$v => FnId::$v as u16,)*
            }
        }
        /// 声明序即判别序 (无显式判别值的 enum 保证), 切片线性取回
        pub fn fid_from_u16(v: u16) -> Option<FnId> {
            const IDS: &[FnId] = &[$(FnId::$v),*];
            IDS.get(v as usize).copied()
        }
    };
}
fn_id_codec!(Abs, Min, Max, Sqrt, Sin, Cos, Atan2, Exp, Ln, Floor, Ceil, Round, Clamp, IsValid, Na, IsNan, Lerp, Interp1d, Interp1dEx, Interp2d, IsaPressure, IsaDensity, IsaTemp, IasToTas, TasToIas, IasPerMach, Invalid, FmVne, FmMne, FmAoaHigh, FmFlapAllowSpeed, FmFlapAllowAngle, Sma, Prev, Blend, Deriv, Vote, Stable, LearnMax, Latch);
/// 表实参取引用; 数值实参 → None (类型错误, 上层 NaN)
fn tbl(v: &Value) -> Option<&Arc<Vec<f64>>> {
    match v {
        Value::Table(t) => Some(t),
        Value::Num(_) => None,
    }
}

/// F_INVALID 哨兵 (parser/state.rs 全域约定, 公式侧同一约定)
const F_INVALID: f64 = -65535.0;

/// 纯函数求值 (arity 已由编译期检查; 状态原语不走这里)
pub fn eval_pure(fid: FnId, args: &[Value]) -> Value {
    use FnId as F;
    // 数值实参统一收集 (Table 混入会变 NaN, 由 num() 语义隔离)
    let nums: Vec<f64> = args.iter().map(|v| match v {
        Value::Num(x) => *x,
        Value::Table(_) => f64::NAN,
    })
    .collect();
    let v = match fid {
        F::Abs => nums[0].abs(),
        F::Min => nums.iter().cloned().fold(f64::INFINITY, f64::min),
        F::Max => nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        F::Sqrt => nums[0].sqrt(),
        F::Sin => nums[0].sin(),
        F::Cos => nums[0].cos(),
        F::Atan2 => nums[0].atan2(nums[1]),
        F::Exp => nums[0].exp(),
        F::Ln => nums[0].ln(),
        F::Floor => nums[0].floor(),
        F::Ceil => nums[0].ceil(),
        F::Round => {
            // round(x, n): n 位小数四舍五入
            let k = 10f64.powi(nums[1] as i32);
            (nums[0] * k).round() / k
        }
        F::Clamp => nums[0].clamp(nums[1], nums[2]),
        F::IsValid => ((nums[0] != F_INVALID && !nums[0].is_nan()) as u8) as f64,
        F::Na => F_INVALID,
        F::IsNan => (nums[0].is_nan()) as u8 as f64,
        F::Lerp => interpolation::lerp(nums[0], nums[1], nums[2], nums[3], nums[4]),
        F::Interp1d => {
            // interp1d(x, xs表, ys表)
            match (tbl(&args[1]), tbl(&args[2])) {
                (Some(xs), Some(ys)) => interpolation::interp1d(nums[0], xs, ys),
                _ => f64::NAN,
            }
        }
        F::Interp1dEx => match (tbl(&args[1]), tbl(&args[2])) {
            (Some(xs), Some(ys)) => interpolation::interp1d_extrapolate(nums[0], xs, ys, nums[3] != 0.0),
            _ => f64::NAN,
        },
        F::Interp2d => {
            // interp2d(x, y, xs表, ys表, zz表of表) — zz 内层暂以 ys 等长拍平不支持, 阶段 3 接 FM 推力表时扩展
            let _ = (tbl(&args[2]), tbl(&args[3]));
            f64::NAN
        }
        F::IsaPressure => atmosphere_model::pressure(nums[0]),
        F::IsaDensity => atmosphere_model::density_at_altitude(nums[0]),
        F::IsaTemp => atmosphere_model::temperature_at_altitude(15.0, nums[0]),
        F::IasToTas => atmosphere_model::ias_to_tas(nums[0], nums[1]),
        F::TasToIas => atmosphere_model::tas_to_ias(nums[0], nums[1]),
        F::IasPerMach => {
            // ias(km/h)/mach = 1 时的声速组合分母: 3.6*sqrt(1.4/1.225*101325*(1-kh)^e)
            // 与 derive / service_loop 的手写 mach 式逐项一致
            let h = nums[0];
            3.6 * (1.4 / 1.225 * 101325.0 * (1.0 - 0.0000225577 * h).powf(5.25588)).sqrt()
        }
        // 状态原语/FM 查表不由本函数处理 (编译期已分流); 防御性兜底 NaN
        F::Sma | F::Prev | F::Blend | F::Deriv | F::Vote | F::Stable | F::LearnMax | F::Latch
        | F::FmVne | F::FmMne | F::FmAoaHigh | F::FmFlapAllowSpeed | F::FmFlapAllowAngle => f64::NAN,
        F::Invalid => f64::NAN,
    };
    Value::Num(v)
}
