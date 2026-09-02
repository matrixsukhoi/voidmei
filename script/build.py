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
  python script/build.py rustdist           rust 构建链后组装 Rust 版分发包 -> dist/VoidMei_Rust_*.zip
  python script/build.py fmdata             从 War Thunder 客户端解包并裁剪 FM 数据 (blkx 文本, Java 端数据源)
  python script/build.py fmdatajson         解包 JSON 版 FM 数据 (Rust 端数据源, 与 blkx 同名并存 data/)
  python script/build.py web                D9 前端构建 (pnpm → rust/crates/vm-webui/web/dist)
  python script/build.py rust               D9 Rust 构建链 (web 前端 + cargo release → voidmei.exe)
  python script/build.py clean              清理 bin/ build/ dist/

环境变量:
  VOIDMEI_VERSION    版本号 (CI 从 git tag 注入, 如 1.590; 缺省 dev)
  VOIDMEI_FMDATA_ZIP dist 使用的现成裁剪版 data zip (CI 从 data prerelease 下载; 缺省用项目内 ./data)
  VOIDMEI_RUSTDATA_ZIP rustdist 使用的 JSON 版 data zip (CI 组 Rust 包用; 缺省用项目内 ./data 的 .json)
  VOIDMEI_LAUNCH4J   launch4j 可执行文件或 launch4j.jar 的路径 (缺省从 PATH 及常见位置查找)
  WT_GAME_DIR        fmdata 子命令: War Thunder 游戏安装目录
                     (缺省自动探测: 注册表 > Steam 库 > 常见路径, 命中后缓存 .wt_game_dir)
  VOIDMEI_WT_EXT_CLI fmdata 子命令: wt_ext_cli 可执行文件路径 (缺省自动探测)
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
    exclude 为 fnmatch 模式元组 (如 ("*.json",)), 命中文件名的文件不进 zip —
    data/ 为 blkx(Java)/json(Rust) 双栖目录, 打 zip 时按消费方过滤另一格式。
    """
    src_dir = Path(src_dir)
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
        files = sorted(p for p in src_dir.rglob("*") if p.is_file()
                       and not any(fnmatch.fnmatch(p.name, pat) for pat in exclude))
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
    # 全量真机 FM 边界普查 (检视反馈): 遍历 fm/ 全部文件断言引擎数/档位极值不触防御护栏,
    # 解析零异常, invalid 仅限空文件。机型参数为 "*" 表示遍历模式 (不传单机型路径)
    "fm-all": ("FM All-Data Boundary Scan", "TestFMAllBoundaries", "*"),
    # blkx 文本变异 fuzz (P6): 种子 bf-109e-4 的真机物理 FM (中等体积且含 PASSPORT 曲线块,
    # 覆盖 getAllplotdata 路径; spitfire_f24 无 PASSPORT 块不适用), data 缺失自动跳过 (同上)
    "fuzz-blkx": ("Blkx Parser Fuzz Tests", "FMParserFuzzer", "bf-109e-4"),
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
        if plane == "*":
            # 遍历模式 (TestFMAllBoundaries): 测试类自己扫描 fm/ 目录, 只检查目录存在
            if not (DATA / "aces" / "gamedata" / "flightmodels" / "fm").is_dir():
                warn("跳过 %s: 项目内 data/ 缺少 fmdata (先运行 python script/build.py fmdata)" % label)
                return
            run_one(label, cls)
            return
        fm_root = DATA / "aces" / "gamedata" / "flightmodels"
        central = fm_root / (plane + ".blkx")
        fmfile = fm_root / "fm" / (plane + ".blkx")
        if not (central.is_file() and fmfile.is_file()):
            warn("跳过 %s: 项目内 data/ 缺少 %s 的 FM 文件 (先运行 python script/build.py fmdata)" % (label, plane))
            return
        run_one(label, cls, ["--central", central, "--fm", fmfile])

    def run_e2e_suite():
        """端到端套件: 核心场景 (正常 / FM 缺失), 复用 e2e_fm.sh 全套编排
        (起 mock -> 翻转 autoStartGameMode 起真实应用 -> 计时 -> 断言 A1~A6 -> 清理还原)"""
        nonlocal passed, failed

        # 端口占用前置检查: 游戏在跑或残留 mock 时跳过整个 e2e (环境不可用, 不计失败)
        import socket
        probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        probe.settimeout(0.5)
        busy = probe.connect_ex(("127.0.0.1", 8111)) == 0
        probe.close()
        if busy:
            warn("跳过 e2e 套件: 端口 8111 已被占用 (游戏在运行或残留 mock 进程?)")
            return

        # duration 定档依据 (检视反馈 "30 秒太长"): 目标行为均在启动后 ~10 秒内发生
        # (死循环特征每 50ms 一周期, 10 秒 = 200 个轮询周期); 下限受断言器归一化约束 ——
        # A1 阈值 2x窗口分钟+0.5, 15 秒窗口才不会把正常的 1 次加载误判为高频。
        scenarios = [
            ("s2_preview_live", 20, "E2E 正常场景 (实时机型供数)"),
            ("s5_missing_fm", 20, "E2E FM 缺失场景 (issue #55 复现)"),
        ]
        for sc, dur, label in scenarios:
            print("Running %s ..." % label)
            if run_ok(["bash", str(ROOT / "script" / "e2e_fm.sh"),
                       "--scenario", sc, "--duration", str(dur)]):
                print("%s: PASSED" % label)
                passed += 1
            else:
                print("%s: FAILED" % label, file=sys.stderr)
                failed += 1

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
    elif suite == "e2e":
        # 端到端套件 (检视反馈接入): 起真实应用连 mock_8111 场景, 断言器 A1~A6 判定。
        # 刻意不进 "test all" —— 需真实 Swing 进程 (CI 无 display 会挂)、单场景数十秒、
        # 且 e2e_fm.sh 会临时翻转 ui_layout.user.cfg 的 autoStartGameMode (退出还原)。
        # 显式 `python script/build.py test e2e` 触发
        run_e2e_suite()
    elif suite in FM_SUITES:
        label, cls, plane = FM_SUITES[suite]
        run_fm_test(label, cls, plane)
    else:
        err("未知测试套件: %s (可选: all/e2e/%s/%s)" % (
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
def stage_data(stage_dir, ext=".blkx", zip_env="VOIDMEI_FMDATA_ZIP"):
    """data 源解析: zip env (CI, 优先) -> 项目内 ./data (本地默认)。

    ext 为本包的 FM 数据格式 (".blkx"=Java dist / ".json"=Rust rustdist):
    本地 data/ 为 blkx/json 双栖 (fmdata 与 fmdatajson 各产一份, 同名不同扩展名),
    按扩展名过滤各取所需; zip env 指向对应格式的 data zip。
    程序只读 data/aces/version 与 data/aces/gamedata/flightmodels 子树。
    """
    other = ".json" if ext == ".blkx" else ".blkx"
    gen_cmd = "fmdatajson" if ext == ".json" else "fmdata"
    data_dir = Path(stage_dir) / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
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
        # 防御: zip 若混入另一格式 (上传错了 zip), 就地剔除保证包内格式纯净
        for p in fm_root.rglob("*" + other):
            if p.is_file():
                p.unlink()
    elif (DATA / "aces" / "gamedata" / "flightmodels").is_dir():
        # 本地: 从项目内 ./data 裁剪 (按 ext 过滤双栖目录)
        (data_dir / "aces" / "gamedata").mkdir(parents=True, exist_ok=True)
        ver = DATA / "aces" / "version"
        if ver.is_file():
            shutil.copy2(ver, data_dir / "aces" / "version")
        copytree(DATA / "aces" / "gamedata" / "flightmodels",
                 data_dir / "aces" / "gamedata" / "flightmodels",
                 ignore=shutil.ignore_patterns("*" + other))
    else:
        err("缺少 FM 数据: 请先运行 python script/build.py %s 生成项目内 data/, 或设置 %s" % (gen_cmd, zip_env))
        sys.exit(1)
    # 格式就绪校验: 目标扩展名文件数为 0 说明对应产线没跑过
    cnt = sum(1 for p in (data_dir / "aces" / "gamedata" / "flightmodels").rglob("*" + ext) if p.is_file())
    if cnt == 0:
        err("data 中没有 %s 格式的 FM 文件 (先运行 python script/build.py %s)" % (ext, gen_cmd))
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
    """分发包收尾: 打 zip + sha256 侧车, 清 staging (Java/Rust 分发包共用)。"""
    zip_path = DIST / (zipname + ".zip")
    zip_tree(stage, zip_path, zipname)
    rmtree(DIST / "stage")
    # sha256 文件与 sha256sum 命令输出格式一致 ("<hash>  <name>")。
    # 必须显式 newline="\n": Windows 文本模式会把 \n 转 \r\n, sha256sum -c 解析失败
    with open(DIST / (zipname + ".zip.sha256"), "w", encoding="utf-8", newline="\n") as f:
        f.write("%s  %s\n" % (sha256_of(zip_path), zipname + ".zip"))
    log("分发包完成: dist/%s.zip (%.1f MB)" % (zipname, zip_path.stat().st_size / (1 << 20)))


def cmd_dist():
    cmd_jar()
    cmd_exe()

    zipname = dist_zip_name()
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

    pack_dist(stage, zipname)


# ---------- rustdist: 组装 Rust 版分发包 ----------
RUST_REL = ROOT / "rust" / "target" / "release"
RUST_EXE = RUST_REL / "voidmei.exe"
# Rust 渲染 (tiny-skia/swash) 实际使用的字体白名单; DIN Pro 是 Java 商业字体, 绝不进包
RUST_DIST_FONTS = ("sarasa-mono-sc-bold.ttf", "sarasa-mono-sc-regular.ttf")


def cmd_rustdist():
    """组装 Rust 版分发包: rust 构建链 → dist/VoidMei_Rust_*.zip (解压即用, 无 JRE 依赖)。

    与 Java dist 同形态 (data/fonts/image/voice/ui_layout.cfg/文档), 差异:
    少 jar/bat/exe/dep/lang (前端与语言表已内嵌 exe), 多 voidmei.exe + WebView2Loader.dll + manifest。
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
            warn("%s 不在 rust/target/release/, 跳过 (%s)" % (name, why))
    # --- fonts: 白名单两文件 (与 Java dist 整目录拷不同, 从根上杜绝商业字体混入) ---
    (stage / "fonts").mkdir()
    for f in RUST_DIST_FONTS:
        if not (ROOT / "fonts" / f).is_file():
            err("fonts/%s 缺失 (Rust 渲染必需)" % f)
            sys.exit(1)
        shutil.copy2(ROOT / "fonts" / f, stage / "fonts" / f)
    # --- 其余资源同 Java dist: 整目录 + 配置 + 文档 (白名单复制, 天然排除用户数据) ---
    copytree(ROOT / "image", stage / "image")
    copytree(ROOT / "voice", stage / "voice")
    shutil.copy2(ROOT / "ui_layout.cfg", stage / "ui_layout.cfg")
    for txt in ("使用说明.txt", "快速使用说明.txt", "更新日志.txt"):
        if (ROOT / txt).is_file():
            shutil.copy2(ROOT / txt, stage / txt)
    # --- FM 数据 (裁剪版, JSON 格式 — Rust 端 FM 数据源, 与 Java 的 blkx 分道) ---
    stage_data(stage, ext=".json", zip_env="VOIDMEI_RUSTDATA_ZIP")

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
    """fmdata/fmdatajson 公共前置: 游戏目录 + vromfs 包 + wt_ext_cli, 失败即退出。"""
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
    """按扩展名清理 flightmodels 下旧文件 (data/ 为 blkx/json 双栖, 不能整树删)。"""
    for rel in ("", "fm"):
        d = target / rel if rel else target
        d.mkdir(parents=True, exist_ok=True)
        for f in d.glob("*" + ext):
            f.unlink()


