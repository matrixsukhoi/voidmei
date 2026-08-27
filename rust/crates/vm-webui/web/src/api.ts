/**
 * Rust IPC 封装 + cfg 树类型 (与 vm-webui dto.rs 一一对应, camelCase)
 */

export interface RowDto {
  label: string
  type: string
  property: string | null
  value: string | number | boolean | null
  defaultValue: string | number | boolean | null
  unit: string
  format: string
  desc: string | null
  descImg: string | null
  minVal: number
  maxVal: number
  groupColumns: number
  children: RowDto[]
}

export interface PanelDto {
  title: string
  x: number
  y: number
  alpha: number
  hotkey: number
  visible: boolean
  fontName: string | null
  fontSize: number
  columns: number
  panelColumns: number
  switchKey: string | null
  rows: RowDto[]
}

/** 表单消息 (FormMessageDto 的 tag 形态) */
export type FormMessage =
  | { kind: 'Toggle'; panel: string; key: string; value: boolean }
  | { kind: 'Slider'; panel: string; key: string; value: number }
  | { kind: 'Combo'; panel: string; key: string; value: string }
  | { kind: 'ColorPicked'; panel: string; key: string; value: [number, number, number, number] }
  | { kind: 'Save' }
  | { kind: 'StartGame' }
  | { kind: 'EndGame' }
  | { kind: 'RefreshPreviews' }
  | { kind: 'ButtonAction'; action: string }
  | { kind: 'ConfirmPending' }
  | { kind: 'CancelPending' }

import { invoke } from '@tauri-apps/api/core'

export const getLayoutTree = (): Promise<PanelDto[]> => invoke('get_layout_tree')
export const getComboOptions = (source: string, current: string): Promise<string[]> =>
  invoke('get_combo_options', { source, current })
export const sendFormMessage = (message: FormMessage): Promise<unknown> =>
  invoke('form_message', { message })

/** COLOR 行值解析: "232, 147, 50, 200" 十进制 / "#RRGGBBAA" hex → rgba (color.rs 同语义) */
export function parseColorValue(v: unknown): [number, number, number, number] {
  const s = String(v ?? '').trim()
  const dec = s.split(',').map((x) => parseInt(x.trim(), 10))
  if (dec.length === 4 && dec.every((x) => !Number.isNaN(x))) {
    return dec as [number, number, number, number]
  }
  const hex = s.replace('#', '')
  if (/^[0-9a-fA-F]{8}$/.test(hex)) {
    return [
      parseInt(hex.slice(0, 2), 16),
      parseInt(hex.slice(2, 4), 16),
      parseInt(hex.slice(4, 6), 16),
      parseInt(hex.slice(6, 8), 16),
    ]
  }
  return [255, 255, 255, 255]
}

export function rgbaToHex([r, g, b, a]: [number, number, number, number]): string {
  const h = (n: number) => n.toString(16).padStart(2, '0').toUpperCase()
  return `#${h(r)}${h(g)}${h(b)}${h(a)}`
}

// ---------------------------------------------------------------------------
// 阶段③扩展: 语音包/FM 列表/配置导入/资产根 IPC + cfg 值域纯函数
// ---------------------------------------------------------------------------

export const getVoicePacks = (): Promise<string[]> => invoke('get_voice_packs')
export const getFmList = (): Promise<string[]> => invoke('get_fm_list')
export const importConfig = (path: string): Promise<unknown> => invoke('import_config', { path })
export const getAssetRoot = (): Promise<string> => invoke('get_asset_root')

/** 模块级 promise 缓存 (整生命周期不变的只读 IPC 取一次); 失败置空下次重试 */
function once<T>(make: () => Promise<T>): () => Promise<T> {
  let p: Promise<T> | null = null
  return () => {
    if (!p) {
      p = make().catch((e) => {
        p = null
        throw e
      })
    }
    return p
  }
}
export const voicePacksOnce = once(getVoicePacks)
export const fmListOnce = once(getFmList)
export const assetRootOnce = once(getAssetRoot)

/** INFO 值切分段 (text/url) */
export interface TextSeg {
  type: 'text' | 'url'
  value: string
}

/** URL 自动链接切分: /(https?:\/\/[^\s)]+)/ (Java InfoRowRenderer.URL_PATTERN 同源;
 *  空白与 ')' 截断 URL, 防 "链接) 后文" 吞括号) */
export function splitUrlText(text: string): TextSeg[] {
  const out: TextSeg[] = []
  const re = /(https?:\/\/[^\s)]+)/g
  let last = 0
  for (const m of text.matchAll(re)) {
    const idx = m.index ?? 0
    if (idx > last) out.push({ type: 'text', value: text.slice(last, idx) })
    out.push({ type: 'url', value: m[1] })
    last = idx + m[1].length
  }
  if (last < text.length) out.push({ type: 'text', value: text.slice(last) })
  return out
}

/** desc-img 前缀归一: 已带 image/ (或 image\) 原样, 否则加前缀
 *  (Java ReplicaBuilder.java:625 同语义 — 最终统一落在资产根 image/ 目录) */
export function normalizeDescImg(name: string): string {
  return name.startsWith('image/') || name.startsWith('image\\') ? name : `image/${name}`
}

