//! 公式系统黑盒场景 (kernel::formula): 出厂集装载 / 求值语义 / 会话量 /
//! 编译错误 / 用户 delta。求值经 FormulaManager::eval_frame (pub 面),
//! 输入 = 手造 State/SessionInputs/MetaInputs。

use std::sync::Arc;

use expect_test::expect;
use kernel::formula::registry::{MetaInputs, RawInputs, SessionInputs};
use kernel::formula::{persistence, FormulaManager};
use kernel::game_api::parser::State;

const FORMULAS_CFG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../formulas.cfg");

fn loaded() -> FormulaManager {
    let m = FormulaManager::new();
    m.install(&persistence::load_merged(FORMULAS_CFG, ""));
    m
}

/// state 速造 (eval_frame 的输入面; 能量系公式吃 TAS 域)
fn state(ias: i32, tas: i32, alt: f64) -> State {
    let mut s = State::new();
    s.ias = ias;
    s.tas = tas;
    s.heightm = alt;
    s
}

fn eval(m: &FormulaManager, raw: &RawInputs, session: &SessionInputs) -> (Vec<(String, f64)>, Arc<kernel::formula::VarSnapshot>) {
    let meta = MetaInputs {
        interval_ms: 50.0,
        freq: 20.0,
        ..Default::default()
    };
    let (results, snap) = m.eval_frame(raw, session, &meta, 0);
    let set = m.current();
    let mut out: Vec<(String, f64)> = set
        .formulas
        .iter()
        .enumerate()
        .map(|(slot, f)| (f.def.name.clone(), results.get(slot as u16)))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    (out, snap)
}

fn fmt_results(v: &[(String, f64)]) -> String {
    v.iter()
        .filter(|(_, x)| !x.is_nan())
        // f64::MAX 类哨兵值 (无 FM 的 allow_speed 族) 显示为 MAX, 免 300 位数字
        .map(|(n, x)| {
            if x.abs() > 1e30 {
                format!("{n}=MAX")
            } else {
                format!("{n}={x:.4}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 出厂集装载: 公式名单与编译全集 (数量级 + 代表名)
#[test]
fn 出厂集_装载与名单() {
    let m = loaded();
    let set = m.current();
    let names: Vec<&str> = set.formulas.iter().map(|f| f.def.name.as_str()).collect();
    assert!(names.len() >= 20, "出厂公式应 ≥20 个 (实际 {})", names.len());
    for want in ["energy_jkg", "energy_m", "mach", "maneuver_index"] {
        assert!(names.contains(&want), "出厂集应含 {want}");
    }
}

/// 核心派生量求值: energy_jkg = v²/8 + g·h (单位总能量, v=TAS m/s), energy_m 换算米
#[test]
fn 求值_能量公式() {
    let m = loaded();
    let s = state(400, 720, 2000.0); // TAS 720 km/h = 200 m/s, 高度 2000m
    let raw = RawInputs {
        state: Some(&s),
        indic: None,
        fmdata: None,
    };
    let (v, _) = eval(&m, &raw, &SessionInputs::default());
    let want_energy = 200.0 * 200.0 / 8.0 + 9.8 * 2000.0; // g = 注册常量 9.8
    let got: Vec<_> = v
        .iter()
        .filter(|(n, _)| n == "energy_jkg" || n == "energy_m")
        .collect();
    assert_eq!(got.len(), 2, "能量两式应有效 (无 FM 不拦能量)");
    for (n, x) in &got {
        if n == "energy_jkg" {
            assert!(
                (x - want_energy).abs() < 1.0,
                "energy_jkg 期望 ~{want_energy:.1}, 实际 {x:.1}"
            );
        }
        if n == "energy_m" {
            expect!["2510"].assert_eq(&format!("{x:.0}")); // E/g (语义精度)
        }
    }
}

/// 无 FM 门: mach 接管公式在 fm_loaded=false → invalid (NaN), 不接管系统变量
#[test]
fn 求值_无FM时mach不接管() {
    let m = loaded();
    let s = state(400, 454, 2000.0);
    let raw = RawInputs {
        state: Some(&s),
        indic: None,
        fmdata: None,
    };
    let (v, _) = eval(&m, &raw, &SessionInputs::default());
    let mach = v.iter().find(|(n, _)| n == "mach");
    assert!(
        mach.is_some_and(|(_, x)| x.is_nan()),
        "无 FM 时 mach 应 invalid (hide-when-zero 依赖)"
    );
}

/// 编译错误面: 环引用公式 → 拒绝安装该集 (错误可见)
#[test]
fn 编译_环引用拒绝() {
    let m = FormulaManager::new();
    let defs = vec![
        kernel::formula::FormulaDef {
            name: "a".into(),
            expr: "b + 1".into(),
            ..Default::default()
        },
        kernel::formula::FormulaDef {
            name: "b".into(),
            expr: "a + 1".into(),
            ..Default::default()
        },
    ];
    m.install(&defs);
    let set = m.current();
    assert!(
        set.formulas.iter().all(|f| f.rexpr.is_none()),
        "环引用公式应全部无效"
    );
}

/// 出厂集求值全量快照 (回归锚: 改公式语义时 diff 可见)
#[test]
fn 求值_出厂集典型帧全量() {
    let m = loaded();
    let s = state(474, 454, 46.0); // 真机 p51d 快照同值 (IAS/TAS/高度)
    let raw = RawInputs {
        state: Some(&s),
        indic: None,
        fmdata: None,
    };
    let mut sess = SessionInputs::default();
    sess.total_fuel = 319.0;
    let (v, _) = eval(&m, &raw, &sess);
    let actual = fmt_results(&v);
    expect![[r#"
        acceleration=2522.2222
        aileron_lock_ratio=0.0000
        an=0.0000
        energy_jkg=2438.8015
        energy_m=248.8573
        flap_allow_angle=125.0000
        flap_allow_speed=MAX
        iastotascooff=0.9578
        is_downing_flap=0.0000
        maneuver_index=0.0000
        ny=0.0000
        rudder_lock_ratio=0.0000
        sep=16228.5840
        speed_limit_ratio=0.0000
        speed_raw=131.6667
        speedv=126.1111
        sum_speedv=126.1111
        turn_rate=0.0000
        turn_rds=0.0000
        vario=0.0000
        warn_stall=0.0000
        warn_vne=0.0000"#]]
    .assert_eq(&actual);
}
