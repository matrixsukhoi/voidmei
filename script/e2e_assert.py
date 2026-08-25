#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
VoidMei E2E 日志断言器 (纯标准库)

对应用运行日志做行为断言, 用于 issue #55 (机型不在 FM 库时的死循环) 的
回归验收。判定死循环的四个特征:

  A1 FM 加载风暴: FM 全量解析日志 (新架构 "Parsed FM file '<机型>' in N ms",
     旧架构 "Lazily Loading Flight Model for: <机型>") 每机型 > 2 次/分钟
  A2 异常堆栈刷屏: 同一异常首行 (类名+消息) 重复 > 5 次
  A3 FM 缺失提示: 缺失场景下应出现 >= 1 次, 且同类提示 <= 3 次/分钟 (降级不刷屏)
  A5 日志总量: 总行数 > 2500 行/分钟 (单模板互不重复的海量爆炸兜底;
     历史三档基线 s2=74/s5=107/畸形692 行/分钟, 峰值 3.6x 余量)
  A6 WARN/ERROR 速率: > 30 行/分钟 (错误场景降级也不该告警刷屏; 实测三档均 0)
  A4 (附加) 消息刷屏: 任意同一模板消息 > 120 次/分钟 (捕获如
     "Aircraft type changed ... Restarting Controller" 的 S4toS1 重启循环)

用法:
  python script/e2e_assert.py --log app.log --duration 60 [--allow-missing-notify] [--json]

