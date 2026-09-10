/**
 * 功率曲线图表纯布局函数 (PowerCurveWindow.java ChartPanel 的像素/布局面)。
 * 无 React/antd/tauri 依赖 — vitest 钉住: 坐标映射 / 标签防碰撞
 * (LabelPosition flipSide/offsetY 逻辑) / 单双模式展示分支。
 * 数据面 (曲线采样/拐点检测/显示域) 在 Rust W1, 此处只消费 DTO。
 */

import type { PowerCurve, PowerCurveData } from '../api'

/** Java PowerCurveWindow.java:37-41 图表常量 (像素域) */
export const CHART = {
  width: 1000,
  height: 650,
  margin: 80,
  maxAlt: 10000,
  altStep: 25,
} as const

/** Java :44-60 调色板 (Material Dark; FM0 绿系 / FM1 青系) */
export const PC_COLORS = {
  bg: '#1E1E23', // BG_COLOR (30,30,35)
  chartBg: '#282832', // CHART_BG (40,40,50)
  grid: '#3C3C46', // GRID_COLOR (60,60,70)
  axis: '#B4B4B4', // AXIS_COLOR (180,180,180)
  error: '#FFA000', // ERROR_COLOR (255,160,0)
  /** [FM0, FM1] 按曲线序取色 */
  curve: ['#2EFF71', '#00D4FF'],
  peak: ['#FFD700', '#FF80B4'],
  valley: ['#64C8FF', '#FF9966'],
  kink: ['#B482FF', '#82FFB4'],
} as const

/** 拐点 kind+曲线序 → 标注色 (Java 以 Color 字段区分三族) */
export function kindColor(kind: string, curveIndex: 0 | 1): string {
  const table =
    kind === 'peak' ? PC_COLORS.peak : kind === 'valley' ? PC_COLORS.valley : PC_COLORS.kink
  return table[curveIndex]
}

/** "#RRGGBB" + alpha(0-255) → rgba() (Java new Color(r,g,b,a) 对位) */
export function withAlpha(hex: string, a: number): string {
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  return `rgba(${r},${g},${b},${(a / 255).toFixed(3)})`
}

/**
 * Java String.format("%.0f") = HALF_UP (远离零半舍)。JS Math.round 对负数是
 * 向上取整, 手工复刻; 功率/高度域为正, 但保持通用。
 */
export function fmt0(v: number): number {
  return v >= 0 ? Math.floor(v + 0.5) : Math.ceil(v - 0.5)
}

/** Java `(int)` 截断 (向零) */
const trunc = (v: number): number => Math.trunc(v)

/** 功率 → 图 x (drawPowerCurve :821): MARGIN + (p - min)/range * chartW, (int) 截断 */
export function powerToX(power: number, minP: number, maxP: number, chartW: number): number {
  return CHART.margin + trunc(((power - minP) / (maxP - minP)) * chartW)
}

/** 高度 → 图 y (:822): MARGIN + chartH - alt/maxAlt * chartW, (int) 截断 */
export function altToY(altM: number, chartH: number): number {
  return CHART.margin + chartH - trunc((altM / CHART.maxAlt) * chartH)
}

/** 水平网格线 + Y 轴标签 (drawGrid/drawAxes :763-767/:788-793): 每 1000m 一条 */
export function gridHLines(chartH: number): { y: number; label: string }[] {
  const altSteps = CHART.maxAlt / 1000
  const out: { y: number; label: string }[] = []
  for (let i = 0; i <= altSteps; i++) {
    out.push({ y: altToY(i * 1000, chartH), label: `${i * 1000}m` })
  }
  return out
}

/** 垂直网格线 + X 轴标签 (:771-775/:797-803): 每 100hp 从 min 到 max */
export function gridVLines(
  minP: number,
  maxP: number,
  chartW: number,
): { x: number; label: string }[] {
  const minHp = trunc(minP / 100)
  const maxHp = trunc(maxP / 100)
  const range = maxP - minP
  const out: { x: number; label: string }[] = []
  for (let i = minHp; i <= maxHp; i++) {
    const hp = i * 100.0
    out.push({ x: CHART.margin + trunc(((hp - minP) / range) * chartW), label: `${i * 100}` })
  }
  return out
}

/**
 * 曲线折线点 (drawPowerCurve :817-829): 索引 i ↔ 高度 i×altStep, 截到
 * maxAltIdx (10000/25=400) 与数组长度。
 */
export function curvePoints(
  curve: readonly number[],
  minP: number,
  maxP: number,
  chartW: number,
  chartH: number,
): [number, number][] {
  const maxAltIdx = CHART.maxAlt / CHART.altStep
  const pts: [number, number][] = []
  for (let i = 0; i <= maxAltIdx && i < curve.length; i++) {
    pts.push([
      powerToX(curve[i], minP, maxP, chartW),
      altToY(i * CHART.altStep, chartH),
    ])
  }
  return pts
}

