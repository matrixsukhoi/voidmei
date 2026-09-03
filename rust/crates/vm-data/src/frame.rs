//! 不可变帧快照 (重构波4): Service 线程每周期末构建 `Arc<Frame>` 经
//! [`FrameStore`] 原子发布; 跨线程读者 (渲染线程/语音/主线程) 零锁取整帧。
//! 取代原 `Arc<RwLock<ServiceData>>` 的共享读面 — 锁争用与持锁跨计算段
//! (feed_overlays_live 的 B-W2 备案) 一并消亡; Service 线程内部仍持
//! ServiceData 短锁读写 (单写者, 竞态面不变)。
//! 取数接口与 ServiceData 同款: impl FormulaView (公式槽 > 会话 > 直通三层)。

use std::sync::{Arc, RwLock};

use vm_core::fm::FMHandle;
use vm_core::formula::registry::{self, FormulaView, MetaVar, VarSrc};
use vm_core::formula::rules::RuleTriggered;
use vm_core::formula::FormulaResults;
use vm_core::game_api::parser::{Indicators, MapInfo, State};

use crate::service_fields::{AltScalars, EngineScalars, FuelScalars, ServiceData};

/// 一帧完整快照 (跨线程可见集; Service 私有状态量 — SMA/prev 族 — 不入帧)。
/// State/Indicators/MapInfo 每帧整体克隆 (向量族 ~几 KB @20Hz, 可忽略;
/// 换取读者永远拿到一致帧)。
/// 波17 F14: 派生标量按语义聚合为 engine/fuel/altm 三组, 与 ServiceData
/// 同持, 拷贝整组搬 (原 53 行手写字段拷贝收敛)。
pub struct Frame {
    // API 对象整帧
    pub s_state: Option<State>,
    pub s_indic: Option<Indicators>,
    pub mapinfo: Option<MapInfo>,
    pub loc: Option<[f64; 2]>,
    pub dir: Option<[f64; 2]>,
    /// R1 周期 FM 句柄快照
    pub fm: Arc<FMHandle>,
    /// C 级会话量 (原每帧组装搬运层, 波4 起内嵌帧内)
    pub session: registry::SessionInputs,
    /// 帧序号 (FrameStore publish 自增; 消费侧按序号去重, 取代原
    /// rule_triggers 的 drain 语义)
    pub frame_seq: u64,
    /// 跨线程写点的原子镜像 (VoiceWarning set_fatal_warn / Controller openpad)
    pub fatal_warn: bool,
    pub start_time: i64,
    // 派生标量组镜像 (ServiceData 同名组, 分组语义见 service_fields.rs)
    pub engine: EngineScalars,
    pub fuel: FuelScalars,
    pub altm: AltScalars,
    // 平铺杂项 (时钟/方向/会话量, 无成组语义)
    pub freq: i64,
    pub current_time_ms: i64,
    pub actual_interval_ms: i64,
    pub elapsed_time: i64,
    pub compass_delta: f64,
    pub cur_load_min_work_time: f64,
    pub player_live: bool,
    pub n_vy: f64,
    // 公式产物
    pub formula_values: FormulaResults,
    pub formula_slots: Arc<std::collections::HashMap<String, u16>>,
    pub rule_triggers: Vec<RuleTriggered>,
}

impl Frame {
    /// Service 周期末从 ServiceData 读快照构建 (读锁内一次成帧; state/indic
    /// 深拷换一致帧语义)。`fatal_warn`/`start_time` 走原子镜像入参 (跨线程写点)。
    pub fn from_service_data(d: &ServiceData) -> Frame {
        Frame {
            s_state: d.s_state.clone(),
            s_indic: d.s_indic.clone(),
            mapinfo: d.mapinfo.clone(),
            loc: d.loc,
            dir: d.dir,
            fm: Arc::clone(&d.fm),
            session: crate::service_loop::session_inputs(d),
            frame_seq: 0,      // FrameStore::publish 覆写
            fatal_warn: false, // FrameStore::publish 镜像真值
            start_time: 0,     // FrameStore::publish 镜像真值
            // 派生标量三组整组搬 (波17 F14)
            engine: d.engine.clone(),
            fuel: d.fuel.clone(),
            altm: d.altm.clone(),
            freq: d.freq,
            current_time_ms: d.current_time_ms,
            actual_interval_ms: d.actual_interval_ms,
            elapsed_time: d.elapsed_time,
            compass_delta: d.compass_delta,
            cur_load_min_work_time: d.cur_load_min_work_time,
            player_live: d.player_live,
            n_vy: d.n_vy,
            formula_values: d.formula_values.clone(),
            formula_slots: Arc::clone(&d.formula_slots),
            rule_triggers: d.rule_triggers.clone(),
        }
    }
}

