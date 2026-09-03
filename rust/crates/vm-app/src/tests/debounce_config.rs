//! 防抖与 WYSIWYG 配置写链 (波11 自 tests.rs 分片)

use super::*;

/// 连发变更 (leading+trailing): 首条立即刷 + 末条安静期后收尾, 之后无更多
#[test]
fn debounce_leading_immediate_and_trailing_final() {
    let shared = Arc::new(ControllerShared::new());
    let (out_tx, out_rx) = std::sync::mpsc::channel::<UiCommand>();
    let mut deb = ConfigDebouncer::spawn(Duration::from_millis(40), out_tx, Arc::clone(&shared));
    let tx = deb.sender();
    for k in ["k1", "k2", "k3", "k4", "k5"] {
        tx.send(DebounceMsg::ConfigKey(k.to_string())).unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(tx); // shutdown 前 drop 全部发送端克隆, 否则 join 等 Disconnected 永阻塞
              // leading: 首条 k1 立即 (30ms 门槛 < 纯尾沿最早 65ms = k5@25ms + 窗 40ms,
              // 区分两种实现且留调度余量)
    match out_rx.recv_timeout(Duration::from_millis(30)) {
        Ok(UiCommand::RefreshPreviews {
            changed_key,
            generation,
        }) => {
            assert_eq!(changed_key, Some("k1".to_string()), "首条立即刷 (leading)");
            assert_eq!(
                generation,
                shared.preview_generation.load(Ordering::SeqCst),
                "世代号为发送时快照"
            );
        }
        other => panic!("leading 沿应立即送达: {:?}", other),
    }
    // trailing: 窗口内连发合并, 末条 k5 生效
    match out_rx.recv_timeout(Duration::from_millis(500)) {
        Ok(UiCommand::RefreshPreviews { changed_key, .. }) => {
            assert_eq!(
                changed_key,
                Some("k5".to_string()),
                "末条变更收尾 (trailing)"
            );
        }
        other => panic!("trailing 沿应送达末条刷新: {:?}", other),
    }
    // 安静期无第三条 (leading+trailing 各一次, 不多刷)
    assert!(
        out_rx.recv_timeout(Duration::from_millis(120)).is_err(),
        "连发只产生 leading+trailing 两次刷新"
    );
    deb.shutdown();
}

/// FmChanged → 全量刷新 (changed_key=None); RESET_COMPLETED → 全量。
/// (leading 下单发场景只此一条: 窗口内无后续 → trailing 不触发)
#[test]
fn debounce_fm_and_reset_full_refresh() {
    let shared = Arc::new(ControllerShared::new());
    let (out_tx, out_rx) = std::sync::mpsc::channel::<UiCommand>();
    let mut deb = ConfigDebouncer::spawn(Duration::from_millis(30), out_tx, shared);
    {
        let tx = deb.sender();
        tx.send(DebounceMsg::FmChanged).unwrap();
    } // 块尾 drop 发送端克隆
    match out_rx.recv_timeout(Duration::from_millis(500)) {
        Ok(UiCommand::RefreshPreviews {
            changed_key: None, ..
        }) => {}
        other => panic!("FmChanged 应产全量刷新: {:?}", other),
    }
    {
        let tx = deb.sender();
        tx.send(DebounceMsg::ConfigKey(
            ui_state_events::ACTION_RESET_COMPLETED.to_string(),
        ))
        .unwrap();
    }
    match out_rx.recv_timeout(Duration::from_millis(500)) {
        Ok(UiCommand::RefreshPreviews {
            changed_key: None, ..
        }) => {}
        other => panic!("RESET_COMPLETED 应产全量刷新: {:?}", other),
    }
    deb.shutdown();
}

// ------------------------------------------------------------------
// WYSIWYG 链 (CONFIG_CHANGED → 防抖 → RefreshPreviews)
// ------------------------------------------------------------------

/// Preview 态: CONFIG_CHANGED → 转发 → 防抖 (30ms) → RefreshPreviews(键) 命令
#[test]
fn wysiwyg_config_change_refreshes_via_debounce() {
    let mut shell = fixture_with_debounce(30);
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    // MainForm 侧配置写入 (服务内联发布 CONFIG_CHANGED — vm-ui 链同源)
    publish_ui_event(
        &shell.ui_bus,
        ui_state_events::CONFIG_CHANGED,
        "showSpeedBar",
    );
    assert!(
        pump_events(&mut shell),
        "CONFIG_CHANGED 应经转发到达监督循环"
    );
    // 防抖产出直达渲染线程命令通道 — 接收端留在 shell (未 spawn 渲染线程)。
    // 容忍 UI_READY→preview() 的 Preview-Refresh 线程全量刷新 (None) 抢先入队
    let mut keyed = None;
    let mut generation_seen = 0u64;
    for _ in 0..4 {
        match shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(500))
        {
            Ok(UiCommand::RefreshPreviews {
                changed_key: Some(k),
                generation,
            }) => {
                keyed = Some(k);
                generation_seen = generation;
                break;
            }
            Ok(_) => continue, // 全量刷新 (Preview-Refresh/FmChanged) — 竞态容忍
            Err(e) => panic!("防抖后应有键控刷新: {:?}", e),
        }
    }
    assert_eq!(keyed.as_deref(), Some("showSpeedBar"), "最后一条变更生效");
    assert_eq!(
        generation_seen,
        shell.shared.preview_generation.load(Ordering::SeqCst)
    );
}