/** 谷形判定 (createLabelPosition :916-921): 前降后升 → 谷 (标签放左侧) */
export function isValleyAt(curve: readonly number[], altitudeM: number): boolean {
  const altIdx = Math.trunc(altitudeM / CHART.altStep)
  const lookback = Math.max(altIdx - 2, 0)
  const lookahead = Math.min(altIdx + 2, curve.length - 1)
  const fallingBefore = curve[altIdx] < curve[lookback]
  const risingAfter = curve[lookahead] > curve[altIdx]
  return fallingBefore && risingAfter
}

/** 拐点标注文本 (createLabelPosition :924): "%s: %.0fhp / %dm" */
export function labelText(label: string, power: number, altitudeM: number): string {
  return `${label}: ${fmt0(power)}hp / ${altitudeM}m`
}

// ---------------------------------------------------------------------------
// LabelPosition (Java :126-156) — 标注位置 + 防碰撞 (可变对象, 逐句对位)
// ---------------------------------------------------------------------------

/** 标注布局态 (LabelPosition 字段面; labelWidth/Height 由调用方测量传入) */
export interface LabelLayout {
  markerX: number
  markerY: number
  labelX: number
  labelY: number
  labelWidth: number
  labelHeight: number
  text: string
  color: string
  /** true = 标注在标记左侧 */
  isLeftSide: boolean
  /** 0 = FM0, 1 = FM1 (碰撞策略只翻 FM1) */
  curveIndex: 0 | 1
}

/** getBounds (:136-139): (labelX-4, labelY-labelHeight+3, w+8, h+2) */
export function labelBounds(l: LabelLayout): { x: number; y: number; w: number; h: number } {
  return { x: l.labelX - 4, y: l.labelY - l.labelHeight + 3, w: l.labelWidth + 8, h: l.labelHeight + 2 }
}

