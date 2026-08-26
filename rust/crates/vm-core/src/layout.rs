//! RenderContext 布局公式 1:1 移植 (src/ui/renderer/RenderContext.java
//! + BOSStyleRenderer.java + TextGauge.java 的整数/float 运算)

// ui/layout/Anchor.java 的挂载 (P2 批二): layout 是 lib.rs 已预挂的挂载点,
// ui/layout 族文件以子模块形式挂入。
pub mod anchor;

pub use anchor::Anchor;

/// 渲染上下文 (对应 Java RenderContext, num_height 由 Java meta 校准值传入)
pub struct RenderCtx {
    /// 24 + font_add
    pub font_size: i32,
    /// Math.round(fontSize / 2.0)
    pub label_font_size: i32,
    /// Math.round(fontSize / 2.0)
    pub unit_font_size: i32,
    pub column_num: i32,
    /// Java Toolkit.getFontMetrics(numFont).getHeight() 的实测校准值
    pub num_height: i32,
}

/// Java Math.round(float) = floor(x + 0.5)
fn java_round_f(x: f32) -> i32 {
    (x + 0.5).floor() as i32
}

impl RenderCtx {
    /// 对应 RenderContext.create: 字号派生关系固化在此
    pub fn new(font_add: i32, column_num: i32, num_height: i32) -> Self {
        let font_size = 24 + font_add;
        RenderCtx {
            font_size,
            label_font_size: java_round_f(font_size as f32 / 2.0),
            unit_font_size: java_round_f(font_size as f32 / 2.0),
            column_num,
            num_height,
        }
    }

    /// 对应 getTotalWidth(): (fontSize>>1) + (int)((columnNum+0.5)*5f*fontSize)
    pub fn total_width(&self) -> i32 {
        (self.font_size >> 1)
            + ((self.column_num as f32 + 0.5) * 5f32 * self.font_size as f32) as i32
    }

    /// 对应 getTotalHeight(visibleCount): numHeight + (vis/col + addnum + 1)*numHeight (整数除法)
    pub fn total_height(&self, visible_count: i32) -> i32 {
        let addnum = if visible_count % self.column_num == 0 { 0 } else { 1 };
        let rows = visible_count / self.column_num + addnum + 1;
        (self.num_height as f32 + rows as f32 * self.num_height as f32) as i32
    }

    /// 渲染起始偏移 (fontSize>>1, fontSize>>1), 对应 BOSStyleRenderer.render offset 初始化
    pub fn start_offset(&self) -> (i32, i32) {
        (self.font_size >> 1, self.font_size >> 1)
    }

    /// 列步进 Math.round(5f * fontSize)
    pub fn advance_x(&self) -> i32 {
        java_round_f(5f32 * self.font_size as f32)
    }

    /// 行步进 Math.round(1 * numHeight) = numHeight
    pub fn advance_y(&self) -> i32 {
        self.num_height
    }

    /// TextGauge 标签区宽 (13 * fontNum.getSize()) >> 2
    pub fn lwidth(&self) -> i32 {
        (13 * self.font_size) >> 2
    }

    /// 数值右对齐内边距 Math.max(4, fontNum.getSize() / 4)
    pub fn num_padding(&self) -> i32 {
        std::cmp::max(4, self.font_size / 4)
    }

    /// 数值基线 y: Java (y + y + labelSize + unitSize) >> 1
    /// (drawString 的 y 坐标是基线)
    pub fn value_baseline(&self, y: i32) -> i32 {
        (y + y + self.label_font_size + self.unit_font_size) >> 1
    }

    /// 单位基线 y: y + fontLabel.getSize()
    pub fn unit_baseline(&self, y: i32) -> i32 {
        y + self.label_font_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认参数: fontAdd=0, columnNum=1 (ui_layout.cfg 默认)
    #[test]
    fn default_layout() {
        let ctx = RenderCtx::new(0, 1, 33);
        assert_eq!(ctx.font_size, 24);
        assert_eq!(ctx.label_font_size, 12);
        assert_eq!(ctx.unit_font_size, 12);
        assert_eq!(ctx.total_width(), 192); // 12 + 180
        assert_eq!(ctx.total_height(16), 33 * 18); // 1 + 16 + 1 行
        assert_eq!(ctx.total_height(15), 33 * 17); // 15 行 → 1 + 15 + 1
        assert_eq!(ctx.start_offset(), (12, 12));
        assert_eq!(ctx.advance_x(), 120);
        assert_eq!(ctx.advance_y(), 33);
        assert_eq!(ctx.lwidth(), 78); // 13*24>>2
        assert_eq!(ctx.num_padding(), 6); // max(4, 6)
    }

    #[test]
    fn multi_column_layout() {
        // columnNum=3: 宽 = 12 + 3.5*120 = 432
        let ctx = RenderCtx::new(0, 3, 33);
        assert_eq!(ctx.total_width(), 432);
        // 16 字段 3 列 → ceil(16/3)=6 行 → 33*(1+6+1) = 264
        assert_eq!(ctx.total_height(16), 264);
        // 恰好整除: 6 字段 3 列 → 2 行 → 33*4
        assert_eq!(ctx.total_height(6), 33 * 4);
    }

    #[test]
    fn font_add_variants() {
        // fontAdd=4: fontSize=28, labelSize=round(14)=14
        let ctx = RenderCtx::new(4, 1, 39);
        assert_eq!(ctx.font_size, 28);
        assert_eq!(ctx.label_font_size, 14);
        assert_eq!(ctx.lwidth(), (13 * 28) >> 2); // 91
        assert_eq!(ctx.num_padding(), 7);
        // fontAdd=-6: fontSize=18, round(9)=9
        let ctx = RenderCtx::new(-6, 1, 26);
        assert_eq!(ctx.font_size, 18);
        assert_eq!(ctx.label_font_size, 9);
        assert_eq!(ctx.num_padding(), 4); // max(4, 18/4=4) = 4
    }

    #[test]
    fn value_baseline_position() {
        let ctx = RenderCtx::new(0, 1, 33);
        // y=12: (12+12+12+12)>>1 = 24
        assert_eq!(ctx.value_baseline(12), 24);
        assert_eq!(ctx.unit_baseline(12), 24); // 12+12
    }
}
