//! AppShell 生命周期黑盒场景: 状态机 (Init→Preview→Connected→InGame)、
//! confirm/start/stop 链、flags 丢失会话清理、静默流超时、换机。
//! 全部经 pub 面驱动 (ui_bus 事件 + pump + dispatch), 不 spawn 真窗口。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


mod common;

use std::sync::Arc;

use common::*;
use kernel::base::event::ui_state_events;
use voidmei::{ControllerState, UiCommand};

/// UI_READY → Preview (Java uiReadyHandler → Preview 链)
#[test]
fn 生命周期_UI_READY进Preview() {
    let mut shell = fixture();
    assert_eq!(state_of(&shell), ControllerState::Init);
    send_ui_event_and_pump(&mut shell, ui_state_events::UI_READY, "");
    assert_eq!(state_of(&shell), ControllerState::Preview);
}

/// confirm 链: Preview → endPreview → start → Service 线程起 + live 登记
#[test]
fn 生命周期_confirm起Service() {
    let mut shell = fixture();
    send_ui_event_and_pump(&mut shell, ui_state_events::UI_READY, "");
    shell.dispatch(UiCommand::StartGame);
    let c = shell.controller.as_ref().unwrap();
    assert_eq!(c.state(), ControllerState::Init, "end_preview 后置 INIT");
    assert!(c.service.is_some(), "start() 应建 Service 线程");
    assert!(
        shell.shared.live.read().unwrap().is_some(),
        "live 帧仓应登记"
    );
}

/// Service 轮询驱动: flags 真 + playerLive → InGame→Preview + FM identify 提交
/// + 会话首机记名
#[test]
fn 生命周期_drive_from_live进Preview并识别() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
        assert_eq!(c.state(), ControllerState::InGame);
    }
    set_live(&shell, live_store_of(&live_service_data("test-plane")));
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(state_of(&shell), ControllerState::Preview, "changeS3 应进 PREVIEW");
    assert_eq!(
        shell.fm.current_target_name().as_deref(),
        Some("test-plane")
    );
    assert_eq!(
        shell.shared.flags.lock().unwrap().session_aircraft_type.as_deref(),
        Some("test-plane"),
        "会话首机记名"
    );
}

/// flags 丢失 → S4toS1: Preview → Init + 识别目标/会话记忆清除
#[test]
fn 生命周期_flags丢失清会话() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
    }
    let mut data = live_service_data("p1");
    let live_store = live_store_of(&data);
    set_live(&shell, Arc::clone(&live_store));
    shell.controller.as_mut().unwrap().drive_from_live(); // 进 Preview + identify(p1)

    // flags 翻假 (真实断连形态: 对象保留, flag=false)
    data.s_state.as_mut().unwrap().flag = false;
    data.s_indic.as_mut().unwrap().flag = false;
    live_store.publish(data::frame::Frame::from_service_data(&data));

    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(state_of(&shell), ControllerState::Init);
    assert_eq!(shell.fm.current_target_name(), None, "会话结束清识别目标");
    assert!(
        shell.shared.flags.lock().unwrap().session_aircraft_type.is_none(),
        "会话机型记忆清除"
    );
}

/// 静默流超时 (游戏退出, 事件流停更): S4toS1 退出 + 稳定停 Init + 清识别目标。
/// 时间戳经 shared.last_flight_event_ms 直写 (真机由 Service 事件发布面更新)
#[test]
fn 生命周期_静默流超时退出() {
    use std::sync::atomic::Ordering;
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
    }
    let data = live_service_data("p1");
    set_live(&shell, live_store_of(&data));
    let now = kernel::base::java_compat::current_time_millis();
    shell
        .shared
        .last_flight_event_ms
        .store(now - 100, Ordering::SeqCst); // 新鲜
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(state_of(&shell), ControllerState::Preview);

    // 事件停更超阈值 (flags/playerLive 陈旧真值保留 — 真实断连形态)
    let now = kernel::base::java_compat::current_time_millis();
    shell.shared.last_flight_event_ms.store(
        now - voidmei::FLIGHT_SILENT_EXIT_MS - 100,
        Ordering::SeqCst,
    );
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(state_of(&shell), ControllerState::Init, "静默超时应 S4toS1");
    assert_eq!(shell.fm.current_target_name(), None, "会话结束清识别目标");
    // 再一轮 (仍静默): 稳定停 Init 不回弹
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(state_of(&shell), ControllerState::Init);
}

/// 换机: 机型名变化 → FM 目标跟随 + 会话机型记忆更新 (轻量 swap, 不重启核)
#[test]
fn 生命周期_换机轻量swap() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
    }
    let live_store = live_store_of(&live_service_data("plane-a"));
    set_live(&shell, Arc::clone(&live_store));
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(shell.fm.current_target_name().as_deref(), Some("plane-a"));

    let live_store2 = live_store_of(&live_service_data("plane-b"));
    set_live(&shell, live_store2);
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(
        shell.fm.current_target_name().as_deref(),
        Some("plane-b"),
        "换机应切识别目标"
    );
    assert_eq!(
        state_of(&shell),
        ControllerState::Preview,
        "换机走轻量 swap, 核状态不回退"
    );
    assert_eq!(
        shell.shared.flags.lock().unwrap().session_aircraft_type.as_deref(),
        Some("plane-b"),
        "会话机型记忆跟随"
    );
}

/// stop 五步销毁: Service join 干净 + live 撤销 + service 句柄收
#[test]
fn 生命周期_stop五步收线() {
    let mut shell = fixture();
    send_ui_event_and_pump(&mut shell, ui_state_events::UI_READY, "");
    shell.dispatch(UiCommand::StartGame);
    assert!(shell.controller.as_ref().unwrap().service.is_some());

    // 五步销毁 (对位 Java Controller.stop): ①gen/CloseAll → ②退订 → ③设置窗
    // → ④停 Service → ⑤存配置
    shell
        .controller
        .as_mut()
        .unwrap()
        .stop(&mut shell.release_main_form);
    let c = shell.controller.as_ref().unwrap();
    assert!(c.service.is_none(), "步④后 Service 句柄应收");
    assert!(
        shell.shared.live.read().unwrap().is_none(),
        "live 帧仓应随停机清空"
    );
}

/// EndGame (mCancel) 语义: 置退出标志, 不动 Service
#[test]
fn 生命周期_EndGame置退出标志() {
    let mut shell = fixture();
    send_ui_event_and_pump(&mut shell, ui_state_events::UI_READY, "");
    shell.dispatch(UiCommand::StartGame);
    shell.dispatch(UiCommand::EndGame);
    assert!(shell.is_exit_requested(), "mCancel 应置退出标志");
    assert!(
        shell.controller.as_ref().unwrap().service.is_some(),
        "EndGame 不停 Service (退出归 run_supervisor 收尾)"
    );
}
