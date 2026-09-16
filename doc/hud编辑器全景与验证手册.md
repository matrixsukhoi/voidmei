# HUD 真窗编辑器 — 特性全景与验证手册

> 对应特性: 所见即所得 + 自定义组件编辑 UI（"真窗即画布"重设计, R1~R8 + W1~W3 审查批次）。
> 本文 = 代码级全景图 + 人工验收场景清单。写作基线: 提交 `5e71208`; 2026-09-12 试驾场原生化 A~D 落地后同步。
> 姊妹文档: `doc/overlay_java_to_rust_migration.md`（迁移史）、记忆 `hud-true-window-editor-redesign`。

---

# 第一部分 · 全景图

## 0. 一句话定位

MainForm footer 点「编辑HUD」→ 桌面上**真实的 overlay 窗口变成画布**：点选/拖动/resize/吸附
参考线直接画进渲染帧；编辑窗口群全部原生自绘（侧栏 + 悬浮条），MainForm 隐退让屏。
没有任何快照/贴图/仿真画布 — 编辑层直改内存中的页面节点，**所见即所得 = 所改即所渲染**。

## 1. 五层架构

```
┌─ 编辑 chrome (voidmei/src/edit_chrome.rs, 原生 skia 自绘窗口群) ─────────┐
│ 侧栏: 页面tab(D1 管理: ＋新建/右键复制·删除·恢复出厂) · 组件库(D2 缩略图    │
│   + D3 搜索, widgets/catalog.rs) · 大纲(缩进+enabled 方块) · 属性表单      │
│   (B2, widgets/prop_form.rs; Text 行 = Win32 EDIT 子控件+IME)            │
│ 悬浮条: 撤销/重做/场景拨杆/放弃(3s 二次确认)/完成 — 同线程直达编辑会话      │
└────────────△──────────────────────────────────────────────────────────────┘
        唯一编辑 IPC 入口: invoke (begin_edit_session)
┌────────────┴──────────────────────────────────────────────────────────────┐
│ IPC 层 (webui): commands_edit.rs begin_edit_session → RequestKind →    │
│   主线程 dispatcher → UiCommand 通道 → 渲染线程 (阶段 C 后唯一编辑 IPC;   │
│   编辑动作全在渲染线程内部; HUD_EDIT_SESSION 起止键驱动 MainForm 隐退/恢复) │
└────────────┬─────────────────────────────────────────────────────────────┘
┌────────────▽─────────────────────────────────────────────────────────────┐
│ 编辑会话 (voidmei/src/edit_session.rs ~1590 行)                           │
│ EditSession { target_page, docs(编辑仓), selection, gesture,              │
│   forced_open, snapping/show_guides/marquee_mode, hit_rects, pending }    │
│ 两段式: EditBridge 闭包(即时裁决/入队/装饰) + edit_pump(手势推进/直改节点)  │
│ apply_command → CommandEffect { None | Full } 分级处理                    │
└────────────┬─────────────────────────────────────────────────────────────┘
┌────────────▽─────────────────────────────────────────────────────────────┐
│ 渲染线程 (voidmei/src/render_thread.rs)                                    │
│ RenderSession { strategies(激活策略表), edit, params(reinit 参数仓), ... } │
│ on_begin/on_end_edit_session · rebuild_edit_target · on_reinit_overlays   │
│ (编辑仓写权接管) · 主循环 10ms 泵内调 edit_pump                            │
└────────────┬─────────────────────────────────────────────────────────────┘
┌────────────▽─────────────────────────────────────────────────────────────┐
│ 窗口宿主 (overlay/src/platform/): host.rs EditBridge 三闭包面 +         │
│ win.rs WNDPROC (WM_RBUTTONDOWN/WM_LBUTTONDBLCLK, CS_DBLCLKS)              │
│ PageOverlay 页面编排器 (widgets 域) — 组件节点的真渲染面                    │
└──────────────────────────────────────────────────────────────────────────┘
```

## 2. 数据模型与持久化

