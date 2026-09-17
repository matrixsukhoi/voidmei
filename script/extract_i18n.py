# -*- coding: utf-8 -*-
"""ui_layout.cfg 文本抽取与 @key 化 (i18n P2 工具, 幂等可重跑)。

用法: python script/extract_i18n.py [--dry-run]

做三件事:
1. 解析 ui_layout.cfg (S 表达式), 为用户可见文本生成稳定 key:
   panel 标题   -> ui.<panelId>.title
   group 标题   -> ui.<panelId>.g<gi>.title          (gi = panel 内扁平序号)
   item 文本    -> ui.<panelId>.<target>.label/.desc/.tn/.val
                   无 :target 的 item -> ui.<panelId>.g<gi>i<ii>....
2. 中文文本追加进 lang/zh.properties (已存在的 key 跳过, 不覆盖人工修订)
3. ui_layout.cfg 原地替换为 "@key" 引用 (保持其余字节不变)

不抽取: :unit / :source / :desc-img / :hotkey / 空 label / 非 info 的 :value。
"""
import io
import re
import sys

ROOT = __file__.rsplit('/', 2)[0] if '/' in __file__ else '.'
CFG = ROOT + '/ui_layout.cfg'
ZH = ROOT + '/lang/zh.properties'

DRY = '--dry-run' in sys.argv


# ---------- S 表达式 tokenizer (记录字面量 span 供原地改写) ----------

class Tok:
    def __init__(self, kind, val, start, end):
        self.kind, self.val, self.start, self.end = kind, val, start, end


def tokenize(src):
    toks, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        if c in ' \t\r\n':
            i += 1
        elif c == '(':
            toks.append(Tok('(', '(', i, i + 1)); i += 1
        elif c == ')':
            toks.append(Tok(')', ')', i, i + 1)); i += 1
        elif c == '"':
            j, buf = i + 1, []
            while j < n and src[j] != '"':
                if src[j] == '\\' and j + 1 < n:
                    buf.append(src[j + 1]); j += 2
                else:
                    buf.append(src[j]); j += 1
            toks.append(Tok('str', ''.join(buf), i, j + 1)); i = j + 1
        else:
            j = i
            while j < n and src[j] not in ' \t\r\n()"':
                j += 1
            toks.append(Tok('sym', src[i:j], i, j)); i = j
    return toks


class Node:
    """(head child...) 列表节点; 叶子为 Tok"""

    def __init__(self):
        self.items = []

    def syms(self):
        return [t.val for t in self.items if t.kind == 'sym']


def parse(toks):
    pos = [0]
    roots = []

    def node():
        nd = Node()
        while pos[0] < len(toks):
            t = toks[pos[0]]
            if t.kind == '(':
                pos[0] += 1
                nd.items.append(node())
            elif t.kind == ')':
                pos[0] += 1
                return nd
            else:
                pos[0] += 1
                nd.items.append(t)
        return nd

    while pos[0] < len(toks):
        t = toks[pos[0]]
        if t.kind == '(':
            pos[0] += 1
            roots.append(node())
        else:
            pos[0] += 1
    return roots


# ---------- key 生成与遍历 ----------

def kw(node, name):
    """item/group 节点内取 :name 关键字的值 token(跳过嵌套 Node; 值可为 str 或裸 sym 如 :type info)"""
    for k, tok in enumerate(node.items):
        if hasattr(tok, 'kind') and tok.kind == 'sym' and tok.val == name and k + 1 < len(node.items):
            v = node.items[k + 1]
            if hasattr(v, 'kind') and v.kind in ('str', 'sym'):
                return v
    return None


def has_text(tok):
    return tok is not None and tok.val.strip() != '' and not tok.val.startswith('@')


def prop_escape(s):
    return s.replace('\\', '\\\\').replace('\n', '\\n').replace('\t', '\\t')


