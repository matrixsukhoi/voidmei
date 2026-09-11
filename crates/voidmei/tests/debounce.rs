//! ConfigDebouncer 黑盒场景: 防抖窗口合并 (leading+trailing)、CONFIG_CHANGED
//! 触达链 (广播送达 + 配置读回)、跨核重建存活。
//! 只经 voidmei pub 面驱动 (ConfigDebouncer/DebounceMsg/UiCommand + fixture 的
//! ui_bus/pump/config), 不 spawn 渲染线程/真窗口。
//! 观测边界: 防抖输出通道 (ui_cmd_rx) 是 AppShell 私有面 — shell 内防抖器的
//! 逐次触发无黑盒读面; 场景一以同款注入 (30ms = ShellParts.debounce_delay)
//! 自建防抖器直验合并语义, 场景二/三经 fixture 验全链触达。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


mod common;

use std::sync::Arc;
use std::time::Duration;

use common::*;
use expect_test::expect;
use kernel::base::event::ui_state_events;
use kernel::config::config_api::ConfigProvider; // set_config/get_config trait 面
use voidmei::{
    AppShell, ConfigDebouncer, ControllerShared, ControllerState, DebounceMsg, UiCommand,
};

/// 防抖窗内多次 CONFIG_CHANGED (ConfigKey 转发形态) 合并: 首条立即刷 (leading)
/// + 末条安静期收尾 (trailing), 刷新链恰好两次 — 30ms 与 ShellParts.debounce_delay
/// 同款注入值。接收端自持 (测试 = 渲染线程替身; fixture 下该通道为私有面)。
#[test]
fn 防抖_窗口内连发合并为首尾两次刷新() {
    let shared = Arc::new(ControllerShared::new());
    let (tx, rx) = std::sync::mpsc::channel::<UiCommand>();
    let mut deb = ConfigDebouncer::spawn(Duration::from_millis(30), tx, Arc::clone(&shared));
    let sender = deb.sender();
    for k in ["k1", "k2", "k3", "k4", "k5"] {
        // 5ms 间隔连发: 总跨度 20ms 全部落在彼此的 30ms 安静窗内 (CI 慢机留 6 倍余量)
        sender.send(DebounceMsg::ConfigKey(k.to_string())).unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(sender); // 发送端克隆 drop, 防 Disconnected 前的消息堆积干扰

    // leading: 首条 k1 立即送达 (远小于 30ms 窗)
    let leading = rx
        .recv_timeout(Duration::from_millis(200))
        .expect("leading 沿应立即送达首条刷新");
    // trailing: 窗口收尾, 末条 k5 生效
    let trailing = rx
        .recv_timeout(Duration::from_millis(500))
        .expect("trailing 沿应送达末条刷新");
    let snap = expect![[r#"
        RefreshPreviews { changed_key: Some("k1"), generation: 0 }
        RefreshPreviews { changed_key: Some("k5"), generation: 0 }"#]];
    snap.assert_eq(&format!("{leading:?}\n{trailing:?}"));
    // 安静期无第三条 (连发只产生 leading+trailing 两次, 不多刷)
    assert!(
        rx.recv_timeout(Duration::from_millis(120)).is_err(),
        "连发只产生 leading+trailing 两次刷新"
    );
    deb.shutdown();
    assert!(
        rx.recv_timeout(Duration::from_millis(50)).is_err(),
        "shutdown 后不应再有输出"
    );
}

/// 触达链: Preview 态下逐次写配置 → CONFIG_CHANGED 广播每次恰好送达一次
/// (ui_bus 订阅探针) + 末值可读回 (set_config 写树同步) → pump 转发进防抖链
/// (fixture 30ms) 全程无异常。
#[test]
fn 防抖_触达链广播逐次到达且配置读回末值() {
    let mut shell = fixture();
    send_ui_event_and_pump(&mut shell, ui_state_events::UI_READY, "");
    assert_eq!(state_of(&shell), ControllerState::Preview);

    // 广播探针: 只数 CONFIG_CHANGED 的送达 (publish 栈内同步执行)
    let hits = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let probe = Arc::clone(&hits);
    let sub = shell.ui_bus.subscribe(
        ui_state_events::CONFIG_CHANGED,
        move |msg: &kernel::base::bus::ui_state_bus::UiStateEvent| {
            if let Some(k) = msg.data.as_deref() {
                if k == "crosshairSwitch" || k == "enableEngineControl" {
                    probe.lock().unwrap().push(k.to_string());
                }
            }
        },
    );
    let set_cfg = |shell: &mut AppShell, key: &str, val: &str| {
        shell.controller.as_ref().unwrap().config.set_config(key, val);
    };
    // 同键连发三值 + 异键一发 (全部落进防抖窗口的源事件流);
    // set 后 pump 把转发事件经 handle_main_event 送进防抖器
    set_cfg(&mut shell, "crosshairSwitch", "false");
    shell.pump();
    set_cfg(&mut shell, "crosshairSwitch", "true");
    shell.pump();
    set_cfg(&mut shell, "crosshairSwitch", "false");
    set_cfg(&mut shell, "enableEngineControl", "true");
    shell.pump();

    let snap = expect![[r#"
        ["crosshairSwitch", "crosshairSwitch", "crosshairSwitch", "enableEngineControl"]
        false
        true"#]];
    let c = shell.controller.as_ref().unwrap();
    let readback = format!(
        "{:?}\n{}\n{}",
        hits.lock().unwrap(),
        c.config.get_config("crosshairSwitch").unwrap(),
        c.config.get_config("enableEngineControl").unwrap(),
    );
    snap.assert_eq(&readback);
    drop(sub); // 探针退订, 不泄漏给后续测试
}

/// 跨核存活 (Java static configDebouncer 语义): 托盘重建 (rebuild_controller)
/// 换核不动 AppShell — 重建后 CONFIG_CHANGED→防抖链仍正常消化 (新核出厂默认
/// 树读回 + 写值到达)。防抖输出通道为私有面无逐次计数, 存活性以链路运转为准;
/// 防抖器线程生命周期由 AppShell 持有 (Drop→shutdown) 兜底。
#[test]
fn 防抖_跨核重建后链路仍运转() {
    let mut shell = fixture();
    send_ui_event_and_pump(&mut shell, ui_state_events::UI_READY, "");
    shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .set_config("crosshairSwitch", "false");
    shell.pump(); // 旧核经防抖消化

    shell.rebuild_controller(false); // 托盘 Activate 路径: 旧核 stop + 磁盘重装载
    // 新核出厂默认树 (crosshairSwitch 出厂 true) — 证明核确实换新、不继承旧树
    let fresh = shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .get_config("crosshairSwitch")
        .unwrap();

    // 新核重走 UI_READY→Preview, CONFIG_CHANGED 再经防抖链消化
    send_ui_event_and_pump(&mut shell, ui_state_events::UI_READY, "");
    assert_eq!(state_of(&shell), ControllerState::Preview);
    shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .set_config("crosshairSwitch", "false");
    shell.pump();
    let after = shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .get_config("crosshairSwitch")
        .unwrap();

    let snap = expect![[r#"
        true
        false"#]];
    snap.assert_eq(&format!("{fresh}\n{after}"));
}
