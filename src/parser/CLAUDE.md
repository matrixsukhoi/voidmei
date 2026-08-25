# Parser Module Development Guide

## Overview

The `parser` package handles data ingestion from War Thunder's HTTP API and flight model (.blk) files. All parsers convert raw game data into structured Java objects for use by the UI and calculation layers.

## File Overview

| File | Responsibility |
|------|----------------|
| `Blkx.java` | Flight model (.blk) file parser - extracts engine, aerodynamic, and performance data |
| `State.java` | Real-time state JSON parser (from `/state` endpoint) |
| `Indicators.java` | Cockpit indicators JSON parser (from `/indicators` endpoint) |
| `FlightAnalyzer.java` | Derived metrics calculation from raw telemetry |
| `FlightLog.java` | Flight data logging and replay |
| `HudMsg.java` | HUD message parsing |
| `MapInfo.java` | Map information parser |
| `MapObj.java` | Map object (vehicles, markers) parser |

> FM 加载编排不在本包——统一走 `prog.fm.FMLoader`（项目内唯一 `new Blkx` 的地方），
> 详见 [`src/prog/fm/`](../prog/fm/) 与根 CLAUDE.md 的 Architecture 节。

---

## Blkx.java - Flight Model Parser

The main FM file parser, extracting engine performance data, aerodynamic coefficients, and structural limits from War Thunder's `.blk` files.

### Thrust Table API

For jet engines, thrust is stored in 2D tables indexed by altitude and velocity.

#### Data Fields

| Field | Type | Description |
|-------|------|-------------|
| `altThrNum` | `int` | Number of altitude points in thrust table |
| `velThrNum` | `int` | Number of velocity points in thrust table |
| `maxThr` | `double[][]` | Military thrust table [altitude][velocity] (kgf) |
| `maxThrAft` | `double[][]` | Afterburner thrust table [altitude][velocity] (kgf) |
| `peakThrMil` | `double` | Peak military thrust (kgf) |
| `peakThrAft` | `double` | Peak afterburner thrust (kgf) |

#### peakThrust API

```java
// Get peak thrust value
double peakMil = blkx.peakThrust(false);  // Military thrust
double peakAft = blkx.peakThrust(true);   // Afterburner thrust
```

**Algorithm:** Traverses the full altitude × velocity grid to find maximum thrust:

```java
private double calculatePeakThrust(double[][] table) {
    double peak = 0;
    for (int i = 0; i < altThrNum; i++) {
        for (int j = 0; j < velThrNum; j++) {
            if (table[i][j] > peak) {
                peak = table[i][j];
            }
        }
    }
    return peak;
}
```

**Search Grid:**
- Altitude: `altThrNum` points (from FM `ThrustMax` table)
- Velocity: `velThrNum` points (from FM `ThrustMax` table)
- Total iterations: `altThrNum × velThrNum`

This correctly accounts for the fact that jet thrust varies with both altitude and airspeed, and peak thrust may occur at a specific altitude/speed combination rather than at sea level static conditions.

### Fuel Modification Support

The parser can extract fuel quality modifications from Central files:

```java
// Extract fuel modifications from Central file data
Blkx.FuelModification fuelMod = Blkx.extractFuelModifications(centralFileData);

// Available fuel types
FuelModification.FuelType.SOVIET_B95     // Soviet B-95 fuel
FuelModification.FuelType.SOVIET_B100    // Soviet B-100 fuel
FuelModification.FuelType.BRITISH_150_OCTANE  // British 150 octane
FuelModification.FuelType.BRITISH_100_SPITFIRE  // British 100 octane Spitfire
```

### Integration with PistonPowerModel

For piston engines, use `FMPowerExtractor` to convert `Blkx` data to `CompressorStageParams`:

```java
Blkx blkx = new Blkx(fmFilePath);
Blkx.FuelModification fuelMod = Blkx.extractFuelModifications(centralData);
CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx, fuelMod);

// Then use with PistonPowerModel
double power = PistonPowerModel.optimalPowerAdvanced(stages, altitude, isWep, speed, true, 15.0);
```

---

## Defensive Parsing Rules (防御性解析规则)

P1/P6 防御加固后，`parser` 包对畸形输入的契约如下。改动这些文件时**不得回退**任何一条：

