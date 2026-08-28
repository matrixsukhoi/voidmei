/**
 * 功率曲线 web 窗口 (PowerCurveWindow.java 复刻): SVG 双曲线图 (X=功率 hp,
 * Y=高度 m), 峰/谷/拐点标注 (防碰撞布局, chartLayout.ts 纯函数), 图例 (双机
 * 模式), 统计面板, errorMessage 形态 (双失败居中大字 / 单侧失败入统计行)。
 * 键: ESC 关窗 (Java :641-642)。
 */
import React, { useCallback, useEffect, useState } from 'react'
import { ConfigProvider, Spin } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { PowerCurveData } from '../api'
import { getPowerCurveData } from '../api'
import {
  CHART,
  PC_COLORS,
  collectLabels,
  curvePoints,
  gridHLines,
  gridVLines,
  hasAnyCurve,
  showLegend,
  showErrorCenter,
  statLines,
  titleLines,
  withAlpha,
} from './chartLayout'

/** 图表面板尺寸 (Java setPreferredSize(CHART_WIDTH, CHART_HEIGHT)) */
const PANEL_W = CHART.width
const PANEL_H = CHART.height
const CHART_W = PANEL_W - 2 * CHART.margin
const CHART_H = PANEL_H - 2 * CHART.margin
/** 标注字体 (Java bold 11f) 与行高 (FontMetrics.getHeight 近似) */
const LABEL_FONT = "bold 11px 'Microsoft YaHei UI', sans-serif"
const LABEL_H = 15

/** canvas 文本测量 (Java FontMetrics.stringWidth 的 web 对位); 上下文模块级复用 */
let measureCtx: CanvasRenderingContext2D | null = null
function measureText(text: string): number {
  if (!measureCtx) measureCtx = document.createElement('canvas').getContext('2d')
  if (!measureCtx) return text.length * 7 // 无 2d 上下文兜底 (等宽估算)
  measureCtx.font = LABEL_FONT
  return Math.ceil(measureCtx.measureText(text).width)
}

const PowerCurveApp: React.FC<{
  fm0Initial: string
  fm1Initial: string | null
  speedInitial: number
  wepInitial: boolean
}> = ({ fm0Initial, fm1Initial, speedInitial, wepInitial }) => {
  const [data, setData] = useState<PowerCurveData | null>(null)
  const [err, setErr] = useState('')

  useEffect(() => {
    let cancelled = false
    getPowerCurveData(fm0Initial, fm1Initial, speedInitial, wepInitial)
      .then((d) => !cancelled && (setData(d), setErr('')))
      .catch((e) => !cancelled && setErr(String(e)))
    return () => {
      cancelled = true
    }
  }, [fm0Initial, fm1Initial, speedInitial, wepInitial])

  const close = useCallback(() => {
    getCurrentWindow().close().catch(() => undefined)
  }, [])

  // ESC to close (Java :641-642)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [close])

  const [title, subtitle] = data
    ? titleLines(data)
    : ['', '']

  return (
    <div
      style={{
        background: PC_COLORS.bg,
        height: '100vh',
        display: 'flex',
        flexDirection: 'column',
        padding: 15,
        color: '#fff',
        boxSizing: 'border-box',
      }}
    >
      {/* Title (NORTH): 大字机型 + 小字 速度|模式 */}
      <div style={{ textAlign: 'center', marginBottom: 10 }}>
        <div style={{ fontSize: 19, fontWeight: 700 }}>{title}</div>
        <div style={{ fontSize: 13, marginTop: 2 }}>{subtitle}</div>
      </div>
      {/* CENTER: 图表 / 错误 / 加载中 */}
      <div style={{ flex: 1, minHeight: 0, overflow: 'auto', display: 'flex', justifyContent: 'center' }}>
        {err && <div style={{ color: PC_COLORS.error, padding: 24 }}>加载失败: {err}</div>}
        {!data && !err && (
          <div style={{ padding: 24 }}>
            <Spin />
          </div>
        )}
        {data && showErrorCenter(data) && (
          // No valid curves - show error (居中大字, Java :587-592)
          <div style={{ color: PC_COLORS.error, fontSize: 14, padding: 24 }}>{data.errorMessage}</div>
        )}
        {data && hasAnyCurve(data) && <ChartSvg data={data} />}
      </div>
      {/* SOUTH: 统计面板 + 关闭按钮 (Java addCloseButton 复合南面板) */}
      {data && hasAnyCurve(data) && (
        <div style={{ marginTop: 10 }}>
          <div style={{ display: 'flex', justifyContent: 'center', flexWrap: 'wrap', gap: '5px 20px', marginBottom: 10 }}>
            {statLines(data).map((s, i) => {
              // 峰值着色 (Java <b style='color:...'>: fm0 恒 #2EFF71, fm1 #00D4FF,
              // 错误行 #FFA000 且字号 12)
              const style: React.CSSProperties =
                s.kind === 'fm0'
                  ? { fontSize: 14, color: PC_COLORS.curve[0] }
                  : s.kind === 'fm1'
                    ? { fontSize: 14, color: PC_COLORS.curve[1] }
                    : { fontSize: 12, color: PC_COLORS.error }
              return (
                <span key={i} style={style}>
                  {s.text}
                </span>
              )
            })}
          </div>
        </div>
      )}
      <button
        ref={(el) => {
          if (!el) return
          el.onmouseenter = () => (el.style.background = '#D32F2F')
          el.onmouseleave = () => (el.style.background = '#B71C1C')
        }}
        onClick={close}
        style={{
          background: '#B71C1C',
          color: '#fff',
          fontWeight: 700,
          fontSize: 12,
          border: 'none',
          padding: '8px 0',
          cursor: 'pointer',
        }}
      >
        关闭
      </button>
    </div>
  )
}

