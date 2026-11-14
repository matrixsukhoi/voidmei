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

**item 类型**：`switch`/`switch_inv`（反相，UI ON = value false）/`slider`（`:min :max :unit`）/`combo`（`:options`）/`color`（hex `#RRGGBBAA` 或十进制 `R,G,B,A`；显示用 hex、存储用十进制）/`font`/`hotkey`/`button`/`data`/`help`（只读帮助行，悬浮显示 `:desc`/`:desc-img`，不写配置）。

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

## MiniHUD 自由画布存储

- MiniHUD 的布局存在该面板的一个**隐藏配置项**（`miniHudLayout`）里，不生成控件；随模板合并、备份、导入、恢复出厂一起走。
- 值是自由画布文档的版本化字符串：原子组件列表 + 显式分组，组件保存绝对坐标/缩放/层级/属性；没有父子/锚点字段。文本组件的 `preview`（预览演示值）和 `template`（宽度模板）分开存，恢复默认/导入后预览不能靠全局演示数据兜底。
- 该配置项的值**必须是字符串**：配置更新会按现有值类型强转，初始值写成布尔/数字会把布局串吃掉。
- **不要把布局拆成“每组件一行”**：模板升级/导入合并只保留模板行，用户新增行会丢；整份布局必须活在一个固定配置项的值里（同类先例：语音包配置）。
- 老用户升级时一次性把旧逐项开关换算成等价布局；旧行随模板删除消失；已存在的合法 V2 文档不覆盖，旧版/坏串回退默认布局。
- 用户手工放回旧配置时模板 hash 可能没变；初始化会检测旧 MiniHUD 开关或 V1 布局并强制走一次合并/迁移。
- 旧版三标签全关时右列几何是 3.5 行高，迁移不能只清标签文字。
- 旧 `disableHUD*Label` 是 switch-inv：文件值 **false=隐藏、true=显示**（名字是 disable，存的是显示值），迁移时不要按字面取反。
- 新增这类隐藏行时，解析/序列化/合并三处都要能往返（见上方“新增 RowConfig 字段检查清单”）。
- 数据字段目录从现有 `:type data` 行汇总，配置重载/导入/出厂后重建。

## WYSIWYG 预览链

`setConfig(key)` → `UIStateBus.publish(CONFIG_CHANGED, key)` → Controller（仅 PREVIEW 态）→ `OverlayManager.refreshPreviews(key)` → 匹配 interest → `reinitConfig()`。

## 注意事项

- 根 `.gitignore` 的运行时目录规则必须写成 `/config/`（只匹配项目根的 config/）；写成 `config/` 会把 `src/prog/config/` 下**新增**文件静默忽略（本目录历史文件已跟踪所以不易发现）。新增本目录文件后 `git status` 看不到时先查这个。

- 配置键**大小写敏感**，ui_layout.cfg 与代码必须精确一致
- 配置在用户退出 MainForm/游戏模式时自动持久化；`saveLayoutConfig()` 强制立即保存
- 首运行自动把出厂配置拷到用户路径（`ConfigManager.getUserConfigPath()` 平台相关）
- **改模板 info 行文案必须同时改 label**：`mergeRow()` 中 `value` 是用户保留字段，非空 label 的 info 行只改 `:value` 时老用户 user.cfg 的旧值会覆盖新文案；同时改 label 则视为新行走模板。空 label 行（key 为空）直接取模板，改 value 即生效
- v1.580 及以前的设置文件 `config/config.properties` 与本系统**无迁移路径**（升级即回默认），文档已向用户明示；勿假设存在迁移代码
- **准星拆出 MiniHUD 的一次性迁移**：`mergeConfigs()` 发现新面板「准星」缺失时，从旧 MiniHUD 面板复制 `displayCrosshair`/`crosshairName`/`crosshairScale`→`crosshairSize`（`seedCrosshairPanel`）；`mergePanel()` 兼容 MiniHUD 启用键 `crosshairSwitch`→`miniHUDSwitch`。仅在模板变更合并/导入旧配置时触发，老版本配置不再被导入后可删
- **`crosshairScale` 是历史键名，现在只控制 MiniHUD 尺寸**（`HUDSettings.getMiniHUDScale()`）；准星尺寸走新键 `crosshairSize`。改名会重置老用户尺寸
- `Lang.mImportSuccessContent`（"请重启程序"）是死文案且说法过时——导入实际走 CONFIG_CHANGED 热重载即时生效，接入时需删掉"重启"表述
