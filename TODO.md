# TODO

- [ ] FM 缺失/损坏缺语音告警：目前只有右下角 toast 提示（`Controller.fmChangedHandler`
  在 `FM_CHANGED` 广播 `isMissingLike` 时调 `NotificationService.showBottomRight`，
  文案 `Lang.fmMissingToast` / `fmCorruptToast`），尚未接入 VoiceWarning 语音播报。
  触发点同 toast：FMManager 广播 FM_CHANGED，换机才触发一次，天然不刷屏。
