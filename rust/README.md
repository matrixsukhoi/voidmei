# VoidMei Rust 版 (全量迁移完成)

Java 版 (159 文件 Swing) → Rust 全量迁移产物。设计档案: `build/migration/`
(宪法 PORTING/全库地图 CLASSIFY/生命周期 LIFETIMES/决策 DECISIONS D1-D8/进度 PROGRESS);
迁移文档: `doc/overlay_java_to_rust_migration.md` (§11 全量执行记录)。

## Workspace 结构

| crate | 职责 |
|---|---|
| vm-core | 纯逻辑: 物理/parser/config 栈/fm 栈/总线/voice/logger (Java A+B 类) |
| vm-data | 8111 轮询/派生量/Service 链 (catch_unwind 护航) |
| vm-overlay | 平台窗口(ULW 多窗口)/渲染(render2d+tiny-skia)/全部 overlay 组件/托盘/热键 |
| vm-ui | MainForm (iced + tiny-skia 软件渲染, D1) |
| vm-app | AppShell/Controller 组装 bin (`voidmei`) |

## 常用命令

```bash
cd rust && cargo test --workspace     # 1239 测试
cargo build --release
bash script/rust_run.sh               # 完整应用 (设置窗 + overlay 预览)
bash script/rust_run.sh --game-mode   # 直接游戏模式 (e2e 用)
bash script/rust_run.sh --mock-smoke  # mock 冒烟 (逐 overlay present 断言)
bash script/rust_e2e.sh               # e2e 三场景 (A1~A6 断言, 复用 e2e_assert.py)
bash script/rust_compare.sh           # Java↔Rust 全链像素对拍 (preview/gauge/minihud)
./target/release/voidmei-overlay ...  # POC 工具 (render-png/compare/analyze/gauge)
```

## 验收状态 (2026-08-27)

- 1,239 测试 / e2e 三场景 PASS / 像素对拍无结构偏差 / 真窗共存验证
- 人工验收清单见迁移文档 §11.5