**PageDoc**（`kernel/src/config/json_model.rs`, serde camelCase）:

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

**持久化 = 出厂 ⊕ delta**（`kernel/src/config/json_store.rs` + `configuration_service/mod.rs`）:

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

## 4. 组件注册表（palette 目录, `overlay/src/widgets/registry.rs`）

| 族 | type_name | 品类 |
|---|---|---|
| MiniHUD 行族 | `core.minihud.speed/aoa/altitude/energy/flaps/airbrake/gear/sep/gload/maneuverbar/flapBar/...` | 文本/仪表（外观由设置面板配置键驱动, 无 props 表单） |
| 数据字段 | `core.data.field` | 文本（target=公式变量, 有完整 propsSchema） |
| 引擎仪表 | `core.engine.gauge` | 仪表（kind 选型） |
| 复合黑盒 | `core.axes.crosshair` / `core.attitude.window` / `core.fm.list` / `core.fm.thrust_chart` | 内部不可拆, palette 打「黑盒」标 |
| 起落襟翼原子 | `core.gearflaps.flapbar` / `core.gearflaps.warn` | 仪表/文本 |
| 轴系原子 | `core.axes.rudderbar` | 仪表 |
| FM 字段 | `core.fm.field` / `core.fm.meta` | 文本（sidecar 数据面） |

「常用字段」预设区（出厂 fieldPresets）已随 web 控制台退役 — 现状 = 侧栏组件库目录点击添加,
target 等字段经属性表单手输。

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
   │ ⑤ publish HUD_EDIT_SESSION "begin" → MainForm 隐退让屏 │
   │    (唯一对外事件; web 控制台/hud-edit-* 镜像已退役)      │
   └──────────────────────────────────────────────────────┘
                      │
        ┌── 编辑期 (每轮主循环 10ms) ──┐
        │ edit_pump: 消费事件队列        │
        │  → 手势推进 (直改 PageOverlay  │
        │    节点 + doc 双写)           │
        │  → refresh_cache (hit_rects)  │
        │  → chrome 追平 (大纲/属性区    │
        │    随 doc 变化重渲染)          │
        │  → 活动时即时 render_tick      │
        │ 命令 = 渲染线程内部直达         │
        │  (edit_chrome/菜单/拖放结算     │
        │   同线程构造 → apply_command): │
        │   None: 仅装饰刷新             │
        │   Full: rebuild_edit_target   │
        │    (策略表重建 + params.pages  │
        │     覆写 + 增删页同步窗口集 +  │
        │     reinit_active + 缓存刷新)  │
        └────────────┬────────────────┘
                     │ 悬浮条「完成 ✓」(commit=true) /「放弃」(commit=false)
                     │ (渲染线程内部; 托盘自动收尾经 UiCommand 直达)
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
3. 空白（marquee_mode 开 = 框选; 否则清选 + host 整窗拖 → 位置走 PageDoc.pos 持久化链。
   marquee_mode 无开关恒 false — 框选不可达, 见 N1）

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

## 7. 命令面（EditCommand, 渲染线程内部 — C 阶段后无 IPC/serde）

`SetTargetPage / Select / Undo / Redo / InsertComponent{comp,at} /
UpdateComponent{comp,old_id} / RemoveComponents / ReorderComponent /
UpdatePage / UpsertPage / DeletePage / SetScenario{name}`

- 构造方: edit_chrome（侧栏/属性表单/页面管理 tab 菜单）与真窗菜单/拖放结算 —
  全部同线程直达 apply_command（原 web IPC 变体 SetOptions/DragInsertBegin/Nudge 已随阶段 C 退役）。
- `UpdateComponent.old_id`: 改名 = 新 id 替换旧槽位 + parent 引用重指 + 选中集跟随;
  撞名预检在**入撤销栈之前**（拒绝路径不留 no-op 快照 — Err 后栈上无冗余项, 免多按一次撤销）。
