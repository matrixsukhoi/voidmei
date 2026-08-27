# VoidMei Rust 版 (全量迁移完成; MainForm 已换 Tauri web 壳 — D9)

Java 版 (159 文件 Swing) → Rust 全量迁移产物。设计档案: `build/migration/`
(宪法 PORTING/全库地图 CLASSIFY/生命周期 LIFETIMES/决策 DECISIONS D1-D9/进度 PROGRESS);
迁移文档: `doc/overlay_java_to_rust_migration.md` (§11 全量执行记录)。

## Workspace 结构

| crate | 职责 |
|---|---|
| vm-core | 纯逻辑: 物理/parser/config 栈/fm 栈/总线/voice/logger (Java A+B 类) |
| vm-data | 8111 轮询/派生量/Service 链 (catch_unwind 护航) |
| vm-overlay | 平台窗口(ULW 多窗口)/渲染(render2d+tiny-skia)/全部 overlay 组件/托盘/热键 |
| vm-ui | MainForm **数据层** (main_form 状态机 + renderers 写回链; D9 起 view 归 web 壳) |
| vm-webui | MainForm **Tauri 2 web 壳** (D9): 常驻隐藏预热窗口 + IPC(dto/commands) + `web/` React/AntD 前端 |
| vm-app | AppShell/Controller 组装 bin (`voidmei`; 单循环主线程 = shell.pump + tauri run_iteration) |

## 常用命令

```bash
cd rust && cargo test --workspace     # workspace 全测试
cargo build --release
bash script/rust_run.sh               # 完整应用 (设置窗 + overlay 预览)
bash script/rust_run.sh --game-mode   # 直接游戏模式 (e2e 用)
bash script/rust_run.sh --mock-smoke  # mock 冒烟 (逐 overlay present 断言)
bash script/rust_e2e.sh               # e2e 三场景 (A1~A6 断言, 复用 e2e_assert.py)
bash script/rust_compare.sh           # Java↔Rust 全链像素对拍 (preview/gauge/minihud)
./target/release/voidmei-overlay ...  # POC 工具 (render-png/compare/analyze/gauge)
./target/debug/vm-webui-selftest --bench-reopen 10   # D9: 预热重开延迟
bash script/measure_reopen.sh         # D9: 进程启动→窗口可见延迟 (外部观测)
./target/debug/vm-ui --headless --persist <path>     # 固定序列落盘 (换框架 diff 基线)
```

## D9 web 前端构建 (vm-webui/web)

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
  `rust/crates/vm-app/app.manifest`)。缺它进程加载期即 0xC0000139 (D9 坑位 1)。

## 验收状态

- 2026-08-27: 1,239 测试 / e2e 三场景 PASS / 像素对拍无结构偏差 / 真窗共存验证
- 2026-08-28 (D9 阶段①): tauri 壳五项 POC PASS — 预热重开 12-18ms (目标<300ms)、
  show/hide×500 长跑、同进程共存 (mock-smoke 7 overlay × 116 帧)、CJK、干净退出;
  进程冷启动→窗口可见 ~1265ms (iced 基线 284ms, WebView2 初始化一次性成本)
