/**
 * 行渲染器族 (阶段②+③): 交互渲染 + INFO/VOICE/FMLIST/HOTKEY 专项 + 占位键只读兜底。
 * 语义对位 vm-ui renderers (SWITCH_INV 落库取反在 Rust apply 层, 前端只发显示值)。
 */
import React, { useEffect, useState } from 'react'
import { Button, ColorPicker, Input, InputNumber, Modal, Select, Slider, Switch, Tooltip, Typography, message } from 'antd'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { RowDto } from './api'
import {
  assetRootOnce,
  browserCodeToVc,
  fmListOnce,
  getComboOptions,
  normalizeDescImg,
  parseColorValue,
  parseVoicePackValue,
  rgbaToHex,
  sendFormMessage,
  splitUrlText,
  vcToKeyName,
  voicePacksOnce,
} from './api'

const { Text } = Typography

interface RowProps {
  row: RowDto
  panel: string
  /** 值树本地态 (受控): panel→key→值; undefined = 未初始化 (用 row.value) */
  values: Record<string, unknown>
}

/** 行键: :target 优先, 无则 label (与 Rust 取键规则一致) */
const rowKey = (row: RowDto): string => row.property ?? row.label

/** desc/desc-img 气泡 (Java ReplicaBuilder.applyStylizedTooltip: 文本 + image/ 目录图片)。
 *  行内统一 label 载体 — 行布局对位 createSwitchItem: label 左, 控件紧随 */
const Label: React.FC<{ text: string; desc?: string | null; descImg?: string | null }> = ({
  text,
  desc,
  descImg,
}) => {
  const [imgUrl, setImgUrl] = useState<string | null>(null)
  // desc-img → asset protocol URL; assetRoot 模块级只取一次, 失败静默降级纯文本气泡
  useEffect(() => {
    setImgUrl(null)
    if (!descImg) return
    assetRootOnce()
      .then((root) => setImgUrl(convertFileSrc(`${root}/${normalizeDescImg(descImg)}`)))
      .catch(() => undefined)
  }, [descImg])
  if (!text) return null
  if (!desc && !descImg) return <Text style={{ fontSize: 14 }}>{text}</Text>
  // Java applyStylizedTooltip 无视觉标记 (纯悬停弹层) — 不加下划线, 仅 help 光标
  return (
    <Tooltip
      title={
        <>
          {desc}
          {imgUrl && <img src={imgUrl} style={{ maxWidth: 280, marginTop: 6, display: 'block' }} />}
        </>
      }
    >
      <span style={{ cursor: 'help', fontSize: 14 }}>{text}</span>
    </Tooltip>
  )
}

/**
 * 行骨架 (对位 ReplicaBuilder.createSwitchItem 的 BorderLayout.WEST+CENTER):
 * label 段 + 控件段两列 grid — 配合外层 RowsTree 的 `max-content 1fr` 轨道
 * (subgrid), 同列 label 等宽 → 控件垂直对齐 (= ResponsiveGridLayout 的
 * maxLabelWidthPerColumn 的 CSS 等价)。
 */
const RowLine: React.FC<{ label?: React.ReactNode; children?: React.ReactNode; full?: boolean }> = ({
  label,
  children,
  full,
}) => (
  <div
    className={`row-line${full ? ' full' : ''}`}
    style={full ? { gridColumn: '1 / -1' } : undefined}
  >
    {label}
    <span className="ctrl">{children}</span>
  </div>
)

/** SWITCH / SWITCH_INV / DATA (DATA 是开关, Java data toggles): label 左, 开关紧随 */
const SwitchRow: React.FC<RowProps> = ({ row, panel, values }) => {
  const key = rowKey(row)
  const local = values[key]
  const checked = typeof local === 'boolean' ? local : String(row.value ?? '').toLowerCase() === 'true'
  return (
    <RowLine label={<Label text={row.label} desc={row.desc} descImg={row.descImg}  />}> 
      <Switch
        checked={checked}
        onChange={(v) => sendFormMessage({ kind: 'Toggle', panel, key, value: v })}
      />
    </RowLine>
  )
}

/** SLIDER: 滑条 + 数值输入 (对位 Java WebSlider+WebSpinner); onChange 实时
 *  (拖拽期不落盘), onChangeComplete/输入失焦 → Save (valueIsAdjusting 语义) */
