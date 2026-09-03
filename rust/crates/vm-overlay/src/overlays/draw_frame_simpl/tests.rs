use super::*;
use std::cell::RefCell;
use std::rc::Rc;

const REGULAR: &str = "../../../fonts/sarasa-mono-sc-regular.ttf";

fn font(size: i32) -> LoadedFont {
    LoadedFont::new(std::path::Path::new(REGULAR), size).unwrap()
}

/// 读预乘 RGBA 像素 (overlays_field2 测试同约定)
fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
    [d[0], d[1], d[2], d[3]]
}

/// 直通色 → tiny-skia 预乘取整 ((c*a+127)/255), 断言基准用
fn premul(c: [u8; 4]) -> [u8; 4] {
    [
        ((c[0] as u32 * c[3] as u32 + 127) / 255) as u8,
        ((c[1] as u32 * c[3] as u32 + 127) / 255) as u8,
        ((c[2] as u32 * c[3] as u32 + 127) / 255) as u8,
        c[3],
    ]
}

/// 测试用喷气机 FM (3 高度档 × 3 速度档推力表; Java paintComponent 消费面:
/// velocityThr/maxThrAft/altitudeThr + alt/velThrNum)
fn jet_fmdata() -> FmData {
    let mut b = FmData::default();
    b.is_jet = true;
    b.vel_thr_num = 3;
    b.alt_thr_num = 3;
    let mut vt = [0.0; 30];
    vt[..3].copy_from_slice(&[100.0, 200.0, 400.0]);
    b.velocity_thr = Some(vt);
    let mut at = [0.0; 30];
    at[..3].copy_from_slice(&[0.0, 2000.0, 4000.0]);
    b.altitude_thr = Some(at);
    b.max_thr_aft = Some(vec![
        vec![3000.0, 2800.0, 2000.0],
        vec![2500.0, 2300.0, 1600.0],
        vec![2000.0, 1800.0, 1200.0],
    ]);
    b
}

// ---- chart_geometry: Java :557-592 公式逐式 基线 ----

/// 手算期望 (jet_blkx 输入): xn=[100,200,400] → xmin/xmax=100/400;
/// ymin=findMin(row2)=1200, ymax=findMax(row0)=3000 (对齐 10 后不变);
/// xgap=round(((401-100)/5)/5.0)*5 = round(12.0)*5 = 60;
/// ygap=round(((3001-1200)/5)/5.0)*5 = round(72.0)*5 = 360;
/// pxmin=100, pxmax=460, pymin=1200, pymax=3360;
/// ggx4=800/(460-100), ggy4=400/(3360-1200); rgbx=(int)(255/4)=63
#[test]
fn chart_geometry_oracle() {
    let g = chart_geometry(&jet_fmdata());
    assert_eq!((g.dwidth, g.dheight), (800, 400));
    assert_eq!(
        (g.xmin, g.xmax, g.ymin, g.ymax),
        (100.0, 400.0, 1200.0, 3000.0)
    );
    assert_eq!((g.xgap, g.ygap), (60, 360));
    assert_eq!((g.pxmin, g.pymin), (100, 1200));
    assert_eq!(g.ggx4, 800.0 / 360.0, "dwidth/(pxmax-pxmin)");
    assert_eq!(g.ggy4, 400.0 / 2160.0, "dheight/(pymax-pymin)");
    assert_eq!(g.rgbx, 63, "(int)(255.0f/4) 截断");
}

/// findMin/findMax 初值保真: 空数组返回 Java 初值 (Float.MAX_VALUE /
/// Float.MIN_VALUE=1.4e-45, 非 f32::MIN)
#[test]
fn find_min_max_empty_slice_returns_java_sentinels() {
    assert_eq!(find_min(&[]), f32::MAX as f64);
    assert_eq!(find_max(&[]), f64::from(f32::from_bits(1)));
    assert_eq!(find_min(&[5.0, -3.0, 9.0]), -3.0);
    assert_eq!(find_max(&[5.0, -3.0, 9.0]), 9.0);
}

/// java Math.round(float) 半-up 语义
#[test]
fn java_round_i32_half_up() {
    assert_eq!(java_round_f32(11.5), 12);
    assert_eq!(java_round_f32(11.4), 11);
    assert_eq!(java_round_f32(-11.5), -11, "floor(-11.5+0.5)=floor(-11)");
}

