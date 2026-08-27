#!/usr/bin/env bash
# -*- coding: utf-8 -*-
#
# Rust 版 e2e 编排 (对齐 script/e2e_fm.sh 的编排结构, 断言器复用 e2e_assert.py)
#
# 流程 (每场景): 端口空闲探测(被占 SKIP) → 起 mock → 起 voidmei --live --port
#   (日志落文件) → 启动锚点校验 → 跑 duration 秒(每秒探活) → taskkill 杀进程树 →
#   mock 终态快照落盘 → 停 mock → python script/e2e_assert.py 断言 (A1~A6) → 汇总
#
# 防空转通过 (审查警告 W1): 应用秒崩/中途退出时残缺日志上的断言可能整体空转
# PASS, 故门禁三重收口:
#   1) 启动锚点: 日志必须在 30s 内出现 "Auto-start enabled"/"Starting Game Mode
#      Services" (live 模式进入的必经日志, 文本对位 Java), 否则 FAIL;
#   2) 运行期探活: duration 内每秒 kill -0 探活, 应用中途死亡立即 FAIL;
#   3) --require-run: 全场景 SKIP (零实跑) 时退出码 1, 供 CI/P6 门禁拒绝空转绿。
#
# 与 Java e2e_fm.sh 的差异:
#   - live 模式由 `voidmei --live` CLI 注入 (等价 autoStartGameMode=true),
#     无需临时翻转 ui_layout.user.cfg (也就无需备份/还原)。
#   - 断言结果同时落 JSON (build/e2e_rust_<场景>_<时间戳>.json) 供 CI/收口读取。
#
# 用法:
#   script/rust_e2e.sh                                        # 三场景全跑 (各 20s)
#   script/rust_e2e.sh --scenario s5_missing_fm --duration 40  # 单场景
#   script/rust_e2e.sh --scenario menu_flag_false --port 18111 # mock 换端口
#
# 参数:
#   --scenario <name>  s2_preview_live / s5_missing_fm / menu_flag_false / all(默认)
#   --duration  <s>    每场景运行秒数 (默认 20)
#   --port      <p>    mock+应用端口 (默认 9222; 应用经 --port CLI 同步覆盖。
#                      白盒测试端口约定: 9222 = Java 备用端口 (appPortBkp) 域,
#                      游戏本地 API 恒占 8111 而 9222 游戏永不监听 — 真机在跑
#                      测试也不再被挤掉/误读游戏数据)
#   --log       <path> 应用日志输出 (默认 build/e2e_rust_<场景>_<时间戳>.log;
#                      仅单场景时生效, all 模式每场景独立文件)
#   --no-build        跳过预热构建, 直接用既有 rust/target/release/voidmei
#                      (共享工作区源码被并行改动编译不过时, 用上一稳定产物验收;
#                      二进制缺失仍 FAIL —— 不假通过)
#   --require-run     门禁模式: 全部场景 SKIP (零实跑) 时退出码 1。
#                      默认关 (对齐 e2e_fm.sh "被占自动跳过"惯例); CI/P6 验收门禁
#                      应带上, 避免 9222 被占时零实跑也绿。
#
# 产物 (每场景, build/ 下): <场景>.log 应用日志 / mock_<场景>.log mock 日志 /
#   mock_state_<场景>.json mock 终态快照 (含 /state 等请求计数, 量化证据可复验) /
#   <场景>.json 断言结果 (A1~A6) + e2e_meta + mock_state 合体。
#
# 退出码: 0=全部通过(含 SKIP; --require-run 下全 SKIP 为 1)  1=断言失败/流程失败
#
# 注意: 端口 (默认 9222) 被其它白盒测试/mock 占用时该场景 SKIP (项目惯例: 不换端口硬凑、不假通过)。
#       SKIP 不计入失败 —— 三场景默认串行, 每场景起停独立 mock。

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCENARIO="all"
DURATION=20
PORT=9222
LOG=""
NO_BUILD=0
REQUIRE_RUN=0
ALL_SCENARIOS="s2_preview_live s5_missing_fm menu_flag_false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario)    SCENARIO="$2"; shift 2 ;;
    --duration)    DURATION="$2"; shift 2 ;;
    --port)        PORT="$2"; shift 2 ;;
    --log)         LOG="$2"; shift 2 ;;
    --no-build)    NO_BUILD=1; shift ;;
    --require-run) REQUIRE_RUN=1; shift ;;
    *) echo "未知参数: $1"; exit 1 ;;
  esac
