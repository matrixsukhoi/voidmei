#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""从 更新日志.txt 提取指定版本的条目块, 输出为 GitHub Release body。

更新日志.txt 是唯一的人工维护更新记录 (git 跟踪, 发版前手写新版本块);
CI 发版时只读不改 —— tag 的 commit 里就带着最新日志, zip 内外天然一致,
不存在任何回写/同步环节。

用法:
  python script/release_notes.py extract <version>   提取 vX.YYY 条目块 -> stdout (Release body)
  python script/release_notes.py preview <version>   同上, 本地预览别名
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEP_LINE = "_______________________________________"


def log_err(msg):
    print("[error] " + msg, file=sys.stderr)


def extract_block(ver):
    """提取 "v{ver}" 行到下一个 分隔线/vX 版本行 之间的条目 (剥 \\r 兼容 CRLF)。

    版本行整行全等匹配, v1.58 不会误配 v1.584;
    结束界同时认分隔线与下一版本行, 兼容历史上缺分隔线的脏数据
    (如 v1.572 曾连续出现两块、第二块无分隔线开头)。
    """
    try:
        text = (ROOT / "更新日志.txt").read_text(encoding="utf-8")
    except FileNotFoundError:
        return []
    want = "v%s" % ver
    out, started = [], False
    for raw in text.splitlines():
        line = raw.rstrip("\r")
        if not started:
            if line == want:
                started = True
            continue
        if line.startswith("_") or re.match(r"^v[0-9]", line):
            break
        out.append(line)
    while out and not out[-1].strip():  # 去尾部空行
        out.pop()
    return out


def main():
    args = sys.argv[1:]
    if len(args) != 2 or args[0] not in ("extract", "preview"):
        print(__doc__)
        sys.exit(1)
    ver = args[1]
    block = extract_block(ver)
    if not block:
        log_err("更新日志.txt 中未找到 v%s 条目块" % ver)
        sys.exit(1)
    print("# VoidMei v%s" % ver)
    print()
    print("\n".join(block))  # 条目本身是 "- xxx", GitHub 会渲染成列表


if __name__ == "__main__":
    main()
