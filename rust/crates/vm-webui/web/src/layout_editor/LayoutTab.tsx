/**
 * W4 HUD 布局编辑器主组件 (三栏: palette / 画布 / inspector)。
 * 页面数据 (PageDoc) 在前端全量编辑, 100ms 防抖 solve_page 取 Rust 布局
 * 矩形 + PNG 底图 (布局求解唯一真相在 Rust); 保存走 save_page (delta)。
 * 桌面真窗的实时预览经既有 WYSIWYG 链 (save 后 CONFIG_CHANGED → reinit)。
 *
 * P0 数据流: 脏页集 per-page dirty / refreshList 显式调用+脏页保留 /
 * 删页自动落剩余页 / uid 递增查重 / 改名收口 (唯一性+parent 重指)。
 * C4 交互层: 多选 (selectedIds + 框选 + 成组拖) / 缩放 (zoom + fit) /
 * 键盘 (方向键微调 Shift 大步 / Delete / Escape)。
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Alert, Button, Popconfirm, Select, Space, message } from 'antd'
import type { ComponentDoc, PageDoc, PageSummary, SolveResult } from './types'
import {
  deletePage,
  getPages,
  resetPageToFactory,
  savePage,
  solvePage,
} from './api'
import { Palette } from './Palette'
import { Canvas } from './Canvas'
import { Inspector } from './Inspector'

/** 网格吸附步长 (line_height 单位) */
const SNAP = 0.1

/** 显示层预设 (覆盖 Rust defaultProps 同名键 — 展示更友好的初值) */
const PRESET_DEFAULT_PROPS: Record<string, Record<string, unknown>> = {
  'core.data.field': { target: 'ias', label: '表  速', unit: 'Km/h', precision: 0, previewValue: '500' },
  'core.fm.field': { key: 'weight.empty' },
  'core.fm.meta': { key: 'fm.version' },
}

export const LAYOUT_TAB_KEY = '__hud_layout__'

/** 页 id 生成 (时间36进制 + 随机段 — Date.now() 取模会碰撞) */
const newPageId = () =>
  `user-page-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`

/** 组件 id 生成: 序号递增至页内不撞 (数量+1 在删除过组件后会重复) */
const nextComponentId = (page: PageDoc, base: string) => {
  let n = page.components.length + 1
  while (page.components.some(c => c.id === `${base}-${n}`)) n++
  return `${base}-${n}`
}

const roundSnap = (v: number) => Math.round(v / SNAP) * SNAP

