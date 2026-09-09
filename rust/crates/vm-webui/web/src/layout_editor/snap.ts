/**
 * 拖动吸附 (纯函数, 画布 px 域): 网格 round + 对齐参考线。
 * 消费方 Canvas 拖动中实时调用 — 松手 round 的旧实现无视觉反馈。
 */
export interface Rect {
  x: number
  y: number
  w: number
  h: number
}

export interface SnapGuide {
  /** 参考线轴向: x = 竖线 (左右对齐), y = 横线 (上下对齐) */
  axis: 'x' | 'y'
  /** 画布 px 坐标 */
  at: number
}

export interface SnapOptions {
  /** 网格步长 (画布 px; 0 = 禁用网格) */
  gridPx: number
  /** 对齐吸附阈值 (画布 px; 屏幕阈值 / zoom 换算后传入) */
  thresholdPx: number
  /** 内容包围盒 (画布系; 四边参与对齐) */
  contentRect: Rect | null
  /** 逻辑画布 (中线参与对齐) */
  canvasW: number
  canvasH: number
}

export interface SnapResult {
  /** 修正后的位移 (画布 px; 输入 rawDelta 的吸附版本) */
  dx: number
  dy: number
  /** 命中的参考线 (渲染层画粉色线) */
  guides: SnapGuide[]
}

/** 矩形在某轴的三基准 (L/C/R 或 T/M/B) */
const marks = (r: Rect, axis: 'x' | 'y'): number[] =>
  axis === 'x' ? [r.x, r.x + r.w / 2, r.x + r.w] : [r.y, r.y + r.h / 2, r.y + r.h]

const near = (a: number, b: number, tol: number) => Math.abs(a - b) <= tol

/**
 * 对移动集计算吸附位移:
 * 1. 网格: 移动后位置 round 到 gridPx (拖动中实时预览);
 * 2. 对齐线: 移动矩形三基准 × 静止矩形三基准 + 包围盒四边 + 画布中线,
 *    命中阈值 → 吸附并产出参考线。
 * 二者合流: 对齐命中的分量覆盖网格分量 (对齐优先, 未命中轴落回网格)。
 */
export function computeSnap(
  moving: Rect[],
  statics: Rect[],
  rawDelta: { dx: number; dy: number },
  opts: SnapOptions,
): SnapResult {
  const { gridPx, thresholdPx, contentRect, canvasW, canvasH } = opts
  let { dx, dy } = rawDelta
  const guides: SnapGuide[] = []

  // ① 网格分量 (移动集整体按首个矩形原点对齐; 拖动实时预览)
  if (gridPx > 0 && moving.length > 0) {
    const gx = Math.round((moving[0].x + dx) / gridPx) * gridPx - moving[0].x
    const gy = Math.round((moving[0].y + dy) / gridPx) * gridPx - moving[0].y
    if (Math.abs(gx - dx) <= thresholdPx) dx = gx
    if (Math.abs(gy - dy) <= thresholdPx) dy = gy
  }

  if (moving.length === 0 || thresholdPx <= 0) return { dx, dy, guides }

  // ② 对齐线: 汇总静止基准 (静止组件 + 包围盒 + 画布中线)
  const xTargets: number[] = []
  const yTargets: number[] = []
  for (const s of statics) {
    xTargets.push(...marks(s, 'x'))
    yTargets.push(...marks(s, 'y'))
  }
  if (contentRect) {
    xTargets.push(contentRect.x, contentRect.x + contentRect.w, contentRect.x + contentRect.w / 2)
    yTargets.push(contentRect.y, contentRect.y + contentRect.h, contentRect.y + contentRect.h / 2)
  }
  xTargets.push(canvasW / 2)
  yTargets.push(canvasH / 2)

  // 每轴独立: 找移动基准与目标的最小差, 命中即吸附
  for (const axis of ['x', 'y'] as const) {
    let best: { adjust: number; at: number } | null = null
    for (const m of moving) {
      for (const mark of marks(m, axis)) {
        const cur = mark + (axis === 'x' ? dx : dy)
        for (const t of axis === 'x' ? xTargets : yTargets) {
          const diff = t - cur
          if (!near(cur, t, thresholdPx)) continue
          if (!best || Math.abs(diff) < Math.abs(best.adjust)) {
            best = { adjust: diff, at: t }
          }
        }
      }
    }
    if (best) {
      if (axis === 'x') dx += best.adjust
      else dy += best.adjust
      guides.push({ axis, at: best.at })
    }
  }
  return { dx, dy, guides }
}
