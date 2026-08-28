# TODO

- [ ] FM 缺失/损坏缺语音告警：目前只有右下角 toast 提示（`Controller.fmChangedHandler`
  在 `FM_CHANGED` 广播 `isMissingLike` 时调 `NotificationService.showBottomRight`，
  文案 `Lang.fmMissingToast` / `fmCorruptToast`），尚未接入 VoiceWarning 语音播报。
  触发点同 toast：FMManager 广播 FM_CHANGED，换机才触发一次，天然不刷屏。

## rust

- 测试改用test-expect, 放到独立文件
- 白盒测试没考虑到ui_layout.user.cfg副作用
- mainform行为不一致
- gamemode下一直在写入配置文件
[18:22:21.450] [ConfigurationService] ACTION: ConfigurationService: Saving to ./ui_layout.user.cfg
[18:22:21.451] [ConfigurationService] ACTION: ConfigurationService: Saving to ./ui_layout.user.cfg
[18:22:21.452] [ConfigurationService] ACTION: ConfigurationService: Saving to ./ui_layout.user.cfg
- gamemode -> live
- 语音“试听”按钮是占位(语音子系统未装配，与 UI 解耦)；openComparison/openPowerCurve 弹“阶段④”提示——这两项属后续阶段
- 点击tray icon重新唤起的预览的overlay保留了上次gamemode下的数据?
- 关闭mainform时没有退出游戏
- 编译速度慢
- mainform的自动上下拉伸很奇怪

