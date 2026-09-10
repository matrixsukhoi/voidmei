# HUD 真窗编辑器 — 特性全景与验证手册

> 对应特性: 所见即所得 + 自定义组件编辑 UI（"真窗即画布"重设计, R1~R8 + W1~W3 审查批次）。
> 本文 = 代码级全景图 + 人工验收场景清单。写作基线: 提交 `5e71208`。
> 姊妹文档: `doc/overlay_java_to_rust_migration.md`（迁移史）、记忆 `hud-true-window-editor-redesign`。

---

# 第一部分 · 全景图

## 0. 一句话定位

MainForm footer 点「编辑HUD」→ 桌面上**真实的 overlay 窗口变成画布**：点选/拖动/resize/吸附
参考线直接画进渲染帧；MainForm 整体切换为编辑控制台（palette + 大纲 | 工具栏 | inspector）。
没有任何快照/贴图/仿真画布 — 编辑层直改内存中的页面节点，**所见即所得 = 所改即所渲染**。

## 1. 五层架构

```
┌─ 前端控制台 (vm-webui/web/src/layout_editor/) ────────────────────────────┐
│ LayoutTab (编排) · Palette (组件目录+常用字段预设) · Outline (大纲/启用)     │
│ Inspector (组件属性/页面属性/多选对齐)                                      │
│ 数据源: hud-edit-doc (80ms 节流镜像) + hud-edit-selection + hud-edit-error │
└────────────△──────────────────────────────┬──────────────────────────────┘
        emit (bridge_hud_edit)          invoke (begin/end/edit_command)
┌────────────┴──────────────────────────────▽──────────────────────────────┐
│ IPC 层 (vm-webui): commands_edit.rs → RequestKind → 主线程 dispatcher     │
│                    → UiCommand 通道 → 渲染线程                            │
└────────────┬─────────────────────────────────────────────────────────────┘
┌────────────▽─────────────────────────────────────────────────────────────┐
│ 编辑会话 (vm-app/src/edit_session.rs ~950 行)                             │
│ EditSession { target_page, docs(编辑仓), selection, gesture,              │
│   forced_open, snapping/show_guides/marquee_mode, hit_rects, pending }    │
│ 两段式: EditBridge 闭包(即时裁决/入队/装饰) + edit_pump(手势推进/直改节点)  │
│ apply_command → CommandEffect { None | Light | Full } 分级处理             │
└────────────┬─────────────────────────────────────────────────────────────┘
┌────────────▽─────────────────────────────────────────────────────────────┐
│ 渲染线程 (vm-app/src/render_thread.rs)                                    │
│ RenderSession { strategies(激活策略表), edit, params(reinit 参数仓), ... } │
│ on_begin/on_end_edit_session · rebuild_edit_target · on_reinit_overlays   │
│ (编辑仓写权接管) · 主循环 10ms 泵内调 edit_pump                            │
└────────────┬─────────────────────────────────────────────────────────────┘
┌────────────▽─────────────────────────────────────────────────────────────┐
│ 窗口宿主 (vm-overlay/src/platform/): host.rs EditBridge 三闭包面 +         │
│ win.rs WNDPROC (WM_RBUTTONDOWN/WM_LBUTTONDBLCLK, CS_DBLCLKS)              │
│ PageOverlay 页面编排器 (widgets 域) — 组件节点的真渲染面                    │
└──────────────────────────────────────────────────────────────────────────┘
```

## 2. 数据模型与持久化

**PageDoc**（`vm-core/src/config/json_model.rs`, serde camelCase）:

| 字段 | 语义 |
|---|---|
| `id` | 唯一键 = host 条目键 = 位置存档键 |
| `name` | 显示名 |
| `activation` | `None` 恒显；`{key, requires:["jet"/"live"]}` 声明式激活 |
| `pos: [f64;2]` | 归一化窗口位置（唯一真源，拖窗落盘走这里） |
| `padding` | 包围盒留白 |
| `sizing` | `auto`（内容自适应）/ `fixed{w,h}`（thrust-chart） |
| `dock` | `bottomLeft{fromBottom}`（thrust-chart 贴屏底） |
| `dataface` | `page`（常规喂入）/ `sidecar`（FM 黑盒自管数据面） |
| `startHidden` | live 形态起步隐藏（fm-list 热键显隐语义） |
| `font.sizeAdd` | 页字号增量 |
| `interestKeys` | 页级兴趣键（∪ 组件 config_keys = WYSIWYG 刷新键集） |
| `contentVersion` | 出厂版本号（升级跟随判断） |
| `components` | `ComponentDoc[]` |