const SliderRow: React.FC<RowProps> = ({ row, panel, values }) => {
  const key = rowKey(row)
  const local = values[key]
  const initial = typeof local === 'number' ? local : Number(row.value ?? row.minVal)
  const [v, setV] = useState(initial)
  useEffect(() => setV(initial), [initial])
  const push = (nv: number, persist: boolean) => {
    setV(nv)
    const p = sendFormMessage({ kind: 'Slider', panel, key, value: nv })
    if (persist) p.then(() => sendFormMessage({ kind: 'Save' }))
  }
  return (
    <RowLine label={<Label text={row.label} desc={row.desc} descImg={row.descImg}  />}> 
      <Slider
        style={{ flex: 1, minWidth: 100, margin: '0 2px 0 0' }}
        min={row.minVal}
        max={row.maxVal}
        value={v}
        onChange={(nv) => push(nv, false)}
        onChangeComplete={(nv) => push(nv, true)}
      />
      <InputNumber
        size="small"
        style={{ width: 76 }}
        min={row.minVal}
        max={row.maxVal}
        value={v}
        formatter={(n) => (row.unit ? `${n} ${row.unit}` : `${n}`)}
        parser={(s) => parseInt(String(s).replace(/[^\d-]/g, ''), 10) || 0}
        onChange={(nv) => nv != null && push(nv, true)}
      />
    </RowLine>
  )
}

/** COMBO: _FONTS_/_CROSSHAIRS_/静态 options (Rust resolve_options 同源) */
const ComboRow: React.FC<RowProps> = ({ row, panel, values }) => {
  const key = rowKey(row)
  const local = values[key]
  const current = typeof local === 'string' ? local : String(row.value ?? '')
  const source = String(row.format ?? '') // cfg :format 承载 options 源 (Rust combo.rs 同)
  const [options, setOptions] = useState<string[]>([])
  useEffect(() => {
    getComboOptions(source, current)
      .then((opts) => setOptions(opts.length ? opts : [current]))
      .catch(() => setOptions([current]))
  }, [source, current])
  return (
    <RowLine label={<Label text={row.label} desc={row.desc} descImg={row.descImg}  />}> 
      <Select
        size="small"
        style={{ minWidth: 160 }}
        value={current || undefined}
        options={options.map((o) => ({ value: o, label: o }))}
        onChange={(v) => sendFormMessage({ kind: 'Combo', panel, key, value: v })}
      />
    </RowLine>
  )
}

/** COLOR: AntD ColorPicker (HSB 调色板 + alpha, 对位 Java ColorPickerPopup) + hex 输入 */
const ColorRow: React.FC<RowProps> = ({ row, panel, values }) => {
  const key = rowKey(row)
  const local = values[key]
  const rgba = parseColorValue(local ?? row.value)
  const [text, setText] = useState(rgbaToHex(rgba))
  useEffect(() => setText(rgbaToHex(parseColorValue(local ?? row.value))), [local, row.value])
  /** 提交 (Java ColorRowRenderer.applyColorChange: 每次选定即存, 主键十进制 + legacy 分键在 Rust apply) */
  const commit = (v: [number, number, number, number]) =>
    sendFormMessage({ kind: 'ColorPicked', panel, key, value: v })
  const commitHex = (t: string) => {
    const hex = t.replace('#', '')
    if (/^[0-9a-fA-F]{8}$/.test(hex) || /^[0-9a-fA-F]{6}$/.test(hex)) {
      const full = hex.length === 6 ? hex + 'FF' : hex
      commit([
        parseInt(full.slice(0, 2), 16),
        parseInt(full.slice(2, 4), 16),
        parseInt(full.slice(4, 6), 16),
        parseInt(full.slice(6, 8), 16),
      ])
    }
  }
  return (
    <RowLine label={<Label text={row.label} desc={row.desc} descImg={row.descImg}  />}> 
      <ColorPicker
        size="small"
        value={rgbaToHex(rgba)}
        onChangeComplete={(c) => {
          const { r, g, b, a } = c.toRgb()
          commit([r, g, b, Math.round(a * 255)])
        }}
      />
      <Input
        size="small"
        style={{ width: 110 }}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onPressEnter={() => commitHex(text)}
        onBlur={() => commitHex(text)}
      />
    </RowLine>
  )
}

