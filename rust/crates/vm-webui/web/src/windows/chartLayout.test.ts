/**
 * 功率曲线图表纯布局函数测试: 坐标映射 (网格/轴) / 标签防碰撞
 * (LabelPosition flipSide/offsetY 逐句对位) / 单双模式展示分支。
 */
import { describe, expect, it } from 'vitest'
import type { PowerCurve, PowerCurveData } from '../api'
import {
  CHART,
  altToY,
  collectLabels,
  curvePoints,
  fmt0,
  flipSide,
  gridHLines,
  gridVLines,
  hasAnyCurve,
  initialLabelPosition,
  isValleyAt,
  kindColor,
  labelBounds,
  labelText,
  powerToX,
  resolveCollisions,
  showErrorCenter,
  showLegend,
  statLines,
  titleLines,
  withAlpha,
} from './chartLayout'

// 面板几何 (Java: 1000×650 面板, MARGIN 80 → chartW/H 840/490)
const CW = CHART.width - 2 * CHART.margin
const CH = CHART.height - 2 * CHART.margin

describe('坐标映射 (drawGrid/drawAxes/drawPowerCurve (int) 截断)', () => {
  it('powerToX: MARGIN + (p-min)/range*chartW 截断', () => {
    expect(powerToX(500, 0, 1000, CW)).toBe(500) // 80 + 420
    expect(powerToX(123.7, 0, 1000, CW)).toBe(183) // 80 + trunc(103.908)
    expect(powerToX(300, 300, 1000, CW)).toBe(80) // 左边界
  })
  it('altToY: 底 0m → MARGIN+chartH, 顶 10000m → MARGIN', () => {
    expect(altToY(0, CH)).toBe(CHART.margin + CH)
    expect(altToY(CHART.maxAlt, CH)).toBe(CHART.margin)
    expect(altToY(5000, CH)).toBe(325) // 80+490-245
  })
  it('gridHLines: 每 1000m 一条, 11 条带标签', () => {
    const lines = gridHLines(CH)
    expect(lines).toHaveLength(11)
    expect(lines[0]).toEqual({ y: CHART.margin + CH, label: '0m' })
    expect(lines[10]).toEqual({ y: CHART.margin, label: '10000m' })
  })
  it('gridVLines: 每 100hp, min/max 向零截断取档', () => {
    const lines = gridVLines(0, 1000, CW)
    expect(lines).toHaveLength(11)
    expect(lines[0].x).toBe(80)
    expect(lines[5]).toEqual({ x: 80 + 5 * 84, label: '500' })
    // min=300 → 从 300 档起, 首线落在左边界
    const from300 = gridVLines(300, 1000, CW)
    expect(from300).toHaveLength(8)
    expect(from300[0].x).toBe(80)
    expect(from300[0].label).toBe('300')
  })
  it('curvePoints: 0..10000m 步 25 → 401 点, 截到数组长度', () => {
    const short = new Array(10).fill(500)
    expect(curvePoints(short, 0, 1000, CW, CH)).toHaveLength(10)
    expect(curvePoints(new Array(401).fill(0), 0, 1000, CW, CH)).toHaveLength(401)
    const pts = curvePoints(new Array(401).fill(0), 0, 1000, CW, CH)
    expect(pts[0][1]).toBe(altToY(0, CH))
    expect(pts[400][1]).toBe(altToY(10000, CH))
  })
})

describe('fmt0 (Java String.format %.0f = HALF_UP)', () => {
  it('正数四舍五入; .5 进位; 负数远离零', () => {
    expect(fmt0(1800.4)).toBe(1800)
    expect(fmt0(1799.5)).toBe(1800)
    expect(fmt0(-2.5)).toBe(-3)
    expect(fmt0(-2.4)).toBe(-2)
  })
})

describe('拐点标注着色与文本', () => {
  it('kindColor: FM0/FM1 各自色族 (Java Color 字段)', () => {
    expect(kindColor('peak', 0)).toBe('#FFD700')
    expect(kindColor('peak', 1)).toBe('#FF80B4')
    expect(kindColor('valley', 0)).toBe('#64C8FF')
    expect(kindColor('kink', 1)).toBe('#82FFB4')
  })
  it('withAlpha: hex+alpha(0-255) → rgba', () => {
    expect(withAlpha('#2EFF71', 80)).toBe('rgba(46,255,113,0.314)') // 80/255=0.3137→0.314
  })
  it('labelText: "%s: %.0fhp / %dm"', () => {
    expect(labelText('1档', 1523.6, 2000)).toBe('1档: 1524hp / 2000m')
    expect(labelText('Kink', 999.49, 350)).toBe('Kink: 999hp / 350m')
  })
  it('isValleyAt: 前降后升 → 谷 (±2 档窗口)', () => {
    const valley = [0, 10, 10, 10, 5, 5, 10, 10] // idx4 谷 (高度 100m)
    expect(isValleyAt(valley, 100)).toBe(true)
    const peak = [0, 5, 5, 5, 10, 10, 5, 5]
    expect(isValleyAt(peak, 100)).toBe(false)
    // 边界: 高度 0 → lookback 钳到 0, falling 判定失效
    expect(isValleyAt([10, 10, 20, 30], 0)).toBe(false)
  })
})

