# Parser 模块

数据摄取层：8111 遥测 JSON（`State`/`Indicators`）、FM 文件（`Blkx`）、派生计算（`FlightAnalyzer`）、飞行记录（`FlightLog`）、地图/HUD 消息（`MapInfo`/`MapObj`/`HudMsg`）。

> FM 加载编排不在本包——统一走 `prog.fm.FMLoader`（项目内唯一 `new Blkx` 的地方）。

## 防御性解析契约（P1/P6 加固，不得回退）

- **无界扫描禁令**：任何 `indexOf`/`charAt` 推进的循环必须有长度上界——匹配失败（截断行/注释/畸形块）时按"未找到"返回（`"null"`/空串/null），不许扫出字符串末尾抛越界。已加固点：`Blx.cut`/`cutStatic`、`getArray`、`getlastone`、`getoneinData` 等
- **toUpperCase 索引漂移**：`toUpperCase()` 可能使特殊字符变长（ß→SS），大写串里量出的索引不一定落在原串内——用前必须做 `bix >= tmp.length()` 之类守卫
- **valid 语义**：`Blx.valid == true` 是"对象可安全使用"的唯一凭证；`false` 时只许看布尔本身，不许访问任何解析字段（调用方约定见 `FMHandle.hasFM()`）。以下情况必须置 `valid = false` 且不得抛出：读文件 IOException、空文件/纯空白、JSON 误喂（`.blk` 不可能以 `{` 开头，以此快速识别）、`getload()` 内部任何异常（构造器包 try，不外泄半初始化对象）
- **IOException**：走 `ExceptionHelper.logAndContinue` 收敛为 `valid = false`——不允许"data 为空串但 valid 仍 true"的假有效对象流入后续流程
- **曲线数据容错**：`getplotdata` 逐行解析 PASSPORT 曲线块时，畸形行（缺逗号/数字混字符）**跳过该数据点**而非抛异常（P6 fuzz 发现的缺陷：曲线少一个点、数据照常可用，好过崩溃）

## Fuzz 套件

`python script/build.py test fuzz-blkx`：真机 `fm/bf-109e-4.blkx` 为种子（含 PASSPORT 曲线块；spitfire_f24 无该块，当种子会空转该路径），字节/行/结构/语义四类 13 种变异，每个变异体走 `new Blkx → getAllplotdata → finalizeLoading` 全管线。验收：任何 Throwable 逃逸即失败；单变异体限 5s；抽样 30 个另走 `FMLoader.load` 断言句柄契约（status ∈ {READY, MISSING, CORRUPT}）。固定种子（默认 20260825）；data/ 缺失自动跳过。

**8111 遥测不做 fuzz**——Gaijin 官方 API 序列化固定，按可信处理。解析层合同两态：字段缺失返回哨兵值（-65535），脏类型抛 `NumberFormatException` 由 `Service.run` 顶层 catch 兜住（一条 ERROR + sleep 1s + 下轮自愈）。真实瞬态（断连/菜单态）由 e2e s5 场景覆盖。

## 设计原则

Fail soft, never throw：畸形 FM 文件早发现早置 `valid = false`，解析器永不向调用方抛出；失败分类（READY/MISSING/CORRUPT）是 loader 的职责。解析结果视为不可变；可能被后台线程调用。