**ComponentDoc**: `{id, type, pos(行高倍), anchor[自身,父], parent, visibleWhen, enabled, size(物理px覆盖|null), props}`。

**持久化 = 出厂 ⊕ delta**（`vm-core/src/config/json_store.rs` + `configuration_service/mod.rs`）:

- 出厂 = `factory_default.json` 编译期内嵌；用户 = `voidmei_config.json`（delta）。
- 出厂页被编辑 → **owned 区整页提升**（记 contentVersion）；用户页 → user 区 upsert。
- 拖窗位置: 出厂未提升页走 `page_positions` 轻量区（拖一下不整页提升）。
- 提交链: `commit_pages(&pages)` 批量 — N 页**一次落盘 + 一次广播**。

## 3. 出厂页面（9 页, factory_default.json）

| 页 id | 组件数 | 特殊面 |
|---|---|---|
| `minihud-default` | 16 | **专用编排器页, 不可编辑**（minihud_overlay_spec 独占, 不走 PageOverlay） |
| `flight-info-default` | 16 | — |
| `power-info-default` | 19 | — |
| `engine-control-default` | 7 | — |
| `gear-flaps-default` | 2 | — |
| `axis-default` | 6 | — |
| `attitude-default` | 1 | core.attitude.window 黑盒 |
| `fm-list-default` | 1 | core.fm.list 黑盒（zebra 可伸长列表）; sidecar + startHidden |
| `thrust-chart-default` | 1 | core.fm.thrust_chart 黑盒; sidecar + fixed 900×500 + dock bottomLeft 500 |

## 4. 组件注册表（palette 目录, `vm-overlay/src/widgets/registry.rs`）

| 族 | type_name | 品类 |
|---|---|---|
| MiniHUD 行族 | `core.minihud.speed/aoa/altitude/energy/flaps/airbrake/gear/sep/gload/maneuverbar/flapBar/...` | 文本/仪表（外观由设置面板配置键驱动, 无 props 表单） |
| 数据字段 | `core.data.field` | 文本（target=公式变量, 有完整 propsSchema） |
| 引擎仪表 | `core.engine.gauge` | 仪表（kind 选型） |
| 复合黑盒 | `core.axes.crosshair` / `core.attitude.window` / `core.fm.list` / `core.fm.thrust_chart` | 内部不可拆, palette 打「黑盒」标 |
| 起落襟翼原子 | `core.gearflaps.flapbar` / `core.gearflaps.warn` | 仪表/文本 |
| 轴系原子 | `core.axes.rudderbar` | 仪表 |
| FM 字段 | `core.fm.field` / `core.fm.meta` | 文本（sidecar 数据面） |

palette 另有「常用字段」预设区（出厂 fieldPresets: 表速/真空速/马赫数…, 点击 = 完整配置组件）。

## 5. 编辑会话生命周期