describe('LabelPosition 初始位置与边界钳制 (createLabelPosition :899-947)', () => {
  const mk = (markerX: number, markerY: number, isValley: boolean, curveIndex: 0 | 1 = 0, w = 100) =>
    initialLabelPosition(markerX, markerY, 't', '#fff', curveIndex, isValley, w, 15, 1000, 650)

  it('谷标签放左侧 (markerX-w-14), 峰/拐点放右侧 (markerX+14); labelY = markerY-10', () => {
    const v = mk(500, 300, true)
    expect(v.labelX).toBe(386)
    expect(v.isLeftSide).toBe(true)
    expect(v.labelY).toBe(290)
    const p = mk(500, 300, false)
    expect(p.labelX).toBe(514)
    expect(p.isLeftSide).toBe(false)
  })
  it('右溢 (labelX+w > panelW-4) 翻转到左侧', () => {
    const l = mk(950, 300, false)
    expect(l.labelX).toBe(950 - 100 - 14)
    expect(l.isLeftSide).toBe(true)
  })
  it('左溢钳到 4; 顶溢钳到 labelHeight+1', () => {
    expect(mk(90, 300, true).labelX).toBe(4)
    const top = mk(500, 5, false)
    expect(top.labelY).toBe(16) // h=15 → 15+1
  })
  it('labelBounds: (x-4, y-h+3, w+8, h+2)', () => {
    expect(labelBounds(mk(500, 300, false))).toEqual({ x: 510, y: 278, w: 108, h: 17 })
  })
  it('flipSide: 左→右 (markerX+14) / 右→左 (markerX-w-14)', () => {
    const l = mk(500, 300, true)
    flipSide(l)
    expect(l.labelX).toBe(514)
    expect(l.isLeftSide).toBe(false)
    flipSide(l)
    expect(l.labelX).toBe(386)
    expect(l.isLeftSide).toBe(true)
  })
})

describe('resolveCollisions (防碰撞 :952-997)', () => {
  const mk = (
    curveIndex: 0 | 1,
    markerX: number,
    markerY: number,
    w = 100,
    panelH = 650,
  ) => initialLabelPosition(markerX, markerY, 't', '#fff', curveIndex, false, w, 15, 1000, panelH)

  it('策略1: FM1 标签翻侧解除碰撞', () => {
    const a = mk(0, 500, 300)
    const b = mk(1, 520, 305)
    resolveCollisions([a, b], 1000, 650)
    expect(b.labelX).toBe(520 - 100 - 14)
    expect(b.isLeftSide).toBe(true)
    // 翻侧后与 a 不再相交 (a 右缘 618, b 新右缘 510, 开区间不相交)
    const ra = labelBounds(a)
    const rb = labelBounds(b)
    expect(ra.x < rb.x + rb.w && ra.x + ra.w > rb.x).toBe(false)
  })
  it('策略2: 翻侧仍碰撞 → offsetY(overlap+5)', () => {
    // a 宽 400 (右缘 918), b 翻到左侧仍落在 a 区间内
    const a = mk(0, 500, 300, 400)
    const b = mk(1, 600, 305)
    resolveCollisions([a, b], 1000, 650)
    // overlap = r1.y+r1.h-r2.y = 278+17-283 = 12 → +5 = 17
    expect(b.labelY).toBe(295 + 17)
  })
  it('FM0 标签不翻侧 (策略只对 FM1), 直接 offset', () => {
    const a = mk(0, 500, 300)
    const b = mk(0, 520, 305)
    resolveCollisions([a, b], 1000, 650)
    expect(b.labelX).toBe(534) // 未翻
    expect(b.labelY).toBe(312)
  })
  it('offset 触底钳制 (panelH-6)', () => {
    const a = mk(0, 500, 300, 400, 310)
    const b = mk(1, 600, 305, 100, 310)
    resolveCollisions([a, b], 1000, 310)
    expect(b.labelY).toBe(304) // 310-6
  })
  it('按 markerY 排序 (输入序无关) + 单标签无操作', () => {
    const a = mk(0, 500, 300)
    const b = mk(1, 520, 305)
    resolveCollisions([b, a], 1000, 650) // 乱序输入
    expect(b.isLeftSide).toBe(true)
    const single = mk(0, 500, 300)
    expect(() => resolveCollisions([single], 1000, 650)).not.toThrow()
    expect(single.labelX).toBe(514)
  })
})

// ---------------------------------------------------------------------------
// 单双模式展示分支 (initUI :560-633)
// ---------------------------------------------------------------------------

const mkCurve = (over: Partial<PowerCurve>): PowerCurve => ({
  fmName: 'x',
  valid: true,
  powerCurve: [1000, 1200],
  altStep: 25,
  maxDisplayAlt: 10000,
  maxPower: 1800,
  minPower: 900,
  peakAltitude: 2000,
  inflectionPoints: [],
  errorMessage: null,
  ...over,
})