- 页面管理命令（upsert/delete/setTargetPage）也是会话内命令, **退出时统一提交**;
  编辑期增删页即时同步窗口集（新页落窗/删页摘窗, rebuild_edit_target 收口）。
- **撤销/重做 = 会话级全局单一栈**（`edit_session.undo_stack` 全量编辑仓快照）:
  手势（Move/Resize 整段一项）/ 命令 / 菜单动作 / 拖放落件 / 属性表单改值全部入栈;
  按钮使能由 chrome 状态直接驱动（前端旧局部栈与 hud-edit-doc 推送已退役）。
- `InsertComponent`: 拖放/点击添加的落件结算（见 §8a）。
- `SetScenario`: 场景拨杆位（normal/jet/gear/nodata, 会话选项族不入撤销栈）。

## 8. 编辑窗口群（试驾场, 原生 skia 自绘 — `voidmei/src/edit_chrome.rs`）

**形态（2026-09-12 试驾场原生化 A~D 落地）**: 编辑会话 begin/end 在渲染线程直建/销毁 —
侧边栏（右缘满高: 页面tab / 组件库(D2 缩略图 + D3 搜索) / 大纲(缩进 + enabled 方块) /
属性表单(B2 常驻底部)）+ 悬浮条（上缘居中五钮: ↩撤销 ↪重做 场景拨杆(四态循环)
放弃(3s 二次确认) 完成✓ — 无「属性」钮）。与真窗同栈: WinOverlay 复用 + skia 绘制 +
同一页面字体/调色板; webview 面板 (edit-toolbar/edit-panel) 与 web Inspector 已退役,
MainForm 经 bridge.rs 隐退/恢复（HUD_EDIT_SESSION 唯一对外事件）; 长尾属性 = 侧栏属性
表单常驻 — 选中组件 = 组件属性, 无选中 = 页面属性; Text 行 = Win32 EDIT 子控件
（系统白给 IME/光标/选择/剪贴板, 失焦提交）, IntStepper/Toggle/EnumCycle 自绘每击提交,
全部入单一撤销栈。

| 原生面 | 职责 |
|---|---|
| 页面tab (D1) | 行尾「＋」新建默认名页（零弹窗, 属性区改名）+ 右键菜单: 复制页 / 删除页（仅剩一页禁用）/ 恢复出厂（出厂页专属, 保用户 pos） |
| 组件库 (D2/D3) | 分组目录（`widgets/catalog.rs`）+ 条目缩略图（120×52 静态小样, `widgets/thumbnail.rs`）+ 搜索框（EDIT 子控件, 每键过滤 + 标题命中数, 空 = 全量）; 点击添加 / 按下拖放发起 |
| 大纲 | doc 序平铺 + parent 缩进 + enabled 方块（禁用组件画布不渲染, 大纲是唯一寻回入口）; 单击选中 |
| 属性表单 (B2) | 行模型纯函数下沉 `widgets/prop_form.rs`; `__id/__enabled/__visible_when` 通用行 + propsSchema 行（Text/IntStepper/Toggle/EnumCycle）; 失焦/每击提交入撤销栈; 区头「删除」= 删选中 |
| Win32 子控件基座 | `overlay/src/platform/controls.rs`: EDIT 文本框 + GDI 字体; WM_COMMAND → OverlayEvent::Control; 自绘 IME 明确不做（重造输入法轮子风险不可控） |
| 两档字号 (D4a) | tab 名/区头标题/截断标注类装饰文字 = 主字号 × 0.72 小档 |

## 8a. 拖放组件库（侧栏 → 真窗, 全局鼠标轮询）

链路: 侧栏组件库条目按下（同线程直连 `es.drag_insert` 置位）→ 渲染线程编辑泵 ⓪ 步轮询
`platform::cursor`（GetCursorPos/GetAsyncKeyState）→ `compute_drop_hint` 算落点
（容器子项 = 项间插入线（上半个项前插/下半个后插）; 容器本体 = 尾插线;
自由区 = 10px 网格幽灵）→ paint_decorations 画反馈 → 左键释放
`build_insert_command` 结算（`InsertComponent{comp, at}` 入撤销栈, 落地即选中）。
悬在窗口外释放 = 取消; 点击添加（原地松手）不受影响。

