/**
 * W4 HUD 布局编辑器主组件 (三栏: palette / 画布 / inspector)。
 * 页面数据 (PageDoc) 在前端全量编辑, 100ms 防抖 solve_page 取 Rust 布局
 * 矩形 + PNG 底图 (布局求解唯一真相在 Rust); 保存走 save_page (delta)。
 * 桌面真窗的实时预览经既有 WYSIWYG 链 (save 后 CONFIG_CHANGED → reinit)。
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Button, Popconfirm, Select, Space, message } from 'antd'
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

export const LAYOUT_TAB_KEY = '__hud_layout__'

export const LayoutTab: React.FC = () => {
  const [pages, setPages] = useState<PageSummary[]>([])
  const [pageDocs, setPageDocs] = useState<Record<string, PageDoc>>({})
  const [activeId, setActiveId] = useState<string>('')
  const [selectedComp, setSelectedComp] = useState<string>('')
  const [solve, setSolve] = useState<SolveResult | null>(null)
  const [dirty, setDirty] = useState(false)
  const solveTimer = useRef<number>(0)

  const active = pageDocs[activeId] ?? null

  const refreshList = useCallback(async () => {
    const { pages: list, docs } = await getPages()
    setPages(list)
    setPageDocs(Object.fromEntries(docs.map(d => [d.id, d])))
    if (!activeId && list.length) setActiveId(list[0].id)
  }, [activeId])

  useEffect(() => {
    refreshList()
  }, [refreshList])

  /** 页面文档加载 (编辑器内全量编辑) — solve_page 顺带取文档? Rust 只回快照;
   * 文档由 save_page 往返维护: 首次进入用 pages() 的 id 逐个 solve 无法取文档。
   * 改法: get_pages 返回文档本体 (Rust 侧 pages() 已含全量)。 */
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
      setDirty(true)
    },
    [active, activeId],
  )

  /** palette 点击添加 (画布中心落点, 吸附网格) */
  const addComponent = useCallback(
    (typeName: string, displayZh: string) => {
      if (!active) return
      const cx = Math.round((active.components.length ? 2 : 1) / SNAP) * SNAP
      // data.field 默认绑定 ias (可用初值 — 空 target 工厂 Err 组件不显示)
      const defaultProps: Record<string, unknown> =
        typeName === 'core.data.field'
          ? { target: 'ias', label: '表  速', unit: 'Km/h', precision: 0, previewValue: '500' }
          : {}
      const comp: ComponentDoc = {
        id: `${displayZh}-${active.components.length + 1}`,
        type: typeName,
        pos: [cx, active.components.length * SNAP * 10],
        anchor: ['TopLeft', 'TopLeft'],
        parent: null,
        enabled: true,
        props: defaultProps,
      }
      patchPage(d => ({ ...d, components: [...d.components, comp] }))
      setSelectedComp(comp.id)
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

  const removeComponent = useCallback(
    (id: string) => {
      patchPage(d => ({
        ...d,
        components: d.components.filter(c => c.id !== id),
      }))
      setSelectedComp('')
    },
    [patchPage],
  )

  const duplicateComponent = useCallback(
    (id: string) => {
      if (!active) return
      const src = active.components.find(c => c.id === id)
      if (!src) return
      const copy: ComponentDoc = {
        ...src,
        id: `${src.id}-copy`,
        pos: [src.pos[0] + SNAP * 5, src.pos[1] + SNAP * 5],
      }
      patchPage(d => ({ ...d, components: [...d.components, copy] }))
      setSelectedComp(copy.id)
    },
    [active, patchPage],
  )

  /** 画布拖拽落点 (px → unit; 吸附) */
  const onDragComponent = useCallback(
    (id: string, dPx: [number, number]) => {
      if (!solve) return
      const lh = solve.lineHeightPx || 1
      patchComponent(id, c => ({
        ...c,
        pos: [
          Math.round((c.pos[0] + dPx[0] / lh) / SNAP) * SNAP,
          Math.round((c.pos[1] + dPx[1] / lh) / SNAP) * SNAP,
        ],
      }))
    },
    [solve, patchComponent],
  )

  const onSave = useCallback(async () => {
    if (!active) return
    try {
      await savePage(active)
      setDirty(false)
      message.success(`页面「${active.name}」已保存 (桌面预览窗已实时更新)`)
    } catch (e) {
      message.error(`保存失败: ${e}`)
    }
  }, [active])

  const selected = useMemo(
    () => active?.components.find(c => c.id === selectedComp) ?? null,
    [active, selectedComp],
  )

  if (!active) {
    return <div style={{ padding: 24 }}>加载页面中…</div>
  }

  return (
    <div style={{ display: 'flex', gap: 8, height: 'calc(100vh - 132px)', minHeight: 480 }}>
      {/* palette */}
      <Palette onAdd={addComponent} />

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
          <Button type="primary" disabled={!dirty} onClick={onSave}>
            保存
          </Button>
          <Popconfirm
            title="恢复出厂"
            description="丢弃对该页的全部修改?"
            onConfirm={async () => {
              await resetPageToFactory(activeId)
              message.success('已恢复出厂版本')
            }}
          >
            <Button >恢复出厂</Button>
          </Popconfirm>
          <Popconfirm
            title="删除页面"
            description={`删除「${active.name}」?`}
            onConfirm={async () => {
              try {
                await deletePage(activeId)
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
              id: `${active.id}-copy-${Date.now() % 1000}`,
              name: `${active.name} 副本`,
            }
            setPageDocs(prev => ({ ...prev, [copy.id]: copy }))
            savePage(copy).then(() => refreshList())
          }}>
            复制页
          </Button>
          <Button onClick={() => {
            const fresh: PageDoc = {
              id: `user-page-${Date.now() % 10000}`,
              name: '新页面',
              switchKey: null,
              pos: [0.5, 0.5],
              padding: 20,
              font: { family: '', sizeAdd: 0, scaleSource: '' },
              contentVersion: 0,
              components: [],
            }
            setPageDocs(prev => ({ ...prev, [fresh.id]: fresh }))
            setActiveId(fresh.id)
          }}>
            新建页
          </Button>
        </Space>
        <Canvas
          solve={solve}
          page={active}
          selectedId={selectedComp}
          onSelect={setSelectedComp}
          onDrag={onDragComponent}
        />
      </div>

      {/* inspector */}
      <Inspector
        page={active}
        component={selected}
        onPatchPage={patchPage}
        onPatchComponent={(id, mut) => patchComponent(id, mut)}
        onRemove={removeComponent}
        onDuplicate={duplicateComponent}
      />
    </div>
  )
}
