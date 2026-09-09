/**
 * W4 画布 (C4 交互层): Rust PNG 快照底图 + 求解矩形 + 拖拽/框选/缩放/吸附。
 * 坐标系 = 画布系 (组件 pos × 行高的自然投影; 窗口是派生物):
 * - items 矩形直用画布系坐标; PNG (窗口视图) 以 −offset 反变换锚定,
 *   其边界即"真实窗口外形" — 拖动只改 pos, 重 solve 后其余组件纹丝不动
 * - 拖拽中 computeSnap 实时吸附 (网格 + 对齐参考线, Shift 临时禁用),
 *   松手提交 → Rust 重 solve 校准 — 布局求解唯一真相在 Rust
 * - 缩放: Ctrl+滚轮 (光标锚定) / 工具栏 ±/适应/100% (LayoutTab)
 * - 多选: Shift/Ctrl 点选 toggle + 空白拖框选; 成组拖动
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { PageDoc, SolveResult } from './types'
import { pngToDataUrl } from './api'
import { computeSnap, type Rect, type SnapGuide } from './snap'

export const ACCENT = '#FF69B4'
const ZOOM_MIN = 0.2
const ZOOM_MAX = 3
/** 画布系内容外容纳边 (px, zoom 前 — 自由画布组件可溢出窗口) */
const STAGE_MARGIN = 80
/** 对齐吸附阈值 (屏幕 px → 画布 px 由 zoom 换算) */
const ALIGN_TOL_SCREEN = 6

interface CanvasProps {
  solve: SolveResult | null
  page: PageDoc
  selectedIds: string[]
  onSelectionChange: (ids: string[]) => void
  onDragCommit: (ids: string[], dPx: [number, number]) => void
  zoom: number
  onZoom: (z: number) => void
  /** fit-to-view 触发计数 (LayoutTab 工具栏按钮递增) */
  fitTick: number
}

/** imperative 面 (palette 拖放落点换算 — stage 几何只有 Canvas 知道) */
export interface CanvasHandle {
  /** 屏幕 client → 画布 px (不在 stage 上返回 null) */
  clientToCanvas: (clientX: number, clientY: number) => [number, number] | null
}

