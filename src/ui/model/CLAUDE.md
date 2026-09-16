# UI 模型层（ui.model）

数据模型与接口，隔离数据源（`Service`）与 UI（overlay）。文件：`TelemetrySource`、`FlightDataProvider`（旧接口）、`ServiceDataAdapter`、`FieldDefinition`/`FieldManager`/`DefaultFieldManager`、`GaugeField`（EngineControlOverlay 表盘用，含零 GC 数字缓冲）、`DataField`、`FlightDataBindings`、`FlightInfoConfig`/`EngineInfoConfig`。

## TelemetrySource

**存在理由**：事件式传参每帧造对象（Map/Event）造成 GC 压力；该接口直接返回 `double` 基本类型，零分配。`Service` 实现它（方法名即数据名：getIAS/getTAS/getMach/getAoA/getNy/getSEP/getThrottle/getRPM/…，完整清单看接口本身）。

**使用规则**：
1. 高频数值走 `TelemetrySource`；布尔/低频标志走 `FlightDataEvent.getPayload()`（EventPayload）
2. getter 实现必须返回已有基本类型值，**禁止在 getter 内创建对象**
3. `init()` 时缓存 source 引用，整个生命周期复用

## 动态单位/精度

单位/精度需随机型变化的字段（如进气压 Ata vs psi），用 `DataField` 的 `unitSupplier`/`precisionSupplier`（每帧调用），由 ui_layout.cfg 的 `:unit-source`/`:precision-source` 驱动，经 `FieldManager.bind` 绑定（可见性则由 `:visible-when` 配置控制，不走代码 supplier）。详见 `src/prog/config/CLAUDE.md`。
