/**
 * "公式"编辑器 tab (MainForm 手工追加 tab 的内容组件, 三栏布局):
 * - 左列 (~220px): 公式列表 (内置/自定义分组, 单位显示 + 错误红点 + disabled 灰显) + 新建/恢复出厂;
 * - 中列 (flex 自适应): 选中公式的编辑表单 (名称/表达式编辑器/单位/精度/描述/启用) + 保存/删除;
 * - 右列 (~300px): 变量目录常驻面板 (可折叠为窄条, 状态记 localStorage);
 *   窗口 <900px 时右列退化为抽屉 (左列出现"变量目录"按钮唤起)。
 * 统一命名空间 ("公式即变量"): 目录 = 系统变量 + 公式产出; 公式条目点击定位左栏公式,
 * 接管型公式 (与系统变量同名, 如内置 mach) 列表项带 ⚡ 标记。
 * 保存链 = save_formulas 全量提交 (后端 merge: builtin 条目保留标志 = 用户覆盖内置)。
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Button, Input, InputNumber, List, Modal, Popconfirm, Space, Switch, Tooltip, Typography, message } from 'antd'
import type { FormulaItem, VarCatalogEntry } from '../api'
import { getFormulaList, getVarCatalog, resetFormulas, saveFormulas } from '../api'
import { ExprEditor } from './ExprEditor'
import { VarDrawer, VarPanel } from './VarCatalog'

const { Text } = Typography

/** tab 标题 (App.tsx tabs key 同源; reload 的 tab 记忆判定也用它) */
export const FORMULA_TAB = '公式'

/** 窄窗判定 (<900px 右栏退化为抽屉; 899px 边界与常见 900px 分栏断点对齐) */
const NARROW_QUERY = '(max-width: 899px)'
/** 变量面板折叠状态持久化键 ('1' = 折叠; 缺省展开) */
const COLLAPSE_KEY = 'vm-varpanel-collapsed'

/**
 * 三栏高度: tab 外层容器 (App.tsx) = 100vh - 36(标题栏) - 52(底栏), 上下 padding 6+16,
 * 可用内容高 = 100vh - 110px。用 calc 定死 (父链无显式高度, 100% 会失效),
 * 三栏各自内部滚动, 不撑破动态窗口高度测量。
 */
const TAB_HEIGHT = 'calc(100vh - 110px)'

/** 新建条目模板 */
const newFormula = (name: string): FormulaItem => ({
  name,
  expr: '',
  unit: '',
  precision: 2,
  desc: '',
  disabled: false,
  builtin: false,
  getter: null,
  error: null,
})

/** 窄窗响应式: matchMedia 监听 (比 resize 监听少一次无效回调) */
const useNarrow = (): boolean => {
  const [narrow, setNarrow] = useState(() => window.matchMedia(NARROW_QUERY).matches)
  useEffect(() => {
    const mq = window.matchMedia(NARROW_QUERY)
    const onChange = (e: MediaQueryListEvent) => setNarrow(e.matches)
    mq.addEventListener('change', onChange)
    return () => mq.removeEventListener('change', onChange)
  }, [])
  return narrow
}