/// 非 Preview 态: CONFIG_CHANGED → 立即 ReinitActiveOverlays (无防抖)
#[test]
fn config_change_non_preview_reinits_active() {
    let mut shell = fixture_with_debounce(30);
    publish_ui_event(&shell.ui_bus, ui_state_events::CONFIG_CHANGED, "fontNum");
    assert!(pump_events(&mut shell));
    let cmd = shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(200))
        .expect("fontNum 触发五色直送 + ReinitActiveOverlays");
    // 五色键: 先 SetGlobalColors (fixture cfg 五色全白 #FFFFFFFF), 后 Reinit
    let all_white = GlobalColors {
        num: [255, 255, 255, 255],
        label: [255, 255, 255, 255],
        unit: [255, 255, 255, 255],
        warning: [255, 255, 255, 255],
        shade_shape: [255, 255, 255, 255],
    };
    assert_eq!(cmd, UiCommand::SetGlobalColors(all_white));
    // WYSIWYG reinit 参数直送 (ReinitActiveOverlays 前置): 参数包随命令进
    // 渲染线程 (配置 !Send — 五色直送同款模式)
    let cmd_rp = shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(200))
        .expect("ReinitActiveOverlays 前应先到 ReinitOverlays");
    match cmd_rp {
        UiCommand::ReinitOverlays { params } => {
            // fixture cfg 无地平仪组 → Java reinitConfig 缺省 150×300
            assert_eq!(params.attitude.width, 150);
        }
        other => panic!("应是 ReinitOverlays: {:?}", other),
    }
    let cmd2 = shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(200))
        .expect("非 Preview 态应立即 ReinitActiveOverlays");
    assert_eq!(cmd2, UiCommand::ReinitActiveOverlays);
    // 防抖不排队 (无后续刷新命令)
    assert!(shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(100))
        .is_err());
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init
    );
}

/// CONFIG_CHANGED 的 reinit 参数直送: 配置写入值即时进 ReinitOverlays 参数包
/// (写 "地平仪" 组的 attitudeIndicatorWidth 行 → attitude_width)
#[test]
fn config_change_reinit_params_carry_written_config() {
    // cfg 需含目标行 (Java setConfig 只改既有行, 无行即 no-op)
    let cfg = fixture_cfg(
        "(panel \"地平仪\" :visible true\n\
             \x20 (item \"宽\" :type slider :target \"attitudeIndicatorWidth\" :value 150))\n",
    );
    let mut shell = fixture_full(30, cfg);
    // 发布方 (渲染器) 已写配置树 — 本测试直接经 set_config 模拟写点
    shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .set_config("attitudeIndicatorWidth", "222");
    publish_ui_event(
        &shell.ui_bus,
        ui_state_events::CONFIG_CHANGED,
        "attitudeIndicatorWidth",
    );
    assert!(pump_events(&mut shell));
    let mut saw_reinit = None;
    for _ in 0..4 {
        match shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(200))
        {
            Ok(UiCommand::ReinitOverlays { params }) => {
                saw_reinit = Some(params);
                break;
            }
            Ok(_) => continue,
            Err(e) => panic!("应有 ReinitOverlays: {:?}", e),
        }
    }
    let params = saw_reinit.expect("ReinitOverlays 应到达");
    // 写值即时进参数包 (初值 150, 写 222 — 证明非 spawn 期冻结快照)
    assert_eq!(params.attitude.width, 222, "写值应即时进参数包");
}

/// Preview 态: ReinitOverlays 先于防抖的 RefreshPreviews 入队
/// (渲染线程消费序 = 参数先刷新, 再跑各 overlay reinit — Java refreshPreviews →
/// reinitConfig 读即时配置的时序等价)
#[test]
fn preview_reinit_params_precede_debounced_refresh() {
    let mut shell = fixture_with_debounce(30);
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    publish_ui_event(
        &shell.ui_bus,
        ui_state_events::CONFIG_CHANGED,
        "showSpeedBar",
    );
    assert!(pump_events(&mut shell));
    let mut reinit_at: Option<usize> = None;
    let mut refresh_at: Option<usize> = None;
    for i in 0..6 {
        match shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(400))
        {
            Ok(UiCommand::ReinitOverlays { .. }) => reinit_at = Some(i),
            Ok(UiCommand::RefreshPreviews {
                changed_key: Some(k),
                ..
            }) if k == "showSpeedBar" => refresh_at = Some(i),
            Ok(_) => {}
            Err(_) => break,
        }
        if reinit_at.is_some() && refresh_at.is_some() {
            break;
        }
    }
    let (r, f) = (
        reinit_at.expect("ReinitOverlays 应到达"),
        refresh_at.expect("键控 RefreshPreviews 应到达"),
    );
    assert!(r < f, "参数直送 ({}) 应先于防抖刷新 ({})", r, f);
}

