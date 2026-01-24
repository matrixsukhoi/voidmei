# MiniHUD 贡献者开发手册 (MiniHUD Contributor Guide)

> **版本 (Version)**: 2026.01 (Modern Engine)
> **适用对象 (Target Audience)**: UI 开发者 / 模组制作者

MinimalHUD 是 VoidMei 中最复杂的覆盖层之一，它模拟了现代战斗机的平视显示器 (HUD)。本手册旨在通过介绍 **Modern Layout Engine** 帮助开发者理解布局逻辑并添加新组件。

---

## 1. 架构总览 (Architecture Overview)

MinimalHUD 采用了 **依赖式布局引擎 (Dependency-based Layout Engine)**，彻底摒弃了传统的绝对坐标和网格系统。

### 1.1 核心组件

*   **MinimalHUD (`src/ui/overlay/MinimalHUD.java`)**: 主控制器，负责初始化组件、订阅数据事件 (`onEvent`) 和驱动渲染循环。
*   **ModernHUDLayoutEngine (`src/ui/layout/ModernHUDLayoutEngine.java`)**: 布局解算器。它负责根据依赖关系（Parent-Child）和锚点 (`Anchor`) 计算每个组件的最终像素坐标。
*   **HUDLayoutNode (`src/ui/layout/HUDLayoutNode.java`)**: 布局图中的节点。包裹一个 `HUDComponent`，定义其相对位置、锚点和父节点。

### 1.2 为什么使用新引擎？

旧版引擎依赖固定的像素坐标（如 `x=42, y=70`）和硬编码的网格，这导致：
1.  **无法缩放**: 无法适应高分屏或动态调整字体大小。
2.  **难以维护**: 添加一个新行需要手动重新计算下面所有行的 Y 坐标。

新引擎引入了 **Unit (单位)** 和 **Anchor (锚点)** 的概念：
*   所有坐标都使用 `Unit`（1 Unit ≈ 当前字体高度）。
*   组件位置相对于其 **父组件**（而非总是屏幕左上角）。
*   自动处理依赖关系：如果 Row 2 的内容变高了，Row 3 会自动下移。

---

## 2. 快速上手：添加一个新组件 (Quick Start)

假设我们要添加一个新的 "雷达锁定指示器 (Radar Lock)"。

### Step 1: 创建组件类
继承 `HUDComponent` 并实现绘制逻辑。
```java
public class RadarLockIndicator extends HUDComponent {
    public RadarLockIndicator(String id) {
        super(id);
    }
    
    @Override
    public Dimension getPreferredSize() {
        return new Dimension(40, 40);
    }

    @Override
    public void draw(Graphics2D g, int x, int y) {
        g.setColor(Color.RED);
        g.drawRect(x, y, 40, 40);
    }
}
```

### Step 2: 注册组件 (MinimalHUD.java)

在 `init()` 或 `initComponentsLayout()` 中实例化组件：
```java
// 1. Create Component
RadarLockIndicator radar = new RadarLockIndicator("radar_lock");
components.add(radar);
```

### Step 3: 定义布局 (MinimalHUD.java -> initModernLayout)

这是最关键的一步。我们需要告诉引擎该组件放在哪里。

```java
// 在 initModernLayout() 方法中：

// 创建布局节点
ui.layout.HUDLayoutNode radarNode = new ui.layout.HUDLayoutNode("radar_node", radar);

// 定义位置：
// 1. 为了演示依赖，我们要把它挂在 "Attitude"（姿态仪）的右边。
ui.layout.HUDLayoutNode attitudeNode = modernLayout.getNode("attitude");

radarNode.setParent(attitudeNode) // 设置父节点
         .setRelativePosition(0.5, 0) // 向右偏移 0.5 个单位
         .setAnchors(ui.layout.Anchor.TOP_RIGHT, ui.layout.Anchor.TOP_LEFT);
         // Anchor 逻辑：父节点的 TOP_RIGHT 对齐 到 本节点的 TOP_LEFT

// 添加到引擎
modernLayout.addNode(radarNode);
```

---

## 3. 布局原理深度解析 (Deep Dive)

### 3.1 坐标计算公式
`PixelPos = ParentPixels + (UnitPos * LineHeight) + AnchorOffset`

*   **LineHeight**: 全局缩放因子，通常来自 `ctx.hudFontSize`。
*   **UnitPos**: 相对坐标。`0.5` 意味着半行高。

### 3.2 Anchor System (锚点系统)
`Anchor` 枚举定义了 9 个点：`TOP_LEFT`, `CENTER`, `BOTTOM_RIGHT` 等。
`setAnchors(ParentAnchor, SelfAnchor)` 的含义是：**把我的 SelfAnchor 点，重合到父节点的 ParentAnchor 点上**。

**例子**:
*   `setAnchors(BOTTOM_LEFT, TOP_LEFT)`: 这种组合常用于垂直列表。我的左上角，接着上一行的左下角。

### 3.3 调试
如果组件未显示或位置错误：
1.  检查 `script/build.sh` 输出的日志。新引擎会打印 `Topology Order`。
2.  确认 `Topology Order` 中包含你的节点 ID。如果没有，说明未调用 `addNode`。
3.  搜索日志 `LayoutDebug`（需在代码中取消注释），查看计算出的绝对像素坐标。

---

## 4. 常见问题 (FAQ)

*   **Q: 旧代码里的 `HUDLayoutSlot` 去哪了？**
    *   A: 已删除。不再使用预定义的 Slot（如 `TOP_LEFT`），所有位置完全自由定义。
*   **Q: 为什么文字重叠了？**
    *   A: 检查 `setRelativePosition` 的 Y 轴偏移。如果使用依赖布局，通常需要 `0.1` ~ `0.5` 的间距。
*   **Q: Crosshair（准星）的位置怎么算的？**
    *   A: 它是 Root Node（无父节点）。它的位置是相对于 Canvas（窗口）左上角的。目前的逻辑是动态计算：`x = (Width + UserOffsetCrossX) / LineHeight`。
