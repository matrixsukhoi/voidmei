# *** 使用中文思考 ***
# 代码里的注释要简洁精炼
# 现在的测试很糟糕, 以后要进行重构. 不要跑e2e测试, 不要跑冒烟测试, 不要补充或新增更多测试了
# 不用担心兼容性问题, 可以随便改架构. 我也建议你在做特性时更多考虑架构方面的重构, 以及各种微重构.
# 引入现代化组件和依赖是件好事
# 写代码时, 关键的地方和问题修复一定要添加和补充中文注释

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VoidMei 是 War Thunder 的遥测 HUD overlay 应用(Rust 实现): 读取游戏本地 HTTP API(8111)实时计算飞行数据, 在游戏画面上方绘制 HUD 悬浮窗(帧率/姿态/引擎/告警/飞行模型数据), 并提供设置窗(Tauri web 壳)与语音告警。

**Rust workspace 六 crate**(cargo workspace,仓库根即 workspace 根),约 289 个 .rs 文件 / 87k 行。原 Java 版已于 2026-09 全量迁移后退役删除。

## 构建与运行

```bash
cargo test --workspace            # 全部单元测试 (~1256, 仓库根直接跑)
cargo build --release             # release 构建 → target/release/voidmei.exe

python script/build.py rust       # web 前端 + cargo release 一键构建链
python script/build.py web        # 仅 web 前端 (pnpm → crates/webui/web/dist)
python script/build.py rustdist   # 组装分发包 → dist/VoidMei_Rust_*.zip (解压即用)
python script/build.py fmdata     # 游戏版本更新后: 从客户端解包 JSON 版 FM 数据
python script/build.py clean      # 清 build/ dist/ (cargo 产物用 cargo clean)

bash script/rust_run.sh           # 完整应用 (设置窗 + overlay 预览)
bash script/rust_run.sh --live    # 直接 live 模式 (e2e 用)
bash script/rust_e2e.sh           # e2e 三场景 (A1~A6 断言; 一般不跑, 见头部指令)
```

**关键约束**:
- **web/dist 未入库且被 cargo 编译期嵌入**(`generate_context!`)— 任何 cargo 编译前必须先 `build.py web`(CI 已内置此步;本仓库已带 dist 时可跳过)。
- **voidmei.exe 必须与 voidmei.exe.manifest 同目录分发**(common-controls v6;缺它启动即 0xC0000139;build.rs 自动拷到 target/<profile>/)。
- **运行时 CWD = 仓库根**: 程序按 `./fonts ./data ./lang` 解析资源(rust_run.sh 已处理)。
- 版本号由 `VOIDMEI_VERSION` 环境变量注入(CI 从 git tag 提取,本地缺省 dev)。

## 架构

### 架构地图

```
War Thunder HTTP API (127.0.0.1:8111)
        │  ~10Hz 轮询 (data crate Service 线程, 单线程阻塞 HTTP/ureq)
        ▼
┌─ data ─────────────────────────────────────────────────┐
│ Service 轮询线程: State/Indicators 解析 → identify FM   │
│ → 公式求值 (kernel formula) → 每周期发布 Arc<Frame>      │
│ 进 FrameStore (不可变帧快照)                            │
└──────────────┬─────────────────────────────────────────┘
               │ Frame (跨线程读面, 零锁 clone)
               ▼
┌─ voidmei (组装 bin) ────────────────────────────────────┐
│ 主线程: AppShell 监督循环 + webui ShellForm (Tauri 泵)   │
│ 渲染线程: render_thread.rs (OverlayHost 单泵全部 overlay │
│           窗口 + 托盘 + 热键消费, Rc<RefCell> 单线程共享) │
│ Controller: 生命周期状态机 (Init→Connected→InGame→Preview)│
└───┬──────────────────────┬─────────────────────────────┘
    │ ReinitParams/UiCommand │ 规则触发/状态推送
    ▼                        ▼
┌─ overlay ────────────┐  ┌─ webui ────────────────────────┐
│ 六域: platform(窗口/  │  │ Tauri 2 web 设置壳 (常驻隐藏预热)│
│ 托盘/热键/host)/render│  │ IPC: command → mpsc → dispatcher│
│ (canvas/基元/字体/    │  │ → 主线程执行体 → oneshot 回执    │
│ 调色板)/widgets/      │  │ 前端 web/: React 18 + AntD 5    │
│ overlays/layout/     │  └────────────────────────────────┘
│ ui_model             │
└──────────────────────┘
```