def _copy_fm_files(fm_dir, target, ext):
    """解包产物裁剪拷贝: 根 *<ext> (中央文件) + fm/*<ext> (物理 FM) 两层白名单。

    程序只读这三处 (FMDataPaths 是路径唯一来源, Java/Rust 同): 根目录中央文件 /
    fm/ 子目录物理 FM / aces/version。解包产物其余子树 (weaponpresets/
    performance/dm/exhausteffects/fueldumping, 约 1 万个文件) 程序不读,
    不拷入; 若未来新增读取处, 须同步这里的白名单。
    """
    for rel in ("", "fm"):
        dst_dir = target / rel if rel else target
        dst_dir.mkdir(parents=True, exist_ok=True)
        for f in (fm_dir / rel).glob("*" + ext):
            shutil.copy2(f, dst_dir / f.name)


def _write_version(wtcli, vromfs):
    """生成 data/aces/version (供 Blkx.getVersion() 显示 FM 数据版本)。

    优先 WT_VERSION 显式指定; 缺省用 wt_ext_cli vromf_version 从 vromfs 二进制头读取。
    fmdata 与 fmdatajson 写同一文件 (同一 vromfs, 幂等)。
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
    """产出上传用的 data zip + manifest (fmdata/fmdatajson 共用收尾)。

    exclude 排除另一格式 (双栖 data/ 打成单格式视图); zip 名只带游戏版本号
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
    """解包并裁剪 FM 数据 (BlkText/blkx 文本格式, Java 端数据源)。"""
    vromfs, wtcli = _fmdata_sources()
    # --format BlkText: 输出 "名字:类型 = 值" 文本格式; --blk_extension blkx: Java 程序
    # 主加载路径 (Controller/DrawFrame) 硬编码查找 .blkx 扩展名, 缺省的 .blk 不兼容
    unpack_tmp = BUILD / "fmdata_unpack"
    fm_dir = _unpack_flightmodels(wtcli, vromfs, unpack_tmp, "BlkText", "blkx")

    # 裁剪更新项目内 ./data —— 单一来源, 本地即刻可用
    log("裁剪并更新项目内 data/ (仅根 blkx + fm/, 程序只读这两处) ...")
    target = DATA / "aces" / "gamedata" / "flightmodels"
    # 只清 .blkx (保住 fmdatajson 产的 .json — data/ 双栖, 整树删会误伤)
    _prune_fm_ext(target, ".blkx")
    _copy_fm_files(fm_dir, target, ".blkx")
    wtver = _write_version(wtcli, vromfs)

    # data zip 为 Java 侧产物: 排除 json 视图
    blkx_count = sum(1 for _ in target.rglob("*.blkx"))
    data_zip = _pack_data_zip(wtver, "blkx_count", blkx_count, ("*.json",),
                              "VoidMei_data", "data_manifest.json")

    rmtree(unpack_tmp)
    log("fmdata 更新完成: %s (%.1f MB, %d 个 blkx)" % (
        data_zip, data_zip.stat().st_size / (1 << 20), blkx_count))
    log("上传到 data 存储 (供 CI 组包): gh release upload data \"%s\" dist/data_manifest.json --clobber" % data_zip.name)


