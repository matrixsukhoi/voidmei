// R7 真窗编辑会话命令封装 (begin/end/editCommand 三件)
import { invoke } from '@tauri-apps/api/core'

export async function beginEditSession(): Promise<void> {
  await invoke('begin_edit_session')
}

export async function endEditSession(commit: boolean): Promise<void> {
  await invoke('end_edit_session', { commit })
}

/** 编辑命令 (载荷 = Rust EditCommand 的 serde camelCase; kind 标签分发) */
export async function editCommand(payload: Record<string, unknown>): Promise<void> {
  await invoke('edit_command', { payload })
}
