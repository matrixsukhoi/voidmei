#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
VoidMei 战雷 8111 端口模拟器（纯标准库实现, Python 3.8+, CI 可跑）

子命令:
  capture  从真机 8111 抓取 /state /indicators /map_obj.json /map_info.json
           (--save-as <name> 直接存为快照)
  serve    启动模拟服务器 (默认端口 8111 = VoidMei 实际轮询端口, 备用 9222 由应用自行翻转)
  list     列出可用快照与场景

=== 游戏端点响应的 byte-perfect 兼容性 (勿破坏!) =========================
VoidMei 用 HttpHelper.sendGetFastBuf 裸 socket 读响应, StringHelper.getString
做子串级朴素解析, 因此 mock 响应必须满足:
  1. 恰好 6 行头: 状态行 + 4 个头 + 空行 (Java 端 readLine x6 跳头)
  2. 头标签避开 "type"/"valid" 子串 (否则 getString 抢先命中头字段)
  3. JSON 冒号后恰好一个空格 (getString 的 bix = 冒号后第 2 字符,
     无空格会吃掉字符串值的首引号)
  4. 整个响应一次 write 发出, 保证 Java 单次 read() 拿全 (无 Content-Type 头)
=========================================================================

=== 控制通道 (前缀 /_mock/, 与游戏端口共用; VoidMei 不会请求这些路径) ===
  GET /_mock/state                       运行状态 JSON (场景/快照/请求计数/时长)
  GET /_mock/scenario/<name>?restart=1   切换场景, 即时生效 (restart=1 从头重放)
  GET /_mock/snapshot/<name>             直切单快照 (等效单步场景, 不走脚本)
  GET /_mock/raw?type=indicators&body=.. 让指定端点持续返回原文 (fuzz 手工探针;
                                         body 为空则清除该端点 override)
  GET /_mock/shutdown                    优雅退出
