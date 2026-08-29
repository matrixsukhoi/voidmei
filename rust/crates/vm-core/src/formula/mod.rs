//! 公式系统: 内置+自定义数学公式的统一承载 (设计: doc/formula_system_design.md)。
//!
//! 三层: registry(L0 原子变量) → definition/eval(L1 公式 DAG 求值) → rules(L2, 阶段5)。
//! 求值收敛 Service 线程单点 (裁决 A1); 引擎全静态注册, 零反射零新依赖。

pub mod ast;
pub mod definition;
pub mod eval;
pub mod functions;
pub mod lexer;
pub mod parser;
pub mod persistence;
pub mod registry;
pub mod rules;

pub use definition::{CompiledFormulaSet, FormulaDef, FormulaResults, VarLookup};
pub use eval::{EvalCtx, StateStore};
pub use functions::{FnId, Value};
pub use registry::{assemble_snapshot, registry, MetaInputs, Registry, VarSnapshot, VarSrc};

use crate::ui_model::fm_data_source::FMDataSource;
use crate::ui_model::telemetry_source::TelemetrySource;
use std::sync::{Arc, Mutex, RwLock};

// ---------------------------------------------------------------------------
// :target 统一解析 (设计 §8): getter 名 | 短名 | 公式名 | "X * N" 乘数语法
// (乘数语法复刻 reflect_binder 的 java_split_star 语义)。三静态表
// (fields/overlays_field1/flight_info) 改走本解析器随阶段 2 A 级外置同步接线。
// ---------------------------------------------------------------------------

/// 解析产物: 注册表变量 (仅 Telemetry 源) 或公式名 (延迟到编译集判定)
#[derive(Debug, Clone, PartialEq)]
pub enum TargetVar {
    Var(u16),
    Formula(String),
}

/// 解析 :target 字符串 → (变量, 乘数)。未知名按 Java NoSuchMethod 语义
/// 返回公式名形态 (取值时 0 降级), 不在此处失败 — cfg 用户容错。
pub fn resolve_target(target: &str) -> Option<(TargetVar, f64)> {
    let (name, mult) = match target.split_once('*') {
        Some((n, m)) => (n.trim(), m.trim().parse::<f64>().ok()?),
        None => (target.trim(), 1.0),
    };
    if let Some(vid) = registry().lookup(name) {
        return Some((TargetVar::Var(vid), mult));
    }
    Some((TargetVar::Formula(name.to_string()), mult))
}

/// 求值: 变量限 Telemetry 源 (fm.*/元变量/常量应进公式, 不进 :target);
/// 公式经 TelemetrySource::get_formula_value (None = 0 降级, Java 语义)。
pub fn target_value(var: &TargetVar, mult: f64, s: &dyn TelemetrySource) -> Option<f64> {
    match var {
        TargetVar::Var(vid) => {
            let meta = registry().vars.get(*vid as usize)?;
            match &meta.src {
                VarSrc::Tel(f) => Some(f(s) * mult),
                VarSrc::TelBool(f) => Some(f(s) as u8 as f64 * mult),
                _ => None,
            }
        }
        TargetVar::Formula(name) => s.get_formula_value(name).map(|v| v * mult),
    }
}

/// 公式系统管理器: 持当前编译集 (原子换 Arc 热更新) + 状态原语仓。
/// 求值唯一发生在 Service 线程; Mutex 仅为试算命令跨线程防御。
pub struct FormulaManager {
    current: RwLock<Arc<CompiledFormulaSet>>,
    store: Mutex<StateStore>,
    /// 最近一帧快照缓存 (eval_frame 更新; 编辑器试算数据源)
    last_snap: RwLock<Arc<VarSnapshot>>,
}

impl Default for FormulaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FormulaManager {
    pub fn new() -> Self {
        FormulaManager {
            current: RwLock::new(Arc::new(CompiledFormulaSet::compile(
                &[],
                registry(),
                &[],
            ))),
            store: Mutex::new(StateStore::new()),
            last_snap: RwLock::new(Arc::new(VarSnapshot::empty(registry().len()))),
        }
    }

    /// 最近一帧变量快照 (无数据帧 = 全 NaN, 公式自然降级)
    pub fn last_snapshot(&self) -> Arc<VarSnapshot> {
        self.last_snap.read().expect("快照锁中毒").clone()
    }

