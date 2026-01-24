# ConfigLoader.java 技术解析与 UI 扩展指南

`ConfigLoader.java` 是 VoidMei “动态 UI 系统”的核心。它负责将文本形式的 `ui_layout.cfg` 翻译成内存中的配置模型，并驱动整个设置界面和 HUD 覆盖层的生成。

---

## 1. 实现原理：数据驱动 UI (Data-Driven UI)

该文件没有使用标准的 `java.util.Properties`，而是实现了一个自定义的 **INI 改良版解析器**。
*   **分级结构**: `[Section]` 对应一个悬浮窗或设置页分组（`GroupConfig`），下方的行对应具体的显示项或控件（`RowConfig`）。
*   **行列解析**: 使用 `||` 作为分隔符：`标签 || 公式/类型 || 格式化字符 || 默认值/开关`。
*   **反射兼容**: 变量解析部分（如 `S.rpm`）配合 `AttributePool` 实现动态绑定。

---

## 2. 那个“大 Switch” (if-else if 链) 在做什么？

您注意到的 `loadConfig` 方法中从 207 行到 287 行的长串 `if-else if` 实际上是一个 **类型分发器 (Type Dispatcher)**。

### 它的职责：
它通过识别“公式”字段的**前缀**来决定这一行该渲染成什么。

*   **识别前缀**: 比如 `SLIDER:`, `COMBO:`, `SWITCH:` 等。
*   **元数据提取**: 如果识别到 `SLIDER:fontSize:0:20`，它会解析出：
    *   类型是 `SLIDER`。
    *   绑定的配置键是 `fontSize`。
    *   范围是 `0` 到 `20`。
*   **默认降级**: 如果没有任何前缀，它会认为这是一个普通的 `DATA` 类型，直接显示遥测数据（如 `S.ias`）。

---

## 3. 如何新增一种 UI 类型？(以新增 `MY_COLOR` 为例)

如果您想在 `ui_layout.cfg` 中写出类似 `背景颜色 || MY_COLOR:bg_color || %s || true` 这样的配置，您需要修改三个地方：

### 第一步：修改 `ConfigLoader.java` (识别解析)
在该文件的 `loadConfig` 循环中增加识别逻辑：

```java
// 在那个 if-else if 链中增加
} else if (formula.startsWith("MY_COLOR:")) {
    rc.type = "MY_COLOR";
    rc.property = formula.substring(9); // 提取 bg_color
    rc.visible = true;
}
```

### 第二步：修改 `RowRendererRegistry.java` (注册映射)
告诉系统：当遇到 `MY_COLOR` 类型时，使用哪个渲染器。

```java
// static 块中
renderers.put("MY_COLOR", new MyColorRowRenderer());
```

### 第三步：新建 `MyColorRowRenderer.java` (界面实现)
这种模式采用了 **策略模式 (Strategy Pattern)**。您需要实现 `RowRenderer` 接口，定义具体的 WebLaF 界面：

```java
public class MyColorRowRenderer implements RowRenderer {
    @Override
    public WebPanel render(RowConfig row, GroupConfig group, RenderContext ctx) {
        WebPanel p = new WebPanel(new BorderLayout());
        // 在这里创建 WebButton、WebLabel 或颜色选择器
        // 使用 ctx.syncStringToConfigService 或 ctx.onSave 来保存数据
        return p;
    }
}
```

---

## 4. 总结：新增类型的完整流转图

```mermaid
graph LR
    CFG["ui_layout.cfg"] -- "文本匹配" --> Loader["ConfigLoader.java"]
    Loader -- "设置 RowConfig.type" --> Group["GroupConfig 对象"]
    Group -- "循环遍历" --> Page["DynamicDataPage.java"]
    Page -- "请求渲染器" --> Registry["RowRendererRegistry.java"]
    Registry -- "返回实例" --> Renderer["MyTypeRenderer.java"]
    Renderer -- "返回 WebPanel" --> UI["显示在设置界面"]
```

这种架构允许您像搭积木一样，通过配置文件就能控制设置界面的布局，而不需要修改核心 UI 逻辑。
