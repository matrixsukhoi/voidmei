//! --headless 状态机驱动 (无窗口验收工具, E10 自 main_form.rs 状态机核心提离)。
//!
//! 构建真实表单 (读真实 ui_layout.cfg) → 驱动固定 Message 序列 → 断言 WYSIWYG
//! 链路。6 段同构的 first_row_of_type 块收敛为步骤表 ([`STEPS`]) 数组循环,
//! verdict 文案与失败计数泛型化为 [`Report`]。仅 vm-ui bin (--headless) 与
//! CI 验收消费, 不进状态机核心。

use std::sync::{Arc, Mutex};

use vm_core::base::bus::Subscription;
use vm_core::base::bus::ui_state_bus::{UIStateBus, UiStateEvent};
use vm_core::base::event::ui_state_events;
use vm_core::config::configuration_service::ConfigurationService;

use super::{update, MainFormState, Message};
use crate::renderers;

/// 仓库模板 ui_layout.cfg 的 CWD 相对候选 (cargo run 自 rust/ 或手工自仓库根)。
pub fn locate_template_cfg() -> Option<&'static str> {
    const CANDIDATES: &[&str] = &[
        "./ui_layout.cfg",
        "../ui_layout.cfg",
        "../../ui_layout.cfg",
        "../../../ui_layout.cfg",
    ];
    CANDIDATES
        .iter()
        .copied()
        .find(|p| std::path::Path::new(p).exists())
}

/// 单步驱动器: 对定位到的 (panel, key) 行发消息并断言, 返回 (ok, 打印段全文)
/// — 段文本由各驱动器自持, 保持逐段原输出形态
type StepDrive = fn(&mut MainFormState, panel: &str, key: &str) -> (bool, String);

/// 步骤表: (行类型, 驱动器) — 数据驱动的固定序列 (原 6 段同构块收敛)
const STEPS: &[(&str, StepDrive)] = &[
    ("SWITCH", step_switch),
    ("SWITCH_INV", step_switch_inv),
    ("SLIDER", step_slider),
    ("COMBO", step_combo),
    ("COLOR", step_color),
];

/// 开关链路: 翻转 → 服务/快照/总线事件
fn step_switch(state: &mut MainFormState, panel: &str, key: &str) -> (bool, String) {
    let before = state.service_string(key);
    let new_val = !before.eq_ignore_ascii_case("true");
    update(state, Message::Toggle { panel: panel.into(), key: key.into(), value: new_val });
    let after = state.service_string(key);
    let ok = after == new_val.to_string();
    (
        ok,
        format!("Toggle {key}: 服务 {before:?} → {after:?} (期望 {new_val})"),
    )
}

/// 反相开关链路: 显示 true → 落库 false
fn step_switch_inv(state: &mut MainFormState, panel: &str, key: &str) -> (bool, String) {
    update(state, Message::Toggle { panel: panel.into(), key: key.into(), value: true });
    let after = state.service_string(key);
    let ok = after == "false";
    (ok, format!("SwitchInv {key}: 显示 true → 服务 {after:?} (期望 false)"))
}

/// 滑条链路: 区间中点 → 快照行值 + 服务值 (不落盘)
fn step_slider(state: &mut MainFormState, panel: &str, key: &str) -> (bool, String) {
    let row = state.snapshot_row(panel, key);
    let (min, max) = row.as_ref().map_or((0, 100), |r| (r.min_val, r.max_val));
    let (min, max) = renderers::slider::effective_range(min, max);
    let v = min + (max - min) / 2;
    update(state, Message::Slider { panel: panel.into(), key: key.into(), value: v });
    let row_ok = state.snapshot_row(panel, key).is_some_and(|r| r.get_int() == v);
    let svc = state.service_string(key);
    let ok = row_ok && svc == v.to_string();
    (ok, format!("Slider {key}={v}: 快照行值+服务 {svc:?} (期望 {v})"))
}

/// 下拉链路: 重选当前值 → 服务保持 + 快照 Str
fn step_combo(state: &mut MainFormState, panel: &str, key: &str) -> (bool, String) {
    let current = state.service_string(key);
    update(state, Message::Combo { panel: panel.into(), key: key.into(), value: current.clone() });
    let svc = state.service_string(key);
    let row_ok = state
        .snapshot_row(panel, key)
        .is_some_and(|r| r.get_str() == current);
    let ok = svc == current && row_ok;
    (ok, format!("Combo {key}={current:?}: 服务 {svc:?} + 快照行值"))
}

