# Debug Power Curve

调试活塞发动机功率曲线，对比 VoidMei 与 WAPC 的计算结果。

## 使用方法

```
/debug-power-curve [飞机名称]
```

## 参数

- `飞机名称`: 可选，指定要调试的飞机 FM 文件名（不含 .blkx 后缀）

## 工作流程

当用户调用此命令时，执行以下步骤：

### 1. 确定目标飞机

如果用户提供了飞机名称，使用该名称。否则询问用户要调试哪个飞机。

### 2. 查找 FM 文件

在以下位置查找 FM 文件：
- `./data/aces/gamedata/flightmodels/fm/[飞机名称].blkx`
- 用户指定的路径

### 3. 提取 FM 参数

从 FM 文件中提取以下关键参数：
- `Compressor.Altitude0/1/2` → 临界高度
- `Compressor.Power0/1/2` → 临界功率
- `Compressor.Ceiling0/1/2` → 升限高度
- `Compressor.PowerAtCeiling0/1/2` → 升限功率
- `Main.Power` → 海平面功率
- `Compressor.SpeedManifoldMultiplier` → RAM 系数
- `Compressor.ExactAltitudes` → 旧格式标记
- `Compressor.CompressorOmegaFactorSq` → 增压器参数

### 4. 创建或更新调试脚本

基于提取的参数，更新 `test/DebugYak3PowerCurve.java` 中的硬编码值。

### 5. 运行调试

编译并运行调试脚本：
```bash
javac -encoding UTF-8 -d bin -classpath bin test/DebugYak3PowerCurve.java
java -classpath bin DebugYak3PowerCurve
```

### 6. 分析结果

输出功率曲线数据，包括：
- 静态功率 (speed=0) 在各高度
- RAM 效果功率 (301km/h IAS) 在各高度
- 最优增压器级选择

### 7. 与 WAPC 对比（如果可用）

如果 WAPC 项目路径可用，运行 Python 脚本进行对比：
```bash
python3 script/debug_wapc_yak3.py
```

## 相关文档

- `doc/功率曲线调试手册.md` - 详细调试指南
- `src/prog/util/PistonPowerModel.java` - 功率计算核心
- `src/prog/util/FMPowerExtractor.java` - FM 参数提取

## 示例输出

```
=== POWER CURVE (military, static, stage 0) ===

[alt=0] power=1290.0hp
[alt=300] power=1310.0hp (critical altitude)
[alt=675] power=1247.1hp
[alt=1000] power=1194.5hp
...
```
