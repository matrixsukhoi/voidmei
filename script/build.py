#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""VoidMei 统一构建脚本 —— 唯一构建入口。

只依赖 Python 3.8+ 标准库; cmd / PowerShell / git-bash / CI 行为一致,
无 shell 环境差异问题 (PATH/CRLF/工具集版本)。外部命令依赖 Node/pnpm
(web 前端) 与 cargo (Rust 工具链); fmdata 另需 wt_ext_cli。

用法:
  python script/build.py rustdist   rust 构建链后组装分发包 -> dist/VoidMei_Rust_*.zip
  python script/build.py fmdata     从 War Thunder 客户端解包 JSON 版 FM 数据 (唯一数据管道)
  python script/build.py web        web 前端构建 (pnpm → crates/webui/web/dist)
  python script/build.py rust       rust 构建链 (web 前端 + cargo release → voidmei.exe)
  python script/build.py clean      清理 build/ dist/

测试请直接用 cargo: cargo test --workspace

环境变量:
  VOIDMEI_VERSION     版本号 (CI 从 git tag 注入, 如 1.590; 缺省 dev)
  VOIDMEI_RUSTDATA_ZIP rustdist 使用的 JSON 版 data zip (CI 组包用; 缺省用项目内 ./data 的 .json)
  WT_GAME_DIR         fmdata 子命令: War Thunder 游戏安装目录
                     (缺省自动探测: 注册表 > Steam 库 > 常见路径, 命中后缓存 .wt_game_dir)
  VOIDMEI_WT_EXT_CLI  fmdata 子命令: wt_ext_cli 可执行文件路径 (缺省自动探测)
