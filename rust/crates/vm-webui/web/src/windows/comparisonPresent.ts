/**
 * 对比窗口纯展示函数 (CompactComparisonWindow.java 布局/配色/合并的选择器逻辑)。
 * 无 React/antd/tauri 依赖 — vitest 直接钉住数据→渲染映射。
 *
 * 备案: Java 源无斑马纹 (initUI 只按胜负着色), 任务书"行斑马纹"与源不符 —
 * 按 PORTING.md 行为保真以 Java 为准, 未加。
 */

/** Java CompactComparisonWindow.java:165-173 调色板 (RGB 照搬) */
export const CMP_COLORS = {
  bg: '#121212', // BG_COLOR (18,18,18)
  textPrimary: '#FFFFFF', // TEXT_PRIMARY 纯白
  textSecondary: '#DCDCDC', // TEXT_SECONDARY 亮灰 (220,220,220)
  accentBetter: '#2EFF71', // ACCENT_BETTER 霓虹绿 (46,255,113)
  accentWorse: '#FF3C3C', // ACCENT_WORSE 亮红 (255,60,60)
  headerA: '#00DCFF', // HEADER_A 亮青 (0,220,255)
  headerB: '#FF50B4', // HEADER_B 亮粉 (255,80,180)
  symbol: '#FFD700', // SYMBOL_COLOR 金 (255,215,0)
  copy: '#2196F3', // COPY 底 Blue 500 (33,150,243)
  copyHover: '#42A5F5', // hover Blue 400 (66,165,245)
  close: '#B71C1C', // CLOSE 底 Red 900 (183,28,28)
  closeHover: '#D32F2F', // hover Red 700 (211,47,47)
} as const

/**
 * 行值配色 (addComparisonRow :138/:156): 左值 win==-1 绿 / win==1 红 / 平灰;
 * 右值镜像; 单机模式无比较色 (恒灰)。
 */
export function rowColors(win: number, singleMode: boolean): { c0: string; c1: string } {
  if (singleMode) return { c0: CMP_COLORS.textSecondary, c1: CMP_COLORS.textSecondary }
  const c0 =
    win === -1 ? CMP_COLORS.accentBetter : win === 1 ? CMP_COLORS.accentWorse : CMP_COLORS.textSecondary
  const c1 =
    win === 1 ? CMP_COLORS.accentBetter : win === -1 ? CMP_COLORS.accentWorse : CMP_COLORS.textSecondary
  return { c0, c1 }
}

/**
 * 网格列轨道 (GridBag weightx): 单机 0.4/0.6 (值列左对齐), 双机 0.35/0.25/0.15/0.25
 * (表头 CENTER, 值列 LEFT, 符号列两侧 15px insets)。
 */
export function gridTemplate(singleMode: boolean): string {
  return singleMode ? '40fr 60fr' : '35fr 25fr 15fr 25fr'
}

/**
 * GridSelectorDialog.filter (GridSelectorDialog.java:165-188) 逐句对位:
 * 搜索子串 (toLowerCase contains) + 启发式过滤 — WWII 排除 f-16/su-27,
 * Modern 要求 f-16/su-27/mig-29; Red/Blue 无条件分支 (Java mock 原样, 全放行)。
 */
export function filterPlanes(planes: string[], query: string, filter: string): string[] {
  const q = query.toLowerCase()
  return planes.filter((plane) => {
    let matches = plane.toLowerCase().includes(q)
    // Basic Heuristic Filtering (Mock implementation) — 原注释语义
    if (matches) {
      if (filter === 'WWII' && (plane.includes('f-16') || plane.includes('su-27'))) matches = false
      if (
        filter === 'Modern' &&
        !(plane.includes('f-16') || plane.includes('su-27') || plane.includes('mig-29'))
      )
        matches = false
    }
    return matches
  })
}

/**
 * GridSelectorDialog.createPlaneButton 的 recent 维护 (:196-199): 移除已有项 →
 * 头插 → 截断到 5 (Java recentPlanes.remove/add(0)/remove(5))。纯函数返回新数组。
 */
export function pushRecent(recent: readonly string[], plane: string): string[] {
  const next = recent.filter((p) => p !== plane)
  next.unshift(plane)
  if (next.length > 5) next.length = 5
  return next
}