done

TS="$(date +%Y%m%d_%H%M%S)"
mkdir -p "$ROOT/build"

APP_PID=""
MOCK_PID=""

# ---- 清理兜底: 正常流程每场景自清, EXIT trap 只兜异常中断 (对齐 e2e_fm.sh) ----
cleanup() {
  stop_app
  stop_mock
}
trap cleanup EXIT INT TERM

APP_STOP_FAILED=0
stop_app() {
  APP_STOP_FAILED=0
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    # MSYS $! 是 msys pid, taskkill 需要真实 Windows PID (/proc/<pid>/winpid);
    # 直接拿 msys pid 喂 taskkill 会静默失败 → 残留进程锁 target/release/voidmei.exe
    local WINPID=""
    [[ -r "/proc/$APP_PID/winpid" ]] && WINPID="$(cat "/proc/$APP_PID/winpid")"
    echo "[rust-e2e] 停止应用 (PID $APP_PID winpid=$WINPID, 杀进程树) ..."
    if command -v taskkill >/dev/null 2>&1 && [[ -n "$WINPID" ]]; then
      # //T 杀整棵进程树; //F 强杀 (GUI 应用不响应 WM_CLOSE)
      taskkill //F //T //PID "$WINPID" >/dev/null 2>&1 || kill -f "$APP_PID" 2>/dev/null
    else
      kill -f "$APP_PID" 2>/dev/null   # MSYS kill -f = TerminateProcess
    fi
    # 死透校验 (5s): 杀不死即置失败标志, 不假通过 (残留进程干扰后续场景构建/端口)
    local _i
    for _i in 1 2 3 4 5; do
      kill -0 "$APP_PID" 2>/dev/null || break
      sleep 1
    done
    if kill -0 "$APP_PID" 2>/dev/null; then
      echo "[rust-e2e] 错误: 应用进程 $APP_PID 5s 内未终止 (残留锁 exe/端口)"
      APP_STOP_FAILED=1
    fi
  fi
  APP_PID=""
}

stop_mock() {
  if [[ -n "$MOCK_PID" ]] && kill -0 "$MOCK_PID" 2>/dev/null; then
    echo "[rust-e2e] 停止 mock (/_mock/shutdown) ..."
    curl -s --noproxy '*' --max-time 5 "http://127.0.0.1:${PORT}/_mock/shutdown" >/dev/null 2>&1
    for _ in 1 2 3 4 5; do
      kill -0 "$MOCK_PID" 2>/dev/null || break
      sleep 1
    done
    kill "$MOCK_PID" 2>/dev/null
  fi
  MOCK_PID=""
}

