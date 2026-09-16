# 配置系统

三层架构：

```
ui_layout.cfg (S 表达式 DSL) → ConfigLoader/SExpParser → GroupConfig/RowConfig 内存模型
    → ConfigurationService (实现 ConfigProvider) → Settings 接口 (HUDSettings/OverlaySettings)
```

主要文件：`ConfigurationService`（统一入口+持久化）、`ConfigLoader`（解析/保存）、`ConfigManager`（首运行/用户路径/合并/出厂重置/导入导出）、`SExpParser`、`ConfigWatcherService`（热重载，可选）。

## ⚠️ 新增 RowConfig 字段检查清单（关键！）

新增字段**必须**同时适配两处，否则功能异常（真实案例：commit 2e53f94 修 `unitSource` 遗漏——用户配置合并时丢失，美系飞机进气压显示 "Ata" 而非 "P/inHg"）：

1. **`ConfigManager.mergeRow()`**：按字段性质选来源——结构/定义字段（type/property/unit/format/unitSource/visibleWhen/naWhen 等）取 `template.xxx`；用户自定义字段（value/x/y/alpha/hotkey 等）取 `user.xxx`。遗漏→用户配置合并时新字段值丢失
2. **`ConfigLoader.saveConfig()` + 解析**：序列化写 `:my-field`、解析配 `getKeywordString`/`getKeywordSExp`。遗漏→字段无法持久化，重启丢失

改完 grep 确认三处都有该字段名（ConfigManager 的 merge、ConfigLoader 的 save、ConfigLoader 的 parse）。

## ui_layout.cfg DSL 速查

```lisp
(panel "标题" :cols 2
  (group "组名" :switch "组开关键"          ; 可选: 可见性开关/位置/透明度/字体/热键
    (item "标签" :type switch :target "配置键" :value true :default true
          :desc "提示" :descImg "帮助图")))
```

**item 类型**：`switch`/`switch_inv`（反相，UI ON = value false）/`slider`（`:min :max :unit`）/`combo`（`:options`）/`color`（hex `#RRGGBBAA` 或十进制 `R,G,B,A`；显示用 hex、存储用十进制）/`font`/`hotkey`/`button`/`data`。

**data 类型的条件属性**（均为 S 表达式，同类语法）：

| 属性 | 作用 | 示例 |
|------|------|------|
| `:visible-when` | 条件隐藏整行 | `(and (not (isJetEngine)) (> value 0))` |
| `:na-when` | 条件显示 "-"（不隐藏） | `(> value 9999)` |
| `:unit-source` / `:precision-source` | 指向 TelemetrySource 方法名，每帧动态取单位/精度（公制/英制切换用） | `:unit-source "getManifoldPressureDisplayUnit"` |
| `:unit` / `:precision` | 静态值（也是 preview 模式缺省） | `:unit "Ata" :precision 2` |

**visible-when 表达式**：方法调用 `(isJetEngine)` `(isPropEngine)` `(isPistonEngine)` `(isTurbopropEngine)` `(hasWep)` `(isEngineCheckDone)`；比较 `(> value N)` `(>= <= = !=)`（`= 带万分之一容差`）；逻辑 `(not)` `(and)` `(or)`。preview（TelemetrySource 为 null）时方法调用一律返回 true。`:hide-when-zero` 已废弃，用 `:visible-when (> value 0)` 替代。无 `:visible-when` 的字段恒可见（不从 isXXXValid 自动推断，防意外隐藏）。

动态绑定实现链：`ui_layout.cfg` 的 `:unit-source`/`:precision-source` → overlay（如 `PowerInfoOverlay.bindDynamicFields`）经 `ReflectBinder` 建 supplier → `FieldManager.bind` → `FieldOverlay.onFlightData()` 每帧调用更新。

## 添加新配置项（五步）

1. `ui_layout.cfg` 加 `(item ...)` 定义
2. `HUDSettings`/`OverlaySettings` 加接口方法
3. `ConfigurationService` 内部 Impl 实现（key 与 `:target` 一致）
4. 目标 overlay/组件用配置控制行为
5. `Controller` 把 key 加进对应 overlay 的 `.withInterest()`（WYSIWYG 预览刷新）

## WYSIWYG 预览链

`setConfig(key)` → `UIStateBus.publish(CONFIG_CHANGED, key)` → Controller（仅 PREVIEW 态）→ `OverlayManager.refreshPreviews(key)` → 匹配 interest → `reinitConfig()`。

## 注意事项

- 配置键**大小写敏感**，ui_layout.cfg 与代码必须精确一致
- 配置在用户退出 MainForm/游戏模式时自动持久化；`saveLayoutConfig()` 强制立即保存
- 首运行自动把出厂配置拷到用户路径（`ConfigManager.getUserConfigPath()` 平台相关）
