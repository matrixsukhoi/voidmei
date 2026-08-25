# voidmei-overlay (Rust POC)

VoidMei `FlightInfoOverlay` 的 Rust 最小复现, 用于验证 overlay 技术栈
(平台原生纯 CPU 透明窗口 / 鼠标穿透 / 拖拽 / 低占用) 并建立与 Java 版的像素级比对方法论。
设计文档见 `doc/overlay_java_to_rust_migration.md`。

## 构建

```bash
cd rust
cargo build --release
cargo test            # 单元测试 (布局公式 / 格式化 / 派生量)
```

## 运行模式

```bash
./target/release/voidmei-overlay --preview     # 预览: 静态数据, 可拖拽 (位置存 rust/user_pos.json)
./target/release/voidmei-overlay --live        # 游戏: 轮询 8111, 穿透+置顶
./target/release/voidmei-overlay --log-values  # 从 8111 取一帧, 输出 getter=value (回灌 Java --values 对拍)
./target/release/voidmei-overlay --render-png <p> [--meta <m.json>]   # 离屏导出
./target/release/voidmei-overlay compare <a.png> <b.png> [--heatmap <d.png>]
./target/release/voidmei-overlay analyze <p.png> [行带高]   # 渲染分布调试
```

mock 端到端: `python script/mock_8111.py serve --port 8111 --scenario s2_preview_live` 后 `--live`。

## 与 Java 版像素对拍

```bash
# 1. Java 端离屏导出 (需桌面环境, 因 Toolkit 字体度量)
python script/build.py compile
java -classpath "bin;dep/*" ui.debug.OverlayPngExport \
     --out build/rust_ref/java_preview.png --meta build/rust_ref/java_meta.json

# 2. Rust 端离屏导出
cargo run --release -- --render-png build/rust_ref/rust_preview.png \
     --meta build/rust_ref/rust_meta.json

# 3. 比对 (meta 硬断言 + 差异热力图人工审)
cargo run --release -- compare build/rust_ref/java_preview.png \
     build/rust_ref/rust_preview.png --heatmap build/rust_ref/diff.png
```

一键脚本: `bash script/rust_compare.sh`

## 里程碑状态

- [x] M0 骨架 + 纯函数 (布局公式 / FastNumberFormatter 移植)
- [x] M1 离屏 PNG 对齐 (swash + ttf-parser 渲染; 布局/meta 硬断言 PASS, 字形光栅化差异 ~11% 像素, 根因=zeno vs FreeType hinting, 人工审)
- [x] M2 平台窗口层: Windows ULW 已验证 (透明/置顶/穿透/无任务栏/不抢焦点, 稳态 CPU 0%); X11 设计完成待接线
- [x] M3 mock_8111 数据端到端 (50ms 轮询/派生量/SMA/脏检查重绘) + compare/analyze 工具
- [x] M4 迁移文档 (doc/overlay_java_to_rust_migration.md)
