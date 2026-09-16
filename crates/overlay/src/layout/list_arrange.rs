//! 列表容器排列策略 (试驾场设计: "排列是选择不是搭建")。
//!
//! 容器节点 ([`crate::layout::hud_layout_node::HUDLayoutNode::arrange`]) 持有
//! 本策略 → 布局引擎求解时接管子树: 子项 pos/anchor 被忽略, 顺序语义
//! (children 序 = 文档 components 序) + 按模式排列。
//! 隐藏子项 (preferred 高度 0) 自动塌缩补位, 整块随内容收缩 —
//! 替代旧竖向锚链的被动补位 (链式 = 退化单列容器)。

use crate::layout::hud_layout_node::Dimension;

/// 排列模式 (用户语言: 单列 / 两列 / 自动换行 / 网格)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListMode {
    /// 单列竖向堆叠 (缺省; flightinfo 锚链的容器化形态)
    Column,
    /// 固定列数: 连续分段, 先填满左列再开下一列 (段长 = ceil(n/列数))
    Columns(usize),
    /// 自动换行: 列高预算 (line_height 单位) 耗尽即开新列; 列首项必收 (防超高单项空转)
    Wrap { max_height_units: f64 },
    /// 网格: 行优先填充 (先横后纵), 共享行高/列宽 (表语义)。
    /// cols 决定换行; rows 仅为文档/UI 元数据 (空行不占高, 超容项续行防丢件)
    Grid { rows: usize, cols: usize },
}

/// 列表容器排列策略 (块属性; 引擎求解期读取)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListArrange {
    pub mode: ListMode,
    /// 间距 (line_height 单位, DPI 不变口径; 0 = 紧贴)
    pub gap: f64,
}

/// 排列产物: 各子项相对容器原点的偏移 + 内容尺寸
pub struct ArrangedList {
    pub offsets: Vec<(i32, i32)>,
    pub size: Dimension,
}

/// 单列堆叠内核 (Column / Columns 各列共用):
/// 逐项下移, 隐藏项 (高 0) 不占位不计间距; 返回 (偏移, 列宽, 内容高)
fn stack_column(sizes: &[Dimension], gap: i32) -> (Vec<(i32, i32)>, i32, i32) {
    let mut offsets = Vec::with_capacity(sizes.len());
    let mut y = 0;
    let mut w = 0;
    for s in sizes {
        offsets.push((0, y));
        if s.height > 0 {
            y += s.height + gap;
        }
        w = w.max(s.width);
    }
    // 尾距修正: 存在可见项时末尾多累计了一个 gap
    if y > 0 {
        y -= gap;
    }
    (offsets, w, y.max(0))
}

/// 换列结算: 汇总列宽/列高, 游标 x 前进 (列宽 + 间距)
fn close_column(x: &mut i32, col_w: &mut i32, col_h: &mut i32, sum_w: &mut i32, n_cols: &mut usize, max_h: &mut i32, gap: i32) {
    if *col_w > 0 || *col_h > 0 {
        *sum_w += *col_w;
        *n_cols += 1;
        *max_h = (*max_h).max(*col_h);
        *x += *col_w + gap;
    }
    *col_w = 0;
    *col_h = 0;
}

