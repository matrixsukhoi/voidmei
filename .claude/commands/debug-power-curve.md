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

### 3. 查找 Central 文件（燃料修改）

在以下位置查找 Central 文件：
- `./data/aces/gamedata/flightmodels/[飞机名称].blkx`
- 自动检测燃料修改类型（苏联 B-95/B-100、英国 150 辛烷）

### 4. 提取 FM 参数

从 FM 文件中提取以下关键参数：
- `Compressor.Altitude0/1/2` → 临界高度
- `Compressor.Power0/1/2` → 临界功率
- `Compressor.Ceiling0/1/2` → 升限高度
- `Compressor.PowerAtCeiling0/1/2` → 升限功率
- `Compressor.AltitudeConstRPM0/1/2` → ConstRPM 高度
- `Compressor.PowerConstRPM0/1/2` → ConstRPM 功率
- `Main.Power` → 海平面功率
- `Compressor.SpeedManifoldMultiplier` → RAM 系数
- `Compressor.ExactAltitudes` → 旧格式标记

### 5. 创建验证脚本

基于 `doc/功率曲线调试手册.md` 中的模板创建临时验证脚本。
使用 `optimalPowerAdvanced`（完整 WAPC 模型）进行计算。

### 6. 运行调试

编译并运行验证脚本：
```bash
javac -encoding UTF-8 -d bin -classpath bin test/VerifyXXX.java
java -classpath bin VerifyXXX
```

### 7. 分析结果

输出功率曲线数据，包括：
- Military 功率在各高度（含 WAPC 对比值）
- WEP 功率在各高度（如有 WEP）
- 最优增压器级选择
- 燃料修改影响

### 8. 与 WAPC 对比（如果可用）

如果用户有 WAPC 项目和对应飞机的 datamine JSON 文件：
```bash
cd /path/to/wtapc/performance_calculators
python3 single_aircraft_calculator.py \
    --fm /path/to/fm/xxx.json \
    --central /path/to/xxx.json \
    --octane true \
    --speed 300 \
    --modes military WEP
```

## 相关文档

- `doc/功率曲线调试手册.md` - 详细调试指南（含燃料修改、已验证飞机）
- `src/prog/util/PistonPowerModel.java` - 功率计算核心
- `src/prog/util/FMPowerExtractor.java` - FM 参数提取（含燃料加成）
- `src/prog/util/PowerCurveHelper.java` - 功率曲线形状判断

## 示例输出

```
Alt(m)   WAPC(hp)     VoidMei(hp)  Diff(hp)
-------------------------------------------
0        2300.0       2300.0       0.0
1000     2160.8       2160.2       -0.6
2000     2000.0       2000.0       0.0
...
Military max diff: 0.7 hp
WEP max diff: 0.7 hp

>>> ALL VALUES MATCH WAPC (< 1 hp tolerance) <<<
```