# ---- 8111 空闲探测: 判定结果三态, 任一非 FREE 都 SKIP (不假通过) ----
#   BUSY-LISTEN: connect 成功 = 有监听者 (游戏/残留 mock)
#   BUSY-BIND  : connect 被拒但裸 bind 失败 = mock 自己也起不来 (TIME_WAIT 残留/
#               隐蔽占用) —— 与 MockServer 的 ("", port) + Windows 不设
#               SO_REUSEADDR 的绑定行为完全对齐, 提前判 SKIP 而非跑到 15s FAIL
#   BUSY-ERROR : 探测脚本自身异常 —— 保守按占用处理 (宁 SKIP 不空跑)
# (单独 connect 判定最稳; bind 预检对齐 mock 绑定语义, 二者互补)
port_busy() {
  python - "$1" <<'PYEOF'
import socket, sys
port = int(sys.argv[1])
s = socket.socket()
s.settimeout(1.0)
try:
    s.connect(("127.0.0.1", port))
    print("BUSY-LISTEN")
    sys.exit(0)
except OSError:
    pass   # 无监听者, 继续 bind 预检
finally:
    s.close()
try:
    b = socket.socket()
    b.bind(("", port))   # 与 MockServer 相同的绑定地址族
    b.close()
    print("FREE")
except OSError:
    print("BUSY-BIND")
except Exception:
    print("BUSY-ERROR")
PYEOF
  # python 本身不可用/崩溃时输出为空 → 由调用方按 BUSY-ERROR 处理
}