### 无界扫描禁令

*   任何 `indexOf`/`charAt` 推进的循环必须有**长度上界**：匹配处之后扫不到目标字符
    （截断行、注释、畸形块）时必须退出并按"未找到"返回（`"null"`/空串/null），
    不允许扫出字符串末尾抛 `StringIndexOutOfBoundsException`。
    已加固点：`Blx.cut`/`cutStatic`（toUpperCase 索引漂移）、`getArray`、`getlastone`、
    `getoneinData` 等。
*   `toUpperCase()` 可能使特殊字符变长（如 ß→SS），在大写串里量出的索引不一定落在
    原串范围内——用前必须做 `bix >= tmp.length()` 之类的守卫。

### valid 语义

*   `Blx.valid == true` 是"对象可安全使用"的唯一凭证；`valid == false` 的对象只允许
    看布尔本身，**不允许访问任何解析字段**（调用方约定，见 `FMHandle.hasFM()`）。
*   以下情况必须置 `valid = false` 且不得抛出：文件读入失败（IOException）、
    空文件/纯空白、JSON 内容误喂（`.blk` 格式不可能以 `{` 开头，以此快速识别）、
    `getload()` 内部任何异常（构造器包 try，失败置无效，不外泄半初始化对象）。

### IOException 处理

*   文件读取失败走 `ExceptionHelper.logAndContinue`，标记读入失败并收敛为
    `valid = false`——**不允许**"data 为空串但 valid 仍为 true"的假有效对象流入
    后续解析流程。

### 曲线数据容错

*   `getplotdata` 对 PASSPORT 曲线块逐行解析时，畸形行（缺逗号/数字混入字符）
    **跳过该数据点**而不是抛 `NumberFormatException`/越界异常——这是 P6 fuzz
    （`test/FMParserFuzzer.java`）发现的缺陷，加固后曲线少一个点、发动机数据照常可用。

### Fuzz 套件说明

*   `python script/build.py test fuzz-blkx`：以真机 `fm/bf-109e-4.blkx` 为种子
    （中等体积且含 PASSPORT 曲线块，能覆盖 `getAllplotdata` 的 parseDouble 路径；
    注意 spitfire_f24 无 PASSPORT 块，用它当种子该路径会空转），
    施加字节级/行级/结构级/语义级四类共 13 种变异，每个变异体走
    `new Blkx → getAllplotdata → finalizeLoading` 全管线。验收：任何 Throwable
    逃逸即失败；单变异体限时 5s；抽样 30 个变异体另走 `FMLoader.load` 断言句柄契约
    （status ∈ {READY, MISSING, CORRUPT}，READY ⇒ blkx 非 null）。
    固定种子（默认 20260825）可复现；data/ 缺失时 build.py 自动跳过。
*   8111 遥测 (`State`/`Indicators`) 不做 fuzz——Gaijin 官方 API 序列化固定，按可信
    处理；解析层合同只有两态：字段缺失返回哨兵值（-65535），脏类型抛
    `NumberFormatException` 由 `Service.run` 顶层 catch 兜住（一条 ERROR +
    sleep 1s + 下轮自愈）。真实瞬态（断连/菜单态缺字段）由 e2e s5 场景覆盖。

---

## State.java & Indicators.java

Parse real-time telemetry from War Thunder's local HTTP API (port 8111).

### Data Flow

```
War Thunder HTTP API
    ↓
State.java      → Aircraft position, orientation, speed, altitude
Indicators.java → Cockpit instruments, engine parameters, fuel
    ↓
FlightDataBus (event publisher)
    ↓
Overlay components
```

### Key Endpoints

| Endpoint | Parser | Data |
|----------|--------|------|
| `/state` | `State.java` | Position, velocity, orientation |
| `/indicators` | `Indicators.java` | Engine RPM, manifold pressure, temperatures |

---

## Design Principles

1. **Defensive Parsing**: Handle missing or malformed data gracefully (see "Defensive Parsing Rules" above)
2. **Immutable Results**: Parsed objects should be treated as immutable
3. **Thread Safety**: Parsers may be called from background threads
4. **Fail Soft, Never Throw**: Invalid FM files are detected early and marked `valid = false` — the parser never throws to its caller; failure classification (READY/MISSING/CORRUPT) is the loader's job
