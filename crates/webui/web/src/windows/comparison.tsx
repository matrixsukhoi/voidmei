/**
 * 对比 web 窗口 (CompactComparisonWindow.java 复刻, MODELESS 对位 = 独立
 * WebviewWindow)。布局: 表头 (空 | fm0 名 | VS | fm1 名) + 行清单 (胜负高亮) +
 * 底部 COPY/CLOSE。数据经 W1 comparison_data; 机型切换 = GridSelectorDialog
 * 对位的窗内搜索选择器 (fm_list)。
 * 键: ESC 关窗 / Ctrl+C 复制 (Java registerKeyboardAction WHEN_IN_FOCUSED_WINDOW)。
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ConfigProvider, Input, Modal, Spin, theme as antdTheme } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { ComparisonData } from '../api'
import { getComparisonData, getWebFmList } from '../api'
import { CMP_COLORS, filterPlanes, gridTemplate, pushRecent, rowColors } from './comparisonPresent'

/** Java GridSelectorDialog.recentPlanes 静态字段 (会话级) — web 窗口销毁即重建,
 *  生命周期 = 窗口 (Java 是 JVM 会话; 备案差异) */
let recentPlanes: string[] = []

/** 机型搜索选择器 (GridSelectorDialog.java: 搜索 + 启发式过滤页签 + Recent + 网格) */
const GridSelector: React.FC<{
  open: boolean
  onPick: (plane: string) => void
  onCancel: () => void
}> = ({ open, onPick, onCancel }) => {
  const [all, setAll] = useState<string[]>([])
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState('All')
  const [recent, setRecent] = useState<string[]>(recentPlanes)
  useEffect(() => {
    if (open) getWebFmList().then(setAll).catch(() => setAll([]))
  }, [open])
  const shown = useMemo(() => filterPlanes(all, query, filter), [all, query, filter])
  const pick = (plane: string) => {
    recentPlanes = pushRecent(recentPlanes, plane)
    setRecent(recentPlanes)
    onPick(plane)
  }
  const btn: React.CSSProperties = { width: 140, height: 40 }
  return (
    <Modal
      open={open}
      title="Select Aircraft"
      footer={null}
      width={860}
      styles={{ body: { height: 520, overflowY: 'auto' } }}
      onCancel={onCancel}
      destroyOnClose
    >
      {/* Search Bar (居中) */}
      <div style={{ display: 'flex', justifyContent: 'center', marginBottom: 8 }}>
        <Input
          placeholder="Search aircraft..."
          value={query}
          style={{ width: 240 }}
          onChange={(e) => setQuery(e.target.value)}
          allowClear
        />
      </div>
      {/* Filter Tabs (启发式: All/WWII/Modern/Red/Blue; Red/Blue 无条件放行) */}
      <div style={{ display: 'flex', justifyContent: 'center', gap: 10, marginBottom: 12 }}>
        {['All', 'WWII', 'Modern', 'Red', 'Blue'].map((name) => (
          <button
            key={name}
            onClick={() => setFilter(name)}
            style={{
              padding: '2px 14px',
              cursor: 'pointer',
              border: filter === name ? '1px solid #1677ff' : '1px solid #d9d9d9',
              borderRadius: 4,
              background: filter === name ? '#e6f4ff' : 'transparent',
            }}
          >
            {name}
          </button>
        ))}
      </div>
      {/* Recent Section (TitledBorder "Recent") */}
      {recent.length > 0 && (
        <div style={{ borderTop: '1px solid #999', margin: '6px 0 12px', paddingTop: 10 }}>
          <div style={{ fontSize: 12, color: '#666', marginBottom: 6 }}>Recent</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 10 }}>
            {recent.map((p) => (
              <button key={p} style={btn} onClick={() => pick(p)}>
                {p}
              </button>
            ))}
          </div>
        </div>
      )}
      {/* Grid Area (FlowLayout wrap 10,10; 按钮 140×40) */}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 10 }}>
        {shown.map((p) => (
          <button key={p} style={btn} onClick={() => pick(p)}>
            {p}
          </button>
        ))}
      </div>
    </Modal>
  )
}