# ---- 单场景主流程: $1=场景名; 结果写全局 LAST_RESULT ("PASS|FAIL|SKIP:<场景>") ----
LAST_RESULT=""
run_scenario() {
  local SC="$1"
  local SC_LOG="$LOG"
  if [[ -z "$SC_LOG" ]]; then
    SC_LOG="$ROOT/build/e2e_rust_${SC}_${TS}.log"
  fi
  local MOCK_LOG="$ROOT/build/e2e_rust_mock_${SC}_${TS}.log"
  local RESULT_JSON="$ROOT/build/e2e_rust_${SC}_${TS}.json"

  echo "==================================================================="
  echo "[rust-e2e] 场景 $SC (端口 $PORT, 时长 ${DURATION}s)"
  echo "==================================================================="

  # 1. 端口占用检查 (游戏/残留进程在跑 → SKIP, 不做假通过)
  local PB
  PB="$(port_busy "$PORT")"
  case "$PB" in
    FREE)        : ;;
    BUSY-LISTEN) echo "[rust-e2e] SKIP: 端口 $PORT 已被监听 (游戏/其它 mock 在跑?)" ;;
    BUSY-BIND)   echo "[rust-e2e] SKIP: 端口 $PORT 暂不可绑定 (TIME_WAIT 残留/隐蔽占用, mock 起不来)" ;;
    *)           echo "[rust-e2e] SKIP: 端口 $PORT 探测异常 ('$PB' — python 不可用?), 保守跳过" ;;
  esac
  if [[ "$PB" != "FREE" ]]; then
    LAST_RESULT="SKIP:$SC"
    return 0
  fi

  # 2. 起 mock + 就绪等待
  echo "[rust-e2e] 启动 mock (场景=$SC 端口=$PORT) ..."
  python "$ROOT/script/mock_8111.py" serve --port "$PORT" --scenario "$SC" >"$MOCK_LOG" 2>&1 &
  MOCK_PID=$!
  local READY=0
  for _ in $(seq 1 15); do
    if curl -s --noproxy '*' --max-time 2 "http://127.0.0.1:${PORT}/_mock/state" >/dev/null 2>&1; then
      READY=1; break
    fi
    sleep 1
  done
  if [[ "$READY" != "1" ]]; then
    echo "[rust-e2e] FAIL: mock 15s 内未就绪, mock 日志:"
    cat "$MOCK_LOG"
    LAST_RESULT="FAIL:$SC"
    return 1
  fi
  echo "[rust-e2e] mock 就绪 (日志: $MOCK_LOG)"

  # 3. 起应用 (--live = autoStartGameMode 注入, 无需动 user cfg)。
  #    直起二进制而非 rust_run.sh: 预热构建已保证产物新鲜, rust_run.sh 内的
  #    cargo build 输出会混入应用日志污染断言 (build + cd 根 + exec 的语义
  #    已被预热步骤与本处 cd 完整覆盖)
  VOIDMEI_BIN="$ROOT/rust/target/release/voidmei"
  if [[ ! -f "$VOIDMEI_BIN" ]]; then
    echo "[rust-e2e] FAIL: $VOIDMEI_BIN 不存在 (预热构建异常)"
    LAST_RESULT="FAIL:$SC"
    return 1
  fi
  echo "[rust-e2e] 启动 voidmei --live (日志: $SC_LOG) ..."
  ( cd "$ROOT" && exec "$VOIDMEI_BIN" --live --port "$PORT" >"$SC_LOG" 2>&1 ) &
  APP_PID=$!
  # 归属留痕: 打出 winpid, 残留进程可按此判定是否本脚本泄漏并清理
  local APP_WINPID=""
  [[ -r "/proc/$APP_PID/winpid" ]] && APP_WINPID="$(cat "/proc/$APP_PID/winpid")"
  echo "[rust-e2e] voidmei 已启动 (PID $APP_PID, winpid=$APP_WINPID)"

  # 3.5 启动锚点校验 (防空转通过 W1): 应用秒崩时日志无启动序列, 断言会在
  #     残缺/空日志上空转 PASS —— 锚点 (游戏模式必经日志) 30s 未出现即 FAIL
  echo "[rust-e2e] 等待启动锚点 (Auto-start enabled / Starting Game Mode Services) ..."
  local ANCHOR=0
  for _ in $(seq 1 30); do
    if grep -qE "Auto-start enabled|ACTION: Starting Game Mode Services" "$SC_LOG" 2>/dev/null; then
      ANCHOR=1; break
    fi
    kill -0 "$APP_PID" 2>/dev/null || break   # 进程已死, 提前跳出走 FAIL
    sleep 1
  done
  if [[ "$ANCHOR" != "1" ]]; then
    echo "[rust-e2e] FAIL: 启动锚点未出现 (应用启动失败/秒崩?), 日志尾部:"
    tail -n 20 "$SC_LOG" 2>/dev/null || echo "(日志不存在或为空)"
    stop_app
    stop_mock
    LAST_RESULT="FAIL:$SC"
    return 1
  fi
  echo "[rust-e2e] 启动锚点确认 (游戏模式已进入)"

  # 等应用开始轮询 (mock 的 /state 计数 > 0), 最多 60s
  echo "[rust-e2e] 等待应用开始轮询 $PORT ..."
  local POLLING=0
  for _ in $(seq 1 60); do
    local CNT
    CNT=$(curl -s --noproxy '*' --max-time 2 "http://127.0.0.1:${PORT}/_mock/state" \
          | python -c "import sys,json;d=json.load(sys.stdin);print(d.get('requests',{}).get('/state',0))" 2>/dev/null || echo 0)
    if [[ "${CNT:-0}" -gt 0 ]]; then POLLING=1; break; fi
    sleep 1
  done
  if [[ "$POLLING" != "1" ]]; then
    echo "[rust-e2e] 警告: 60s 内未观测到应用轮询 (menu 场景 valid=false 不拉 /state 数据为正常), 继续执行断言..."
  fi

  # 4. 运行 duration (每秒探活, 中途打一次 mock 状态 —— 探活防空转通过 W1:
  #    应用中途退出时残缺日志上的断言会空转 PASS, 立即判 FAIL 不再干等)
  echo "[rust-e2e] 运行 ${DURATION}s (每秒探活) ..."
  local HALF=$(( DURATION / 2 ))
  local ELAPSED=0
  while [[ "$ELAPSED" -lt "$DURATION" ]]; do
    if ! kill -0 "$APP_PID" 2>/dev/null; then
      echo "[rust-e2e] FAIL: 应用进程在第 ${ELAPSED}s 已退出 (运行期中途死亡), 日志尾部:"
      tail -n 20 "$SC_LOG" 2>/dev/null
      stop_app
      stop_mock
      LAST_RESULT="FAIL:$SC"
      return 1
    fi
    sleep 1
    ELAPSED=$(( ELAPSED + 1 ))
    if [[ "$HALF" -gt 0 && "$ELAPSED" == "$HALF" ]]; then
      echo "[rust-e2e] 中途状态: $(curl -s --noproxy '*' --max-time 2 "http://127.0.0.1:${PORT}/_mock/state" | head -c 400)"
    fi
  done

  # 5. 停应用 + 停 mock (顺序与 e2e_fm.sh 一致)
  stop_app
  if [[ "$APP_STOP_FAILED" == "1" ]]; then
    stop_mock
    LAST_RESULT="FAIL:$SC"
    return 1
  fi
  sleep 2   # 给日志 flush 留时间

  # 5.5 mock 终态快照落盘 (审查警告 W2: '/state 轮询 N 次' 原本只存在于 stdout
  #     中途 echo, 产物无法复验 —— 停 mock 前抓取 /_mock/state 终值落盘,
  #     并合入断言 JSON (见步骤 6), 量化证据机读可查)
  local MOCK_STATE_JSON="$ROOT/build/e2e_rust_mock_state_${SC}_${TS}.json"
  curl -s --noproxy '*' --max-time 5 "http://127.0.0.1:${PORT}/_mock/state" >"$MOCK_STATE_JSON" 2>/dev/null || true
  local STATE_CNT
  STATE_CNT="$(python - "$MOCK_STATE_JSON" <<'PYEOF'
