/** W4 palette: 常用字段预设 (出厂数据字段, 点击即完整配置) + 组件目录分组 */
import React, { useEffect, useMemo, useState } from 'react'
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

interface PaletteProps {
  onAdd: (typeName: string, displayZh: string) => void
  /** 常用字段预设添加 (props 完整配置) */
  onAddField: (preset: FieldPreset) => void
}

export const Palette: React.FC<PaletteProps> = ({ onAdd, onAddField }) => {
  const [catalog, setCatalog] = useState<ComponentCatalogEntry[]>([])
  const [presets, setPresets] = useState<FieldPreset[]>([])

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
              onClick={() => onAdd(e.typeName, e.displayZh)}
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
              title={e.typeName}
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
      <style>{`
        .palette-item:hover { background: rgba(22,119,255,0.08); }
      `}</style>
    </div>
  )
}
