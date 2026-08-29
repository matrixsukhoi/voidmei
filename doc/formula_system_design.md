# VoidMei 公式系统设计文档

> 状态: 设计定稿(2026-08-28);**阶段 0-3 + L2 规则引擎已实施**(2026-08-29,实施记录见 §14)。
> 决策记录: `build/migration/DECISIONS.md` D10。
> 本文所有文件路径行号以 `rust/` workspace 当前 rust 分支为准。

## 1. 背景与目标

### 1.1 为什么做

VoidMei 的派生指标计算(单位动能、失速速度、马赫数、增压器功率、告警判定……)目前全部**硬编码**在 Rust 里(vm-core 的 `hud_calculator.rs`、vm-data 的 `Deriver`/`service_loop.rs`/`methods_engine.rs`),想看"这个数怎么算的"只能读源码,想改算法必须改代码重新编译。

D9(MainForm 切 Tauri2+React)时确立的演进目标是**公式管理编辑器**:把这些计算外置成运行时可查看、可编辑的"公式",MainForm 里统一管理内置公式与用户自定义公式。

Java 版的 `FormulaEvaluator.java`(Nashorn JS 引擎 + 编译缓存)与 `Blkx.getVariableMap()`(反射变量导出)是从未接线的孤儿组件——本系统是全新设计,不是移植。D4/D7 已裁决:不做反射,变量是编译期显式注册表。

### 1.2 目标

1. **内核更小更紧凑**:内核只剩采数(8111 轮询/解析)、FM(.blkx)加载解析、overlay 窗口与渲染、配置持久化。数学计算全部上移到公式系统(分级外置,见 §7)。
2. **公式统一承载**:内置派生指标 = 出厂预置公式;用户自定义公式 = 同一引擎同一格式。MainForm 编辑器里能看、能改、能新建、能试算。
3. **变量驱动动作**:变量超阈值触发语音/toast/警告标志,用规则配置表达,不再硬编码判定。
4. **数值行为不变**:每个内置计算外置时必须对拍(公式定义自带 `:test` 期望值),存量测试零回归。

### 1.3 非目标

- 不支持字符串计算(现有计算全数值;`map_grid`/`time_str` 等格式化留渲染层)。
- 不做脚本语言(无赋值/循环/对象/闭包)——不是嵌入式 rhai,是"带函数库与状态原语的表达式语言"。
- 不改变 overlay 渲染架构(win32 原生窗口、tiny-skia 绘制不变)。

## 2. 三层架构与数据流

```
┌─────────────────────────────────────────────────────────────────────┐
│ 前端 MainForm (React/AntD, 仅编辑期参与)                                │
│   公式 tab: 列表 | CodeMirror 编辑器 | 变量目录 | 试算面板 | 规则编辑      │
└──────────────┬──────────────────────────────────────────────────────┘
               │ dispatcher 类 command (CRUD/保存)   直算类 command (校验/试算)
┌──────────────▼──────────────────────────────────────────────────────┐
│ L2 规则层  rules: 变量/阈值/持续/冷却 → 动作(voice/toast/flag)          │
├─────────────────────────────────────────────────────────────────────┤
│ L1 公式层   FormulaDef(内置+用户) → 编译(拓扑+环检测) → 每帧按拓扑求值    │
│            状态原语 StateStore (sma/prev/blend/vote/hold/learn_max)    │
├─────────────────────────────────────────────────────────────────────┤
│ L0 原子变量注册表  编译期显式 match (D4/D7 裁决: 不做反射)               │
│   /state 直通 ~40 + /indicators 直通 ~55 + TelemetrySource 71          │
│   + FM 字段 58 + 元变量(interval_ms/engine_type/fm_loaded/...)          │
│   → 每帧组装 VarSnapshot (VarId → f64 平坦 Vec)                        │
└──────────────▲──────────────────────────────────────────────────────┘
               │ Service 线程 (vm-data service_loop.rs calculate() 尾部
               │                新增 formula_step — 求值唯一发生点)
               │ 结果写回 ServiceData.formula_values
               │ 既有 RwLock<ServiceData> (app_shell.rs feed_overlays_live
               │ L2858-2862 已只读快照) → 零新总线
┌──────────────┴──────────────────────────────────────────────────────┐
│ 内核: 8111 采数+JSON 解析 | FM 加载解析 | overlay 窗口+渲染 | 配置持久化   │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.1 关键架构裁决

| # | 裁决 | 理由 |
|---|---|---|
| A1 | **公式求值收敛 Service 线程单点** | Rust 迁移版 HUDData 计算在 win32 线程(`vm-app/src/app_shell.rs` L2869-2900,与 Java 的 Service 线程预计算**不同**)。若两线程各自求值,`sma/prev` 状态原语会双份漂移。阶段 4 把 HUDData 计算一并迁回 Service 线程,恢复 Java 语义(预计算降 EDT——此处是 win32——延迟) |
| A2 | 结果经既有 `Arc<RwLock<ServiceData>>` 传递,零新总线 | win32 线程 `feed_overlays_live` 已只读快照 ServiceData |
| A3 | 引擎放 **vm-core** | vm-webui 不依赖 vm-data(`commands_windows.rs` 头注),直算类试算/校验命令只能触达 vm-core |
| A4 | **自研解释器,不引 rhai** | workspace 零新依赖惯例(`config_manager.rs` 手写 MD5);语言面小(表达式+函数库+状态原语),不需要 rhai 的循环/对象;与 ui_layout.cfg 的 S-expr 配置体系风格统一 |
| A5 | 每帧**全量**按拓扑序求值,不做增量/失效传播 | 10-20Hz × 几十公式 × AST 解释 <100µs/帧,增量复杂度不值得 |
| A6 | `:target` 值域 = 变量名 ∪ 公式名(getter 名作别名向后兼容) | 现有 35 个 `:type data` 行(`ui_layout.cfg` L114-169)零改动 |

## 3. 公式语言 Spec

### 3.1 语法(EBNF)

```
expr        := ternary
ternary     := or [ "?" expr ":" ternary ]
or          := and { ("||" | "or") and }
and         := cmp { ("&&" | "and") cmp }
cmp         := add [ ("==" | "!=" | "<" | "<=" | ">" | ">=") add ]
add         := mul { ("+" | "-") mul }
mul         := unary { ("*" | "/" | "%") unary }
unary       := ("-" | "!" | "not") unary | pow
pow         := primary [ "^" unary ]            // 右结合
primary     := NUMBER | IDENT | call | "(" expr ")"
call        := IDENT "(" [ expr { "," expr } ] ")"
IDENT       := [A-Za-z_][A-Za-z0-9_.]*          // 允许 fm.max_rpm 点路径
NUMBER      := [0-9]+("." [0-9]+)? ([eE] [+-]? [0-9]+)?
```

注释 `// 到行尾`。无赋值、无循环、无对象、无字符串字面量。