```
            footer「编辑HUD」(invoke begin_edit_session)
                      │ 主线程 dispatcher → UiCommand::BeginEditSession
                      ▼
   ┌─ on_begin_edit_session (渲染线程) ─────────────────────┐
   │ 守卫: 已有会话→EditRejected / state≠Preview→EditRejected │
   │ ① host.dialog_will_show()        ← 压 z 序, MainForm 不被盖 │
   │ ② 全页 force_open_preview        ← 未开窗页强制可见, 记 forced_open │
   │ ③ 编辑仓 docs = params.pages 快照  ← 会话期页文档唯一真相 │
   │ ④ host.set_edit_bridge(三闭包)    ← 共享 Rc<EditSession> │
   │ ⑤ publish HUD_EDIT_SESSION "begin" → 前端整体切 LayoutTab │
   └──────────────────────────────────────────────────────┘
                      │
        ┌── 编辑期 (每轮主循环 10ms) ──┐
        │ edit_pump: 消费事件队列        │
        │  → 手势推进 (直改 PageOverlay  │
        │    节点 + doc 双写)           │
        │  → refresh_cache (hit_rects)  │
        │  → 80ms 节流推前端            │
        │  → 活动时即时 render_tick      │
        │ UiCommand::Edit(cmd):         │
        │  apply_command → effect 分级   │
        │   None: 仅装饰刷新             │
        │   Light: 缓存刷新+即时渲染     │
        │   Full: rebuild_edit_target   │
        │    (策略表重建 + params.pages  │
        │     覆写 + reinit_active +     │
        │     缓存刷新)                  │
        └────────────┬────────────────┘
                     │ invoke end_edit_session(commit)
                     ▼
   ┌─ on_end_edit_session ────────────────────────────────┐
   │ 卸桥 · dialog_did_dismiss · 收回 forced_open 页        │
   │ commit=true  → MainEvent::EditCommitted{pages,deleted}│
   │ commit=false → MainEvent::EditDiscarded               │
   └────────────┬──────────────────────────────────────────┘
                ▼
   主线程 (lib.rs handle_main_event):
   · EditCommitted: 逐 deleted delete_page + commit_pages(一次落盘+一次广播)
     → CONFIG_CHANGED → Controller 全参数 ReinitOverlays + RefreshPreviews
     → sync_page_entries 对齐窗口集 (新页落窗/删页摘窗)
   · EditDiscarded: OverlayInputs 全量重建 → ReinitOverlays + RefreshPreviews
     (外部真相覆盖编辑态)
```

**自动收尾**: 托盘 Activate/Start → `EndEditSession{commit:true}`（防编辑内容丢失/进游戏前必退）。

## 6. 真窗交互层细节（两段式, 防借用冲突）

**EditBridge 三闭包**（host.rs, 全部只碰 `Rc<RefCell<EditSession>>`）:

| 闭包 | 时机 | 实现 |
|---|---|---|
| `on_press(id,x,y,win_pos)` | WM_LBUTTONDOWN, **同步** | `press_decision` 用缓存矩形立即裁决: true=编辑接管(host 不启整窗拖) |
| `on_event(id, EditMouse)` | Move(无拖拽时)/Release/RightPress/DoubleClick | 入 `pending` 队列, edit_pump 消费 |
| `on_paint(id, canvas)` | render 闭包之后 | `paint_decorations` 用缓存矩形画装饰 |

**命中测试优先级**（`hit_test`, 坐标 = 屏幕 − window.position − sizing.offset）:
1. resize 手柄（仅单选, 8 向, 命中 10×10 物理 px / 绘制 6×6）
2. 组件本体（doc 逆序 = z 顶层优先, 2px 容差）
3. 空白（marquee_mode 开 = 框选; 否则清选 + host 整窗拖 → 位置走 PageDoc.pos 持久化链）

**press_decision 特例**:
- 点**非目标页**窗口（可编辑页）= 切目标页, 装饰转移, 该击不启手势;
- 点 `minihud-default` = 不切不编（返回 false, 纯窗口交互）。

**手势**（`EditGesture`）:
- `Move`: Δroot/行高 → `node.set_relative_position` + doc 双写; 吸附 = 网格 round 0.1 行高
  （仅首个移动组件原点, 阈值 6px）+ 对齐线（选中集 bbox 6 特征线 × 静止组件同特征线, 取最优）;
  窗口包围盒跟随（每次 apply_auto_sizing + 条件 resize_entry）。
- `Resize`: 手柄方位 × Δ → 新矩形（最小 8px 钳制; N/W 手柄 origin 位移 → pos 换算跟随）;
  `set_size_override` + doc 写 `size`; DIB 昂贵 → 33ms 节流, 松手补终值。
- `Marquee`: 画布系起止点, 松手与缓存矩形求交 → 选中集。

**装饰**（只画目标页, 粉色 `#FF69B4` 系）: hover 1px / 选中 2px + 手柄 / 参考线虚线 /
marquee 框 + 20 alpha 填充。像素变化自然触发 host 脏检查 present。

## 7. 命令面（EditCommand, serde camelCase tag=kind）

`SetTargetPage / Select / Nudge / UpdateComponent{comp,old_id} / RemoveComponents /
ReorderComponent / UpdatePage / UpsertPage / DeletePage / SetOptions`

