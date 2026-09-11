#!/usr/bin/env bash
# 本地运行入口: 增量构建 + 仓库根工作区运行。
# 用法: script/rust_run.sh [--live | --debug]
#   --live        对齐 autoStartGameMode=true: 跳过设置窗直接 live 模式
#   (--port <p> 打桩调试, mock_8111.py 起桩见 doc/打桩调试手册.md)
# 工作区 = 仓库根 (lang/ fonts/ data/ 均按项目根解析)
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)

# 构建 (增量: 产物新鲜时秒过)
cd "$ROOT"
cargo build --release --bin voidmei

# 运行 (CWD=仓库根; 参数原样透传)
cd "$ROOT"
exec ./target/release/voidmei "$@"