export const Canvas = React.forwardRef<CanvasHandle, CanvasProps>(function Canvas(
  {
    solve,
    page,
    selectedIds,
    onSelectionChange,
    onDragCommit,
    zoom,
    onZoom,
    fitTick,
  },
  ref,
) {
  const dataUrl = useMemo(
    () => (solve && solve.png.length ? pngToDataUrl(solve.png) : ''),
    [solve],
  )
  const viewRef = useRef<HTMLDivElement>(null)
  const stageRef = useRef<HTMLDivElement>(null)

  // 拖动/框选状态 (ref 供事件处理, state 供渲染)
  const dragRef = useRef<{
    mode: 'move' | 'marquee'
    ids: string[]
    startClient: [number, number]
    baseRects: Rect[]
    snapped: [number, number]
  } | null>(null)
  const [dragOffset, setDragOffset] = useState<[number, number]>([0, 0])
  const [guides, setGuides] = useState<SnapGuide[]>([])
  const [marquee, setMarquee] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null)

  // 选中/求解变化清拖拽视觉
  useEffect(() => {
    setDragOffset([0, 0])
    setGuides([])
    setMarquee(null)
    dragRef.current = null
  }, [selectedIds, solve])

  // ---- 缩放: Ctrl+滚轮光标锚定 (native listener — React onWheel passive 无法 preventDefault) ----
  useEffect(() => {
    const el = viewRef.current
    if (!el) return
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return // 普通滚轮 = 容器滚动
      e.preventDefault()
      const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12
      const next = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoom * factor))
      if (next === zoom) return
      // 光标下的画布点保持不动: 视口坐标换算 scrollLeft/top 补偿
      const cursorX = e.clientX - el.getBoundingClientRect().left
      const cursorY = e.clientY - el.getBoundingClientRect().top
      const canvasX = (el.scrollLeft + cursorX) / zoom
      const canvasY = (el.scrollTop + cursorY) / zoom
      onZoom(next)
      requestAnimationFrame(() => {
        el.scrollLeft = canvasX * next - cursorX
        el.scrollTop = canvasY * next - cursorY
      })
    }
    el.addEventListener('wheel', onWheel, { passive: false })
    return () => el.removeEventListener('wheel', onWheel)
  }, [zoom, onZoom])

  // ---- fit-to-view (工具栏触发 + 换页自动一次) ----
  const fittedPage = useRef<string>('')

  // ---- stage 几何 (画布系 → 屏幕 px 的换算基; hooks 需先于 early return) ----
  const geo = useMemo(() => {
    if (!solve) return null
    const xs = solve.items.flatMap(it => [it.x, it.x + it.w]).concat([solve.contentX, solve.contentX + solve.pageW])
    const ys = solve.items.flatMap(it => [it.y, it.y + it.h]).concat([solve.contentY, solve.contentY + solve.pageH])
    return {
      originX: Math.min(0, ...xs) - STAGE_MARGIN,
      originY: Math.min(0, ...ys) - STAGE_MARGIN,
      maxX: Math.max(...xs, solve.contentX + solve.contentW) + STAGE_MARGIN,
      maxY: Math.max(...ys, solve.contentY + solve.contentH) + STAGE_MARGIN,
    }
  }, [solve])

  /** fit-to-view (工具栏触发; 换页自动一次 — 用户随后手动缩放不被覆盖) */
  const fit = useCallback(() => {
    const el = viewRef.current
    if (!el || !geo) return
    const w = geo.maxX - geo.originX
    const h = geo.maxY - geo.originY
    if (w <= 0 || h <= 0) return
    const z = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.min((el.clientWidth - 32) / w, (el.clientHeight - 32) / h)))
    onZoom(z)
    requestAnimationFrame(() => {
      el.scrollLeft = -geo.originX * z - (el.clientWidth - w * z) / 2
      el.scrollTop = -geo.originY * z - (el.clientHeight - h * z) / 2
    })
  }, [geo, onZoom])
  useEffect(() => {
    if (fitTick > 0) fit()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fitTick])
  useEffect(() => {
    if (solve && geo && page.id !== fittedPage.current) {
      fittedPage.current = page.id
      fit()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [solve, page.id])

  // palette 拖放落点换算 (stage 几何只有 Canvas 知道)
  React.useImperativeHandle(
    ref,
    () => ({
      clientToCanvas: (clientX: number, clientY: number): [number, number] | null => {
        const r = stageRef.current?.getBoundingClientRect()
        if (!r || !geo) return null
        if (clientX < r.left || clientX > r.right || clientY < r.top || clientY > r.bottom) return null
        return [(clientX - r.left) / zoom + geo.originX, (clientY - r.top) / zoom + geo.originY]
      },
    }),
    [geo, zoom],
  )

  if (!solve) return <div style={{ flex: 1, display: 'grid', placeItems: 'center' }}>求解中…</div>

  // ---- stage 几何 (渲染换算) ----
  const { originX, originY, maxX, maxY } = geo!
  const toScreen = (v: number) => v * zoom
  const sx = (x: number) => toScreen(x - originX)
  const sy = (y: number) => toScreen(y - originY)

  // 屏幕 client → 画布 px
  const clientToCanvas = (clientX: number, clientY: number): [number, number] => {
    const r = stageRef.current?.getBoundingClientRect()
    if (!r) return [0, 0]
    return [(clientX - r.left) / zoom + originX, (clientY - r.top) / zoom + originY]
  }

  // ---- pointer 事件 ----
  const onComponentPointerDown = (e: React.PointerEvent, id: string) => {
    e.stopPropagation()
    let ids: string[]
    if (e.shiftKey || e.ctrlKey || e.metaKey) {
      ids = selectedIds.includes(id)
        ? selectedIds.filter(s => s !== id) // toggle
        : [...selectedIds, id]
      onSelectionChange(ids)
      if (!ids.includes(id)) return // toggle 掉的不进入拖动
    } else {
      ids = selectedIds.includes(id) ? selectedIds : [id]
      if (!selectedIds.includes(id)) onSelectionChange(ids)
    }
    const rects = ids
      .map(id2 => solve.items.find(it => it.id === id2))
      .filter((v): v is NonNullable<typeof v> => !!v)
      .map(it => ({ x: it.x, y: it.y, w: it.w, h: it.h }))
    dragRef.current = {
      mode: 'move',
      ids,
      startClient: [e.clientX, e.clientY],
      baseRects: rects,
      snapped: [0, 0],
    }
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
  }

  const onStagePointerDown = (e: React.PointerEvent) => {
    // 空白: 开框选
    dragRef.current = {
      mode: 'marquee',
      ids: [],
      startClient: [e.clientX, e.clientY],
      baseRects: [],
      snapped: [0, 0],
    }
    const [cx, cy] = clientToCanvas(e.clientX, e.clientY)
    setMarquee({ x0: cx, y0: cy, x1: cx, y1: cy })
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
  }

  const onPointerMove = (e: React.PointerEvent) => {
    const d = dragRef.current
    if (!d) return
    if (d.mode === 'marquee') {
      const [cx, cy] = clientToCanvas(e.clientX, e.clientY)
      setMarquee(m => (m ? { ...m, x1: cx, y1: cy } : m))
      return
    }
    // move: 屏幕位移 → 画布 px → 吸附
    const rawDx = (e.clientX - d.startClient[0]) / zoom
    const rawDy = (e.clientY - d.startClient[1]) / zoom
    if (e.shiftKey) {
      // Shift = 临时禁用全部吸附 (自由拖)
      setDragOffset([rawDx, rawDy])
      setGuides([])
      d.snapped = [rawDx, rawDy]
      return
    }
    const statics = solve.items
      .filter(it => !d.ids.includes(it.id))
      .map(it => ({ x: it.x, y: it.y, w: it.w, h: it.h }))
    const r = computeSnap(d.baseRects, statics, { dx: rawDx, dy: rawDy }, {
      gridPx: 0.1 * solve.lineHeightPx,
      thresholdPx: ALIGN_TOL_SCREEN / zoom,
      contentRect: solve.contentW > 0
        ? { x: solve.contentX, y: solve.contentY, w: solve.contentW, h: solve.contentH }
        : null,
      canvasW: solve.canvasW,
      canvasH: solve.canvasH,
    })
    setDragOffset([r.dx, r.dy])
    setGuides(r.guides)
    d.snapped = [r.dx, r.dy]
  }

  const endDrag = (cancelled: boolean) => {
    const d = dragRef.current
    dragRef.current = null
    setGuides([])
    setDragOffset([0, 0])
    if (!d) return
    if (d.mode === 'marquee') {
      const m = marquee
      setMarquee(null)
      if (cancelled || !m) return
      const [dx, dy] = [Math.abs(m.x1 - m.x0), Math.abs(m.y1 - m.y0)]
      if (dx * zoom < 3 && dy * zoom < 3) {
        onSelectionChange([]) // 空白点击 = 清选
        return
      }
      const sel = {
        x: Math.min(m.x0, m.x1),
        y: Math.min(m.y0, m.y1),
        w: Math.max(m.x0, m.x1) - Math.min(m.x0, m.x1),
        h: Math.max(m.y0, m.y1) - Math.min(m.y0, m.y1),
      }
      const hit = solve.items
        .filter(it => it.x < sel.x + sel.w && it.x + it.w > sel.x && it.y < sel.y + sel.h && it.y + it.h > sel.y)
        .map(it => it.id)
      onSelectionChange(hit)
      return
    }
    if (cancelled) return
    if (d.snapped[0] || d.snapped[1]) onDragCommit(d.ids, d.snapped)
  }

  const dragging = dragOffset[0] !== 0 || dragOffset[1] !== 0

  return (
    <div
      ref={viewRef}
      style={{
        flex: 1,
        overflow: 'auto',
        background: 'repeating-conic-gradient(#fafafa 0% 25%, #f0f0f0 0% 50%) 0 0/24px 24px',
        border: '1px solid #eee',
        borderRadius: 6,
      }}
      onPointerMove={onPointerMove}
      onPointerUp={() => endDrag(false)}
      onPointerCancel={() => endDrag(true)}
    >
      <div
        ref={stageRef}
        style={{ position: 'relative', width: toScreen(maxX - originX), height: toScreen(maxY - originY) }}
        onPointerDown={onStagePointerDown}
      >
        {dataUrl && (
          // 窗口视图 PNG 反变换: 画布系原点 = −offset (边界即真实窗口外形)
          <img
            src={dataUrl}
            width={toScreen(solve.pageW)}
            height={toScreen(solve.pageH)}
            style={{
              position: 'absolute',
              left: sx(-solve.offsetX),
              top: sy(-solve.offsetY),
              pointerEvents: 'none',
            }}
            alt="page snapshot"
          />
        )}
        {solve.items.map(it => {
          const sel = selectedIds.includes(it.id)
          const inDrag = sel && dragging
          return (
            <div
              key={it.id}
              onPointerDown={e => onComponentPointerDown(e, it.id)}
              style={{
                position: 'absolute',
                left: sx(it.x) + (inDrag ? toScreen(dragOffset[0]) : 0),
                top: sy(it.y) + (inDrag ? toScreen(dragOffset[1]) : 0),
                width: Math.max(toScreen(it.w), 6),
                height: Math.max(toScreen(it.h), 6),
                border: sel ? `2px solid ${ACCENT}` : '1px dashed rgba(0,0,0,0.25)',
                background: sel ? 'rgba(255,105,180,0.10)' : 'transparent',
                cursor: 'move',
                borderRadius: 3,
                fontSize: 10,
                color: ACCENT,
                overflow: 'hidden',
                whiteSpace: 'nowrap',
              }}
              title={it.id}
            >
              {sel ? it.id : ''}
            </div>
          )
        })}
        {/* 对齐参考线 (粉色; 拖动中 computeSnap 产出) */}
        {guides.map((g, i) =>
          g.axis === 'x' ? (
            <div key={i} style={{ position: 'absolute', left: sx(g.at), top: 0, bottom: 0, width: 1, background: ACCENT, opacity: 0.55 }} />
          ) : (
            <div key={i} style={{ position: 'absolute', top: sy(g.at), left: 0, right: 0, height: 1, background: ACCENT, opacity: 0.55 }} />
          ),
        )}
        {/* 框选 rubber band */}
        {marquee && (
          <div
            style={{
              position: 'absolute',
              left: sx(Math.min(marquee.x0, marquee.x1)),
              top: sy(Math.min(marquee.y0, marquee.y1)),
              width: toScreen(Math.abs(marquee.x1 - marquee.x0)),
              height: toScreen(Math.abs(marquee.y1 - marquee.y0)),
              border: `1px solid ${ACCENT}`,
              background: 'rgba(255,105,180,0.08)',
              pointerEvents: 'none',
            }}
          />
        )}
        {page.components.length === 0 && (
          <div
            style={{
              position: 'absolute',
              inset: 0,
              display: 'grid',
              placeItems: 'center',
              color: '#bbb',
              fontSize: 13,
              pointerEvents: 'none',
            }}
          >
            从左侧 palette 点击/拖放组件添加
          </div>
        )}
      </div>
    </div>
  )
})