// ---- draw: 空缓存跳过 + 像素墨迹 (aa=false 精确断言) ----

/// 测试字体组 (12/16/18 三档, 同一 regular 文件)
fn dfs_fonts() -> (LoadedFont, LoadedFont, LoadedFont) {
    (font(12), font(16), font(18))
}

/// 无 FM / velThrNum==0 → paintComponent 直接 return (Java :554-555 null 守卫)
#[test]
fn draw_blank_without_fm_data() {
    let (f12, f16, f18) = dfs_fonts();
    let fonts = DfsFonts {
        num12: &f12,
        text16: &f16,
        text18: &f18,
        text12: &f12,
    };
    let mut cv = PixCanvas::new(900, 500).unwrap();
    DrawFrameSimpl::new().draw(&mut cv, &fonts, false);
    assert!(cv.pixmap().data().iter().all(|&b| b == 0), "无句柄全空");
    let mut b0 = jet_fmdata();
    b0.vel_thr_num = 0;
    let mut d = DrawFrameSimpl::new();
    d.reload_fm(Some(Arc::new(b0)));
    d.draw(&mut cv, &fonts, false);
    assert!(
        cv.pixmap().data().iter().all(|&b| b == 0),
        "velThrNum=0 跳过"
    );
}

/// 坐标系/数据点/图例的像素落点 (aa=false):
/// - x 轴 (50..850, y=460, 宽 3) 与 y 刻度 ii=0 (y=460) 同色叠 → 纯黑;
/// - 行1 首点 (v=100→px=50, thr=2500→py=219): dot fill_rect(49,218,2,2) 覆盖
///   y 轴黑底 (premul r=62 + 黑底 SrcOver 保 r);
/// - 图例行0 线段 (760..780, y=100, 宽 1, 灰 63) 无叠 → premul 精确
#[test]
fn draw_curve_pixels() {
    let (f12, f16, f18) = dfs_fonts();
    let fonts = DfsFonts {
        num12: &f12,
        text16: &f16,
        text18: &f18,
        text12: &f12,
    };
    let mut d = DrawFrameSimpl::new();
    d.reload_fm(Some(Arc::new(jet_fmdata())));
    let mut cv = PixCanvas::new(900, 500).unwrap();
    d.draw(&mut cv, &fonts, false);

    // x 轴内点: 黑 250 (与 y 刻度 ii=0 同色 SrcOver 叠加, alpha 只增不减)
    let axis = px(&cv, 200, 460);
    assert_eq!((axis[0], axis[1], axis[2]), (0, 0, 0), "x 轴纯黑");
    assert!(axis[3] >= 250, "alpha ≥ 250 (实测 {})", axis[3]);
    // y 轴 (x=50, y∈[60,460], 宽 3 → 列 49-51)
    let yaxis = px(&cv, 50, 300);
    assert_eq!((yaxis[0], yaxis[1], yaxis[2]), (0, 0, 0), "y 轴纯黑");
    assert!(yaxis[3] >= 250);

    // 行1 首点 dot (49,218): 灰 (1+1)*63=126 over 黑轴 → r 保持 premul 124
    let dot = px(&cv, 49, 218);
    assert_eq!(
        (dot[0], dot[1], dot[2]),
        (124, 124, 124),
        "数据点灰 126 直通的预乘"
    );
    assert!(dot[3] >= 250);
    // dot 下缘外一格仍是轴黑 (dot 恰 2×2)
    assert_eq!(px(&cv, 49, 220)[0], 0, "dot 外回轴黑");

    // 图例行0 线段 (760..780, y=100, 灰 63/α250, 透明底单覆盖)
    assert_eq!(px(&cv, 765, 100), premul([63, 63, 63, 250]), "图例线段");
    // 图例文本带 ("高度0m" @ 785 基线 105)
    assert!(
        (785..830).any(|x| px(&cv, x, 95)[3] > 0 || px(&cv, x, 104)[3] > 0),
        "图例文本墨迹"
    );
    // 标题带 ("推力-真空速曲线" @ x=450 基线 y=50, 字号 18)
    assert!(
        (300..620).any(|x| (30..50).any(|y| px(&cv, x, y)[3] > 0)),
        "标题墨迹"
    );
    // 画布全域有量级墨迹 (曲线 3 行 × 3 点 + 网格)
    let ink = cv
        .pixmap()
        .data()
        .chunks_exact(4)
        .filter(|p| p[3] > 0)
        .count();
    assert!(ink > 2000, "非零像素量级 (实测 {ink})");
}