## 8b. 右键菜单（对象级操作统一入口, 画进渲染帧）

右键命中组件 → 分型建菜单（`build_menu`, skia 绘制 + 命中同源 rect）:
- **项**: 上移/下移（兄弟段内换位 — 容器子项 = 排列序, 自由区 = z 序）/ 隐藏·显示
  （enabled 软隐藏, 大纲可寻回）/ 删除（右键目标在选中集内 = 整集删）。
- **容器**（core.layout.list）: 排列族（单列/两列/三列/自动换行, 当前项 ✓）。
菜单模态: 在场时点击全归菜单（执行/关闭）, 跨页点击关闭。动作经 apply_command
入单一撤销栈, `need_rebuild` 主循环收口整页重装配。

## 8c. 列表容器与流动数据（试驾场地基）

- **core.layout.list**: 块 = 排列策略载体（`layout::list_arrange`: Column/
  Columns(n)/Wrap{预算}/Grid + gap, line_height 单位 DPI 不变）; 引擎求解接管子树
  （measure 递归 → 锚定自身 → 排列子项, 嵌套容器递归; 容器不可见/空 → 塌缩传播,
  后代从渲染与包围盒剔除）。子项顺序 = 文档序 = 排列序。flight-info/power-info
  出厂页已容器化（contentVersion 3, 旧用户拷贝走升级提示链）; minihud 等自由
  拓扑页保持锚链。
- **运行时窗口跟随**: 普通页（dataface=Page, sizing=Auto）live 喂数后
  `converge_page_sizing` 包围盒收敛（字段条件隐藏 → 整块收缩可见）。
- **SimFrame**（`kernel::derived::sim_frame`）: preview 期 10Hz 合成流动数据喂
  通用页（速度缓变/油量缓降/告警周期量）; 场景拨杆 normal/jet/gear/nodata
  切换形态演出（无数据 = 条件塌缩形态）。

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
2. ~~撤销栈只覆盖目标页文档~~（2026-09-12 试驾场改造: 会话级全局单一栈, 全量编辑仓快照 —
   页面级操作/手势/菜单/拖放全部入栈, 已修）。
3. 崩溃丢编辑（无草稿恢复 — 会话内改动仅存内存, 悬浮条「完成」前崩溃即丢）。
4. ~~`lineHeightPx` 推送字段是占位~~（已随 hud-edit-doc 推送退役, 字段不复存在）。
5. resize 组件 size 为物理 px 覆盖, 不随字号缩放（备案: 尺寸语义后续统一）。
6. 容器子项的 pos/anchor 被排列策略忽略（顺序语义）— 旧编辑手势在容器子项上拖动
   无视觉位移（顺序换位走右键 上移/下移）; 自由区组件不受影响。

**本轮复查新发现的断链（⚠ 未修, 见第二部分 N 组核对项; 2026-09-12 原生化后口径更新）**:
- **N1 多选不可达**: `marquee_mode` 无开关（默认 false, SetOptions 退役后无入口）,
  `EditMouse` 不带修饰键 → 画布上既不能 Shift/Ctrl 加选也不能拖框;
  原生大纲 = 单击单选（无 Ctrl/Shift 多选）→ 多选整体不可达（含批量拖动/批量删除）。
- **N2 构建失败无提示**: EditSession 不跟踪重装配错误, 原生面无提示 UI
  （web 时代 Alert/红点随 webview 面板退役）→ 构建失败组件静默不渲染。

---

# 第二部分 · 验证场景清单

> 使用方式: Preview 态（未开始游戏）启动应用, 逐项勾选。标注 ⚠ 的是已知断链/边界, 预期值 = 当前实际行为。

## A. 会话进出与入口