const mkDto = (over: Partial<PowerCurveData>): PowerCurveData => ({
  fm0Name: 'spitfire_f24',
  fm1Name: null,
  dualMode: false,
  speedKmh: 0,
  wepMode: false,
  curve0: mkCurve({}),
  curve1: null,
  displayMaxPower: 2000,
  displayMinPower: 900,
  errorMessage: null,
  ...over,
})

describe('titleLines (标题两行 :560-573)', () => {
  it('双机: "a vs b" + 速度/模式; 单机: 仅 fm0', () => {
    const [l1, l2] = titleLines(mkDto({ dualMode: true, fm1Name: 'spitfire_f22', speedKmh: 400, wepMode: true }))
    expect(l1).toBe('spitfire_f24 vs spitfire_f22')
    expect(l2).toBe('速度: 400 km/h (IAS) | 模式: WEP')
    const [s1, s2] = titleLines(mkDto({}))
    expect(s1).toBe('spitfire_f24')
    expect(s2).toBe('速度: 静态 | 模式: 军用')
  })
})

describe('statLines (统计面板 :599-632)', () => {
  it('单机: "峰值功率" 标签; 双机: 各自机型名', () => {
    // Java: "%s 峰值: %.0f hp @ %d m" — 单机 %s = "峰值功率" (字面拼接后双"峰值")
    expect(statLines(mkDto({ curve0: mkCurve({ maxPower: 1800.4, peakAltitude: 2000 }) }))).toEqual([
      { kind: 'fm0', text: '峰值功率 峰值: 1800 hp @ 2000 m' },
    ])
    const dual = mkDto({
      dualMode: true,
      fm1Name: 'spitfire_f22',
      curve1: mkCurve({ maxPower: 1650.5, peakAltitude: 1800 }),
    })
    expect(statLines(dual).map((s) => s.text)).toEqual([
      'spitfire_f24 峰值: 1800 hp @ 2000 m',
      'spitfire_f22 峰值: 1651 hp @ 1800 m',
    ])
  })
  it('无效曲线不出统计行; 部分错误追加 error 行', () => {
    const d = mkDto({
      dualMode: true,
      fm1Name: 'a-10c',
      curve1: mkCurve({ valid: false, errorMessage: 'a-10c 不是活塞引擎' }),
      errorMessage: 'a-10c 不是活塞引擎',
    })
    const lines = statLines(d)
    expect(lines).toHaveLength(2)
    expect(lines[1]).toEqual({ kind: 'error', text: 'a-10c 不是活塞引擎' })
  })
})

describe('图表绘制守卫 (单双/错误形态分支 :580-592/:751-753)', () => {
  it('双失败 + 有错文案 → 居中大错误', () => {
    const d = mkDto({
      curve0: mkCurve({ valid: false, errorMessage: 'x 不是活塞引擎' }),
      errorMessage: 'x 不是活塞引擎',
    })
    expect(hasAnyCurve(d)).toBe(false)
    expect(showErrorCenter(d)).toBe(true)
    expect(showLegend(d)).toBe(false)
  })
  it('单侧失败 → 图表照画, 错误入统计行; 双机模式图例在场 (与有效无关)', () => {
    const d = mkDto({
      dualMode: true,
      fm1Name: 'a-10c',
      curve1: mkCurve({ valid: false, errorMessage: 'a-10c 不是活塞引擎' }),
      errorMessage: 'a-10c 不是活塞引擎',
    })
    expect(hasAnyCurve(d)).toBe(true)
    expect(showErrorCenter(d)).toBe(false)
    expect(showLegend(d)).toBe(true)
  })
  it('单机有效曲线: 无图例无错误', () => {
    const d = mkDto({})
    expect(hasAnyCurve(d)).toBe(true)
    expect(showErrorCenter(d)).toBe(false)
    expect(showLegend(d)).toBe(false)
  })
})

describe('collectLabels (标注收集 + 防碰撞闭环)', () => {
  it('原始曲线序保持: 仅 curve1 在场仍按 FM1 策略翻侧', () => {
    // 两条同位标注: curve0 侧 + curve1 侧碰撞 → FM1 翻侧
    const entries = [
      { curve: mkCurve({ inflectionPoints: [{ kind: 'peak', label: '1档', altitudeM: 2000, power: 1500 }] }), index: 0 as const },
      { curve: mkCurve({ inflectionPoints: [{ kind: 'kink', label: 'Kink', altitudeM: 2000, power: 1500 }] }), index: 1 as const },
    ]
    const labels = collectLabels(entries, 900, 2000, CW, CH, () => 100, 15, 1000, 650)
    expect(labels).toHaveLength(2)
    expect(labels[0].curveIndex).toBe(0)
    expect(labels[1].curveIndex).toBe(1)
    expect(labels[1].isLeftSide).toBe(true) // 翻侧解除
    // 高度超 10000m 的标注丢弃 (:903 守卫)
    const far = collectLabels(
      [{ curve: mkCurve({ inflectionPoints: [{ kind: 'kink', label: 'K', altitudeM: 10500, power: 1000 }] }), index: 0 as const }],
      900, 2000, CW, CH, () => 100, 15, 1000, 650,
    )
    expect(far).toHaveLength(0)
  })
})