// ---- spec 工厂 (Controller.java:746-752 registerWithStrategy) ----

fn feed_fm() -> Arc<FMManager> {
    Arc::new(FMManager::new(
        Arc::new(vm_core::base::bus::EventBus::new()),
    ))
}

/// 工厂初态 = initPreview 形态 (恒可见); spec 尺寸 = setBounds 字面量 900×500;
/// reinit 无 (Java reinitConfig 空); 空缓存渲染空画布
#[test]
fn draw_frame_simpl_spec_shape_and_factory_state() {
    let (h, mut spec) =
        draw_frame_simpl_spec(std::path::Path::new("../../../fonts"), &feed_fm()).unwrap();
    assert_eq!(
        (spec.id.as_str(), spec.config_key.as_str()),
        ("thrustdFS", "thrustdFS")
    );
    assert_eq!(
        (spec.width, spec.height),
        (900, 500),
        "setBounds(0, H-500, 900, 500)"
    );
    {
        let d = h.borrow();
        assert!(d.is_preview, "工厂 initPreview 形态");
        assert!(d.visible, "preview: always visible");
        assert!(d.should_show());
    }
    assert!(spec.reinit.is_none(), "Java reinitConfig :723-725 空实现");
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().all(|&b| b == 0), "空缓存空画布");
    // FM_CHANGED 装载后渲染通道出曲线 (render 闭包共享句柄)
    h.borrow_mut().reload_fm(Some(Arc::new(jet_fmdata())));
    let mut cv2 = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv2);
    assert!(cv2.pixmap().data().iter().any(|&b| b != 0), "曲线墨迹");
}

// ---- DrawFrameSimplFeed: run() 循环 (Java :737-767) ----

/// 最小 mock 窗口: 记 set_visible (FeedMockWin 同款)
struct FeedMockWin {
    log: Rc<RefCell<Vec<String>>>,
}

impl crate::platform::OverlayWindow for FeedMockWin {
    fn present(&mut self, buf: &[u8]) -> Result<(), String> {
        // 渲染实效断言用: 记录缓冲长度 (host/tests MockWindow 同款)
        self.log.borrow_mut().push(format!("present:{}", buf.len()));
        Ok(())
    }
    fn set_position(&mut self, _x: i32, _y: i32) {}
    fn position(&self) -> (i32, i32) {
        (60, 100)
    }
    fn set_click_through(&mut self, _on: bool) {}
    fn set_topmost(&mut self, _on: bool) {}
    fn set_visible(&mut self, visible: bool) {
        self.log.borrow_mut().push(format!("set_visible:{visible}"));
    }
    fn set_size(&mut self, _w: i32, _h: i32) {}
    fn poll_event(&mut self) -> Option<crate::platform::OverlayEvent> {
        None
    }
    fn screen_size(&self) -> (i32, i32) {
        (1920, 1080)
    }
}

fn feed_host(log: &Rc<RefCell<Vec<String>>>) -> OverlayHost {
    let log = Rc::clone(log);
    OverlayHost::with_factory(Box::new(move |_cfg| {
        Ok(Box::new(FeedMockWin {
            log: Rc::clone(&log),
        }) as Box<dyn crate::platform::OverlayWindow>)
    }))
}

