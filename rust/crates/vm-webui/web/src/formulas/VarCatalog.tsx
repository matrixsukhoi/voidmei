/**
 * 变量目录 (重构: 原 Drawer → 三栏布局中的右栏常驻面板; 统一目录 = 系统变量 + 公式产出):
 * - VarPanel: 常驻右栏 (搜索 + 来源/类别筛选 + 计数 + 紧凑两行条目 + 底部说明),
 *   直接渲染不引入虚拟滚动, 列表容器固定高度内部滚动;
 * - VarDrawer: 窗口过窄 (<900px) 右栏放不下时退化回抽屉模式 (内容复用 CatalogBody);
 * - 点击行为区分 ("公式即变量"): 系统变量 → 插入表达式编辑器光标处 (整行可点);
 *   公式产出变量 → 经 onOpenFormula 上抛, 跳转左栏公式列表选中该公式;
 * - 公式条目: 来源 Tag gold 色"公式", 接管型 (overridesSystem) 追加 ⚡ 标记,
 *   第一行右侧显示最近帧值 (formatVarValue);
 * - 来源筛选选中 "indicators" 时列表上方显示机型差异提示 (Alert)。
 */
import React, { useMemo, useState } from 'react'
import { Alert, Button, Drawer, Input, Select, Tag, Tooltip, Typography } from 'antd'
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

const { Text } = Typography

/** 等宽字体串 (ExprEditor cmTheme 同源) */
const MONO = "Consolas, 'Cascadia Mono', monospace"

/** 接管标记的 gold 色 (antd gold-6, 与公式来源 Tag 同色系) */
const GOLD = '#faad14'

/** indicators 来源提示 (可用字段随机型而异, 见任务背景 #2) */
const INDICATORS_TIP =
  '「8111 /indicators」的可用字段随机型而异——部分机型不回传某些字段, 缺失时该变量为哨兵值或 0。公式中建议配合 is_valid() 判断或用三元降级, 显示行可配 hide-when-zero。'

/**
 * 单条变量 (紧凑两行): 第一行 = 变量名(等宽) + 单位 + [公式变量最近值];
 * 第二行 = 描述 + 来源 Tag + [接管 ⚡]。点击: 系统变量插入光标处 / 公式变量定位公式。
 */
const VarRow: React.FC<{
  v: VarCatalogEntry
  onInsert: (name: string) => void
  /** 公式变量点击回调 (跳转左栏公式列表选中该公式) */
  onOpenFormula: (name: string) => void
}> = ({ v, onInsert, onOpenFormula }) => {
  const formula = isFormulaVar(v)
  return (
    <div
      className="var-row"
      title={formula ? `点击定位公式 ${v.name} (左栏公式列表)` : `点击插入 ${v.name} 到表达式光标处`}
      onClick={() => (formula ? onOpenFormula(v.name) : onInsert(v.name))}
    >
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 6, minWidth: 0 }}>
        <span style={{ fontFamily: MONO, fontSize: 12, color: '#1677ff' }}>{v.name}</span>
        {v.unit && (
          <Text type="secondary" style={{ fontSize: 11, flexShrink: 1 }} ellipsis>
            {v.unit}
          </Text>
        )}
        {/* 公式产出变量: 最近一帧值 (有限 "= 0.447", 无数据 "-") */}
        {formula && (
          <span
            style={{ fontFamily: MONO, fontSize: 11, color: '#8c8c8c', marginLeft: 'auto', flexShrink: 0 }}
          >
            {formatVarValue(v)}
          </span>
        )}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 4, minWidth: 0 }}>
        <Text type="secondary" style={{ fontSize: 11, flex: 1, minWidth: 0 }} ellipsis title={v.desc}>
          {v.desc}
        </Text>
        <Tag
          color={originTagColor(v.originKey)}
          title={`${v.origin} · ${categoryCn(v.category)}`}
          style={{ fontSize: 10, lineHeight: '16px', marginInlineEnd: 0, paddingInline: 4, flexShrink: 0 }}
        >
          {v.origin}
        </Tag>
        {/* 接管型公式: 与系统变量同名, 其值由本公式接管 */}
        {formula && v.overridesSystem === true && (
          <Tooltip title="此公式与系统变量同名, 其值由本公式接管">
            <span style={{ fontSize: 11, lineHeight: '16px', color: GOLD, flexShrink: 0, cursor: 'help' }}>⚡</span>
          </Tooltip>
        )}
      </div>
    </div>
  )
}

/**
 * 目录主体 (面板/抽屉共用): 筛选条 + 计数 + 滚动条目列表 + 底部 FM 说明。
 * 根节点 height:100% — 两种容器都提供确定高度, 列表区 flex:1 内部滚动防溢出。
 */