- 前端 LayoutTab 是唯一发送方（键盘微调/属性面板/页面管理/大纲选择）。
- `UpdateComponent.old_id`: 改名 = 新 id 替换旧槽位 + parent 引用重指 + 选中集跟随;
  撞名 → Err → hud-edit-error toast。
- `Nudge` doc + 真窗节点**双写**（方向键微调即时生效）。
- 页面管理命令（upsert/delete/setTargetPage）也是会话内命令, **退出时统一提交**。

## 8. 前端控制台（vm-webui/web/src/layout_editor/）

| 组件 | 职责 |
|---|---|
| `LayoutTab.tsx` | 编排: 事件监听(hud-edit-*) / 撤销重做快照栈(50 层) / 键盘 / 页面管理 / 对齐换算 |
| `Palette.tsx` | 组件目录（点击添加, R8 后无拖放）+ 常用字段预设 |
| `Outline.tsx` | doc 序平铺 + parent 缩进; enabled 开关（禁用组件画布不渲染, 大纲是唯一寻回入口）; Ctrl/Shift 多选 |
| `Inspector.tsx` | 单选: id 改名/启用/坐标/锚点/父/显示条件 + propsSchema 表单(Str/Int/Bool/Enum/Color/Target); 无选中: 页面属性; 多选: 对齐/批量删复制 |
| `editApi.ts` | begin/end/editCommand 三 invoke |
| `App.tsx` | footer「编辑HUD」↔「保存编辑」; editSession 态整体切 LayoutTab; 编辑期禁用 保存/刷新预览/导入/开始; 窗口 1280×800 + localStorage 记忆 |

撤销 = 前端页文档快照栈 + `updatePage` 回放（`undoable=false` 的命令不入栈 — 见 §11 已知边界）。

## 9. 互斥与保护清单

| 保护点 | 机制 |
|---|---|
| 编辑期外部配置变更 | `on_reinit_overlays`: new_params.pages 被编辑仓覆写（gauge/hud/dpi 照收）— 写权接管 |
| 编辑期 FocusMonitor 失焦隐藏 | `HideAllOverlays` 命令在 `session.edit.is_some()` 时忽略 |
| 编辑期 MainForm 被盖 | `dialog_will_show` 挂起计数压 z 序 |
| 游戏态进入编辑 | `state != Preview` → EditRejected |
| 绕过编辑仓的落盘 | footer 保存/导入/恢复出厂/开始 全部禁用 |
| 僵尸 forced_open 页 | 退出时逐页 close, refresh_preview 按激活探测重开应开的 |

## 10. 非 editing 的 WYSIWYG 链路（同一特性面）

常规设置面板改键 → `set_config` → CONFIG_CHANGED → Controller:
- 兴趣键命中 → `RefreshPreviews{changed_key}` → `refresh_preview_key`（WYSIWYG 局部刷新）;
- 全参数变化 → `ReinitOverlays{params}` → 各 spec reinit 闭包重取参数仓重建。
- 页面激活 = `strategies` 表（entry id → `strategy_of(doc)`, 注册/sync/rebuild_edit_target 三处重建）,
  host 激活探测闭包 Rc 共享恒读最新表。
- 位置链: 拖窗（编辑内外）→ `PositionSaved{page_id}` → `save_page_position` → delta。

## 11. 已知边界与本轮复查新发现

**设计内边界**:
1. `minihud-default` 不可组件级编辑（专用编排器, 需先统一进 PageOverlay 面 — 备案）。
2. 撤销栈只覆盖目标页文档变化; 页面级操作（新建/删除/复制页/恢复出厂）`undoable=false` 不入栈。
3. 崩溃丢编辑（无 localStorage 草稿恢复, UI 已引导「保存并退出」）。
4. `lineHeightPx` 推送字段是占位（= padding, 前端不消费）。
5. resize 组件 size 为物理 px 覆盖, 不随字号缩放（备案: 尺寸语义后续统一）。

**本轮复查新发现的断链（⚠ 未修, 见第二部分 N 组核对项）**:
- **N1 画布多选不可达**: `marquee_mode` 前端无开关（默认 false）, `EditMouse` 不带修饰键 →
  画布上既不能 Shift/Ctrl 加选也不能拖框; Inspector 提示文案「Shift/Ctrl 点选或拖框」与实际不符。
  多选唯一可达路径 = 大纲 Ctrl/Shift 点选。
