//! gauge_crosshair: CrosshairGauge C 类语义复刻 (src/ui/component/CrosshairGauge.java)
//!
//! 准星 = 同几何双层矢量十字 (圆 + 四向外伸刻线):
//! 影层 (strokeWidth+1, 半透明黑) → 前景层 (strokeWidth, 金黄), 两层均为
//! BasicStroke(w, CAP_ROUND, JOIN_ROUND) (Java:93-94), 对应 render2d 的
//! draw_line/stroke_circle (Round 线型族)。
//!
//! | 几何量 | Java 出处 | 语义 |
//! |---|---|---|
//! | strokeWidth | :91 | Math.max(width/30, 2), int 除法 |
//! | halfW | :98 | width/2 (int 截断 → 圆直径 2·halfW, 奇 width 比外框小 1px) |
//! | quarterW | :99 | width/4 (刻线内端离圆心距离) |
//! | lineLength | :88,100 | l=4 → width·4/4 = width (刻线外端伸出圆外至整宽) |
//! | 中心 | :69-70 | (x+width/2, y+width/2), int 截断 |
//!
//! 刻线外端 = cx±lineLength = cx±width, 远超圆半径 halfW — 线臂伸出组件外框
//! 半宽 (Java 原样, 画布裁剪负责截断)。
//!
//! PORT: Java useTexture 纹理路径不迁移 — Rust 只做软件矢量路径:
//! - getPreferredSize 的 useTexture 分支 (:40-42 返回 crossWidthVario×2);
//! - draw 的 drawImage 分支 (:68, :72-83);
//! - setTextureStyle (:51-55)。
//! 纹理准星整链 (MiniHUDOverlay.java:604-607 useTextureCrosshair 配置 +
//! MinimalHUDContext.java:161-178 crosshairImageScaled 双线性缩放加载) 归
//! 配置裁剪: Rust 侧设置面板不提供该选项, 软件路径即唯一视觉语义。
//! 可见性 (AbstractHUDComponent.visible / MiniHUDOverlay.java:323-324
//! setVisible(isDisplayCrosshair)) 由组装层布局引擎持有, 组件本体不存该态
//! (与 gauge_attitude/gauges_bars 既有口径一致)。

use crate::render2d::PixCanvas;

/// CrosshairGauge.java:27 shadowColor = new Color(0, 0, 0, 75) (RGBA 直通)
pub const CROSSHAIR_SHADOW: [u8; 4] = [0, 0, 0, 75];
/// CrosshairGauge.java:28 foregroundColor = new Color(255, 215, 8, 255) 金黄
pub const CROSSHAIR_FOREGROUND: [u8; 4] = [255, 215, 8, 255];

/// Length multiplier for lines (CrosshairGauge.java:88)
const LINE_MULTIPLIER: i32 = 4;

/// 瞄准准星 (软件矢量路径)。
/// Java 的 BasicStroke 缓存 (:16-18, :90-96) 在 Rust 侧无对应物 —
/// stroke 是 draw_line/stroke_circle 的值参数, 每次调用即"重建"。
pub struct CrosshairGauge {
    width: i32,
    shadow_color: [u8; 4],
    foreground_color: [u8; 4],
}

impl CrosshairGauge {
    /// Java:30-31 构造 (width=0, 颜色字段默认 :27-28)
    pub fn new() -> Self {
        CrosshairGauge {
            width: 0,
            shadow_color: CROSSHAIR_SHADOW,
            foreground_color: CROSSHAIR_FOREGROUND,
        }
    }

    /// Java:33-36 getId
    pub fn id(&self) -> &'static str {
        "gauge.crosshair"
    }

    /// Java:38-44 getPreferredSize。
    /// PORT: :40-41 useTexture 分支返回 crossWidthVario×2 — 纹理路径不迁移, 恒软件口径
    pub fn preferred_size(&self) -> (i32, i32) {
        (self.width, self.width)
    }

    /// Java:46-49 setStyleContext (软件绘制路径, 同时置 useTexture=false)
    pub fn set_style_context(&mut self, width: i32) {
        self.width = width;
        // PORT: Java:48 useTexture=false — Rust 无纹理态可置
    }

    /// Java:60-63 setColors (MiniHUD 现不调用, 默认色即生产色, 保留供组装层覆写)
    pub fn set_colors(&mut self, shadow: [u8; 4], foreground: [u8; 4]) {
        self.shadow_color = shadow;
        self.foreground_color = foreground;
    }

    /// Java:91 strokeWidth = Math.max(width / 30, 2) — int 除法
    pub fn stroke_width(width: i32) -> i32 {
        std::cmp::max(width / 30, 2)
    }

    /// Java:65-113 draw (软件矢量路径)。
    /// aa 对齐 MiniHUDOverlay.paintComponent 的 graphAASetting
    /// (MiniHUDOverlay.java:244 / Application.java:102, 生产恒 ON;
    /// false 供非 AA 像素对拍路径)。
    pub fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, aa: bool) {
        // Convert Top-Left (x, y) to Center (Java:67-70, int 截断)
        // PORT: Java:68 drawW 的纹理分支不迁移, 恒 width
        let draw_w = self.width;
        let center_x = x + draw_w / 2;
        let center_y = y + draw_w / 2;

        let stroke_width = Self::stroke_width(self.width);

        // (crosshairScale≤200 溢出不可达, 防御性消除 debug panic; /4 两语言同向零截断)
        let half_width = self.width / 2;
        let quarter_width = self.width / 4;
        let line_length = self.width.wrapping_mul(LINE_MULTIPLIER) / 4;

        // Draw shadow layer (:102-106) → Draw foreground layer (:108-112):
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

/// Java:115-126 drawCrosshairShape: 圆 + 四向刻线 (先圆后线, CAP_ROUND 圆帽)
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
    // Circle (Java:117): drawOval(cx-halfW, cy-halfW, 2·halfW, 2·halfW) 的圆特例
    cv.stroke_circle(cx, cy, half_w, stroke_w, color, aa);
    // Horizontal lines (left and right of center) (Java:120-121)
    cv.draw_line(cx - line_len, cy, cx - quarter_w, cy, stroke_w, color, aa);
    cv.draw_line(cx + quarter_w, cy, cx + line_len, cy, stroke_w, color, aa);
    // Vertical lines (top and bottom of center) (Java:124-125)
    cv.draw_line(cx, cy - line_len, cx, cy - quarter_w, stroke_w, color, aa);
    cv.draw_line(cx, cy + quarter_w, cx, cy + line_len, stroke_w, color, aa);
}

#[cfg(test)]
mod tests;
