#!/usr/bin/env bash
# -*- coding: utf-8 -*-
#
# VoidMei FM 端到端编排脚本 (Git Bash / Windows 可跑)
#
# 流程: 起 mock(指定场景) → 起应用(日志重定向) → 跑指定时长 → 停应用 →
#       停 mock(/_mock/shutdown) → e2e_assert.py 断言 → 汇总退出码
#
# 用法:
#   ./script/e2e_fm.sh --scenario s5_missing_fm --duration 60
#   ./script/e2e_fm.sh --scenario s3_disconnect --duration 90 --port 8111
#
# 参数:
#   --scenario <name>  mock 场景 (默认 s5_missing_fm, 见 script/mock_scenarios/scenarios/)
#   --duration  <s>    运行秒数 (默认 60)
#   --port      <p>    mock 端口 (默认 8111 = VoidMei 轮询端口; 测试他用 --port)
#   --log       <path> 应用日志输出 (默认 build/e2e_<scenario>_<时间戳>.log)
#   --manual-app       不自动起应用, 等待用户手动启动 (bin/ 缺失/编译冲突时用)
#
# 退出码: 0=断言全过  1=断言失败/流程失败
#
# 注意: 若 bin/ 不存在, 本脚本不会自动编译 (python script/build.py run 会触发编译,
#       可能与并行工作冲突), 改为提示手动启动。
#
# 游戏模式强制: e2e 的意义在于跑起 Service 10Hz 轮询线程 (issue #55 的回归面)。
# 预览模式 (autoStartGameMode=false, 配置默认值) 下应用只做一次 FM-Detect 探测,
# 没有持续轮询 —— 断言会"空转通过"。因此起应用前临时把 ui_layout.user.cfg 的
# autoStartGameMode 翻成 true (备份/退出时还原), 保证进入游戏模式。

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCENARIO="s5_missing_fm"
DURATION=60
PORT=8111
LOG=""
MANUAL_APP=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario)  SCENARIO="$2"; shift 2 ;;
    --duration)  DURATION="$2"; shift 2 ;;
    --port)      PORT="$2"; shift 2 ;;
    --log)       LOG="$2"; shift 2 ;;
    --manual-app) MANUAL_APP=1; shift ;;
    *) echo "未知参数: $1"; exit 1 ;;
  esac
done

mkdir -p "$ROOT/build"
if [[ -z "$LOG" ]]; then
  LOG="$ROOT/build/e2e_${SCENARIO}_$(date +%Y%m%d_%H%M%S).log"
fi
MOCK_LOG="$ROOT/build/e2e_mock_$(date +%Y%m%d_%H%M%S).log"

APP_PID=""
MOCK_PID=""

cleanup() {
  # 优雅收尾: 先停应用再停 mock (顺序与正向流程一致, 失败也要尽量执行)
  stop_app
  stop_mock
  restore_user_cfg
}
trap cleanup EXIT INT TERM

# ---- ui_layout.user.cfg 备份/还原 (autoStartGameMode 临时翻 true) ----
USER_CFG="$ROOT/ui_layout.user.cfg"
CFG_BACKUP=""
restore_user_cfg() {
  if [[ -n "$CFG_BACKUP" && -f "$CFG_BACKUP" ]]; then
    cp -f "$CFG_BACKUP" "$USER_CFG"
    rm -f "$CFG_BACKUP"
    echo "[e2e] 已还原 ui_layout.user.cfg (autoStartGameMode)"
  fi
}
force_game_mode() {
  if [[ ! -f "$USER_CFG" ]]; then
    echo "[e2e] 警告: ui_layout.user.cfg 不存在, 无法强制游戏模式 (应用可能停在预览模式不轮询)"
    return
  fi
  cp -f "$USER_CFG" "$USER_CFG.e2e_bak"
  CFG_BACKUP="$USER_CFG.e2e_bak"
  sed -i 's/:target "autoStartGameMode" :value false/:target "autoStartGameMode" :value true/' "$USER_CFG"
  if grep -q ':target "autoStartGameMode" :value true' "$USER_CFG"; then
    echo "[e2e] 已临时启用 autoStartGameMode (退出时还原)"
  else
    echo "[e2e] 警告: 未能翻转 autoStartGameMode (user cfg 中无该项?), 应用可能停在预览模式"
  fi
}

stop_app() {
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    echo "[e2e] 停止应用 (PID $APP_PID, 杀进程树) ..."
    if command -v taskkill >/dev/null 2>&1; then
      # Windows: //T 杀整棵进程树 (python -> java); //F 强杀 (GUI 应用不响应 WM_CLOSE)
      taskkill //F //T //PID "$APP_PID" >/dev/null 2>&1 || kill "$APP_PID" 2>/dev/null
    else
      kill "$APP_PID" 2>/dev/null
    fi
  else
    echo "[e2e] 提示: 若应用为手动启动, 请自行关闭 (taskkill //F //IM java.exe)"
  fi
}