/** TEXT / INPUT: Enter/失焦提交 (Java 语义; iced 版逐键, 以 Java 为准) */
const TextRow: React.FC<RowProps> = ({ row, panel, values }) => {
  const key = rowKey(row)
  const local = values[key]
  const [text, setText] = useState(String(local ?? row.value ?? ''))
  useEffect(() => setText(String(local ?? row.value ?? '')), [local, row.value])
  const commit = () => {
    if (text !== String(row.value ?? '')) {
      // 无 :target 的 INPUT 行落 row.value (Rust text::apply 经 Combo 链写 row.value)
      sendFormMessage({ kind: 'Combo', panel, key, value: text })
    }
  }
  return (
    <RowLine label={<Label text={row.label} desc={row.desc} descImg={row.descImg}  />}> 
      <Input
        size="small"
        style={{ width: 200 }}
        value={text}
        placeholder={row.property ? undefined : row.label}
        onChange={(e) => setText(e.target.value)}
        onPressEnter={commit}
        onBlur={commit}
      />
    </RowLine>
  )
}

/** BUTTON: resetConfig/factoryReset 前端确认 (Rust pending 语义保持) */
const ButtonRow: React.FC<RowProps> = ({ row }) => {
  const onClick = () => {
    const action = rowKey(row)
    if (action === 'resetConfig' || action === 'factoryReset') {
      Modal.confirm({
        title: action === 'factoryReset' ? '恢复出厂设置?' : '重置全部配置?',
        content:
          action === 'factoryReset'
            ? '将把全部配置恢复为出厂默认, 当前配置会先备份。'
            : '将把全部配置项重置为默认值。',
        onOk: async () => {
          await sendFormMessage({ kind: 'ButtonAction', action })
          await sendFormMessage({ kind: 'ConfirmPending' })
        },
      })
    } else {
      // open* 三键阶段④前为占位 (Rust 侧日志备案)
      sendFormMessage({ kind: 'ButtonAction', action }).then(() =>
        sendFormMessage({ kind: 'CancelPending' }),
      )
    }
  }
  return (
    <Button size="small" onClick={onClick}>
      {row.label || rowKey(row)}
    </Button>
  )
}

/** INFO: 只读长文本 + URL 自动链接 (Java InfoRowRenderer 的 JEditorPane HTML 超链接);
 *  占整行 (Java INFO 行不进网格槽, 全宽段落) */
const InfoRow: React.FC<{ row: RowDto }> = ({ row }) => (
  <div style={{ display: 'flex', gap: 8, alignItems: 'flex-start', padding: '2px 0', gridColumn: '1 / -1' }}>
    {row.label && (
      <Text strong style={{ whiteSpace: 'nowrap', fontSize: 14 }}>
        {row.label}
      </Text>
    )}
    <Typography.Paragraph style={{ fontSize: 14, whiteSpace: 'pre-wrap', marginBottom: 0, flex: 1 }}>
      {splitUrlText(String(row.value ?? '')).map((seg, i) =>
        seg.type === 'url' ? (
          <a key={i} href={seg.value} target="_blank" rel="noreferrer">
            {seg.value}
          </a>
        ) : (
          <span key={i}>{seg.value}</span>
        ),
      )}
    </Typography.Paragraph>
  </div>
)

/** VOICE / VOICE_GLOBAL: 语音包 Select + 试听占位 (Java VoiceRowRenderer/VoiceGlobalRenderer;
 *  播放依赖语音子系统装配, 阶段③ disabled + Tooltip 备案) */
const VoiceRow: React.FC<RowProps> = ({ row, panel, values }) => {
  const key = rowKey(row)
  const [packs, setPacks] = useState<string[]>([])
  useEffect(() => {
    voicePacksOnce().then(setPacks).catch(() => setPacks(['default']))
  }, [])
  // 值回退链: 本地态 → row.value → row.defaultValue (Java VoicePackConfig.parse 空值→default)
  const current = parseVoicePackValue(values[key] ?? row.value ?? row.defaultValue)
  return (
    <RowLine label={<Label text={row.label} desc={row.desc} descImg={row.descImg}  />}> 
      <Select
        size="small"
        style={{ minWidth: 140 }}
        value={current || undefined}
        options={packs.map((p) => ({ value: p, label: p }))}
        onChange={(v) => sendFormMessage({ kind: 'Combo', panel, key, value: v })}
      />
      <Tooltip title="试听待语音子系统装配">
        <Button size="small" disabled>
          ▶
        </Button>
      </Tooltip>
    </RowLine>
  )
}

