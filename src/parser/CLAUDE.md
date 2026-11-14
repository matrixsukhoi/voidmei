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

## 玩家地图状态（公共 API）

- `MapObj.isPlayerDead(json)` / `isPlayerDead(json, loc)`：根据 map_obj 判断玩家是否阵亡/不在战场上；无 Player 图标或坐标超出 0..1 返回 true。阵亡后地图仍返回整图机场，消费方不得再用旧 loc 算距离。
- **补给瞬移（2026-11-14 真机抓包，jf_17）**：飞机停在跑道上时 map_obj 玩家坐标连续 30s 完全不变；补给完成瞬间坐标一次性跳约 441m 到跑道端，IAS 仍 0、油门 0、`Mfuel` 不变、RPM 有一次抖动。8111 没有 repair/rearm 标志。补给场景 Player 图标全程存在（104/104 帧），检测不依赖死亡判断。
- **实现 `parser.PlayerResupplyDetector`**（Service 每轮 map poll 调用）：连续 ≥5s 满足 IAS<5km/h 且单轮位移<1m → 单轮位移≥100m → 落点距任一条 ≥100m 跑道端点 ≤250m，返回 true；Service 记录 `lastResupplyTimeMs`。用抓包回归：t=48.358s 精确触发，之前减速/滑行阶段 0 误报。玩家离开地图/坐标无效时 `reset()`，避免把重生当补给。

## 地图机场解析（v1.587+）

- `MapObj.parseAirfields` 用正则扫 `/map_obj.json` 的 `"type":"airfield"` 对象，取跑道两端 `sx/sy/ex/ey`、`color[]`/`color`、`blink`；坐标支持科学计数法，缺字段的对象跳过。
- `MapObj.resolveAirfields` 换算米制：`(坐标差) × MapInfo.grid_size`。**grid_size 才是物理米数**，不是 `cmapmaxsize`（map_max-map_min）；真机快照 `grid_size=(57584,64194)` 而 `cmapmaxsize=(65536,65536)`。`MapInfo.update` 用容错正则单独解析 `grid_size`。
- 敌我判定跟随旧版罗盘机场指示：`blink != 0 || (R>100 && R>B)` 为敌方；其余友方。
- `Airfield` 额外携带 `bearingDeg`（世界方位，0=北顺时针；箭头方向 = bearing - 当前航向）和 `mapGrid`（与玩家 mapGrid 同一套 `mapStage/inGameOffset` 公式）。
- 玩家 loc 未解析到（保持 0,0）或 grid_size 未就绪时返回空列表；`Service.resetvaria()` 会清空机场列表并重置 grid_size，换图后由 playerLive / map poll 重新拉取。
- **阵亡/观战/过渡帧**：`getPlayerLoc` 返回本轮是否找到 Player 图标；`Service` 只在 `playerLive && playerFound` 时解析机场，否则清空。`resolveAirfields` 还会丢弃 loc/机场中心超出 0..1 的异常坐标。地图在阵亡后仍会返回整张图的机场，直接用会产生几百公里的假距离。
  - 真机抓包（2026-11-14）：阵亡后 Player 图标消失，map_obj 机场坐标从 0..1 变成 `1.35/-0.33`、`1.93/2.60`、`0.24/4.14` 等网格单位值，同时 `/map_info.json` 的 `grid_size` 短暂变成 `2300x2400`，几秒后恢复 `131072x131072`。旧逻辑用最后 loc × 131072 算出 149~446km 假距离。防线：无 Player 图标即清空；坐标 0..1 守卫再兜底。

## 设计原则

Fail soft, never throw：畸形 FM 文件早发现早置 `valid = false`，解析器永不向调用方抛出；失败分类（READY/MISSING/CORRUPT）是 loader 的职责。解析结果视为不可变；可能被后台线程调用。