| crate | 依赖 | 职责 |
|---|---|---|
| kernel | — | 纯逻辑 11 域: base(总线/事件/日志/java_compat 语义复刻/数值格式化)/config(配置栈)/game_api(8111 客户端 ureq+serde)/fm(管理栈+数据+功率模型)/formula(公式系统)/derived(HUD 派生)/audio(语音告警)/ui_support(行定义/机型对比/颜色)/platform(焦点检测)/lang(i18n)/activation(激活) |
| data | kernel | 8111 轮询/派生量计算/Service 链;FrameStore 不可变帧 = 跨线程唯一读面 |
| overlay | kernel | 六域(见上图);**widgets = 组件注册表+页面编排器+sidecar(W1~W4 组件化)** |
| ui | kernel | MainForm 表单数据层(main_form 状态机 + renderers 写回链;view 归 web 壳) |
| webui | kernel | Tauri 2 web 壳: IPC(dto/commands 按域分文件) + web/ React 前端 |
| voidmei | 全部 | 组装 bin `voidmei`: AppShell/Controller/render_thread/主循环 |

依赖方向恒单向(kernel ← data/overlay/ui/webui ← voidmei),无环。

### 线程模型

| 线程 | 职责 | 关键约束 |
|---|---|---|
| 主线程 | AppShell 监督循环 + ShellForm 事件泵(`App::run_iteration` 手动泵不阻塞) | `!Send` 的 AppShell 恒留主线程 |
| Service 轮询线程 (data) | HTTP 轮询 → 解析 → 公式求值 → 发布 Frame | 顶层 catch_unwind 护航;锁纪律:临界区内不调回调/不做 IO(`with_snapshot`/`apply` 助手收口) |
| 渲染线程 (render_thread.rs) | OverlayHost 泵全部 overlay 窗口消息 + 脏检查渲染 + 托盘 + 热键消费 | 单线程拥有全部窗口(`&mut self` 整体 `!Send`);`Rc<RefCell>` 句柄不跨线程 |
| FM-Loader / 一次性线程 | FM 文件加载 / 预览刷新等短命任务 | spawn 失败统一 error+降级,不 panic |

跨线程数据面三条通道: **Frame 帧快照**(读,零锁)、**UIStateBus/FlightDataBus**(事件广播,嵌套 publish 有安全垫片)、**UiCommand mpsc**(主线程 → 渲染线程命令,唯一入口 `AppShell::send_ui`)。

### Overlay 组件化 (W1~W5, 已完成)

- 配置 = 出厂 `crates/kernel/src/config/factory_default.json`(编译期内嵌)⊕ 用户 `voidmei_config.json`(delta,升级跟随语义)。
- overlay = **HUD 页面**(PageDoc,画布即窗口)— 9 个出厂页全部数据驱动(`overlay::widgets` 域: HudWidget trait + WidgetMeta 注册表 + PageOverlay 通用编排器 + WidgetSidecar FM 黑盒数据面)。
- 用户可在 MainForm「HUD 布局」tab 拖拽组装自己的 HUD 页面(palette/画布/inspector 三栏,编辑会话 = MainForm 整体形态切换)。
- 新增组件 → widgets 域注册表一处注册。

### 分层导览(关键文件)

- `crates/kernel/src/lib.rs` → 各域 mod 头注。重点: `base/format.rs`(数值格式化语义唯一真相)、`base/java_compat.rs`(JDK 语义族)、`formula/`(L0 registry/L1 编译/L2 规则引擎;数据直通 State/Blkx → 公式 → overlay,无 getter 中转)、`fm/manager.rs`(identify/负缓存/FM_CHANGED 广播)。
- `crates/data/src/service_loop.rs` → 轮询主循环;`frame.rs` → Frame 快照结构。
- `crates/overlay/src/platform/host.rs` → OverlayHost(注册表/生命周期/脏检查渲染);`render/primitives.rs` → 像素基元唯一真相。
- `crates/voidmei/src/lib.rs` → AppShell 装配;`controller.rs` → 生命周期状态机;`render_thread.rs` → 渲染线程。
- `crates/webui/src/ipc.rs` → 壳与 IPC 拓扑;`commands_*.rs` → 按域命令;`web/src/` → 前端源码。

### 扩展指南

- **加一个 overlay 组件**: `crates/overlay/src/widgets/` 写组件 + WidgetMeta 注册表注册 → 出厂页 JSON 或用户页引用。
- **加一个配置项**: `crates/kernel/src/config/factory_default.json` 加节点 → 消费方读配置(WYSIWYG 预览刷新经 interest 键集)。
- **加/改一个派生量**: 改根目录 `formulas.cfg`(公式槽唯一真相);需要 C 级会话聚合量时先读 `crates/kernel/src/formula/registry.rs` 的 Session 通道。
- **加一个 i18n 键**: `lang/cur.properties` 加键 → `crates/kernel/src/lang/mod.rs` 三点同步(struct 字段/init_lang 赋值/table.rs 静态表;table.rs 是 cur.properties 的静态快照,有防漂移测试)。

## 历史决策要点(为什么是现在这样)

