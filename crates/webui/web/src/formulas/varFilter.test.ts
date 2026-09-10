/**
 * 变量目录筛选/展示纯函数测试 (三栏重构): 三条件叠加过滤、来源 Tag 色、
 * 动态选项收集与固定排序、公式产出变量判定与最近值显示 (统一命名空间)。
 * 组件渲染不在此测 (tauri/antd 依赖 webview 环境)。
 */
import { describe, expect, it } from 'vitest'
import type { VarCatalogEntry } from '../api'
import {
  categoryCn,
  categoryOptions,
  filterVarEntries,
  formatVarValue,
  isFormulaVar,
  originOptions,
  originTagColor,
} from './varFilter'

/** 造一条目录项 (缺省字段按常见形态补齐) */
const v = (p: Partial<VarCatalogEntry>): VarCatalogEntry => ({
  name: 'x',
  unit: '',
  desc: '',
  category: 'Flight',
  origin: '内部计算',
  originKey: 'derived',
  ...p,
})

const VARS: VarCatalogEntry[] = [
  v({ name: 'ias', unit: 'km/h', desc: '表速', origin: '8111 /state', originKey: 'state' }),
  v({ name: 'throttle', desc: '油门', category: 'Engine', origin: '8111 /indicators', originKey: 'indicators' }),
  v({ name: 'fm.vne', unit: 'km/h', desc: '极限速度', category: 'Fm', origin: 'FM 文件', originKey: 'fm' }),
  v({ name: 'g_load', desc: '过载', origin: '运行时', originKey: 'meta' }),
  v({ name: 'G', desc: '重力常数', category: 'Const', origin: '常量', originKey: 'const' }),
  // 公式产出变量 (统一目录新增: originKey=formula / category=Formula;
  // mach 与系统变量同名 = 接管; my_var 最近帧求值无效 → value null)
  v({
    name: 'mach',
    desc: '公式: v_tas / 1225',
    category: 'Formula',
    origin: '公式',
    originKey: 'formula',
    value: 0.447,
    overridesSystem: true,
  }),
  v({ name: 'my_var', desc: '我的公式', category: 'Formula', origin: '公式', originKey: 'formula', value: null }),
]

describe('filterVarEntries (三条件叠加)', () => {
  it('空条件 = 全量', () => {
    expect(filterVarEntries(VARS, '', '', '')).toHaveLength(7)
  })
  it('关键字匹配名字或描述, 忽略大小写与首尾空白', () => {
    expect(filterVarEntries(VARS, ' IAS ', '', '').map((x) => x.name)).toEqual(['ias'])
    expect(filterVarEntries(VARS, '表速', '', '').map((x) => x.name)).toEqual(['ias'])
    expect(filterVarEntries(VARS, 'fm.', '', '').map((x) => x.name)).toEqual(['fm.vne'])
  })
  it('来源筛选 (originKey 精确)', () => {
    expect(filterVarEntries(VARS, '', 'indicators', '').map((x) => x.name)).toEqual(['throttle'])
    expect(filterVarEntries(VARS, '', 'state', '').map((x) => x.name)).toEqual(['ias'])
    // "公式" 来源 = 公式产出变量 (统一命名空间)
    expect(filterVarEntries(VARS, '', 'formula', '').map((x) => x.name)).toEqual(['mach', 'my_var'])
  })
  it('类别筛选 (category 精确)', () => {
    expect(filterVarEntries(VARS, '', '', 'Fm').map((x) => x.name)).toEqual(['fm.vne'])
    expect(filterVarEntries(VARS, '', '', 'Const').map((x) => x.name)).toEqual(['G'])
  })
  it('三条件叠加取交集; 无交集为空', () => {
    expect(filterVarEntries(VARS, '油门', 'indicators', 'Engine').map((x) => x.name)).toEqual(['throttle'])
    expect(filterVarEntries(VARS, '表速', 'indicators', '')).toEqual([]) // 名字命中但来源不符
  })
})

describe('originTagColor (来源 → Tag 色)', () => {
  it('七类来源各自配色, derived 为默认灰 (空串), 公式用 gold', () => {
    expect(originTagColor('state')).toBe('geekblue')
    expect(originTagColor('indicators')).toBe('cyan')
    expect(originTagColor('derived')).toBe('')
    expect(originTagColor('fm')).toBe('orange')
    expect(originTagColor('meta')).toBe('purple')
    expect(originTagColor('const')).toBe('green')
    expect(originTagColor('formula')).toBe('gold')
  })
  it('未知来源键兜底默认灰', () => {
    expect(originTagColor('unknown')).toBe('')
  })
})

describe('originOptions / categoryOptions (动态收集 + 固定排序)', () => {
  it('按 originKey 去重, label 用后端中文 origin, 按固定序排列', () => {
    // 数据序故意乱放 (fm 在 state 前), 期望输出仍按 state → fm 序
    const vars = [v({ origin: 'FM 文件', originKey: 'fm' }), v({ origin: '8111 /state', originKey: 'state' })]
    expect(originOptions(vars)).toEqual([
      { value: 'state', label: '8111 /state' },
      { value: 'fm', label: 'FM 文件' },
    ])
  })
  it('未知来源键排在固定七类 (含 formula) 之后', () => {
    const vars = [
      v({ origin: '新来源', originKey: 'zz' }),
      v({ origin: '常量', originKey: 'const' }),
      v({ origin: '公式', originKey: 'formula' }),
    ]
    expect(originOptions(vars).map((o) => o.value)).toEqual(['const', 'formula', 'zz'])
  })
  it('类别去重 + 中文映射 + 固定序 (Formula = 公式变量, 排尾部)', () => {
    expect(categoryOptions(VARS).map((o) => o.label)).toEqual(['飞行', '引擎', 'FM 数据', '常量', '公式变量'])
  })
  it('未知类别回退原串', () => {
    expect(categoryCn('Flight')).toBe('飞行')
    expect(categoryCn('NewCat')).toBe('NewCat')
  })
})

describe('isFormulaVar / formatVarValue (公式产出变量)', () => {
  it('originKey=formula 判定为公式产出变量', () => {
    expect(VARS.filter(isFormulaVar).map((x) => x.name)).toEqual(['mach', 'my_var'])
  })
  it('最近帧值: 有限值 3 位小数 "= 0.447", null/缺键 (NaN) 为 "-"', () => {
    expect(formatVarValue(VARS[5])).toBe('= 0.447') // mach, value=0.447
    expect(formatVarValue(VARS[6])).toBe('-') // my_var, value=null
    expect(formatVarValue(VARS[0])).toBe('-') // ias, 系统变量无 value 键
    expect(formatVarValue(v({ originKey: 'formula', value: 1234.5678 }))).toBe('= 1234.568')
  })
})
