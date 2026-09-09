/**
 * W4 inspector: 选中组件属性 (id/坐标/锚点/父/条件) + props 表单
 * (Rust propsSchema 驱动: Str/Int/Bool/Enum/Target — Target = 公式目录
 * 下拉可自由输入) + 页面属性。坐标单位 = line_height 倍数 (字号相对)。
 */
import React, { useEffect, useMemo, useState } from 'react'
import { AutoComplete, Button, Input, InputNumber, Select, Space, Switch } from 'antd'
import type { ComponentDoc, PageDoc, PropSchemaEntry } from './types'
import { getComponentCatalog } from './api'
import { getVarCatalog } from '../api'

const ANCHORS = [
  'TopLeft',
  'TopCenter',
  'TopRight',
  'MiddleLeft',
  'Center',
  'MiddleRight',
  'BottomLeft',
  'BottomCenter',
  'BottomRight',
]

/** 常用 visibleWhen 预设 (布局级条件; 数据级条件在组件 props) */
const VW_PRESETS = [
  { value: '', label: '总是显示' },
  { value: 'displayCrosshair', label: '准星开关 (displayCrosshair)' },
]

interface InspectorProps {
  page: PageDoc
  component: ComponentDoc | null
  onPatchPage: (mut: (doc: PageDoc) => PageDoc) => void
  onPatchComponent: (id: string, mut: (c: ComponentDoc) => ComponentDoc) => void
  /** 改名收口 (唯一性校验 + parent 重指); 返回 false = 撞名拒绝 */
  onRename: (oldId: string, newName: string) => boolean
  onRemove: (id: string) => void
  onDuplicate: (id: string) => void
}

