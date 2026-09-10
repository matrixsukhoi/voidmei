# VoidMei - 战争雷霆 8111 端口遥测 HUD

Rust 实现(原生 exe,无需 JRE 等任何运行时依赖)。

# 工作原理

- 通过 HTTP/GET 请求读取 127.0.0.1:8111 端口的飞行状态(state)与飞行仪表(indicators)数据
- 解析离线拆包的气动模型文件(FM, JSON 格式)
- 处理/计算上述信息, 以 HUD 悬浮窗 + 设置窗的形式呈现给用户

# 从源码构建

**需 Rust 工具链(stable)与 Node.js(含 pnpm/corepack), Windows 构建**(overlay 用 Win32 API)。

```bash
git clone https://github.com/matrixsukhoi/voidmei.git
cd voidmei
# FM 数据: 运行 python script/build.py fmdata 从游戏客户端解包生成, 或从 release 包复制 data/
python script/build.py rust       # web 前端 + cargo release 一键构建
python script/build.py rustdist   # 组装分发包 → dist/VoidMei_Rust_*.zip (解压即用)
bash script/rust_run.sh           # 本地运行 (仓库根即工作区)
cargo test --workspace            # 单元测试
```

也可直接用 cargo: `cargo build --release`(前提: 先 `python script/build.py web` 生成前端 dist, 它被编译期嵌入 exe)。

# 目录速览

```
crates/{kernel,data,overlay,ui,webui,voidmei}/   cargo workspace 六 crate
script/          build.py(构建入口) / rust_run.sh / rust_e2e.sh / mock_8111.py
lang/ fonts/ image/ voice/ data/                 运行时资源 (data 为派生数据不进 git)
formulas.cfg     公式槽唯一真相
doc/             开发文档
更新日志.txt      发版记录
```

架构与开发指南详见 `CLAUDE.md`。

# Windows 命令行模式安装 VoidMei (scoop)

打开非管理员模式的终端[按下 WIN+R - 输入 cmd - 按下回车], 输入以下命令

先安装 scoop(如果已安装可跳过)
```
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
irm get.scoop.sh | iex
```

设置代理(如果网络无问题可以直接跳过)
```
scoop config proxy [ip:port]
```

安装 git(如果已安装可跳过)
```
scoop install git
```

添加 @Lustra-Fs 大佬提供的 bucket
```
scoop bucket add Lutra-Fs_scoop-bucket https://github.com/Lutra-Fs/scoop-bucket
```

安装 VoidMei, 安装完成后开始菜单中应该能看到 VoidMei 可执行文件
```
scoop install Lutra-Fs_scoop-bucket/voidmei
```

版本升级请用该命令
```
scoop update voidmei
```