- **N2 构建失败提示链死**: `push_doc_event` 硬编码 `errors: []` → LayoutTab 的
  「N 个组件构建失败」Alert 与 Outline 红点永远不显示（EditSession 不跟踪重装配错误）。

---

# 第二部分 · 验证场景清单

> 使用方式: Preview 态（未开始游戏）启动应用, 逐项勾选。标注 ⚠ 的是已知断链/边界, 预期值 = 当前实际行为。

## A. 会话进出与入口

- [ ] **A1 正常进入**: Preview 态 → footer 点「编辑HUD」→ 所有 HUD 窗口强制可见（含平时隐藏的 FM 拆包页）,
  MainForm 切换为三栏控制台, 窗口放大到 ~1280×800, 提示条显示「正在编辑「xxx」— 画布就是桌面上的 HUD 窗口」。
  MainForm 不被 overlay 盖住（可正常点按）。
- [ ] **A2 游戏态拒绝**: 开始游戏（InGame/Connected）后托盘/其它路径尝试进入 → toast
  「请先结束游戏会话再编辑」, 不进入编辑态。
- [ ] **A3 footer 门控**: 编辑期「保存」「刷新预览」「导入配置」按钮禁用（悬停有提示）;
  「开始」禁用; footer 按钮变为「保存编辑」。
- [ ] **A4 窗口尺寸记忆**: 编辑期拖大 MainForm → 退出 → 再次进入 → 恢复上次编辑窗口尺寸;
  退出后常规窗口尺寸不受污染。
- [ ] **A5 托盘自动收尾**: 编辑期点托盘「开始」→ 编辑自动**提交**退出并进入游戏流程;
  点托盘主图标（Activate）→ 编辑自动提交退出。
- [ ] **A6 退出恢复**: 退出编辑后 MainForm 回常规设置面板, forced_open 的页收回,
  各页按激活探测恢复应有可见态, z 序恢复（overlay 重新恒顶）。

## B. 画布点选

- [ ] **B1 单选**: 点击某页窗口内组件 → 粉色 2px 选中框 + 8 个 6×6 手柄; Inspector 显示该组件属性。
- [ ] **B2 z 序**: 两组件重叠 → 点击重叠区选中**视觉顶层**（doc 序后者）。
- [ ] **B3 hover**: 无手势时鼠标掠过组件 → 1px 半透明粉框跟随, 离开消失。
- [ ] **B4 空白**: 点击组件外空白 → 清空选中; 按住空白拖动 = 移动**整窗**（非组件）。
- [ ] **B5 切目标页**: 点击另一可编辑页的窗口 → 装饰（选中框等）消失转移, 控制台目标页下拉同步,
  该次点击不产生拖拽副作用。
- [ ] **B6 MiniHUD 排除**: 点击 MiniHUD 窗口 → 不切换目标页, 无装饰, 行为与普通窗口一致
  （拖动它 = 移动整窗）; 目标页下拉中无「MiniHUD」项。

## C. 拖动与吸附

- [ ] **C1 跟手拖动**: 按住组件拖 → 组件实时跟随（无明显滞后）, 窗口包围盒随内容外扩/收缩;
  松手后位置保持, Inspector 坐标值同步。
- [ ] **C2 网格吸附**: 开「吸附」拖动 → 位置按 0.1 行高格点跳动（靠近格点 ≤6px 时吸附）。
- [ ] **C3 对齐参考线**: 开「参考线」拖动组件靠近另一组件边缘/中线 → 出现粉色虚线对齐线,
  松手后虚线消失。
- [ ] **C4 关吸附**: 关「吸附」→ 自由连续拖动, 无格点无参考线。
- [ ] **C5 多选成组拖动**: 大纲 Ctrl 点选 2+ 组件 → 真窗单选其中一个拖动 → 全组同步位移
  （组内相对位置不变）。

## D. Resize

