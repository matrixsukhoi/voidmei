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
/** 试听语音 (Java VoiceRowRenderer 播放按钮: loadClip + setFramePosition(0) + start;
 *  key = voice_<alert> 配置键, pack = 当前选中包, 失败静默无声 — Java clip==null 同款) */
export const previewVoice = (key: string, pack: string): Promise<unknown> =>
  invoke('preview_voice', { key, pack })
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

/** VOICE 值 → 包名: "pack|enabled" 取 pack 段, 空回退 default (Java VoicePackConfig.parse
 *  — 分隔符是 '|' 不是 ':', Java toConfigString 序列化为 "packName|enabled";
 *  修复: 原实现误切 ':' 会让 Java 版存档值 "jarvis|true" 整串当包名, 试听/下拉全错)。
 *  PORT: 比 Java parse 多 trim 首/尾空白 — Java parse(" jarvis|true") 得包名 " jarvis";
 *  配置值实际来自 toConfigString/下拉选项不含空白, 该差异不可达 (审查 A-W2 备案) */
export function parseVoicePackValue(v: unknown): string {
  const s = String(v ?? '').trim()
  if (!s) return 'default'
  return s.split('|')[0] || 'default'
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

// ---------------------------------------------------------------------------
// 批3小件: 应用版本 IPC + checkUpdate 纯解析函数 (Application.java:451-484)
// ---------------------------------------------------------------------------

/** 应用版本号 (Java Application.readVersion ↔ VOIDMEI_VERSION 构建注入, 缺省 "dev";
 *  vm-webui commands.rs get_app_version — 标题栏与更新检查同源) */
export const getAppVersion = (): Promise<string> => invoke('get_app_version')

/**
 * GitHub releases/latest 响应文本 → 最新版本号。Java Application.checkUpdate 的
 * 截取+正则逐句对位 (Application.java:464-472):
 *   indexOf("tag_name") → indexOf(",", sidx) → substring → Pattern "[0-9].([0-9])*" find
 * 失败面 (统一 null, 调用方静默继续): 无 tag_name / 无后续逗号 (Java substring
 * 越界抛异常 → 线程池吞掉等价静默) / 正则不中 (m.find()==false 分支)。
 */
export function extractLatestVersion(res: string): string | null {
  const sidx = res.indexOf('tag_name')
  if (sidx === -1) return null
  const eidx = res.indexOf(',', sidx)
  if (eidx === -1) return null
  const seg = res.substring(sidx, eidx) // Java: res.substring(sidx, eidx)
  const m = seg.match(/[0-9].([0-9])*/) // Java: Pattern.compile("[0-9].([0-9])*")
  return m ? m[0] : null
}

/**
 * 版本比较 (Java: Double.parseDouble(version) < Double.parseDouble(latest),
 * Application.java:474)。parseFloat 与 Double.parseDouble 同为最长数值前缀解析;
 * 输入恒为正则产物 (数字+单字符+数字), 无 NaN 面。
 */
export function hasNewerVersion(local: string, latest: string): boolean {
  return parseFloat(local) < parseFloat(latest)
}

// ---------------------------------------------------------------------------
// 批3 web 窗口域: 对比/功率曲线窗口的数据 IPC (commands_windows.rs W1 直算命令)
// ---------------------------------------------------------------------------

/** 对比窗口一行 (ComparisonRowDto; camelCase) */
export interface ComparisonRow {
  isHeader: boolean
  text: string
  value0: string | null
  value1: string | null
  /** -1=左胜(v0) 0=平 1=右胜(v1) */
  win: number
  symbol: string
}

/** 对比窗口全量数据 (ComparisonDataDto) */
export interface ComparisonData {
  fm0Name: string
  fm1Name: string | null
  singleMode: boolean
  title: string
  rows: ComparisonRow[]
  copyText: string
}

/** 功率曲线拐点标注 (InflectionPointDto; kind 换语义标着色) */
export interface InflectionPoint {
  kind: 'peak' | 'valley' | 'kink' | string
  label: string
  altitudeM: number
  power: number
}

/** 单条功率曲线 (PowerCurveDto) */
export interface PowerCurve {
  fmName: string
  valid: boolean
  powerCurve: number[]
  altStep: number
  maxDisplayAlt: number
  maxPower: number
  minPower: number
  peakAltitude: number
  inflectionPoints: InflectionPoint[]
  errorMessage: string | null
}

/** 功率曲线窗口全量数据 (PowerCurveDataDto) */
export interface PowerCurveData {
  fm0Name: string
  fm1Name: string | null
  dualMode: boolean
  speedKmh: number
  wepMode: boolean
  curve0: PowerCurve
  curve1: PowerCurve | null
  displayMaxPower: number
  displayMinPower: number
  errorMessage: string | null
}

/** 对比窗口数据 (W1 直算; fm1 null/空 = 单机数据视图) */
export const getComparisonData = (fm0: string, fm1: string | null): Promise<ComparisonData> =>
  invoke('comparison_data', { fm0, fm1 })
/** 功率曲线窗口数据 (W1 直算; fm1 空/==fm0 归一单曲线在 Rust 侧) */
export const getPowerCurveData = (
  fm0: string,
  fm1: string | null,
  speedKmh: number,
  wep: boolean,
): Promise<PowerCurveData> => invoke('power_curve_data', { fm0, fm1, speedKmh, wep })
/** 机型列表 (GridSelectorDialog.loadPlanes 对位; 与设置页 get_fm_list 分通道不混用) */
export const getWebFmList = (): Promise<string[]> => invoke('fm_list')
/** 打开对比 web 窗口 (FMLIST 行 对比按钮 — 选中机型单机视图; 经主线程 dispatcher) */
export const openComparisonWindow = (fm0: string, fm1: string | null): Promise<unknown> =>
  invoke('open_comparison_window', { fm0, fm1 })

// ---------------------------------------------------------------------------
// 公式系统 IPC (commands_formula.rs 全部 7 个命令; camelCase DTO 一一对应)
// ---------------------------------------------------------------------------

/** 公式条目 (FormulaItemDto; error 只读由后端编译时生成, 提交时后端忽略) */
export interface FormulaItem {
  name: string
  expr: string
  unit: string
  precision: number
  desc: string
  disabled: boolean
  builtin: boolean
  /** Java getter 别名 (:getter; 内置公式的 overlay 面板绑定键, 保存往返保留) */
  getter: string | null
  error: string | null
}

/** 校验/试算/保存结果 (FormulaEvalDto; NaN/inf 经 JSON 序列化为 null) */
export interface FormulaEval {
  ok: boolean
  value: number | null
  error: string | null
}

/** 变量目录条目 (VarCatalogEntryDto; 统一命名空间 = 系统变量 + 公式产出变量,
 *  category = Flight/Engine/State/Limit/Fm/Meta/Const/Formula — "公式即变量" 设计 §5) */
export interface VarCatalogEntry {
  name: string
  unit: string
  desc: string
  category: string
  /** 数据来源中文标签 ("8111 /state" | "8111 /indicators" | "内部计算" | "FM 文件" | "运行时" | "常量" | "公式") */
  origin: string
  /** 来源筛选键 ("state" | "indicators" | "derived" | "fm" | "meta" | "const" | "formula") */
  originKey: string
  /** 公式产出变量最近一帧值 (serde skip None — 系统变量条目无此键; null = 最近帧求值无效) */
  value?: number | null
  /** 公式与系统变量同名 = 接管其值 (如内置 mach; serde skip false — 键缺省即未接管) */
  overridesSystem?: boolean
}

/** 最近一帧变量快照 (试算 "当前数据" 用; names 与 values 下标一一对应) */
export interface VarSnapshot {
  names: string[]
  values: (number | null)[]
}

/** 公式列表 (含编译错误标注) */
export const getFormulaList = (): Promise<FormulaItem[]> => invoke('get_formula_list')
/** 单公式校验 (语法/未知符号/arity, 不求值状态原语) */
export const formulaValidate = (expr: string): Promise<FormulaEval> =>
  invoke('formula_validate', { expr })
/** 单公式试算 (对最近一帧数据求值) */
export const formulaTryEval = (expr: string): Promise<FormulaEval> =>
  invoke('formula_try_eval', { expr })
/** 变量目录 (统一目录 = 系统变量 + 公式产出变量; 公式条目带最近帧值与接管标志) */
export const getVarCatalog = (): Promise<VarCatalogEntry[]> => invoke('get_var_catalog')
/** 最近一帧变量快照 */
export const getLastVarSnapshot = (): Promise<VarSnapshot> => invoke('get_last_var_snapshot')
/** 全量保存 + 热更新 (编辑器保存链; builtin 条目保留标志 = 用户覆盖内置) */
export const saveFormulas = (items: FormulaItem[]): Promise<FormulaEval> =>
  invoke('save_formulas', { items })
/** 恢复出厂公式 (删除自定义 + 重置内置覆盖) */
export const resetFormulas = (): Promise<FormulaEval> => invoke('reset_formulas')