/** FMLIST: FM 机型搜索下拉 + 对比占位 (Java FMListRowRenderer; 对比窗口属阶段④) */
const FmListRow: React.FC<RowProps> = ({ row, panel, values }) => {
  const key = rowKey(row)
  const [fms, setFms] = useState<string[]>([])
  useEffect(() => {
    fmListOnce().then(setFms).catch(() => setFms([]))
  }, [])
  const local = values[key]
  const current = typeof local === 'string' ? local : String(row.value ?? '')
  return (
    <RowLine label={<Label text={row.label} desc={row.desc} descImg={row.descImg}  />}> 
      <Select
        size="small"
        showSearch
        optionFilterProp="label"
        style={{ minWidth: 180 }}
        value={current || undefined}
        options={fms.map((f) => ({ value: f, label: f }))}
        onChange={(v) => sendFormMessage({ kind: 'Combo', panel, key, value: v })}
      />
      <Button size="small" onClick={() => message.info('对比窗口属阶段④')}>
        对比
      </Button>
    </RowLine>
  )
}

/** HOTKEY: 键录制按钮 (Java HotkeyRowRenderer; JS KeyboardEvent.code → VC 码转换,
 *  Escape 放弃, Lock 键/无映射键忽略; Rust 侧 CONFIG_CHANGED 自动换绑全局钩键) */
const HotkeyRow: React.FC<RowProps> = ({ row, panel, values }) => {
  const key = rowKey(row)
  const raw = values[key] ?? row.value
  const vc = typeof raw === 'number' ? raw : parseInt(String(raw ?? '0'), 10) || 0
  const [recording, setRecording] = useState(false)
  // 录制态: window 一次性 keydown; 卸载/退出录制必移除 (防泄漏 + 防复发)
  useEffect(() => {
    if (!recording) return
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault()
      if (e.code === 'Escape') {
        setRecording(false)
        return
      }
      const code = browserCodeToVc(e.code)
      if (code == null) return
      setRecording(false)
      sendFormMessage({ kind: 'Combo', panel, key, value: String(code) })
    }
    window.addEventListener('keydown', onKey, { once: true })
    return () => window.removeEventListener('keydown', onKey)
  }, [recording, panel, key])
  // 键名显示: VC→名称映射 (Java getKeyText 近似), 无映射兜底 "键码 N"; 0=未设置
  const name = vc === 0 ? '无' : (vcToKeyName(vc) ?? `键码 ${vc}`)
  return (
    <RowLine label={<Label text={row.label} desc={row.desc} descImg={row.descImg}  />}> 
      <Button size="small" style={{ minWidth: 90 }} onClick={() => setRecording((r) => !r)}>
        {recording ? '按键…' : name}
      </Button>
    </RowLine>
  )
}

/** 占位键 (FILELIST; cfg 实际 0 个) — 只读兜底 */
const FallbackRow: React.FC<RowProps> = ({ row }) => (
  <RowLine
    label={
      <Text type="secondary" style={{ fontSize: 14 }}>
        {row.label}
      </Text>
    }
  >
    {row.value != null && (
      <Text code style={{ fontSize: 13 }}>
        {String(row.value)}
      </Text>
    )}
    <Text type="secondary" style={{ fontSize: 12 }}>
      ({row.type})
    </Text>
  </RowLine>
)

/** 类型分发 (对位 vm-ui renderers/mod.rs view_row 分发) */
export const RowRenderer: React.FC<RowProps> = (props) => {
  switch (props.row.type.toUpperCase()) {
    case 'SWITCH':
    case 'SWITCH_INV':
    case 'DATA':
      return <SwitchRow {...props} />
    case 'SLIDER':
      return <SliderRow {...props} />
    case 'COMBO':
      return <ComboRow {...props} />
    case 'COLOR':
      return <ColorRow {...props} />
    case 'TEXT':
    case 'INPUT':
      return <TextRow {...props} />
    case 'BUTTON':
      return <ButtonRow {...props} />
    case 'INFO':
      return <InfoRow {...props} />
    case 'VOICE':
    case 'VOICE_GLOBAL':
      return <VoiceRow {...props} />
    case 'FMLIST':
      return <FmListRow {...props} />
    case 'HOTKEY':
      return <HotkeyRow {...props} />
    default:
      return <FallbackRow {...props} />
  }
}