stop_mock() {
  if [[ -n "$MOCK_PID" ]] && kill -0 "$MOCK_PID" 2>/dev/null; then
    echo "[e2e] 停止 mock (/_mock/shutdown) ..."
    curl -s --noproxy '*' --max-time 5 "http://127.0.0.1:${PORT}/_mock/shutdown" >/dev/null 2>&1
    # 给 shutdown 一点时间, 兜底强杀
    for _ in 1 2 3 4 5; do
      kill -0 "$MOCK_PID" 2>/dev/null || break
      sleep 1
    done
    kill "$MOCK_PID" 2>/dev/null
  fi
}

# ---------------------------------------------------------------- 1. 起 mock
echo "[e2e] 启动 mock (场景=$SCENARIO 端口=$PORT) ..."
python "$ROOT/script/mock_8111.py" serve --port "$PORT" --scenario "$SCENARIO" >"$MOCK_LOG" 2>&1 &
MOCK_PID=$!

READY=0
for i in $(seq 1 15); do
  if curl -s --noproxy '*' --max-time 2 "http://127.0.0.1:${PORT}/_mock/state" >/dev/null 2>&1; then
    READY=1; break
  fi
  sleep 1
done
if [[ "$READY" != "1" ]]; then
  echo "[e2e] 错误: mock 15s 内未就绪 (可能场景名错误或端口被占), mock 日志:"
  cat "$MOCK_LOG"
  exit 1
fi
echo "[e2e] mock 就绪 (日志: $MOCK_LOG)"

# ---------------------------------------------------------------- 2. 起应用
# 先强制游戏模式 (备份 user cfg, cleanup 还原) —— 见文件头注释
force_game_mode

if [[ "$MANUAL_APP" == "1" || ! -d "$ROOT/bin/prog" ]]; then
  echo "[e2e] bin/ 缺失或指定 --manual-app: 不自动编译/启动 (避免与并行编译冲突)"
  echo "[e2e] 请手动启动应用 (日志务必重定向到同一文件):"
  echo "        cd $ROOT && python script/build.py run > '$LOG' 2>&1 &"
  if [[ "$MANUAL_APP" != "1" && ! -d "$ROOT/bin/prog" ]]; then
    echo "[e2e] 或先完成编译后再跑本脚本 (不带 --manual-app)。"
    # 无 bin 且非显式 manual: 直接失败退出, 避免空日志浪费一轮
    exit 1
  fi
else
  echo "[e2e] 启动应用 (python script/build.py run, 日志: $LOG) ..."
  ( cd "$ROOT" && python script/build.py run >"$LOG" 2>&1 ) &
  APP_PID=$!
fi

# 等应用真正开始轮询 (mock 的 /state 计数 > 0), 最多 60s
echo "[e2e] 等待应用开始轮询 8111 ..."
POLLING=0
for i in $(seq 1 60); do
  CNT=$(curl -s --noproxy '*' --max-time 2 "http://127.0.0.1:${PORT}/_mock/state" \
        | python -c "import sys,json;d=json.load(sys.stdin);print(d.get('requests',{}).get('/state',0))" 2>/dev/null || echo 0)
  if [[ "${CNT:-0}" -gt 0 ]]; then POLLING=1; break; fi
  sleep 1
done
if [[ "$POLLING" != "1" ]]; then
  echo "[e2e] 警告: 60s 内未观测到应用轮询 (应用可能未启动/在轮询备用端口), 继续执行断言..."
fi

# ---------------------------------------------------------------- 3. 运行
echo "[e2e] 运行 ${DURATION}s (场景 $SCENARIO) ..."
HALF=$(( DURATION / 2 ))
sleep "$HALF"
echo "[e2e] 中途状态: $(curl -s --noproxy '*' --max-time 2 "http://127.0.0.1:${PORT}/_mock/state" | head -c 400)"
sleep "$(( DURATION - HALF ))"

# ---------------------------------------------------------------- 4/5. 停应用/停 mock
stop_app
sleep 2   # 给日志 flush 留时间
stop_mock

# ---------------------------------------------------------------- 6. 断言
EXTRA=""
case "$SCENARIO" in
  s5*|*missing*)  ;;                                  # 缺失 FM 场景: 要求出现缺失提示
  *)               EXTRA="--allow-missing-notify" ;;  # 其余场景允许 0 次缺失提示
esac

echo "[e2e] 断言日志: $LOG"
python "$ROOT/script/e2e_assert.py" --log "$LOG" --duration "$DURATION" $EXTRA
RC=$?

echo "[e2e] 完成, 退出码 $RC (0=通过 1=失败); 日志: $LOG / $MOCK_LOG"
exit $RC
