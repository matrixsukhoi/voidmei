# TODO

- [ ] FM 缺失/损坏缺语音告警: 目前只有右下角 toast 提示(FMManager 广播
  FM_CHANGED 且 isMissingLike 时), 尚未接入 VoiceWarning 语音播报。
  触发点同 toast: 换机才触发一次, 天然不刷屏。
- [ ] 测试改用 test-expect
- [ ] 白盒测试没考虑到 ui_layout.user.cfg 副作用
- [ ] 语音"试听"按钮是占位(语音子系统未装配, 与 UI 解耦); openComparison/openPowerCurve
  弹"阶段④"提示 — 这两项属后续阶段
- [ ] 点击 tray icon 后, 从 live → preview 重新唤起的预览 overlay 保留了上次 live 的数据?
  minihud 还有残留
- [ ] mainform 的自动上下拉伸很奇怪
- [ ] 变量名和可读性
- [ ] 误报引擎转速低?
- [ ] flightinfo 的所见即所得不生效
- [ ] webui/voidmei 每次重编是 cmd_web 无条件 vite 重建触发的 tauri build.rs 重跑,
  属 build.py rust 的存量行为
- [ ] 确认有无内存泄漏
- [ ] format_strings 模板化(显示字符串的模板系统, 最大单项)
- [ ] SessionInputs 队列消解(C 级会话聚合量: 引擎类型投票/rpm learn/sum 聚合原语)
- [ ] voice/flag 动作消费面、VoiceWarning 17 条外置(需真机验证)