export const Inspector: React.FC<InspectorProps> = ({
  page,
  component,
  onPatchPage,
  onPatchComponent,
  onRename,
  onRemove,
  onDuplicate,
}) => {
  const [schema, setSchema] = useState<Record<string, PropSchemaEntry[]>>({})
  const [varNames, setVarNames] = useState<{ value: string; label: string }[]>([])
  const [unitByName, setUnitByName] = useState<Record<string, string>>({})
  /** id 改名本地草稿 (onBlur/Enter 提交 → onRename 收口; 撞名回显 error) */
  const [idDraft, setIdDraft] = useState<string | null>(null)
  const [idError, setIdError] = useState<string | null>(null)

  // 组件切换时清草稿
  useEffect(() => {
    setIdDraft(null)
    setIdError(null)
  }, [component?.id])

  const submitRename = () => {
    if (idDraft == null || !component) return
    const name = idDraft.trim()
    if (!name || name === component.id) {
      setIdDraft(null)
      setIdError(null)
      return
    }
    if (!onRename(component.id, name)) {
      setIdError(`id「${name}」已存在`)
      return
    }
    setIdDraft(null)
    setIdError(null)
  }

  // 目录一次拉取 (组件类型 → propsSchema)
  useEffect(() => {
    getComponentCatalog()
      .then(r => {
        const m: Record<string, PropSchemaEntry[]> = {}
        for (const e of r.components ?? []) m[e.typeName] = e.propsSchema ?? []
        setSchema(m)
      })
      .catch(() => setSchema({}))
  }, [])

  // 公式目录 (Target 下拉数据源 + unit 自动带出表; 懒加载一次)
  useEffect(() => {
    getVarCatalog()
      .then(vs => {
        setVarNames(
          vs.map(v => ({
            value: v.name,
            label: v.unit ? `${v.name} (${v.unit})` : v.name,
          })),
        )
        const u: Record<string, string> = {}
        for (const v of vs) if (v.unit) u[v.name] = v.unit
        setUnitByName(u)
      })
      .catch(() => setVarNames([]))
  }, [])

  const activeSchema = useMemo(
    () => (component ? schema[component.type] ?? [] : []),
    [schema, component],
  )

  if (!component) {
    return (
      <div style={{ width: 280, flexShrink: 0, overflowY: 'auto', paddingLeft: 8 }}>
        <SectionTitle>页面属性</SectionTitle>
        <Field label="名称">
          <Input
            value={page.name}
            onChange={e => onPatchPage(d => ({ ...d, name: e.target.value }))}
          />
        </Field>
        <Field label="页面 id">
          <Input value={page.id} disabled />
        </Field>
        <Field label="开关键">
          <Input
            value={page.switchKey ?? ''}
            placeholder="空 = 恒显"
            onChange={e =>
              onPatchPage(d => ({ ...d, switchKey: e.target.value || null }))
            }
          />
        </Field>
        <Field label="包围盒留白">
          <InputNumber
            value={page.padding}
            onChange={v => onPatchPage(d => ({ ...d, padding: v ?? 0 }))}
          />
        </Field>
        <Field label="组件数">
          <span>{page.components.length}</span>
        </Field>
        <p style={{ fontSize: 12, color: '#999' }}>
          点击画布中的组件编辑其属性; 拖拽移动 (网格吸附 0.1 行高)。
        </p>
      </div>
    )
  }

  const patch = (mut: (c: ComponentDoc) => ComponentDoc) =>
    onPatchComponent(component.id, mut)

  const patchProp = (key: string, value: unknown) =>
    patch(c => ({ ...c, props: { ...c.props, [key]: value } }))

  return (
    <div style={{ width: 280, flexShrink: 0, overflowY: 'auto', paddingLeft: 8 }}>
      <SectionTitle>组件: {component.id}</SectionTitle>
      <Space style={{ marginBottom: 8 }}>
        <Button size="small" onClick={() => onDuplicate(component.id)}>
          复制
        </Button>
        <Button size="small" danger onClick={() => onRemove(component.id)}>
          删除
        </Button>
      </Space>
      <Field label="id">
        <Input
          value={idDraft ?? component.id}
          status={idError ? 'error' : undefined}
          onChange={e => {
            setIdDraft(e.target.value)
            setIdError(null)
          }}
          onBlur={submitRename}
          onPressEnter={submitRename}
          placeholder="改名 (Enter 提交; 子组件引用自动跟随)"
        />
        {idError && (
          <div style={{ fontSize: 11, color: '#ff4d4f', marginTop: 2 }}>{idError}</div>
        )}
      </Field>
      <Field label="类型">
        <Input value={component.type} disabled />
      </Field>
      <Field label="启用">
        <Switch
          size="small"
          checked={component.enabled}
          onChange={v => patch(c => ({ ...c, enabled: v }))}
        />
      </Field>
      <Field label="X (行高倍)">
        <InputNumber
          step={0.1}
          value={component.pos[0]}
          onChange={v => patch(c => ({ ...c, pos: [v ?? 0, c.pos[1]] }))}
        />
      </Field>
      <Field label="Y (行高倍)">
        <InputNumber
          step={0.1}
          value={component.pos[1]}
          onChange={v => patch(c => ({ ...c, pos: [c.pos[0], v ?? 0] }))}
        />
      </Field>
      <Field label="自身锚点">
        <Select
          size="small"
          value={component.anchor[0]}
          options={ANCHORS.map(a => ({ value: a }))}
          onChange={v => patch(c => ({ ...c, anchor: [v, c.anchor[1]] }))}
        />
      </Field>
      <Field label="父锚点">
        <Select
          size="small"
          value={component.anchor[1]}
          options={ANCHORS.map(a => ({ value: a }))}
          onChange={v => patch(c => ({ ...c, anchor: [c.anchor[0], v] }))}
        />
      </Field>
      <Field label="父组件">
        <Select
          size="small"
          allowClear
          value={component.parent ?? undefined}
          options={page.components
            .filter(c => c.id !== component.id)
            .map(c => ({ value: c.id, label: c.id }))}
          onChange={v => patch(c => ({ ...c, parent: v ?? null }))}
        />
      </Field>
      <Field label="显示条件">
        <Select
          size="small"
          value={VW_PRESETS.some(p => p.value === (component.visibleWhen ?? ''))
            ? (component.visibleWhen ?? '')
            : '__custom__'}
          options={[
            ...VW_PRESETS,
            { value: '__custom__', label: '自定义…' },
          ]}
          onChange={v =>
            patch(c => ({ ...c, visibleWhen: v === '__custom__' ? (c.visibleWhen ?? '') : v || null }))
          }
        />
      </Field>
      {component.visibleWhen && (
        <Field label="条件表达式">
          <Input
            size="small"
            value={component.visibleWhen}
            placeholder="displayCrosshair / value > 0 && !isJetEngine"
            onChange={e => patch(c => ({ ...c, visibleWhen: e.target.value || null }))}
          />
        </Field>
      )}

      {/* props 表单 (schema 驱动; 黑盒组件空表) */}
      {activeSchema.length > 0 && (
        <>
          <SectionTitle>组件属性</SectionTitle>
          {activeSchema.map(p => {
            const v = (component.props as Record<string, unknown>)[p.key]
            switch (p.kind) {
              case 'Target':
                return (
                  <Field key={p.key} label={p.displayZh}>
                    <AutoComplete
                      size="small"
                      value={typeof v === 'string' ? v : ''}
                      options={varNames}
                      placeholder="变量短名 / 公式名 / X * N"
                      filterOption={(input, opt) =>
                        (opt?.value ?? '').toLowerCase().includes(input.toLowerCase())
                      }
                      onChange={val => {
                        patch(c => {
                          const props = { ...c.props, [p.key]: val }
                          // 选中目录变量且单位非空 → 自动带出 (仅未手填时)
                          const u = unitByName[val]
                          if (u && !props.unit) props.unit = u
                          return { ...c, props }
                        })
                      }}
                    />
                  </Field>
                )
              case 'Int':
                return (
                  <Field key={p.key} label={p.displayZh}>
                    <InputNumber
                      size="small"
                      value={typeof v === 'number' ? v : 0}
                      onChange={n => patchProp(p.key, n ?? 0)}
                    />
                  </Field>
                )
              case 'Bool':
                return (
                  <Field key={p.key} label={p.displayZh}>
                    <Switch
                      size="small"
                      checked={v === true}
                      onChange={b => patchProp(p.key, b)}
                    />
                  </Field>
                )
              case 'Enum':
                return (
                  <Field key={p.key} label={p.displayZh}>
                    <Select
                      size="small"
                      value={typeof v === 'string' && v ? v : (p.values?.[0] ?? '')}
                      options={(p.values ?? []).map(s => ({ value: s }))}
                      onChange={s => patchProp(p.key, s)}
                    />
                  </Field>
                )
              default:
                // Str / Color (Color 本期无组件使用, 同文本输入)
                return (
                  <Field key={p.key} label={p.displayZh}>
                    <Input
                      size="small"
                      value={typeof v === 'string' ? v : ''}
                      onChange={e => patchProp(p.key, e.target.value)}
                    />
                  </Field>
                )
            }
          })}
        </>
      )}
    </div>
  )
}

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontWeight: 600, margin: '4px 0 10px', fontSize: 13 }}>{children}</div>
)

const Field: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
    <span style={{ width: 72, fontSize: 12, color: '#666', flexShrink: 0 }}>{label}</span>
    <div style={{ flex: 1, minWidth: 0 }}>{children}</div>
  </div>
)
