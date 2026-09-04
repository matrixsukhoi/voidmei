/**
 * W4 画布: Rust PNG 快照底图 + 求解矩形选择框 + 原生 pointer 拖拽。
 * 拖拽中前端本地换算 (px/unit) 平移选中框, 松手后重 solve 校准 —
 * 布局求解唯一真相在 Rust (画布只做视觉反馈)。
 */
import React, { useEffect, useMemo, useRef, useState } from 'react'
import type { PageDoc, SolveResult } from './types'
import { pngToDataUrl } from './api'

const ZOOM = 1.6

interface CanvasProps {
  solve: SolveResult | null
  page: PageDoc
  selectedId: string
  onSelect: (id: string) => void
  onDrag: (id: string, dPx: [number, number]) => void
}

export const Canvas: React.FC<CanvasProps> = ({ solve, page, selectedId, onSelect, onDrag }) => {
  const dataUrl = useMemo(
    () => (solve && solve.png.length ? pngToDataUrl(solve.png) : ''),
    [solve],
  )
  const [dragOffset, setDragOffset] = useState<[number, number]>([0, 0])
  const dragRef = useRef<{ id: string; startX: number; startY: number } | null>(null)

  // 选中变化清拖拽偏移
  useEffect(() => setDragOffset([0, 0]), [selectedId, solve])

  if (!solve) return <div style={{ flex: 1, display: 'grid', placeItems: 'center' }}>求解中…</div>

  const w = solve.pageW * ZOOM
  const h = solve.pageH * ZOOM

  return (
    <div
      style={{
        flex: 1,
        overflow: 'auto',
        background: 'repeating-conic-gradient(#fafafa 0% 25%, #f0f0f0 0% 50%) 0 0/24px 24px',
        border: '1px solid #eee',
        borderRadius: 6,
        padding: 12,
      }}
      onPointerUp={() => {
        if (dragRef.current && (dragOffset[0] || dragOffset[1])) {
          onDrag(dragRef.current.id, [dragOffset[0] / ZOOM, dragOffset[1] / ZOOM])
        }
        dragRef.current = null
        setDragOffset([0, 0])
      }}
      onPointerMove={e => {
        const d = dragRef.current
        if (!d) return
        setDragOffset([e.clientX - d.startX, e.clientY - d.startY])
      }}
    >
      <div style={{ position: 'relative', width: w, height: h, margin: '0 auto' }}>
        {dataUrl && (
          <img
            src={dataUrl}
            width={w}
            height={h}
            style={{ position: 'absolute', inset: 0, pointerEvents: 'none', imageRendering: 'pixelated' }}
            alt="page snapshot"
          />
        )}
        {solve.items.map(it => {
          const sel = it.id === selectedId
          return (
            <div
              key={it.id}
              onPointerDown={e => {
                onSelect(it.id)
                dragRef.current = { id: it.id, startX: e.clientX, startY: e.clientY }
                ;(e.target as HTMLElement).setPointerCapture?.(e.pointerId)
              }}
              style={{
                position: 'absolute',
                left: it.x * ZOOM + (sel ? dragOffset[0] : 0),
                top: it.y * ZOOM + (sel ? dragOffset[1] : 0),
                width: Math.max(it.w * ZOOM, 6),
                height: Math.max(it.h * ZOOM, 6),
                border: sel ? '2px solid #1677ff' : '1px dashed rgba(0,0,0,0.25)',
                background: sel ? 'rgba(22,119,255,0.10)' : 'transparent',
                cursor: 'move',
                borderRadius: 3,
                fontSize: 10,
                color: '#1677ff',
                overflow: 'hidden',
                whiteSpace: 'nowrap',
              }}
              title={it.id}
            >
              {sel ? it.id : ''}
            </div>
          )
        })}
        {page.components.length === 0 && (
          <div
            style={{
              position: 'absolute',
              inset: 0,
              display: 'grid',
              placeItems: 'center',
              color: '#bbb',
              fontSize: 13,
            }}
          >
            从左侧 palette 点击组件添加
          </div>
        )}
      </div>
    </div>
  )
}
