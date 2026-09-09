/**
 * 组件大纲 (左栏下段): 页文档序平铺, 缩进 = parent 链深度。
 * enabled=false 组件在画布上不渲染 (布局引擎不建节点) — 大纲是唯一寻回入口;
 * solve errors (类型未注册/工厂 Err) 的组件同样只在: 此处红点标注。
 */
import React, { useEffect, useMemo, useState } from 'react'
import { Tooltip } from 'antd'
import type { PageDoc } from './types'
import { getComponentCatalog } from './api'

interface OutlineProps {
  page: PageDoc
  /** 旧快照链遗留位 (R8 退役; errors 经会话推送) */
  solve?: unknown
  selectedIds: string[]
  onSelectionChange: (ids: string[]) => void
  /** 翻 enabled (大纲是禁用组件的唯一开关入口) */
  onToggleEnabled: (id: string, enabled: boolean) => void
}

/** parent 链深度 (环防御: 访问集截断) */
const depthOf = (page: PageDoc, id: string): number => {
  const seen = new Set<string>([id])
  let d = 0
  let cur = page.components.find(c => c.id === id)?.parent ?? null
  while (cur && !seen.has(cur)) {
    seen.add(cur)
    d++
    cur = page.components.find(c => c.id === cur)?.parent ?? null
  }
  return d
}

export const Outline: React.FC<OutlineProps> = ({
  page,
  solve,
  selectedIds,
  onSelectionChange,
  onToggleEnabled,
}) => {
  const [displayByName, setDisplayByName] = useState<Record<string, string>>({})

  useEffect(() => {
    getComponentCatalog()
      .then(r => {
        const m: Record<string, string> = {}
        for (const e of r.components ?? []) m[e.typeName] = e.displayZh
        setDisplayByName(m)
      })
      .catch(() => setDisplayByName({}))
  }, [])

  const errorById = useMemo(() => {
    const m: Record<string, string> = {}
    for (const [id, reason] of (solve as { errors?: [string, string][] } | null)?.errors ?? [])
      m[id] = reason
    return m
  }, [solve])

  const pick = (e: React.MouseEvent, id: string) => {
    if (e.ctrlKey || e.metaKey || e.shiftKey) {
      onSelectionChange(
        selectedIds.includes(id)
          ? selectedIds.filter(s => s !== id)
          : [...selectedIds, id],
      )
    } else {
      onSelectionChange([id])
    }
  }

  return (
    <div style={{ flex: 1, minHeight: 120, overflowY: 'auto' }}>
      <div style={{ fontSize: 12, color: '#888', margin: '6px 0 4px' }}>组件大纲</div>
      {page.components.map(c => {
        const sel = selectedIds.includes(c.id)
        const err = errorById[c.id]
        return (
          <div
            key={c.id}
            className="outline-row"
            onClick={e => pick(e, c.id)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              padding: '2px 4px',
              marginLeft: depthOf(page, c.id) * 12,
              cursor: 'pointer',
              borderRadius: 4,
              fontSize: 12,
              color: c.enabled ? undefined : '#bbb',
              background: sel ? 'rgba(255,105,180,0.10)' : undefined,
            }}
            title={err ?? c.type}
          >
            {/* eye 开关: 禁用组件的唯一寻回开关 */}
            <span
              onClick={e => {
                e.stopPropagation()
                onToggleEnabled(c.id, !c.enabled)
              }}
              style={{ fontSize: 11, width: 16, textAlign: 'center', color: c.enabled ? '#1677ff' : '#ccc', flexShrink: 0 }}
            >
              {c.enabled ? '👁' : '–'}
            </span>
            <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {c.id}
            </span>
            {err ? (
              <Tooltip title={err}>
                <span style={{ color: '#ff4d4f', fontSize: 10, flexShrink: 0 }}>●</span>
              </Tooltip>
            ) : (
              <span style={{ fontSize: 10, color: '#999', flexShrink: 0 }}>
                {displayByName[c.type] ?? c.type}
              </span>
            )}
          </div>
        )
      })}
      {page.components.length === 0 && (
        <div style={{ fontSize: 11, color: '#bbb', padding: '4px 6px' }}>暂无组件</div>
      )}
      <style>{`
        .outline-row:hover { background: rgba(255,105,180,0.06); }
      `}</style>
    </div>
  )
}