/// 游戏会话: 隐藏起步 (init) → 1000ms 节流 → toggle 拉起 → 幂等;
/// preview 会话恒可见
#[test]
fn dfs_feed_game_visibility_flow() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) =
        draw_frame_simpl_spec(std::path::Path::new("../../../fonts"), &feed_fm()).unwrap();
    host.register(spec);
    host.open_all().unwrap();
    // 游戏形态 (渲染线程 OpenAllOverlays 处理点同款): init = 隐藏起步
    h.borrow_mut().init(None);
    let mut feed = DrawFrameSimplFeed::new();
    log.borrow_mut().clear();
    // ① 首轮 (displayFmKey=80 热键路径): 隐藏落窗
    feed.pump(&mut host, "thrustdFS", &h, 1_000, 80, None);
    assert_eq!(*log.borrow(), vec!["set_visible:false".to_string()]);
    // ② sleepQuietly(1000) 节流: 窗口内再 pump 无动作
    feed.pump(&mut host, "thrustdFS", &h, 1_500, 80, None);
    assert_eq!(log.borrow().len(), 1, "1000ms 内节流");
    // ③ FM_OVERLAY_TOGGLE → 拉起
    h.borrow_mut().toggle();
    feed.pump(&mut host, "thrustdFS", &h, 2_100, 80, None);
    assert_eq!(log.borrow().last().unwrap(), "set_visible:true");
    // ④ 稳态幂等 (Issue #54 防抖)
    feed.pump(&mut host, "thrustdFS", &h, 3_200, 80, None);
    assert_eq!(log.borrow().len(), 2);
    // ⑤ 预览会话: reset_preview 后恒可见 (flight=None 冻结退出判定)
    h.borrow_mut().reset_preview();
    feed.pump(&mut host, "thrustdFS", &h, 4_300, 0, None);
    assert_eq!(log.borrow().last().unwrap(), "set_visible:true");
    assert!(host.is_active("thrustdFS"), "预览无 Service 不退场");
}

/// displayFmKey==0 的收腿自动退场: 条件命中 → 10s 沉睡 (无窗口动作) →
/// dispose (host.close 销毁链: 存位置 + 窗口销毁) → run 线程终止短路;
/// 地面静止 (gear=100 & 低速) 不命中; reset 后 run 循环重生
#[test]
fn dfs_feed_auto_exit_when_gear_up() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) =
        draw_frame_simpl_spec(std::path::Path::new("../../../fonts"), &feed_fm()).unwrap();
    host.register(spec);
    host.open_all().unwrap();
    h.borrow_mut().init(None); // 游戏形态
    let mut feed = DrawFrameSimplFeed::new();
    log.borrow_mut().clear();
    // 收腿 (gear=0 ≠ 100) 命中: 本轮先隐藏再进入 10s 等待
    feed.pump(
        &mut host,
        "thrustdFS",
        &h,
        1_000,
        0,
        Some(DfsFlight {
            gear: 0,
            speedv: 0.0,
            throttle: 0,
        }),
    );
    assert_eq!(*log.borrow(), vec!["set_visible:false".to_string()]);
    // 等待期 (Java sleepQuietly(10000)): 线程沉睡, 无窗口动作
    feed.pump(
        &mut host,
        "thrustdFS",
        &h,
        6_000,
        0,
        Some(DfsFlight {
            gear: 0,
            speedv: 300.0,
            throttle: 100,
        }),
    );
    assert_eq!(log.borrow().len(), 1, "等待期无动作");
    // 到点: break → dispose (close 链存位置 + 销毁)
    feed.pump(
        &mut host,
        "thrustdFS",
        &h,
        11_100,
        0,
        Some(DfsFlight {
            gear: 0,
            speedv: 300.0,
            throttle: 100,
        }),
    );
    assert!(!host.is_active("thrustdFS"), "dispose 后窗口销毁");
    assert!(
        host.saved_position("thrustdFS").is_some(),
        "close 销毁链存位置 (saveCurrentPosition 面)"
    );
    // run 线程已死: 后续 pump 短路 (Java dispose 后实例僵在 entry 直至 closeAll)
    feed.pump(&mut host, "thrustdFS", &h, 20_000, 0, None);
    assert_eq!(log.borrow().len(), 1);
    // CloseAll 会话收尾 (渲染线程处理点同序: dfs_feed.reset + host.close_all —
    // close 清僵尸 instance=null) → 会话重开 run 循环重生 (Java 实例销毁后重建)
    feed.reset();
    host.close_all();
    host.open_all().unwrap();
    assert!(host.is_active("thrustdFS"), "会话重开 materialize");
    feed.pump(&mut host, "thrustdFS", &h, 21_000, 80, None);
    assert_eq!(
        log.borrow().last().unwrap(),
        "set_visible:false",
        "新 run 循环接管隐藏"
    );
}

