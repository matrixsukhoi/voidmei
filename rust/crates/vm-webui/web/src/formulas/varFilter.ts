/**
 * 变量目录的筛选/展示纯函数 (VarCatalog 面板与单测共用, 与 antd 解耦):
 * - filterVarEntries: 关键字 × 来源 × 类别 三条件叠加过滤;
 * - originTagColor: 来源筛选键 → AntD Tag 预设色 (derived 用默认灰 = 空 color);
 * - originOptions/categoryOptions: 选项从数据动态收集 (防后端新增漏配), 按固定序排列;
 * - isFormulaVar/formatVarValue: 公式产出变量 (统一目录新增来源) 的判定与最近值显示。
 */
import type { VarCatalogEntry } from '../api'

/** 变量类别 → 中文 (registry.rs VarCategory Debug 串对位; Formula = 公式产出变量) */
export const CATEGORY_CN: Record<string, string> = {
  Flight: '飞行',
  Engine: '引擎',
  State: '状态',
  Limit: '限制',
  Fm: 'FM 数据',
  Meta: '元信息',
  Const: '常量',
  Formula: '公式变量',
}
export const categoryCn = (c: string): string => CATEGORY_CN[c] ?? c

/** 来源键 → Tag 色 (任务规格; derived 无 color = AntD 默认灰; formula 用 gold 区分公式产出) */
export const ORIGIN_COLOR: Record<string, string> = {
  state: 'geekblue',
  indicators: 'cyan',
  derived: '',
  fm: 'orange',
  meta: 'purple',
  const: 'green',
  formula: 'gold',
}
/** 未知来源键兜底为默认灰 */
export const originTagColor = (originKey: string): string => ORIGIN_COLOR[originKey] ?? ''

/** 是否公式产出变量 (统一目录里 originKey === "formula" 的条目) */
export const isFormulaVar = (v: VarCatalogEntry): boolean => v.originKey === 'formula'

/**
 * 公式变量最近帧值显示串: 有限值 "= 0.447" (3 位小数, 编辑器试算结果同格式),
 * 无数据/非有限 (键缺省或 null=NaN) → "-"。
 */
export function formatVarValue(v: VarCatalogEntry): string {
  return v.value != null && Number.isFinite(v.value) ? `= ${v.value.toFixed(3)}` : '-'
}

/** 来源/类别选项的固定展示顺序 (未知项排尾部, 防新枚举错位) */
const ORIGIN_ORDER = ['state', 'indicators', 'derived', 'fm', 'meta', 'const', 'formula']
const CATEGORY_ORDER = Object.keys(CATEGORY_CN)
const orderOf = (order: string[], key: string): number => {
  const i = order.indexOf(key)
  return i < 0 ? order.length : i
}

export interface FilterOption {
  value: string
  label: string
}

/** 来源下拉选项: 按 originKey 去重, label 用后端中文 origin 标签 */
export function originOptions(vars: VarCatalogEntry[]): FilterOption[] {
  const seen = new Map<string, string>()
  for (const v of vars)
    if (!seen.has(v.originKey)) seen.set(v.originKey, v.origin)
  return [...seen.entries()]
    .sort((a, b) => orderOf(ORIGIN_ORDER, a[0]) - orderOf(ORIGIN_ORDER, b[0]))
    .map(([value, label]) => ({ value, label }))
}

/** 类别下拉选项: 按 category 去重, label 用中文映射 */
export function categoryOptions(vars: VarCatalogEntry[]): FilterOption[] {
  const seen = new Set<string>()
  for (const v of vars) seen.add(v.category)
  return [...seen]
    .sort((a, b) => orderOf(CATEGORY_ORDER, a) - orderOf(CATEGORY_ORDER, b))
    .map((value) => ({ value, label: categoryCn(value) }))
}

/**
 * 三条件叠加过滤 (全部 '' = 不过滤):
 * - kw: 名字/描述模糊匹配, 忽略大小写与首尾空白;
 * - originKey / category: 精确匹配。
 */
export function filterVarEntries(
  vars: VarCatalogEntry[],
  kw: string,
  originKey: string,
  category: string,
): VarCatalogEntry[] {
  const k = kw.trim().toLowerCase()
  return vars.filter(
    (v) =>
      (!originKey || v.originKey === originKey) &&
      (!category || v.category === category) &&
      (!k || v.name.toLowerCase().includes(k) || v.desc.toLowerCase().includes(k)),
  )
}
