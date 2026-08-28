/**
 * 公式表达式编辑器 (CodeMirror 6):
 * - 自定义简单语言高亮 (lexer.rs 同源: 数字/标识符/运算符 + - * / % ^ 比较 ? : ( ) , //注释);
 * - 变量 (get_var_catalog 统一目录: 系统变量 + 公式产出) + 内置函数 (resolve_fn 全量 33 个) 自动补全;
 * - 输入防抖 400ms → formula_validate, 结果显示在编辑器下方;
 * - "试算"按钮 → formula_try_eval (对最近一帧数据求值)。
 * React 集成: useEffect 创建 EditorView + RefObject 挂载, 卸载 destroy (标准做法)。
 */
import React, { useCallback, useEffect, useRef, useState } from 'react'
import { Button, Space, Typography } from 'antd'
import {
  EditorView,
  drawSelection,
  highlightSpecialChars,
  keymap,
  lineNumbers,
} from '@codemirror/view'
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { StreamLanguage, type StringStream } from '@codemirror/language'
import { autocompletion, type Completion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete'
import type { VarCatalogEntry } from '../api'
import { formulaTryEval, formulaValidate } from '../api'
import { categoryCn, isFormulaVar } from './varFilter'

const { Text } = Typography

/**
 * 内置函数目录 (functions.rs resolve_fn 全量, 签名/中文说明按 arity()/eval_stateful 对齐)。
 * apply 补出 "(" — 补全后直接输入实参。
 */
const FN_CATALOG: { label: string; sig: string; desc: string }[] = [
  // 数学族
  { label: 'abs', sig: 'abs(x)', desc: '绝对值' },
  { label: 'min', sig: 'min(a, b, …)', desc: '最小值 (≥2 参数, 变长)' },
  { label: 'max', sig: 'max(a, b, …)', desc: '最大值 (≥2 参数, 变长)' },
  { label: 'sqrt', sig: 'sqrt(x)', desc: '平方根' },
  { label: 'sin', sig: 'sin(x)', desc: '正弦 (弧度)' },
  { label: 'cos', sig: 'cos(x)', desc: '余弦 (弧度)' },
  { label: 'atan2', sig: 'atan2(y, x)', desc: '反正切 (象限正确)' },
  { label: 'exp', sig: 'exp(x)', desc: '自然指数 e^x' },
  { label: 'ln', sig: 'ln(x)', desc: '自然对数' },
  { label: 'floor', sig: 'floor(x)', desc: '向下取整' },
  { label: 'ceil', sig: 'ceil(x)', desc: '向上取整' },
  { label: 'round', sig: 'round(x, n)', desc: 'n 位小数四舍五入' },
  { label: 'clamp', sig: 'clamp(x, lo, hi)', desc: '夹取到 [lo, hi]' },
  // 哨兵族
  { label: 'is_valid', sig: 'is_valid(x)', desc: 'x 有效 (非 NaN 且非哨兵) → 1/0' },
  { label: 'na', sig: 'na()', desc: '无效哨兵值 (-65535)' },
  { label: 'is_nan', sig: 'is_nan(x)', desc: 'x 为 NaN → 1/0' },
  // 插值族
  { label: 'lerp', sig: 'lerp(x, x0, y0, x1, y1)', desc: '两点线性插值' },
  { label: 'interp1d', sig: 'interp1d(x, xs, ys)', desc: '一维查表插值 (表来自 FM 数据)' },
  { label: 'interp1d_ex', sig: 'interp1d_ex(x, xs, ys, extrap)', desc: '一维插值, extrap≠0 外推' },
  { label: 'interp2d', sig: 'interp2d(x, y, xs, ys, zz)', desc: '二维查表插值' },
  // 大气族 (ISA)
  { label: 'isa_pressure', sig: 'isa_pressure(alt)', desc: 'ISA 大气压 (Pa)' },
  { label: 'isa_density', sig: 'isa_density(alt)', desc: 'ISA 大气密度 (kg/m³)' },
  { label: 'isa_temp', sig: 'isa_temp(alt)', desc: 'ISA 温度 (°C)' },
  { label: 'ias_to_tas', sig: 'ias_to_tas(ias, rho)', desc: '表速 → 真空速 (km/h)' },
  { label: 'tas_to_ias', sig: 'tas_to_ias(tas, rho)', desc: '真空速 → 表速 (km/h)' },
  { label: 'ias_per_mach', sig: 'ias_per_mach(alt)', desc: 'Ma=1 对应表速 (km/h, 按高度)' },
  // 状态原语 (帧间记忆, 编辑期试算从零起步)
  { label: 'sma', sig: 'sma(x, n)', desc: 'n 帧滑动平均 (状态)' },
  { label: 'prev', sig: 'prev(x)', desc: '上一帧值 (状态)' },
  { label: 'blend', sig: 'blend(x, ratio)', desc: '一阶滞后平滑 (状态)' },
  { label: 'deriv', sig: 'deriv(x)', desc: '每秒变化率 dx/dt (状态)' },
  { label: 'vote', sig: 'vote(up, down, n)', desc: '计数表决, ±n 冻结输出 ±1 (状态)' },
  { label: 'stable', sig: 'stable(x, ms)', desc: 'x 持续不变达 ms 毫秒 → 1 (状态)' },
  { label: 'learn_max', sig: 'learn_max(x, gate, timeout_ms)', desc: 'gate 有效期学习最大值, 超时锁定 (状态)' },
]

/**
 * 流式分词高亮 (lexer.rs Tok 同源映射为 CM 标准 style:
 * comment/number/keyword(函数调用)/variableName/operator/punctuation)。
 * 标识符规则 [A-Za-z_][A-Za-z0-9_.]* — 含 '.' 以支持 fm.vne 点路径名。
 */
const formulaLanguage = StreamLanguage.define({
  token(stream: StringStream): string | null {
    // 注释: // 到行尾
    if (stream.match('//')) {
      stream.skipToEnd()
      return 'comment'
    }
    // 数字: 123 / 1.5 / .5 / 1e-3 (lexer.rs 同规则)
    if (stream.match(/(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?/)) return 'number'
    // 标识符: 后跟 "(" = 函数调用 (keyword 色), 否则变量
    if (stream.match(/[A-Za-z_][A-Za-z0-9_.]*/)) {
      return stream.match(/\s*\(/, false) ? 'keyword' : 'variableName'
    }
    // 双字符运算符先于单字符 (最长匹配)
    if (stream.match(/==|!=|<=|>=|&&|\|\|/)) return 'operator'
    if (stream.match(/[+\-*/%^<>!?:]/)) return 'operator'
    if (stream.match(/[(),]/)) return 'punctuation'
    stream.next()
    return null
  },
})

/** 编辑器观感 (粉白主题对位: 白底/浅槽/等宽字体, 固定高度内滚动) */
const cmTheme = EditorView.theme({
  '&': { height: '170px', fontSize: '13px', backgroundColor: '#ffffff', border: '1px solid #d9d9d9', borderRadius: 6 },
  '&.cm-focused': { outline: 'none', borderColor: '#FF69B4' },
  '.cm-scroller': { overflow: 'auto', fontFamily: "Consolas, 'Cascadia Mono', monospace" },
  '.cm-gutters': { backgroundColor: '#fafafa', color: '#999999', borderRight: '1px solid #f0f0f0' },
})

/** 校验状态机 (防抖窗口/IPC 途中的中间态) */
type ValState =
  | { state: 'idle' } // 空表达式或未开始
  | { state: 'checking' } // 防抖等待或 IPC 途中
  | { state: 'ok' }
  | { state: 'err'; msg: string }

interface ExprEditorProps {
  value: string
  onChange: (expr: string) => void
  /** 变量补全数据源 (统一目录: 系统变量 + 公式产出; 父组件加载共享给 VarCatalog) */
  varCatalog: VarCatalogEntry[]
  /** 公式名 → 表达式映射 (公式产出变量补全 detail 用, 与目录数据同源不额外请求) */
  formulaExprs?: Record<string, string>
  /** 光标插入函数注册器 (变量目录点击插入用; 卸载时以 null 注销) */
  registerInsert?: (fn: ((text: string) => void) | null) => void
}

/** 补全 detail 单行截断 (24 字符外省略) */
const truncDetail = (s: string): string => (s.length > 24 ? `${s.slice(0, 24)}…` : s)

export const ExprEditor: React.FC<ExprEditorProps> = ({ value, onChange, varCatalog, formulaExprs, registerInsert }) => {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const viewRef = useRef<EditorView | null>(null)
  // 最新值 ref: view 只建一次, 回调/补全源经 ref 读最新 (extensions 闭包不重建)
  const onChangeRef = useRef(onChange)
  const varsRef = useRef(varCatalog)
  const exprsRef = useRef(formulaExprs)
  useEffect(() => {
    onChangeRef.current = onChange
    varsRef.current = varCatalog
    exprsRef.current = formulaExprs
  }, [onChange, varCatalog, formulaExprs])

  const [val, setVal] = useState<ValState>({ state: 'idle' })
  /** 试算结果 (null = 未试算; value 为 null = NaN/无效 → 显示 "-") */
  const [tryRes, setTryRes] = useState<{ ok: boolean; value: number | null; error: string | null } | null>(null)

  /** 补全源: 变量 (统一目录: 系统变量 + 公式产出) + 内置函数表; 匹配光标前的标识符片段 */
  const completionSource = useCallback((ctx: CompletionContext): CompletionResult | null => {
    const word = ctx.matchBefore(/[\w.]+/)
    if (!word || (word.from === word.to && !ctx.explicit)) return null
    const opts: Completion[] = [
      ...varsRef.current.map<Completion>((v) => {
        // 公式产出变量: detail 标 "公式: <表达式截断>" (表达式取公式列表映射;
        // 无映射时退回目录 desc, 剥后端补的 "公式: " 前缀防重复) — 保证公式 B 能补全引用公式 A
        if (isFormulaVar(v)) {
          const expr = exprsRef.current?.[v.name] ?? v.desc.replace(/^公式: /, '')
          return {
            label: v.name,
            type: 'variable',
            detail: `公式: ${truncDetail(expr || '-')}`,
            info: v.desc,
          }
        }
        return {
          label: v.name,
          type: 'variable',
          detail: v.unit,
          info: `${v.desc} (${categoryCn(v.category)})`,
        }
      }),
      ...FN_CATALOG.map<Completion>((f) => ({
        label: f.label,
        type: 'function',
        detail: f.sig,
        info: f.desc,
        apply: `${f.label}(`, // 补出左括号直接填实参
      })),
    ]
    return { from: word.from, options: opts, validFor: /^[\w.]*$/ }
  }, [])

  // 创建/销毁 EditorView (StrictMode 双挂载安全: destroy 后重建)
  useEffect(() => {
    const host = hostRef.current
    if (!host) return
    const view = new EditorView({
      doc: value,
      parent: host,
      extensions: [
        lineNumbers(),
        history(),
        drawSelection(),
        highlightSpecialChars(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        formulaLanguage,
        autocompletion({ override: [completionSource] }),
        cmTheme,
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onChangeRef.current(u.state.doc.toString())
        }),
      ],
    })
    viewRef.current = view
    return () => {
      view.destroy()
      viewRef.current = null
    }
  }, [completionSource])

  // 外部 value 同步 (切换选中公式时全量替换; 相同内容跳过防回环)
  useEffect(() => {
    const view = viewRef.current
    if (view && value !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } })
    }
  }, [value])

  /** 光标处插入文本 (变量目录点击回调), 插入后聚焦编辑器 */
  const insertAtCursor = useCallback((text: string) => {
    const view = viewRef.current
    if (!view) return
    const pos = view.state.selection.main.head
    view.dispatch({
      changes: { from: pos, to: pos, insert: text },
      selection: { anchor: pos + text.length },
      scrollIntoView: true,
    })
    view.focus()
  }, [])
  useEffect(() => {
    registerInsert?.(insertAtCursor)
    return () => registerInsert?.(null)
  }, [registerInsert, insertAtCursor])

  // 防抖 400ms 校验 (空表达式静默 idle; 切公式自动重验)
  useEffect(() => {
    setTryRes(null) // 表达式已变, 旧试算结果作废
    if (!value.trim()) {
      setVal({ state: 'idle' })
      return
    }
    setVal({ state: 'checking' })
    const t = setTimeout(() => {
      formulaValidate(value)
        .then((r) => setVal(r.ok ? { state: 'ok' } : { state: 'err', msg: r.error ?? '未知错误' }))
        .catch((e) => setVal({ state: 'err', msg: String(e) }))
    }, 400)
    return () => clearTimeout(t)
  }, [value])

  /** 试算: 对最近一帧遥测数据求值 (状态原语从零起步, 编辑期近似) */
  const doTry = () => {
    formulaTryEval(value)
      .then(setTryRes)
      .catch((e) => setTryRes({ ok: false, value: null, error: String(e) }))
  }

  /** 试算值显示: null/NaN/±Inf → "-" (serde 对非有限 f64 序列化为 null) */
  const tryText =
    tryRes == null
      ? null
      : tryRes.ok
        ? tryRes.value != null && Number.isFinite(tryRes.value)
          ? tryRes.value.toFixed(3)
          : '-'
        : null

  return (
    <div>
      <div ref={hostRef} />
      <div style={{ marginTop: 4, display: 'flex', alignItems: 'flex-start', gap: 8, minHeight: 22 }}>
        {/* 校验结果行 (编辑器正下方): 绿 ✓ / 红 error */}
        <div style={{ flex: 1, minWidth: 0 }}>
          {val.state === 'idle' && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              输入表达式后自动校验; 可用变量与函数可在"变量目录"中检索点击插入
            </Text>
          )}
          {val.state === 'checking' && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              校验中…
            </Text>
          )}
          {val.state === 'ok' && (
            <Text style={{ fontSize: 12, color: '#52c41a' }}>
              ✓ 语法正确
            </Text>
          )}
          {val.state === 'err' && (
            <Text type="danger" style={{ fontSize: 12 }}>
              {val.msg}
            </Text>
          )}
        </div>
        <Space size={8}>
          <Button size="small" onClick={doTry} disabled={!value.trim()}>
            试算
          </Button>
          {tryRes && (
            <Text style={{ fontSize: 12 }}>
              {tryRes.ok ? (
                <>
                  结果: <Text strong>{tryText}</Text>
                </>
              ) : (
                <Text type="danger">{tryRes.error}</Text>
              )}
            </Text>
          )}
        </Space>
      </div>
    </div>
  )
}
