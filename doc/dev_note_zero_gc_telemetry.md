# Zero-GC Telemetry Pipeline 开发记录

**日期**: 2026-01-25
**模块**: Performance / Telemetry / UI Rendering

## 1. 背景与问题 (Context & Problem)

在原有的架构中，用户在游戏过程中遇到微卡顿（Stuttering）现象，尤其是在高帧率更新时。经分析，主要性能瓶颈位于数据总线和 UI 渲染管线。

### 1.1 原有架构 ("Legacy Pipeline")
*   **数据流**: UDP 接收 -> 解析为 `double` -> 转换为 `String` -> 存入 `Map<String, String>` -> EventBus 分发 -> Overlay 接收 Map -> `Double.parseDouble(String)` -> 渲染。
*   **核心问题**:
    1.  **字符串对象泛滥**: 每一帧、每一个仪表数据（速度、高度、RPM等）都会创建一个新的 `String` 对象（例如 `"350.5"`）。
    2.  **垃圾回收 (GC) 压力**: 按照 60FPS 计算，每秒产生数千个短命对象，导致 JVM 频繁触发 Minor GC，造成画面微卡顿。
    3.  **解析开销**: 反复在 数值 -> 字符串 -> 数值 之间转换，浪费 CPU 周期。

### 1.2 为什么 Map<String, Double> 也不行？
在后续的技术讨论中，我们分析了替代方案：
> *"难道不能放 Double 吗?"*

即使将 Map 的值类型改为 `Double`，依然无法解决 GC 问题：
*   **自动装箱 (Autoboxing)**: Java 集合（Map/List）只能存储对象引用。`double`（原始类型）必须被包装成 `Double`（堆对象）。
*   这意味着每一帧更新数据时，依然要 `new Double(val)`，依然产生大量临时对象，无法避免 GC。

---

## 2. 解决方案：Zero-GC 架构 (Solution)

为了彻底消除每帧的对象分配，我们设计并实施了 **Zero-GC Telemetry Pipeline**。

### 2.1 核心设计理念
1.  **原始类型传递 (Primitives Only)**: 整个数据链路只传递 `double` 或 `int`，杜绝对象封装。
2.  **直接内存访问 (Direct Access)**: 消费者（Overlay）直接向生产者（Service）拉取数据，跳过中间容器。
3.  **缓冲区复用 (Buffer Reuse)**: 字符串只能在最终渲染到屏幕的一瞬间存在，且使用可复用的 `char[]` 替代 `String`。

### 2.2 架构实现细节

#### A. 数据源层 (Publisher)
引入 `TelemetrySource` 接口，直接由 `Service` 实现。

```java
public interface TelemetrySource {
    // 直接返回原始类型，无对象分配
    double getIAS();
    double getRPMThrottle();
    // ...
}
```
`Service` 类直接从 UDP 解码后的结构体 (`sState`) 返回字段，不做任何转换。

#### B. 数据分发层 (Inter-Process Communication)
采用 **“推-拉结合” (Push-Signal, Pull-Data)** 模式：
1.  **Push (信号)**: `Service` 通过 `EventBus` 发送 `FlightDataEvent`。这仅作为一个“时钟信号 (Tick)”，告知订阅者“新一帧数据已就绪”。
2.  **Pull (数据)**: 订阅者（Overlay）收到信号后，调用 `TelemetrySource` 的方法直接拉取所需的原始数值。

> **Q&A: Telemetry 可以实现订阅吗？**
> 我们没有采用“字段级订阅”（如 `subscribeIAS(callback)`），因为对于每帧都需要更新所有数据的仪表盘来说，批量拉取是最高效的，且保证了数据的时间一致性。

#### C. 渲染层 (Consumer & Renderer)
为了配合链路对 `String` 的移除，渲染层必须支持 `char[]` 绘图。

1.  **格式化**: 引入 `FastNumberFormatter.format(double val, char[] buf, int precision)`。它将数字直接写入预先分配好的 `char[]` 数组，不创建任何新对象。
2.  **组件支持**:
    *   `LinearGauge` 和 `LabeledLinearGauge` 增加了接受 `char[]` 的 `update` 和 `draw` 方法。
    *   `UIBaseElements` 增加了 `__drawStringShade` 的 `char[]` 重载。
    *   `StickValue` 等复杂仪表内部维护了 `char[]` 缓冲区，替代原本的 `String` 成员变量。

---

## 3. 实施范围 (Scope of Migration)

本项目实施了“Project Wide Rollout”，覆盖了核心高频仪表：

| 模块 | 迁移状态 | 说明 |
| :--- | :--- | :--- |
| **EngineInfo** | ✅ 完成 | 使用 `FieldManager` 绑定机制 |
| **FlightInfo** | ✅ 完成 | 这个是 Main HUD，最为关键，已全量绑定 |
| **EngineControl** | ✅ 完成 | 重写了 `onFlightData` 循环逻辑 |
| **StickValue** | ✅ 完成 | 移除 String 字段，改用 Buffer + Formatter |
| **AttitudeInd** | ✅ 原生支持 | 原本逻辑就是直接访问 Service 状态，性能最优 |
| **GearAndFlaps** | ⚠️ 部分优化 | 数据获取已优化，渲染层仍有少量 String (低频更新) |

---

## 4. 性能收益 (Performance Impact)

*   **GC 频率**: 在高负载下显著降低，消除了由数据遥测导致的持续性 Minor GC。
*   **帧率稳定性**: 解决了“每隔几秒卡顿一下”的问题，支持更高的刷新率设置（如 60FPS+）。
*   **CPU 占用**: 移除了大量的 `Double.parseDouble` 和字符串操作，降低了 UI 线程负载。

## 5. 开发者指南 (Developer Guide)

若要为新仪表添加 Zero-GC 支持：

1.  **不要**从 `FlightDataEvent.getData()` (Map) 获取数据。
2.  在 `init` 方法中保存 `Service` (作为 `TelemetrySource`) 的引用。
3.  在类中预分配 `char[] buffer = new char[32];`。
4.  在 `onFlightData` 中：
    ```java
    double val = telemetrySource.getSomeValue();
    int len = FastNumberFormatter.format(val, buffer, 1);
    renderer.drawText(..., buffer, len, ...);
    ```
5.  如果 `TelemetrySource` 缺少所需字段，请先在接口中添加并在 `Service` 中实现。