- [ ] **A1 正常进入**: Preview 态 → footer 点「编辑HUD」→ 所有 HUD 窗口强制可见（含平时隐藏的 FM 拆包页）,
  原生侧栏（右缘满高）+ 悬浮条（上缘居中）出现, MainForm 隐退让屏（设置窗整体不可见）。
- [ ] **A2 游戏态拒绝**: 开始游戏（InGame/Connected）后托盘/其它路径尝试进入 → toast
  「请先结束游戏会话再编辑」, 不进入编辑态。
- [ ] **A3 footer 门控**: 编辑期 MainForm 隐退 → footer「保存/刷新预览/导入/开始」整体不可达
  （按钮 disabled 门控仍在代码层兜底, 悬停提示不可验）。
- **A4 窗口尺寸记忆**: 已随 web 控制台退役（编辑期 MainForm 隐退, 无编辑窗口尺寸可言）。
- [ ] **A5 托盘自动收尾**: 编辑期点托盘「开始」→ 编辑自动**提交**退出并进入游戏流程;
  点托盘主图标（Activate）→ 编辑自动提交退出。
- [ ] **A6 退出恢复**: 退出编辑后 MainForm 恢复显示常规设置面板, forced_open 的页收回,
  各页按激活探测恢复应有可见态, z 序恢复（overlay 重新恒顶）。

## B. 画布点选

- [ ] **B1 单选**: 点击某页窗口内组件 → 粉色 2px 选中框 + 8 个 6×6 手柄; 侧栏属性表单显示该组件属性。
- [ ] **B2 z 序**: 两组件重叠 → 点击重叠区选中**视觉顶层**（doc 序后者）。
- [ ] **B3 hover**: 无手势时鼠标掠过组件 → 1px 半透明粉框跟随, 离开消失。
- [ ] **B4 空白**: 点击组件外空白 → 清空选中; 按住空白拖动 = 移动**整窗**（非组件）。
- [ ] **B5 切目标页**: 点击另一可编辑页的窗口 → 装饰（选中框等）消失转移, 侧栏页面 tab 同步,
  该次点击不产生拖拽副作用。
- [ ] **B6 MiniHUD 排除**: 点击 MiniHUD 窗口 → 不切换目标页, 无装饰, 行为与普通窗口一致
  （拖动它 = 移动整窗）; 侧栏页面 tab 中无「MiniHUD」项。

## C. 拖动与吸附

- [ ] **C1 跟手拖动**: 按住组件拖 → 组件实时跟随（无明显滞后）, 窗口包围盒随内容外扩/收缩;
  松手后位置保持（属性表单无坐标行, 位置唯一改法 = 拖动）。
- [ ] **C2 网格吸附**: 拖动 → 位置按 0.1 行高格点跳动（靠近格点 ≤6px 时吸附; 吸附恒开,
  SetOptions 退役后无开关）。
- [ ] **C3 对齐参考线**: 拖动组件靠近另一组件边缘/中线 → 出现粉色虚线对齐线,
  松手后虚线消失（参考线恒开, 无开关）。
- **C4 关吸附**: 已随 SetOptions 退役（吸附/参考线恒开, 无关闭入口）。
- **C5 多选成组拖动**: 已不可达（多选整体不可达, 见 N1）。

## D. Resize

- [ ] **D1 八向**: 单选组件 → 拖 8 个手柄任一 → 对应边/角改变组件尺寸, 内容重排。
- [ ] **D2 最小钳制**: 向内压缩到极小 → 尺寸停在 ~8px 不再缩小。
- [ ] **D3 N/W 手柄**: 拖北/西侧手柄 → 组件起点（pos）跟随移动而非只改宽高。
- [ ] **D4 包围盒与节流**: resize 中窗口外框平滑跟随（约 30Hz）, 松手后终值精确落位;
  重启后 size 保持（size 写入 doc; 属性表单无 size 行）。
