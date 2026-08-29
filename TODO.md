# TODO

- [ ] FM 缺失/损坏缺语音告警：目前只有右下角 toast 提示（`Controller.fmChangedHandler`
  在 `FM_CHANGED` 广播 `isMissingLike` 时调 `NotificationService.showBottomRight`，
  文案 `Lang.fmMissingToast` / `fmCorruptToast`），尚未接入 VoiceWarning 语音播报。
  触发点同 toast：FMManager 广播 FM_CHANGED，换机才触发一次，天然不刷屏。

## rust

- 测试改用test-expect
- 白盒测试没考虑到ui_layout.user.cfg副作用
- 语音“试听”按钮是占位(语音子系统未装配，与 UI 解耦)；openComparison/openPowerCurve 弹“阶段④”提示——这两项属后续阶段
- 点击tray icon后, 从live->preview后, 重新唤起的预览的overlay保留了上次live下的数据? minihud还有残留
- mainform的自动上下拉伸很奇怪
- 变量名和可读性
- 误报引擎转速低?
- flightinfo的所见即所得不生效
- vm-webui/vm-app 每次重编是 cmd_web 无条件 vite 重建触发的 tauri build.rs 重跑，属 build.py rust 的存量行为
- fmdata切换成json的
- 确认有无内存泄漏
-   四层搬运(TelemetrySource→getter→变量→target)收敛为直绑闭包一层。fm.* 53 个变量现在直接读 blkx 字段，不再经过 adapter 快照。

  C 级暂存通道(SessionInputs)

  22 个聚合/状态机产物(总功率/水温/WEP 消耗/引擎类型投票等)经 session_inputs() 搬运——这是 W8 之后继续消解的队列，is_downing_flap 已是第一个消解为公式的(证明通道可行)。

  剩余精简空间(按价值排序)

  1. format_strings 模板化(1018 行)——显示字符串的模板系统，最大单项
  2. SessionInputs 队列消解——check_engine_jet(vote)、rpm learn(learn_max)、update_engine_state(sum_eng 聚合原语)
  3. voice/flag 动作消费面、VoiceWarning 17 条外置(需真机验证)
- 