import json, sys
try:
    with open(sys.argv[1], encoding="utf-8") as f:
        print(json.load(f).get("requests", {}).get("/state", 0))
except Exception:
    print("?")
PYEOF
)"
  echo "[rust-e2e] mock 终态: /state 请求 ${STATE_CNT} 次 (快照: $MOCK_STATE_JSON)"
  stop_mock

  # 6. 断言 (场景 → 参数映射同 e2e_fm.sh: 缺失场景要求 >=1 次缺失提示)
  local EXTRA=""
  case "$SC" in
    s5*|*missing*)  ;;                                  # 缺失 FM 场景: 要求出现缺失提示
    *)               EXTRA="--allow-missing-notify" ;;  # 其余场景允许 0 次缺失提示
  esac

  echo "[rust-e2e] 断言日志: $SC_LOG"
  python "$ROOT/script/e2e_assert.py" --log "$SC_LOG" --duration "$DURATION" $EXTRA
  local RC=$?
  # JSON 结果落盘 (--json 模式幂等重跑, 供 CI/收口机读; PYTHONIOENCODING 钉死
  # UTF-8 —— Windows 重定向到文件时 Python 默认 ANSI 代码页, 中文细节会落成 GBK)
  PYTHONIOENCODING=utf-8 python "$ROOT/script/e2e_assert.py" --log "$SC_LOG" --duration "$DURATION" $EXTRA --json >"$RESULT_JSON" 2>/dev/null
  # 同输入两次运行退出码应一致 (确定性); 不一致说明有隐藏状态, 取 FAIL 侧不假通过
  local JSON_RC=$?
  if [[ "$JSON_RC" -ne "$RC" ]]; then
    echo "[rust-e2e] 警告: 断言两次运行退出码不一致 (人读=$RC, JSON=$JSON_RC), 取 FAIL 侧"
    [[ "$JSON_RC" -ne 0 ]] && RC="$JSON_RC"
  fi
  # 断言 JSON 合入 e2e 元数据 + mock 终态 (路径经 argv 传递 —— MSYS 会做
  # POSIX→Windows 路径转换, 内嵌在 -c 字符串里则不会转, 务必保持 argv 形式)
  PYTHONIOENCODING=utf-8 python - "$RESULT_JSON" "$MOCK_STATE_JSON" "$SC" "$DURATION" "$PORT" <<'PYEOF'