export const LayoutTab: React.FC = () => {
  const [pages, setPages] = useState<PageSummary[]>([])
  const [pageDocs, setPageDocs] = useState<Record<string, PageDoc>>({})
  const [activeId, setActiveId] = useState<string>('')
  const [selectedIds, setSelectedIds] = useState<string[]>([])
  const [solve, setSolve] = useState<SolveResult | null>(null)
  /** 脏页集 (per-page; 切页不丢、refreshList 不覆盖) */
  const [dirty, setDirty] = useState<Set<string>>(new Set())
  /** 首次加载完成 (区分 "加载中" 与 "无页面" 空态) */
  const [loaded, setLoaded] = useState(false)
  /** 画布缩放 (Canvas Ctrl+滚轮 / 工具栏) + fit 触发计数 */
  const [zoom, setZoom] = useState(1)
  const [fitTick, setFitTick] = useState(0)
  /** 升级提示已表态页 (本会话不再弹) */
  const [upgradeDismissed, setUpgradeDismissed] = useState<Set<string>>(new Set())
  const solveTimer = useRef<number>(0)
  /** refreshList 读 dirty 的桥 (避免 useCallback 依赖导致每次 dirty 变都重建) */
  const dirtyRef = useRef(dirty)
  dirtyRef.current = dirty

  const active = pageDocs[activeId] ?? null
  const activeUpgrade =
    pages.find(p => p.id === activeId)?.upgradeAvailable &&
    !upgradeDismissed.has(activeId)

  const markDirty = useCallback((id: string) => {
    setDirty(prev => (prev.has(id) ? prev : new Set(prev).add(id)))
  }, [])
  const clearDirty = useCallback((id: string) => {
    setDirty(prev => {
      if (!prev.has(id)) return prev
      const n = new Set(prev)
      n.delete(id)
      return n
    })
  }, [])

  /** 页面清单拉取 (mount + 显式刷新点; 未保存页保留本地版本) */
  const refreshList = useCallback(async () => {
    const { pages: list, docs } = await getPages()
    setPages(list)
    setPageDocs(prev => {
      const next: Record<string, PageDoc> = Object.fromEntries(docs.map(d => [d.id, d]))
      // 脏页保留本地编辑 (服务端数据是上次保存的旧版)
      for (const id of dirtyRef.current) if (prev[id]) next[id] = prev[id]
      return next
    })
    // 当前页消失 (删除/外部变更) → 落到剩余第一页, 不再卡死空态
    setActiveId(cur => (list.some(p => p.id === cur) ? cur : (list[0]?.id ?? '')))
    setLoaded(true)
  }, [])

  useEffect(() => {
    refreshList()
  }, [refreshList])

  // 切页清旧快照与选择 (避免上一页 PNG 闪帧/跨页选中)
  useEffect(() => {
    setSolve(null)
    setSelectedIds([])
  }, [activeId])

  // 防抖 solve
  useEffect(() => {
    if (!active) return
    window.clearTimeout(solveTimer.current)
    solveTimer.current = window.setTimeout(async () => {
      try {
        const r = await solvePage(active)
        setSolve(r)
      } catch (e) {
        message.error(`快照求解失败: ${e}`)
      }
    }, 100)
  }, [active])

  const patchPage = useCallback(
    (mut: (doc: PageDoc) => PageDoc) => {
      if (!active) return
      setPageDocs(prev => ({ ...prev, [activeId]: mut(prev[activeId]) }))
      markDirty(activeId)
    },
    [active, activeId, markDirty],
  )

  /** palette 点击添加 (画布中心落点, 吸附网格; Rust defaultProps 兜底) */
  const addComponent = useCallback(
    (typeName: string, displayZh: string, defaultProps?: Record<string, unknown>) => {
      if (!active) return
      const cx = roundSnap(active.components.length ? 2 : 1)
      // 初值 = Rust defaultProps (工厂必填项) ⊕ 显示层预设 (同名覆盖)
      const props = { ...(defaultProps ?? {}), ...(PRESET_DEFAULT_PROPS[typeName] ?? {}) }
      const comp: ComponentDoc = {
        id: nextComponentId(active, displayZh),
        type: typeName,
        pos: [cx, active.components.length * SNAP * 10],
        anchor: ['TopLeft', 'TopLeft'],
        parent: null,
        enabled: true,
        props,
      }
      patchPage(d => ({ ...d, components: [...d.components, comp] }))
      setSelectedIds([comp.id])
    },
    [active, patchPage],
  )

  /** palette 常用预设添加 (字段/引擎仪表; props 完整配置, 链式追加到页尾) */
  const addFieldPreset = useCallback(
    (preset: { label: string; props: Record<string, unknown> }) => {
      if (!active) return
      const type = preset.props.kind ? 'core.engine.gauge' : 'core.data.field'
      const idBase = String(preset.props.target ?? preset.props.kind ?? 'field')
      const comp: ComponentDoc = {
        id: nextComponentId(active, idBase),
        type,
        pos: [0, 0],
        anchor: ['TopLeft', 'BottomLeft'],
        parent: null, // 布局引擎: 父缺席退化根 — 链式改由用户在 Inspector 挂
        enabled: true,
        props: { ...preset.props },
      }
      patchPage(d => ({ ...d, components: [...d.components, comp] }))
      setSelectedIds([comp.id])
    },
    [active, patchPage],
  )

  const patchComponent = useCallback(
    (id: string, mut: (c: ComponentDoc) => ComponentDoc) => {
      patchPage(d => ({
        ...d,
        components: d.components.map(c => (c.id === id ? mut(c) : c)),
      }))
    },
    [patchPage],
  )

  /** 组件改名收口: 唯一性校验 + 其它组件 parent 引用重指 + 选中态联动。
   * 返回 false = 撞名拒绝 (Inspector 显示 error) */
  const renameComponent = useCallback(
    (oldId: string, newName: string): boolean => {
      if (!active || !newName || newName === oldId) return true
      if (active.components.some(c => c.id === newName)) return false
      patchPage(d => ({
        ...d,
        components: d.components.map(c =>
          c.id === oldId
            ? { ...c, id: newName }
            : { ...c, parent: c.parent === oldId ? newName : c.parent },
        ),
      }))
      setSelectedIds([newName])
      return true
    },
    [active, patchPage],
  )

  const removeComponents = useCallback(
    (ids: string[]) => {
      patchPage(d => ({
        ...d,
        components: d.components
          .filter(c => !ids.includes(c.id))
          // 悬空 parent 重指根 (布局引擎同样宽容退化, 这里显式落盘防困惑)
          .map(c => (c.parent && ids.includes(c.parent) ? { ...c, parent: null } : c)),
      }))
      setSelectedIds([])
    },
    [patchPage],
  )
  const removeComponent = useCallback((id: string) => removeComponents([id]), [removeComponents])

  const duplicateComponent = useCallback(
    (id: string) => {
      if (!active) return
      const src = active.components.find(c => c.id === id)
      if (!src) return
      const copy: ComponentDoc = {
        ...src,
        id: nextComponentId(active, src.id),
        pos: [src.pos[0] + SNAP * 5, src.pos[1] + SNAP * 5],
      }
      patchPage(d => ({ ...d, components: [...d.components, copy] }))
      setSelectedIds([copy.id])
    },
    [active, patchPage],
  )

  /** 画布拖拽落点 (画布 px → unit, 吸附; 成组位移) */
  const onDragCommit = useCallback(
    (ids: string[], dPx: [number, number]) => {
      if (!solve) return
      const lh = solve.lineHeightPx || 1
      patchPage(d => ({
        ...d,
        components: d.components.map(c =>
          ids.includes(c.id)
            ? {
                ...c,
                pos: [
                  roundSnap(c.pos[0] + dPx[0] / lh),
                  roundSnap(c.pos[1] + dPx[1] / lh),
                ],
              }
            : c,
        ),
      }))
    },
    [solve, patchPage],
  )

  /** 键盘: 方向键微调 (Shift 大步) / Delete 删除 / Escape 清选 */
  const nudge = useCallback(
    (dxUnit: number, dyUnit: number) => {
      patchPage(d => ({
        ...d,
        components: d.components.map(c =>
          selectedIds.includes(c.id)
            ? { ...c, pos: [roundSnap(c.pos[0] + dxUnit), roundSnap(c.pos[1] + dyUnit)] }
            : c,
        ),
      }))
    },
    [patchPage, selectedIds],
  )
  const onRootKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      // 表单控件聚焦时不拦截
      const t = e.target as HTMLElement
      if (t.closest('input, textarea, [contenteditable="true"]')) return
      if (e.key === 'Escape') {
        setSelectedIds([])
        return
      }
      if (!selectedIds.length || !active) return
      const step = e.shiftKey ? 1 : SNAP
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
          removeComponents(selectedIds)
          break
        default:
          break
      }
    },
    [selectedIds, active, nudge, removeComponents],
  )

  /** 新建/本地注入页 (标记脏 — 此前新页不置 dirty, 保存按钮恒禁用无法保存) */
  const upsertLocalPage = useCallback(
    (doc: PageDoc) => {
      setPageDocs(prev => ({ ...prev, [doc.id]: doc }))
      setActiveId(doc.id)
      setSelectedIds([])
      markDirty(doc.id)
    },
    [markDirty],
  )

  const onSave = useCallback(async () => {
    if (!active) return
    try {
      await savePage(active)
      clearDirty(activeId)
      message.success(`页面「${active.name}」已保存 (桌面预览窗实时更新)`)
    } catch (e) {
      message.error(`保存失败: ${e}`)
    }
  }, [active, activeId, clearDirty])

  /** 单选语义面 (Inspector); 多选批量栏 C5 接入 */
  const selected = useMemo(
    () =>
      selectedIds.length === 1
        ? active?.components.find(c => c.id === selectedIds[0]) ?? null
        : null,
    [active, selectedIds],
  )

  if (!loaded) {
    return <div style={{ padding: 24 }}>加载页面中…</div>
  }

  // 空态 (全部页面被删): 给恢复手段, 不再卡死
  if (!active) {
    return (
      <div style={{ padding: 24 }}>
        <p style={{ color: '#999' }}>没有页面了</p>
        <Button
          type="primary"
          onClick={() =>
            upsertLocalPage({
              id: newPageId(),
              name: '新页面',
              switchKey: null,
              pos: [0.5, 0.5],
              padding: 20,
              font: { family: '', sizeAdd: 0, scaleSource: '' },
              contentVersion: 0,
              components: [],
            })
          }
        >
          新建页
        </Button>
      </div>
    )
  }

  return (
    <div
      style={{ display: 'flex', gap: 8, height: '100%', minHeight: 480, outline: 'none' }}
      tabIndex={0}
      onKeyDown={onRootKeyDown}
    >
      {/* palette */}
      <Palette onAdd={addComponent} onAddField={addFieldPreset} />

      {/* 画布 + 工具栏 */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 6, minWidth: 0 }}>
        <Space wrap>
          <Select
            value={activeId}
            onChange={setActiveId}
            style={{ minWidth: 160 }}
            options={pages.map(p => ({
              value: p.id,
              label: `${p.name}${p.upgradeAvailable ? ' ⬆' : ''}`,
            }))}
          />
          <Button type="primary" disabled={!dirty.has(activeId)} onClick={onSave}>
            保存
          </Button>
          {activeUpgrade && (
            <Popconfirm
              title="出厂页有更新"
              description={`「${active.name}」的出厂版本已更新。采用新版将丢弃你对本页的修改 (可先复制页备份)，确定采用？`}
              okText="采用新版"
              cancelText="保留我的"
              onConfirm={async () => {
                await resetPageToFactory(activeId)
                clearDirty(activeId)
                message.success('已采用出厂新版')
                refreshList()
              }}
              onCancel={() =>
                setUpgradeDismissed(prev => new Set(prev).add(activeId))
              }
            >
              <Button>出厂有更新 ⬆</Button>
            </Popconfirm>
          )}
          <Popconfirm
            title="恢复出厂"
            description="丢弃对该页的全部修改?"
            onConfirm={async () => {
              await resetPageToFactory(activeId)
              clearDirty(activeId)
              message.success('已恢复出厂版本')
              refreshList()
            }}
          >
            <Button>恢复出厂</Button>
          </Popconfirm>
          <Popconfirm
            title="删除页面"
            description={`删除「${active.name}」?`}
            onConfirm={async () => {
              try {
                await deletePage(activeId)
                clearDirty(activeId)
                message.success('已删除')
                refreshList()
              } catch (e) {
                message.error(`${e}`)
              }
            }}
          >
            <Button danger disabled={pages.length <= 1}>
              删除页
            </Button>
          </Popconfirm>
          <Button onClick={() => {
            const copy: PageDoc = {
              ...active,
              id: newPageId(),
              name: `${active.name} 副本`,
            }
            savePage(copy).then(() => refreshList())
          }}>
            复制页
          </Button>
          <Button onClick={() => {
            upsertLocalPage({
              id: newPageId(),
              name: '新页面',
              switchKey: null,
              pos: [0.5, 0.5],
              padding: 20,
              font: { family: '', sizeAdd: 0, scaleSource: '' },
              contentVersion: 0,
              components: [],
            })
          }}>
            新建页
          </Button>
          {/* 缩放控制 (替代硬编码 ZOOM=1.6) */}
          <Space.Compact>
            <Button size="small" onClick={() => setZoom(z => Math.max(0.2, z / 1.2))}>−</Button>
            <Button size="small" style={{ pointerEvents: 'none', width: 52 }}>
              {Math.round(zoom * 100)}%
            </Button>
            <Button size="small" onClick={() => setZoom(z => Math.min(3, z * 1.2))}>＋</Button>
          </Space.Compact>
          <Button size="small" onClick={() => setFitTick(t => t + 1)}>适应画布</Button>
          <Button size="small" onClick={() => setZoom(1)}>100%</Button>
        </Space>
        {/* 构建错误回显 (类型未注册/工厂 Err — 此前仅 Rust warn 日志静默失败) */}
        {solve && solve.errors.length > 0 && (
          <Alert
            type="error"
            showIcon
            message={`${solve.errors.length} 个组件构建失败`}
            description={solve.errors.map(([id, reason]) => `「${id}」: ${reason}`).join('；')}
          />
        )}
        <Canvas
          solve={solve}
          page={active}
          selectedIds={selectedIds}
          onSelectionChange={setSelectedIds}
          onDragCommit={onDragCommit}
          zoom={zoom}
          onZoom={setZoom}
          fitTick={fitTick}
        />
      </div>

      {/* inspector */}
      <Inspector
        page={active}
        component={selected}
        onPatchPage={patchPage}
        onPatchComponent={(id, mut) => patchComponent(id, mut)}
        onRename={renameComponent}
        onRemove={removeComponent}
        onDuplicate={duplicateComponent}
      />
    </div>
  )
}
