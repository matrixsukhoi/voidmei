/**
 * MainForm 主界面 (阶段②+③, 视觉对位 Java PinkStyle/WebLaF):
 * - 自绘标题栏 (对位 Java setUndecorated: 拖拽区 + ─/✕ 窗口按钮, X=hide);
 * - 左侧 Tabs (WebTabbedPane LEFT 同位);
 * - TitledBorder 卡片 (标题嵌边框线, 白底) + 网格 (DynamicDataPage.buildContainer 同构);
 * - 底部左组 (保存/刷新预览/导入) + 右组胶囊 [退　出|开　始] (Java mCancel/mStart 同款);
 * - 水印 (Java setWatermark(image/watermark.png))。
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { Badge, Button, Space, Spin, Tabs, Typography, message, notification } from 'antd'
import type { PanelDto, RowDto } from './api'
import { getAppVersion, getAssetRoot, getLayoutTree, importConfig, sendFormMessage } from './api'
import { RowRenderer } from './rows'
import { AppDialogs } from './dialogs'
import { FORMULA_TAB, FormulaTab } from './formulas/FormulaTab'

const { Title, Text } = Typography

const appWindow = getCurrentWindow()

/** 自绘标题栏: 拖拽区 + 版本号 + 状态徽标 + 最小化/关闭 (关闭=退出, 对位 Java EXIT_ON_CLOSE) */
const TitleBar: React.FC<{ ctrlState: string; version: string; onQuit: () => void }> = ({
  ctrlState,
  version,
  onQuit,
}) => (
  <div className="titlebar" data-tauri-drag-region>
    <Title level={5} style={{ margin: 0, fontSize: 14, flex: 1 }} data-tauri-drag-region>
      VoidMei 设置{version ? ` v${version}` : ''}
    </Title>
    <Badge {...(STATE_BADGE[ctrlState] ?? STATE_BADGE.Init)} style={{ marginRight: 10 }} />
    <button className="win-btn" title="最小化" onClick={() => appWindow.minimize()}>
      ─
    </button>
    <button className="win-btn close" title="退出 VoidMei" onClick={onQuit}>
      ✕
    </button>
  </div>
)

/** TitledBorder 卡片 (Java PinkStyle.createContainer: 白底+细边+标题嵌边框线) */
const TitledCard: React.FC<{ title: string; nested?: boolean; children: React.ReactNode }> = ({
  title,
  nested,
  children,
}) => (
  <div className={`titled-card${nested ? ' nested' : ''}`}>
    <span className="tc-title">{title}</span>
    {children}
  </div>
)

