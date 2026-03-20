# Compressor Gauge 初始显示延迟 - 已知行为记录

## 现象描述

三级增压器飞机启动后约 5 秒内，compressor gauge 可能显示不准确的档位（如显示"增2"），约 5 秒后自动校正为正确值（如"增1"）。

**这是正常行为，无需修改。**

## 原因说明

### 数据流

```
War Thunder API → State.compressorstage → Service.getCompressorStage()
               → EngineControlOverlay.updateGaugesZeroGC()
```

### 根本原因

1. **War Thunder API 初始化延迟**：游戏刚启动时，API 返回的遥测数据可能不准确
2. **引擎类型检测周期**：VoidMei 需要约 5 秒完成引擎类型检测（100 次采样 × 50ms/次）

### `engineCheckDone` 机制详解

**目的**：自适应判断飞机引擎类型（活塞 vs 喷气 vs 涡桨）

**位置**：`Service.java:493-526` - `checkEngineJet()` 方法

**检测原理**：
```
活塞机特征：有磁电机 (magneto >= 0)，有螺距控制 (pitch != -65535)
喷气机特征：无磁电机 (magneto < 0)，无螺距控制
涡桨特征：无磁电机，但有螺距控制
```

**算法逻辑**：
```java
// 每次 poll（约 50ms）累积一票
if (sState.magenato < 0) {
    checkEngineType--;  // 倾向喷气
} else {
    checkEngineType++;  // 倾向活塞
}

// 累积 100 票后才确定引擎类型
if (Math.abs(checkEngineType) >= 100) {
    checkEngineFlag = true;  // = engineCheckDone
    // 根据 checkEngineType 和 checkPitch 确定具体类型
}
```

**为什么需要 100 次采样？**
1. **避免误判**：War Thunder API 在游戏初始化时可能返回不稳定/错误的值
2. **统计置信度**：累积足够样本才能可靠区分引擎类型
3. **时间成本**：`freqService` 默认 50ms，100 次 = **约 5 秒**

## 相关代码位置

| 功能 | 文件 | 行号 |
|------|------|------|
| 引擎类型检测 | `Service.java` | 493-526 |
| `checkEngineFlag` 发布 | `Service.java` | 484 |
| `EventPayload.engineCheckDone` | `EventPayload.java` | 17, 33, 45 |
| Compressor gauge 更新 | `EngineControlOverlay.java` | 487-496 |
| Compressor maxValue 设置 | `EngineControlOverlay.java` | 400-415 |

## 用户体验说明

- 这是设计如此的行为，不是 bug
- 约 5 秒后数据会自动校正
- 在引擎类型检测完成后，增压器档位、最优档位标记等功能才能正常工作