// --- FormulaView: 与 ServiceData 同款三层优先 (公式槽 > 会话字段 > 直通) ---

impl FormulaView for Frame {
    fn var_value(&self, name: &str) -> Option<f64> {
        if let Some(&slot) = self.formula_slots.get(name) {
            let v = self.formula_values.get(slot);
            if !v.is_nan() {
                return Some(v);
            }
        }
        let vid = registry::registry().lookup(name)?;
        let src = &registry::registry().vars[vid as usize].src;
        let v = match src {
            VarSrc::State(f) => self.s_state.as_ref().map(f)?,
            VarSrc::Indic(f) => self.s_indic.as_ref().map(f)?,
            VarSrc::Blk(f) => self.fm.fmdata.as_ref().map(f)?,
            VarSrc::Session(f) => f(&self.session),
            VarSrc::Const(c) => *c,
            VarSrc::Meta(m) => match m {
                MetaVar::IntervalMs => self.actual_interval_ms.max(1) as f64,
                MetaVar::Freq => self.freq as f64,
                MetaVar::FmLoaded => (self.fm.fmdata.is_some()) as u8 as f64,
                _ => 0.0,
            },
        };
        if v.is_nan() {
            None
        } else {
            Some(v)
        }
    }

    fn get_formula_value(&self, name: &str) -> Option<f64> {
        let slot = self.formula_slots.get(name)?;
        let v = self.formula_values.get(*slot);
        if v.is_nan() {
            None
        } else {
            Some(v)
        }
    }
}

/// 帧仓: 单写者 (Service 线程) 原子换 `Arc<Frame>`, 多读者零锁 clone。
/// 锁只在 clone Arc 的纳秒窗口持有 (20Hz 单写 + 3 读者, 无竞争面)。
/// None = Service 已装配但尚未产出首帧。
#[derive(Default)]
pub struct FrameStore {
    inner: RwLock<Option<Arc<Frame>>>,
    seq: std::sync::atomic::AtomicU64,
    /// fatal_warn 跨线程写点 (VoiceWarning set_fatal_warn, 渲染线程写;
    /// Service 帧发布时镜像入 Frame)
    fatal_warn: std::sync::atomic::AtomicBool,
    /// start_time 跨线程写点 (Controller openpad, 主线程写; Service 每帧读)
    start_time: std::sync::atomic::AtomicI64,
}

impl FrameStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 发布新帧 (Service 线程独占调用); frame_seq 自增并写入帧,
    /// fatal_warn/start_time 镜像原子真值入帧
    pub fn publish(&self, mut frame: Frame) {
        use std::sync::atomic::Ordering;
        frame.frame_seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        frame.fatal_warn = self.fatal_warn.load(Ordering::SeqCst);
        frame.start_time = self.start_time.load(Ordering::SeqCst);
        *self.inner.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(frame));
    }

    /// 最近一帧 (读者零锁; 尚无帧 = None, 消费方按"等待数据"降级)
    pub fn latest(&self) -> Option<Arc<Frame>> {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    // ---- 跨线程写点 (波4: 原子真相源, 见字段注) ----
    pub fn set_fatal_warn(&self, v: bool) {
        self.fatal_warn
            .store(v, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn fatal_warn(&self) -> bool {
        self.fatal_warn.load(std::sync::atomic::Ordering::SeqCst)
    }
    pub fn set_start_time(&self, v: i64) {
        self.start_time
            .store(v, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn start_time_load(&self) -> i64 {
        self.start_time.load(std::sync::atomic::Ordering::SeqCst)
    }
}
