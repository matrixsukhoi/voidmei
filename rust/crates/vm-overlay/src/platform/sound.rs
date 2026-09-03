//! winmm 声音播放 — javax.sound.sampled Clip/AudioSystem 使用面,
//! 实现 vm-core `voice_resource_manager::{SoundPlayer, SoundClip}` trait
//! (PORTING §3 库映射裁决: winmm PlaySound, "语音是整文件播放, 够用")。
//! 波16 自 extras.rs 按域拆出 (原三合一文件备案的拆分落地)。

use vm_core::audio::voice_resource_manager::SoundError;

// =====================================================================
// WAV 头解析 — SoundPlayer winmm 腿的文件验收面 (跨平台纯函数)
// =====================================================================

/// 校验 RIFF/WAVE 容器并返回时长 (秒)。
/// 对应 Java `AudioSystem.getAudioInputStream` 的验收面: 文件不可读/非受支持
/// 音频格式抛异常 → 本函数以 Err 收敛 (open_clip 的失败合同)。
/// PORT: 只认 RIFF/WAVE 容器 (VoidMei 语音包全为 wav); Java 另支持 AU/AIFF
/// 等, 项目内无使用面。fmt 只取 byteRate (时长计算用), 不校验压缩格式 —
/// PlaySound 能否播放某编码由其自身决定, 与 Java 解码器集的差异属已知取舍。
/// 又: 多 fmt/data 块取最后出现者、顶层 RIFF 尺寸字段不校验 (Java AudioSystem
/// 取首个且严格校验容器) — 规范语音包单块 PCM, 该分歧不可达, 备案。
pub fn parse_wav_duration(bytes: &[u8]) -> Result<f64, SoundError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut byte_rate: Option<u32> = None;
    let mut data_size: Option<u32> = None;
    // RIFF 块遍历: [4B id][4B LE size][body]; size 奇数时后随 1 字节 pad
    let mut i = 12usize;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let body = i + 8;
        if id == b"fmt " {
            if size < 16 || body + 16 > bytes.len() {
                return Err("malformed fmt chunk".into());
            }
            // fmt 布局: format(2) channels(2) sampleRate(4) byteRate(4) ...
            byte_rate = Some(u32::from_le_bytes([
                bytes[body + 8],
                bytes[body + 9],
                bytes[body + 10],
                bytes[body + 11],
            ]));
        } else if id == b"data" {
            // 尺寸字段可能虚标超出文件尾, 钳到实际可读长度
            data_size = Some(size.min(bytes.len() - body) as u32);
        }
        // 奇数尺寸的 pad 字节一并跳过; PORT: 32 位目标上 body+size (u32 虚标)
        // 可使 usize 回绕 → 理论死循环, checked 加法, 溢出视为越过文件尾结束
        // 遍历 (64 位目标无此面, 防御性)
        match body.checked_add(size).and_then(|n| n.checked_add(size & 1)) {
            Some(n) => i = n,
            None => break,
        }
    }
    match (byte_rate, data_size) {
        (Some(br), Some(ds)) if br > 0 => Ok(ds as f64 / br as f64),
        _ => Err("missing fmt/data chunk or zero byteRate".into()),
    }
}

// =====================================================================
// Windows 平台腿: winmm 播放
// =====================================================================
#[cfg(target_os = "windows")]
mod win {
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use std::time::Instant;

    use vm_core::audio::voice_resource_manager::{SoundClip, SoundError, SoundPlayer};
    use vm_core::base::logger;

    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::{
        PlaySoundW, SND_ASYNC, SND_FILENAME, SND_FLAGS, SND_NODEFAULT,
    };

    use super::parse_wav_duration;

    /// javax.sound.sampled.AudioSystem 工厂面的 winmm 实现 (D7 注入面的平台腿)。
    pub struct WinMmSoundPlayer;

