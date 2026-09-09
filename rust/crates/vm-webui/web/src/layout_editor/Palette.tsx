/**
 * W4 palette: 常用字段预设 + 组件目录分组。
 * C5 拖放: pointer 系 (与画布统一事件模型) — 按住拖到画布指定位置释放
 * (落点换算经 LayoutTab 持有的 CanvasHandle); 位移 < 4px = 点击, 走原
 * 画布中心落点添加。
 */
import React, { useEffect, useMemo, useRef, useState } from 'react'
import { Tag } from 'antd'
import type { CatalogResponse, ComponentCatalogEntry, FieldPreset } from './types'
import { getComponentCatalog } from './api'

const CATEGORY_ZH: Record<string, string> = {
  Text: '文本',
  Gauge: '仪表',
  Chart: '图表',
  List: '列表',
  Composite: '复合 (黑盒)',
  Decor: '装饰',
}

/** 点击/拖放判定阈值 (px) */
const DRAG_THRESHOLD = 4

interface PaletteProps {
  onAdd: (typeName: string, displayZh: string, defaultProps?: Record<string, unknown>) => void
  /** 拖放释放 (画布外释放由 LayoutTab 静默取消) */
  onDrop: (
    entry: { typeName: string; displayZh: string; defaultProps?: Record<string, unknown> },
    clientX: number,
    clientY: number,
  ) => void
  /** 常用字段预设添加 (props 完整配置; 拖放同 onDropField) */
  onAddField: (preset: FieldPreset) => void
}

export const Palette: React.FC<PaletteProps> = ({ onAdd, onDrop, onAddField }) => {
  const [catalog, setCatalog] = useState<ComponentCatalogEntry[]>([])
  const [presets, setPresets] = useState<FieldPreset[]>([])
  /** 拖动 ghost 位置 (null = 未拖) */
  const [ghost, setGhost] = useState<{ label: string; x: number; y: number } | null>(null)
  const pending = useRef<{ entry: { typeName: string; displayZh: string; defaultProps?: Record<string, unknown> }; start: [number, number] } | null>(null)

  useEffect(() => {
    getComponentCatalog()
      .then((r: CatalogResponse) => {
        setCatalog(r.components ?? [])
        setPresets(r.fieldPresets ?? [])
      })
      .catch(() => {
        setCatalog([])
        setPresets([])
      })
  }, [])

  const groups = useMemo(() => {
    const m = new Map<string, ComponentCatalogEntry[]>()
    for (const e of catalog) {
      if (!m.has(e.category)) m.set(e.category, [])
      m.get(e.category)!.push(e)
    }
    return [...m.entries()]
  }, [catalog])

  // window 级 move/up (拖出 palette 后仍跟踪)
  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const p = pending.current
      if (!p) return
      const dist = Math.hypot(e.clientX - p.start[0], e.clientY - p.start[1])
      if (dist >= DRAG_THRESHOLD) setGhost({ label: p.entry.displayZh, x: e.clientX, y: e.clientY })
    }
    const onUp = (e: PointerEvent) => {
      const p = pending.current
      pending.current = null
      setGhost(null)
      if (!p) return
      const dist = Math.hypot(e.clientX - p.start[0], e.clientY - p.start[1])
      if (dist < DRAG_THRESHOLD) {
        onAdd(p.entry.typeName, p.entry.displayZh, p.entry.defaultProps) // 点击
      } else {
        onDrop(p.entry, e.clientX, e.clientY) // 拖放 (画布外 = LayoutTab 静默取消)
      }
    }
    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', onUp)
    return () => {
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerup', onUp)
    }
  }, [onAdd, onDrop])

  const startEntryDrag = (e: React.PointerEvent, entry: { typeName: string; displayZh: string; defaultProps?: Record<string, unknown> }) => {
    pending.current = { entry, start: [e.clientX, e.clientY] }
  }

  return (
    <div
      style={{
        width: 190,
        flexShrink: 0,
        overflowY: 'auto',
        borderRight: '1px solid #eee',
        paddingRight: 6,
      }}
    >
      {/* 常用字段 (出厂预设 — 表速/真空速/马赫数… 点一下就是完整组件) */}
      {presets.length > 0 && (
        <div style={{ marginBottom: 10 }}>
          <div style={{ fontSize: 12, color: '#888', margin: '6px 0 4px' }}>常用字段</div>
          {presets.map((p, i) => (
            <div
              key={`${p.label}-${i}`}
              onClick={() => onAddField(p)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                padding: '4px 6px',
                cursor: 'pointer',
                borderRadius: 4,
                fontSize: 13,
              }}
              className="palette-item"
              title={String(p.props.target ?? '')}
            >
              <span style={{ fontSize: 10, color: '#999' }}>＋</span>
              <span style={{ flex: 1 }}>{p.label}</span>
              {p.props.unit ? (
                <span style={{ fontSize: 10, color: '#999' }}>{String(p.props.unit)}</span>
              ) : null}
            </div>
          ))}
        </div>
      )}
      {groups.map(([cat, items]) => (
        <div key={cat} style={{ marginBottom: 10 }}>
          <div style={{ fontSize: 12, color: '#888', margin: '6px 0 4px' }}>
            {CATEGORY_ZH[cat] ?? cat}
          </div>
          {items.map(e => (
            <div
              key={e.typeName}
              onPointerDown={ev => startEntryDrag(ev, e)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                padding: '4px 6px',
                cursor: 'grab',
                borderRadius: 4,
                fontSize: 13,
                userSelect: 'none',
              }}
              className="palette-item"
              title={`${e.displayZh} (拖到画布放置)`}
            >
              <span style={{ fontSize: 10, color: '#999' }}>＋</span>
              <span style={{ flex: 1 }}>{e.displayZh}</span>
              {e.composite && (
                <Tag style={{ fontSize: 10, lineHeight: '16px', padding: '0 4px' }}>黑盒</Tag>
              )}
            </div>
          ))}
        </div>
      ))}
      {/* 拖放 ghost (跟随光标) */}
      {ghost && (
        <div
          style={{
            position: 'fixed',
            left: ghost.x + 10,
            top: ghost.y + 8,
            padding: '2px 8px',
            background: 'rgba(255,105,180,0.9)',
            color: '#fff',
            borderRadius: 4,
            fontSize: 12,
            pointerEvents: 'none',
            zIndex: 1000,
          }}
        >
          {ghost.label}
        </div>
      )}
      <style>{`
        .palette-item:hover { background: rgba(22,119,255,0.08); }
      `}</style>
    </div>
  )
}
