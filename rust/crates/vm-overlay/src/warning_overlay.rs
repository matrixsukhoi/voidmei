//! warning_overlay: WarningOverlay C 类语义复刻 (src/ui/component/WarningOverlay.java)
//! + 闪烁节奏宿主 (MiniHUDOverlay.java:47-86, 263-265 的 drawBlinkX 链)
//!
//! 警告 X = 覆盖整个 MiniHUD 面板 (0,0,ctx.width,ctx.height) 的对角双层 X:
//! 影层 BasicStroke(5) 偏移 ±2 → 前景层 BasicStroke(3) 偏移 ±1, 均
//! CAP_ROUND+JOIN_ROUND (WarningOverlay.java:43-44); 颜色取 Application 静态色
//! colorShadeShape / colorNum。
//!
//! PORT: Java WarningOverlay 本体无文本输出 (仅 X 图形) — "警告文本"语义由
//! HUD 行族 (row/ 组件的 isWarning 着色) 与语音链路 (VoiceWarning) 承担,
//! 不在本文件; X 由 Service.fatalWarn (VoiceWarning.java:556 置位) 经
//! EventPayload.fatalWarn 触发 (MiniHUDOverlay.java:458 blinkX = ...)。
//!
//! 闪烁节奏 (帧语义, MiniHUDOverlay.java:75-86): 每帧 repaint (与 service
//! 轮询同频 ~10Hz) 且 blinkX=true 时 — 先按当前 blinkActing 绘制
//! (true = off 相位整体跳过), 再 blinkCheckTicks+=1, 每 blinkTicks 帧翻转
//! blinkActing; blinkX=false 时完全不推进 (计数/相位冻结)。
//! blinkTicks = (1000/intervalMs)>>3 (Java:263, long 整除), 0 钳 1 (:264-265)。


use crate::global_colors::colors;
use crate::render2d::PixCanvas;

/// WarningOverlay.java:43 影层线宽 (常量 — Java 按 width 重建缓存但值与 width 无关,
/// :42-46 纯 GC 优化, 无视觉分支)
const OUTER_STROKE: f32 = 5.0;
/// WarningOverlay.java:44 前景层线宽
const INNER_STROKE: f32 = 3.0;

/// 高度/临界警告 X 覆盖层 (WarningOverlay.java:16)
pub struct WarningOverlay {}

impl WarningOverlay {
    /// Java:23-25 构造
    pub fn new() -> Self {
        WarningOverlay {}
    }

    /// Java:36-59 draw — 对角 X 双层。
    /// aa 对齐 MiniHUDOverlay.paintComponent 的 graphAASetting
    /// (MiniHUDOverlay.java:244, 生产恒 ON; false 供非 AA 像素对拍)。
    #[allow(clippy::too_many_arguments)] // 对齐 Java draw(g2d,x,y,width,height,isBlinkOff) + aa
    pub fn draw(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        is_blink_off: bool,
        aa: bool,
    ) {
        if is_blink_off {
            return;
        }

        // Draw shadow X (Java:48-52): colorShadeShape (Application.java:108),
        // 两端各内缩 2px
        cv.draw_line(x + 2, y + 2, x + width - 2, y + height - 2, OUTER_STROKE, colors().shade_shape, aa);
        cv.draw_line(x + width - 2, y + 2, x + 2, y + height - 2, OUTER_STROKE, colors().shade_shape, aa);

        // Draw foreground X (Java:54-58): colorNum (Application.java:111),
        // 两端各内缩 1px
        cv.draw_line(x + 1, y + 1, x + width - 1, y + height - 1, INNER_STROKE, colors().num, aa);
        cv.draw_line(x + width - 1, y + 1, x + 1, y + height - 1, INNER_STROKE, colors().num, aa);
    }
}

impl Default for WarningOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// 闪烁节奏宿主 (MiniHUDOverlay.java:47-54, 72-86, 263-265 的宿主侧字段与方法,
/// warningOverlay 实例按 :528 组合持有; ctx!=null 门卫 :77 是 init 顺序保护,
/// Rust 侧画布经参数恒在, 不复刻)。
///
/// 驱动契约: 每帧调用 [`draw_blink_x`](Self::draw_blink_x), 帧率 = service
/// 轮询频率 (~10Hz, MiniHUDOverlay.java:267 refreshInterval 同源)。
pub struct WarningBlinkHost {
    warning: WarningOverlay,
    /// Java:72 blinkActing — true = off 相位 (跳过绘制); 初始 false = 首帧即亮
    blink_acting: bool,
    /// Java:48 blinkCheckTicks — 帧计数器
    blink_check_ticks: i32,
    /// Java:47/263 blinkTicks — 翻转周期 (帧)
    blink_ticks: i32,
    /// Java:49/458 blinkX = payload.fatalWarn — 警告使能
    blink_x: bool,
}

impl WarningBlinkHost {
    /// Java:263-265 — serviceLoopIntervalMs 为 long: 1000/interval 是 long
    /// 整除, >>3 后 (int) 截断高位; 结果 0 钳 1。
    /// interval=0 在 Java 抛 ArithmeticException (init 崩溃), Rust i64 除零
    /// panic 同为致命语义 (PORTING §1 非受检异常映射), 见 should_panic 测试。
    pub fn new(service_loop_interval_ms: i64) -> Self {
        let mut blink_ticks = ((1000i64 / service_loop_interval_ms) >> 3) as i32;
        if blink_ticks == 0 {
            blink_ticks = 1;
        }
        WarningBlinkHost {
            warning: WarningOverlay::new(),
            blink_x: false,
            blink_acting: false,
            blink_check_ticks: 0,
            blink_ticks,
        }
    }

    /// Java:263-265 周期值 (暴露供节奏单测/组装层日志)
    pub fn blink_ticks(&self) -> i32 {
        self.blink_ticks
    }

    /// Java:458 blinkX = event.getPayload().fatalWarn
    pub fn set_blink_x(&mut self, v: bool) {
        self.blink_x = v;
    }

    pub fn is_blink_acting(&self) -> bool {
        self.blink_acting
    }

    /// Java:75-86 drawBlinkX — 在 (0,0,ctx.width,ctx.height) 全面板画 X
    /// (绘制顺序: modernLayout.render 之后, Java:250-255, X 压在 HUD 内容之上;
    /// 注意 crosshair 显示时窗口宽为 ctx.width·2 (:144) 但 X 仍只盖 ctx.width —
    /// Java 原样, 保真不改)。
    pub fn draw_blink_x(&mut self, cv: &mut PixCanvas, width: i32, height: i32, aa: bool) {
        if self.blink_x {
            self.warning.draw(cv, 0, 0, width, height, self.blink_acting, aa);
            // PORT: Java 静默回绕 (§2.2) — ~10Hz 下 i32 计 ~6.8 年回绕,
            // wrapping_add + % (两语言同为向零取余) 精确对齐 Java 溢出后行为
            self.blink_check_ticks = self.blink_check_ticks.wrapping_add(1);
            if self.blink_check_ticks % self.blink_ticks == 0 {
                self.blink_acting = !self.blink_acting;
            }
        }
    }
}

#[cfg(test)]
mod tests;
