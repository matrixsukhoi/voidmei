//! 公式系统命令层 (公式管理编辑器 tab 的全部后端面)。
//!
//! 全部**直算模式** (commands_comparison 先例): 计算面只依赖 vm-core 的
//! formula 模块 (FormulaManager 自身线程安全), 不经主线程 dispatcher,
//! vm-app 的 form_dispatch 零改动。
//!
//! 共享态 (E11 注入形态统一): [`FormulaShared`] — FormRuntime 字段, 经 tauri
//! State 分发到本层 (vm-app 装配方写: 启动桥注入 + 会话 start 覆盖 Service
//! 实例), 原 commands_formula 全局静态桥已退役。
//!
//! 设计: doc/formula_system_design.md §9 (偏离备案: 原设计 CRUD 走
//! dispatcher 类, 实施时发现 FormulaManager 全方法线程安全, 直算更简)。

use std::sync::Arc;

use vm_core::formula::{FormulaDef, FormulaManager};

use crate::ipc::FormulaShared;

/// 当前 manager (State 同源读 — 不经主线程 dispatcher)
fn manager(state: &FormulaShared) -> Arc<FormulaManager> {
    state.get()
}

/// 编辑器试算的状态原语参数 (三处 try_eval 共用): now_ms=0 = 时间基准从零跑,
/// interval_ms=50 = 近似 Service 轮询间隔
const TRY_EVAL_NOW_MS: u64 = 0;
const TRY_EVAL_INTERVAL_MS: f64 = 50.0;

// =====================================================================
// DTO
// =====================================================================

/// 列表/编辑条目 (serde camelCase, 对齐 dto.rs 惯例)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaItemDto {
    pub name: String,
    pub expr: String,
    pub unit: String,
    pub precision: u8,
    pub desc: String,
    pub disabled: bool,
    pub builtin: bool,
    /// 编译错误 (只读, 后端生成)
    #[serde(skip_deserializing)]
    pub error: Option<String>,
}

impl From<&FormulaDef> for FormulaItemDto {
    fn from(d: &FormulaDef) -> Self {
        FormulaItemDto {
            name: d.name.clone(),
            expr: d.expr.clone(),
            unit: d.unit.clone(),
            precision: d.precision,
            desc: d.desc.clone(),
            disabled: d.disabled,
            builtin: d.builtin,
            error: None,
        }
    }
}

fn def_of(dto: &FormulaItemDto) -> FormulaDef {
    FormulaDef {
        name: dto.name.clone(),
        expr: dto.expr.clone(),
        unit: dto.unit.clone(),
        precision: dto.precision,
        desc: dto.desc.clone(),
        disabled: dto.disabled,
        builtin: dto.builtin,
    }
}

/// 变量目录条目
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VarCatalogEntryDto {
    pub name: String,
    pub unit: String,
    pub desc: String,
    pub category: String,
    /// 数据来源标签 (如 "8111 /state"; 公式产出 = "公式")
    pub origin: String,
    /// 原始来源枚举键 (前端筛选: state/indicators/derived/fm/meta/const/formula)
    #[serde(rename = "originKey")]
    pub origin_key: String,
    /// 公式产出变量: 最近一帧值 (系统变量为 None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// 公式产出变量: 接管了同名系统变量 (如内置 mach)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub overrides_system: bool,
}

/// 校验/试算结果
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaEvalDto {
    pub ok: bool,
    pub value: Option<f64>,
    pub error: Option<String>,
}

// =====================================================================
// 命令
// =====================================================================

/// 公式列表 (含编译错误标注)
#[tauri::command]
pub async fn get_formula_list(
    state: tauri::State<'_, FormulaShared>,
) -> Result<Vec<FormulaItemDto>, String> {
    let mgr = manager(&state);
    let set = mgr.current();
    Ok(set
        .formulas
        .iter()
        .map(|f| {
            let mut dto = FormulaItemDto::from(&f.def);
            dto.error = f.err.as_ref().map(|e| e.to_string());
            dto
        })
        .collect())
}

/// 单公式校验 (语法/未知符号/arity; 空快照即可 — 不求值状态原语)
#[tauri::command]
pub async fn formula_validate(
    expr: String,
    state: tauri::State<'_, FormulaShared>,
) -> Result<FormulaEvalDto, String> {
    let mgr = manager(&state);
    let snap = mgr.last_snapshot();
    // fm_blkx=None: 编辑器试算暂无 FM 句柄 (W3 补桥), 查表函数得 NaN
    let r = mgr.try_eval(&expr, &snap, TRY_EVAL_NOW_MS, TRY_EVAL_INTERVAL_MS, None);
    Ok(match r {
        Ok(_) => FormulaEvalDto { ok: true, value: None, error: None },
        Err(e) => FormulaEvalDto { ok: false, value: None, error: Some(e) },
    })
}