/// Java run 自动退场的僵尸实例语义 (OverlayManager.java:294-299/:332-336):
/// 退场后 RefreshPreviews (refreshAllPreviews — MainForm 打开/全局键变更触达)
/// 与 openAll 均不重建死窗口; closeAll 清僵尸后才允许 materialize
#[test]
fn dfs_zombie_blocks_rematerialize_until_close_all() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) =
        draw_frame_simpl_spec(std::path::Path::new("../../../fonts"), &feed_fm()).unwrap();
    host.register(spec);
    host.open_all().unwrap();
    h.borrow_mut().init(None); // 游戏形态
    let mut feed = DrawFrameSimplFeed::new();
    log.borrow_mut().clear();
    // 收腿命中 → 10s → dispose + 僵尸化
    feed.pump(
        &mut host,
        "thrustdFS",
        &h,
        1_000,
        0,
        Some(DfsFlight {
            gear: 0,
            speedv: 0.0,
            throttle: 0,
        }),
    );
    feed.pump(
        &mut host,
        "thrustdFS",
        &h,
        11_100,
        0,
        Some(DfsFlight {
            gear: 0,
            speedv: 0.0,
            throttle: 0,
        }),
    );
    assert!(!host.is_active("thrustdFS"));
    // ① refreshAllPreviews (激活全真): 僵尸实例只跑 reinitializer, 不建窗
    host.refresh_preview().unwrap();
    assert!(!host.is_active("thrustdFS"), "僵尸不 materialize");
    // ② openAll 同跳过 (Java "already active")
    host.open_all().unwrap();
    assert!(!host.is_active("thrustdFS"), "openAll 跳过僵尸");
    // ③ closeAll 清僵尸 (instance=null) → 重开可 materialize (run 循环重生)
    feed.reset();
    host.close_all();
    host.open_all().unwrap();
    assert!(host.is_active("thrustdFS"), "closeAll 后会话重开");
}

/// 渲染实效 (游戏/预览两会话): present>0 且缓冲满幅 900×500×4 (BGRA) —
/// host 材质化→render→present 通道; 曲线像素非空面由 draw_curve_pixels 锁定
#[test]
fn dfs_present_frames_in_both_modes() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) =
        draw_frame_simpl_spec(std::path::Path::new("../../../fonts"), &feed_fm()).unwrap();
    host.register(spec);
    // jet FM 装载 (FM_CHANGED reload 面) — 曲线数据就位
    h.borrow_mut().reload_fm(Some(Arc::new(jet_fmdata())));
    // 游戏模式: open_all materialize → 首帧 present (指纹 None→Some)
    host.open_all().unwrap();
    host.render_tick().unwrap();
    assert!(
        log.borrow().iter().any(|l| l == "present:1800000"),
        "游戏模式 present 满幅缓冲 (实测 {:?})",
        log.borrow()
    );
    // 预览模式: 会话收尾 → refreshPreview materialize (preview=true 恒可见) → present
    log.borrow_mut().clear();
    host.close_all();
    h.borrow_mut().reset_preview();
    host.refresh_preview().unwrap();
    host.render_tick().unwrap();
    assert!(
        log.borrow().iter().any(|l| l == "present:1800000"),
        "预览模式 present (实测 {:?})",
        log.borrow()
    );
}

/// 地面静止 (gear=100 + speedv≤10 + throttle=0) 不退场 — 退出条件的否定支
#[test]
fn dfs_feed_no_exit_on_ground_parked() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) =
        draw_frame_simpl_spec(std::path::Path::new("../../../fonts"), &feed_fm()).unwrap();
    host.register(spec);
    host.open_all().unwrap();
    h.borrow_mut().init(None);
    let mut feed = DrawFrameSimplFeed::new();
    let parked = || {
        Some(DfsFlight {
            gear: 100,
            speedv: 0.0,
            throttle: 0,
        })
    };
    for t in [1_000i64, 5_000, 12_000, 30_000] {
        feed.pump(&mut host, "thrustdFS", &h, t, 0, parked());
    }
    assert!(host.is_active("thrustdFS"), "停场不退场");
    // 滑跑起飞 (speedv>10 且 throttle>0) 才命中
    feed.pump(
        &mut host,
        "thrustdFS",
        &h,
        31_000,
        0,
        Some(DfsFlight {
            gear: 100,
            speedv: 30.0,
            throttle: 90,
        }),
    );
    feed.pump(
        &mut host,
        "thrustdFS",
        &h,
        42_000,
        0,
        Some(DfsFlight {
            gear: 100,
            speedv: 30.0,
            throttle: 90,
        }),
    );
    assert!(!host.is_active("thrustdFS"), "起飞 (热键=0) 后退场");
}
