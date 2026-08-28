/**
 * 对比窗口纯展示函数测试: 数据→渲染映射 (行值配色/网格轨道/选择器过滤/recent 维护)。
 * (IPC/React 面依赖 webview, 不在此测 — Rust 侧 commands_windows 已有数据面单测。)
 */
import { describe, expect, it } from 'vitest'
import { CMP_COLORS, filterPlanes, gridTemplate, pushRecent, rowColors } from './comparisonPresent'

describe('CMP_COLORS (CompactComparisonWindow.java:165-173 RGB 照搬)', () => {
  it('关键色值逐个钉住 (防抄写漂移)', () => {
    expect(CMP_COLORS.bg).toBe('#121212') // (18,18,18)
    expect(CMP_COLORS.textSecondary).toBe('#DCDCDC') // (220,220,220)
    expect(CMP_COLORS.accentBetter).toBe('#2EFF71') // (46,255,113)
    expect(CMP_COLORS.accentWorse).toBe('#FF3C3C') // (255,60,60)
    expect(CMP_COLORS.headerA).toBe('#00DCFF') // (0,220,255)
    expect(CMP_COLORS.headerB).toBe('#FF50B4') // (255,80,180)
    expect(CMP_COLORS.symbol).toBe('#FFD700') // (255,215,0)
    expect(CMP_COLORS.copy).toBe('#2196F3') // Blue 500
    expect(CMP_COLORS.close).toBe('#B71C1C') // Red 900
  })
})

describe('rowColors (addComparisonRow :138/:156 胜负配色)', () => {
  it('win=-1 左胜: v0 绿 v1 红; win=1 镜像; 平局双灰', () => {
    expect(rowColors(-1, false)).toEqual({ c0: CMP_COLORS.accentBetter, c1: CMP_COLORS.accentWorse })
    expect(rowColors(1, false)).toEqual({ c0: CMP_COLORS.accentWorse, c1: CMP_COLORS.accentBetter })
    expect(rowColors(0, false)).toEqual({ c0: CMP_COLORS.textSecondary, c1: CMP_COLORS.textSecondary })
  })
  it('单机模式无比较色 (Java !singleMode 守卫)', () => {
    expect(rowColors(-1, true)).toEqual({ c0: CMP_COLORS.textSecondary, c1: CMP_COLORS.textSecondary })
    expect(rowColors(1, true)).toEqual({ c0: CMP_COLORS.textSecondary, c1: CMP_COLORS.textSecondary })
  })
})

describe('gridTemplate (GridBag weightx 列宽比例)', () => {
  it('单机 0.4/0.6; 双机 0.35/0.25/0.15/0.25', () => {
    expect(gridTemplate(true)).toBe('40fr 60fr')
    expect(gridTemplate(false)).toBe('35fr 25fr 15fr 25fr')
  })
})

describe('filterPlanes (GridSelectorDialog.java:165-188 启发式过滤)', () => {
  const planes = ['f-16a', 'su-27', 'mig-29', 'spitfire_f24', 'a6m5_zero']
  it('All 全放行; 搜索子串忽略大小写', () => {
    expect(filterPlanes(planes, '', 'All')).toEqual(planes)
    expect(filterPlanes(planes, 'SPIT', 'All')).toEqual(['spitfire_f24'])
    expect(filterPlanes(planes, 'zero', 'All')).toEqual(['a6m5_zero'])
  })
  it('WWII 排除 f-16/su-27 (Java mock 判据)', () => {
    expect(filterPlanes(planes, '', 'WWII')).toEqual(['mig-29', 'spitfire_f24', 'a6m5_zero'])
  })
  it('Modern 要求 f-16/su-27/mig-29', () => {
    expect(filterPlanes(planes, '', 'Modern')).toEqual(['f-16a', 'su-27', 'mig-29'])
  })
  it('Red/Blue 无条件分支 — Java 原样全放行 (mock)', () => {
    expect(filterPlanes(planes, '', 'Red')).toEqual(planes)
    expect(filterPlanes(planes, '', 'Blue')).toEqual(planes)
  })
  it('搜索与过滤叠加: Modern 域内再搜', () => {
    expect(filterPlanes(planes, 'mig', 'Modern')).toEqual(['mig-29'])
    expect(filterPlanes(planes, 'spit', 'Modern')).toEqual([])
  })
})

describe('pushRecent (GridSelectorDialog.createPlaneButton :196-199)', () => {
  it('新机型头插; 已有项去重后头插', () => {
    expect(pushRecent([], 'a')).toEqual(['a'])
    expect(pushRecent(['a', 'b'], 'a')).toEqual(['a', 'b'])
    expect(pushRecent(['a', 'b'], 'c')).toEqual(['c', 'a', 'b'])
  })
  it('上限 5 (Java remove(5))', () => {
    expect(pushRecent(['1', '2', '3', '4', '5'], '6')).toEqual(['6', '1', '2', '3', '4'])
  })
})