export const FormulaTab: React.FC = () => {
  const [items, setItems] = useState<FormulaItem[]>([])
  const [vars, setVars] = useState<VarCatalogEntry[]>([])
  const [selectedName, setSelectedName] = useState('')
  const [saving, setSaving] = useState(false)
  /** 窄窗模式下的变量抽屉开关 (宽窗面板常驻不用它) */
  const [drawerOpen, setDrawerOpen] = useState(false)
  /** 右栏折叠 (收为窄条; localStorage 记忆, 缺省展开) */
  const [collapsed, setCollapsed] = useState(() => localStorage.getItem(COLLAPSE_KEY) === '1')
  const narrow = useNarrow()
  /** 光标插入函数 (ExprEditor 挂载时注入, 变量目录点击时调用) */
  const insertRef = useRef<(text: string) => void>(() => undefined)

  const toggleCollapse = () =>
    setCollapsed((c) => {
      localStorage.setItem(COLLAPSE_KEY, c ? '0' : '1') // 状态与持久化同步写
      return !c
    })

  // 公式列表加载 (保存/删除/恢复出厂后都重拉: 拿后端编译 error 标注 + 归一名单)
  const load = useCallback(async () => {
    try {
      setItems(await getFormulaList())
    } catch (e) {
      message.error(`加载公式失败: ${e}`)
    }
  }, [])
  useEffect(() => {
    load()
  }, [load])
  // 变量目录加载 (补全与面板/抽屉共享; 公式产出条目随公式集变化, 保存/恢复出厂后重拉)
  const loadVars = useCallback(() => {
    getVarCatalog()
      .then(setVars)
      .catch(() => undefined)
  }, [])
  useEffect(() => {
    loadVars()
  }, [loadVars])

  const builtinItems = useMemo(() => items.filter((i) => i.builtin), [items])
  const userItems = useMemo(() => items.filter((i) => !i.builtin), [items])
  const selected = useMemo(() => items.find((i) => i.name === selectedName) ?? null, [items, selectedName])
  /** 接管型公式名集 (目录 originKey=formula 且 overridesSystem — 公式与系统变量同名即接管其值) */
  const overrideNames = useMemo(
    () => new Set(vars.filter((v) => v.originKey === 'formula' && v.overridesSystem === true).map((v) => v.name)),
    [vars],
  )
  /** 公式名 → 表达式映射 (补全源的 detail 显示 "公式: <expr 截断>", 与目录数据同源无需额外请求) */
  const formulaExprs = useMemo(() => {
    const m: Record<string, string> = {}
    for (const i of items) m[i.name] = i.expr
    return m
  }, [items])

  /** 就地修改选中条目字段 (本地待保存副本, 保存时全量提交) */
  const patch = (p: Partial<FormulaItem>) =>
    setItems((old) => old.map((i) => (i.name === selectedName ? { ...i, ...p } : i)))

  /** 改名 (自定义条目): 列表键与选中键同步迁移 */
  const rename = (newName: string) => {
    const oldName = selectedName
    setItems((old) => old.map((i) => (i.name === oldName ? { ...i, name: newName } : i)))
    setSelectedName(newName)
  }

  /** 新建: 追加本地条目并选中 (保存时才真正落库) */
  const addNew = () => {
    let name = '新公式'
    let n = 2
    while (items.some((i) => i.name === name)) name = `新公式${n++}`
    setItems((old) => [...old, newFormula(name)])
    setSelectedName(name)
  }

  /** 全量保存 + 热更新 (后端 merge; 空名/空表达式前端先拦) */
  const save = async () => {
    if (items.some((i) => !i.name.trim())) {
      message.warning('公式名称不能为空')
      return
    }
    setSaving(true)
    try {
      const r = await saveFormulas(items)
      if (r.ok) {
        message.success('公式已保存并热更新')
        await load() // 重拉拿编译错误标注
        loadVars() // 公式产出条目 (值/接管标志/补全源) 随公式集变化
      } else {
        message.error(`保存失败: ${r.error ?? '未知错误'}`)
      }
    } catch (e) {
      message.error(`保存失败: ${e}`)
    } finally {
      setSaving(false)
    }
  }

  /** 删除自定义条目 (本地剔除 + 立即提交, 否则重拉后复活) */
  const remove = async (name: string) => {
    const rest = items.filter((i) => i.name !== name)
    try {
      const r = await saveFormulas(rest)
      if (r.ok) {
        setItems(rest)
        if (selectedName === name) setSelectedName('')
        message.success(`已删除 ${name}`)
      } else {
        message.error(`删除失败: ${r.error ?? '未知错误'}`)
      }
    } catch (e) {
      message.error(`删除失败: ${e}`)
    }
  }

  /** 恢复出厂 (二次确认; 删除自定义 + 重置内置覆盖) */
  const resetAll = () => {
    Modal.confirm({
      title: '恢复出厂公式?',
      content: '将删除全部自定义公式, 并把内置公式恢复为出厂定义 (你的覆盖会丢失)。',
      onOk: async () => {
        try {
          const r = await resetFormulas()
          if (r.ok) {
            message.success('已恢复出厂公式')
            setSelectedName('')
            await load()
            loadVars() // 自定义公式产出条目随恢复出厂消失, 目录同步重拉
          } else {
            message.error(`恢复失败: ${r.error ?? '未知错误'}`)
          }
        } catch (e) {
          message.error(`恢复失败: ${e}`)
        }
      },
    })
  }

  /**
   * 变量目录点击公式产出变量 → 选中左栏对应公式 (选中态即高亮);
   * 窄窗抽屉模式下顺手收起抽屉, 露出左栏让用户看到定位结果。
   */
  const openFormula = (name: string) => {
    setSelectedName(name)
    if (narrow) setDrawerOpen(false)
  }

  /** 列表条目渲染 (红点 = 后端编译错误, Tooltip 看详情; ⚡ = 接管系统变量; disabled 灰显; 单位右缀) */
  const renderRow = (item: FormulaItem) => {
    const active = item.name === selectedName
    return (
      <List.Item
        style={{
          padding: '4px 8px',
          borderRadius: 6,
          cursor: 'pointer',
          opacity: item.disabled ? 0.45 : 1,
          background: active ? '#FFF0F7' : undefined,
          border: active ? '1px solid #FFD6E8' : '1px solid transparent',
        }}
        onClick={() => setSelectedName(item.name)}
      >
        <Space size={6} style={{ width: '100%', minWidth: 0 }}>
          {item.error ? (
            <Tooltip title={item.error}>
              <span style={{ color: '#ff4d4f', lineHeight: 1 }}>●</span>
            </Tooltip>
          ) : (
            <span style={{ color: 'transparent', lineHeight: 1 }}>●</span> // 占位对齐
          )}
          {/* 接管型公式: 公式名与系统变量同名, 该变量的值由本公式接管 (如内置 mach) */}
          {overrideNames.has(item.name) && (
            <Tooltip title={`接管系统变量 ${item.name} 的值`}>
              <span style={{ color: '#faad14', lineHeight: 1, fontSize: 11 }}>⚡</span>
            </Tooltip>
          )}
          <Text ellipsis style={{ flex: 1, minWidth: 0 }}>
            {item.name}
          </Text>
          {item.unit && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              {item.unit}
            </Text>
          )}
        </Space>
      </List.Item>
    )
  }

  /** 左列列表 (固定高度内滚动, 两个分组) */
  const groupList = (
    <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
      <Text strong style={{ fontSize: 12, display: 'block', margin: '4px 0' }}>
        内置公式
      </Text>
      <List size="small" dataSource={builtinItems} renderItem={renderRow} split={false} />
      <Text strong style={{ fontSize: 12, display: 'block', margin: '4px 0' }}>
        我的公式
      </Text>
      <List size="small" dataSource={userItems} renderItem={renderRow} split={false} />
    </div>
  )

  /** 右栏折叠态窄条 (竖排"变量目录"四字, 点击展开) */
  const varRail = (
    <div className="var-rail" title="展开变量目录" onClick={toggleCollapse}>
      <span style={{ writingMode: 'vertical-rl', letterSpacing: 4, fontSize: 13, color: '#555' }}>
        变量目录
      </span>
    </div>
  )

  return (
    <div style={{ display: 'flex', gap: 12, height: TAB_HEIGHT }}>
      {/* 左列: 操作按钮 + 分组列表 (窄窗多一个"变量目录"唤起抽屉) */}
      <div style={{ width: 220, flexShrink: 0, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
        <Space size={6} style={{ marginBottom: 8 }} wrap>
          <Button size="small" type="primary" onClick={addNew}>
            新建公式
          </Button>
          {narrow && (
            <Button size="small" onClick={() => setDrawerOpen(true)}>
              变量目录
            </Button>
          )}
          <Button size="small" danger onClick={resetAll}>
            恢复出厂
          </Button>
        </Space>
        {groupList}
      </div>

      {/* 中列: 选中条目编辑区 */}
      <div style={{ flex: 1, minWidth: 0, overflowY: 'auto', paddingRight: 4 }}>
        {selected ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {/* 第一行: 名称 / 单位 / 精度 / 启用 */}
            <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>名称</Text>
                <Input
                  size="small"
                  style={{ width: 170 }}
                  value={selected.name}
                  disabled={selected.builtin}
                  onChange={(e) => rename(e.target.value)}
                />
              </span>
              <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>单位</Text>
                <Input
                  size="small"
                  style={{ width: 90 }}
                  value={selected.unit}
                  onChange={(e) => patch({ unit: e.target.value })}
                />
              </span>
              <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>精度</Text>
                <InputNumber
                  size="small"
                  style={{ width: 64 }}
                  min={0}
                  max={9}
                  precision={0}
                  value={selected.precision}
                  onChange={(v) => patch({ precision: v ?? 0 })}
                />
              </span>
              <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>启用</Text>
                <Switch
                  size="small"
                  checked={!selected.disabled}
                  onChange={(v) => patch({ disabled: !v })}
                />
              </span>
            </div>
            {/* 表达式编辑器 (CodeMirror, 校验/试算/补全) */}
            <div>
              <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>
                表达式{selected.builtin ? ' (内置公式, 可覆盖编辑)' : ''}
              </Text>
              <ExprEditor
                value={selected.expr}
                onChange={(expr) => patch({ expr })}
                varCatalog={vars}
                formulaExprs={formulaExprs}
                registerInsert={(fn) => {
                  // fn=null 为卸载注销, 兜底空函数防野调用
                  insertRef.current = fn ?? (() => undefined)
                }}
              />
            </div>
            {/* 描述 */}
            <div>
              <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>
                描述
              </Text>
              <Input.TextArea
                rows={2}
                value={selected.desc}
                onChange={(e) => patch({ desc: e.target.value })}
                placeholder="公式用途说明 (可选)"
              />
            </div>
            {/* 操作行: 保存 (全量提交) / 删除 (仅自定义) */}
            <Space>
              <Button type="primary" size="small" loading={saving} onClick={save} disabled={!selected.name.trim()}>
                保存
              </Button>
              {!selected.builtin && (
                <Popconfirm title={`删除公式 ${selected.name}?`} onConfirm={() => remove(selected.name)}>
                  <Button danger size="small">
                    删除
                  </Button>
                </Popconfirm>
              )}
            </Space>
          </div>
        ) : (
          <div style={{ padding: 24 }}>
            <Text type="secondary">选择左侧公式进行编辑, 或点击"新建公式"。</Text>
          </div>
        )}
      </div>

      {/* 右列: 变量目录常驻面板 / 折叠窄条 (宽窗); 抽屉 (窄窗) */}
      {!narrow &&
        (collapsed ? varRail : (
          <VarPanel
            vars={vars}
            onInsert={(n) => insertRef.current(n)}
            onOpenFormula={openFormula}
            onCollapse={toggleCollapse}
          />
        ))}
      {narrow && (
        <VarDrawer
          open={drawerOpen}
          onClose={() => setDrawerOpen(false)}
          vars={vars}
          onInsert={(n) => insertRef.current(n)}
          onOpenFormula={openFormula}
        />
      )}
    </div>
  )
}
