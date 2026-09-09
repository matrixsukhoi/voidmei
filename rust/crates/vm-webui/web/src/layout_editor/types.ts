// W4 HUD 布局编辑器类型面 (与 Rust dto 对齐: camelCase serde)

/** 属性表单项 (Rust PropSchema; kind → 控件映射) */
export interface PropSchemaEntry {
  key: string;
  displayZh: string;
  kind: 'Bool' | 'Int' | 'Str' | 'Color' | 'Target' | 'Enum';
  /** Enum kind 的可选值 */
  values?: string[];
}

export interface ComponentCatalogEntry {
  typeName: string;
  displayZh: string;
  category: 'Text' | 'Gauge' | 'Chart' | 'List' | 'Composite' | 'Decor';
  composite: boolean;
  configKeys: string[];
  dataShorts: string[];
  propsSchema: PropSchemaEntry[];
  /** palette 新建组件的合法初值 (Rust 工厂必填项兜底 — 空 props 工厂 Err 组件不建) */
  defaultProps?: Record<string, unknown>;
}

/** 常用字段预设 (出厂页 data.field 原样导出 — 点击即完整配置) */
export interface FieldPreset {
  label: string
  props: Record<string, unknown>
}

export interface CatalogResponse {
  components: ComponentCatalogEntry[]
  fieldPresets: FieldPreset[]
}

/** 激活策略 (声明式; null = 恒显) */
export interface ActivationSpec {
  key: string;
  /** 附加条件 ("jet" = 喷气机; "live" = 仅游戏态) */
  requires?: string[];
}

export interface PageSummary {
  id: string;
  name: string;
  activation: ActivationSpec | null;
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
  /** 尺寸覆盖 (物理 px; null = 内容自适应 — 真窗 resize 手柄写回) */
  size?: [number, number] | null;
  props: Record<string, unknown>;
}

export interface PageDoc {
  id: string;
  name: string;
  activation: ActivationSpec | null;
  pos: [number, number];
  padding: number;
  /** 画布语义 (null = 自由画布; "minihud" = ctx 派生画布) */
  canvas?: string | null;
  /** 窗口尺寸语义 ("auto" | {fixed:{w,h}}) */
  sizing?: unknown;
  /** 固定停靠 (null = 自由定位) */
  dock?: unknown;
  /** 数据面形态 ("page" | "sidecar") */
  dataface?: string;
  /** live 起步隐藏 */
  startHidden?: boolean;
  font: { sizeAdd: number };
  /** 页级兴趣键声明 */
  interestKeys?: string[];
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
  /** 窗口尺寸 (内容包围盒 + 2×padding — 派生物) */
  pageW: number;
  pageH: number;
  /** 窗口视图 ← 画布视图的平移 (PNG 按窗口系渲染, 画布以 −offset 反变换锚定) */
  offsetX: number;
  offsetY: number;
  /** 内容包围盒 (画布系) */
  contentX: number;
  contentY: number;
  contentW: number;
  contentH: number;
  padding: number;
  /** 逻辑画布 (4096 自由 / minihud ctx 派生) */
  canvasW: number;
  canvasH: number;
  /** 组件矩形 (画布系, 页文档序 = z 序) */
  items: SolveItem[];
  /** 构建错误 (id, 原因): 类型未注册 / 工厂 Err — 静默失败回显 */
  errors: [string, string][];
  /** RGBA PNG (Rust 侧 base64 — 此前 number[] JSON 逐字节传输) */
  png: string;
}