/** VOICE 值 → 包名: "pack:enabled" 取 pack 段, 空回退 default (Java VoicePackConfig.parse) */
export function parseVoicePackValue(v: unknown): string {
  const s = String(v ?? '').trim()
  if (!s) return 'default'
  return s.split(':')[0].trim() || 'default'
}

/**
 * KeyboardEvent.code → jnativehook VC 码 (Linux evdev/set-1 扫描码域, VC_P=25)。
 * displayFmKey 配置值与 Rust HotkeyManager 绑定键全部是该域 (vm-overlay hotkey.rs 备案);
 * 浏览器 e.keyCode 是另一体系 (P=80), 直发会绑错键 — 必须经本表转换。
 * Lock 三键返回 null (Java HotkeyRowRenderer 忽略); 无映射键返回 null (录制态忽略)。
 */
const CODE_TO_VC: Record<string, number> = {
  Escape: 1,
  Digit1: 2, Digit2: 3, Digit3: 4, Digit4: 5, Digit5: 6,
  Digit6: 7, Digit7: 8, Digit8: 9, Digit9: 10, Digit0: 11,
  Minus: 12, Equal: 13, Backspace: 14, Tab: 15,
  KeyQ: 16, KeyW: 17, KeyE: 18, KeyR: 19, KeyT: 20,
  KeyY: 21, KeyU: 22, KeyI: 23, KeyO: 24, KeyP: 25,
  BracketLeft: 26, BracketRight: 27, Enter: 28, ControlLeft: 29,
  KeyA: 30, KeyS: 31, KeyD: 32, KeyF: 33, KeyG: 34,
  KeyH: 35, KeyJ: 36, KeyK: 37, KeyL: 38,
  Semicolon: 39, Quote: 40, Backquote: 41, ShiftLeft: 42, Backslash: 43,
  KeyZ: 44, KeyX: 45, KeyC: 46, KeyV: 47, KeyB: 48, KeyN: 49, KeyM: 50,
  Comma: 51, Period: 52, Slash: 53, ShiftRight: 54,
  NumpadMultiply: 55, AltLeft: 56, Space: 57,
  CapsLock: 58, // Lock 三键: 录制态忽略 (Java 同)
  F1: 59, F2: 60, F3: 61, F4: 62, F5: 63, F6: 64, F7: 65, F8: 66, F9: 67, F10: 68,
  NumLock: 69, ScrollLock: 70, // 同上忽略
  Numpad7: 71, Numpad8: 72, Numpad9: 73, NumpadSubtract: 74,
  Numpad4: 75, Numpad5: 76, Numpad6: 77, NumpadAdd: 78,
  Numpad1: 79, Numpad2: 80, Numpad3: 81, Numpad0: 82, NumpadDecimal: 83,
  F11: 87, F12: 88,
  NumpadEnter: 96, ControlRight: 97, NumpadDivide: 98,
  AltRight: 100, Home: 102, ArrowUp: 103, PageUp: 104, ArrowLeft: 105, ArrowRight: 106,
  End: 107, ArrowDown: 108, PageDown: 109, Insert: 110, Delete: 111,
  MetaLeft: 125, MetaRight: 126, ContextMenu: 127,
}
const LOCK_CODES = new Set(['CapsLock', 'NumLock', 'ScrollLock'])

export function browserCodeToVc(code: string): number | null {
  if (LOCK_CODES.has(code)) return null
  return CODE_TO_VC[code] ?? null
}

/** code → 显示键名 (KeyP→P / Digit1→1 / 符号与方向键图形化) */
const CODE_DISPLAY: Record<string, string> = {
  Escape: 'Esc', Enter: 'Enter', Tab: 'Tab', Space: 'Space', Backspace: 'Backspace',
  BracketLeft: '[', BracketRight: ']', Semicolon: ';', Quote: "'", Backquote: '`',
  Backslash: '\\', Comma: ',', Period: '.', Slash: '/', Minus: '-', Equal: '=',
  ArrowUp: '↑', ArrowDown: '↓', ArrowLeft: '←', ArrowRight: '→',
  ShiftLeft: '左Shift', ShiftRight: '右Shift', ControlLeft: '左Ctrl', ControlRight: '右Ctrl',
  AltLeft: '左Alt', AltRight: '右Alt', MetaLeft: '左Win', MetaRight: '右Win', ContextMenu: '菜单',
  Delete: 'Delete', Insert: 'Insert', Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
  NumpadEnter: '小键盘Enter', NumpadAdd: '小键盘+', NumpadSubtract: '小键盘-',
  NumpadMultiply: '小键盘*', NumpadDivide: '小键盘/', NumpadDecimal: '小键盘.',
}
const codeDisplayName = (code: string): string => {
  if (CODE_DISPLAY[code]) return CODE_DISPLAY[code]
  if (/^Key[A-Z]$/.test(code)) return code.slice(3)
  if (/^Digit\d$/.test(code)) return code.slice(5)
  if (/^Numpad\d$/.test(code)) return `小键盘${code.slice(6)}`
  return code
}

/** VC 码 → 显示键名 (Java NativeKeyEvent.getKeyText 近似); 无映射返回 null (调用方兜底 "键码 N") */
export function vcToKeyName(vc: number): string | null {
  for (const [code, v] of Object.entries(CODE_TO_VC)) {
    if (v === vc) return codeDisplayName(code)
  }
  return null
}