### 3.2 类型系统

| 类型 | 语义 |
|---|---|
| `f64` | 主类型 |
| `bool` | 与 f64 互通:bool 参与算术 → 1.0/0.0;f64 进布尔上下文 → 非零为真 |
| `Table` | **不透明**:仅来自命名表变量(§3.4),仅可作插值函数实参,不可参与算术 |

### 3.3 函数库(全部为现有 Rust 纯函数薄封装)

| 族 | 函数 | 封装来源 |
|---|---|---|
| 数学 | `abs/min/max/sqrt/sin/cos/atan2/exp/ln/pow/floor/ceil/round(x,n)/clamp(x,lo,hi)` | std |
| 哨兵 | `is_valid(x)`(非 F_INVALID 且非 NaN)/ `na()`(返回 F_INVALID)/ `is_nan(x)` | 现有 F_INVALID 约定 |
| 插值 | `lerp(x,x0,y0,x1,y1)` / `interp1d(x,xs,ys)` / `interp2d(x,y,xs,ys,zz)` | `vm-core/src/interpolation.rs` L49/L62/L107 |
| 大气 | `isa_pressure(alt)` / `isa_density(alt)` / `isa_temp(alt)` / `ias_to_tas(ias,rho)` / `tas_to_ias(tas,rho)` / `ias_per_mach(alt)`(分母声速组合式,`mach = ias / ias_per_mach(alt)` 等价现 derive.rs L116-120 手写式) | `vm-core/src/atmosphere_model.rs` |
| 活塞 | `stage_power(alt,wep,speed,is_ias)`(当前 FM 最优档)/ `optimal_stage(alt)` | `vm-core/src/piston_power_model.rs` L270/L351 |
| FM 查表 | `fm_vne(sweep)` / `fm_mne(sweep)` / `fm_aoa_high(sweep)` / `fm_flap_allow_angle(flap)` | `vm-core/src/blkx/model.rs` L37-114 + `methods_engine.rs` L264-410 档位插值(双胞胎合一后) |
| 常量 | `g`(9.80)/ `rho0`(1.225)/ `P0`(101325) | `vm-core/src/physics_constants.rs`(禁止字面量硬编码,沿用 CLAUDE.md 规则) |

新函数加入 = `functions.rs` 注册表加一项,前端函数目录自动同步。

### 3.4 命名表(Table)

- **文件内静态表**:`(table "ias_grid" (0 200 400 600))` → 公式里 `interp1d(x, ias_grid, ys)`。
- **FM 动态表**(襟翼档位表/增压器 stages):**不落文件**,注册为 `fm.*` 派生表变量,FM_CHANGED 时从 blkx 提取重建。公式经 Table 类型引用——这满足"功率曲线全部外置"而无需把 stages 结构塞进语言。

### 3.5 状态原语(带隐藏状态的函数)

引擎按 **(公式 id, AST 调用点 id)** 维护状态;同一表达式两处调用 `prev` 各有各的状态。

| 原语 | 语义 | 对齐的现有实现 |
|---|---|---|
| `sma(x, n)` | n 点滑动均值,窗口未满按已有点均值 | `vm-core/src/calc_helper.rs` L5-31 SimpleMovingAverage |
| `prev(x)` | 上一帧值(初始 0) | derive.rs L79 `speedvp = speedv` |
| `blend(x, ratio)` | 一阶惯性 `ratio_1*prev + ratio*x` | service_loop.rs L1190-1226 三处 |
| `deriv(x)` | 每秒变化率(按帧间隔折算) | fuel 消耗率语义 |
| `vote(up, down, n)` | 计数投票,±n 冻结输出 | 磁电机/桨距投票 service_loop.rs L1095-1130 |
| `hold(x, ms)` | 条件持续 ms 才输出真 | 襟翼 1 秒稳定判定 methods_engine.rs L36-81 |
| `learn_max(x, gate, timeout_ms)` | 超时+门控内的最大值学习 | RPM 20 秒自适应 methods_engine.rs L91-131 |

**重置三层语义**:
1. 热更新:新旧公式集差集——消失公式的状态清除,保留公式状态延续;
2. `FM_CHANGED`(换机):全部状态清零(FM 相关上下文失效);
3. 会话重置(游戏重开/Controller stop→start):挂现有 resetvaria 调用点,顺带收口 `service_fields.rs` L48-54 状态双主边界问题。

### 3.6 错误模型

| 阶段 | 错误 | 处理 |
|---|---|---|
| 编译期 | 语法错/未知变量或函数/arity 不符/循环依赖 | 拒绝保存;用户文件里已存在的标 `invalid` 并**隔离**(跳过求值,依赖它的公式得 NaN),不影响其余公式 |
| 运行期 | 除零 → IEEE NaN;FM 未加载引用 `fm.*` → NaN;Table 空表 → NaN | NaN 逐公式隔离传播;当帧错误计数进诊断信息,**不中断**当帧其余公式 |
| 显示期 | 公式结果 NaN | 降级为 "-"(与现有 na_when 语义合流),不引入 panic 路径 |

