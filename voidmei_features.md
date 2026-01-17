# VoidMei 特性与依赖映射文档 (Developer Feature Map)

本文档旨在为开发人员提供 VoidMei 各项特性的深度映射。它不仅描述功能，更重点记录了**代码实现的关键依赖**、**逻辑约束**以及**修改时的潜在风险点**。
在进行代码修改时，请查阅此文档以评估变更的影响范围 (Impact Analysis)。

---

## 1. 核心架构特性 (Core Architecture)

### 1.1 悬浮窗渲染体系 (Overlay System)
*   **特性描述**: 提供透明、可拖拽、位置记忆的 HUD 窗口。
*   **实现类**: `ui.overlay.BaseOverlay` (基类), `ui.renderer.OverlayRenderer` (渲染接口)。
*   **关键分支**:
    *   **FieldOverlay**: 基于**事件驱动**的 KV 键值对显示（如速度、高度）。使用了 `DraggableOverlay` 逻辑。
    *   **BaseOverlay (直接继承)**: 基于**线程轮询**的自定义绘制（如 FMData 的文本列表）。
*   **重要约束**:
    *   **线程启动**: `OverlayManager` 必须在 Preview 模式下也为 `needsThread=true` 的 Overlay 启动线程，否则 `BaseOverlay` 子类（如 FMData）**不会刷新**。
    *   **渲染循环**: `BaseOverlay` 依赖 `run()` 中的 `while(doit)` 循环触发 `updateUI`。
*   **回归测试点**:
    *   修改 `BaseOverlay` 或 `OverlayManager` 后，必须检查 **FMDataOverlay 在预览模式下是否显示内容**。

### 1.2 零闪烁启动 (Zero-Flash Startup)
*   **特性描述**: 程序启动时，面板不应闪烁空白或显示 `null`/`0`。
*   **实现细节**:
    1.  **数据预热**: `Service.java` 中将所有 FlightData 字段初始化为 `"-"`。
    2.  **启动延迟**: `Controller.java` 中的 `changeS3` (Game Mode) 和 `initPreview` 强制加入了 **100ms - 2000ms** 的延迟 (`Thread.sleep`)，等待数据总线稳定后再创建窗口。
*   **潜在风险**:
    *   如果移除延迟，可能会在旧电脑上导致 Overlay 创建时数据尚未 Bind，导致 NPE 或空白。
    *   如果修改了 `Service` 初始化逻辑，可能导致启动瞬间显示 `0.0` 而不是 `-`。

---

## 2. 功能模块映射 (Feature Modules)

### 2.1 气动数据面板 (FM Data Overlay)
*   **用户特性**: 显示 `.blk` 原始数据；起飞后自动隐藏。
*   **实现类**: `ui.overlay.FMDataOverlay`。
*   **渲染器**: `ui.overlay.ZebraListRenderer` (斑马纹列表)。
*   **数据源**: `Controller.getBlkx()` -> `UIStateEvents.FM_DATA_LOADED`。
*   **关键逻辑约束 (Critical Logic)**:
    *   **自动隐藏**: 在 `shouldExit()` 中硬编码了逻辑：`if (gear != 100 || (speed > 10 && throttle > 0)) return true;`。这意味着**只有在地面且低速时**该面板才可见。
    *   **数据更新**: 采用**混合模式**——事件 (`FM_DATA_LOADED`) 触发缓存更新，后台线程 (`run()`) 触发 UI 重绘。
*   **修改影响**:
    *   如果将基类改为 `FieldOverlay`，将**丢失**列表渲染能力（因为 FieldOverlay 强制网格布局）。
    *   如果移除 `UIStateBus` 订阅，切换飞机时数据不会刷新。

### 2.2 引擎信息面板 (Engine Info)
*   **用户特性**: 显示马力、推力、温度等 MEC 数据。
*   **实现类**: `ui.overlay.EngineInfo` (现已重构为 Extends `FieldOverlay`)。
*   **配置类**: `ui.overlay.config.EngineInfoConfig`。
*   **数据源**: `FlightDataBus` (Keys: `sTotalHp`, `sTotalThr` 等)。
*   **关键变化**:
    *   从硬编码绘图迁移到了 `FieldManager` 管理。
    *   依赖 `ui_layout.cfg` 中的 `[引擎信息]` 段落配置位置。

### 2.3 飞行信息 HUD (Flight Info)
*   **用户特性**: 核心 HUD (IAS, TAS, Climb)。
*   **实现类**: `ui.overlay.FlightInfo`。
*   **数据源**: `FlightDataBus`。
*   **潜在风险**:
    *   很多单位转换（米制/英制）是在 `FlightAnalyzer` 中计算后推送的，UI 层只负责显示。如果数值不对，应检查 Parser 层而非 Overlay 层。

### 2.4 预览模式 (Preview Mode)
*   **特性描述**: 离线状态下展示模拟数据。
*   **实现**: `Controller` 加载 `fm/p-51d-30_usaaf_korea.blk`。
*   **风险点**:
    *   Preview 模式复用了 Overlay 的实例化逻辑，但初始化路径不同 (`initPreview` vs `init`)。修改 Overlay 构造函数时需**同时验证**两种模式。

---

## 3. 配置文件依赖 (Configuration Dependencies)

### 3.1 ui_layout.cfg
*   **作用**: 定义所有面板的坐标 (X,Y)、开关 (Visible)、字体大小。
*   **依赖关系**:
    *   **Header Name**: 代码中 `DefaultFieldManager` 可能依赖 Config Key 来决定是否加载某行数据。
    *   **SwitchKey**: `Controller` 中注册 Overlay 时使用的 Key (如 "enableFMPrint") **必须**与 cfg 文件中的 SwitchKey 一致，否则开关失效。

---

## 4. 调试与排错指南 (Debugging Guide)

### 现象：某个 Overlay 不显示
1.  **检查 Config Key**: `ui_layout.cfg` 中的开关名是否与 `Controller.register` 中的字面量一致？
2.  **检查 Thread**: 如果是 `BaseOverlay` 子类，检查 `OverlayManager` 是否启动了线程（尤其是在 Preview 模式）。
3.  **检查 shouldExit()Logic**: 是否触发了自动隐藏逻辑（如 FMData 的起飞隐藏）？

### 现象：数据不刷新
1.  **Event Bus**: 检查 `UIStateBus` 或 `FlightDataBus` 是否有 PUBLISH 日志。
2.  **Poll Loop**: 检查 `BaseOverlay` 的日志 `Data changed` 是否触发。

### 现象：启动闪烁
1.  **Service Init**: 检查变量初始值是否为 `null`。
2.  **Delay**: 检查 `Controller` 线程启动延迟是否被意外移除。
