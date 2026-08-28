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
