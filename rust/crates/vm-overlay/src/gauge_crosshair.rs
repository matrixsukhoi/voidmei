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

        // Java:98-100 全 int 算术 — width*l 按 §2.2 wrapping_mul 对齐 Java 静默回绕
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
mod tests {
    use super::*;

    /// 读预乘 RGBA 像素 (与 gauges_bars/render2d 测试同约定)
    fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
        let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
        [d[0], d[1], d[2], d[3]]
    }

    const RED: [u8; 4] = [255, 0, 0, 255]; // 影层测试色 (不透明 → 影专属像素可精确断言)
    const GREEN: [u8; 4] = [0, 255, 0, 255]; // 前景层测试色

    /// 标准被测件: width=40 @ (10,10) → 中心 (30,30), halfW=20, quarterW=10,
    /// lineLen=40, strokeWidth=2 (影 3/前景 2), 自定义不透明双色
    fn subject() -> (PixCanvas, CrosshairGauge) {
        let mut g = CrosshairGauge::new();
        g.set_style_context(40);
        g.set_colors(RED, GREEN);
        (PixCanvas::new(80, 80).unwrap(), g)
    }

    /// Java:91 strokeWidth 公式 (int 除法 + 下限 2)
    #[test]
    fn stroke_width_formula() {
        assert_eq!(CrosshairGauge::stroke_width(40), 2, "40/30=1 → max(1,2)=2");
        assert_eq!(CrosshairGauge::stroke_width(59), 2, "59/30=1 → 2");
        assert_eq!(CrosshairGauge::stroke_width(60), 2, "60/30=2 → 2");
        assert_eq!(CrosshairGauge::stroke_width(90), 3, "90/30=3");
        assert_eq!(CrosshairGauge::stroke_width(150), 5);
        assert_eq!(CrosshairGauge::stroke_width(600), 20);
        assert_eq!(CrosshairGauge::stroke_width(0), 2, "宽度 0 仍钳 2");
        assert_eq!(CrosshairGauge::stroke_width(-30), 2, "负宽 Java max(-1,2)=2");
    }

    /// preferred_size = width×width (:38-44 软件分支)
    #[test]
    fn preferred_size_software() {
        let mut g = CrosshairGauge::new();
        assert_eq!(g.preferred_size(), (0, 0), "构造默认 width=0");
        g.set_style_context(40);
        assert_eq!(g.preferred_size(), (40, 40));
        assert_eq!(g.id(), "gauge.crosshair");
    }

    /// 线臂几何: 行/列 30 上的臂本体为前景色; 中心与四向 quarter 间隙为空。
    /// 像素中心距几何线 ≤1 (前景 stroke 2) / ≤1.5 (影 stroke 3) 判覆盖。
    #[test]
    fn crosshair_arms_and_center_gap() {
        let (mut c, mut g) = subject();
        g.draw(&mut c, 10, 10, false);
        // 中心: 四臂的 quarterW=10 间隙内, 距圆心 0.7 — 无任何图元
        assert_eq!(px(&c, 30, 30), [0, 0, 0, 0], "中心间隙为空");
        // 臂本体 (左/右/上/下, 均在 lineLen 覆盖内)
        assert_eq!(px(&c, 5, 30), GREEN, "左臂 (行30, x∈[-10,20])");
        assert_eq!(px(&c, 55, 30), GREEN, "右臂 (行30, x∈[40,70])");
        assert_eq!(px(&c, 30, 5), GREEN, "上臂 (列30, y∈[-10,20])");
        assert_eq!(px(&c, 30, 65), GREEN, "下臂 (列30, y∈[40,70])");
        // 水平/垂直两向的间隙 (quarterW=10 → [20,40] 空带, 圆帽覆盖不到)
        assert_eq!(px(&c, 25, 30), [0, 0, 0, 0], "水平间隙");
        assert_eq!(px(&c, 30, 25), [0, 0, 0, 0], "垂直间隙");
        assert_eq!(px(&c, 35, 35), [0, 0, 0, 0], "非臂非圆区");
    }

    /// 圆环几何: 半径 20 双层环 — 前景带 [19,21] / 影带 [18.5,21.5]。
    /// 45° 方向 (44,44) 距心 20.51 = 前景; (45,44) 距心 21.23 = 仅影
    /// (前景 3px 宽不足以覆盖, 1px 影轮廓透出); (45,45) 距心 21.92 = 环外空。
    #[test]
    fn crosshair_circle_two_layers() {
        let (mut c, mut g) = subject();
        g.draw(&mut c, 10, 10, false);
        assert_eq!(px(&c, 44, 44), GREEN, "环带前景 (径向 20.51)");
        assert_eq!(px(&c, 45, 44), RED, "影层轮廓 (径向 21.23, 前景外/影内)");
        assert_eq!(px(&c, 45, 45), [0, 0, 0, 0], "环外 (径向 21.92 > 21.5)");
        assert_eq!(px(&c, 9, 30), GREEN, "9 点方向环带 (径向 20.51)");
    }

    /// CAP_ROUND 圆帽: 端点沿方向外伸 stroke/2 — 右臂起点 (40,30) 的圆帽伸入
    /// quarter 间隙 (39,30); 左臂终点 (20,30) 同理; 再外 1px (38/21) 为空。
    #[test]
    fn crosshair_round_caps_poke_into_gap() {
        let (mut c, mut g) = subject();
        g.draw(&mut c, 10, 10, false);
        assert_eq!(px(&c, 39, 30), GREEN, "右臂起点圆帽伸入间隙 (距端点 0.71)");
        assert_eq!(px(&c, 38, 30), [0, 0, 0, 0], "圆帽外 (距端点 1.58 > 1.5)");
        assert_eq!(px(&c, 20, 30), GREEN, "左臂终点圆帽 (距端点 0.71)");
        assert_eq!(px(&c, 21, 30), [0, 0, 0, 0], "圆帽外 (距端点 1.58 > 1.5)");
    }

    /// 奇数 width 的 int 截断 (:98-100): width=41 → halfW=20 (圆同 40 口径),
    /// lineLen=41·4/4=41 → 右臂外端 cx+41=71 (width=40 时为 70)。
    #[test]
    fn crosshair_odd_width_int_truncation() {
        let mut g = CrosshairGauge::new();
        g.set_style_context(41);
        g.set_colors(RED, GREEN);
        let mut c = PixCanvas::new(80, 80).unwrap();
        g.draw(&mut c, 10, 10, false);
        assert_eq!(g.preferred_size(), (41, 41));
        // 中心 x+41/2 = 30 同 width=40; 圆 halfW=20 → (45,44) 仍为影轮廓
        assert_eq!(px(&c, 45, 44), RED, "圆半径仍取 41/2=20");
        assert_eq!(px(&c, 71, 30), GREEN, "右臂外端 cx+41=71 圆帽");
        assert_eq!(px(&c, 72, 30), [0, 0, 0, 0], "外端外 (距端点 1.58)");
    }

    /// 默认色 (:27-28): 前景不透明金黄盖影 → 纯 [255,215,8,255];
    /// 影专属像素 = 预乘 [0,0,0,75] (黑色影预乘仅 alpha 通道)。
    #[test]
    fn crosshair_default_colors() {
        let mut g = CrosshairGauge::new();
        g.set_style_context(40);
        let mut c = PixCanvas::new(80, 80).unwrap();
        g.draw(&mut c, 10, 10, false);
        assert_eq!(px(&c, 5, 30), CROSSHAIR_FOREGROUND, "臂 = 前景金黄");
        assert_eq!(px(&c, 45, 44), [0, 0, 0, 75], "影轮廓 = Color(0,0,0,75)");
    }

    /// 退化 width=0: halfW=0 → stroke_circle r≤0 不绘制; 四臂退化为零长线。
    /// 仅守护不 panic (Java 零长 CAP_ROUND 线画点 vs tiny-skia 行为未钉,
    /// 生产中 0 尺寸组件不进布局, 不构成保真对象)。
    #[test]
    fn crosshair_zero_width_no_panic() {
        let mut g = CrosshairGauge::new();
        let mut c = PixCanvas::new(16, 16).unwrap();
        g.draw(&mut c, 0, 0, false);
    }

    /// aa=true (生产 graphAASetting 恒 ON) 冒烟: 几何仍在, 像素非空
    #[test]
    fn crosshair_aa_smoke() {
        let (mut c, mut g) = subject();
        g.draw(&mut c, 10, 10, true);
        assert!(
            c.pixmap().data().iter().any(|&b| b != 0),
            "AA 开启时准星有输出"
        );
        // 臂中线深核心仍为纯前景 (覆盖率 1 处 SrcOver 不透明源 = 本色)
        let p = px(&c, 55, 30);
        assert_eq!((p[0], p[1], p[2]), (0, 255, 0), "AA 臂核心仍纯绿");
    }
}
