// 布局编辑器只读命令 (写面全走 editApi 的编辑会话命令)
import type {
  CatalogResponse,
  PageDoc,
  PageSummary,
  UpgradeHint,
} from './types';

import { invoke } from '@tauri-apps/api/core'

export async function getComponentCatalog(): Promise<CatalogResponse> {
  return invoke('get_component_catalog');
}

export async function getPages(): Promise<{
  pages: PageSummary[];
  docs: PageDoc[];
  /** 出厂文档全量 (编辑控制台「恢复出厂页」的源) */
  factoryDocs: PageDoc[];
  upgradeHints: UpgradeHint[];
}> {
  return invoke('get_pages');
}


