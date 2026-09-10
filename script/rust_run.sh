#!/usr/bin/env bash
# 本地运行入口: 增量构建 + 仓库根工作区运行。
# 用法: script/rust_run.sh [--live | --mock-smoke | --debug]
#   --live        对齐 autoStartGameMode=true: 跳过设置窗直接 live 模式 (e2e)
#   --mock-smoke  起 mock_8111 s2 场景 → live 模式 8 秒 → 断言收数/present 帧 → 退出
# 工作区 = 仓库根 (lang/ fonts/ data/ 均按项目根解析)
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)

# 构建 (增量: 产物新鲜时秒过)
cd "$ROOT"
cargo build --release --bin voidmei

# 运行 (CWD=仓库根; 参数原样透传)
cd "$ROOT"
exec ./target/release/voidmei "$@"