/** Java Rectangle.intersects (开区间相交) */
function intersects(
  a: { x: number; y: number; w: number; h: number },
  b: { x: number; y: number; w: number; h: number },
): boolean {
  return a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

/** flipSide (:141-151): 左→右 (markerX+14) / 右→左 (markerX-w-14) */
export function flipSide(l: LabelLayout): void {
  if (l.isLeftSide) {
    l.labelX = l.markerX + 14
    l.isLeftSide = false
  } else {
    l.labelX = l.markerX - l.labelWidth - 14
    l.isLeftSide = true
  }
}

/** offsetY (:153-155): 垂直平移 */
export function offsetY(l: LabelLayout, delta: number): void {
  l.labelY += delta
}

/**
 * createLabelPosition (:899-947): 初始位置 (谷左/峰右, labelY = markerY-10) +
 * 边界钳制 (右溢翻转左 / 左<4 → 4 / 顶 <4 → h+1 / 底溢 → panelH-6)。
 */
export function initialLabelPosition(
  markerX: number,
  markerY: number,
  text: string,
  color: string,
  curveIndex: 0 | 1,
  isValley: boolean,
  labelWidth: number,
  labelHeight: number,
  panelW: number,
  panelH: number,
): LabelLayout {
  let labelY = markerY - 10
  let labelX: number
  let isLeftSide: boolean
  if (isValley) {
    labelX = markerX - labelWidth - 14
    isLeftSide = true
  } else {
    labelX = markerX + 14
    isLeftSide = false
  }
  // Boundary clamping (:938-944)
  if (labelX + labelWidth > panelW - 4) {
    labelX = markerX - labelWidth - 14
    isLeftSide = true
  }
  if (labelX < 4) labelX = 4
  if (labelY - labelHeight + 3 < 4) labelY = labelHeight + 1
  if (labelY + 2 > panelH - 4) labelY = panelH - 6
  return {
    markerX,
    markerY,
    labelX,
    labelY,
    labelWidth,
    labelHeight,
    text,
    color,
    isLeftSide,
    curveIndex,
  }
}

/**
 * resolveCollisions (:952-997): 按 markerY 升序排序后逐对消解 —
 * 策略1: FM1 且未翻过侧 (triedFlip = 已在左侧) → flipSide + 钳制, 解除则过;
 * 策略2: 垂直重叠量 (r1.y+r1.h-r2.y) > 0 → offsetY(overlap+5) + 底部钳制。
 * 就地修改 (Java 同款可变语义)。
 */
export function resolveCollisions(labels: LabelLayout[], panelW: number, panelH: number): void {
  if (labels.length < 2) return
  // Sort by Y coordinate (altitude)
  labels.sort((a, b) => a.markerY - b.markerY)
  for (let i = 0; i < labels.length; i++) {
    const r1 = labelBounds(labels[i])
    for (let j = i + 1; j < labels.length; j++) {
      const lp2 = labels[j]
      const r2 = labelBounds(lp2)
      if (!intersects(r1, r2)) continue
      // Strategy 1: Flip FM1 label to other side (if it's FM1)
      // triedFlip (:1000-1003): 已在左侧 = 试过 → 未试 = isLeftSide false
      if (lp2.curveIndex === 1 && !lp2.isLeftSide) {
        flipSide(lp2)
        if (lp2.labelX + lp2.labelWidth > panelW - 4) lp2.labelX = panelW - 4 - lp2.labelWidth
        if (lp2.labelX < 4) lp2.labelX = 4
        if (!intersects(r1, labelBounds(lp2))) continue // Resolved
      }
      // Strategy 2: Offset Y position
      const overlap = r1.y + r1.h - r2.y
      if (overlap > 0) {
        offsetY(lp2, overlap + 5)
        if (lp2.labelY + 2 > panelH - 4) lp2.labelY = panelH - 6
      }
    }
  }
}

// ---------------------------------------------------------------------------
// 单双模式展示分支 (initUI :560-633 的纯数据面)
// ---------------------------------------------------------------------------

/** 标题两行 (:560-573): 大字 fm 名 (双机 "a vs b") + 小字 速度|模式 */
export function titleLines(d: PowerCurveData): [string, string] {
  const modeText = d.wepMode ? 'WEP' : '军用'
  const speedText = d.speedKmh > 0 ? `${d.speedKmh} km/h (IAS)` : '静态'
  const first = d.dualMode ? `${d.fm0Name} vs ${d.fm1Name}` : d.fm0Name
  return [first, `速度: ${speedText} | 模式: ${modeText}`]
}

/** 统计面板行 (:599-632): FM0 峰值 (单机标签 "峰值功率") / FM1 峰值 / 部分错误 */
export interface StatLine {
  kind: 'fm0' | 'fm1' | 'error'
  text: string
}

export function statLines(d: PowerCurveData): StatLine[] {
  const out: StatLine[] = []
  if (d.curve0.valid) {
    out.push({
      kind: 'fm0',
      text: `${d.dualMode ? d.fm0Name : '峰值功率'} 峰值: ${fmt0(d.curve0.maxPower)} hp @ ${d.curve0.peakAltitude} m`,
    })
  }
  if (d.curve1?.valid) {
    out.push({
      kind: 'fm1',
      text: `${d.fm1Name} 峰值: ${fmt0(d.curve1.maxPower)} hp @ ${d.curve1.peakAltitude} m`,
    })
  }
  // Show partial error if one FM failed (:625-630)
  if (d.errorMessage) out.push({ kind: 'error', text: d.errorMessage })
  return out
}

/** 有任一有效曲线 (chart 面绘制守卫, :580-582/:721-722) */
export function hasAnyCurve(d: PowerCurveData): boolean {
  return d.curve0.valid || !!d.curve1?.valid
}

/** 居中大错误标签形态 (:587-592): 双失败且有错文案 */
export function showErrorCenter(d: PowerCurveData): boolean {
  return !hasAnyCurve(d) && d.errorMessage != null
}

/** 图例仅双曲线模式 (drawLegend 调用点 :751-753, 与曲线有效无关) */
export function showLegend(d: PowerCurveData): boolean {
  return d.dualMode
}

/** 单条曲线的拐点标注输入 (drawAllInflectionPoints :846-862 的收集面);
 *  index = 原始曲线序 (curve0 缺效时 curve1 仍按 FM1 策略, 不重排) */
export function collectLabels(
  entries: readonly { curve: PowerCurve; index: 0 | 1 }[],
  minP: number,
  maxP: number,
  chartW: number,
  chartH: number,
  measure: (text: string) => number,
  labelHeight: number,
  panelW: number,
  panelH: number,
): LabelLayout[] {
  const labels: LabelLayout[] = []
  for (const { curve, index } of entries) {
    for (const ip of curve.inflectionPoints) {
      if (ip.altitudeM > CHART.maxAlt) continue // createLabelPosition :903 守卫
      const markerX = powerToX(ip.power, minP, maxP, chartW)
      const markerY = altToY(ip.altitudeM, chartH)
      const text = labelText(ip.label, ip.power, ip.altitudeM)
      labels.push(
        initialLabelPosition(
          markerX,
          markerY,
          text,
          kindColor(ip.kind, index),
          index,
          isValleyAt(curve.powerCurve, ip.altitudeM),
          measure(text),
          labelHeight,
          panelW,
          panelH,
        ),
      )
    }
  }
  resolveCollisions(labels, panelW, panelH)
  return labels
}