"""

import fnmatch
import json
import os
import re
import shutil
import subprocess
import sys
import zipfile
from datetime import datetime
from pathlib import Path

# Windows 控制台 (GBK codepage) 下防止中文输出乱码/报错
for _s in (sys.stdout, sys.stderr):
    try:
        if _s.encoding and _s.encoding.lower() not in ("utf-8", "utf8"):
            _s.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

ROOT = Path(__file__).resolve().parent.parent
os.chdir(ROOT)  # 所有相对路径以项目根为基准 (repo 即工作区)

SCRIPT = ROOT / "script"
BUILD = ROOT / "build"
DIST = ROOT / "dist"
DATA = ROOT / "data"

VERSION = os.environ.get("VOIDMEI_VERSION", "dev")
GAME_DIR_CACHE = ROOT / ".wt_game_dir"


def log(msg):  print("[build] " + msg)
def warn(msg): print("[warn ] " + msg, file=sys.stderr)
def err(msg):  print("[error] " + msg, file=sys.stderr)


def run(cmd, **kw):
    """subprocess.run 封装: 失败即终止 (等价 bash set -e)。"""
    cmd = [str(c) for c in cmd]
    return subprocess.run(cmd, check=True, **kw)


def run_ok(cmd):
    """静默运行, 返回是否成功 (探测类调用)。"""
    try:
        return subprocess.run([str(c) for c in cmd], capture_output=True).returncode == 0
    except Exception:
        return False


def capture(cmd):
    """捕获 stdout (utf-8 容错), 失败返回 ''。"""
    try:
        r = subprocess.run([str(c) for c in cmd], capture_output=True)
        return r.stdout.decode("utf-8", errors="replace")
    except Exception:
        return ""


def rmtree(path):
    if Path(path).exists():
        shutil.rmtree(path)


def copytree(src, dst, ignore=None):
    shutil.copytree(src, dst, dirs_exist_ok=True, ignore=ignore)


def sha256_of(path):
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def zip_tree(src_dir, zip_path, arc_root, exclude=()):
    """把 src_dir 整棵目录打包进 zip, zip 内顶层目录名为 arc_root。

    保留文件原始字节 (不做行尾转换); Windows 路径分隔由 zipfile 自动归一为 /。
    exclude 为 fnmatch 模式元组 (如 ("*.blkx",)), 命中文件名的文件不进 zip —
    data/ 可能残留 Java 版 blkx, 打 zip 时按需排除。
    """
    src_dir = Path(src_dir)
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
        files = sorted(p for p in src_dir.rglob("*") if p.is_file()
                       and not any(fnmatch.fnmatch(p.name, pat) for pat in exclude))
        for p in files:
            zf.write(p, str(Path(arc_root) / p.relative_to(src_dir)))


# ---------- rustdist: 组装分发包 ----------
def stage_data(stage_dir):
    """data 源解析: VOIDMEI_RUSTDATA_ZIP (CI, 优先) -> 项目内 ./data (本地默认)。

    FM 数据为 JSON 格式; 程序只读 data/aces/version 与
    data/aces/gamedata/flightmodels 子树。Java 版遗留的 .blkx 就地剔除, 不进包。
    """
    data_dir = Path(stage_dir) / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
    zip_env = "VOIDMEI_RUSTDATA_ZIP"
    zip_val = os.environ.get(zip_env, "")
    if zip_val:
        src = Path(zip_val).resolve()
        if not src.is_file():
            err("%s 不存在: %s" % (zip_env, src))
            sys.exit(1)
        with zipfile.ZipFile(src) as zf:
            zf.extractall(str(stage_dir))  # zip 顶层为 data/
        fm_root = data_dir / "aces" / "gamedata" / "flightmodels"
        if not fm_root.is_dir():
            err("data zip 内容异常: 缺少 data/aces/gamedata/flightmodels")
            sys.exit(1)
        # 防御: zip 若混入 blkx (Java 版遗留/上传错了 zip), 就地剔除保证包内格式纯净
        for p in fm_root.rglob("*.blkx"):
            if p.is_file():
                p.unlink()
    elif (DATA / "aces" / "gamedata" / "flightmodels").is_dir():
        # 本地: 从项目内 ./data 裁剪 (剔除残留的 blkx)
        (data_dir / "aces" / "gamedata").mkdir(parents=True, exist_ok=True)
        ver = DATA / "aces" / "version"
        if ver.is_file():
            shutil.copy2(ver, data_dir / "aces" / "version")
        copytree(DATA / "aces" / "gamedata" / "flightmodels",
                 data_dir / "aces" / "gamedata" / "flightmodels",
                 ignore=shutil.ignore_patterns("*.blkx"))
    else:
        err("缺少 FM 数据: 请先运行 python script/build.py fmdata 生成项目内 data/, 或设置 %s" % zip_env)
        sys.exit(1)
    # 格式就绪校验: json 文件数为 0 说明 fmdata 没跑过
    cnt = sum(1 for p in (data_dir / "aces" / "gamedata" / "flightmodels").rglob("*.json") if p.is_file())
    if cnt == 0:
        err("data 中没有 .json 格式的 FM 文件 (先运行 python script/build.py fmdata)")
        sys.exit(1)


def git_short():
    r = capture(["git", "rev-parse", "--short", "HEAD"])
    return r.strip() or "nogit"


def dist_zip_name(prefix="VoidMei"):
    """分发包命名: 正式版 <prefix>_v1_590.zip (版本号 . 换 _, 与历史分发包一致,
    亦为 Lutra-Fs/scoop-bucket autoupdate 模板 $underscoreVersion 所需); 本地 dev 版带 commit hash 与日期。"""
    if VERSION == "dev":
        return "%s_dev_%s_%s" % (prefix, git_short(), datetime.now().strftime("%Y%m%d"))
    return "%s_v%s" % (prefix, VERSION.replace(".", "_"))


def pack_dist(stage, zipname):
    """分发包收尾: 打 zip + sha256 侧车, 清 staging。"""
    zip_path = DIST / (zipname + ".zip")
    zip_tree(stage, zip_path, zipname)
    rmtree(DIST / "stage")
    # sha256 文件与 sha256sum 命令输出格式一致 ("<hash>  <name>")。
    # 必须显式 newline="\n": Windows 文本模式会把 \n 转 \r\n, sha256sum -c 解析失败
    with open(DIST / (zipname + ".zip.sha256"), "w", encoding="utf-8", newline="\n") as f:
        f.write("%s  %s\n" % (sha256_of(zip_path), zipname + ".zip"))
    log("分发包完成: dist/%s.zip (%.1f MB)" % (zipname, zip_path.stat().st_size / (1 << 20)))


RUST_REL = ROOT / "target" / "release"
RUST_EXE = RUST_REL / "voidmei.exe"
# Rust 渲染 (tiny-skia/swash) 实际使用的字体白名单; DIN Pro 是商业字体, 绝不进包
RUST_DIST_FONTS = ("sarasa-mono-sc-bold.ttf", "sarasa-mono-sc-regular.ttf")


def cmd_rustdist():
    """组装分发包: rust 构建链 → dist/VoidMei_Rust_*.zip (解压即用, 无任何运行时依赖)。

    包形态: voidmei.exe + data/ + fonts(白名单) + image/ + voice/ + 文档;
    前端与语言表已内嵌 exe, 不需 lang/。
    """
    cmd_rust()
    if not RUST_EXE.is_file():
        err("构建产物缺失: %s" % RUST_EXE)
        sys.exit(1)

    zipname = dist_zip_name("VoidMei_Rust")
    stage = DIST / "stage" / zipname
    rmtree(stage)
    stage.mkdir(parents=True)

    log("组装 Rust 分发包: %s ..." % zipname)
    # --- 程序三件套: exe 必需; dll/manifest 存在则拷 (缺失仅告警 — 静态链工具链可无 dll) ---
    shutil.copy2(RUST_EXE, stage / "voidmei.exe")
    for name, why in (("WebView2Loader.dll", "exe 导入表依赖, 目标机缺失会启动失败"),
                      ("voidmei.exe.manifest", "manifest 冗余腿 (主腿已 windres 嵌入 exe)")):
        src = RUST_REL / name
        if src.is_file():
            shutil.copy2(src, stage / name)
        else:
            warn("%s 不在 target/release/, 跳过 (%s)" % (name, why))
    # --- fonts: 白名单两文件 (整目录拷会混入 gitignored 的商业字体) ---
    (stage / "fonts").mkdir()
    for f in RUST_DIST_FONTS:
        if not (ROOT / "fonts" / f).is_file():
            err("fonts/%s 缺失 (Rust 渲染必需)" % f)
            sys.exit(1)
        shutil.copy2(ROOT / "fonts" / f, stage / "fonts" / f)
    # --- 其余资源: 整目录 + 配置 + 文档 (白名单复制, 天然排除用户数据) ---
    copytree(ROOT / "image", stage / "image")
    copytree(ROOT / "voice", stage / "voice")
    for txt in ("使用说明.txt", "快速使用说明.txt", "更新日志.txt"):
        if (ROOT / txt).is_file():
            shutil.copy2(ROOT / txt, stage / txt)
    # --- FM 数据 (裁剪版, JSON 格式) ---
    stage_data(stage)

    pack_dist(stage, zipname)


# ---------- fmdata: 解包并裁剪 FM 数据 ----------
def has_vromfs(d):
    return (Path(d) / "aces.vromfs.bin_gz").is_file() or (Path(d) / "aces.vromfs.bin").is_file()


def find_game_dir():
    """探测 War Thunder 安装目录: 注册表 -> Steam 库 (vdf, 兼容多盘) -> 常见路径。"""
    candidates = []

    # 1. 注册表: Gaijin Net Launcher 记录的游戏工作目录 (仅 Windows; Steam 版无此键, 失败静默)
    if os.name == "nt" and shutil.which("reg"):
        out = capture(["reg", "query",
                       r"HKCU\Software\Gaijin\NetLauncher\Launchers\warthunder",
                       "/v", "WorkingDir"])
        m = re.search(r"WorkingDir\s+REG_SZ\s+(.*)", out, re.IGNORECASE)
        if m and m.group(1).strip():
            candidates.append(Path(m.group(1).strip()))

    # 2. Steam 库: vdf 枚举所有库路径 (含其他盘的 SteamLibrary); 每个入口兜底 common 默认路径
    steam_roots = [r"C:\Program Files (x86)\Steam", r"D:\Steam", r"E:\Steam",
                   r"D:\Program Files (x86)\Steam",
                   Path.home() / ".steam" / "steam",
                   Path.home() / ".local" / "share" / "Steam"]
    for sr in steam_roots:
        sr = Path(sr)
        if not sr.is_dir():
            continue
        vdf = sr / "steamapps" / "libraryfolders.vdf"
        if vdf.is_file():
            text = vdf.read_text(encoding="utf-8", errors="replace")
            for m in re.finditer(r'"path"\s*"([^"]*)"', text):
                # vdf 中盘符路径为双反斜杠转义
                candidates.append(Path(m.group(1).replace("\\\\", "\\")) / "steamapps" / "common" / "War Thunder")
        candidates.append(sr / "steamapps" / "common" / "War Thunder")

    # 3. Gaijin 启动器直装/其他常见位置
    candidates += [Path(p) for p in (
        r"C:\Games\War Thunder", r"D:\Games\War Thunder", r"E:\Games\War Thunder",
        r"C:\Program Files (x86)\War Thunder", r"D:\Program Files (x86)\War Thunder")]

    for d in candidates:
        if has_vromfs(d):
            return d
    return None


def resolve_game_dir():
    """游戏目录解析: WT_GAME_DIR 显式指定 > 上次探测缓存 > 自动探测。

    缓存 .wt_game_dir 由本脚本维护; 旧 bash 版写入的 posix 格式 (/c/...) 直接作废重探。
    """
    explicit = os.environ.get("WT_GAME_DIR", "")
    if explicit:
        return Path(explicit)
    if GAME_DIR_CACHE.is_file():
        cached = GAME_DIR_CACHE.read_text(encoding="utf-8").strip()
        if cached and not cached.startswith("/") and Path(cached).is_dir():
            log("使用缓存的游戏目录: %s (删除 .wt_game_dir 可重新探测)" % cached)
            return Path(cached)
        if cached:
            warn("缓存的游戏目录已失效或为旧格式: %s, 重新探测" % cached)
            GAME_DIR_CACHE.unlink()
    found = find_game_dir()
    if found:
        GAME_DIR_CACHE.write_text(str(found) + "\n", encoding="utf-8")
        log("自动探测到 War Thunder 安装目录: %s (已缓存到 .wt_game_dir)" % found)
        return found
    err("未找到 War Thunder 安装目录, 请显式指定, 例:")
    err(r'  set WT_GAME_DIR=C:\Program Files (x86)\Steam\steamapps\common\War Thunder')
    err("  python script/build.py fmdata")
    sys.exit(1)


def find_wt_ext_cli():
    env = os.environ.get("VOIDMEI_WT_EXT_CLI", "")
    if env and Path(env).exists():
        return Path(env)
    downloads = Path.home() / "Downloads"
    for pat in ("wt_ext_cli-*/wt_ext_cli.exe", "wt_ext_cli-*/wt_ext_cli"):
        for m in downloads.glob(pat):
            return m
    return None


def _fmdata_sources():
    """fmdata 前置: 游戏目录 + vromfs 包 + wt_ext_cli, 失败即退出。"""
    game_dir = resolve_game_dir()
    log("游戏目录: %s" % game_dir)

    # 定位 vromfs 包 (WT 客户端为 gzip 压缩格式 _gz)
    vromfs = None
    for name in ("aces.vromfs.bin_gz", "aces.vromfs.bin"):
        p = game_dir / name
        if p.is_file():
            vromfs = p
            break
    if not vromfs:
        err("在 %s 下未找到 aces.vromfs.bin_gz / aces.vromfs.bin" % game_dir)
        sys.exit(1)

    # 定位 wt_ext_cli 解包工具
    wtcli = find_wt_ext_cli()
    if not wtcli:
        err("未找到 wt_ext_cli (设置 VOIDMEI_WT_EXT_CLI 指向其可执行文件)")
        err("工具主页: https://github.com/Warthunder-Open-Source-Foundation/wt_ext_cli")
        sys.exit(1)
    return vromfs, wtcli


def _unpack_flightmodels(wtcli, vromfs, unpack_tmp, fmt, blk_ext):
    """wt_ext_cli 解包 flightmodels 子树 (仅此子树, 数秒完成), 返回裁剪源目录。

    --folder: 只解 vromfs 内的 gamedata/flightmodels 子树, 实际输出到
    <output>/aces.vromfs.bin_u/gamedata/flightmodels
    """
    log("wt_ext_cli 解包 flightmodels 子树 (%s) ..." % fmt)
    rmtree(unpack_tmp)
    run([wtcli, "unpack_vromf", "-i", vromfs, "-o", unpack_tmp,
         "--format", fmt, "--blk_extension", blk_ext,
         "--folder", "gamedata/flightmodels", "--continue", "Quiet"])
    fm_dir = unpack_tmp / "aces.vromfs.bin_u" / "gamedata" / "flightmodels"
    if not fm_dir.is_dir():
        err("解包结果异常: 缺少 gamedata/flightmodels "
            "(wt_ext_cli 版本可能滞后于游戏格式, 请检查其 releases)")
        sys.exit(1)
    return fm_dir


def _prune_fm_ext(target, ext):
    """按扩展名清理 flightmodels 下旧文件 (data/ 可能残留 Java 版 .blkx, 不能整树删)。"""
    for rel in ("", "fm"):
        d = target / rel if rel else target
        d.mkdir(parents=True, exist_ok=True)
        for f in d.glob("*" + ext):
            f.unlink()


def _copy_fm_files(fm_dir, target, ext):
    """解包产物裁剪拷贝: 根 *<ext> (中央文件) + fm/*<ext> (物理 FM) 两层白名单。

    程序只读这三处 (FMDataPaths 是路径唯一来源): 根目录中央文件 / fm/ 子目录
    物理 FM / aces/version。解包产物其余子树 (weaponpresets/performance/dm/
    exhausteffects/fueldumping, 约 1 万个文件) 程序不读, 不拷入;
    若未来新增读取处, 须同步这里的白名单。
    """
    for rel in ("", "fm"):
        dst_dir = target / rel if rel else target
        dst_dir.mkdir(parents=True, exist_ok=True)
        for f in (fm_dir / rel).glob("*" + ext):
            shutil.copy2(f, dst_dir / f.name)


def _write_version(wtcli, vromfs):
    """生成 data/aces/version (供 FM 数据版本显示)。

    优先 WT_VERSION 显式指定; 缺省用 wt_ext_cli vromf_version 从 vromfs 二进制头读取。
    """
    wtver = os.environ.get("WT_VERSION", "")
    if not wtver:
        out = capture([wtcli, "vromf_version", "-i", vromfs, "-f", "plain"])
        wtver = out.strip().splitlines()[0].strip() if out.strip() else ""
    if wtver:
        (DATA / "aces" / "version").write_text(wtver + "\n", encoding="utf-8")
    else:
        warn("未读到游戏版本号 (建议设置 WT_VERSION 显式指定), data/aces/version 未生成 (程序可容错运行)")
    return wtver


def _pack_data_zip(wtver, count_key, count, exclude, zip_prefix, manifest_name):
    """产出上传用的 data zip + manifest (fmdata 收尾)。

    exclude 排除 blkx (data/ 可能残留 Java 版遗留); zip 名只带游戏版本号
    (同版本重跑直接覆盖, 无需日期; 日期记录在 manifest)。
    """
    files = [p for p in DATA.rglob("*") if p.is_file()
             and not any(fnmatch.fnmatch(p.name, pat) for pat in exclude)]
    file_count = len(files)
    total_bytes = sum(f.stat().st_size for f in files)
    date = datetime.now().strftime("%Y%m%d")

    DIST.mkdir(exist_ok=True)
    for old in DIST.glob(zip_prefix + "_*.zip"):
        old.unlink()
    data_zip = DIST / ("%s_%s.zip" % (zip_prefix, wtver or "unknown"))
    zip_tree(DATA, data_zip, "data", exclude=exclude)

    manifest = {
        "wt_version": wtver or "unknown",
        "date": date,
        count_key: count,
        "file_count": file_count,
        "total_bytes": total_bytes,
        "zip": data_zip.name,
        "sha256": sha256_of(data_zip),
    }
    (DIST / manifest_name).write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return data_zip


def cmd_fmdata():
    """解包并裁剪 FM 数据 (JSON 格式, 唯一数据管道)。

    JSON 为 blk 树 1:1 镜像 (嵌套 object / 同名键合并为数组 / 浮点 f32 最短表示)。
    """
    vromfs, wtcli = _fmdata_sources()
    # --blk_extension json: FMDataPaths 查找 .json 扩展名
    # 刻意不传 --override: 键保持字面量
    unpack_tmp = BUILD / "fmdata_unpack"
    fm_dir = _unpack_flightmodels(wtcli, vromfs, unpack_tmp, "Json", "json")

    log("裁剪并更新项目内 data/ (仅根 json + fm/) ...")
    target = DATA / "aces" / "gamedata" / "flightmodels"
    # 只清 .json (data/ 可能残留 Java 版 .blkx, 整树删会误删)
    _prune_fm_ext(target, ".json")
    _copy_fm_files(fm_dir, target, ".json")
    wtver = _write_version(wtcli, vromfs)

    # data zip 为 json 视图: 排除 blkx 残留
    json_count = sum(1 for _ in target.rglob("*.json"))
    data_zip = _pack_data_zip(wtver, "json_count", json_count, ("*.blkx",),
                              "VoidMei_RustData", "rust_data_manifest.json")

    rmtree(unpack_tmp)
    blkx_left = sum(1 for _ in target.rglob("*.blkx"))
    if blkx_left:
        warn("data/ 尚有 %d 个 .blkx (Java 版遗留, 可手动清理; 打 zip 时自动排除)" % blkx_left)
    log("fmdata 更新完成: %s (%.1f MB, %d 个 json)" % (
        data_zip, data_zip.stat().st_size / (1 << 20), json_count))
    log("上传到 data 存储 (供 CI 组包): gh release upload data \"%s\" dist/rust_data_manifest.json --clobber" % data_zip.name)


# ---------- clean ----------
def cmd_clean():
    for d in (BUILD, DIST):
        rmtree(d)
    log("已清理 build/ dist/ (cargo 产物用 cargo clean)")


def _find_pnpm():
    """pnpm 探测: PATH > corepack (Node 自带; corepack 按 web/package.json 的
    packageManager 字段自动钉版本, 与 CI 一致)。"""
    p = shutil.which("pnpm")
    if p:
        return [p]
    if shutil.which("corepack"):
        return ["corepack", "pnpm"]
    err("未找到 pnpm/corepack (web 前端构建需要 Node 工具链, 见 README.md)")
    raise SystemExit(1)


def cmd_web():
    """web 前端构建: pnpm install + build → web/dist (cargo 编译期被 generate_context! 嵌入)。"""
    web_dir = ROOT / "crates" / "webui" / "web"
    pnpm = _find_pnpm()
    lock = web_dir / "pnpm-lock.yaml"
    install = pnpm + (["install", "--frozen-lockfile"] if lock.exists() else ["install"])
    run(install, cwd=str(web_dir))
    run(pnpm + ["build"], cwd=str(web_dir))
    log("前端 dist 构建完成 (crates/webui/web/dist)")


def cmd_rust():
    """Rust 构建链: 前端 dist → cargo release (voidmei.exe 含 web 壳 + 外部 manifest)。"""
    cmd_web()
    cargo = shutil.which("cargo")
    if not cargo:
        err("未找到 cargo (Rust 工具链, 见 README.md)")
        raise SystemExit(1)
    # rustc 不把 option_env! 读的环境变量计入编译指纹, VOIDMEI_VERSION 变化不会触发重编
    # (exe 内嵌版本号会陈旧)。用版本戳检测变化, 变了就 clean webui (option_env! 所在
    # crate), 下游 voidmei crate 随依赖 hash 连锁重编; 版本不变零代价。
    stamp = BUILD / "rust_version.stamp"
    prev = stamp.read_text(encoding="utf-8").strip() if stamp.is_file() else None
    if prev is not None and prev != VERSION:
        warn("VOIDMEI_VERSION 变化 (%s -> %s), 强制重编 webui 使 exe 内嵌版本号生效" % (prev, VERSION))
        run([cargo, "clean", "--release", "-p", "webui"], cwd=str(ROOT))
    run([cargo, "build", "--release"], cwd=str(ROOT))
    BUILD.mkdir(exist_ok=True)
    stamp.write_text(VERSION + "\n", encoding="utf-8")
    log("Rust 构建完成: %s (注入版本: %s)" % (RUST_EXE, VERSION))


def main():
    import argparse
    parser = argparse.ArgumentParser(prog="build.py", description="VoidMei 统一构建脚本")
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("rustdist", help="组装分发包 (web+cargo 构建 → dist/VoidMei_Rust_*.zip)")
    sub.add_parser("fmdata", help="解包 JSON 版 FM 数据 (wt_ext_cli → data/)")
    sub.add_parser("web", help="web 前端构建 (pnpm → web/dist)")
    sub.add_parser("rust", help="Rust 构建链 (web + cargo release)")
    sub.add_parser("clean", help="清理构建产物 (build/ dist/)")
    args = parser.parse_args()

    if args.cmd == "rustdist":
        cmd_rustdist()
    elif args.cmd == "fmdata":
        cmd_fmdata()
    elif args.cmd == "web":
        cmd_web()
    elif args.cmd == "rust":
        cmd_rust()
    elif args.cmd == "clean":
        cmd_clean()


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as e:
        err("命令失败 (exit %s): %s" % (e.returncode, " ".join(str(c) for c in e.cmd)))
        sys.exit(1)
