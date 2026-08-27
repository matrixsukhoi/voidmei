#!/usr/bin/env bash
# measure_reopen.sh — MainForm 启动→窗口可见延迟测量 (D9 阶段①验收)
# 用法: bash script/measure_reopen.sh [exe路径] [次数]
#   默认: rust/target/debug/voidmei.exe, 5 次
# 输出: 每次样本 + 均值/P95; 首样本 = 真冷启动
# 对照基线 (切换前): build/migration/baseline/reopen_iced.txt
set -euo pipefail
cd "$(dirname "$0")/.."
EXE="${1:-rust/target/debug/voidmei.exe}"
RUNS="${2:-5}"
powershell -NoProfile -ExecutionPolicy Bypass -File script/measure_reopen.ps1 -Exe "$EXE" -Runs "$RUNS"