/// FM_CHANGED (missing-like) → 摘要转发 (toast 面) + Preview 态防抖全量刷新
#[test]
fn fm_changed_missing_schedules_full_refresh() {
    let mut shell = fixture_with_debounce(30);
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    // 直接向 FM 通道发布 missing-like 句柄 (Java FM-Loader 线程发布形态)
    let handle = FMHandle {
        name: Some("nosuch-plane".to_string()),
        status: FMStatus::Missing,
        ..FMHandle::UNRESOLVED
    };
    shell.fm.fm_changed_bus().publish(&handle);
    // 两条转发 (missing 摘要 + 刷新调度) 到达监督通道
    let mut saw_missing = false;
    while let Ok(ev) = shell.main_event_rx.recv_timeout(Duration::from_millis(200)) {
        if let MainEvent::FmChanged { name: Some(_), .. } = &ev {
            saw_missing = true;
        }
        shell.handle_main_event(ev);
    }
    assert!(saw_missing, "missing-like 摘要应转发");
    // Preview 态 → 防抖全量刷新命令
    let cmd = shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(500))
        .expect("FM_CHANGED 应触发防抖全量刷新");
    match cmd {
        UiCommand::RefreshPreviews {
            changed_key: None, ..
        } => {}
        other => panic!("应为全量刷新: {:?}", other),
    }
}

// ------------------------------------------------------------------
// 托盘重建 (Application.ctr 替换)
// ------------------------------------------------------------------

/// 激活缓存随配置装载 (渲染线程激活面的 WYSIWYG 输入)
#[test]
fn activation_cache_tracks_config() {
    let shell = fixture();
    let v = shell
        .activation
        .lock()
        .unwrap()
        .get("crosshairSwitch")
        .cloned();
    assert_eq!(
        v.as_deref(),
        Some("true"),
        "tmp cfg 的 crosshairSwitch=true 应入缓存"
    );
    let v2 = shell
        .activation
        .lock()
        .unwrap()
        .get("enableEngineControl")
        .cloned();
    assert_eq!(v2.as_deref(), Some("false"));
}

// ------------------------------------------------------------------
// 渲染线程生命周期 (真实窗口冒烟)
// ------------------------------------------------------------------

/// RefreshPreviews 不翻转会话窗口形态 (审查 blocker 回归锚): 游戏稳态
/// (openpad 后 overlay_ctx_preview=false, State=Preview) 下 FM_CHANGED/
/// ConfigChanged 防抖必发 RefreshPreviews — 原实现置 true 后生产路径无复位点,
/// feed_overlays_live 整帧 early-return, 6 窗口数据冻结; 修复 = 激活探测期
/// 临时置位 (Java refreshPreviews 的 forPreviewMode ctx), 完毕恢复会话形态。
/// CloseAllOverlays 复位 true (会话结束回预览态)。
///
/// 断言用轮询终态: refresh_preview 的激活探测期内物化真窗口 (Win32
/// CreateWindow 冷启动可达百 ms 级), 标志短暂为 true 是修复的合法瞬态 —
/// 轮询穿过后核对终值; 原 bug 形态下永久 true, 轮询超时必失败。
#[test]
fn refresh_previews_keep_session_window_mode() {
    fn wait_flag(flag: &AtomicBool, want: bool) -> bool {
        let start = Instant::now();
        while flag.load(Ordering::SeqCst) != want {
            if start.elapsed() > Duration::from_millis(2000) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        true
    }
    let mut shell = fixture();
    shell.spawn_render_thread().expect("渲染线程启动");
    *shell.shared.state.write().unwrap() = ControllerState::Preview;
    let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
    // 模拟游戏形态 (openpad → OpenAllOverlays 处理点置 false)
    shell
        .shared
        .overlay_ctx_preview
        .store(false, Ordering::SeqCst);
    shell.send_ui(UiCommand::RefreshPreviews {
        changed_key: None,
        generation: gen,
    });
    assert!(
        wait_flag(&shell.shared.overlay_ctx_preview, false),
        "游戏稳态刷新预览后 live 门控应恢复开启 (原 bug: 置 true 后 live 冻结)"
    );
    // CloseAll (stop/end_preview/S4toS1 的公共出口) → 回预览态
    shell.send_ui(UiCommand::CloseAllOverlays);
    assert!(
        wait_flag(&shell.shared.overlay_ctx_preview, true),
        "CloseAll 后窗口形态应回预览态"
    );
    shell.send_ui(UiCommand::Shutdown);
    let join = shell.render.take().unwrap();
    assert!(join.join().is_ok());
}

// ------------------------------------------------------------------
// 辅助
// ------------------------------------------------------------------