控制通道返回普通 HTTP/JSON (不要求 byte-perfect)。
=========================================================================
"""

import argparse
import datetime
import http.server
import json
import socketserver
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# ---------------- Windows 控制台编码修复 ----------------
# Windows 控制台默认 GBK: 中文提示会乱码, 个别字符 (如 ↔) 直接 UnicodeEncodeError。
# 强制 stdout/stderr 用 UTF-8 (Python 3.7+), 失败则静默回退 (极老版本无 reconfigure)。
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, OSError, ValueError):
        pass

# ---------------- 常量 ----------------
DEFAULT_SOURCE_PORT = 8111          # 真机游戏数据端口
DEFAULT_MOCK_PORT = 8111            # serve 默认端口: 与 VoidMei 轮询端口一致 (旧版 8112 是错的)
SCENARIOS_ROOT = Path(__file__).parent / "mock_scenarios"
DEFAULT_SNAPSHOTS_DIR = SCENARIOS_ROOT / "snapshots"
DEFAULT_SCENARIOS_DIR = SCENARIOS_ROOT / "scenarios"
DEFAULT_DATA_FILE = Path(__file__).parent / "mock_data.json"   # 旧版单文件数据 (capture 默认输出)

# VoidMei 轮询的 4 个游戏端点
ENDPOINTS = ["/state", "/indicators", "/map_obj.json", "/map_info.json"]

# 端点短名 → 完整路径 (供 raw_body / /_mock/raw 的 type 参数使用)
EP_ALIASES = {
    "state": "/state",
    "indicators": "/indicators",
    "map_obj": "/map_obj.json",
    "map_obj.json": "/map_obj.json",
    "map_info": "/map_info.json",
    "map_info.json": "/map_info.json",
}


def normalize_ep(name: str) -> Optional[str]:
    """把端点的各种写法 (indicators / /indicators) 规范化为完整路径"""
    name = name.strip().lstrip("/")
    return EP_ALIASES.get(name)


# ---------------- psutil 可选 (仅端口占用提示) ----------------
try:
    import psutil  # type: ignore
except ImportError:
    psutil = None  # CI 无 psutil 时跳过提示, 不影响功能


def get_process_info_on_port(port: int) -> Optional[str]:
    """查找占用端口的进程描述; psutil 缺失时返回 None (仅提示用途)"""
    if psutil is None:
        return None
    try:
        for conn in psutil.net_connections(kind="inet"):
            if conn.laddr.port == port and conn.status == "LISTEN":
                try:
                    proc = psutil.Process(conn.pid)
                    return "%s (PID: %s)" % (proc.name(), conn.pid)
                except (psutil.NoSuchProcess, psutil.AccessDenied):
                    return "PID: %s" % conn.pid
    except Exception:
        return None
    return None


# ---------------- 快照仓库 ----------------
class SnapshotStore:
    """管理 script/mock_scenarios/snapshots/ 下的快照文件 (格式同 mock_data.json:
    {"/state": {...}, "/indicators": {...}, "/map_obj.json": [...], "/map_info.json": {...}})"""

    def __init__(self, dir_path: Path):
        self.dir_path = dir_path

    def load_all(self) -> Dict[str, dict]:
        if not self.dir_path.is_dir():
            return {}
        result = {}
        for f in sorted(self.dir_path.glob("*.json")):
            try:
                result[f.stem] = json.loads(f.read_text(encoding="utf-8"))
            except (OSError, ValueError) as e:
                print("[!] 跳过无效快照 %s: %s" % (f.name, e), file=sys.stderr)
        return result

    def save(self, name: str, data: dict) -> Path:
        """按快照名保存 (自动补 .json 后缀)"""
        if not name.endswith(".json"):
            name += ".json"
        self.dir_path.mkdir(parents=True, exist_ok=True)
        target = self.dir_path / name
        with open(target, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        return target


# ---------------- 场景 ----------------
class Scenario:
    """一个场景 = 按时间轴排列的步骤列表, 每步引用一个快照并可选挂 behavior 开关"""

    def __init__(self, name: str, steps: List[dict], loop: bool = False, desc: str = ""):
        self.name = name
        self.steps = steps
        self.loop = loop
        self.desc = desc

    @classmethod
    def from_dict(cls, name: str, d: dict) -> "Scenario":
        steps = d.get("steps")
        if not isinstance(steps, list) or not steps:
            raise ValueError("场景 %s 缺少非空 steps 数组" % name)
        return cls(name, steps, bool(d.get("loop", False)), str(d.get("desc", "")))

    def step_duration_ms(self, index: int) -> float:
        """单步时长 (毫秒); 缺省 5s, <=0 视为 1ms 防御除零/卡死"""
        try:
            v = float(self.steps[index].get("duration_ms", 5000))
        except (TypeError, ValueError):
            v = 5000.0
        return v if v > 0 else 1.0

    def total_ms(self) -> float:
        return sum(self.step_duration_ms(i) for i in range(len(self.steps)))

    def missing_snapshots(self, available) -> List[str]:
        """返回场景引用但不存在的快照名 (启动时校验用)"""
        missing = []
        for step in self.steps:
            snap = step.get("snapshot")
            if snap and snap not in available:
                missing.append(str(snap))
        return missing


def load_scenarios(dir_path: Path) -> Dict[str, Scenario]:
    if not dir_path.is_dir():
        return {}
    result = {}
    for f in sorted(dir_path.glob("*.json")):
        try:
            d = json.loads(f.read_text(encoding="utf-8"))
            result[f.stem] = Scenario.from_dict(f.stem, d)
        except (OSError, ValueError) as e:
            print("[!] 跳过无效场景 %s: %s" % (f.name, e), file=sys.stderr)
    return result


# ---------------- 场景引擎 (线程安全) ----------------
class MockEngine:
    """
    根据当前时刻惰性计算"当前生效的 step": 无后台线程, 每次请求时用
    time.monotonic() 推进时间轴。loop=false 时最后一步播完停住。
    """

    def __init__(self):
        self._lock = threading.Lock()
        self._mode = "idle"            # idle | scenario | snapshot
        self._scenario: Optional[Scenario] = None
        self._snapshot_name: Optional[str] = None
        self._start = time.monotonic()  # 当前场景/快照的开始时刻
        # /_mock/raw 设置的手工 override: {"/indicators": "原文"}; 场景/快照切换时清除
        self._raw_overrides: Dict[str, str] = {}

    # ---- 切换 ----
    def set_scenario(self, scen: Scenario):
        with self._lock:
            self._mode = "scenario"
            self._scenario = scen
            self._snapshot_name = None
            self._raw_overrides.clear()   # 切换数据源时清掉手工探针, 避免状态纠缠
            self._start = time.monotonic()

    def set_snapshot(self, name: str):
        with self._lock:
            self._mode = "snapshot"
            self._scenario = None
            self._snapshot_name = name
            self._raw_overrides.clear()
            self._start = time.monotonic()

    def set_raw_override(self, ep: str, body: str) -> bool:
        """设置/清除单端点原文 override (body 为空串则清除); True=已设置"""
        with self._lock:
            if body:
                self._raw_overrides[ep] = body
                return True
            self._raw_overrides.pop(ep, None)
            return False

    # ---- 查询 ----
    def current(self) -> Tuple[int, Optional[str], dict]:
        """返回 (step_index, snapshot_name, behavior_dict); idle 时 (0, None, {})"""
        with self._lock:
            if self._mode == "snapshot":
                return 0, self._snapshot_name, {}
            if self._mode != "scenario" or self._scenario is None:
                return 0, None, {}
            scen = self._scenario
            elapsed_ms = (time.monotonic() - self._start) * 1000.0
            total = scen.total_ms()
            # 播完后: loop 则取模回卷, 否则停 (hold) 在最后一步
            if elapsed_ms >= total:
                if scen.loop and total > 0:
                    elapsed_ms = elapsed_ms % total
                else:
                    idx = len(scen.steps) - 1
                    step = scen.steps[idx]
                    return idx, step.get("snapshot"), dict(step.get("behavior") or {})
            # 沿时间轴找当前 step
            acc = 0.0
            for i, step in enumerate(scen.steps):
                acc += scen.step_duration_ms(i)
                if elapsed_ms < acc:
                    return i, step.get("snapshot"), dict(step.get("behavior") or {})
            idx = len(scen.steps) - 1
            step = scen.steps[idx]
            return idx, step.get("snapshot"), dict(step.get("behavior") or {})

    def get_raw_override(self, ep: str) -> Optional[str]:
        with self._lock:
            return self._raw_overrides.get(ep)

    def describe(self, requests: Dict[str, int], uptime_s: float) -> dict:
        """供 /_mock/state 返回的完整状态"""
        idx, snap, behavior = self.current()
        with self._lock:
            info = {
                "mode": self._mode,
                "scenario": self._scenario.name if self._scenario else None,
                "scenario_loop": self._scenario.loop if self._scenario else None,
                "step_index": idx if self._mode == "scenario" else None,
                "step_count": len(self._scenario.steps) if self._scenario else None,
                "snapshot": snap,
                "behavior": behavior,
                "raw_overrides": dict(self._raw_overrides),
                "uptime_s": round(uptime_s, 1),
                "requests": dict(requests),
            }
        return info


# ---------------- behavior 变换 (对快照 JSON 做 fuzz 修饰) ----------------
EXTREME_CYCLE = [1000000000, -65535, float("nan")]  # 极端值轮换; NaN 由 json.dumps 输出字面量


def apply_behaviors(content, behavior: dict):
    """
    按 step 的 behavior 开关修饰快照内容 (仅作用于 dict/list 的 JSON 端点):
      invalid_flag  → /state //indicators 的 valid 置 false (玩家在菜单)
      drop_fields   → 从 state/indicators 顶层删除指定字段
      extreme_values→ 数值字段轮换替换为极端值
    malformed / disconnect / raw_body 在外层处理 (整体替换/断开), 不进本函数。
    """
    if not behavior or not isinstance(content, (dict, list)):
        return content
    if behavior.get("invalid_flag") and isinstance(content, dict):
        content = dict(content)
        content["valid"] = False
    drop = behavior.get("drop_fields")
    if drop and isinstance(content, dict):
        content = dict(content)
        for key in drop:
            content.pop(key, None)
    if behavior.get("extreme_values") and isinstance(content, dict):
        replaced = dict(content)
        cyc = 0
        for k, v in replaced.items():
            # 只动数值字段; 跳过 bool (isinstance(True,int)==True) 与字符串/数组
            if isinstance(v, (int, float)) and not isinstance(v, bool):
                replaced[k] = EXTREME_CYCLE[cyc % len(EXTREME_CYCLE)]
                cyc += 1
        content = replaced
    return content


# ---------------- 服务器状态 (handler 间共享) ----------------
class MockState:
    """聚合引擎 + 请求计数, 由 ThreadingTCPServer 持有并注入每个 handler"""

    def __init__(self, engine: MockEngine, snapshots: Dict[str, dict], verbose: bool = False):
        self.engine = engine
        self.snapshots = snapshots
        self.verbose = verbose
        self.start_time = time.monotonic()
        self._lock = threading.Lock()
        self.counters: Dict[str, int] = {ep: 0 for ep in ENDPOINTS}

    def bump(self, ep: str) -> int:
        """游戏端点请求计数 +1, 返回新值"""
        with self._lock:
            self.counters[ep] = self.counters.get(ep, 0) + 1
            return self.counters[ep]

    def uptime(self) -> float:
        return time.monotonic() - self.start_time

    # ---- 游戏端点响应决策 ----
    def game_response(self, ep: str):
        """
        返回 (kind, payload):
          ("drop",  None)   → 本 step disconnect, 直接断开连接不回包
          ("bytes", b"...") → 完整 raw HTTP 响应 (byte-perfect 6 行头)
          ("404",   None)   → 无数据可服务 (idle 或快照缺该端点)
        优先级: disconnect > /_mock/raw 手工 override > step raw_body > 快照(含变换)
        """
        self.bump(ep)
        _idx, snap_name, behavior = self.engine.current()

        # 1) disconnect: 接受连接但不回任何字节 (Java read 得 EOF, 等价连接失败翻转端口)
        if behavior.get("disconnect"):
            return ("drop", None)

        # 2) /_mock/raw 手工探针 (最高优先的原文 override)
        raw = self.engine.get_raw_override(ep)
        if raw is not None:
            return ("bytes", wrap_raw_response(raw.encode("utf-8")))

        # 3) step 级 raw_body (键支持 indicators / /indicators 等写法)
        raw_body = behavior.get("raw_body") or {}
        if isinstance(raw_body, dict):
            for key, val in raw_body.items():
                if normalize_ep(str(key)) == ep and val is not None:
                    return ("bytes", wrap_raw_response(str(val).encode("utf-8")))

        # 4) 快照数据 (malformed 整体替换; 否则 JSON 变换后压缩输出)
        if snap_name is None:
            return ("404", None)
        snap = self.snapshots.get(snap_name)
        if snap is None:
            return ("404", None)
        content = snap.get(ep)
        if content is None:
            return ("404", None)

        if behavior.get("malformed"):
            body = "not json at all"  # 任务书指定的非 JSON 垃圾原文
        elif isinstance(content, (dict, list)):
            # 冒号后恰好一个空格, 逗号后无空格 — StringHelper.getString 的硬性假设
            body = json.dumps(apply_behaviors(content, behavior), separators=(",", ": "),
                              allow_nan=True)
        else:
            body = str(content)
        return ("bytes", wrap_raw_response(body.encode("utf-8")))


# ---------------- byte-perfect 响应包装 ----------------
def wrap_raw_response(body: bytes) -> bytes:
    """
    手工拼整个 HTTP 响应 (勿改!):
      恰好 6 行: 状态行 + Date + Server + Connection + Content-Length + 空行
      头标签避开 "type"/"valid" 子串 (StringHelper 朴素子串搜索会抢先命中)
      一次 write 发出, 保证 Java sendGetFastBuf 的单次 read() 读全
    """
    date_str = datetime.datetime.now(datetime.timezone.utc).strftime("%a, %d %b %Y %H:%M:%S GMT")
    raw = [
        b"HTTP/1.1 200 OK",
        ("Date: " + date_str).encode("ascii"),
        b"Server: MockServer/3.0",
        b"Connection: close",
        ("Content-Length: %d" % len(body)).encode("ascii"),
        b"",  # 空行
        body,
    ]
    return b"\r\n".join(raw)


# ---------------- HTTP handler ----------------
class MockRequestHandler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        if self.server.mock_state.verbose:  # serve --verbose 时打印请求轨迹
            sys.stderr.write("[mock] %s - %s\n" % (self.address_string(), fmt % args))

    # ---- 工具 ----
    @property
    def mstate(self) -> MockState:
        return self.server.mock_state

    def _send_json(self, obj, status: int = 200):
        """控制通道普通 JSON 响应 (VoidMei 不请求 /_mock/*, 无 byte-perfect 要求)"""
        data = json.dumps(obj, ensure_ascii=False, indent=2).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _send_raw_404(self):
        """自写 404 (同样避开 'type' 头标签, 不用 send_error 的 Content-Type)"""
        body = b"not found"
        raw = [
            b"HTTP/1.1 404 Not Found",
            b"Server: MockServer/3.0",
            b"Connection: close",
            ("Content-Length: %d" % len(body)).encode("ascii"),
            b"",
            body,
        ]
        self.close_connection = True
        self.wfile.write(b"\r\n".join(raw))

    # ---- 主入口 ----
    def do_GET(self):
        try:
            parsed = urllib.parse.urlsplit(self.path)
            path = parsed.path
            if path.startswith("/_mock/"):
                self.handle_control(path, urllib.parse.parse_qs(parsed.query))
            elif path in ENDPOINTS:
                self.handle_game(path)
            else:
                self._send_raw_404()
        except (BrokenPipeError, ConnectionResetError):
            pass  # 客户端 (Java) 读完即关, 属常态
        except Exception as e:  # 兜底: 不能让单个坏请求杀死 handler 线程
            sys.stderr.write("[mock] handler error %s: %s\n" % (self.path, e))

    # ---- 游戏端点 ----
    def handle_game(self, ep: str):
        kind, payload = self.mstate.game_response(ep)
        if kind == "drop":
            # disconnect step: 不回任何字节直接关连接 (Java 侧 read()==-1 → 空串 → 翻转端口)
            self.close_connection = True
            return
        if kind == "404":
            self._send_raw_404()
            return
        # byte-perfect 响应: 一次 write + 主动关闭 (Java 每次 poll 都新建连接)
        self.close_connection = True
        self.wfile.write(payload)

    # ---- 控制通道 ----
    def handle_control(self, path: str, query: dict):
        state = self.mstate
        parts = [p for p in path.split("/") if p]  # ["_mock", "state"] 等

        if path == "/_mock/state":
            self._send_json(state.engine.describe(state.counters, state.uptime()))
            return

        if len(parts) >= 2 and parts[1] == "shutdown":
            self._send_json({"ok": True, "message": "shutting down"})
            # 异步停 serve_forever 循环 (不能在 handler 线程里同步等它退出)
            threading.Thread(target=self.server.shutdown, daemon=True).start()
            return

        if len(parts) >= 3 and parts[1] == "scenario":
            name = parts[2]
            scen = self.server.scenarios.get(name)
            if scen is None:
                self._send_json({"ok": False, "error": "场景不存在: %s" % name,
                                 "available": sorted(self.server.scenarios)}, 404)
                return
            # 切换即从头重放 (restart=1 显式语义; 换场景天然 restart)
            state.engine.set_scenario(scen)
            self._send_json({"ok": True, "scenario": name, "restarted": True,
                             "restart": query.get("restart", ["0"])[0] == "1"})
            return

        if len(parts) >= 3 and parts[1] == "snapshot":
            name = parts[2]
            if name not in state.snapshots:
                self._send_json({"ok": False, "error": "快照不存在: %s" % name,
                                 "available": sorted(state.snapshots)}, 404)
                return
            state.engine.set_snapshot(name)
            self._send_json({"ok": True, "snapshot": name})
            return

        if len(parts) >= 2 and parts[1] == "raw":
            ep = normalize_ep(query.get("type", [""])[0])
            body = query.get("body", [""])[0]
            if ep is None:
                self._send_json({"ok": False, "error": "type 须为 state/indicators/map_obj/map_info"},
                                400)
                return
            set_ = state.engine.set_raw_override(ep, body)
            self._send_json({"ok": True, "endpoint": ep,
                             "action": "set" if set_ else "cleared"})
            return

        self._send_json({"ok": False, "error": "未知控制路径: %s" % path}, 404)


# ---------------- 服务器 ----------------
class MockServer(socketserver.ThreadingTCPServer):
    """
    多线程 TCP (VoidMei 会并发请求 /state 与 /indicators)。
    allow_reuse_address 仅在非 Windows 启用: Windows 的 SO_REUSEADDR 是"允许抢占绑定"
    语义 (两个进程可同时 bind 同一端口, 流量归属不确定), 会完全掩盖端口占用错误;
    Linux 上启用才是正语义 (避免 TIME_WAIT 导致的快速重启 bind 失败)。
    """
    allow_reuse_address = (sys.platform != "win32")
    daemon_threads = True

    def __init__(self, addr, handler, mstate: MockState, scenarios: Dict[str, Scenario]):
        super().__init__(addr, handler)
        self.mock_state = mstate
        self.scenarios = scenarios


# ---------------- 子命令: capture ----------------
def run_capture(args):
    """从真机抓数据: urllib 实现 (标准库); --save-as 直接存为快照"""
    base = "http://127.0.0.1:%d" % args.source_port
    print("Capturing flight data from %s ..." % base)
    captured = {}
    for ep in ENDPOINTS:
        url = base + ep
        try:
            with urllib.request.urlopen(url, timeout=2) as resp:
                raw = resp.read().decode("utf-8", errors="replace")
            try:
                captured[ep] = json.loads(raw)  # 能解析则存结构化 JSON
            except ValueError:
                captured[ep] = raw              # 否则存原文 (与旧版行为一致)
            print(" [+] Captured %s" % ep)
        except (urllib.error.URLError, OSError, ValueError) as e:
            print(" [!] Failed to capture %s: %s" % (ep, e))
    if not captured:
        print("Error: No data captured. Is War Thunder running?")
        return 1

    if args.save_as:
        store = SnapshotStore(args.snapshots_dir)
        target = store.save(args.save_as, captured)
        print("Snapshot saved: %s (name=%s)" % (target, target.stem))
    else:
        with open(args.file, "w", encoding="utf-8") as f:
            json.dump(captured, f, indent=4, ensure_ascii=False)
        print("Data successfully saved to %s" % args.file)
    return 0


# ---------------- 子命令: serve ----------------
def run_serve(args):
    snapshots = SnapshotStore(args.snapshots_dir).load_all()
    scenarios = load_scenarios(args.scenarios_dir)

    engine = MockEngine()
    start_notice = ""

    if args.scenario:
        scen = scenarios.get(args.scenario)
        if scen is None:
            print("错误: 场景 '%s' 不存在。可用场景: %s"
                  % (args.scenario, ", ".join(sorted(scenarios)) or "(无)"), file=sys.stderr)
            return 2
        missing = scen.missing_snapshots(snapshots)
        if missing:
            print("错误: 场景 '%s' 引用了不存在的快照: %s。可用快照: %s"
                  % (args.scenario, ", ".join(sorted(set(missing))),
                     ", ".join(sorted(snapshots)) or "(无)"), file=sys.stderr)
            return 2
        engine.set_scenario(scen)
        start_notice = "scenario=%s (%d steps, loop=%s)" % (
            scen.name, len(scen.steps), scen.loop)
    elif args.snapshot:
        if args.snapshot not in snapshots:
            print("错误: 快照 '%s' 不存在。可用快照: %s"
                  % (args.snapshot, ", ".join(sorted(snapshots)) or "(无)"), file=sys.stderr)
            return 2
        engine.set_snapshot(args.snapshot)
        start_notice = "snapshot=%s" % args.snapshot
    elif args.file:
        # 兼容旧用法: serve --file mock_data.json → 单快照模式 (内容即快照)
        if not args.file.exists():
            print("错误: %s 不存在, 请先运行 capture。" % args.file, file=sys.stderr)
            return 2
        try:
            data = json.loads(args.file.read_text(encoding="utf-8"))
        except ValueError as e:
            print("错误: %s 不是合法 JSON: %s" % (args.file, e), file=sys.stderr)
            return 2
        snapshots[args.file.stem] = data
        engine.set_snapshot(args.file.stem)
        start_notice = "snapshot(file)=%s" % args.file.name
    elif "plane_p51d" in snapshots:
        # 默认开箱即用: 加载内置 p51d 快照, 保持旧版 "serve 即有数据" 体验
        engine.set_snapshot("plane_p51d")
        start_notice = "snapshot=plane_p51d (默认)"
    else:
        start_notice = "idle (无数据, 游戏端点 404; 用 /_mock/scenario/<name> 或 /_mock/snapshot/<name> 切换)"

    mstate = MockState(engine, snapshots, verbose=args.verbose)
    print("Starting mock server on port %d ..." % args.port, flush=True)
    print("  数据源: %s" % start_notice, flush=True)
    try:
        server = MockServer(("", args.port), MockRequestHandler, mstate, scenarios)
    except OSError as e:
        # errno 98=Linux EADDRINUSE, 10048=Windows WSAEADDRINUSE
        if e.errno in (98, 10048) or "in use" in str(e).lower() or "通常每个套接字" in str(e):
            occupier = get_process_info_on_port(args.port)
            suffix = " (被 %s 占用)" % occupier if occupier else ""
            print("错误: 端口 %d 已被占用%s。可用 --port 换端口, 或结束占用进程。\n"
                  "  (若 VoidMei 或真机战雷正在运行, 它们就是占用者)" % (args.port, suffix),
                  file=sys.stderr)
        else:
            raise
        return 2

    print("  Mock server active at http://127.0.0.1:%d" % args.port, flush=True)
    print("  控制通道: /_mock/state | /_mock/scenario/<name> | /_mock/snapshot/<name>")
    print("            /_mock/raw?type=<ep>&body=<urlencoded> | /_mock/shutdown")
    print("  Press Ctrl+C to stop.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopping mock server (Ctrl+C).")
    finally:
        server.server_close()
    print("Mock server stopped.", flush=True)
    return 0


# ---------------- 子命令: list ----------------
def run_list(args):
    snapshots = SnapshotStore(args.snapshots_dir).load_all()
    scenarios = load_scenarios(args.scenarios_dir)
    print("快照目录: %s" % args.snapshots_dir)
    if snapshots:
        for name in sorted(snapshots):
            data = snapshots[name]
            eps = ", ".join(k for k in ENDPOINTS if k in data)
            type_ = ""
            ind = data.get("/indicators")
            if isinstance(ind, dict) and "type" in ind:
                type_ = '  type="%s"' % ind["type"]
            print("  [快照] %-22s 端点: %s%s" % (name, eps or "(空)", type_))
    else:
        print("  (无快照; 用 capture --save-as <name> 从真机抓取)")
    print("场景目录: %s" % args.scenarios_dir)
    if scenarios:
        for name in sorted(scenarios):
            scen = scenarios[name]
            print("  [场景] %-22s loop=%-5s steps=%d" % (name, str(scen.loop), len(scen.steps)))
            if scen.desc:
                print("           %s" % scen.desc)
            for i, step in enumerate(scen.steps):
                b = step.get("behavior") or {}
                extra = ("  behavior=%s" % json.dumps(b, ensure_ascii=False)) if b else ""
                print("           step%d: %-20s %6dms%s" % (
                    i, step.get("snapshot", "(无)"), step.get("duration_ms", 5000), extra))
    else:
        print("  (无场景)")
    return 0


# ---------------- CLI ----------------
def main():
    parser = argparse.ArgumentParser(
        description="War Thunder 8111 端口模拟器 (VoidMei E2E 测试用, 纯标准库)")
    sub = parser.add_subparsers(dest="command", required=True)

    cap = sub.add_parser("capture", help="从真机 8111 抓取数据")
    cap.add_argument("--source-port", type=int, default=DEFAULT_SOURCE_PORT,
                     help="真机数据端口 (默认 8111)")
    cap.add_argument("--file", type=Path, default=DEFAULT_DATA_FILE,
                     help="旧版单文件输出路径 (默认 script/mock_data.json)")
    cap.add_argument("--save-as", type=str, default=None,
                     help="直接存为快照 (存入 --snapshots-dir, 名字自动补 .json)")
    cap.add_argument("--snapshots-dir", type=Path, default=DEFAULT_SNAPSHOTS_DIR,
                     help="快照目录 (默认 script/mock_scenarios/snapshots)")

    srv = sub.add_parser("serve", help="启动模拟服务器")
    srv.add_argument("--port", type=int, default=DEFAULT_MOCK_PORT,
                     help="监听端口 (默认 8111, 与 VoidMei 轮询端口一致)")
    srv.add_argument("--scenario", type=str, default=None,
                     help="启动即加载的场景名")
    srv.add_argument("--snapshot", type=str, default=None,
                     help="启动即加载的单快照名 (忽略场景脚本)")
    srv.add_argument("--file", type=Path, default=None,
                     help="兼容旧参数: 直接以指定 JSON 文件作为单快照服务")
    srv.add_argument("--snapshots-dir", type=Path, default=DEFAULT_SNAPSHOTS_DIR,
                     help="快照目录 (默认 script/mock_scenarios/snapshots)")
    srv.add_argument("--scenarios-dir", type=Path, default=DEFAULT_SCENARIOS_DIR,
                     help="场景目录 (默认 script/mock_scenarios/scenarios)")
    srv.add_argument("--verbose", action="store_true",
                     help="打印每个请求的日志 (默认静默)")

    lst = sub.add_parser("list", help="列出可用快照与场景")
    lst.add_argument("--snapshots-dir", type=Path, default=DEFAULT_SNAPSHOTS_DIR)
    lst.add_argument("--scenarios-dir", type=Path, default=DEFAULT_SCENARIOS_DIR)

    args = parser.parse_args()
    if args.command == "capture":
        sys.exit(run_capture(args))
    elif args.command == "serve":
        sys.exit(run_serve(args))
    elif args.command == "list":
        sys.exit(run_list(args))


if __name__ == "__main__":
    main()
