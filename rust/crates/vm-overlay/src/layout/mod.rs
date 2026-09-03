//! 布局域 (波10 分域): RenderContext 布局公式 (Java RenderContext/BOSStyleRenderer/
//! TextGauge 的整数/float 运算 1:1 移植) + 锚点 (anchor) + HUD 布局节点
//! (hud_layout_node) + MiniHUD 布局引擎 (minihud_layout) + DPI 常量 (ui_constants)。

pub mod anchor;
pub mod hud_layout_node;
pub mod minihud_layout;
pub mod ui_constants;

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

impl RenderCtx {
    /// 对应 RenderContext.create: 字号派生关系固化在此
    pub fn new(font_add: i32, column_num: i32, num_height: i32) -> Self {
        let font_size = 24 + font_add;
        RenderCtx {
            font_size,
            label_font_size: vm_core::base::format::java_round_f32(font_size as f32 / 2.0),
            unit_font_size: vm_core::base::format::java_round_f32(font_size as f32 / 2.0),
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
        let addnum = if visible_count % self.column_num == 0 {
            0
        } else {
            1
        };
        let rows = visible_count / self.column_num + addnum + 1;
        (self.num_height as f32 + rows as f32 * self.num_height as f32) as i32
    }

    /// 渲染起始偏移 (fontSize>>1, fontSize>>1), 对应 BOSStyleRenderer.render offset 初始化
    pub fn start_offset(&self) -> (i32, i32) {
        (self.font_size >> 1, self.font_size >> 1)
    }

    /// 列步进 Math.round(5f * fontSize)
    pub fn advance_x(&self) -> i32 {
        vm_core::base::format::java_round_f32(5f32 * self.font_size as f32)
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
mod tests;
