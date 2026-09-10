//! gauge_crosshair: CrosshairGauge C 类语义复刻
//!
//! 准星 = 同几何双层矢量十字 (圆 + 四向外伸刻线):
//! 影层 (strokeWidth+1, 半透明黑) → 前景层 (strokeWidth, 金黄), 两层均为
//! BasicStroke(w, CAP_ROUND, JOIN_ROUND), 对应 render2d 的
//! draw_line/stroke_circle (Round 线型族)。
//!
//! 几何量 (int 算术):
//! - strokeWidth = Math.max(width/30, 2), int 除法
//! - halfW = width/2 (int 截断 → 圆直径 2·halfW, 奇 width 比外框小 1px)
//! - quarterW = width/4 (刻线内端离圆心距离)
//! - lineLength: 倍率 l=4 → width·4/4 = width (刻线外端伸出圆外至整宽)
//! - 中心 = (x+width/2, y+width/2), int 截断
//!
//! 刻线外端 = cx±lineLength = cx±width, 远超圆半径 halfW — 线臂伸出组件外框
//! 半宽 (Java 原样, 画布裁剪负责截断)。
//!
//! Java useTexture 纹理路径不迁移 — Rust 只做软件矢量路径:
//! - getPreferredSize 的 useTexture 分支 (返回 crossWidthVario×2);
//! - draw 的 drawImage 分支;
//! - setTextureStyle。
//! 纹理准星整链 (useTextureCrosshair 配置 + crosshairImageScaled
//! 双线性缩放加载) 归配置裁剪: Rust 侧设置面板不提供该选项,
//! 软件路径即唯一视觉语义。
//! 可见性 (AbstractHUDComponent.visible / setVisible(isDisplayCrosshair))
//! 由组装层布局引擎持有, 组件本体不存该态
//! (与 gauge_attitude/gauges_bars 既有口径一致)。

use crate::render::canvas::PixCanvas;

/// shadowColor = new Color(0, 0, 0, 75) (RGBA 直通)
pub const CROSSHAIR_SHADOW: [u8; 4] = [0, 0, 0, 75];
/// foregroundColor = new Color(255, 215, 8, 255) 金黄
pub const CROSSHAIR_FOREGROUND: [u8; 4] = [255, 215, 8, 255];

/// 刻线长度倍率
const LINE_MULTIPLIER: i32 = 4;

/// 瞄准准星 (软件矢量路径)。
/// Java 的 BasicStroke 缓存在 Rust 侧无对应物 —
/// stroke 是 draw_line/stroke_circle 的值参数, 每次调用即"重建"。
pub struct CrosshairGauge {
    width: i32,
    shadow_color: [u8; 4],
    foreground_color: [u8; 4],
}

impl CrosshairGauge {
    /// 构造 (width=0, 颜色取默认常量)
    pub fn new() -> Self {
        CrosshairGauge {
            width: 0,
            shadow_color: CROSSHAIR_SHADOW,
            foreground_color: CROSSHAIR_FOREGROUND,
        }
    }

    /// 组件 id
    pub fn id(&self) -> &'static str {
        "gauge.crosshair"
    }

    /// 首选尺寸。
    /// useTexture 分支返回 crossWidthVario×2 — 纹理路径不迁移, 恒软件口径
    pub fn preferred_size(&self) -> (i32, i32) {
        (self.width, self.width)
    }

    /// setStyleContext (软件绘制路径)
    pub fn set_style_context(&mut self, width: i32) {
        self.width = width;
        // Java 置 useTexture=false — Rust 无纹理态可置
    }

    /// setColors (MiniHUD 现不调用, 默认色即生产色, 保留供组装层覆写)
    pub fn set_colors(&mut self, shadow: [u8; 4], foreground: [u8; 4]) {
        self.shadow_color = shadow;
        self.foreground_color = foreground;
    }

    /// strokeWidth = Math.max(width / 30, 2) — int 除法
    pub fn stroke_width(width: i32) -> i32 {
        std::cmp::max(width / 30, 2)
    }

    /// draw (软件矢量路径)。
    /// aa 对齐 MiniHUDOverlay.paintComponent 的 graphAASetting
    /// (生产恒 ON; false 供非 AA 像素对拍路径)。
    pub fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, aa: bool) {
        // 左上角 (x, y) 换中心 (int 截断)
        // Java drawW 的纹理分支不迁移, 恒 width
        let draw_w = self.width;
        let center_x = x + draw_w / 2;
        let center_y = y + draw_w / 2;

        let stroke_width = Self::stroke_width(self.width);

        // (crosshairScale≤200 溢出不可达, 防御性消除 debug panic; /4 两语言同向零截断)
        let half_width = self.width / 2;
        let quarter_width = self.width / 4;
        let line_length = self.width.wrapping_mul(LINE_MULTIPLIER) / 4;

        // 影层 → 前景层:
        // 同几何两遍, 影层粗 1px 从前景边缘透出形成轮廓
        draw_crosshair_shape(
            cv,
            center_x,
            center_y,
            half_width,
            quarter_width,
            line_length,
            (stroke_width + 1) as f32,
            self.shadow_color,
            aa,
        );
        draw_crosshair_shape(
            cv,
            center_x,
            center_y,
            half_width,
            quarter_width,
            line_length,
            stroke_width as f32,
            self.foreground_color,
            aa,
        );
    }
}

impl Default for CrosshairGauge {
    fn default() -> Self {
        Self::new()
    }
}

/// drawCrosshairShape: 圆 + 四向刻线 (先圆后线, CAP_ROUND 圆帽)
#[allow(clippy::too_many_arguments)] // 对齐 Java 私有方法 (g2d,cx,cy,halfW,quarterW,lineLen)+线型/aa
fn draw_crosshair_shape(
    cv: &mut PixCanvas,
    cx: i32,
    cy: i32,
    half_w: i32,
    quarter_w: i32,
    line_len: i32,
    stroke_w: f32,
    color: [u8; 4],
    aa: bool,
) {
    // 圆: drawOval(cx-halfW, cy-halfW, 2·halfW, 2·halfW) 的圆特例
    cv.stroke_circle(cx, cy, half_w, stroke_w, color, aa);
    // 水平刻线 (圆心左右)
    cv.draw_line(cx - line_len, cy, cx - quarter_w, cy, stroke_w, color, aa);
    cv.draw_line(cx + quarter_w, cy, cx + line_len, cy, stroke_w, color, aa);
    // 垂直刻线 (圆心上下)
    cv.draw_line(cx, cy - line_len, cx, cy - quarter_w, stroke_w, color, aa);
    cv.draw_line(cx, cy + quarter_w, cx, cy + line_len, stroke_w, color, aa);
}

#[cfg(test)]
mod tests;