- [ ] **D1 八向**: 单选组件 → 拖 8 个手柄任一 → 对应边/角改变组件尺寸, 内容重排。
- [ ] **D2 最小钳制**: 向内压缩到极小 → 尺寸停在 ~8px 不再缩小。
- [ ] **D3 N/W 手柄**: 拖北/西侧手柄 → 组件起点（pos）跟随移动而非只改宽高。
- [ ] **D4 包围盒与节流**: resize 中窗口外框平滑跟随（约 30Hz）, 松手后终值精确落位;
  Inspector 中该组件出现 size 值; 重启后 size 保持。
- [ ] **D5 清除 size**: Inspector 无直接清除入口时, size 一旦设置即持久 ⚠（边界: 覆盖式尺寸,
  无「恢复自适应」操作 — 如需验证恢复, 用恢复出厂页兜底）。

## E. 键盘（控制台聚焦时）

- [ ] **E1 微调**: 选中组件 → 方向键 = 0.1 行高步进; Shift+方向键 = 1.0 行高; 真窗即时移动。
- [ ] **E2 删除**: Delete/Backspace → 选中组件删除（多选全删）。
- [ ] **E3 撤销重做**: Ctrl+Z 逐步回退 / Ctrl+Shift+Z 前进; 按钮深度计数正确; 50 层截断。
- [ ] **E4 输入守卫**: 焦点在 Inspector 输入框内按方向键/Delete → 不触发画布命令（正常打字）。

## F. 组件增删改

- [ ] **F1 palette 添加**: 点击目录项（如「数据字段」）→ 当前页出现新组件（有合法默认值, 立即可见可拖）。
- [ ] **F2 常用字段预设**: 点击「表速」等预设 → 落地即为完整配置组件（label/target/unit/previewValue 齐）。
- [ ] **F3 props 即时生效**: Inspector 改 label 文本 / precision 数值 / Bool 开关 / Enum 选型 /
  Color 颜色 → 真窗对应变化无需退出。
- [ ] **F4 Target 目录**: Target 输入框出公式变量下拉（带单位显示）; 选中带单位的变量且 unit 空时
  → unit 自动带出。
- [ ] **F5 改名**: Inspector id 框输入新 id + Enter → 画布/大纲/父组件引用同步; 撞名 → 红字拒绝。
- [ ] **F6 复制/删除**: 「复制」= 原位偏移 0.5 行高的副本; 删除父组件 → 子组件 parent 自动置空
  （不连带删除, 不悬空）。
- [ ] **F7 enabled**: 大纲眼睛开关关闭 → 画布上组件消失（不渲染）; 重新开启恢复; 禁用期间在大纲仍可选中编辑属性。

## G. 大纲

- [ ] **G1 结构**: 页组件按 doc 序平铺, parent 链缩进展示; 每行右侧显示组件中文名。
- [ ] **G2 多选**: Ctrl/Shift 点击多行 → 多选高亮, Inspector 切多选批量栏。
- [ ] **G3 联动**: 大纲选中 ↔ 画布选中框 ↔ Inspector 三方同步（任一处选择, 其余两处跟随）。

## H. 页面管理（会话内命令, 退出时生效）

- [ ] **H1 切换**: 目标页下拉切换 → 装饰/大纲/Inspector 全部切到新页。
- [ ] **H2 新建页**: 「新建页」→ 空页成为目标 → 从 palette 添加组件 → 保存退出 → 桌面出现新窗口;
  重启后仍在（delta user_pages）。
- [ ] **H3 复制页**: 「复制页」→ 副本含全部组件 → 保存退出 → 双窗并存。
- [ ] **H4 删除页**: 「删除页」（有确认框; 仅剩一页时禁用）→ 保存退出 → 该窗口从桌面消失;
  编辑器目标页自动落到余下第一页。
- [ ] **H5 恢复出厂**: 编辑过组件的出厂页 → 「恢复出厂」→ 内容回出厂版（提示「退出时落盘」）→
  保存退出 → 重启后确为出厂内容（delta owned 区该页已移除）。
- [ ] **H6 删除即时性**: 删除页在**保存退出前**桌面窗口仍在（提交才生效）; 「放弃」则删除被回滚。

## I. 提交 / 丢弃

- [ ] **I1 保存并退出**: 工具栏「保存并退出」→ 会话退出, 全部改动（含页面管理）落盘;
  重启应用 → 改动全部在场（voidmei_config.json delta 核对: 出厂页编辑 → owned 区;
  用户页 → user 区; 位置 → page_positions）。
