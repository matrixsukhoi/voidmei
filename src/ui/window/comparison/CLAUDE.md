# FM 对比模块（ui.window.comparison）

飞机 FM 属性并排对比，按属性胜负着色。文件：`CompactComparisonWindow`（主对比窗）、`ComparisonFrame`、`ComparisonTable`、`logic/ComparisonRules`（规则注册表）、`logic/ComparisonRule`（接口）、`logic/rules/*`（内置规则）。

## 规则系统

手工注册制：每个属性要在 `ComparisonRules` 静态块里显式注册规则才有胜负高亮；无规则的属性显示平局（灰）。流程：窗口取属性行 → `ComparisonRules.get(属性名)` → `extractValue(原始串) → Double` + `isLowerBetter()` → 定胜负（-1 左胜 / 0 平 / 1 右胜）。

规则类型（`logic/rules/`）：
- `SimpleRule.lowerIsBetter()/higherIsBetter()`——取串中第一个数字（跳过数组值）
- `ListIndexRule(index, lowerBetter)`——括号列表取指定下标，如 `[144, 1167]` 取 1
- `MultiListIndexRule(listIdx, itemIdx, lowerBetter)`——嵌套列表，如 `[8.5, -4.2], [10.1, -5.3]`
- `LambdaRule(extractor, lowerBetter)`——自定义提取（如取 `/` 后第二个数）

## 关键坑：属性名精确匹配

规则的 key 必须**逐字匹配** `Blkx.java` 经 `lang/cur.properties` 格式串产出的属性名——格式串里冒号前的整段就是属性名（如 `空重(kg): %.1f\n...` → 属性名 `空重(kg)`）。现有规则清单以 `ComparisonRules.java` 为真相源。

## UI 行为

左胜：左绿右红 + `▶`；右胜：左红右绿 + `◀`；平局（无规则或相等）：双灰 + `-`。
