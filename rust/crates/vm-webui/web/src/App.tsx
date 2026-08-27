/**
 * MainForm 主界面 (阶段②+③): 左侧 Tabs (对位 Java WebTabbedPane LEFT) +
 * cfg 树数据驱动渲染 (HEADER 卡片嵌套 + 列网格) + 底部按钮组。
 * 值变更 → form_message IPC → Rust main_form::update (WYSIWYG 链 Rust 侧闭环);
 * 阶段③: controller-state 核状态徽标 / fm-changed toast / 导入配置。
 */
import React, { useCallback, useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import {
  Badge,
  Button,
  Card,
  Col,
  Row as GridRow,
  Space,
  Spin,
  Tabs,
  Typography,
  message,
  notification,
} from 'antd'
import type { PanelDto, RowDto } from './api'
import { getLayoutTree, importConfig, sendFormMessage } from './api'
import { RowRenderer } from './rows'

const { Title, Text } = Typography

/** 行树 → 卡片/网格 (对位 DynamicDataPage.buildContainer: HEADER=组标题+递归, 其余入网格) */
const RowsTree: React.FC<{ rows: RowDto[]; panel: string; cols: number; values: Record<string, unknown> }> = ({
  rows,
  panel,
  cols,
  values,
}) => {
  const items: React.ReactNode[] = []
  let chunk: React.ReactNode[] = []
  const flush = () => {
    if (!chunk.length) return
    items.push(
      <GridRow key={`g${items.length}`} gutter={[10, 5]}>
        {chunk.map((n, i) => (
          <Col key={i} span={24 / cols}>
            {n}
          </Col>
        ))}
      </GridRow>,
    )
    chunk = []
  }
  for (const r of rows) {
    if (r.type === 'HEADER') {
      flush()
      const childCols = r.groupColumns > 0 ? r.groupColumns : cols
      items.push(
        <Card key={r.label} size="small" title={r.label} style={{ margin: '4px 0' }}>
          <RowsTree rows={r.children} panel={panel} cols={childCols} values={values} />
        </Card>,
      )
    } else {
      chunk.push(<RowRenderer row={r} panel={panel} values={values} />)
    }
  }
  flush()
  return <div>{items}</div>
}

/** fm-changed 事件载荷 (vm-webui bridge.rs FmChangedPayload) */
interface FmChangedPayload {
  name: string | null
  status: string
}

/** 核状态 → 徽标形态 (payload = Rust ControllerState Debug 串, main.rs:200 emit) */
const STATE_BADGE: Record<string, { status: 'default' | 'processing' | 'warning' | 'success'; text: string }> = {
  Init: { status: 'default', text: '初始化' },
  Preview: { status: 'processing', text: '预览' },
  Connected: { status: 'warning', text: '已连接' },
  InGame: { status: 'success', text: '游戏中' },
}

/** 导入外部 ui_layout.user.cfg (open 选路径 → import_config; 成功后 Rust 广播
 *  config-changed → 既有监听自动重拉树) */
const importConfigDialog = async () => {
  const path = await open({ filters: [{ name: 'VoidMei 配置', extensions: ['cfg'] }] })
  if (typeof path !== 'string') return // 取消选择静默
  importConfig(path)
    .then(() => message.success('导入成功, 配置已刷新'))
    .catch((e) => message.error(`导入失败: ${e}`))
}

export default function App() {
  const [ready, setReady] = useState(false)
  const [panels, setPanels] = useState<PanelDto[]>([])
  const [values, setValues] = useState<Record<string, Record<string, unknown>>>({})
  const [loadErr, setLoadErr] = useState('')
  const [activeTab, setActiveTab] = useState('')
  /** 核状态徽标 (初值 Init; controller-state 事件驱动) */
  const [ctrlState, setCtrlState] = useState('Init')

  const reload = useCallback(async () => {
    try {
      const tree = await getLayoutTree()
      setPanels(tree)
      setActiveTab((cur) => (tree.some((p) => p.title === cur) ? cur : tree[0]?.title ?? ''))
      setLoadErr('')
    } catch (e) {
      setLoadErr(String(e))
    }
  }, [])

  useEffect(() => {
    // 就绪 = 监听注册后再上报 (Rust show+emit 与 listen 注册的竞态, 见阶段①记录)
    listen('window-echo', () => {
      invoke('window_echo').catch(console.error)
    })
      .then(() => invoke('ui_ready'))
      .then(() => setReady(true))
      .catch((e) => setLoadErr(String(e)))
    // cfg 树变化 (reset/import 后 Rust 广播) → 重拉
    listen<unknown>('config-changed', () => {
      reload().catch(console.error)
    }).catch(console.error)
    // 核状态徽标 (Init/Preview/Connected/InGame)
    listen<string>('controller-state', (e) => setCtrlState(e.payload)).catch(console.error)
    // FM 缺失/损坏 toast (对位 Java NotificationService; 其余状态静默)
    listen<FmChangedPayload>('fm-changed', (e) => {
      const { name, status } = e.payload
      if (status.includes('Missing')) {
        notification.warning({ message: name ?? '未知机型', description: 'FM 数据缺失, 相关指标已降级' })
      } else if (status.includes('Corrupt')) {
        notification.warning({ message: name ?? '未知机型', description: 'FM 数据损坏' })
      }
    }).catch(console.error)
  }, [reload])

  useEffect(() => {
    if (ready && !panels.length && !loadErr) {
      reload().catch(console.error)
    }
  }, [ready, panels.length, loadErr, reload])

  /** 本地乐观值 (panel→key→v): 受控初值兜底 row.value */
  const valueTree = useMemo(() => values, [values])
  const trackLocal = (panel: string, key: string, v: unknown) =>
    setValues((old) => ({ ...old, [panel]: { ...(old[panel] ?? {}), [key]: v } }))

  const act = (m: Parameters<typeof sendFormMessage>[0], track?: { panel: string; key: string; v: unknown }) => {
    if (track) trackLocal(track.panel, track.key, track.v)
    return sendFormMessage(m).catch((e) => message.error(`IPC 失败: ${e}`))
  }

  const tabs = panels.map((p) => ({
    key: p.title,
    label: p.title,
    children: (
      <div style={{ padding: '8px 12px', overflowY: 'auto', height: 'calc(100vh - 120px)' }}>
        {p.rows.length ? (
          <RowsTree
            rows={p.rows}
            panel={p.title}
            cols={p.panelColumns > 0 ? p.panelColumns : 2}
            values={valueTree[p.title] ?? {}}
          />
        ) : (
          <Text type="secondary">Data (Empty)</Text>
        )}
      </div>
    ),
  }))

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      <div style={{ padding: '8px 12px 0' }}>
        <Space>
          <Title level={4} style={{ margin: 0 }}>
            VoidMei 设置
          </Title>
          <Badge {...(STATE_BADGE[ctrlState] ?? STATE_BADGE.Init)} />
          {!ready && <Spin size="small" />}
          {ready && !panels.length && !loadErr && <Badge status="processing" text="加载 cfg 树…" />}
          {loadErr && <Badge status="error" text={`加载失败: ${loadErr}`} />}
        </Space>
      </div>
      <div style={{ flex: 1, minHeight: 0 }}>{tabs.length ? (
        <Tabs
          tabPosition="left"
          items={tabs}
          activeKey={activeTab}
          onChange={setActiveTab}
          style={{ height: '100%' }}
        />
      ) : (
        <div style={{ padding: 24 }}>
          <Text type="secondary">等待配置树… (先 ui_ready 后拉取)</Text>
        </div>
      )}</div>
      <div style={{ padding: '8px 12px', borderTop: '1px solid #333' }}>
        <Space>
          <Button size="small" onClick={() => act({ kind: 'Save' })}>
            保存
          </Button>
          <Button
            size="small"
            type="primary"
            onClick={() => act({ kind: 'StartGame' })}
          >
            开始游戏
          </Button>
          <Button size="small" danger onClick={() => act({ kind: 'EndGame' })}>
            结束游戏
          </Button>
          <Button size="small" onClick={() => act({ kind: 'RefreshPreviews' })}>
            刷新预览
          </Button>
          <Button size="small" onClick={() => importConfigDialog().catch((e) => message.error(`打开文件框失败: ${e}`))}>
            导入配置
          </Button>
        </Space>
      </div>
    </div>
  )
}
