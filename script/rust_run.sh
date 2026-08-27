#!/usr/bin/env bash
# Rust 版本地运行入口 (对齐 script/build.py run 的语义: 构建缺失自动编译 + 仓库根工作区运行)。
# 用法: script/rust_run.sh [--live | --mock-smoke | --debug]
#   --live        对齐 autoStartGameMode=true: 跳过设置窗直接 live 模式 (e2e)
#   --mock-smoke  起 mock_8111 s2 场景 → live 模式 8 秒 → 断言收数/present 帧 → 退出
# 工作区 = 仓库根 (lang/ fonts/ data/ ui_layout.cfg 均按项目根解析, 同 java -jar)
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)

# 构建 (增量: 产物新鲜时秒过, 即 build.py 的 "bin/ 缺失时自动编译" 等价)
cd "$ROOT/rust"
cargo build --release --bin voidmei

# 运行 (CWD=仓库根; 参数原样透传)
cd "$ROOT"
exec ./rust/target/release/voidmei "$@"