/** 单行 (addComparisonRow): 属性行 4 列 / 单机 2 列 */
const CmpRow: React.FC<{ row: ComparisonData['rows'][number]; singleMode: boolean }> = ({
  row,
  singleMode,
}) => {
  const { c0, c1 } = rowColors(row.win, singleMode)
  return (
    <div style={{ display: 'grid', gridTemplateColumns: gridTemplate(singleMode), fontSize: 13, lineHeight: '22px' }}>
      <span style={{ color: CMP_COLORS.textPrimary, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {row.text}
      </span>
      <span style={{ color: c0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {row.value0}
      </span>
      {!singleMode && (
        <>
          {/* 符号列: insets (0,15,0,15) → padding 0 15, 居中金色 */}
          <span style={{ color: CMP_COLORS.symbol, textAlign: 'center', padding: '0 15px' }}>
            {row.symbol}
          </span>
          <span style={{ color: c1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {row.value1}
          </span>
        </>
      )}
    </div>
  )
}

const ComparisonApp: React.FC<{ fm0Initial: string; fm1Initial: string | null }> = ({
  fm0Initial,
  fm1Initial,
}) => {
  const [fm0, setFm0] = useState(fm0Initial)
  const [fm1, setFm1] = useState<string | null>(fm1Initial)
  const [data, setData] = useState<ComparisonData | null>(null)
  const [err, setErr] = useState('')
  const [selectorSlot, setSelectorSlot] = useState<0 | 1 | null>(null)
  const [copied, setCopied] = useState(false)
  const copyTimer = useRef<number | null>(null)

  // 数据拉取 (Java initUI 构造期同步 loadFmLines → web IPC 异步)
  useEffect(() => {
    let cancelled = false
    getComparisonData(fm0, fm1)
      .then((d) => !cancelled && (setData(d), setErr('')))
      .catch((e) => !cancelled && setErr(String(e)))
    return () => {
      cancelled = true
    }
  }, [fm0, fm1])

  const close = useCallback(() => {
    getCurrentWindow().close().catch(() => undefined)
  }, [])

  /** COPY (copyToClipboard + "COPIED!" 1s 回落, Java :296-318/:444-448) */
  const copy = useCallback(async () => {
    const text = data?.copyText ?? ''
    try {
      await navigator.clipboard.writeText(text)
    } catch {
      // WebView2 剪贴板权限面兜底: 隐藏 textarea + execCommand (Java Toolkit 系统剪贴板等价)
      const ta = document.createElement('textarea')
      ta.value = text
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      document.execCommand('copy')
      document.body.removeChild(ta)
    }
    setCopied(true)
    if (copyTimer.current != null) window.clearTimeout(copyTimer.current)
    copyTimer.current = window.setTimeout(() => setCopied(false), 1000)
  }, [data])

  // ESC 关窗 / Ctrl+C 复制 (Java :351-358)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close()
      else if (e.key === 'c' && e.ctrlKey) copy()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [close, copy])
  // 卸载清定时器 (COPIED! 回落定时器不泄漏)
  useEffect(() => () => {
    if (copyTimer.current != null) window.clearTimeout(copyTimer.current)
  }, [])

  const single = data?.singleMode ?? fm1 == null

  /** 页签按钮底/悬停色 (Java mouseEntered/Exited 换色) */
  const footerBtn = (base: string, hover: string): {
    ref: (el: HTMLButtonElement | null) => void
    style: React.CSSProperties
  } => {
    const style: React.CSSProperties = {
      background: base,
      color: '#fff',
      fontWeight: 700,
      fontSize: 14,
      border: 'none',
      padding: '8px 0',
      cursor: 'pointer',
    }
    return {
      style,
      ref: (el) => {
        if (!el) return
        el.onmouseenter = () => (el.style.background = hover)
        el.onmouseleave = () => (el.style.background = base)
      },
    }
  }

  return (
    <div style={{ background: CMP_COLORS.bg, height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* content: EmptyBorder(10,10,10,10) */}
      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', padding: 10 }}>
        {err && <div style={{ color: CMP_COLORS.accentWorse, padding: 8 }}>加载失败: {err}</div>}
        {!data && !err && (
          <div style={{ display: 'flex', justifyContent: 'center', padding: 24 }}>
            <Spin />
          </div>
        )}
        {data && (
          <>
            {/* Header (addHeader): 底边距 10; 机型名带换机入口 (GridSelector 对位) */}
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: gridTemplate(single),
                marginBottom: 10,
                fontSize: 13,
                fontWeight: 700,
              }}
            >
              <span />
              <span style={{ color: CMP_COLORS.headerA, textAlign: single ? 'left' : 'center' }}>
                {data.fm0Name}{' '}
                <a
                  style={{ color: CMP_COLORS.textSecondary, fontSize: 11, cursor: 'pointer' }}
                  onClick={() => setSelectorSlot(0)}
                  title="更换机型"
                >
                  ▾
                </a>
              </span>
              {!single && (
                <>
                  <span style={{ color: CMP_COLORS.textSecondary, textAlign: 'center' }}>VS</span>
                  <span style={{ color: CMP_COLORS.headerB, textAlign: 'center' }}>
                    {data.fm1Name}{' '}
                    <a
                      style={{ color: CMP_COLORS.textSecondary, fontSize: 11, cursor: 'pointer' }}
                      onClick={() => setSelectorSlot(1)}
                      title="更换机型"
                    >
                      ▾
                    </a>
                  </span>
                </>
              )}
            </div>
            {/* Body 行清单 (分节标题 addSectionHeader: insets(15,0,5,0) bold 13 居中) */}
            {data.rows.map((row, i) =>
              row.isHeader ? (
                <div
                  key={`h${i}`}
                  style={{
                    margin: '15px 0 5px',
                    fontWeight: 700,
                    fontSize: 13,
                    color: CMP_COLORS.textSecondary,
                    textAlign: 'center',
                  }}
                >
                  {row.text}
                </div>
              ) : (
                <CmpRow key={`r${i}`} row={row} singleMode={single} />
              ),
            )}
          </>
        )}
      </div>
      {/* Footer: GridLayout(1,2) COPY | CLOSE */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }}>
        <button {...footerBtn(CMP_COLORS.copy, CMP_COLORS.copyHover)} onClick={() => copy()}>
          {copied ? 'COPIED!' : 'COPY'}
        </button>
        <button {...footerBtn(CMP_COLORS.close, CMP_COLORS.closeHover)} onClick={close}>
          CLOSE
        </button>
      </div>
      {/* GridSelectorDialog 对位: 窗内机型搜索选择器 */}
      <GridSelector
        open={selectorSlot != null}
        onCancel={() => setSelectorSlot(null)}
        onPick={(plane) => {
          if (selectorSlot === 0) setFm0(plane)
          else if (selectorSlot === 1) setFm1(plane)
          setSelectorSlot(null)
        }}
      />
    </div>
  )
}

/** 窗口根 (自带暗色 AntD 主题 — 主窗粉白主题不串染; Java 窗体即暗色) */
export const ComparisonWindowRoot: React.FC = () => {
  const params = new URLSearchParams(window.location.search)
  return (
    <ConfigProvider locale={zhCN} theme={{ algorithm: antdTheme.darkAlgorithm }}>
      <ComparisonApp fm0Initial={params.get('fm0') ?? 'a_4h'} fm1Initial={params.get('fm1')} />
    </ConfigProvider>
  )
}