/** 图表 SVG (ChartPanel.paintComponent 复刻: 网格→轴→曲线→拐点→图例) */
const ChartSvg: React.FC<{ data: PowerCurveData }> = ({ data }) => {
  const hLines = gridHLines(CHART_H)
  const vLines = gridVLines(data.displayMinPower, data.displayMaxPower, CHART_W)
  // 原始曲线序保持 (Java 各曲线绑定自带色族/碰撞策略 — curve0 缺效时 curve1
  // 仍按 FM1 青系着色与翻侧策略, 不得重排为 0)
  const curves: { curve: PowerCurveData['curve0']; index: 0 | 1 }[] = []
  if (data.curve0.valid) curves.push({ curve: data.curve0, index: 0 })
  if (data.curve1?.valid) curves.push({ curve: data.curve1, index: 1 })
  const labels = collectLabels(
    curves,
    data.displayMinPower,
    data.displayMaxPower,
    CHART_W,
    CHART_H,
    measureText,
    LABEL_H,
    PANEL_W,
    PANEL_H,
  )
  // Legend (drawLegend :1008-1034): 右上角圆角底 + 两行机型
  const legendX = CHART.margin + CHART_W - 200
  const legendY = CHART.margin + 20
  return (
    <svg
      viewBox={`0 0 ${PANEL_W} ${PANEL_H}`}
      style={{ width: '100%', maxWidth: PANEL_W, display: 'block', background: PC_COLORS.chartBg }}
      fontFamily="'Microsoft YaHei UI', sans-serif"
    >
      {/* Draw in order: grid, axes, curves, inflection points, legend (原序注释) */}
      {hLines.map((l, i) => (
        <line key={`h${i}`} x1={CHART.margin} y1={l.y} x2={CHART.margin + CHART_W} y2={l.y} stroke={PC_COLORS.grid} strokeWidth={1} />
      ))}
      {vLines.map((l, i) => (
        <line key={`v${i}`} x1={l.x} y1={CHART.margin} x2={l.x} y2={CHART.margin + CHART_H} stroke={PC_COLORS.grid} strokeWidth={1} />
      ))}
      {/* Y 轴标签 (右对齐 MARGIN-5, 基线 y+4) */}
      {hLines.map((l, i) => (
        <text key={`yl${i}`} x={CHART.margin - 5} y={l.y + 4} fill={PC_COLORS.axis} fontSize={12} textAnchor="end">
          {l.label}
        </text>
      ))}
      {/* X 轴标签 (居中, MARGIN+chartH+18) */}
      {vLines.map((l, i) => (
        <text key={`xl${i}`} x={l.x} y={CHART.margin + CHART_H + 18} fill={PC_COLORS.axis} fontSize={12} textAnchor="middle">
          {l.label}
        </text>
      ))}
      {/* X-axis title */}
      <text x={CHART.margin + CHART_W + 5} y={CHART.margin + CHART_H + 4} fill={PC_COLORS.axis} fontSize={12}>
        hp
      </text>
      {/* 曲线 (stroke 2.5, CAP_ROUND/JOIN_ROUND; 色按原始曲线序) */}
      {curves.map(({ curve, index }) => (
        <polyline
          key={`c${index}`}
          points={curvePoints(curve.powerCurve, data.displayMinPower, data.displayMaxPower, CHART_W, CHART_H)
            .map(([x, y]) => `${x},${y}`)
            .join(' ')}
          fill="none"
          stroke={PC_COLORS.curve[index]}
          strokeWidth={2.5}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      ))}
      {/* 拐点标注: 光晕 + 实心点 + 虚线到 Y 轴 + 标签 (drawAllInflectionPoints) */}
      {labels.map((l, i) => (
        <g key={`ip${i}`}>
          <line
            x1={CHART.margin}
            y1={l.markerY}
            x2={l.markerX - 6}
            y2={l.markerY}
            stroke={withAlpha(l.color, 100)}
            strokeWidth={1}
            strokeDasharray="4,4"
          />
          <circle cx={l.markerX} cy={l.markerY} r={10} fill={withAlpha(l.color, 80)} />
          <circle cx={l.markerX} cy={l.markerY} r={6} fill={l.color} />
          <rect
            x={l.labelX - 4}
            y={l.labelY - l.labelHeight + 3}
            width={l.labelWidth + 8}
            height={l.labelHeight + 2}
            rx={4}
            fill="rgba(30,30,35,0.784)"
          />
          <text x={l.labelX} y={l.labelY} fill={l.color} fontSize={11} fontWeight={700}>
            {l.text}
          </text>
        </g>
      ))}
      {/* Legend (双机模式) */}
      {showLegend(data) && (
        <g>
          <rect x={legendX - 10} y={legendY - 15} width={190} height={50} rx={6} fill="rgba(30,30,35,0.863)" />
          <line x1={legendX} y1={legendY} x2={legendX + 30} y2={legendY} stroke={PC_COLORS.curve[0]} strokeWidth={2.5} />
          <text x={legendX + 40} y={legendY + 4} fill="#fff" fontSize={11}>
            {data.fm0Name}
          </text>
          <line x1={legendX} y1={legendY + 22} x2={legendX + 30} y2={legendY + 22} stroke={PC_COLORS.curve[1]} strokeWidth={2.5} />
          <text x={legendX + 40} y={legendY + 26} fill="#fff" fontSize={11}>
            {data.fm1Name}
          </text>
        </g>
      )}
    </svg>
  )
}

/** 窗口根 (暗色 AntD 主题; 主窗粉白主题不串染) */
export const PowerCurveWindowRoot: React.FC = () => {
  const params = new URLSearchParams(window.location.search)
  return (
    <ConfigProvider locale={zhCN}>
      <PowerCurveApp
        fm0Initial={params.get('fm0') ?? 'bf-109f-4'}
        fm1Initial={params.get('fm1')}
        speedInitial={parseInt(params.get('speed') ?? '0', 10) || 0}
        wepInitial={params.get('wep') === 'true'}
      />
    </ConfigProvider>
  )
}