- [ ] **I2 放弃**: 「放弃」（确认框）→ 全部改动回滚（含新建页消失、删除页复活）,
  界面回设置面板, 真窗回外部真相形态。
- [ ] **I3 footer 保存编辑**: footer「保存编辑」= 立即提交（等价保存并退出）。
- [ ] **I4 批量性**: 多页改动一次提交 → 只触发一次全量重建（无多次闪烁/级联刷新）。
- [ ] **I5 位置持久化**: 编辑会话内拖空白移动整窗 → 保存退出 → 重启 → 窗口位置保持
  （含用户页与出厂未提升页 — page_positions 轻量区, 不整页提升）。

## J. 互斥与保护

- [ ] **J1 写权接管**: 编辑期外部 CONFIG_CHANGED（如另一途径改配置）→ 编辑中页面实例不被替换,
  编辑仓内容保持（gauge/dpi 类参数照常生效）。
- [ ] **J2 失焦不隐藏**: 编辑期 Alt+Tab 切走焦点 → HUD 画布窗口不隐藏（FocusMonitor 被忽略）。
- [ ] **J3 策略表同步**: 编辑期改某页「开关键」（Inspector 页面属性）→ 保存退出 →
  设置面板拨该开关 → 该页窗口正确开/关（激活策略随编辑更新）。
- [ ] **J4 MiniHUD 完整性**: 编辑会话进出后 MiniHUD 页渲染/行为无任何变化。

## K. 非 editing 的 WYSIWYG（常规链路回归）

- [ ] **K1 开关键即时刷新**: 设置面板拨某 overlay 组开关/显示项 → 对应真窗即时显隐/改样式（不重启）。
- [ ] **K2 字号/颜色**: 改页字号增量或全局五色 → 真窗重装配生效。
- [ ] **K3 激活语义**: thrust-chart 仅 jet+开关键开时激活; fm-list 热键显隐、起步隐藏正常。
- [ ] **K4 升级提示**: 用户 owned 页 contentVersion 低于出厂 → 打开编辑器出现升级提示。

## L. FM 特殊页编辑

- [ ] **L1 fm-list**: 编辑态它以静态预览行显示（zebra 列表整组件, 单选中可拖动/整组件属性面板
  显示「由设置面板配置驱动」的键标签）; 拖动保存后重启位置保持。
- [ ] **L2 thrust-chart**: 编辑态可见（fixed 900×500 + 贴屏底 dock）; ⚠ 它的固定几何在编辑态
  同样生效 — 拖动组件正常, 拖空白整窗被 dock 钳制属预期。
- [ ] **L3 语义保持**: 编辑 FM 页后保存退出 → 游戏态热键切换/段开关/show* 行归零收缩全部正常。

## M. 崩溃/异常边界

- [ ] **M1 崩溃丢编辑**: 编辑中强杀进程 → 重启无残留编辑（已知边界: 草稿不恢复, 无异常半提交态）。
- [ ] **M2 畸形手写 delta**: 手改 voidmei_config.json 塞非法组件 → 编辑器/加载不 panic
  （组件构建失败仅不渲染）⚠ N2: 目前无红点/Alert 提示 — 核对「静默不渲染」即通过。
- **N 组 ⚠ 已知断链核对**（预期 = 当前实际行为, 修复后本组预期翻转）:
- [ ] **N1 画布多选**: 真窗上 Shift/Ctrl 点击组件 → **不**加入选中（单击恒单选）; 拖框 → **无**框选
  （空白拖 = 移动整窗）。多选唯一路径 = 大纲 Ctrl/Shift 点选。Inspector 提示文案与实际不符（已知）。
- [ ] **N2 错误提示恒空**: 构建失败的组件 → 控制台**无**「N 个组件构建失败」Alert, 大纲**无**红点。

---

## 附: 快速回归路径（最小集）

时间紧时至少跑: A1 → B1/B4/B5 → C1/C3 → D1 → E1/E3 → F1/F3 → H2 → I1/I2 → J4 → K1。
覆盖进出场、核心手势、属性即时性、页面增删、持久化与 MiniHUD 无损。