- **GUI 框架**: 迁移期评估过 iced(POC 基线)/egui/GPUI,最终 D9 切换为 **Tauri 2 + React web 壳**(MainForm 设置窗)+ 自绘 overlay(tiny-skia/swash 直绘 Win32 窗口)。理由: 表单类 UI 用 web 生态效率最高;overlay 高频渲染不进 webview。
- **FrameStore 不可变帧快照**: 替代 Java 时代的锁+getter 快照链,跨线程读面零锁。
- **UIStateBus 统一路由总线**: 嵌套 publish 安全,替代 Java 的多总线散布。
- **单线程阻塞 HTTP(ureq)**: 10Hz 低频轮询无需异步运行时,砍掉 tokio 依赖面(仅 webui IPC 用 tokio oneshot)。
- **cargo workspace 单仓六 crate**: kernel 纯逻辑无依赖 → 各层单向依赖,编译并行度好。

## 目录职责(单一来源架构)

| 路径 | 角色 |
|------|------|
| 仓库根 | 唯一源 + 本地运行工作区(git 跟踪源码/资源 + gitignore 的本地 data/fonts 未入库字体/运行时生成物) |
| `dist/` | 构建产物(gitignore) |
| GitHub `v*` Release | 唯一分发渠道(CI 自动构建) |
| GitHub `data` prerelease | fmdata 云端存储层(CI 组包用,`--prerelease` 保证不进 `/releases/latest`,`checkUpdate()` 永远看不到) |

**资源管理(按"丢了怎么恢复"分类)**: 源码+自有资产(image/lang/voice 已入库部分/fonts 两个开源字体)进 git;`data/` 是派生数据不进 git(build.py fmdata 从游戏客户端再生成);运行时数据(records/ config/ voidmei_config.json user_pos.json)gitignore;`fonts/DIN Pro 400.otf` 为商业字体,gitignore 排除、不分发(程序容错回退)。

## Release(发版流程)

**版本号单一来源 = git tag**(规范 `1.590`,纯数字三段、**无 v 前缀**;fmdata 更新版也占正常版本号,如 `1.591`,更新日志注明 WT 数据版本。**不要用四段号** `1.590.1` — `checkUpdate()` 的正则会截断成 `1.590`,用户收不到更新提示。更新日志.txt 里的版本行**带 v**(`v1.590`,面向用户的显示格式),与 tag 无 v 是两回事,`release_notes.py` 内部做转换)。

**更新记录唯一来源 = `更新日志.txt`**(人手写,git 跟踪;CHANGELOG.md 已废除)。CI 对它只读不改。

**日常发版(全自动,代码内容由 tag 锁定):**
1. 发版前直接在 `更新日志.txt` 顶部(TODO 注释块之后)插入新版本块:`____分隔线 / v1.590 / 一行一条改动` — **只写用户可感知的改动,不写工程实现细节**
2. `git commit`,然后 `git tag 1.590 && git push origin master 1.590` ← 触发 CI
3. CI(release.yml): checkout tag 的 commit → 从 `data` prerelease 下载 `VoidMei_RustData*` zip → `build.py rustdist`(web+cargo 构建+组包)→ 从 `更新日志.txt` 提取该版本条目作 Release body → 创建 **draft** Release
4. 测试同学验证 draft 附件 → 人工点 "Publish release" 转正

**游戏版本更新后(fmdata 更新,纯运维不触发发版):**
```bash
python script/build.py fmdata    # 游戏目录自动探测 (或 WT_GAME_DIR=... 显式指定)
gh release upload data dist/VoidMei_RustData_*.zip dist/rust_data_manifest.json --clobber
# 然后在已测试的 commit 上更新 更新日志.txt 并打新 tag, 由人拍板
```

**灰度测试(不影响用户)**: push 正式 tag → CI 以 draft 创建 Release(公众不可见)→ 测试同学(需协作者权限)下载 draft 附件验证 → 通过后 "Publish release" 转正。不通过即删 draft + 删 tag 重来。**版本号不使用 `-rc`/`-test` 后缀** — 发布状态由 Release 的 draft/published 状态表达。

**纯构建核验**: Actions 页手动触发 `release` workflow 并填写 version + 勾选 dry-run → 只构建产出 artifact(不创建 Release、不改远端状态)。

**原则**: 发版永远是显式动作(人工 Publish draft;tag 触发的只是构建);data 上传不触发任何 workflow;旧版本 Release 一经发布不再改动。

## 文档索引

| 文档 | 内容 |
|------|------|
| `doc/formula_system_design.md` | 公式系统设计(L0/L1/L2, formulas.cfg) |
| `doc/hud编辑器全景与验证手册.md` | HUD 真窗编辑器全景 + 人工验收清单 |
| `doc/rust坏味道登记与重构方案.md` | 坏味道清扫与历波重构登记(活跃台账) |
| `doc/打桩调试手册.md` | mock_8111 打桩调试方法 |
| `更新日志.txt` | 面向用户的发版记录(唯一来源) |
