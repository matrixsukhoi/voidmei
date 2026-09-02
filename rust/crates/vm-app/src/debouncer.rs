//! ConfigDebouncer — Java static configDebouncer 的线程化 (Controller.java:52-59)。
//! 重构波2 自 app_shell.rs 拆出。

use std::sync::atomic::Ordering;
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use vm_core::event::ui_state_events;

use crate::commands::{DebounceMsg, UiCommand};
use crate::controller_shared::ControllerShared;

/// Java Controller.java:59 `CONFIG_DEBOUNCE_MS = 200` (Rust 为 leading+trailing 窗口,
/// 见 [ConfigDebouncer] 头注: 200 = trailing 收尾间隔, leading 沿不受此延迟)
pub const CONFIG_DEBOUNCE_MS: u64 = 200;

/// 单线程防抖器 (leading + trailing): 首条消息**立即**触发刷新, 安静期 `delay`
/// 内的后续变更合并为末条再触发一次。
/// PORT(Java 语义偏差备案): Java 为纯尾沿 `pendingConfigRefresh.schedule(200ms)`
/// —— 配置变更要等满 200ms 才见预览变化, WYSIWYG 明显不跟手 (端到端 ~220ms 大头)。
/// Rust 增强 leading 沿: 开关类单发操作端到端降至 ~30ms; 滑条拖动中每窗口亦刷
/// 一次 (Java 是拖动中完全不刷), 稳态刷新率 ≤ 1/delay, 开销可控。
/// 跨 Controller 重建共享 (Java static; Rust 由 AppShell 持有, tx 分发进各核)。
pub struct ConfigDebouncer {
    tx: Option<Sender<DebounceMsg>>,
    join: Option<JoinHandle<()>>,
}

/// 防抖任务体 (Java Controller.java:525-536/573-576): refreshPreviews(key)/
/// refreshAllPreviews()。loadFromConfig 已挪至主线程调度点 (配置 !Send, 见模块头);
/// 此处只取世代号快照送刷新命令, 消费侧 win32 做守卫。
fn refresh_cmd(msg: DebounceMsg, shared: &ControllerShared) -> UiCommand {
    let generation = shared.preview_generation.load(Ordering::SeqCst);
    let changed_key = match msg {
        DebounceMsg::ConfigKey(ref k) if k == ui_state_events::ACTION_RESET_COMPLETED => {
            None // 全局重置: refreshAllPreviews (Controller.java:530)
        }
        DebounceMsg::ConfigKey(k) => Some(k),
        DebounceMsg::FmChanged => None, // FM_CHANGED: refreshAllPreviews
    };
    UiCommand::RefreshPreviews {
        changed_key,
        generation,
    }
}

impl ConfigDebouncer {
    /// `delay` 可注入 (测试用短间隔; 生产 [`CONFIG_DEBOUNCE_MS`])。
    /// 输出直送 win32 线程 UiCommand 通道 (D8 修正★2: 刷新动作离开本线程)。
    pub fn spawn(delay: Duration, out: Sender<UiCommand>, shared: Arc<ControllerShared>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<DebounceMsg>();
        let join = std::thread::Builder::new()
            .name("ConfigDebounce".to_string())
            .spawn(move || {
                while let Ok(first) = rx.recv() {
                    // leading: 首条立即刷 (跟手关键; 时序上 ReinitOverlays 参数仓
                    // 覆写先入队, 本命令紧随 → win32 消费序参数恒新)
                    let _ = out.send(refresh_cmd(first, &shared));
                    let mut last: Option<DebounceMsg> = None;
                    // 安静期窗口: 每到一条即重排 (cancel+reschedule 的电平等价)
                    loop {
                        match rx.recv_timeout(delay) {
                            Ok(next) => last = Some(next),
                            Err(RecvTimeoutError::Timeout) => break,
                            Err(RecvTimeoutError::Disconnected) => return,
                        }
                    }
                    // trailing: 窗口内有后续变更 → 末条生效 (Java 语义保留)
                    if let Some(last) = last {
                        let _ = out.send(refresh_cmd(last, &shared));
                    }
                }
            })
            .expect("ConfigDebounce 线程创建失败");
        ConfigDebouncer {
            tx: Some(tx),
            join: Some(join),
        }
    }

    pub fn sender(&self) -> Sender<DebounceMsg> {
        // shutdown 后取空句柄 (send 即 Err, 调用方一律 let _ 忽略)
        self.tx
            .clone()
            .unwrap_or_else(|| std::sync::mpsc::channel().0)
    }

    pub fn shutdown(&mut self) {
        // 先 drop 全部自有发送端 → recv 返回 Disconnected → 线程退出 → join。
        // (调用方持有的克隆 drop 前线程可能不退出 — join 前 Controller 已先行
        // drop, AppShell 字段逆序声明保证该次序)
        if let Some(j) = self.join.take() {
            self.tx = None;
            let _ = j.join();
        }
    }
}

impl Drop for ConfigDebouncer {
    fn drop(&mut self) {
        self.shutdown();
    }
}
