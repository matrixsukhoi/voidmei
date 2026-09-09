/**
 * 撤销/重做 (PageDoc 快照栈): 全部修改走 patchPage 单通道, 此处只存快照 —
 * PageDoc 是小 JSON, 快照栈覆盖所有修改路径 (含未来新增); command pattern
 * 需为每类操作写 inverse, 不成比例。
 * coalescing: 同 coalesceKey 的连续提交在窗口期内只替换栈顶 (保留首个前态),
 * 拖动/连按/连续输入合并为一步。
 */
import { useCallback, useEffect, useRef, useState } from 'react'
import type { PageDoc } from './types'

const STACK_LIMIT = 50
/** 同 key 合并窗口 (ms) */
const COALESCE_MS = 500

export interface PageHistoryApi {
  /** 提交一次修改 (prev = 修改前快照, next = 修改后) */
  commit: (prev: PageDoc, next: PageDoc, coalesceKey?: string) => void
  /** 撤销: 返回应回退到的快照 (null = 无可撤销) */
  undo: (current: PageDoc) => PageDoc | null
  /** 重做: 返回应前进到的快照 (null = 无可重做) */
  redo: (current: PageDoc) => PageDoc | null
  canUndo: boolean
  canRedo: boolean
}

export function usePageHistory(pageId: string): PageHistoryApi {
  const past = useRef<PageDoc[]>([])
  const future = useRef<PageDoc[]>([])
  const lastKey = useRef<{ key: string; at: number } | null>(null)
  const [, force] = useState(0)
  const rerender = useCallback(() => force(n => n + 1), [])

  // 切页清栈 (历史不跨页)
  useEffect(() => {
    past.current = []
    future.current = []
    lastKey.current = null
    rerender()
  }, [pageId, rerender])

  const commit = useCallback(
    (prev: PageDoc, _next: PageDoc, coalesceKey?: string) => {
      // 合并窗口内的同 key 提交: 丢弃 prev (保留首个前态), 只留 next
      const now = Date.now()
      const canMerge =
        coalesceKey != null &&
        lastKey.current?.key === coalesceKey &&
        now - lastKey.current.at < COALESCE_MS
      if (!canMerge) {
        past.current.push(prev)
        if (past.current.length > STACK_LIMIT) past.current.shift()
      }
      future.current = []
      lastKey.current = coalesceKey != null ? { key: coalesceKey, at: now } : null
      rerender()
    },
    [rerender],
  )

  const undo = useCallback(
    (current: PageDoc): PageDoc | null => {
      const prev = past.current.pop()
      if (!prev) return null
      future.current.push(current)
      lastKey.current = null
      rerender()
      return prev
    },
    [rerender],
  )

  const redo = useCallback(
    (current: PageDoc): PageDoc | null => {
      const next = future.current.pop()
      if (!next) return null
      past.current.push(current)
      lastKey.current = null
      rerender()
      return next
    },
    [rerender],
  )

  return {
    commit,
    undo,
    redo,
    canUndo: past.current.length > 0,
    canRedo: future.current.length > 0,
  }
}