import json, sys
result_path, state_path = sys.argv[1], sys.argv[2]
try:
    with open(result_path, encoding="utf-8") as f:
        doc = json.load(f)
    doc["e2e_meta"] = {"scenario": sys.argv[3], "duration_s": float(sys.argv[4]),
                       "port": int(sys.argv[5]), "log": doc.get("log")}
    try:
        with open(state_path, encoding="utf-8") as f:
            doc["mock_state"] = json.load(f)
    except Exception:
        doc["mock_state"] = None   # 快照抓取失败如实记空, 不伪造
    with open(result_path, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
except Exception as e:
    print("警告: 结果 JSON 合入失败: %s" % e)
PYEOF
  echo "[rust-e2e] 断言 JSON (含 e2e_meta + mock 终态): $RESULT_JSON"

  if [[ "$RC" -eq 0 ]]; then
    LAST_RESULT="PASS:$SC"
  else
    LAST_RESULT="FAIL:$SC"
  fi
  return "$RC"
}

# ---- 预热构建 (同步完成, 避免首个场景的 duration 计时被增量编译吃掉) ----
if [[ "$NO_BUILD" == "1" ]]; then
  if [[ ! -f "$ROOT/rust/target/release/voidmei.exe" && ! -f "$ROOT/rust/target/release/voidmei" ]]; then
    echo "[rust-e2e] FAIL: --no-build 但二进制缺失 (rust/target/release/voidmei)"
    exit 1
  fi
  echo "[rust-e2e] --no-build: 沿用既有二进制 ($((ls -l "$ROOT/rust/target/release/voidmei.exe" 2>/dev/null || ls -l "$ROOT/rust/target/release/voidmei") | awk '{print $6, $7, $8}'))"
else
  echo "[rust-e2e] 预热构建 (cargo build --release --bin voidmei) ..."
  if ! ( cd "$ROOT/rust" && cargo build --release --bin voidmei ); then
    echo "[rust-e2e] FAIL: 构建失败 (共享工作区源码中间态? 可用 --no-build 沿用既有稳定产物)"
    exit 1
  fi
fi

# ---- 场景调度 ----
declare -a RESULTS
OVERALL=0
RAN=0        # 实跑场景数 (PASS+FAIL); SKIP 不计 —— --require-run 的判据
SKIPPED=0
FAILED=0
if [[ "$SCENARIO" == "all" ]]; then
  SC_LIST="$ALL_SCENARIOS"
else
  SC_LIST="$SCENARIO"
fi
for SC in $SC_LIST; do
  if [[ "$SCENARIO" == "all" ]]; then LOG=""; fi   # all 模式每场景独立日志文件
  LAST_RESULT="FAIL:$SC"                            # 流程中断 (set -u 下未赋值) 的兜底
  run_scenario "$SC" || true
  RESULTS+=("$LAST_RESULT")
  case "${LAST_RESULT%%:*}" in
    PASS) RAN=$(( RAN + 1 )) ;;
    FAIL) RAN=$(( RAN + 1 )); FAILED=$(( FAILED + 1 )); OVERALL=1 ;;
    SKIP) SKIPPED=$(( SKIPPED + 1 )) ;;
  esac
  sleep 2   # 场景间端口释放缓冲
done

echo "==================================================================="
echo "[rust-e2e] 汇总:"
for R in "${RESULTS[@]}"; do
  echo "  $R"
done
echo "[rust-e2e] 实跑 $RAN (FAIL $FAILED) / SKIP $SKIPPED / 共 ${#RESULTS[@]}"
# 门禁模式: 零实跑不构成验收 (默认关, 对齐 e2e_fm.sh "被占自动跳过"惯例)
if [[ "$REQUIRE_RUN" == "1" && "$RAN" -eq 0 ]]; then
  echo "[rust-e2e] FAIL: --require-run 生效且全部场景 SKIP (8111 被占?), 零实跑不接受"
  OVERALL=1
fi
echo "[rust-e2e] 完成, 退出码 $OVERALL (0=通过 1=失败)"
exit "$OVERALL"
