#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""VoidMei 统一构建脚本 —— 唯一构建入口。

只依赖 Python 3.8+ 标准库; cmd / PowerShell / git-bash / CI 行为一致,
无 shell 环境差异问题 (PATH/CRLF/工具集版本)。外部命令仅依赖 JDK。

用法:
  python script/build.py compile            编译 src/ -> bin/
  python script/build.py test [suite]       编译并运行单元测试
                                           (suite: atmosphere|piston|spitfire|tempest|visibility|voicepack|all)
  python script/build.py jar                打 VoidMei.jar (版本号注入 MANIFEST)
  python script/build.py exe                launch4j 打 VoidMei.exe (版本号注入 EXE 资源)
  python script/build.py dist               jar+exe 后组装完整分发包 -> dist/VoidMei_v*.zip
  python script/build.py fmdata             从 War Thunder 客户端解包并裁剪 FM 数据 (游戏版本更新后执行)
  python script/build.py clean              清理 bin/ build/ dist/

环境变量:
  VOIDMEI_VERSION    版本号 (CI 从 git tag 注入, 如 1.590; 缺省 dev)
  VOIDMEI_FMDATA_ZIP dist 使用的现成裁剪版 data zip (CI 从 data prerelease 下载; 缺省用项目内 ./data)
  VOIDMEI_LAUNCH4J   launch4j 可执行文件或 launch4j.jar 的路径 (缺省从 PATH 及常见位置查找)
  WT_GAME_DIR        fmdata 子命令: War Thunder 游戏安装目录
                     (缺省自动探测: 注册表 > Steam 库 > 常见路径, 命中后缓存 .wt_game_dir)
  VOIDMEI_WT_EXT_CLI fmdata 子命令: wt_ext_cli 可执行文件路径 (缺省自动探测)