    /// 安装新公式集 (编辑器保存/启动装载): 编译 → 差集清状态 → 原子换 Arc
    pub fn install(&self, defs: &[FormulaDef], external_refs: &[String]) {
        let set = Arc::new(CompiledFormulaSet::compile(defs, registry(), external_refs));
        let alive = set.alive_sites();
        if let Ok(mut store) = self.store.lock() {
            store.retain_sites(&alive);
        }
        *self.current.write().expect("公式集锁中毒") = set;
    }

    /// 帧求值: 组快照 → 拓扑序求值 → 结果 (Service 线程调用)
    pub fn eval_frame(
        &self,
        tel: &dyn TelemetrySource,
        fm: Option<&dyn FMDataSource>,
        fm_blkx: Option<&crate::blkx::Blkx>,
        meta: &MetaInputs,
        now_ms: u64,
    ) -> FormulaResults {
        let set = self.current.read().expect("公式集锁中毒").clone();
        let snap = assemble_snapshot(tel, fm, meta);
        let results = if set.formulas.is_empty() {
            FormulaResults { values: Vec::new() }
        } else {
            let mut store = self.store.lock().expect("状态仓锁中毒");
            set.eval_frame(&snap, &mut store, now_ms, meta.interval_ms, fm_blkx)
        };
        // 快照缓存供编辑器试算 (求值后 move, 免克隆)
        *self.last_snap.write().expect("快照锁中毒") = Arc::new(snap);
        results
    }

    /// 重置全部状态原语 (FM_CHANGED 换机 / 会话重置)
    pub fn reset_states(&self) {
        if let Ok(mut store) = self.store.lock() {
            store.reset_all();
        }
    }

    /// 当前编译集快照 (只读引用)
    pub fn current(&self) -> Arc<CompiledFormulaSet> {
        self.current.read().expect("公式集锁中毒").clone()
    }

    /// 单公式试算 (编辑器校验/预览; 独立状态仓不污染主集)
    pub fn try_eval(
        &self,
        expr: &str,
        snap: &VarSnapshot,
        now_ms: u64,
        interval_ms: f64,
        fm_blkx: Option<&crate::blkx::Blkx>,
    ) -> Result<f64, String> {
        let mut store = StateStore::new();
        definition::try_eval_single(expr, registry(), snap, &mut store, now_ms, interval_ms, fm_blkx)
            .map_err(|e| e.to_string())
    }

    /// 当前公式定义列表 (编辑器载入)
    pub fn current_defs(&self) -> Vec<FormulaDef> {
        self.current
            .read()
            .expect("公式集锁中毒")
            .formulas
            .iter()
            .map(|f| f.def.clone())
            .collect()
    }

    /// 保存全部公式并热更新 (编辑器保存链): 写用户文件 → 重新编译安装
    pub fn save_all(&self, defs: &[FormulaDef]) -> Result<(), String> {
        let refs: Vec<String> = defs.iter().map(|d| d.name.clone()).collect();
        persistence::save_user(defs, persistence::USER_FORMULAS_PATH)
            .map_err(|e| format!("写入公式文件失败: {e}"))?;
        self.install(defs, &refs);
        Ok(())
    }

    /// 恢复出厂公式 (用户文件清空, 重装内置)
    pub fn reset_to_builtin(&self) -> Result<(), String> {
        let defs = persistence::load_merged(persistence::BUILTIN_FORMULAS_PATH, "");
        self.save_all(&defs)
    }

    /// 从 formulas.cfg + formulas.user.cfg 装载并安装 (全部公式进 live 集 —
    /// :target 引用收窄待三表接线后)。
    /// 调用点: Service::new (每会话) 与 desktop_main 启动桥 (Service 未装配的
    /// preview/空闲期编辑器可用, Service 装配时 publish 覆盖为会话实例)。
    pub fn load_from_files(&self) {
        let defs = persistence::load_merged(
            persistence::BUILTIN_FORMULAS_PATH,
            persistence::USER_FORMULAS_PATH,
        );
        let refs: Vec<String> = defs.iter().map(|d| d.name.clone()).collect();
        self.install(&defs, &refs);
    }
}

#[cfg(test)]
mod tests;