const CatalogBody: React.FC<{
  vars: VarCatalogEntry[]
  onInsert: (name: string) => void
  /** 公式产出变量点击回调 (上抛到 FormulaTab 选中该公式) */
  onOpenFormula: (name: string) => void
}> = ({ vars, onInsert, onOpenFormula }) => {
  const [kw, setKw] = useState('')
  const [origin, setOrigin] = useState('') // '' = 全部来源
  const [cat, setCat] = useState('') // '' = 全部类别

  // 选项动态收集 (后端加来源/类别不漏配)
  const origins = useMemo(() => originOptions(vars), [vars])
  const cats = useMemo(() => categoryOptions(vars), [vars])
  // 三条件叠加过滤
  const filtered = useMemo(() => filterVarEntries(vars, kw, origin, cat), [vars, kw, origin, cat])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', minHeight: 0, gap: 8 }}>
      <Input
        size="small"
        allowClear
        placeholder="搜索变量名 / 描述"
        value={kw}
        onChange={(e) => setKw(e.target.value)}
      />
      <div style={{ display: 'flex', gap: 6 }}>
        <Select
          size="small"
          style={{ flex: 1, minWidth: 0 }}
          placeholder="全部来源"
          allowClear
          value={origin || undefined}
          options={origins}
          onChange={(v) => setOrigin(v ?? '')}
        />
        <Select
          size="small"
          style={{ flex: 1, minWidth: 0 }}
          placeholder="全部类别"
          allowClear
          value={cat || undefined}
          options={cats}
          onChange={(v) => setCat(v ?? '')}
        />
      </div>
      {/* 来源提示: 仅选中 indicators 时出现 (该来源字段随机型而异) */}
      {origin === 'indicators' && (
        <Alert type="info" showIcon style={{ fontSize: 12, padding: '4px 8px' }} message={INDICATORS_TIP} />
      )}
      <Text type="secondary" style={{ fontSize: 11 }}>
        目录 = 系统变量 + 公式产出 (统一命名空间); 共 {vars.length}, 筛选后 {filtered.length} —
        点击系统变量插入光标处, 点击公式变量定位该公式
      </Text>
      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
        {filtered.map((v) => (
          <VarRow key={v.name} v={v} onInsert={onInsert} onOpenFormula={onOpenFormula} />
        ))}
      </div>
      {/* 常驻说明: FM 变量的加载前置条件 */}
      <Text type="secondary" style={{ fontSize: 11, borderTop: '1px solid #f0f0f0', paddingTop: 6 }}>
        fm.* 变量需 FM 数据加载后才有值, 未加载时为 NaN
      </Text>
    </div>
  )
}

/** 右栏常驻面板 (默认展开态; 右上角按钮收起为窄条) */
export const VarPanel: React.FC<{
  vars: VarCatalogEntry[]
  onInsert: (name: string) => void
  /** 公式产出变量点击回调 (上抛到 FormulaTab 选中该公式) */
  onOpenFormula: (name: string) => void
  onCollapse: () => void
}> = ({ vars, onInsert, onOpenFormula, onCollapse }) => (
  <div
    style={{
      width: 300,
      flexShrink: 0,
      display: 'flex',
      flexDirection: 'column',
      minHeight: 0,
      padding: 8,
      border: '1px solid #e6e6e6',
      borderRadius: 8,
      background: '#ffffff',
    }}
  >
    <div style={{ display: 'flex', alignItems: 'center', marginBottom: 4 }}>
      <Text strong style={{ fontSize: 13, flex: 1 }}>
        变量目录
      </Text>
      <Button type="text" size="small" title="收起变量目录" onClick={onCollapse}>
        »
      </Button>
    </div>
    <CatalogBody vars={vars} onInsert={onInsert} onOpenFormula={onOpenFormula} />
  </div>
)

/** 窄窗 (<900px) 备胎: 原抽屉形态 (内容与常驻面板同源) */
export const VarDrawer: React.FC<{
  open: boolean
  onClose: () => void
  vars: VarCatalogEntry[]
  onInsert: (name: string) => void
  /** 公式产出变量点击回调 (上抛到 FormulaTab 选中该公式) */
  onOpenFormula: (name: string) => void
}> = ({ open, onClose, vars, onInsert, onOpenFormula }) => (
  <Drawer
    title="变量目录"
    placement="right"
    width={Math.min(400, Math.max(280, window.innerWidth - 60))}
    open={open}
    onClose={onClose}
    styles={{ body: { paddingTop: 8 } }}
  >
    {/* Drawer 内容区高度 = 全屏高 - 标题头 55px, 列表在其内部滚动 */}
    <div style={{ height: 'calc(100vh - 55px - 16px)' }}>
      <CatalogBody vars={vars} onInsert={onInsert} onOpenFormula={onOpenFormula} />
    </div>
  </Drawer>
)