- [ ] **D5 清除 size**: 属性表单无 size 行 → 无直接清除入口, size 一旦设置即持久 ⚠
  （边界: 覆盖式尺寸, 无「恢复自适应」操作 — 如需验证恢复, 用恢复出厂页兜底）。

## E. 键盘（已随 web 控制台退役）

- **E1~E4**: 画布键盘命令（Nudge 微调/Delete 删除/Ctrl+Z 撤销/输入守卫）已随控制台退役
  （Nudge 变体删除, 渲染线程不消费编辑键盘）。撤销/重做唯一入口 = 悬浮条按钮;
  删除组件 = 右键菜单或属性区头「删除」; 属性表单 EditBox 打字由系统 EDIT 子控件自理
  （不触发画布命令恒真）。

## F. 组件增删改

- [ ] **F1 组件库添加**: 点击侧栏组件库目录条目（如「数据字段」）→ 当前页出现新组件
  （有合法默认值, 立即可见可拖）。
- **F2 常用字段预设**: 已随 web 控制台退役（fieldPresets 预设区不再存在; target = 属性表单手输）。
- [ ] **F3 props 即时生效**: 属性表单改 label 文本（Text 失焦提交）/ precision 数值（IntStepper）/
  Bool 开关（Toggle）/ Enum 选型（EnumCycle）→ 真窗对应变化无需退出; 无 Color 控件
  （颜色类由设置面板配置键驱动）。
- **F4 Target 目录**: 已随 web 控制台退役（无公式变量下拉; target = 属性表单「目标」行手输）。
- [ ] **F5 改名**: 属性表单「标识」行输入新 id, 失焦提交 → 画布/大纲/父组件引用同步;
  撞名 → 拒绝提交（预检在入撤销栈之前, 不留 no-op 快照）。
- [ ] **F6 删除**: 右键菜单「删除」或属性区头「删除」→ 删除父组件 → 子组件 parent 自动置空
  （不连带删除, 不悬空）; 组件级「复制」已随 web 控制台退役（页级复制见 H3）。
- [ ] **F7 enabled**: 大纲 enabled 方块关闭 → 画布上组件消失（不渲染）; 重新开启恢复;
  禁用期间在大纲仍可选中编辑属性。

## G. 大纲（侧栏）

- [ ] **G1 结构**: 页组件按 doc 序平铺, parent 链缩进展示（行文本 = 组件 id）, 行首 enabled 方块。
- **G2 多选**: 已不可达（原生大纲 = 单击单选, 无 Ctrl/Shift 多选; 见 N1）。
- [ ] **G3 联动**: 大纲选中 ↔ 画布选中框 二方同步, 属性表单跟随选中（任一处选择, 其余跟随）。

## H. 页面管理（会话内命令, 编辑期窗口即同步, 退出时统一落盘）

- [ ] **H1 切换**: 页面 tab 点击切换 → 装饰/大纲/属性表单全部切到新页。
- [ ] **H2 新建页**: tab 行尾「＋」→ 默认名空页成为目标（属性区「页名」行改名, 零弹窗）→
  从组件库添加组件 → 完成退出 → 桌面出现新窗口; 重启后仍在（delta user_pages）。
- [ ] **H3 复制页**: tab 右键「复制页」→ 副本含全部组件, 落地即切目标 → 完成退出 → 双窗并存。
- [ ] **H4 删除页**: tab 右键「删除页」（仅剩一页禁用; 无确认框 — 撤销栈兜底）→
  编辑期该窗口即从桌面消失, 目标页自动落到余下第一页。
- [ ] **H5 恢复出厂**: 编辑过的出厂页 → tab 右键「恢复出厂」（出厂页专属, 用户自建页无此项）→
  内容回出厂版（保留用户拖出的 pos）→ 完成退出 → 重启后确为出厂内容（delta owned 区该页已移除）。
- [ ] **H6 回滚性**: 「放弃」→ 全部页面管理改动回滚（新建页消失、删除页复活、恢复出厂撤销）,
  真窗回外部真相形态; 删除/新建页亦可用悬浮条撤销/重做逐级回退。

