/**
 * W4 inspector: 选中组件属性 (id/type/坐标/锚点/父/条件) + 页面属性。
 * 坐标单位 = line_height 倍数 (字号相对 — 改字号整页等比)。
 */
import React from 'react'
import { Button, Input, InputNumber, Select, Space, Switch } from 'antd'
import type { ComponentDoc, PageDoc } from './types'

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

/** 常用 visibleWhen 预设 (W4 简版; 高级模式自由输入) */
const VW_PRESETS = [
  { value: '', label: '总是显示' },
  { value: 'displayCrosshair', label: '准星开关 (displayCrosshair)' },
]

interface InspectorProps {
  page: PageDoc
  component: ComponentDoc | null
  onPatchPage: (mut: (doc: PageDoc) => PageDoc) => void
  onPatchComponent: (id: string, mut: (c: ComponentDoc) => ComponentDoc) => void
  onRemove: (id: string) => void
  onDuplicate: (id: string) => void
}

export const Inspector: React.FC<InspectorProps> = ({
  page,
  component,
  onPatchPage,
  onPatchComponent,
  onRemove,
  onDuplicate,
}) => {
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
        <Input value={component.id} onChange={e => patch(c => ({ ...c, id: e.target.value }))} />
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
      {component.type === 'core.fields.grid' && (
        <Field label="数据面板">
          <Select
            size="small"
            value={String((component.props as any).fieldSet ?? '')}
            options={[
              { value: '飞行信息', label: '飞行信息' },
              { value: '动力信息', label: '动力信息' },
            ]}
            onChange={v =>
              patch(c => ({ ...c, props: { ...c.props, fieldSet: v } }))
            }
          />
        </Field>
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
