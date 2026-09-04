/** W4 palette: 组件目录分组列表 (点击添加到画布) */
import React, { useEffect, useMemo, useState } from 'react'
import { Tag } from 'antd'
import type { ComponentCatalogEntry } from './types'
import { getComponentCatalog } from './api'

const CATEGORY_ZH: Record<string, string> = {
  Text: '文本',
  Gauge: '仪表',
  Chart: '图表',
  List: '列表',
  Composite: '复合 (黑盒)',
  Decor: '装饰',
}

export const Palette: React.FC<{ onAdd: (typeName: string, displayZh: string) => void }> = ({
  onAdd,
}) => {
  const [catalog, setCatalog] = useState<ComponentCatalogEntry[]>([])

  useEffect(() => {
    getComponentCatalog().then(setCatalog).catch(() => setCatalog([]))
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
