/**
 * W4 画布: Rust PNG 快照底图 + 求解矩形选择框 + 原生 pointer 拖拽。
 * 坐标系 = 画布系 (组件 pos × 行高的自然投影; 窗口是派生物):
 * - items 矩形直用画布系坐标; PNG (窗口视图) 以 −offset 反变换回画布视图锚定,
 *   其边界即"真实窗口外形" — 拖动只改 pos, 重 solve 后其余组件纹丝不动
 *   (此前窗口系矩形 + 画布系位移混用 → 拖最左/最上组件时整页反向跳动)
 * - 拖拽中前端本地换算 (px/unit) 平移选中框, 松手后重 solve 校准 —
 *   布局求解唯一真相在 Rust (画布只做视觉反馈)
 * - STAGE_PAD: 画布系坐标可为负/超窗口 (自由画布 4096), stage 留边容纳
 */
import React, { useEffect, useMemo, useRef, useState } from 'react'
import type { PageDoc, SolveResult } from './types'
import { pngToDataUrl } from './api'

const ZOOM = 1.6
/** 画布系负坐标/溢出容纳边 (px, ZOOM 前) */
const STAGE_PAD = 160

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

  // stage 范围: 覆盖画布系内容区 ±PAD (窗口矩形 + 组件矩形 + 负区)
  const maxX = Math.max(
    solve.contentX + solve.contentW,
    ...solve.items.map(it => it.x + it.w),
    solve.contentX + solve.pageW,
  )
  const maxY = Math.max(
    solve.contentY + solve.contentH,
    ...solve.items.map(it => it.y + it.h),
    solve.contentY + solve.pageH,
  )
  const w = (maxX + STAGE_PAD) * ZOOM
  const h = (maxY + STAGE_PAD) * ZOOM
  const pad = STAGE_PAD * ZOOM

  return (
    <div
      style={{
        flex: 1,
        overflow: 'auto',
        background: 'repeating-conic-gradient(#fafafa 0% 25%, #f0f0f0 0% 50%) 0 0/24px 24px',
        border: '1px solid #eee',
        borderRadius: 6,
      }}
      onPointerUp={() => {
        if (dragRef.current && (dragOffset[0] || dragOffset[1])) {
          onDrag(dragRef.current.id, [dragOffset[0] / ZOOM, dragOffset[1] / ZOOM])
        }
        dragRef.current = null
        setDragOffset([0, 0])
      }}
      onPointerCancel={() => {
        dragRef.current = null
        setDragOffset([0, 0])
      }}
      onPointerMove={e => {
        const d = dragRef.current
        if (!d) return
        setDragOffset([e.clientX - d.startX, e.clientY - d.startY])
      }}
    >
      <div style={{ position: 'relative', width: w, height: h, margin: `${pad / 4}px auto` }}>
        {dataUrl && (
          // 窗口视图 PNG 反变换: 画布系原点 = −offset (窗口原点在画布系的落点)
          <img
            src={dataUrl}
            width={solve.pageW * ZOOM}
            height={solve.pageH * ZOOM}
            style={{
              position: 'absolute',
              left: pad + -solve.offsetX * ZOOM,
              top: pad + -solve.offsetY * ZOOM,
              pointerEvents: 'none',
            }}
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
                left: pad + it.x * ZOOM + (sel ? dragOffset[0] : 0),
                top: pad + it.y * ZOOM + (sel ? dragOffset[1] : 0),
                width: Math.max(it.w * ZOOM, 6),
                height: Math.max(it.h * ZOOM, 6),
                border: sel ? '2px solid #FF69B4' : '1px dashed rgba(0,0,0,0.25)',
                background: sel ? 'rgba(255,105,180,0.10)' : 'transparent',
                cursor: 'move',
                borderRadius: 3,
                fontSize: 10,
                color: '#FF69B4',
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