/// 单公式试算 (最近一帧快照; 状态原语从零跑 — 编辑期近似)
#[tauri::command]
pub async fn formula_try_eval(
    expr: String,
    state: tauri::State<'_, FormulaShared>,
) -> Result<FormulaEvalDto, String> {
    let mgr = manager(&state);
    let snap = mgr.last_snapshot();
    // fm_blkx=None: 编辑器试算暂无 FM 句柄 (W3 补桥), 查表函数得 NaN
    let r = mgr.try_eval(&expr, &snap, TRY_EVAL_NOW_MS, TRY_EVAL_INTERVAL_MS, None);
    Ok(match r {
        Ok(v) => FormulaEvalDto { ok: true, value: Some(v), error: None },
        Err(e) => FormulaEvalDto { ok: false, value: None, error: Some(e) },
    })
}

/// 变量目录 (统一命名空间: 系统变量 + 公式产出变量, 设计 §5 — 公式即变量)
#[tauri::command]
pub async fn get_var_catalog(
    state: tauri::State<'_, FormulaShared>,
) -> Result<Vec<VarCatalogEntryDto>, String> {
    let mut out: Vec<VarCatalogEntryDto> = vm_core::formula::registry()
        .catalog()
        .into_iter()
        .map(|(name, unit, desc, cat, origin)| VarCatalogEntryDto {
            name: name.to_string(),
            unit: unit.to_string(),
            desc: desc.to_string(),
            category: format!("{cat:?}"),
            origin: origin.label().to_string(),
            origin_key: format!("{origin:?}").to_lowercase(),
            value: None,
            overrides_system: false,
        })
        .collect();
    // 公式产出变量 (State 恒有 manager — 出厂空集时循环体自然为空)
    {
        let mgr = manager(&state);
        let set = mgr.current();
        let snap = mgr.last_snapshot();
        let reg = vm_core::formula::registry();
        for f in &set.formulas {
            if f.err.is_some() {
                continue;
            }
            // 最近帧值: 试算路径 (独立状态仓+无 FM 句柄, 编辑期近似)
            let value = mgr
                .try_eval(&f.def.expr, &snap, TRY_EVAL_NOW_MS, TRY_EVAL_INTERVAL_MS, None)
                .unwrap_or(f64::NAN);
            out.push(VarCatalogEntryDto {
                name: f.def.name.clone(),
                unit: f.def.unit.clone(),
                desc: if f.def.desc.is_empty() {
                    format!("公式: {}", f.def.expr)
                } else {
                    f.def.desc.clone()
                },
                category: "Formula".to_string(),
                origin: "公式".to_string(),
                origin_key: "formula".to_string(),
                value: if value.is_finite() { Some(value) } else { None },
                // 同名系统变量 = 接管其值 (设计 §5 同名规则)
                overrides_system: reg.lookup(&f.def.name).is_some(),
            });
        }
    }
    Ok(out)
}

/// 最近一帧变量快照 (试算面板 "当前数据" 列)
#[tauri::command]
pub async fn get_last_var_snapshot(
    state: tauri::State<'_, FormulaShared>,
) -> Result<serde_json::Value, String> {
    let mgr = manager(&state);
    let snap = mgr.last_snapshot();
    let reg = vm_core::formula::registry();
    let names: Vec<String> = reg.vars.iter().map(|v| v.name.to_string()).collect();
    // values 是 f64 — NaN/inf JSON 不合法, 序列化为 null
    let values: Vec<Option<f64>> = snap.values.iter().map(|v| {
        if v.is_finite() {
            Some(*v)
        } else {
            None
        }
    }).collect();
    Ok(serde_json::json!({ "names": names, "values": values }))
}

/// 全量保存 + 热更新 (编辑器保存链)
#[tauri::command]
pub async fn save_formulas(
    items: Vec<FormulaItemDto>,
    state: tauri::State<'_, FormulaShared>,
) -> Result<FormulaEvalDto, String> {
    let mgr = manager(&state);
    let defs: Vec<FormulaDef> = items.iter().map(def_of).collect();
    match mgr.save_all(&defs) {
        Ok(()) => Ok(FormulaEvalDto { ok: true, value: None, error: None }),
        Err(e) => Ok(FormulaEvalDto { ok: false, value: None, error: Some(e) }),
    }
}

/// 恢复出厂公式
#[tauri::command]
pub async fn reset_formulas(
    state: tauri::State<'_, FormulaShared>,
) -> Result<FormulaEvalDto, String> {
    let mgr = manager(&state);
    match mgr.reset_to_builtin() {
        Ok(()) => Ok(FormulaEvalDto { ok: true, value: None, error: None }),
        Err(e) => Ok(FormulaEvalDto { ok: false, value: None, error: Some(e) }),
    }
}