impl ListArrange {
    /// 纯排列: 度量尺寸表 → 偏移表 + 内容尺寸。
    /// gap 像素换算 = gap × line_height (与锚点 unit 偏移同款向零截断)
    pub fn arrange(&self, sizes: &[Dimension], line_height: f64) -> ArrangedList {
        let gap = (self.gap * line_height) as i32;
        match self.mode {
            ListMode::Column => {
                let (offsets, w, h) = stack_column(sizes, gap);
                ArrangedList {
                    offsets,
                    size: Dimension::new(w, h),
                }
            }
            ListMode::Columns(n) => {
                let n = n.max(1);
                // 连续分段: 列 i = [i*per, (i+1)*per); 空尾列跳过
                let per = sizes.len().div_ceil(n);
                let mut offsets = vec![(0, 0); sizes.len()];
                let mut x = 0;
                let mut col_ws: Vec<i32> = Vec::new();
                let mut max_h = 0;
                for (ci, chunk) in sizes.chunks(per).enumerate().take(n) {
                    if chunk.is_empty() {
                        break;
                    }
                    let (col_off, w, h) = stack_column(chunk, gap);
                    for (k, (dx, dy)) in col_off.into_iter().enumerate() {
                        offsets[ci * per + k] = (x + dx, dy);
                    }
                    max_h = max_h.max(h);
                    col_ws.push(w);
                    x += w + gap;
                }
                let w = col_ws.iter().sum::<i32>() + gap * col_ws.len().saturating_sub(1) as i32;
                ArrangedList {
                    offsets,
                    size: Dimension::new(w, max_h),
                }
            }
            ListMode::Wrap { max_height_units } => {
                let budget = (max_height_units.max(0.0) * line_height) as i32;
                if budget <= 0 {
                    // 无预算 = 单列 (宽容退化)
                    return self.degenerate_column(sizes, line_height);
                }
                // 流式填列: 超预算且非列首 (y>0) → 结算开新列; 列首项无条件收
                let mut offsets = Vec::with_capacity(sizes.len());
                let mut x = 0;
                let mut y = 0; // 本列游标 (含累计间距)
                let mut col_w = 0;
                let mut col_h = 0; // 本列可见内容高 (不含间距)
                let mut sum_w = 0;
                let mut n_cols = 0;
                let mut max_h = 0;
                for s in sizes {
                    if y > 0 && y + s.height > budget {
                        close_column(&mut x, &mut col_w, &mut col_h, &mut sum_w, &mut n_cols, &mut max_h, gap);
                        y = 0;
                    }
                    offsets.push((x, y));
                    if s.height > 0 {
                        y += s.height + gap;
                        col_h += s.height;
                    }
                    col_w = col_w.max(s.width);
                }
                close_column(&mut x, &mut col_w, &mut col_h, &mut sum_w, &mut n_cols, &mut max_h, gap);
                let w = sum_w + gap * n_cols.saturating_sub(1) as i32;
                ArrangedList {
                    offsets,
                    size: Dimension::new(w, max_h),
                }
            }
            ListMode::Grid { cols, .. } => {
                let cols = cols.max(1);
                let rows = sizes.len().div_ceil(cols);
                // 共享列宽/行高 (表语义: 同列等宽, 同行等高)
                let mut col_w = vec![0; cols];
                let mut row_h = vec![0; rows];
                for (i, s) in sizes.iter().enumerate() {
                    let (r, c) = (i / cols, i % cols);
                    col_w[c] = col_w[c].max(s.width);
                    row_h[r] = row_h[r].max(s.height);
                }
                // 前缀坐标 (含行/列间距)
                let mut col_x = vec![0; cols];
                let mut acc = 0;
                for c in 0..cols {
                    col_x[c] = acc;
                    acc += col_w[c] + gap;
                }
                let mut row_y = vec![0; rows];
                let mut acc = 0;
                for r in 0..rows {
                    row_y[r] = acc;
                    acc += row_h[r] + gap;
                }
                let offsets = sizes
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let (r, c) = (i / cols, i % cols);
                        (col_x[c], row_y[r])
                    })
                    .collect();
                let w = col_w.iter().sum::<i32>() + gap * cols.saturating_sub(1) as i32;
                let h = row_h.iter().sum::<i32>() + gap * rows.saturating_sub(1) as i32;
                ArrangedList {
                    offsets,
                    size: Dimension::new(w, h),
                }
            }
        }
    }

    /// Wrap 无预算退化 = 单列
    fn degenerate_column(&self, sizes: &[Dimension], line_height: f64) -> ArrangedList {
        ListArrange {
            mode: ListMode::Column,
            gap: self.gap,
        }
        .arrange(sizes, line_height)
    }
}

/// props → 排列策略 (core.layout.list 专属解析; 未知值宽容退化 Column)。
/// props 面: arrange = column|columns|wrap|grid, columns/wrapHeight/gridRows/gridCols/gap
pub fn parse_list_props(props: &serde_json::Value) -> ListArrange {
    let num = |k: &str, d: f64| props.get(k).and_then(|v| v.as_f64()).unwrap_or(d);
    let mode = match props.get("arrange").and_then(|v| v.as_str()) {
        Some("columns") => ListMode::Columns((num("columns", 2.0) as usize).clamp(1, 8)),
        Some("wrap") => ListMode::Wrap {
            max_height_units: num("wrapHeight", 12.0).max(0.0),
        },
        Some("grid") => ListMode::Grid {
            rows: (num("gridRows", 2.0) as usize).max(1),
            cols: (num("gridCols", 2.0) as usize).max(1),
        },
        _ => ListMode::Column,
    };
    ListArrange {
        mode,
        gap: num("gap", 0.0),
    }
}