def cmd_fmdatajson():
    """解包并裁剪 FM 数据 (JSON 格式, Rust 端数据源) — 与 blkx 同名并存于 data/。

    JSON 为 blk 树 1:1 镜像 (嵌套 object / 同名键合并为数组 / 浮点 f32 最短表示)。
    """
    vromfs, wtcli = _fmdata_sources()
    # --blk_extension json: Rust 端 FMDataPaths 查找 .json 扩展名
    # 刻意不传 --override: 与 blkx 文本链路同语义 (override: 键保持字面量),
    # 双格式解析结果可位级对拍
    unpack_tmp = BUILD / "fmdata_unpack_json"
    fm_dir = _unpack_flightmodels(wtcli, vromfs, unpack_tmp, "Json", "json")

    log("裁剪并更新项目内 data/ (仅根 json + fm/, 与 blkx 同名并存) ...")
    target = DATA / "aces" / "gamedata" / "flightmodels"
    # 只清 .json (保住 fmdata 产的 .blkx)
    _prune_fm_ext(target, ".json")
    _copy_fm_files(fm_dir, target, ".json")
    wtver = _write_version(wtcli, vromfs)

    # Rust data zip 为 json 视图: 排除 blkx
    json_count = sum(1 for _ in target.rglob("*.json"))
    data_zip = _pack_data_zip(wtver, "json_count", json_count, ("*.blkx",),
                              "VoidMei_RustData", "rust_data_manifest.json")

    rmtree(unpack_tmp)
    log("fmdatajson 更新完成: %s (%.1f MB, %d 个 json)" % (
        data_zip, data_zip.stat().st_size / (1 << 20), json_count))
    log("上传到 data 存储 (供 CI 组 Rust 包): gh release upload data \"%s\" dist/rust_data_manifest.json --clobber" % data_zip.name)