## 4. 模块落点

引擎全部落在 **`rust/crates/vm-core/src/formula/`**(裁决 A3):

| 文件 | 职责 |
|---|---|
| `mod.rs` | 门面 + `FormulaManager`(`RwLock<Arc<CompiledFormulaSet>>`,热更新原子换 Arc) |
| `lexer.rs` | 中缀分词(错误带行列) |
| `parser.rs` | 递归下降 parse → AST |
| `ast.rs` | AST 节点定义 + 调用点 id 标注 |
| `functions.rs` | 函数库注册表(名→FnId→rust fn)+ TableVal 类型 |
| `eval.rs` | 求值器 + StateStore(状态原语) |
| `registry.rs` | L0 变量注册表:VarId 分配 + VarMeta + 快照组装器 |
| `definition.rs` | FormulaDef / CompiledFormulaSet(拓扑序 + 结果槽 + live 标记) |
| `rules.rs` | L2 规则定义 + 触发引擎(RuleState: hold 计时/冷却) |
| `persistence.rs` | formulas.cfg / formulas.user.cfg 双文件读写与合并(参照 config_manager) |
| `tests/` | 词法/语法/求值/状态原语/对拍(内置公式 :test 用例执行器) |

**接线点**(改动面):

| 位置 | 改动 |
|---|---|
| `vm-data/src/service_loop.rs` `calculate()` L807-967 尾部 | 新增 `formula_step`:组快照→求值→写回→规则求值(求值唯一发生点,裁决 A1) |
| `vm-data/src/service_fields.rs` | ServiceData 新增 `formula_values: FormulaResults`(Vec<f64> + 注册表版本号) |
| `vm-app/src/form_dispatch.rs` | 新 RequestKind:`GetFormulaList/SaveFormula/DeleteFormula/ResetFormulas`(写链单点) |
| `vm-app/src/app_shell.rs` | 订阅 FORMULA_CHANGED → 重建 FormulaManager;阶段 4 HUDData 迁移 |
| `vm-webui/src/commands_windows.rs` | 直算类:`formula_validate` / `formula_try_eval` / `get_var_catalog` / `get_last_var_snapshot` |
| `vm-webui/web/src/` | 新 `formulas/` 前端目录 + App.tsx 手工 append tab |

## 5. 变量注册表(L0)

**统一命名空间模型**(2026-08-29 澄清,用户反馈驱动):公式产出本身就是变量——

```
变量 = 有名字的、每帧有值的量 (统一命名空间)
├─ 系统变量 (130): 8111 /state(30) + /indicators(9, 随机型而异) + 内部计算(30)
│                  + FM 文件(52) + 运行时(6) + 常量(3) — 定义不可编辑, 可引用
└─ 公式变量: 由表达式定义, 每帧求值 — 可被其他公式/:target 引用
名字解析顺序: 系统变量表 → 公式表 (resolve_expr)
同名规则: 公式与系统变量同名 = **接管**该系统变量的值
          (内置 mach 公式即此用法); 接管型公式的表达式内
          引用自身 → 编译错误 SelfOverride (防隐式"上一帧值"语义)
```

编辑器的变量目录 = 统一目录(系统 + 公式产出,公式条目带最近值/接管标记)。

- **VarId**(u16 稠密编号)+ **VarMeta** `{ 主名(短名如 `ias`), getter 别名(如 `getIAS`), 单位, 中文描述, 类别, origin 数据来源 }`。
- **快照组装**:每帧一次,在 Service 写锁临界区内,`Vec<f64>` 按 VarId 平坦排布——求值器按下标取数,零查表。
- 注册表扩展自 `vm-core/src/reflect_binder.rs` 的三段式(字符串→enum accessor→typed get,L98-161/L166-228)——reflect_binder 由死代码转为生产通路基底。

## 6. DAG 编译与求值(L1)

- **编译管线**(保存/启动时):parse → AST → 收集自由变量 → 依赖解析(变量名→VarId 或公式名)→ 拓扑排序 → DFS 染色环检测(报出完整环链,如 `a → b → c → a`)。
- **live 标记**:被 `:target`/规则/其他 live 公式引用的传递闭包;死公式保留定义但跳过求值(编辑器里灰显)。
- **每帧**(Service 线程):组装 VarSnapshot → clone Arc<CompiledFormulaSet> → 按拓扑序全量求值(状态原语 &mut StateStore,Service 线程私有无竞争) → 写回 ServiceData → 规则求值(§9)。
- **热更新**:原子换 Arc,下一帧生效。悬空引用(被删公式)→ NaN → "-" + 保存时警告。

### 6.1 性能模型与字节码演进选项(2026-08-29 实测)

**当前模型已是编译制**(即"IR"形态):保存时 parse→resolve(名字→VarId/FnId 编号)→拓扑排序→`Arc<CompiledFormulaSet>`;live 每帧零字符串/零名字查找(VarId 下标直取快照 Vec,FnId match 跳表分派),热更新=重编译+原子换。

**实测**(debug 构建,tests.rs `bench_eval_frame_50_formulas`):50 条混合公式(算术+大气函数+sma/prev 状态原语)**20.2µs/帧**(含快照组装),占 20Hz 轮询预算 **0.04%**。

**字节码化(平坦指令+栈机)的裁决:现在不做**——
- 收益:解释执行约 2-5x,但 0.04%→0.008% 无意义;
- 成本:指令集定义+编译 pass+栈机+测试(数百行);
- 持久化字节码是双源风险:文本是唯一真相源,编译 µs 级连缓存文件都不需要(Python .pyc 存在的理由是编译慢,我们不适用)。

**升级门**:RExpr 已是编号化 IR,flatten 成后缀字节码是机械转换(~200 行)。触发条件:公式数量达数百条 / overlay 渲染层需要逐帧公式值(60fps)/ 实测占用超帧预算 5%。