退出码: 0=全部通过, 1=存在 FAIL, 2=输入错误 (如日志不存在)
"""

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

# ---- 日志行格式: [HH:mm:ss.SSS] [Component ] message  /  ... [WARN ] message ----
RE_TIMESTAMP = re.compile(r"^\[(\d{2}):(\d{2}):(\d{2})\.(\d{3})\]")

# A1: FM 加载 (按机型分组)。
# 新架构 (P2+, FMLoader→Blkx.getAllplotdata): "Parsed FM file 'fm/xxx.blkx' in N ms (...)"
#   —— 每次真正的全量 FM 解析打一次, 路径转 basename 去扩展名即机型名
# 旧架构 (Controller.loadFMData): "(Lazily )Loading Flight Model for: xxx" (兼容保留)
RE_FM_LOADING = re.compile(
    r"(?:Parsed FM file '([^']+)'|(?:Lazily\s+)?Loading Flight Model for:\s*(\S+))")


def plane_key(raw: str) -> str:
    """FM 加载日志的捕获串 → 机型名 (路径取 basename 并剥 .blkx/.blk 扩展)"""
    name = raw.replace("\\", "/").rsplit("/", 1)[-1]
    if name.endswith(".blkx"):
        name = name[:-5]
    elif name.endswith(".blk"):
        name = name[:-4]
    return name

# A2: Java printStackTrace 的异常首行, 行首无缩进, 如:
#     java.lang.NullPointerException: Cannot invoke ...
RE_EXC_FIRST = re.compile(r"^([\w.$]+(?:Exception|Error))(?::\s*(.*))?$")

# A3: FM 缺失/加载失败类提示 (任务书原文 + 代码中实际存在的字符串都覆盖)
# 新架构 (P2+) 补充: FMLoader 的 MISSING 路径 (中央文件不存在) 静默返回句柄不打日志,
# 缺失通知由 FMManager 广播 FM_CHANGED 时 EventBus 留痕:
#   "PUBLISH: FMManager -> FMHandle[MISSING he_162]: fmChanged"
# 该句柄串即"当前机型无 FM 可用"的日志形态, 与 CORRUPT 的 warn 一并计入
RE_MISSING_FM = re.compile(
    r"FM文件不存在|FM文件缺失|FM数据缺失|FM解析异常|FM文件加载失败|加载失败|解析失败"
    r"|FMHandle\[(?:MISSING|CORRUPT)")

# 时间窗口下限 (分钟): 日志极短时避免 1 次/0.01min 的除零式误报
MIN_WINDOW_MIN = 0.1


def parse_ts(line: str):
    """解析行首 [HH:mm:ss.SSS] → 当日秒数 (float); 无时间戳返回 None"""
    m = RE_TIMESTAMP.match(line)
    if not m:
        return None
    h, mi, s, ms = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
    return h * 3600 + mi * 60 + s + ms / 1000.0


# A6: WARN/ERROR 级别行 (Logger 输出 "[HH:MM:SS.mmm] [Component] [WARN ] msg" / "[ERROR]")
RE_WARN_ERR = re.compile(r"\[(WARN|ERROR)\s*\]")

def normalize_template(msg: str) -> str:
    """
    把一条消息归一化为模板 (用于"同类提示"分组):
      - 连续数字 → N   (数值参数)
      - 含 _ / . / - 的标识符 → X  (机型名/路径/类名, 如 p-51d-20_china, ./data/a.blkx)
      - 去掉时间戳与等级前缀由调用方负责
    """
    msg = re.sub(r"\d+(\.\d+)?", "N", msg)
    msg = re.sub(r"[A-Za-z_][\w./-]*[_./-][\w./-]*", "X", msg)
    return msg.strip()


def strip_prefix(line: str) -> str:
    """去掉 '[HH:mm:ss.SSS] [Component ] ([WARN ]) ' 前缀, 留下消息体"""
    m = RE_TIMESTAMP.match(line)
    if not m:
        return line.strip()
    rest = line[m.end():]
    # 连续吃掉最多两个 [xxx] 前缀块 (组件名 / 级别)
    for _ in range(2):
        if rest.startswith("["):
            close = rest.find("]")
            if close == -1:
                break
            inner = rest[1:close]
            # 组件名/等级块内不应有空格过长的句子 (避免误吃正文中的方括号)
            if len(inner) <= 12:
                rest = rest[close + 1:]
                continue
            break
    return rest.strip()


def analyze(lines, duration_s: float):
    """逐行扫描, 返回 (结果列表, 统计信息 dict)"""
    stats = {
        "total_lines": 0,
        "timestamped_lines": 0,
        "span_s": None,
        "window_minutes": None,
        "fm_loading_by_plane": {},
        "exception_first_lines": {},
        "missing_fm_total": 0,
        "missing_fm_templates": {},
        "message_templates": {},
    }

    ts_min, ts_max = None, None
    fm_loading = Counter()
    exc_first = Counter()
    stats.setdefault("warn_err_lines", 0)
    missing_tpl = Counter()
    msg_tpl = Counter()

    for line in lines:
        line = line.rstrip("\n")
        if not line.strip():
            continue
        stats["total_lines"] += 1
        # A6: WARN/ERROR 级别行计数 (Logger 非 INFO 级格式为 "[WARN ]"/"[ERROR]" 标记)
        if RE_WARN_ERR.search(line):
            stats["warn_err_lines"] += 1

        ts = parse_ts(line)
        if ts is not None:
            stats["timestamped_lines"] += 1
            if ts_min is None or ts < ts_min:
                ts_min = ts
            if ts_max is None or ts > ts_max:
                ts_max = ts
            # 跨午夜保护: 单次运行不会超 24h, max<min 视为跨 0 点
            if ts_max < ts_min:
                ts_max += 86400.0

        msg = strip_prefix(line)

        # A1: FM 加载计数 (按机型分组; 双模式捕获, 取非空组)
        m = RE_FM_LOADING.search(msg)
        if m:
            fm_loading[plane_key(m.group(1) or m.group(2))] += 1

        # A2: 异常首行 (printStackTrace 输出, 行首即类全名)
        m = RE_EXC_FIRST.match(line.strip())
        if m:
            key = m.group(1) + (": " + m.group(2) if m.group(2) else "")
            exc_first[key] += 1

        # A3: FM 缺失提示 (按归一化模板分组)
        if RE_MISSING_FM.search(msg):
            missing_tpl[normalize_template(msg)] += 1

        # A4: 任意消息模板刷屏 (排除异常首行与 "\tat " 堆栈帧行, 避免与 A2 双重计数)
        if msg and not RE_EXC_FIRST.match(line.strip()) and not line.lstrip().startswith("at "):
            msg_tpl[normalize_template(msg)] += 1

    # 时间窗口: 优先用日志实际跨度 (时间戳可得时), 且不小于 --duration (保守取大, 降低误报)
    span = (ts_max - ts_min) if (ts_min is not None and ts_max > ts_min) else 0.0
    stats["span_s"] = round(span, 3) if span else None
    window_min = max(span / 60.0, duration_s / 60.0, MIN_WINDOW_MIN)
    stats["window_minutes"] = round(window_min, 3)

    stats["fm_loading_by_plane"] = dict(fm_loading)
    stats["exception_first_lines"] = dict(exc_first)
    stats["missing_fm_total"] = sum(missing_tpl.values())
    stats["missing_fm_templates"] = dict(missing_tpl)
    stats["message_templates"] = dict(msg_tpl.most_common(10))
    return stats


def run_assertions(stats, allow_missing_notify: bool):
    """基于统计跑四组断言, 返回结果列表 [{id, name, pass, detail}]"""
    results = []
    wmin = stats["window_minutes"]

    # ---- A1: FM 加载频率 (每机型每分钟 <= 2 次) ----
    bad = {plane: c for plane, c in stats["fm_loading_by_plane"].items()
           if c > 2.0 * wmin + 0.5}   # +0.5 容忍窗口边界取整噪声
    detail = "各机型加载次数: %s (窗口 %.2f 分钟, 阈值 2 次/分钟)" % (
        stats["fm_loading_by_plane"] or "(无)", wmin)
    results.append({
        "id": "A1", "name": "FM 加载频率 (每机型 <= 2 次/分钟)",
        "pass": not bad,
        "detail": detail + ("; 超标: %s" % bad if bad else ""),
    })

    # ---- A2: 同一异常首行重复 <= 5 次 ----
    bad_exc = {k: c for k, c in stats["exception_first_lines"].items() if c > 5}
    top_exc = sorted(stats["exception_first_lines"].items(), key=lambda kv: -kv[1])[:5]
    results.append({
        "id": "A2", "name": "异常堆栈去重 (同一首行 <= 5 次)",
        "pass": not bad_exc,
        "detail": ("Top 异常: %s" % (top_exc if top_exc else "(无)"))
                  + ("; 超标: %s" % bad_exc if bad_exc else ""),
    })

    # ---- A3: FM 缺失提示 >= 1 (除非 --allow-missing-notify) 且同类 <= 3 次/分钟 ----
    total = stats["missing_fm_total"]
    bad_tpl = {k: c for k, c in stats["missing_fm_templates"].items()
               if c > 3.0 * wmin + 0.5}
    if allow_missing_notify:
        a3_pass = not bad_tpl
        a3_name = "FM 缺失提示不刷屏 (同类 <= 3 次/分钟; 允许 0 次)"
    else:
        a3_pass = (total >= 1) and not bad_tpl
        a3_name = "FM 缺失提示 (>= 1 次且同类 <= 3 次/分钟)"
    results.append({
        "id": "A3", "name": a3_name,
        "pass": a3_pass,
        "detail": "缺失提示共 %d 次, 模板: %s%s" % (
            total, stats["missing_fm_templates"] or "(无)",
            "; 超标模板: %s" % bad_tpl if bad_tpl else ""),
    })

    # ---- A4: 任意消息模板刷屏 (<= 120 次/分钟) ----
    # 捕获非"加载"路径的循环, 如每 100ms 一次的 "Aircraft type changed ... Restarting"
    threshold = 120.0 * wmin
    bad_msg = {k: c for k, c in stats["message_templates"].items() if c > threshold}
    top_msg = list(stats["message_templates"].items())[:5]
    results.append({
        "id": "A4", "name": "消息刷屏检测 (同一模板 <= 120 次/分钟)",
        "pass": not bad_msg,
        "detail": "Top 模板: %s%s" % (top_msg if top_msg else "(无)",
                                      "; 超标: %s" % bad_msg if bad_msg else ""),
    })
    # ---- A5: 日志总量速率 (海量日志兜底; 单模板互不重复的爆炸由总量维度捕获) ----
    # 阈值 2500 行/分钟: 历史实测基线 (s2 正常 74/分钟, s5 缺失 107/分钟,
    # 畸形数据场景高频轮询 692/分钟——该场景已按信任边界裁撤, 基线留档) 峰值 ~3.6x 余量;
    # 死循环风暴 (数千行/分钟) 必红
    total_rate = stats["total_lines"] / wmin
    results.append({
        "id": "A5", "name": "日志总量速率 (<= 2500 行/分钟)",
        "pass": total_rate <= 2500.0,
        "detail": "总行数 %d / 窗口 %.2f 分钟 = %.1f 行/分钟 (WARN/ERROR %d 行)"
                  % (stats["total_lines"], wmin, total_rate, stats.get("warn_err_lines", 0)),
    })

    # ---- A6: WARN/ERROR 级别总量速率 (错误场景降级也不该产生告警刷屏) ----
    # 阈值 30 行/分钟: 三档实测均 0; 启动期瞬时告警可容忍, 持续 0.5 次/秒即异常
    we_rate = stats.get("warn_err_lines", 0) / wmin
    results.append({
        "id": "A6", "name": "WARN/ERROR 速率 (<= 30 行/分钟)",
        "pass": we_rate <= 30.0,
        "detail": "WARN/ERROR %d 行 / 窗口 %.2f 分钟 = %.1f 行/分钟"
                  % (stats.get("warn_err_lines", 0), wmin, we_rate),
    })

    return results


def main():
    parser = argparse.ArgumentParser(description="VoidMei E2E 日志断言器")
    parser.add_argument("--log", type=Path, required=True, help="应用日志文件路径")
    parser.add_argument("--duration", type=float, default=60.0,
                        help="应用实际运行秒数 (费率分母; 默认 60)")
    parser.add_argument("--allow-missing-notify", action="store_true",
                        help="允许 FM 缺失提示为 0 次 (用于不含缺失机型的场景)")
    parser.add_argument("--json", action="store_true", help="以 JSON 输出结果")
    args = parser.parse_args()

    if not args.log.exists():
        print("错误: 日志文件不存在: %s" % args.log, file=sys.stderr)
        sys.exit(2)
    try:
        lines = args.log.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as e:
        print("错误: 无法读取日志: %s" % e, file=sys.stderr)
        sys.exit(2)

    stats = analyze(lines, args.duration)
    results = run_assertions(stats, args.allow_missing_notify)
    failed = [r for r in results if not r["pass"]]

    if args.json:
        print(json.dumps({
            "log": str(args.log),
            "pass": not failed,
            "exit_code": 0 if not failed else 1,
            "stats": stats,
            "assertions": results,
        }, ensure_ascii=False, indent=2))
    else:
        print("=" * 72)
        print("VoidMei E2E 日志断言  %s" % args.log)
        print("日志行数: %d | 实际跨度: %s s | 计费窗口: %.2f 分钟"
              % (stats["total_lines"], stats["span_s"], stats["window_minutes"]))
        print("=" * 72)
        for r in results:
            mark = "PASS" if r["pass"] else "FAIL"
            print("[%s] %s %s" % (mark, r["id"], r["name"]))
            print("       %s" % r["detail"])
        print("=" * 72)
        if failed:
            print("结果: FAIL (%d 条断言未通过)" % len(failed))
        else:
            print("结果: PASS (全部断言通过)")

    sys.exit(0 if not failed else 1)


if __name__ == "__main__":
    main()