/**
 * 行树 → 卡片/网格 (对位 DynamicDataPage.buildContainer: HEADER=组标题+递归, 其余入网格)。
 * 网格轨道 `repeat(cols, max-content 1fr)` + 行组件 subgrid = ResponsiveGridLayout
 * 的 maxLabelWidthPerColumn: 同列 label 等宽 → 控件垂直对齐 (内容感知列宽的 CSS 近似)。
 */
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
      <div key={`g${items.length}`} className="row-grid" style={{ gridTemplateColumns: `repeat(${cols}, max-content 1fr)` }}>
        {chunk}
      </div>,
    )
    chunk = []
  }
  for (const r of rows) {
    if (r.type === 'HEADER') {
      flush()
      const childCols = r.groupColumns > 0 ? r.groupColumns : cols
      items.push(
        <TitledCard key={r.label} title={r.label}>
          <RowsTree rows={r.children} panel={panel} cols={childCols} values={values} />
        </TitledCard>,
      )
    } else {
      chunk.push(<RowRenderer key={r.label + r.type} row={r} panel={panel} values={values} />)
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

/** 核状态 → 徽标形态 (payload = Rust ControllerState Debug 串) */
const STATE_BADGE: Record<string, { status: 'default' | 'processing' | 'warning' | 'success'; text: string }> = {
  Init: { status: 'default', text: '初始化' },
  Preview: { status: 'processing', text: '预览' },
  Connected: { status: 'warning', text: '已连接' },
  InGame: { status: 'success', text: '游戏中' },
}

/** 导入外部 ui_layout.user.cfg (成功后 Rust 广播 config-changed → 既有监听自动重拉树) */
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
  // tab 记忆 (Java UIStateStorage.saveLastTab 的本地等价)
  const [activeTab, setActiveTab] = useState(() => localStorage.getItem('vm-last-tab') ?? '')
  const [ctrlState, setCtrlState] = useState('Init')
  const [watermark, setWatermark] = useState<string | null>(null)
  const [version, setVersion] = useState('')
  /** 内容测量层 (不设 overflow, 高度=真实内容; Java getRequiredHeight 等价) */
  const measureRef = useRef<HTMLDivElement | null>(null)

  const reload = useCallback(async () => {
    try {
      const tree = await getLayoutTree()
      setPanels(tree)
      // 与 setPanels 同批清空乐观层: 重拉真值接管 (reset/导入的全量变更不被旧值遮蔽;
      // 普通开关场景重拉值与乐观值相同, 无闪烁)
      setValues({})
      // 手工 "公式" tab 不在 cfg 树里, 但同样是合法停留位 (config-changed 重拉不踢出)
      setActiveTab((cur) =>
        tree.some((p) => p.title === cur) || cur === FORMULA_TAB ? cur : tree[0]?.title ?? '',
      )
      setLoadErr('')
    } catch (e) {
      setLoadErr(String(e))
    }
  }, [])

  // 切 tab 即记忆 (Java saveLastTab 同点)
  const switchTab = (t: string) => {
    setActiveTab(t)
    localStorage.setItem('vm-last-tab', t)
  }

  /** 退出 VoidMei (对位 Java X=EXIT_ON_CLOSE 与 mCancel 同链: saveConfig + 退出)。
   *  hide 给即时反馈, EndGame 走 Rust 干净退出链 (保存 + 主循环收尾, 非裸 exit) */
  const quit = () => {
    appWindow.hide().catch(() => undefined)
    sendFormMessage({ kind: 'EndGame' }).catch((e) => message.error(`IPC 失败: ${e}`))
  }

  // 动态窗口高度 (Java MainForm.updateDynamicSize: 按 tab 内容高度, min=tab×30+180,
  // max=屏-80) — 300ms 防抖, 高度差 >16px 才调 (防抖动)
  useEffect(() => {
    const t = setTimeout(() => {
      const el = measureRef.current
      if (!el) return
      const panels_n = Math.max(panels.length, 1)
      const minH = panels_n * 30 + 180
      const maxH = window.screen.availHeight - 80
      const target = Math.min(Math.max(el.scrollHeight + 36 + 52 + 16, minH), maxH)
      appWindow
        .innerSize()
        .then(({ height }) => {
          if (Math.abs(height - target) > 16) {
            appWindow.setSize(new LogicalSize(800, Math.round(target)))
          }
        })
        .catch(() => undefined)
    }, 300)
    return () => clearTimeout(t)
  }, [activeTab, panels])

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
    // X/Alt+F4/任务栏关闭 (Rust on_window_event prevent_close 后转发) → 退出
    listen('quit-requested', () => {
      quit()
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
    // 水印 (Java setWatermark(image/watermark.png))
    getAssetRoot()
      .then((root) => setWatermark(convertFileSrc(`${root}/image/watermark.png`)))
      .catch(() => undefined)
    // 版本号 (Java 标题 = appName + " v" + version; 单一来源 = 构建注入 VOIDMEI_VERSION
    // → get_app_version 命令, 与 checkUpdate 的 dev 守卫同源; tauri.conf version 不用)
    getAppVersion()
      .then(setVersion)
      .catch(() => undefined)
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
      <div style={{ padding: '6px 14px 16px', overflowY: 'auto', height: 'calc(100vh - 36px - 52px)' }}>
        {/* 测量层: 高度=真实内容 (getRequiredHeight 等价, 不受滚动容器视口影响) */}
        <div
          ref={(el) => {
            if (p.title === activeTab) measureRef.current = el
          }}
        >
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
        {watermark && <img className="watermark" src={watermark} alt="" />}
      </div>
    ),
  })).concat({
    // 手工 tab: "公式"编辑器 (cfg panels 之外的功能页, 不由 ui_layout.cfg 驱动)
    key: FORMULA_TAB,
    label: FORMULA_TAB,
    children: (
      <div style={{ padding: '6px 14px 16px', overflowY: 'auto', height: 'calc(100vh - 36px - 52px)' }}>
        {/* 测量层同 panels tab (动态窗口高度逻辑统一处理) */}
        <div
          ref={(el) => {
            if (activeTab === FORMULA_TAB) measureRef.current = el
          }}
        >
          <FormulaTab />
        </div>
        {watermark && <img className="watermark" src={watermark} alt="" />}
      </div>
    ),
  })

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', background: '#F5F5F5' }}>
      <TitleBar ctrlState={ctrlState} version={version} onQuit={quit} />
      {/* 批3小件弹窗宿主: checkUpdate 一次 + 托盘关于/config 弹窗监听 (渲染 null) */}
      <AppDialogs ready={ready} />
      <div style={{ flex: 1, minHeight: 0, background: '#FFFFFF' }}>
        {tabs.length ? (
          <Tabs
            tabPosition="left"
            items={tabs}
            activeKey={activeTab}
            onChange={switchTab}
            style={{ height: '100%' }}
          />
        ) : (
          <div style={{ padding: 24 }}>
            <Space>
              {!ready && <Spin size="small" />}
              <Text type="secondary">{loadErr ? `加载失败: ${loadErr}` : '等待配置树…'}</Text>
            </Space>
          </div>
        )}
      </div>
      {/* 底部: 左组 (保存/刷新预览/导入, 透明底文字按钮 = Java createFooterButton
          透明样式) + 右组胶囊 [退　出|开　始] (Java BasePage 右组同款, 全角空格等宽) */}
      <div className="footerbar">
        <Space>
          <Button type="text" className="footer-btn" onClick={() => act({ kind: 'Save' })}>
            保存
          </Button>
          <Button type="text" className="footer-btn" onClick={() => act({ kind: 'RefreshPreviews' })}>
            刷新预览
          </Button>
          <Button
            type="text"
            className="footer-btn"
            onClick={() => importConfigDialog().catch((e) => message.error(`打开文件框失败: ${e}`))}
          >
            导入配置
          </Button>
        </Space>
        <Space.Compact>
          <Button danger onClick={() => act({ kind: 'EndGame' })} style={{ height: 32 }}>
            退　出
          </Button>
          <Button type="primary" onClick={() => act({ kind: 'StartGame' })} style={{ height: 32 }}>
            开　始
          </Button>
        </Space.Compact>
      </div>
    </div>
  )
}