**已采纳的编译期小优化**(随 W1 做):AST 常量折叠;快照取值 Option→直接索引。
- **性能预算**:AST 解释求值 ~1-3µs/公式,几十 live 公式 + 快照组装 <100µs/帧,对比现有 50ms 轮询周期可忽略;编译结果缓存(不逐帧 parse)。

## 7. 外置分级清单(已确认:分级外置)

**判定标准**:纯函数或状态原语可组合 + 单数值输出 + 输入全为注册表变量 + 可读性不劣于原 Rust 代码。四条全满足才外置;否则视情节降 C 级。

### A 级:全外置(~23 处)

纯算术 ~15 处:

| 计算 | 位置 |
|---|---|
| `energy_m = energy_j_kg / g` | hud_calculator.rs L165 |
| `maneuver_index = 1 - nfweight/(nfweight+fuel)` | hud_calculator.rs L186-190 |
| mach 大气式(外置为 `ias / ias_per_mach(alt)`) | derive.rs L116-120 |
| `stall_speed = 3.6*sqrt(2Wg/(1.225*S))` | service_loop.rs L1368 |
| `ny = an/g`、`An` 离心式 | derive.rs L91-100/L140、service_fields.rs L602-604 |
| sep/turn_radius/其余纯算术 ~9 处 | derive.rs / service_loop.rs 散布 |

插值查表 ~8 处:

| 计算 | 位置 |
|---|---|
| 可变翼 VNE/MNE/AoA sweep 插值 | blkx/model.rs L37-114 |
| flap_allow_angle 档位表+lerp(**双胞胎合一**后外置) | hud_calculator.rs L393-448 与 methods_engine.rs L264-410 |
| update_speed_ratio 5 个 ratio | service_loop.rs L1273-1320 |
| 增压器最优档/功率(stage_power 封装) | piston_power_model.rs L270/L351 |

### B 级:状态原语外置(~14 处,阶段 4)

4×SimpleMovingAverage(derive.rs L83-111)、一阶惯性 blend 三处(service_loop.rs L1190-1226)、RPM 20 秒学习(methods_engine.rs L91-131)、磁电机投票(service_loop.rs L1095-1130)、襟翼 1 秒稳定(methods_engine.rs L36-81)、Deriver 四公式族(speed/climb/turn/sep 的 step 链)。

**验收硬门槛**:FlightValues 逐帧回放位级对拍(录制真机帧序列,旧 Rust 路径与公式路径输出按位相等)。

### C 级:保留 Rust,登记为内置变量(3 处)

英制检测三角耦合(service_loop.rs L902-936)、compass 回退链、加油检测——多输出强耦合状态机,外置需语言支持多输出,可读性反而劣化。**保留 Rust 实现,其输出注册为 L0 变量**供公式引用。这是对"全部外置"的修正:目标是"全部**可**外置",C 级明示保留并在变量目录标注来源。

### 留在渲染域(不外置)

