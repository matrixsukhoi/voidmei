/**
 * R7+ 编辑控制台 (真窗即画布形态的 MainForm 会话面板):
 * App 在编辑会话期间整体渲染本组件 (常规设置面板退场) — 画布 = 桌面真实
 * overlay 窗口 (点选/拖拽/resize/吸附全在真窗上), 本面板是控制台:
 * palette + 大纲 | 工具栏 (含页面管理: 目标页切换/新建/复制/删除/恢复出厂,
 * 全走编辑命令, 退出时统一提交) | inspector。
 * 数据源 = hud-edit-doc 事件推送 (80ms 节流镜像) + mount 时 get_pages 的
 * 页清单/出厂文档 (页面管理操作基底)。
 */
import React, { useCallback, useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { Alert, Button, Popconfirm, Select, Space, Switch, Tooltip, message } from 'antd'
import type { PageDoc } from './types'
import { getPages } from './api'
import { editCommand, endEditSession } from './editApi'
import { Palette } from './Palette'
import { Inspector } from './Inspector'
import { Outline } from './Outline'

/** 网格吸附步长 (行高倍) — 键盘微调与 palette 落点共用 */
const SNAP = 0.1

/** 显示层预设 (覆盖 Rust defaultProps 同名键) */
const PRESET_DEFAULT_PROPS: Record<string, Record<string, unknown>> = {
  'core.data.field': { target: 'ias', label: '表  速', unit: 'Km/h', precision: 0, previewValue: '500' },
  'core.fm.field': { key: 'weight.empty' },
  'core.fm.meta': { key: 'fm.version' },
}

const newPageId = () =>
  `user-page-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`

const nextComponentId = (page: PageDoc, base: string) => {
  let n = page.components.length + 1
  while (page.components.some(c => c.id === `${base}-${n}`)) n++
  return `${base}-${n}`
}

const roundSnap = (v: number) => Math.round(v / SNAP) * SNAP

const clone = <T,>(v: T): T => JSON.parse(JSON.stringify(v))

interface PageListEntry {
  id: string
  name: string
  isFactory: boolean
}

/** hud-edit-doc 推送载荷 */
interface EditDocPayload {
  targetPage: string
  page: PageDoc
  items: { id: string; x: number; y: number; w: number; h: number }[]
  lineHeightPx: number
  selection: string[]
  errors: [string, string][]
}

const EMPTY_PAGE: PageDoc = {
  id: '',
  name: '',
  activation: null,
  pos: [0.5, 0.5],
  padding: 45,
  font: { sizeAdd: 0 },
  contentVersion: 0,
  components: [],
}

export const LayoutTab: React.FC = () => {
  /** 编辑镜像 (hud-edit-doc 推送) */
  const [doc, setDoc] = useState<PageDoc | null>(null)
  const [items, setItems] = useState<EditDocPayload['items']>([])
  const [selection, setSelection] = useState<string[]>([])
  const [errors, setErrors] = useState<[string, string][]>([])
  const [snapping, setSnapping] = useState(true)
  const [showGuides, setShowGuides] = useState(true)
  const [undoDepth, setUndoDepth] = useState(0)
  const [redoDepth, setRedoDepth] = useState(0)
  /** 页清单 + 出厂文档 (页面管理操作基底; 会话内命令后本地同步) */
  const [pageList, setPageList] = useState<PageListEntry[]>([])
  const factoryDocs = useRef<Map<string, PageDoc>>(new Map())
  const undoStack = useRef<PageDoc[]>([])
  const redoStack = useRef<PageDoc[]>([])

  // 页清单/出厂文档拉取 (mount = 会话开始, 一次)
  useEffect(() => {
    getPages().then(({ pages, docs, factoryDocs: factory }) => {
      setPageList(pages.map(p => ({ id: p.id, name: p.name, isFactory: p.isFactory })))
      const m = new Map<string, PageDoc>()
      for (const d of factory ?? []) m.set(d.id, d)
      factoryDocs.current = m
      // 恢复出厂/复制需要完整文档 — docs 全量备查
      allDocs.current = new Map(docs.map(d => [d.id, d]))
    }).catch(e => message.error(`页清单拉取失败: ${e}`))
  }, [])
  const allDocs = useRef<Map<string, PageDoc>>(new Map())

  // ---- 会话事件监听 (渲染线程 UIStateBus 桥) ----
  useEffect(() => {
    let un2: (() => void) | undefined
    let un3: (() => void) | undefined
    let un4: (() => void) | undefined
    {
      listen<string>('hud-edit-doc', e => {
        try {
          const p = JSON.parse(e.payload) as EditDocPayload
          setDoc(p.page)
          setItems(p.items)
          setSelection(p.selection)
          setErrors(p.errors ?? [])
        } catch {
          /* 载荷异常忽略 */
        }
      }).then(u => (un2 = u))
      listen<string>('hud-edit-selection', e => {
        try {
          const p = JSON.parse(e.payload) as { ids: string[] }
          setSelection(p.ids ?? [])
        } catch {
          /* ignore */
        }
      }).then(u => (un3 = u))
      listen<string>('hud-edit-error', e => {
        message.error(e.payload || '编辑命令失败')
      }).then(u => (un4 = u))
    }
    return () => {
      un2?.()
      un3?.()
      un4?.()
    }
  }, [])

  /** 命令发送 (带撤销快照) */
  const sendCmd = useCallback(
    (kind: string, rest: Record<string, unknown>, undoable = true) => {
      if (undoable && doc) {
        undoStack.current.push(clone(doc))
        if (undoStack.current.length > 50) undoStack.current.shift()
        redoStack.current = []
        setUndoDepth(undoStack.current.length)
        setRedoDepth(0)
      }
      editCommand({ kind, ...rest }).catch(e => message.error(`${e}`))
    },
    [doc],
  )

  const onUndo = useCallback(() => {
    const prev = undoStack.current.pop()
    if (prev && doc) {
      redoStack.current.push(clone(doc))
      sendCmd('updatePage', { page: prev }, false)
      setUndoDepth(undoStack.current.length)
      setRedoDepth(redoStack.current.length)
    }
  }, [doc, sendCmd])

  const onRedo = useCallback(() => {
    const next = redoStack.current.pop()
    if (next && doc) {
      undoStack.current.push(clone(doc))
      sendCmd('updatePage', { page: next }, false)
      setUndoDepth(undoStack.current.length)
      setRedoDepth(redoStack.current.length)
    }
  }, [doc, sendCmd])

  // ---- 键盘 (方向键微调/Delete/Ctrl+Z; 画布交互在真窗上, 键盘在面板) ----
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement
      if (t.closest('input, textarea, [contenteditable="true"]')) return
      const ctrl = e.ctrlKey || e.metaKey
      if (ctrl && (e.key === 'z' || e.key === 'Z')) {
        e.preventDefault()
        if (e.shiftKey) onRedo()
        else onUndo()
        return
      }
      if (!selection.length || !doc) return
      const step = e.shiftKey ? 1 : SNAP
      const nudge = (dx: number, dy: number) =>
        sendCmd('nudge', { ids: selection, dUnit: [dx, dy] })
      switch (e.key) {
        case 'ArrowLeft':
          e.preventDefault()
          nudge(-step, 0)
          break
        case 'ArrowRight':
          e.preventDefault()
          nudge(step, 0)
          break
        case 'ArrowUp':
          e.preventDefault()
          nudge(0, -step)
          break
        case 'ArrowDown':
          e.preventDefault()
          nudge(0, step)
          break
        case 'Delete':
        case 'Backspace':
          e.preventDefault()
          sendCmd('removeComponents', { ids: selection })
          break
        default:
          break
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [selection, doc, sendCmd, onUndo, onRedo])

  const onEndEdit = useCallback(async (commit: boolean) => {
    try {
      await endEditSession(commit)
      if (!commit) message.info('已放弃修改')
    } catch (e) {
      message.error(`${e}`)
    }
  }, [])

  // ---- 页面管理 (会话内命令, 退出时统一提交) ----
  const upsertPage = useCallback((page: PageDoc, target: boolean) => {
    setPageList(prev =>
      prev.some(p => p.id === page.id)
        ? prev.map(p => (p.id === page.id ? { id: page.id, name: page.name, isFactory: false } : p))
        : [...prev, { id: page.id, name: page.name, isFactory: false }],
    )
    allDocs.current.set(page.id, page)
    sendCmd('upsertPage', { page }, false)
    if (target) sendCmd('setTargetPage', { pageId: page.id }, false)
  }, [sendCmd])

  const targetName = doc?.name ?? ''

  const selected =
    selection.length === 1
      ? doc?.components.find(c => c.id === selection[0]) ?? null
      : null

  const patchPage = useCallback(
    (mut: (d: PageDoc) => PageDoc) => {
      if (!doc) return
      sendCmd('updatePage', { page: mut(doc) })
    },
    [doc, sendCmd],
  )

  const patchComponent = useCallback(
    (id: string, mut: (c: ComponentDocT) => ComponentDocT) => {
      if (!doc) return
      const cur = doc.components.find(c => c.id === id)
      if (cur) sendCmd('updateComponent', { oldId: id, comp: mut(cur) })
    },
    [doc, sendCmd],
  )

  const addComponent = useCallback(
    (typeName: string, displayZh: string, defaultProps?: Record<string, unknown>) => {
      if (!doc) return
      const cx = roundSnap(doc.components.length ? 2 : 1)
      const props = { ...(defaultProps ?? {}), ...(PRESET_DEFAULT_PROPS[typeName] ?? {}) }
      const comp = {
        id: nextComponentId(doc, displayZh),
        type: typeName,
        pos: [cx, doc.components.length * SNAP * 10],
        anchor: ['TopLeft', 'TopLeft'],
        parent: null,
        enabled: true,
        size: null,
        props,
      }
      sendCmd('updateComponent', { oldId: comp.id, comp })
    },
    [doc, sendCmd],
  )

  const addFieldPreset = useCallback(
    (preset: { label: string; props: Record<string, unknown> }) => {
      if (!doc) return
      const type = preset.props.kind ? 'core.engine.gauge' : 'core.data.field'
      const idBase = String(preset.props.target ?? preset.props.kind ?? 'field')
      const comp = {
        id: nextComponentId(doc, idBase),
        type,
        pos: [0, 0],
        anchor: ['TopLeft', 'BottomLeft'],
        parent: null,
        enabled: true,
        size: null,
        props: { ...preset.props },
      }
      sendCmd('updateComponent', { oldId: comp.id, comp })
    },
    [doc, sendCmd],
  )

  return (
    <div style={{ display: 'flex', gap: 8, height: '100%', minHeight: 480, outline: 'none' }}>
      {/* 左: palette + 大纲 */}
      <div style={{ width: 190, flexShrink: 0, display: 'flex', flexDirection: 'column' }}>
        <Palette onAdd={addComponent} onAddField={addFieldPreset} />
        <Outline
          page={doc ?? EMPTY_PAGE}
          solve={{ errors }}
          selectedIds={selection}
          onSelectionChange={ids => sendCmd('select', { ids }, false)}
          onToggleEnabled={(id, enabled) => {
            const cur = doc?.components.find(c => c.id === id)
            if (cur) sendCmd('updateComponent', { oldId: id, comp: { ...cur, enabled } })
          }}
        />
      </div>

      {/* 中: 工具栏 + 状态条 (画布 = 桌面真窗) */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 6, minWidth: 0 }}>
        <Space wrap>
          <Button type="primary" onClick={() => onEndEdit(true)}>
            保存并退出
          </Button>
          <Popconfirm title="放弃修改" description="丢弃本次全部编辑?" onConfirm={() => onEndEdit(false)}>
            <Button danger>放弃</Button>
          </Popconfirm>
          <Button disabled={undoDepth === 0} onClick={onUndo} title="Ctrl+Z">
            撤销
          </Button>
          <Button disabled={redoDepth === 0} onClick={onRedo} title="Ctrl+Shift+Z">
            重做
          </Button>
          <Space size={4}>
            <Tooltip title="拖动组件时吸附网格与对齐线">
              <span style={{ fontSize: 12 }}>吸附</span>
            </Tooltip>
            <Switch
              size="small"
              checked={snapping}
              onChange={v => {
                setSnapping(v)
                sendCmd('setOptions', { snapping: v }, false)
              }}
            />
            <Tooltip title="显示对齐参考线">
              <span style={{ fontSize: 12 }}>参考线</span>
            </Tooltip>
            <Switch
              size="small"
              checked={showGuides}
              onChange={v => {
                setShowGuides(v)
                sendCmd('setOptions', { showGuides: v }, false)
              }}
            />
          </Space>
        </Space>
        {/* 页面管理: 目标页切换 + 新建/复制/恢复出厂/删除 (全会话内命令) */}
        <Space wrap>
          <Select
            value={doc?.id}
            style={{ minWidth: 180 }}
            placeholder="目标页"
            onChange={(id: string) => sendCmd('setTargetPage', { pageId: id }, false)}
            options={pageList
              .filter(p => p.id !== 'minihud-default') /* 专用编排器页不可编辑 */
              .map(p => ({
                value: p.id,
                label: `${p.name}${p.isFactory ? ' (出厂)' : ''}`,
              }))}
          />
          <Button
            size="small"
            onClick={() => {
              const page: PageDoc = {
                ...EMPTY_PAGE,
                id: newPageId(),
                name: `新页面 ${pageList.filter(p => !p.isFactory).length + 1}`,
              }
              upsertPage(page, true)
            }}
          >
            新建页
          </Button>
          <Button
            size="small"
            disabled={!doc}
            onClick={() => {
              if (!doc) return
              upsertPage(
                { ...clone(doc), id: newPageId(), name: `${doc.name} 副本` },
                true,
              )
            }}
          >
            复制页
          </Button>
          <Button
            size="small"
            disabled={!doc || !factoryDocs.current.has(doc.id)}
            onClick={() => {
              if (!doc) return
              const factoryDoc = factoryDocs.current.get(doc.id)
              if (!factoryDoc) return
              sendCmd('updatePage', { page: clone(factoryDoc) })
              message.info(`「${doc.name}」已恢复出厂内容 (退出时落盘)`)
            }}
          >
            恢复出厂
          </Button>
          <Popconfirm
            title="删除页面"
            description={`删除「${targetName}」? (退出编辑时生效)`}
            onConfirm={() => {
              if (!doc) return
              const id = doc.id
              setPageList(prev => prev.filter(p => p.id !== id))
              sendCmd('deletePage', { pageId: id }, false)
            }}
          >
            <Button size="small" danger disabled={!doc || pageList.length <= 1}>
              删除页
            </Button>
          </Popconfirm>
        </Space>
        <Alert
          type="info"
          showIcon
          message={`正在编辑「${targetName}」— 画布就是桌面上的 HUD 窗口`}
          description="点选组件、拖动移动、拖角调整大小; 方向键微调 (Shift 大步) / Delete 删除 / Ctrl+Z 撤销; 空白处拖动 = 移动整窗位置; 点击其它 HUD 窗口切换目标页"
        />
        {errors.length > 0 && (
          <Alert
            type="error"
            showIcon
            message={`${errors.length} 个组件构建失败`}
            description={errors.map(([id, reason]) => `「${id}」: ${reason}`).join('；')}
          />
        )}
        <div style={{ color: '#999', fontSize: 12 }}>
          {items.length > 0 && `${items.length} 组件 · 选中 ${selection.length} · 页面改动退出时统一保存`}
        </div>
      </div>

      {/* 右: 属性面板 */}
      <Inspector
        page={doc ?? EMPTY_PAGE}
        component={selected}
        selectedIds={selection}
        onPatchPage={patchPage}
        onPatchComponent={patchComponent}
        onRename={(oldId, newName) => {
          if (!doc) return true
          const c = doc.components.find(x => x.id === oldId)
          if (!c) return true
          if (newName !== oldId && doc.components.some(x => x.id === newName)) return false
          sendCmd('updateComponent', { oldId: oldId, comp: { ...c, id: newName } })
          return true
        }}
        onRemove={id => sendCmd('removeComponents', { ids: [id] })}
        onRemoveMany={ids => sendCmd('removeComponents', { ids })}
        onDuplicate={id => {
          if (!doc) return
          const src = doc.components.find(c => c.id === id)
          if (!src) return
          sendCmd('updateComponent', {
            oldId: undefined,
            comp: {
              ...src,
              id: nextComponentId(doc, src.id),
              pos: [src.pos[0] + SNAP * 5, src.pos[1] + SNAP * 5],
            },
          })
        }}
        onAlign={(ids, kind) => {
          // 对齐: 前端按矩形镜像算 delta → 逐组件 nudge (Rust 侧吸附收尾)
          const rects = items.filter(it => ids.includes(it.id))
          if (rects.length < 2 || !doc) return
          const box = {
            x: Math.min(...rects.map(r => r.x)),
            y: Math.min(...rects.map(r => r.y)),
            r: Math.max(...rects.map(r => r.x + r.w)),
            b: Math.max(...rects.map(r => r.y + r.h)),
          }
          for (const it of rects) {
            const cx = it.x + it.w / 2
            const cy = it.y + it.h / 2
            let d: [number, number] | null = null
            switch (kind) {
              case 'left':
                d = [box.x - it.x, 0]
                break
              case 'top':
                d = [0, box.y - it.y]
                break
              case 'hcenter':
                d = [(box.x + box.r) / 2 - cx, 0]
                break
              case 'vcenter':
                d = [0, (box.y + box.b) / 2 - cy]
                break
            }
            if (d) sendCmd('nudge', { ids: [it.id], dUnit: d })
          }
        }}
      />
    </div>
  )
}

// 局部类型别名 (组件文档 — 与 types.ts ComponentDoc 同构, 减 import 面)
type ComponentDocT = import('./types').ComponentDoc