    impl SoundPlayer for WinMmSoundPlayer {
        /// Java: `AudioSystem.getAudioInputStream(file)` + `getLine(info)` +
        /// `clip.open(audioStream)` 的合成; 读取/格式失败 → Err。
        /// AudioInputStream 的开关生命周期 (trait 文档注) 在本腿无对应物 —
        /// PlaySound 直接按路径播文件, open 时仅读头校验, 流即读即弃。
        fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
            let bytes = std::fs::read(path)?;
            let duration_secs = parse_wav_duration(&bytes)?;
            Ok(Box::new(WinMmClip {
                path_wide: path_as_wide(path),
                duration_secs,
                playing: AtomicBool::new(false),
                start_at: Mutex::new(None),
                closed: AtomicBool::new(false),
            }))
        }
    }

    /// javax.sound.sampled.Clip 的 winmm PlaySound 实现 (D7 SoundClip 平台腿)。
    ///
    /// PORT (PlaySound 语义边界, 平台件已知取舍):
    /// - 单通道: PlaySound 全进程同一时刻只播一路, 后 start() 打断前一路;
    ///   stop()/close() 的 PlaySound(NULL) 同为进程全局停而非仅本句柄
    ///   (Java stop/close 只作用于自身 line, 对非运行中的 line 是 no-op) —
    ///   已收窄为仅在本句柄 is_running() 时发送。判据不可用粘滞 playing:
    ///   自然播完不回写, 按它发送会使已播完 clip 的 close() 误杀他路刚
    ///   起播的新声 (消费端 reload() 对旧 clip 无条件 close 即该场景);
    ///   is_running = playing ∧ 未超时长, 严格窄于 playing, 恰好复刻
    ///   Java no-op 语义;
    ///   残余不可消除 (两路): (a) 时长近似下被 start 打断且未超自身时长窗
    ///   的 clip 仍 is_running=true, 其 stop/close 仍会误伤当前实际在播者;
    ///   (b) 跨 clip 无锁并发 — 判得 is_running=true 后、发出全局停前,
    ///   他路 start() 刚起播则被误杀 (消费端逐 alert 各持 Mutex, 不同
    ///   alert 锁不同, 窗口真实存在; 判据改 is_running 后已收窄到
    ///   is_running 为真的时段);
    ///   (Java 每 VoiceAlert 独立 Clip, 多告警可并发 — MainForm 试听按钮
    ///   (VoiceRowRenderer) 与告警并发播放即此取舍的实际受影响面;
    ///   PORTING §3 裁决 "语音是整文件播放, 够用" 即接受此限);
    /// - 无查询接口: isRunning 以 start 时刻 + WAV 时长近似 (自然播完判 false,
    ///   误差为轮询粒度; playOnce 的冷却节拍与之兼容);
    /// - 无音量面: master_gain_range() 恒 None → applyVolume 走
    ///   "Control not supported" 空 catch 分支 (§2.7 同款), 音量旋钮本腿暂失;
    /// - 异步文件生存期: PlaySound(NULL) 返回时 winmm 后台线程可能仍在读源
    ///   文件, drop 后立即删/覆盖源文件存在竞态 (install_pack 覆盖语音包
    ///   同理) — SND_ASYNC 路径固有, 已知取舍;
    /// - 每次播放天然从头: setFramePosition(0) 的回绕语义自动满足,
    ///   非零帧 seek 不支持 (全库调用点两处: VoiceWarning.playOnce 与
    ///   VoiceRowRenderer 试听按钮, 均只传 0)。
    pub struct WinMmClip {
        /// UTF-16 NUL 结尾路径 (PlaySoundW 直用; SND_ASYNC 下 winmm 后台线程
        /// 播放期间仍会读文件, 句柄存续期勿删源文件)
        path_wide: Vec<u16>,
        /// WAV 时长 (秒), is_running 的近似基准
        duration_secs: f64,
        /// start() 置 true; stop/close 置 false。自然播完不回写 — is_running()
        /// 按时长判 false; stop()/close() 的全局停发送判据用 is_running()
        /// 而非本标志 (粘滞值会误伤他路在播声音, 见 struct 文档"单通道"注)
        playing: AtomicBool,
        /// 本次 start 时刻 (is_running 时长判据)
        start_at: Mutex<Option<Instant>>,
        /// close 幂等标志 (Java line close 状态机: 已 close 再 close 无副作用)
        closed: AtomicBool,
    }

    impl WinMmClip {
        /// 停止异步播放的 MSDN 指定方式: PlaySound(pszSound=NULL)。
        /// (SND_PURGE 是 16 位遗留, Vista+ 不支持)
        fn winmm_stop() {
            unsafe {
                let _ = PlaySoundW(PCWSTR::null(), None, SND_FLAGS(0));
            }
        }
    }

    impl SoundClip for WinMmClip {
        /// Java: `clip.start()` — 非阻塞; SND_ASYNC 同为即发即忘。
        /// SND_NODEFAULT: 文件不可播时禁掉系统默认提示音 (Java 失败路径无声)。
        fn start(&self) {
            // playOnce 的 catch 吞掉 → debug 日志无声跳过; closed 守卫恢复该语义
            if self.closed.load(Ordering::SeqCst) {
                logger::debug("VoiceAlert", "PlaySoundW 播放失败: line closed");
                return;
            }
            let ok = unsafe {
                PlaySoundW(
                    PCWSTR::from_raw(self.path_wide.as_ptr()),
                    None,
                    SND_ASYNC | SND_FILENAME | SND_NODEFAULT,
                )
            };
            if ok.as_bool() {
                // 先锁写 start_at 再置 playing: 并发 is_running 不会读到
                // playing=true + start_at=None 的瞬态假阴性 (Java Clip 内部
                // 同步, start 返回后即恒 true)
                *self.start_at.lock().unwrap() = Some(Instant::now());
                self.playing.store(true, Ordering::SeqCst);
            } else {
                // ("播放失败: {key} - {msg}", key 属 VoiceAlert 层上下文,
                // clip 层不可得); trait 无 Result 返回面, 就地落同语义 debug
                // 日志, 拼 GetLastError 便于现场排障 (PlaySoundW 返回 BOOL,
                // winmm 不保证失败时设置 last error, 0/脏值仅作参考)
                let err = unsafe { windows::Win32::Foundation::GetLastError() };
                logger::debug(
                    "VoiceAlert",
                    &format!("PlaySoundW 播放失败: GetLastError={}", err.0),
                );
            }
        }

        /// Java: `clip.stop()` — 停止播放但行保留, 可再 start。
        fn stop(&self) {
            // PlaySound(NULL) 是进程全局停: 发送判据用 is_running() 而非粘滞
            // playing — Java stop() 对非运行中的 line 是 no-op, 绝不互扰;
            // playing 在自然播完后仍为 true, 按它发送会让"已播完 clip 的
            // stop"误伤他路正在播的声音 (is_running 严格窄于 playing,
            // 见 struct 文档"单通道"注)
            if self.is_running() {
                WinMmClip::winmm_stop();
            }
            self.playing.store(false, Ordering::SeqCst);
            *self.start_at.lock().unwrap() = None;
        }

        /// Java: `clip.isRunning()` — 播放进行中 true, 自然结束/停止后 false。
        /// PlaySound 无查询面: 以 start 时刻 + 时长近似; 过期后不回写 playing
        /// (无 &mut), 下次调用同样判 false, 语义等价。
        fn is_running(&self) -> bool {
            if !self.playing.load(Ordering::SeqCst) {
                return false;
            }
            match *self.start_at.lock().unwrap() {
                None => false,
                Some(t) => t.elapsed().as_secs_f64() < self.duration_secs,
            }
        }

        /// Java: `clip.setFramePosition(int)` — 见 struct 文档 (PlaySound 每次
        /// 从头播放, 全库两处调用点均传 0, 语义自动满足)。
        fn set_frame_position(&self, _frame: i32) {}

        /// Java: `clip.close()` — 释放行资源并停止播放; 幂等 (Drop 兜底再调)。
        fn close(&self) {
            if self.closed.swap(true, Ordering::SeqCst) {
                return;
            }
            // 同 stop(): 判据 is_running() — 消费端 reload() 对旧 clip 无条件
            // close (voice_warning.rs), 已自然播完的旧 clip 若按粘滞 playing
            // 发送全局停, 会切断他路刚起播的新告警 (B1 修复点)
            if self.is_running() {
                WinMmClip::winmm_stop();
            }
            self.playing.store(false, Ordering::SeqCst);
            *self.start_at.lock().unwrap() = None;
        }

        /// PlaySound 无每句柄增益控制 → None (Java FloatControl 缺失同款,
        /// applyVolume 的空 catch 分支吞掉, §2.7)。
        fn master_gain_range(&self) -> Option<(f32, f32)> {
            None
        }

        /// master_gain_range() 恒 None ⇒ applyVolume 不会调到这里; 空实现保留接口。
        fn set_master_gain(&self, _value: f32) {}
    }

    impl Drop for WinMmClip {
        fn drop(&mut self) {
            // RAII 兜底契约 (trait 文档): Drop 等价 close(), 显式 close 后幂等
            self.close();
        }
    }

    /// 路径 → UTF-16 NUL 结尾 (PCWSTR 要求; 非法单元照搬, Win32 路径语义)
    fn path_as_wide(path: &Path) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);
        wide
    }
}

#[cfg(target_os = "windows")]
pub use win::{WinMmClip, WinMmSoundPlayer};

// Tests — WAV 解析跨平台单测, Win32 腿以真实 API 冒烟 (win.rs 真实窗口
// 测试同款先例), 全部断言真实行为, 不做条件跳过。
#[cfg(test)]
mod tests;