颜色判定/格式化字符串(HUDData 的 *_color/*_str 字段)——但输入与阈值改从公式/L0 变量取,判定逻辑留渲染层。

## 8. overlay 绑定迁移

`:target` 值域扩展:`getter 名 | 短名 | 公式名 | 以上 + " * N" 乘数`(兼容语法在解析层消化),产物 `ResolvedTarget { var: VarId, multiplier: f64 }`——绑定期一次解析(reflect_binder 精神),运行期零字符串。

三静态表迁移路径(先加 fallback 通路,对拍通过后删静态臂):

| 表 | 现状 | 迁移 |
|---|---|---|
| `vm-core/src/fields.rs`(FlightInfo 16 行) | 静态表,但 ui_layout.cfg L114-129 数据开关组已含全部行定义(**cfg 才是定义源**) | 末臂 fallback 统一变量表 → 终态行定义完全由 cfg 驱动(恢复 Java FieldOverlay 语义) |
| `vm-overlay/src/overlays_field1.rs` L922-1046(PowerInfo 19 行) | PowerSource enum 静态 | PowerSource 增 `Formula(VarId)` 变体,`:target` 未命中静态臂走它 |
| `vm-overlay/src/flight_info.rs` flight_value L44-64 | 16 臂 match | 同 fields.rs 路径 |
| MiniHUD(HUDData) | win32 线程调 hud_calculator | 阶段 4 随计算位置迁移(裁决 A1)一并改 |

**现有 35 个 data 行零改动零行为差**;每表迁移配同帧双路径等值对拍测试。

## 9. L2 规则系统(变量驱动动作)

```lisp
(rule "低空告警" :var "radio_altitude" :when "<= 500"
      :hold-ms 300 :cooldown-s 5
      :actions ((voice "warnAltitude") (toast "低空!")))
```

- `:when` 是公式语言表达式(可引用任意变量/公式,如 `mach > 0.85 && fm_loaded`);
- 触发引擎在 Service 公式步后同快照求值;RuleState 持 hold 计时/冷却;
- 动作三类:`voice`(语音资源 key)/ `toast`/ `flag`(置位具名标志,overlay 变色可引用)。

**与现有告警的关系**:
- **VoiceWarning**:音频资源与播放保留;其内部硬编码触发判定(~20 条)阶段 5 外置为出厂规则文件,VoiceWarning 改订阅规则触发事件;
- **HUD 内置告警色**(warn_vne/warn_stall/aoa 变色, hud_calculator.rs L172-259):留渲染域;flag 动作与之**并存**(overlay 可选择引用 flag 覆盖显示,默认不覆盖)。

## 10. MainForm 编辑器

- **tab 落点**:App.tsx L235-260 tabs 数组后 concat 手工 append(现 tab 完全由 cfg panels 驱动,编辑器不是配置行,不走 cfg);不做辅助 WebviewWindow。
- **组件**(`vm-webui/web/src/formulas/`):
  - `FormulaTab`——左列表(内置/自定义分组,内置只读或"另存为副本",自定义可删)右编辑;
  - `FormulaEditor`——**CodeMirror 6**(语法高亮/错误行标注/变量与函数 AutoComplete/依赖链显示);依赖新增到 `web/package.json`;
  - `VarCatalog`——变量目录(名字/单位/中文描述/类别,来自 VarMeta);只读+搜索;
  - `TryPanel`——试算面板:对最近缓存帧序列求值,显示逐帧结果曲线/末值,可验证状态原语行为;
  - `RulesPanel`——规则列表与编辑(阶段 5)。
- **命令**:
  - 直算类(commands_windows.rs 模式,纯 vm-core 面):`formula_validate`(语法+符号+环)/ `formula_try_eval`(对快照序列试算)/ `get_var_catalog`/ `get_last_var_snapshot`;
  - dispatcher 类(form_dispatch.rs 模式,写链):`GetFormulaList/SaveFormula/DeleteFormula/ResetFormulas` → 持久化 → 广播 FORMULA_CHANGED → app_shell 重建 FormulaManager;
  - 试算数据源:vm-app 节流(500ms)发布最近 VarSnapshot DTO(沿用 FLIGHT_RECORD_SNAPSHOT 先例);**环形缓冲最近 200 帧**供状态原语试算。
- **DTO**:沿用 `#[serde(tag="kind")]` enum + camelCase;前端 discriminated union(api.ts 模式)。
- **编辑器防呆**:保存即编译,错误原位标注(行列来自 lexer/parser);循环依赖画出环链;删除被引用公式列出引用方。

## 11. 持久化格式与合并规则

- **文件**:`formulas.cfg`(内置出厂,只读,随程序分发)+ `formulas.user.cfg`(用户覆盖/新增/禁用),与 ui_layout.cfg 同目录(项目根工作区)。
- **格式**:S-expr 外壳(复用 `vm-core/src/sexp_parser.rs`)+ 中缀公式体:

```lisp
(formulas
  (formula "mach" :expr "ias / ias_per_mach(altitude)" :unit "Ma"
           :precision 2 :desc "马赫数(ISA 大气模型)"
           :test ((ias 400 altitude 5000) => 0.663))
  (formula "energy_per_mass" :expr "pow(tas/3.6,2)/(2*g)" :unit "J/kg" :precision 0)
  (table "ias_grid" (0 200 400 600))
  (rule "低空告警" :var "radio_altitude" :when "<= 500" :hold-ms 300 ...))
```

- **`:test` 期望值随定义走**:内置公式外置时从旧 Rust 代码生成对拍用例,`cargo test` 执行公式测试器跑全部 :test——公式定义文件成为 single source of truth,这是"不假通过"的机制保证。
- **合并**(参照 config_manager.rs L146-191):同名用户条目可 `:disabled true`/覆盖 `:expr`/改 unit/precision,其余字段以模板为准;模板 hash(MD5 复用)变化 → merge + .bak + MergeReport 日志。用户**永远可以恢复默认**(ResetFormulas)。

## 12. 实施路线(六阶段,后续批次执行)

| 阶段 | 粗估 | 内容 | 验收(硬门槛) |
|---|---|---|---|
| 0 地基 | 3-4d | formula/ 引擎+注册表+单测,不接线 | test 全绿;150+ 变量元数据齐全 |
| 1 用户价值 | 4-5d | 持久化 + formula_step + :target 通路 + 编辑器 tab(CodeMirror) | 用户公式可在面板引用显示;同帧双路径等值对拍 |
| 2 纯算术 | 3-4d | A 级纯算术 ~15 处外置 | 全部 :test 对拍通过;存量测试零回归 |
| 3 函数库+插值 | 4-5d | 命名表/FM 动态表/flap 双胞胎合一 + 插值 ~8 处 | 双胞胎合一后同输入等值;FM_CHANGED 重建表正确 |
| 4 状态原语+Deriver | 5-7d | 七原语 + B 级 ~14 处 + HUDData 迁回 Service 线程(裁决 A1) | FlightValues 逐帧回放**位级**对拍;像素对拍不回归 |
| 5 告警统一 | 3-4d | rule 引擎 + VoiceWarning ~20 条判定外置为出厂规则 | 冷却/hold 时序真机验证不变;规则禁用后静默 |

- 每阶段独立可停:阶段 1 后即有用户价值(自定义公式),阶段 2-4 逐级瘦身内核,阶段 5 收告警。
- 阶段 4 是最高风险项(状态位级等价 + HUDData 线程迁移回归面),如对拍失败可回退到"Deriver 保留、仅新增量走公式"(开放问题 O4 的降级路径)。

## 13. 风险与开放问题

### 已定默认(不再开放)

| # | 问题 | 决议 |
|---|---|---|
| D1 | 字符串支持 | 不进系统(格式化留渲染层) |
| D2 | :visible-when/:na-when 是否统一到公式语言 | 不统一(现有 S-expr 求值器独立且稳定) |
| D3 | 公式 :precision 与行 :precision 冲突 | 行赢(显示属性归显示行) |
| D4 | flag 动作与 HUD 内置告警色 | 并存,默认不覆盖 |
| D5 | 试算回放深度 | 环形缓冲最近 200 帧 |

### 遗留开放(实施时再拍板)

| # | 问题 | 触发时机 |
|---|---|---|
| O1 | F_INVALID(-65535)哨兵与 NaN 的边界:现有代码大量哨兵判定,外置公式统一 NaN 后,`is_valid()` 判定语义需逐处核对 | 阶段 2 |
| O2 | `get_manifold_pressure_display_unit` 等 String getter 与公式系统的关系(显示类 getter 留在 TelemetrySource) | 阶段 1 |
| O3 | fm_data_adapter.rs 的 BlkxPlaceholder 占位(L139-151)在 FM 动态表提取前需先落地真 blkx | 阶段 3 |
| O4 | Deriver 整族外置若位级对拍失败的降级路径 | 阶段 4 |
| O5 | 内置公式用户覆盖后的升级语义(新版本内置公式变了,用户覆盖是否提示过期) | 阶段 2 |

### 风险登记

| 风险 | 缓解 |
|---|---|
| 状态原语位级等价难达成(浮点求值顺序差异) | 对拍以位级为目标,允许显式登记的容差;失败即走 O4 降级 |
| Service 写锁临界区内快照组装的锁持有时间 | 组装是纯读+Vec 写,预算 <20µs;超预算把组装挪到锁外 clone 快照 |
| 公式错误导致面板刷 "-" 的用户困惑 | 编辑器实时校验 + 保存警告 + 诊断面板(当帧错误计数) |
| CodeMirror 引入的 pnpm 构建许可 | pnpm-workspace.yaml allowBuilds 增项(D9 坑4 先例) |
| 阶段 4 HUDData 线程迁移的回归面 | mock e2e 三场景 + 像素对拍全量跑;迁移单独成波次 |

## 14. 实施记录(2026-08-29)

**W6-W8 数据直通重构**(同日, "telemetry 代码应可删掉" 用户洞察驱动):

| 波次 | 内容 |
|---|---|
| **W6 registry 直通化** | VarSrc 三元直绑(State/Indic/Blkx — fm.* 直绑 blkx 字段); SessionInputs C 级暂存通道; FormulaView 统一取值; **删 FMDataSource/FMDataAdapter/BlkxPlaceholder 三层**; TelemetrySource 全 getter default 化; vario/ny/total_weight 转同名公式 |
| **W7 TelemetrySource 消解** | ServiceData 71 getter 实现 → 5 个(String/精度类); overlay 消费面(hud_calculator/flight_info/PowerInfo/engine_control/gear_flaps/attitude/control_surfaces/minihud) 全走 var_value 桥; visibility_expression/VisExpr 同; ServiceData FormulaView 实时直达源头(State/Indic/Blkx/Session 现取, 不经快照) |
| **W8 check_flap 公式化** | `is_downing_flap = latch(变化方向) * (1 - stable(flaps,1000))` 接管(白名单 bool 写回); flap_allow_speed/angle 公式(fm_flap_allow_* 函数); **删 check_flap 方法 + flap/flapp/flap_check 字段** |

净效果: 21 files, **+867/-2559**(净 -1692 行); vm-data 6438→5613; registry 直绑闭包对齐原 getter 哨兵语义; formulas.cfg 123 行 30+ 公式。

**W9 live 显示回归修复**(2026-08-29, 真机发现"FlightInfo 面板少了很多信息"):

根因 = W6-W8 把公式产出名(registry 短名)与 overlay 静态表 :target(Java getter 名)的**映射通道删了**:overlay 以 getter 名发问(getMach/getVario/...), 公式槽键只有公式名(mach/vario/...), registry 也无这些 getter 索引 → `var_value` 返回 None → FlightInfo 7 行消失(马赫/爬升率/SEP/加速度/过载/转弯率/转半径)、动力信息 3 行恒 0(getTotalWeight/getBoosterFuelKg/getBoosterFuelPercent)、EngineControl 油量表恒 0(fuel_percent)与 WEP/助推器行恒隐(has_wep/has_booster — W7 重构引用了从未注册的名字)。测试桩(MachView 手工 match "getMach")把断链掩成假绿。

修复(1414 测试全绿, e2e 三场景 PASS):
1. **公式 `:getter` 别名 + slots 双键**(FormulaDef.getter;编译期公式名+别名同槽双键;serialize/merge/DTO/前端全链往返保留) — formulas.cfg 8 个公式补别名(mach/vario/sep/acceleration/ny/turn_rate/turn_rds/total_weight)
2. **registry 补 5 量**: booster_fuel_kg/booster_fuel_percent(守卫 NaN 穿透原样)/has_booster(State)/has_wep(Blk 直绑 nitro>0)/fuel_percent(SessionInputs 通道)
3. **build_texts None→0.0**(行不消失; 对位 Java 反射 getter 永不失败, 行只受 visible-when 控制)
4. **守卫测试**(反向验证非假绿): flight_info/overlays_field1 全部消费 target 经双通道可达(canonical_var_name 测试 helper = 生产双通道对位); panel_targets_via_getter_names 端到端真值断言

**W10 根除 getter 双名制**(同日, 用户洞察"getter 名也是和 telemetry 同构的坏味道"):

W9 的 :getter 别名双键本质是**名字搬运层**——与 W6-W8 删除的值搬运层(telemetry getter)同构。W10 把兼容层整个拆除, 单名制落地:

| 动作 | 内容 |
|---|---|
| 静态表短名化 | fields.rs 加 `target()`(短名, 内核取数键); PowerSource 19 臂全部改短名; DataField key/configKey 随之(write-only 字段, 零风险) |
| 删别名机制 | FormulaDef.getter 字段/persistence :getter 解析与序列化/编译 slots 双键/DTO/前端 formulas.cfg 8 处别名 — 全链拆除 |
| registry 单名化 | VarMeta.getter 字段删(127 处)/index 只插 name; getter 名不再进内核索引 |
| 边界保留 | Java getter 名仅存于**对拍文件边界**: vm-overlay main.rs --values/--log-values 的 values.txt 跨端回灌格式(fields.rs `getter()` 专职此用途) |
| 守卫更新 | registry_single_name_no_getter_aliases(getter 名必须**不可达**, 防别名回归); canonical_var_name 简化为单通道; panel_targets_via_short_names 端到端 |

裁决: **内核(公式槽/registry/静态表/resolve)单名制; 兼容翻译只准出现在文件边界**(values.txt 对拍格式、未来 ui_layout.cfg 驱动化解析), 且须集中一处显式映射。1415 测试全绿, e2e 三场景 PASS。


**已完成**(workspace 1427 测试全绿,零回归):

| 项 | 落点 |
|---|---|
| 阶段 0 引擎 | `vm-core/src/formula/`(ast/lexer/parser/functions/eval/definition/registry,34 单测含 SMA 逐值对拍/mach 位级对拍) |
| 阶段 1 接线 | persistence 双文件;`service_loop.rs` calculate() 尾部 `formula_step`(裁决 A1 落地);ServiceData.formula_values/slots;`resolve_target` 统一解析(getter 名/短名/公式名/乘数);7 个直算 command + vm-webui 桥(publish_formula_bridge);前端公式编辑器 tab(CodeMirror 6 高亮/补全/校验/试算/变量目录) |
| 阶段 2 A 级 | energy_m/maneuver_index 公式优先+回退(hud_calculator);mach 覆写(输入位级同源证明+集成测试);formulas.cfg 内置 4 条 |
| 阶段 3 | `Blkx→BlkxPlaceholder` 转换 + 换机重建 adapter(fm.* 58 变量供值,此前恒 0);flap 双胞胎合一(共享实现,valid 检查保留=Java 保真,mock 对齐 READY 形态) |
| L2 规则引擎 | `rules.rs`(hold/cooldown 状态机+NaN 语义,6 单测);(rule ...) 持久化解析;formula_step 尾部求值→ServiceData.rule_triggers;出厂示例规则 |

**偏离备案**:
1. 公式 CRUD 走**直算命令**(commands_formula.rs)而非设计 §9 的 dispatcher 类——FormulaManager 全方法线程安全,直算更简,form_dispatch 零改动。
2. `:target` 三静态表改走 resolve_target **未接线**(解析器已就绪+单测)——表的 cfg 驱动化与阶段 4 表迁移同步做更安全。
3. 阶段 4 走 **O4 降级路径**:Deriver 四族/磁电机投票/襟翼稳定保留 Rust(已注册为 L0 变量供公式引用;磁电机投票/襟翼稳定归 C 级——多输出耦合符合 C 级判定标准);HUDData 线程迁移未做(win32 兜底计算保留,mach/energy_m/maneuver_index 已公式化双保险)。
4. Table 类型机制(Value::Table 已定义)未接入快照——FM 动态表(sweep/推力表)公式化待此设施。

**遗留清单**(W2-W5 后更新,按优先级):
- voice/flag 规则动作的消费面(语音播放/overlay 着色;数据链与前端 toast 已通)
- VoiceWarning ~17 条 check_* 判定外置为出厂规则(需真机验证语音时序)
- overlay 面板的 cfg 驱动化(行定义仍静态固化;取值已走 resolve_target, :target 公式名通路就绪)
- Table 设施 + FM 表格变量(推力表公式化)
- 编辑器:内置公式 `:test` 对拍面板、试算 200 帧环形缓冲(现单帧近似)
- 警告公式路径的独立测试锚定(现由回退路径存量测试+公式系统测试分保)

**W2-W5 实施补充记录**(2026-08-29):
- 语言扩展:`latch(cond, x)` 惰性原语(cond 假时 x 完全不求值,内部 SMA/prev 状态不污染——承载 Deriver 的"if (an != 0) 才更新"条件更新语义);`invalid()`(显式 NaN=本帧不接管)
- 新直通变量:`indic_speed`(/indicators 校正速度, Deriver 独占消费面)、`ny_raw`(/state 原始过载——注册表 ny 是派生量 an/g, an 被接管后二者语义分离,防自引用)
- formula_step 位置:calculate 内 updateEngineState **之前**(speedv 等有效功率输入需本帧公式值)
- 测试基线:1433 全绿;--log-values 对拍工具改走 Service 公式链(数据与生产一致)

## 15. 内核整合重构分析(2026-08-29, 基于 §14 实施现状; W1 已完成见 §15.3)

### 15.1 代码盘点: 公式系统替代后的去留

**可完全抛弃**(等价公式已可表达, 待写回机制+对拍设施):

| 代码 | 位置 | 替代公式 |
|---|---|---|
| Deriver 四公式族 | derive.rs step() 全部 (~150 行) | speedv/prev/an/turn_rds/turn_rate/sep/acceleration/mach 全链(§15.2 验证过 prev 语义位级对齐 speedvp) |
| FlightValues 整包快照 | derive.rs + app_shell 写回段 | FlightInfo 改吃 TelemetrySource(:target 通路), FlightValues 删除 |
| update_speed_ratio (5 量) | service_loop.rs L1273-1320 | **前置: FM 查表函数族**(fm_vne(sweep) 等, 函数库已列未实现) |
| update_stall_speed | service_loop.rs L1368 | `3.6*sqrt(2*total_weight*g/(rho0*fm.wing_area))` — 输入全在注册表, **现在就能外置** |
| check_flap 稳定计时 | methods_engine.rs L36-81 | stable() 原语 |
| flap 双胞胎求值体 | hud_calculator.rs(已合一) | **前置: FM 查表函数**(档位表插值) |
| get_maximum_rpm_learn | methods_engine.rs L91-131 | learn_max() 原语 |
| check_engine_jet 投票 | service_loop.rs L1095-1130 | vote() 组合 — 但输出三态枚举需写回转换约定 |
| slow_calculate fuel SMA | service_loop.rs L974-1019 | sma()(窗口语义需核对 0.5s 慢算周期) |
| hud_calculator 纯算术+警告判定 | hud_calculator.rs ~300 行 | energy_m✓maneuver_index✓已外置; VNE/AoA/低空警告 → 公式变量+规则 |
| 三静态表 | fields.rs / overlays_field1 / flight_info flight_value | resolve_target 已就绪, 行定义走 cfg |

**必须永久保留**(公式系统天然不管):

- 8111 轮询/HTTP/JSON 解析(State/Indicators 解析器)— 公式系统的输入面
- FM 加载解析(blkx reader/getload)
- FlightDataBus/EventBus、win32 线程/窗口/渲染/DPI
- 配置系统(config_manager/loader/S-expr parser)、Controller 生命周期/托盘/热键/焦点
- **多引擎数组状态**(eng_load 会话态/FMHandle.eng_load_state Mutex)— 单值公式语义表达不了 per-engine 状态机
- **update_engine_state 循环聚合**(Σ thrust[i], int 截断语义)— C 级, 保真难
- format_strings 字符串族(~15 个预格式化字段)— 非目标(§1.3)
- VoiceWarning 播放面(判定可外置, 资源/播放保留)
- 英制检测/compass 回退/加油检测 — C 级多输出耦合(§7 已裁决)

净效果估计: 删 ~1000-1200 行计算代码 → formulas.cfg ~40 条声明式公式(~100 行); 内核计算面几乎清空, 全部变为用户可见可改。

### 15.2 缺口清单(按依赖序)

1. **FM 查表函数族**: fm_vne(sweep)/fm_mne(sweep)/fm_aoa_high(sweep,flap)/fm_flap_allow_angle(ias)/fm_flap_allow_speed(flap)/stage_power 族 — EvalCtx 需带 FMDataAdapter(或函数经注册闭包读当前 FM)。解锁: 速度限制族/襟翼族/VNE 警告。
2. **通用公式值写回机制**: 现仅 mach 一处硬编码覆写。需要 FormulaDef 声明 `:writes` 目标字段(或约定同名接管即写回), formula_step 统一执行(NaN 守卫)。解锁: Deriver 全族接管。类型面: f64 直写; 枚举/bool 需转换约定。
3. **注册表增补 indicators 原始直通**: indicators.speed(校正速度, Deriver 独占消费)等 — speedv 链的输入前提。
4. **逐帧位级对拍设施**: 帧序列 fixture(mock_8111 场景供源)→ 旧路径/公式路径双跑逐位比对 — 删除 Deriver 的安全网。
5. **HUDData 求值迁移**: hud_calculator 迁回 Service 线程(裁决 A1 完整落地), win32 侧缩为组装+格式化。
6. **规则动作消费链**: rule_triggers → toast/语音(vm-app 接线)。解锁: VoiceWarning 判定外置。
7. (可选) 编辑器 :test 对拍面板 — 迁移验收工具。

### 15.3 整合重构路线(五波次, 每波独立可验收可停)

| 波次 | 内容 | 前置 | 验收 |
|---|---|---|---|
| **W1 地基 ✅(2026-08-29)** | FM 查表函数族(fm_vne/fm_mne/fm_aoa_high/fm_flap_allow_{speed,angle}, EvalCtx 带 blkx;get_flap_allow_speed 合一入 vm-core)+ 通用写回机制(接管语义: 公式名命中白名单→formula_step 覆写, NaN 守卫, 白名单 14 字段; mach 硬编码并入; hasFM 守卫语义改由公式 `fm_loaded ? ... : invalid()` 表达)+ 帧回放对拍设施(20 帧参数化序列+oracle)+ 常量折叠 + VarId 直接索引 | — | 1433 测试全绿; FnId 编解码宏化根治判别值移位(两事故后加往返守卫测试) |
| **W2 Deriver 消解 ✅(2026-08-29)** | latch 惰性原语(条件更新语义)+ indic_speed/ny_raw 直通变量 + 四族接管公式(speed_raw/iastotascooff/speedv/an/turn_rds/turn_rate/acceleration/sep, formula_step 提前至 engineState 前消除 speedv 一帧滞后)+ FlightInfo 改吃 TelemetrySource + **删 derive.rs/FlightValues/to_state_raw/POC live 轮询**(--log-values 改走 Service 公式链) | W1 | **20 帧位级 oracle 对拍全绿**(an/sep/turn_rate/turn_rds/acceleration 逐位相等, oracle = 删前 Deriver 输出) |
| **W3 限制/襟翼族 ✅(2026-08-29)** | speed_ratio 5 量 + stall_speed 公式化(补 trait/adapter/注册表的 fuse_cl_high/fuselage_aoa_crit_high/full_flaps cl_crit 缺口)+ **删 update_speed_ratio/update_stall_speed**; flap_allow 双胞胎求值留 C 级 check_flap 内联(W1a 已合一) | W1 | 存量 oracle(spitfire 真机数据)数值不变 |
| **W4 HUD 层瘦身 ✅(2026-08-29, 有界)** | flight_value 16 臂/PowerSource 19 臂 → resolve_target 统一解析(:target 可指向公式名); warn_vne/warn_altitude/warn_stall 公式化(hud 读公式优先+原判定回退); **HUDData 线程迁移跳过**(偏离备案: A1 的动机"状态双主"已由 W2 根除, hud_calculator 无跨帧状态, 迁移无收益) | W1 | 1433 全绿(回退路径由存量 hud 测试锚定) |
| **W5 告警统一 ✅(2026-08-29, 有界)** | 规则消费链首段: rule_triggers → vm-app 主循环 emit `rule-triggered` → 前端 toast(voice 播放/flag 着色消费面留接口); VoiceWarning 17 条判定外置**未做**(真机验证不可行, 归遗留) | W1 | 前端 typecheck/build/77 测试过 |
| W3 限制/襟翼族 (~2-3d) | speed_ratio 5 量/stall_speed/flap 族/check_flap → 公式; 删对应段 | W1 | 同上 + mock e2e |
| W4 HUD 汇聚层瘦身 (~2-3d) | hud_calculator 迁线程+警告公式化+三表走 resolve_target; 删 fields.rs 静态表 | W1-W3 | 像素对拍不回归 |
| W5 告警统一 (~2-3d) | 规则消费链 + VoiceWarning 低风险判定外置 + warn_* 走规则 | W1 | 真机语音时序不变 |

总量 ~2-3 周。删除动作一律在对拍全绿后的独立波次(删错可回退)。