"""

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
BIN = ROOT / "bin"
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


def copytree(src, dst):
    shutil.copytree(src, dst, dirs_exist_ok=True)


def sha256_of(path):
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def zip_tree(src_dir, zip_path, arc_root):
    """把 src_dir 整棵目录打包进 zip, zip 内顶层目录名为 arc_root。

    保留文件原始字节 (不做行尾转换); Windows 路径分隔由 zipfile 自动归一为 /。
    """
    src_dir = Path(src_dir)
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
        files = sorted(p for p in src_dir.rglob("*") if p.is_file())
        for p in files:
            zf.write(p, str(Path(arc_root) / p.relative_to(src_dir)))


# ---------- compile: 编译 ----------
def cmd_compile():
    log("编译 src/ -> bin/ ...")
    rmtree(BIN)
    BIN.mkdir(parents=True)
    sources = [str(p).replace("\\", "/") for p in (ROOT / "src").rglob("*.java")]
    if not sources:
        err("src/ 下没有找到 java 源文件")
        sys.exit(1)
    listfile = BUILD / "sources.txt"
    BUILD.mkdir(exist_ok=True)
    listfile.write_text("\n".join(sources), encoding="utf-8")
    run(["javac", "-encoding", "UTF-8", "-d", "bin", "-classpath", "dep/*", "@" + str(listfile)])
    log("编译完成")


def ensure_compiled():
    if not (BIN / "prog").is_dir():
        cmd_compile()


# ---------- run: 本地运行 ----------
def cmd_run():
    # 开发循环: 编译后从 bin/ 直跑, 免打 jar。
    # 入口与 MANIFEST 一致走 prog.Launcher —— 它在 AWT 加载前设置 GPU 兼容属性;
    # classpath 模式下 MANIFEST 不生效, 版本号显示 dev (Application.readVersion 回退, 符合预期)
    ensure_compiled()
    cp = os.pathsep.join(["bin", "dep/*"])
    log("运行 VoidMei (classpath 模式, 入口 prog.Launcher) ...")
    run(["java", "-classpath", cp, "prog.Launcher"])


# ---------- test: 单元测试 ----------
SUITES = [
    ("atmosphere", "AtmosphereModel Tests", "TestAtmosphereModel"),
    ("piston", "PistonPowerModel Tests", "TestPistonPowerModel"),
    ("visibility", "VisibilityExpressionEvaluator Tests", "TestVisibilityExpressionEvaluator"),
    ("voicepack", "VoicePackConfig Tests", "TestVoicePackConfig"),
    ("fmstore", "FM Manager Store Tests", "TestFMStore"),
    ("fmpaths", "FM Data Paths Tests", "TestFMDataPaths"),
    ("fmhandle", "FM Handle Tests", "TestFMHandle"),
]
SUITE_ALIASES = {"atm": "atmosphere", "power": "piston", "vis": "visibility", "voice": "voicepack"}
# 真机 FM 端到端验证套件 (用项目内 data/ 的真实 blkx 跑功率曲线核对): 名 -> (label, 测试类, 机型)
# data/ 不进 git (CI 无数据时自动跳过, 本地跑过 fmdata 即有)
FM_SUITES = {
    "spitfire": ("Spitfire F24 Tests", "TestSpitfireF24Power", "spitfire_f24"),
    "tempest": ("Tempest Mk V Tests", "TestTempestMk5Power", "tempest_mkv"),
}
FM_SUITE_ALIASES = {"f24": "spitfire", "mkv": "tempest"}


def cmd_test(suite="all"):
    ensure_compiled()
    log("编译测试代码 test/ ...")
    tests = [str(p).replace("\\", "/") for p in (ROOT / "test").glob("*.java")]
    run(["javac", "-encoding", "UTF-8", "-d", "bin", "-classpath", "bin"] + tests)

    passed = failed = 0

    def run_one(label, cls, extra_args=()):
        nonlocal passed, failed
        print("Running %s ..." % label)
        if run_ok(["java", "-classpath", "bin", cls] + list(extra_args)):
            print("%s: PASSED" % label)
            passed += 1
        else:
            print("%s: FAILED" % label, file=sys.stderr)
            failed += 1

    def run_fm_test(label, cls, plane):
        # 真机 FM 验证: 文件取自项目内 data/ (自己解包的 fmdata), 缺失则跳过 (不计失败)
        nonlocal passed, failed
        fm_root = DATA / "aces" / "gamedata" / "flightmodels"
        central = fm_root / (plane + ".blkx")
        fmfile = fm_root / "fm" / (plane + ".blkx")
        if not (central.is_file() and fmfile.is_file()):
            warn("跳过 %s: 项目内 data/ 缺少 %s 的 FM 文件 (先运行 python script/build.py fmdata)" % (label, plane))
            return
        run_one(label, cls, ["--central", central, "--fm", fmfile])

    suite = SUITE_ALIASES.get(suite, suite)
    suite = FM_SUITE_ALIASES.get(suite, suite)
    if suite == "all":
        for _, label, cls in SUITES:
            run_one(label, cls)
        for label, cls, plane in FM_SUITES.values():
            run_fm_test(label, cls, plane)
    elif suite in dict((s[0], s) for s in SUITES):
        _, label, cls = next(s for s in SUITES if s[0] == suite)
        run_one(label, cls)
    elif suite in FM_SUITES:
        label, cls, plane = FM_SUITES[suite]
        run_fm_test(label, cls, plane)
    else:
        err("未知测试套件: %s (可选: all/%s/%s)" % (
            suite, "/".join(s[0] for s in SUITES), "/".join(sorted(FM_SUITES))))
        sys.exit(1)

    print("")
    print("Test suites passed: %d  failed: %d" % (passed, failed))
    if failed:
        err("存在失败的测试!")
        sys.exit(1)
    log("全部测试通过")


# ---------- jar: 打包 (版本号注入 MANIFEST) ----------
def cmd_jar():
    ensure_compiled()
    # 生成带版本号的 MANIFEST 副本: Application.readVersion() 运行时从
    # Implementation-Version 读取版本号 (本地未打 jar 直接跑时回退 "dev")
    # 注意: MANIFEST 规范要求末尾换行, 追加前先确保
    data = (ROOT / "MANIFEST.MF").read_bytes()
    if data and not data.endswith(b"\n"):
        data += b"\n"
    data += ("Implementation-Version: %s\n" % VERSION).encode("utf-8")
    gen = BUILD / "MANIFEST.gen"
    gen.write_bytes(data)
    run(["jar", "cfm", "VoidMei.jar", gen, "-C", "bin", "."])
    log("打包完成: VoidMei.jar (版本: %s)" % VERSION)


# ---------- exe: launch4j 打包 ----------
def find_launch4j():
    """查找 launch4j: 环境变量 -> PATH -> Windows 常见安装位置。"""
    env = os.environ.get("VOIDMEI_LAUNCH4J", "")
    if env and Path(env).exists():
        return Path(env)
    which = shutil.which("launch4jc")
    if which:
        return Path(which)
    for c in [r"C:\Program Files (x86)\Launch4j\launch4jc.exe",
              r"C:\Program Files\Launch4j\launch4jc.exe",
              "/usr/local/bin/launch4j",
              "/opt/launch4j/launch4j"]:
        if Path(c).exists():
            return Path(c)
    return None


def cmd_exe():
    if not (ROOT / "VoidMei.jar").is_file():
        cmd_jar()
    l4j = find_launch4j()
    if not l4j:
        warn("未找到 launch4j (可设置 VOIDMEI_LAUNCH4J 指向 launch4jc 或 launch4j.jar), 跳过 EXE 生成")
        warn("CI 环境会自动下载 launch4j Linux 发行包; 本地缺失仅影响 exe, 不影响 jar")
        return
    # EXE 版本资源: fileVersion 必须四段式 (1.590 -> 1.590.0.0); dev 版用 0.0.0.0
    v4 = VERSION + ".0.0" if re.fullmatch(r"[0-9]+(\.[0-9]+)?", VERSION) else "0.0.0.0"
    # 由模板生成临时配置 (生成到 script/ 下, 保证 icon 等相对路径与原配置一致)
    xml = (SCRIPT / "voidmeil4j.xml").read_text(encoding="utf-8")
    xml = xml.replace("@VERSION@", VERSION).replace("@VERSION4@", v4)
    cfg = SCRIPT / "voidmeil4j.gen.xml"
    cfg.write_text(xml, encoding="utf-8")
    if l4j.suffix == ".jar":
        # headless: CI (ubuntu, 无显示) 下避免 AWT 初始化失败
        run(["java", "-Djava.awt.headless=true", "-jar", l4j, cfg])
    else:
        run([l4j, cfg])
    log("EXE 打包完成: VoidMei.exe (版本: %s)" % VERSION)


# ---------- dist: 组装完整分发包 ----------
def stage_data(stage_dir):
    """data 源解析: VOIDMEI_FMDATA_ZIP (CI, 优先) -> 项目内 ./data (本地默认)。

    程序只读 data/aces/version 与 data/aces/gamedata/flightmodels 子树。
    """
    data_dir = Path(stage_dir) / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
    zip_env = os.environ.get("VOIDMEI_FMDATA_ZIP", "")
    if zip_env:
        src = Path(zip_env).resolve()
        if not src.is_file():
            err("VOIDMEI_FMDATA_ZIP 不存在: %s" % src)
            sys.exit(1)
        with zipfile.ZipFile(src) as zf:
            zf.extractall(str(stage_dir))  # zip 顶层为 data/
        if not (data_dir / "aces" / "gamedata" / "flightmodels").is_dir():
            err("data zip 内容异常: 缺少 data/aces/gamedata/flightmodels")
            sys.exit(1)
    elif (DATA / "aces" / "gamedata" / "flightmodels").is_dir():
        # 本地: 从项目内 ./data 裁剪
        (data_dir / "aces" / "gamedata").mkdir(parents=True, exist_ok=True)
        ver = DATA / "aces" / "version"
        if ver.is_file():
            shutil.copy2(ver, data_dir / "aces" / "version")
        copytree(DATA / "aces" / "gamedata" / "flightmodels",
                 data_dir / "aces" / "gamedata" / "flightmodels")
    else:
        err("缺少 FM 数据: 请先运行 python script/build.py fmdata 生成项目内 data/, 或设置 VOIDMEI_FMDATA_ZIP")
        sys.exit(1)


def git_short():
    r = capture(["git", "rev-parse", "--short", "HEAD"])
    return r.strip() or "nogit"


def cmd_dist():
    cmd_jar()
    cmd_exe()

    # zip 命名: 正式版 VoidMei_v1.590.zip; 本地 dev 版带 commit hash 与日期
    if VERSION == "dev":
        zipname = "VoidMei_dev_%s_%s" % (git_short(), datetime.now().strftime("%Y%m%d"))
    else:
        zipname = "VoidMei_v%s" % VERSION

    stage = DIST / "stage" / zipname
    rmtree(stage)
    stage.mkdir(parents=True)

    log("组装分发包: %s ..." % zipname)
    # --- 程序本体 ---
    shutil.copy2(ROOT / "VoidMei.jar", stage / "VoidMei.jar")
    shutil.copy2(ROOT / "VoidMei.bat", stage / "VoidMei.bat")
    if (ROOT / "VoidMei.exe").is_file():
        shutil.copy2(ROOT / "VoidMei.exe", stage / "VoidMei.exe")
    # --- 依赖与资源 (白名单复制, 天然排除 records/ config/ ui_layout.user.cfg 等用户数据) ---
    for d in ("dep", "fonts", "image", "voice"):
        copytree(ROOT / d, stage / d)
    (stage / "lang").mkdir()
    shutil.copy2(ROOT / "lang" / "cur.properties", stage / "lang" / "cur.properties")
    shutil.copy2(ROOT / "ui_layout.cfg", stage / "ui_layout.cfg")
    for txt in ("使用说明.txt", "快速使用说明.txt", "更新日志.txt"):
        if (ROOT / txt).is_file():
            shutil.copy2(ROOT / txt, stage / txt)
    # --- FM 数据 (裁剪版) ---
    stage_data(stage)

    # 提示: 本地 fonts/ 可能含未入库的商业字体 (CI 构建的包不含)
    if (ROOT / "fonts" / "DIN Pro 400.otf").is_file():
        warn("本地 fonts/ 含 DIN Pro 400.otf (商业字体, 未入库), 本地打的包将携带该字体; 正式发布请使用 CI 产物")

    zip_path = DIST / (zipname + ".zip")
    zip_tree(stage, zip_path, zipname)
    rmtree(DIST / "stage")
    # sha256 文件与 sha256sum 命令输出格式一致 ("<hash>  <name>")
    (DIST / (zipname + ".zip.sha256")).write_text(
        "%s  %s\n" % (sha256_of(zip_path), zipname + ".zip"), encoding="utf-8")
    log("分发包完成: dist/%s.zip (%.1f MB)" % (zipname, zip_path.stat().st_size / (1 << 20)))


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


def cmd_fmdata():
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

    # wt_ext_cli 解包 (仅 flightmodels 子树, 数秒完成)
    # --format BlkText: 输出 "名字:类型 = 值" 文本格式; --blk_extension blkx: 程序主加载路径
    # (Controller/DrawFrame) 硬编码查找 .blkx 扩展名, 缺省的 .blk 不兼容
    # --folder: 只解 vromfs 内的 gamedata/flightmodels 子树, 实际输出到
    # <output>/aces.vromfs.bin_u/gamedata/flightmodels
    log("wt_ext_cli 解包 flightmodels 子树 ...")
    unpack_tmp = BUILD / "fmdata_unpack"
    rmtree(unpack_tmp)
    run([wtcli, "unpack_vromf", "-i", vromfs, "-o", unpack_tmp,
         "--format", "BlkText", "--blk_extension", "blkx",
         "--folder", "gamedata/flightmodels", "--continue", "Quiet"])
    unpack_root = unpack_tmp / "aces.vromfs.bin_u"

    fm_dir = unpack_root / "gamedata" / "flightmodels"
    if not fm_dir.is_dir():
        err("解包结果异常: 缺少 gamedata/flightmodels "
            "(wt_ext_cli 版本可能滞后于游戏格式, 请检查其 releases)")
        sys.exit(1)

    # 裁剪更新项目内 ./data —— 单一来源, 本地即刻可用
    log("裁剪并更新项目内 data/ (仅 version + flightmodels, 程序只读这两处) ...")
    target = DATA / "aces" / "gamedata" / "flightmodels"
    rmtree(target)
    target.mkdir(parents=True)
    copytree(fm_dir, target)

    # 生成 version 文件 (供 Blkx.getVersion() 显示 FM 数据版本)
    # 优先 WT_VERSION 显式指定; 缺省用 wt_ext_cli vromf_version 从 vromfs 二进制头读取
    wtver = os.environ.get("WT_VERSION", "")
    if not wtver:
        out = capture([wtcli, "vromf_version", "-i", vromfs, "-f", "plain"])
        wtver = out.strip().splitlines()[0].strip() if out.strip() else ""
    if wtver:
        (DATA / "aces" / "version").write_text(wtver + "\n", encoding="utf-8")
    else:
        warn("未读到游戏版本号 (建议设置 WT_VERSION 显式指定), data/aces/version 未生成 (程序可容错运行)")

    # 统计并产出上传用的 data zip + manifest
    blkx_count = sum(1 for _ in target.rglob("*.blkx"))
    file_count = sum(1 for _ in DATA.rglob("*") if _.is_file())
    total_bytes = sum(f.stat().st_size for f in DATA.rglob("*") if f.is_file())
    date = datetime.now().strftime("%Y%m%d")

    DIST.mkdir(exist_ok=True)
    for old in DIST.glob("VoidMei_data_*.zip"):
        old.unlink()
    # zip 名只带游戏版本号 (同版本重跑直接覆盖, 无需日期; 日期记录在 manifest)
    data_zip = DIST / ("VoidMei_data_%s.zip" % (wtver or "unknown"))
    zip_tree(DATA, data_zip, "data")

    manifest = {
        "wt_version": wtver or "unknown",
        "date": date,
        "blkx_count": blkx_count,
        "file_count": file_count,
        "total_bytes": total_bytes,
        "zip": data_zip.name,
        "sha256": sha256_of(data_zip),
    }
    (DIST / "data_manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    rmtree(unpack_tmp)
    log("fmdata 更新完成: %s (%.1f MB, %d 个 blkx)" % (
        data_zip, data_zip.stat().st_size / (1 << 20), blkx_count))
    log("上传到 data 存储 (供 CI 组包): gh release upload data \"%s\" dist/data_manifest.json --clobber" % data_zip.name)


# ---------- clean ----------
def cmd_clean():
    for d in (BIN, BUILD, DIST):
        rmtree(d)
    log("已清理 bin/ build/ dist/")


def main():
    import argparse
    parser = argparse.ArgumentParser(prog="build.py", description="VoidMei 统一构建脚本")
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("compile", help="编译 src/ -> bin/")
    sub.add_parser("run", help="编译并本地运行 (classpath 模式, 版本号 dev)")
    p_test = sub.add_parser("test", help="编译并运行单元测试")
    p_test.add_argument("suite", nargs="?", default="all")
    sub.add_parser("jar", help="打 VoidMei.jar (版本号注入 MANIFEST)")
    sub.add_parser("exe", help="launch4j 打 VoidMei.exe")
    sub.add_parser("dist", help="组装完整分发包")
    sub.add_parser("fmdata", help="解包并裁剪 FM 数据")
    sub.add_parser("clean", help="清理构建产物")
    args = parser.parse_args()

    if args.cmd == "compile":
        cmd_compile()
    elif args.cmd == "run":
        cmd_run()
    elif args.cmd == "test":
        cmd_test(args.suite)
    elif args.cmd == "jar":
        cmd_jar()
    elif args.cmd == "exe":
        cmd_exe()
    elif args.cmd == "dist":
        cmd_dist()
    elif args.cmd == "fmdata":
        cmd_fmdata()
    elif args.cmd == "clean":
        cmd_clean()


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as e:
        err("命令失败 (exit %s): %s" % (e.returncode, " ".join(str(c) for c in e.cmd)))
        sys.exit(1)
