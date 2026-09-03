# VoidMei Rust 版

War Thunder 遥测 HUD overlay 应用：读取游戏本地 HTTP API (8111)，实时计算飞行数据，
在游戏画面上方绘制 HUD 悬浮窗（帧率/姿态/引擎/告警/飞行模型数据），并提供设置窗与
语音告警。本目录是 Rust workspace 全量实现（Java 版已退役）。

**新人路径**：先读下面的[架构地图](#架构地图)与[线程模型](#线程模型)两节建立全局观 →
按[分层导览](#分层导览)从 vm-core 开始浏览各 crate 的 lib.rs 头注（每个域/模块都有
`//!` 头注说明职责）→ 动手改代码前看[扩展指南](#扩展指南)里对应场景的改动路径。
坏味道清扫与历波重构的登记档案：`doc/rust坏味道登记与重构方案.md`；迁移期设计档案：
`build/migration/`（PORTING 宪法/CLASSIFY/LIFETIMES/DECISIONS/PROGRESS）。

## 架构地图

```
War Thunder HTTP API (127.0.0.1:8111)
        │  ~10Hz 轮询 (vm-data Service 线程, 单线程阻塞 HTTP)
        ▼
┌─ vm-data ───────────────────────────────────────────────┐
│ Service 轮询线程: State/Indicators 解析 → identify FM     │
│ → 公式求值 (vm-core formula) → ServiceData (RwLock 短锁) │
│ → 每周期发布 Arc<Frame> 进 FrameStore (不可变帧快照)      │
└──────────────┬───────────────────────────────────────────┘
               │ Frame (跨线程读面, 零锁 clone)
               ▼
┌─ vm-app (组装层) ─────────────────────────────────────────┐
│ 主线程: AppShell 监督循环 + vm-webui ShellForm (Tauri 泵)  │
│ 渲染线程: render_thread.rs (OverlayHost 单泵全部 overlay   │
│           窗口 + 托盘 + 热键消费, Rc<RefCell> 单线程共享)   │
│ Controller: 生命周期状态机 (INIT→CONNECTED→IN_GAME→PREVIEW)│
└───┬──────────────────────┬────────────────────────────────┘
    │ ReinitParams/UiCommand │ 规则触发/状态推送
    ▼                        ▼
┌─ vm-overlay ─────────┐  ┌─ vm-webui ──────────────────────┐
│ 五域: platform(窗口/  │  │ Tauri 2 web 设置壳 (常驻隐藏预热)│
│ 托盘/热键/host)/render│  │ IPC: command → mpsc → dispatcher │
│ (canvas/基元/字体/    │  │ → 主线程执行体 → oneshot 回执     │
│ 调色板)/overlays(~17  │  │ 前端 web/: React + AntD          │
│ 组件)/layout/ui_model│  └──────────────────────────────────┘
└───────────────────────┘
```

支撑层（被所有上层依赖）：**vm-core** 纯逻辑 11 域 + **vm-ui** 设置表单数据层。

| crate | 依赖 | 职责 |
|---|---|---|
| vm-core | — | 纯逻辑 11 域：base(总线/事件/日志/工具/JDK 语义复刻 java_compat/数值格式化 format)/config(配置栈)/telemetry(HTTP+解析)/fm(管理栈+数据+功率模型)/formula(公式系统)/derived(HUD 派生)/audio(语音告警)/ui_support(行定义/机型对比/颜色)/platform(焦点检测)/lang(i18n)/activation(激活) |
| vm-data | vm-core | 8111 轮询/派生量计算/Service 链；FrameStore 不可变帧 = 跨线程唯一读面 |
| vm-overlay | vm-core | 五域：platform(win/x11/host/tray/hotkey/reinit)/render(canvas/fields/renderers/font/palette/primitives)/overlays(组件, spec_common 工厂脚手架)/layout(布局引擎)/ui_model |
| vm-ui | vm-core | MainForm 数据层（main_form 状态机 + renderers 写回链；view 归 web 壳） |
| vm-webui | vm-core | Tauri 2 web 壳：IPC(dto/commands 三域) + web/ React/AntD 前端 |
| vm-app | 全部 | 组装 bin `voidmei`：AppShell/Controller/render_thread/主循环 |

依赖方向恒单向（core ← data/overlay/ui/webui ← app），无环。

## 线程模型

| 线程 | 职责 | 关键约束 |
|---|---|---|
| 主线程 | AppShell 监督循环 + ShellForm 事件泵（`App::run_iteration` 手动泵不阻塞） | `!Send` 的 AppShell 恒留主线程 |
| Service 轮询线程 (vm-data) | HTTP 轮询 → 解析 → 公式求值 → 发布 Frame | 顶层 catch_unwind 护航；锁纪律：临界区内不调回调/不做 IO（`with_snapshot`/`apply` 助手收口） |
| 渲染线程 (render_thread.rs) | OverlayHost 泵全部 overlay 窗口消息 + 脏检查渲染 + 托盘 + 热键消费 | 单线程拥有全部窗口（`&mut self` 整体 `!Send`）；`Rc<RefCell>` 句柄不跨线程 |
| FM-Loader / 一次性线程 | FM 文件加载 / 预览刷新等短命任务 | spawn 失败统一 error+降级，不 panic |

跨线程数据面三条通道：**Frame 帧快照**（读，零锁）、**UIStateBus/FlightDataBus**（事件
广播，嵌套 publish 有安全垫片）、**UiCommand mpsc**（主线程 → 渲染线程命令，唯一入口
`AppShell::send_ui`）。

## 分层导览

- **vm-core/src/lib.rs** → 各域 mod 头注。重点：`base/format.rs`（Java 数值格式化语义
  唯一真相）、`base/java_compat.rs`（JDK 语义族）、`formula/`（L0 registry/L1 编译/L2
  规则引擎；数据直通 State/Blkx → 公式 → overlay，无 getter 中转）、`fm/manager.rs`
  （identify/负缓存/FM_CHANGED 广播）。
- **vm-data/src/service_loop.rs** → 轮询主循环（编排层，各阶段已拆子函数）；
  `frame.rs` → Frame 快照结构（Engine/Fuel/Alt 三组标量 + 直通字段）。
- **vm-overlay/src/platform/host.rs** → OverlayHost（注册表/生命周期/脏检查渲染）；
  `overlays/spec_common.rs` → spec 工厂脚手架（FontSlot 字体热换）；
  `render/primitives.rs` → 像素基元唯一真相。
- **vm-app/src/lib.rs** → AppShell 装配；`controller.rs` → 生命周期状态机；
  `render_thread.rs` → 渲染线程装配与命令处理。
- **vm-webui/src/lib.rs** → 壳与 IPC 拓扑；`commands_{comparison,powercurve,formula}.rs`
  → 按域命令；`web/` → 前端源码。

## 扩展指南

**加一个 overlay**：① `vm-overlay/src/overlays/<name>.rs` 写组件 + `*_overlay_spec`
工厂（照抄任一现有组件，用 `spec_common::{FontSlot, keyed_spec}`）；②
`vm-app/src/keys.rs` 的 `OVERLAY_SECTIONS` 加键（overlay 键列单一来源，冒烟断言与注册
面均由它派生）；③ `vm-app/src/render_thread.rs` 的 `register_live_overlays` 加一段
`register_one(...)`；④ `ui_layout.cfg` 加开关项。

**加一个配置项**：`ui_layout.cfg` 加 `(item ...)` → vm-ui `renderer_config_helper` 的
`group_field_table!` 表加字段（如属组字段）→ 消费方读配置；WYSIWYG 预览刷新经
`with_interest` 键集。

**加/改一个派生量**：改 `formulas.cfg`（公式槽唯一真相，数据直通 State/Blkx）；
需要 C 级会话聚合量时先读 `vm-core/src/formula/registry.rs` 的 Session 通道。

**加一个 i18n 键**：`lang/cur.properties` 加键 → `vm-core/src/lang/mod.rs` 三点同步
（struct 字段/init_lang 赋值/table.rs 静态表；守则见该文件头注）。

## 常用命令

```bash
cd rust && cargo test --workspace     # workspace 全测试 (1256)
cargo build --release
bash script/rust_run.sh               # 完整应用 (设置窗 + overlay 预览)
bash script/rust_run.sh --live        # 直接 live 模式 (e2e 用)
bash script/rust_run.sh --mock-smoke  # mock 冒烟 (逐 overlay present 断言)
bash script/rust_e2e.sh               # e2e 三场景 (A1~A6 断言, 复用 e2e_assert.py)
python script/build.py rustdist       # 组装 Rust 分发包 → dist/VoidMei_Rust_*.zip
./target/debug/vm-ui --headless --persist <path>     # 固定序列落盘 (换框架 diff 基线)
```

## web 前端构建 (vm-webui/web)

```bash
python script/build.py web    # pnpm install + vite build → web/dist (cargo 编译期嵌入)
python script/build.py rust   # web + cargo release (一键)
```

- 首次需 Node + pnpm (`npm i -g pnpm`); esbuild 构建许可已在
  `web/pnpm-workspace.yaml` 的 `allowBuilds` 声明。
- **改动前端后必须重建 dist 再 cargo build** (`generate_context!` 编译期嵌入;
  本地开发快路径: `python script/build.py rust` 一键)。
- **voidmei.exe 必须与 `voidmei.exe.manifest` 同目录分发** (common-controls v6;
  vm-app 的 build.rs 构建时自动拷贝到 target/<profile>/, 源 =
  `rust/crates/vm-app/app.manifest`)。缺它进程加载期即 0xC0000139。

## 编译速度

改动 workspace 代码后的增量 release 构建 ~27s，三项配置：

1. `[profile.release] lto = "thin"`: 保留 95%+ 跨 crate 优化收益、只重链受影响分区;
2. `[profile.release] incremental = true`: release 默认关; thin LTO 下兼容;
3. `.cargo/config.toml` linker = `rust-lld`: tauri/wry 大符号表链接提速。

仍嫌慢可把 `target/` 加入 Windows Defender 排除列表 (管理员 PowerShell:
`Add-MpPreference -ExclusionPath <repo>\rust\target`)。

## 重构档案

- 坏味道清扫 (波12~18, 2026-09): `doc/rust坏味道登记与重构方案.md` —
  A 真缺陷/B 死代码/C 重复/D 长函数/E 结构/F 数据形态/G 文档/H 裁决保留 八类登记。
- 组织结构重构 (波7~11) 与六波架构重构 (FrameStore/UIStateBus/HTTP 重写等):
  见 git log `重构波` 系列提交。
- 迁移期设计档案: `build/migration/` (PORTING/CLASSIFY/LIFETIMES/DECISIONS/PROGRESS);
  迁移执行记录: `doc/overlay_java_to_rust_migration.md` §11。
