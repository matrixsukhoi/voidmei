# VoidMei Rust 版 (全量迁移完成; MainForm 已换 Tauri web 壳 — D9)

Java 版 (159 文件 Swing) → Rust 全量迁移产物。设计档案: `build/migration/`
(宪法 PORTING/全库地图 CLASSIFY/生命周期 LIFETIMES/决策 DECISIONS D1-D9/进度 PROGRESS);
迁移文档: `doc/overlay_java_to_rust_migration.md` (§11 全量执行记录)。

## Workspace 结构

波7~11 组织结构重构 (2026-09): vm-core 11 域 + 根 shim 退役 (全库唯一路径
`vm_core::<域>::<模块>`), vm-overlay 五域, vm-app lib 化; 引用路径全统一, 无旧扁平别名。

| crate | 职责 |
|---|---|
| vm-core | 纯逻辑, 11 域: base(总线家族/事件/日志/工具/物理/大气)/config(配置栈)/telemetry(HTTP+parser)/fm(管理栈+data+功率族)/formula(公式系统)/derived(HUD 派生)/audio(语音)/ui_support(行定义+机型对比)/platform(平台检测)/lang(i18n)/activation(激活) |
| vm-data | 8111 轮询/派生量/Service 链 (catch_unwind 护航); FrameStore 不可变帧 = 跨线程唯一读面 |
| vm-overlay | 五域: platform(win/x11/host/tray/hotkey/position/reinit)/render(canvas/fields/renderers/font/palette/primitives)/overlays(~17 组件, fields_tests 域级测试)/layout(布局引擎+常量)/ui_model |
| vm-ui | MainForm **数据层** (main_form 状态机 + renderers 写回链; D9 起 view 归 web 壳) |
| vm-webui | MainForm **Tauri 2 web 壳** (D9): 常驻隐藏预热窗口 + IPC(dto/commands) + `web/` React/AntD 前端 |
| vm-app | AppShell/Controller 组装 bin (`voidmei`; 单循环主线程 = shell.pump + tauri run_iteration); lib 标准入口 + form_dispatch + tests/ 四主题分片 |

## 常用命令

```bash
cd rust && cargo test --workspace     # workspace 全测试
cargo build --release
bash script/rust_run.sh               # 完整应用 (设置窗 + overlay 预览)
bash script/rust_run.sh --live        # 直接 live 模式 (e2e 用)
bash script/rust_run.sh --mock-smoke  # mock 冒烟 (逐 overlay present 断言)
bash script/rust_e2e.sh               # e2e 三场景 (A1~A6 断言, 复用 e2e_assert.py)
python script/build.py rustdist       # 组装 Rust 分发包 → dist/VoidMei_Rust_*.zip (解压即用)
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

## 编译速度 (2026-08-28 优化)

改动 workspace 代码后的增量 release 构建 ~27s (原 1m43s, 提速 4 倍), 三项配置:

1. `[profile.release] lto = "thin"` (原 `true`/fat): fat 每次增量全图重优化+重链,
   thin 保留 95%+ 跨 crate 优化收益、只重链受影响分区 — 最大单项;
2. `[profile.release] incremental = true`: release 默认关; thin LTO 下兼容
   (fat 才与增量互斥), 首跑建缓存 48s、次跑起 27s;
3. `.cargo/config.toml` linker = `rust-lld` (工具链自带, 替代 mingw ld.bfd):
   tauri/wry 大符号表链接提速 (改 vm-app 一层 24s, 原 43s)。

产物已验证: mock-smoke PASS (7 overlay × 138 present 帧) + workspace 1290 测试全绿。
性能敏感度: overlay 为低频 GDI 渲染, thin↔fat LTO 差异不可观。
仍嫌慢可再把 `target/` 加入 Windows Defender 排除列表 (管理员 PowerShell:
`Add-MpPreference -ExclusionPath <repo>\rust\target`)。

## 验收状态

- 2026-09-03 (波7~11 组织结构重构): 清扫卫生 (workspace 化/游离产物出库/global_colors
  测试竞态根治) / vm-core 11 域定稿+根留清零+formula↔derived 循环拆解 (flap_limits 归 fm) /
  根 shim 退役+全库 ~490 处引用统一域路径 / vm-overlay 45 平铺→五域+field 壳退役 /
  vm-app lib 化+tests 四分片+configuration_service 三分+md5 抽出 —
  五提交, 1,292 测试全绿, clippy 无新增
- 2026-09-02 (六波架构重构): UIStateBus 死锁根治+统一路由 / vm-core 九域分组+UI 面下沉 /
  overlay 基元收敛+host 摘锁 / Frame 帧快照 (跨线程读者零锁) / HTTP 单线程阻塞重写 /
  Lang 缓存 — 六提交 92e2e63..3a579bf, 1,292 测试全绿, clippy 无新增
- 2026-08-27: 1,239 测试 / e2e 三场景 PASS / 像素对拍无结构偏差 / 真窗共存验证
- 2026-08-28 (D9 阶段①): tauri 壳五项 POC PASS — 预热重开 12-18ms (目标<300ms)、
  show/hide×500 长跑、同进程共存 (mock-smoke 7 overlay × 116 帧)、CJK、干净退出;
  进程冷启动→窗口可见 ~1265ms (iced 基线 284ms, WebView2 初始化一次性成本)
