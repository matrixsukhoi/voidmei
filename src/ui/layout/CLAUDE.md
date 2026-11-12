# 设置面板布局（ui.layout）

本包只负责把 `ui_layout.cfg` 动态生成设置面板：`FlexGridLayout`/`ResponsiveGridLayout` + `renderer/` 下的行渲染器（switch/slider/combo/data 等）。

**MiniHUD 已不再使用本包**。MiniHUD 是自由画布：组件保存绝对坐标，没有父链/锚点/DAG；详见根 `CLAUDE.md`、`src/ui/overlay/CLAUDE.md`、`src/ui/component/CLAUDE.md`。

## 常见坑

| 症状 | 原因 |
|---|---|
| 新行类型不显示 | 未在 `RowRendererRegistry` 注册 renderer |
| 设置改动不即时生效 | 配置键没进 `Controller` 对应 overlay 的 `.withInterest()` |
| hidden 行占位/崩溃 | hidden renderer 返回 null；动态页需容忍 null 组件 |