# ---------- clean ----------
def cmd_clean():
    for d in (BIN, BUILD, DIST):
        rmtree(d)
    log("已清理 bin/ build/ dist/")


def _find_pnpm():
    """pnpm 探测: PATH > corepack (Node 自带; corepack 按 web/package.json 的
    packageManager 字段自动钉版本, 与 CI 一致)。"""
    p = shutil.which("pnpm")
    if p:
        return [p]
    if shutil.which("corepack"):
        return ["corepack", "pnpm"]
    err("未找到 pnpm/corepack (D9 web 前端构建需要 Node 工具链, 见 rust/README.md)")
    raise SystemExit(1)


def cmd_web():
    """D9 前端构建: pnpm install + build → web/dist (cargo 编译期被 generate_context! 嵌入)。"""
    web_dir = ROOT / "rust" / "crates" / "vm-webui" / "web"
    pnpm = _find_pnpm()
    lock = web_dir / "pnpm-lock.yaml"
    install = pnpm + (["install", "--frozen-lockfile"] if lock.exists() else ["install"])
    run(install, cwd=str(web_dir))
    run(pnpm + ["build"], cwd=str(web_dir))
    log("前端 dist 构建完成 (rust/crates/vm-webui/web/dist)")


def cmd_rust():
    """D9 Rust 构建链: 前端 dist → cargo release (voidmei.exe 含 web 壳 + 外部 manifest)。"""
    cmd_web()
    cargo = shutil.which("cargo")
    if not cargo:
        err("未找到 cargo (Rust 工具链, 见 rust/README.md)")
        raise SystemExit(1)
    # rustc 不把 option_env! 读的环境变量计入编译指纹, VOIDMEI_VERSION 变化不会触发重编
    # (exe 内嵌版本号会陈旧)。用版本戳检测变化, 变了就 clean vm-webui (option_env! 所在
    # crate), 下游 vm-app 随依赖 hash 连锁重编; 版本不变零代价。
    stamp = BUILD / "rust_version.stamp"
    prev = stamp.read_text(encoding="utf-8").strip() if stamp.is_file() else None
    if prev is not None and prev != VERSION:
        warn("VOIDMEI_VERSION 变化 (%s -> %s), 强制重编 vm-webui 使 exe 内嵌版本号生效" % (prev, VERSION))
        run([cargo, "clean", "--release", "-p", "vm-webui"], cwd=str(ROOT / "rust"))
    run([cargo, "build", "--release"], cwd=str(ROOT / "rust"))
    BUILD.mkdir(exist_ok=True)
    stamp.write_text(VERSION + "\n", encoding="utf-8")
    log("Rust 构建完成: %s (注入版本: %s)" % (RUST_EXE, VERSION))


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
    sub.add_parser("rustdist", help="组装 Rust 版分发包 (web+cargo 构建 → dist/VoidMei_Rust_*.zip)")
    sub.add_parser("fmdata", help="解包并裁剪 FM 数据 (blkx 文本, Java 端)")
    sub.add_parser("fmdatajson", help="解包 JSON 版 FM 数据 (Rust 端, 与 blkx 并存 data/)")
    sub.add_parser("web", help="D9 前端构建 (pnpm → web/dist)")
    sub.add_parser("rust", help="D9 Rust 构建链 (web + cargo release)")
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
    elif args.cmd == "rustdist":
        cmd_rustdist()
    elif args.cmd == "fmdata":
        cmd_fmdata()
    elif args.cmd == "fmdatajson":
        cmd_fmdatajson()
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
