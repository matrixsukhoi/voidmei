// W4 布局编辑器命令封装 (六命令 + 快照 PNG 转换)
import type {
  CatalogResponse,
  PageDoc,
  PageSummary,
  SolveResult,
  UpgradeHint,
} from './types';

import { invoke } from '@tauri-apps/api/core'

export async function getComponentCatalog(): Promise<CatalogResponse> {
  return invoke('get_component_catalog');
}

export async function getPages(): Promise<{
  pages: PageSummary[];
  docs: PageDoc[];
  upgradeHints: UpgradeHint[];
}> {
  return invoke('get_pages');
}

export async function solvePage(page: PageDoc): Promise<SolveResult> {
  return invoke('solve_page', { page });
}

export async function savePage(page: PageDoc): Promise<void> {
  await invoke('save_page', { page });
}

export async function deletePage(id: string): Promise<void> {
  await invoke('delete_page', { id });
}

export async function resetPageToFactory(id: string): Promise<void> {
  await invoke('reset_page_to_factory', { id });
}

/** PNG (Rust 侧已 base64) → data URL (画布底图) */
export function pngToDataUrl(b64: string): string {
  return `data:image/png;base64,${b64}`;
}
