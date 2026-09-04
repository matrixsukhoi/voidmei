// W4 HUD 布局编辑器类型面 (与 Rust dto 对齐: camelCase serde)

export interface ComponentCatalogEntry {
  typeName: string;
  displayZh: string;
  category: 'Text' | 'Gauge' | 'Chart' | 'List' | 'Composite' | 'Decor';
  composite: boolean;
  configKeys: string[];
  dataShorts: string[];
}

export interface PageSummary {
  id: string;
  name: string;
  switchKey: string | null;
  isFactory: boolean;
  componentCount: number;
  upgradeAvailable: boolean;
}

export interface UpgradeHint {
  id: string;
  userVersion: number;
  factoryVersion: number;
}

export interface ComponentDoc {
  id: string;
  type: string;
  pos: [number, number];
  anchor: [string, string];
  parent: string | null;
  visibleWhen?: string | null;
  enabled: boolean;
  props: Record<string, unknown>;
}

export interface PageDoc {
  id: string;
  name: string;
  switchKey: string | null;
  entryKey?: string | null;
  strategyExtra?: string | null;
  pos?: [number, number] | null;
  padding: number;
  /** 画布语义 (null = 自由画布; "minihud" = ctx 派生画布) */
  canvas?: string | null;
  font: { family: string; sizeAdd: number; scaleSource: string };
  contentVersion: number;
  components: ComponentDoc[];
}

export interface SolveItem {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface SolveResult {
  lineHeightPx: number;
  pageW: number;
  pageH: number;
  items: SolveItem[];
  png: number[];
}