## I. 提交 / 丢弃

- [ ] **I1 保存并退出**: 悬浮条「完成 ✓」→ 会话退出, 全部改动（含页面管理）落盘;
  重启应用 → 改动全部在场（voidmei_config.json delta 核对: 出厂页编辑 → owned 区;
  用户页 → user 区; 位置 → page_positions）。
- [ ] **I2 放弃**: 「放弃」首击变「确认放弃?」（红, 3s 超时还原）, 再击才放弃 →
  全部改动回滚（含新建页消失、删除页复活）, MainForm 恢复显示, 真窗回外部真相形态。
- **I3 footer 保存编辑**: 已随 web 控制台退役（编辑期 MainForm 隐退, footer 不可达）。
- [ ] **I4 批量性**: 多页改动一次提交 → 只触发一次全量重建（无多次闪烁/级联刷新）。
- [ ] **I5 位置持久化**: 编辑会话内拖空白移动整窗 → 保存退出 → 重启 → 窗口位置保持
  （含用户页与出厂未提升页 — page_positions 轻量区, 不整页提升）。

## J. 互斥与保护

- [ ] **J1 写权接管**: 编辑期外部 CONFIG_CHANGED（如另一途径改配置）→ 编辑中页面实例不被替换,
  编辑仓内容保持（gauge/dpi 类参数照常生效）。
- [ ] **J2 失焦不隐藏**: 编辑期 Alt+Tab 切走焦点 → HUD 画布窗口不隐藏（FocusMonitor 被忽略）。
- [ ] **J3 策略表同步**: 编辑期改某页「开关键」（属性表单页面属性）→ 保存退出 →
  设置面板拨该开关 → 该页窗口正确开/关（激活策略随编辑更新）。
- [ ] **J4 MiniHUD 完整性**: 编辑会话进出后 MiniHUD 页渲染/行为无任何变化。

## K. 非 editing 的 WYSIWYG（常规链路回归）

- [ ] **K1 开关键即时刷新**: 设置面板拨某 overlay 组开关/显示项 → 对应真窗即时显隐/改样式（不重启）。
- [ ] **K2 字号/颜色**: 改页字号增量或全局五色 → 真窗重装配生效。
- [ ] **K3 激活语义**: thrust-chart 仅 jet+开关键开时激活; fm-list 热键显隐、起步隐藏正常。
- [ ] **K4 升级提示**: 用户 owned 页 contentVersion 低于出厂 → 打开编辑器出现升级提示。

## L. FM 特殊页编辑

- [ ] **L1 fm-list**: 编辑态它以静态预览行显示（zebra 列表整组件, 单选中可拖动/整组件属性表单
  显示「由设置面板配置驱动」的键标签）; 拖动保存后重启位置保持。
- [ ] **L2 thrust-chart**: 编辑态可见（fixed 900×500 + 贴屏底 dock）; ⚠ 它的固定几何在编辑态
  同样生效 — 拖动组件正常, 拖空白整窗被 dock 钳制属预期。
- [ ] **L3 语义保持**: 编辑 FM 页后保存退出 → 游戏态热键切换/段开关/show* 行归零收缩全部正常。

## M. 崩溃/异常边界

- [ ] **M1 崩溃丢编辑**: 编辑中强杀进程 → 重启无残留编辑（已知边界: 草稿不恢复, 无异常半提交态）。
- [ ] **M2 畸形手写 delta**: 手改 voidmei_config.json 塞非法组件 → 编辑器/加载不 panic
  （组件构建失败仅不渲染）⚠ N2: 无任何提示 UI — 核对「静默不渲染」即通过。
- **N 组 ⚠ 已知断链核对**（预期 = 当前实际行为, 修复后本组预期翻转）:
- [ ] **N1 多选不可达**: 真窗上 Shift/Ctrl 点击组件 → **不**加入选中（单击恒单选）; 拖框 → **无**框选
  （空白拖 = 移动整窗）。大纲 = 单击单选（无 Ctrl/Shift 多选）→ 多选整体不可达,
  批量拖动/批量删除/多选对齐均不可用。