/// 颜色链路: 选色 → 主键十进制写服务 + 快照行值 (分键 keyR/G/B/A 为忠实
/// no-op 写, cfg 无对应行, 语义由 color.rs MapCtx 单测断言)
fn step_color(state: &mut MainFormState, panel: &str, key: &str) -> (bool, String) {
    let rgba = [232u8, 147, 50, 200];
    let decimal = "232, 147, 50, 200";
    update(state, Message::ColorPicked { panel: panel.into(), key: key.into(), value: rgba });
    let svc = state.service_string(key);
    let row_ok = state
        .snapshot_row(panel, key)
        .is_some_and(|r| r.get_str() == decimal);
    let ok = svc == decimal && row_ok;
    (
        ok,
        format!("ColorPicked {key}: 服务 {svc:?} + 快照行值 (期望 {decimal:?})"),
    )
}

/// 验收报告器: verdict 文案 + 失败计数 (原内联闭包/裸计数的泛型化)
struct Report {
    failures: u32,
}

impl Report {
    fn verdict(ok: bool) -> &'static str {
        if ok { "PASS" } else { "FAIL" }
    }

    /// 记一步结果, 返回该步 verdict 文案
    fn record(&mut self, ok: bool) -> &'static str {
        self.failures += u32::from(!ok);
        Self::verdict(ok)
    }
}

/// 无窗口状态机测试: 构建真实表单 → 驱动固定 Message 序列 → 断言 WYSIWYG 链路。
/// 返回进程退出码 (0 = 全部通过)。
///
/// `persist_path` (CLI `--persist <path>`): 固定序列落盘到指定路径 — D9 换框架
/// 验收工具 (相同序列在新旧 UI 层各跑一次 → ui_layout.user.cfg 逐字节 diff=0)。
/// None = 不落盘 (原纯链路断言形态)。
pub fn run_headless(persist_path: Option<String>) -> i32 {
    let Some(cfg_path) = locate_template_cfg() else {
        eprintln!("vm-ui: --headless 未找到 ui_layout.cfg (候选: ./ ../ ../../ ../../../)");
        return 2;
    };
    let bus = Arc::new(UIStateBus::new());
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub: Subscription<UiStateEvent> = bus.subscribe(
        ui_state_events::CONFIG_CHANGED,
        move |m: &UiStateEvent| {
            s2.lock().unwrap().push(m.clone());
        },
    );

    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.load_layout(cfg_path);
    if persist_path.is_some() {
        println!(
            "vm-ui: --persist 基线模式, 固定序列将落盘至 {}",
            persist_path.as_deref().unwrap_or_default()
        );
    }
    let mut state = MainFormState::new(config, Arc::clone(&bus), persist_path);
    println!(
        "vm-ui: --headless 表单构建成功: {} panels / {} rows (源: {cfg_path})",
        state.panel_count(),
        state.row_count()
    );
    let mut report = Report { failures: 0 };

    // 1)~5) 行类型驱动链: 步骤表循环 (cfg 缺该类型行时 SKIP)
    for (row_type, drive) in STEPS {
        if let Some((panel, key)) = state.first_row_of_type(row_type) {
            let (ok, text) = drive(&mut state, &panel, &key);
            println!("vm-ui: [{}] {}", report.record(ok), text);
        } else {
            println!("vm-ui: [SKIP] cfg 无 {row_type} 行");
        }
    }

    // 6) 保存链路: 广播 CONFIG_CHANGED("ui_layout.cfg")
    {
        seen.lock().unwrap().clear();
        update(&mut state, Message::Save);
        let published = seen
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.event_type == ui_state_events::CONFIG_CHANGED && e.data.as_deref() == Some("ui_layout.cfg"));
        println!("vm-ui: [{}] Save 广播 CONFIG_CHANGED(\"ui_layout.cfg\")", report.record(published));
    }

    if report.failures == 0 {
        println!("vm-ui: --headless 状态机全部通过");
        0
    } else {
        eprintln!("vm-ui: --headless 失败 {} 项", report.failures);
        1
    }
}