def main():
    with io.open(CFG, 'r', encoding='utf-8') as f:
        src = f.read()
    toks = tokenize(src)
    roots = parse(toks)

    # 已有 zh keys (不覆盖)
    zh_lines = io.open(ZH, 'r', encoding='utf-8').read().split('\n')
    zh_keys = set(l.split('=', 1)[0].strip() for l in zh_lines if '=' in l and not l.startswith('#'))

    new_pairs = []   # (key, value) 顺序保序(仅 properties 新增项)
    spans = []       # (start, end, key) 待替换的字面量 span (含引号)

    def emit(key, tok):
        # span 总是登记(重跑/回滚 cfg 后仍需替换); properties 仅新 key 追加, 不覆盖人工修订
        if key not in zh_keys:
            new_pairs.append((key, prop_escape(tok.val)))
            zh_keys.add(key)
        if not any(s[2] == key for s in spans):
            spans.append((tok.start, tok.end, key))

    panels = [n for n in roots if n.items and getattr(n.items[0], 'val', '') == 'panel']
    for pi, panel in enumerate(panels):
        pid = kw(panel, ':id')
        pid = pid.val if pid else 'p%d' % pi
        if has_text(panel.items[1]) if len(panel.items) > 1 else False:
            emit('ui.%s.title' % pid, panel.items[1])

        gi = [0]  # panel 内扁平 group 序号

        def walk(parent):
            for it in parent.items:
                if not isinstance(it, Node) or not it.items:
                    continue
                head = getattr(it.items[0], 'val', '')
                if head == 'group':
                    g = gi[0]; gi[0] += 1
                    if len(it.items) > 1 and has_text(it.items[1]):
                        emit('ui.%s.g%d.title' % (pid, g), it.items[1])
                    walk_group_items(it, g)
                    walk(it)  # 嵌套 group 继续编号
                # item 由 walk_group_items 处理, 此处不再递归 item

        def walk_group_items(group, g):
            ii = 0
            for it in group.items:
                if not isinstance(it, Node) or not it.items:
                    continue
                if getattr(it.items[0], 'val', '') != 'item':
                    continue
                target = kw(it, ':target')
                # key 只允许字母数字: properties 语法 key 在首个空格截断,
                # 含空格/符号的 target(如 "getWingSweep * 100")必须 slug 化(getWingSweepx100)
                tslug = target.val.replace('*', 'x') if target else ''
                tslug = ''.join(ch for ch in tslug if ch.isalnum())
                base = ('ui.%s.%s' % (pid, tslug)) if target else ('ui.%s.g%di%d' % (pid, g, ii))
                ii += 1
                if len(it.items) > 1 and has_text(it.items[1]):
                    emit(base + '.label', it.items[1])
                d = kw(it, ':desc')
                if has_text(d):
                    emit(base + '.desc', d)
                tn = kw(it, ':target-name')
                if has_text(tn):
                    emit(base + '.tn', tn)
                # 仅 info 类型长正文 value 抽取
                ty = kw(it, ':type')
                if ty is not None and ty.val == 'info':
                    v = kw(it, ':value')
                    if has_text(v):
                        emit(base + '.val', v)

        walk(panel)

    if not new_pairs and not spans:
        print('[extract] 无文本可处理 (已全部 @key 化)')
        return

    # 从后往前替换 span, 避免偏移漂移
    out = src
    for start, end, key in sorted(spans, reverse=True):
        out = out[:start] + '"@%s"' % key + out[end:]

    if DRY:
        print('[dry-run] 新增 %d 条:' % len(new_pairs))
        for k, v in new_pairs[:20]:
            print('  %s = %s' % (k, v[:50]))
        print('  ... (共 %d)' % len(new_pairs))
        return

    with io.open(CFG, 'w', encoding='utf-8', newline='\n') as f:
        f.write(out)
    with io.open(ZH, 'a', encoding='utf-8', newline='\n') as f:
        f.write('\n# ===== ui_layout.cfg 自动抽取 (extract_i18n.py, 勿手改此区段) =====\n')
        for k, v in new_pairs:
            f.write('%s=%s\n' % (k, v))
    print('[extract] 抽取 %d 条 -> zh.properties, ui_layout.cfg 已 @key 化' % len(new_pairs))


if __name__ == '__main__':
    main()