- [ ] **N2 构建失败无提示**: 构建失败的组件 → **无**任何提示 UI（原生侧栏/悬浮条不显示错误
  — web 时代 Alert/红点随面板退役）, 仅静默不渲染。

## S. 试驾场（2026-09-12 新形态: 原生窗口群 + 容器 + 拖放 + 场景）

- [ ] **S1 窗口群 (原生)**: 进入编辑 → 原生侧边栏（右缘满高: 页面tab/组件库/大纲/属性表单）+
  悬浮条（上缘居中, skia 绘制与真窗同观感）出现; MainForm 隐退; 「完成/放弃」或退出 →
  两窗关闭, MainForm 恢复。侧栏无 webview 控件痕迹 (选中文本/滚动条均为自绘)。
- [ ] **S2 悬浮条动作**: 拖动一个组件 → 「↩撤销」亮起 → 点击回位, 「↪重做」亮起;
  「放弃」首击变「确认放弃?」(红, 3s 超时还原), 再击才放弃; 「完成 ✓」保存收场。
- [ ] **S3 容器排列**: 右键飞行信息块内任一字段 → 菜单（上移/下移/隐藏/删除）;
  右键块空白（字段间隙/边距）→ 排列菜单（单列/两列/三列/自动换行, 当前项 ✓）→
  切「两列」→ 整块当场重排为两列, 数据不断流。
- [ ] **S4 隐藏塌缩**: 字段菜单「隐藏」→ 该行消失, 后续行自动上移, 整窗高度收缩;
  大纲中该字段可寻回（enabled 开关重开）。
- [ ] **S5 拖放落件**: 原生侧栏组件库按下条目拖向飞行信息块 → 项间出现粉色插入线
  （上半个项前插/下半个后插）; 拖向空白 → 半透明幽灵; 释放 → 组件落地即选中;
  拖到桌面其它处释放 → 无事发生。点击（原地松手）→ 照旧添加到当前页。
- [ ] **S6 流动数据**: 编辑期字段值缓慢变化（表速/高度缓变, 油量缓降）;
  场景拨杆切「无数据」→ 条件字段当场塌缩; 切「起落架放下」→ gear 条件字段出现;
  切「喷气机」→ 螺旋桨族字段归零/隐藏。
- [ ] **S7 撤销统一**: S3 改排列 + S4 隐藏 + S5 拖件 后连续撤销三次 → 三步逆序全部回位
  （页面管理/菜单/手势/拖放同栈）。
- [ ] **S8 live 等价**: 完成保存 → 关闭编辑 → mock/真机进 live → 容器排列与隐藏状态
  在游戏态如实呈现; 窗口随字段条件收缩。
- [ ] **S9 属性表单常驻侧栏**: 选中组件 → 属性区表单（EditBox 失焦提交/步进/开关/枚举循环）;
  无选中 → 页面属性; 改名/精度/条件/单位即时生效且入撤销栈。
- [ ] **S10 页面管理**: tab「＋」新建默认名页 → 属性区改名零弹窗; 右键复制/删除（仅剩一页禁用）/
  恢复出厂（保位置）; 撤销/重做窗口随页开合（删页即摘窗, 撤销复活）。
- [ ] **S11 搜索与缩略图**: 组件库搜索框每键过滤 + 区头命中数（搜索空 = 全量）;
  条目左侧静态小样（120×52, 渲染失败兜底色块）。

---

## 附: 快速回归路径（最小集）

时间紧时至少跑: A1 → B1/B4/B5 → C1/C3 → D1 → F1/F3 → H2 → I1/I2 → J4 → K1 → S1/S3/S5/S6/S10/S11。
覆盖进出场、核心手势、属性即时性、页面增删、持久化与 MiniHUD 无损, 试驾场新形态主干
（含页面管理与组件库搜索/缩略图）。
