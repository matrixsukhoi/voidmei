//! voidmei 语音播放平台件: winmm **waveOut 每路独立流**实现
//! `kernel::audio::voice_resource_manager::{SoundPlayer, SoundClip}` (D7 注入面的组装腿)。
//!
//! ## 播放模型裁决 (语音子系统装配批, 2026-08-28)
//!
//! 精读 Java VoiceWarning 的告警触发路径, 判定 Java 存在**多个 Clip 并发混音**:
//! 1. 每个 VoiceAlert 持有独立 Clip (`VoiceResourceManager.loadClip` 每次经
//!    `AudioSystem.getLine` 新建 line);
//! 2. run() 单轮 (100ms tick) 内十余个 check* 互相独立、可同轮齐发 —— 其中
//!    checkSpeedWarning 的 iasWarn 与 machWarn 是**同方法两个非互斥 if**
//!    (俯冲中 IAS≥vne*0.95 与 M≥限速可同时成立),
//!    另有 brake/engOverheat/stall/loadFactor/rudderEff/aileronEff 等同轮可达;
//! 3. 跨轮重叠: 告警音普遍长于 100ms 节拍, 冷却只压制**同一**告警的重触发,
//!    不同告警天然叠音 (如 engWarn 长音在播期间 stallWarn 起播);
//! 4. MainForm 试听按钮与告警并发, 各持独立 Clip。
//! javax.sound.sampled 的多 line 由系统混音器合成 ⇒ Java 语义 = 并发混音。
//!
//! ⇒ **裁决: 并发混音 → waveOut 每路独立流** (每 Clip 一个 HWAVEOUT;
//! Vista+ 会话混音器自动合成多路)。overlay platform/sound 的 PlaySound 腿
//! 是单通道抢占制 (P4 期 "整文件播放够用" 的旧裁决), 无法满足
//! 并发混音, 组装层弃用之; 该腿与其测试原地保留, 回收归 overlay 波次备案。
//!
//! ## 与 Java 语义的对齐/取舍
//! - `open_clip` = Java `getAudioInputStream` + `getLine` + `clip.open` 的合成:
//!   文件读失败 (IOException) / 非受支持格式 (UnsupportedAudioFileException) /
//!   行资源不可用 (LineUnavailableException → waveOutOpen 失败) 三面均 → Err,
//!   由 `loadClip` 的 catch 统一落日志返 None (Java catch→null 语义)。
//! - 每 Clip 的 waveOut 设备在 open_clip 时打开 (对位 Java line 资源随 open
//!   分配), close/Drop 释放 (RAII 兜底契约, trait 文档)。
//! - 增益: waveOut 无 per-line dB 控件, 以**样本幅度缩放**复刻 Java
//!   FloatControl.MASTER_GAIN (幅度 = 10^(dB/20)); 报告范围 (-80, +6.02) dB
//!   = Java 源注释 "approx 80"/"approx 6" 的直译 (Direct Audio Device 典型值),
//!   applyVolume 的对数衰减数学因此完整生效 (音量旋钮不失效)。
//! - set_frame_position: 全库调用点两处 (playOnce/试听) 均传 0 —— 实现"停止
//!   在播 + 重定位游标", 字节游标 = 帧 × blockAlign, 非零 seek 一并支持。

// Path/SoundClip 仅非 Windows 回退腿使用 (windows 腿在 winmm 模块内自取)
#[cfg(not(target_os = "windows"))]
use std::path::Path;
#[cfg(not(target_os = "windows"))]
use kernel::audio::voice_resource_manager::SoundClip;

use kernel::audio::voice_resource_manager::{SoundError, SoundPlayer};

// =====================================================================
// WAV PCM 解析 (纯函数, 跨平台可测 — overlay parse_wav_duration 的原姊妹面)
// =====================================================================

/// 解析出的 PCM 播放参数 + 数据切片
pub(crate) struct WavPcm<'a> {
    pub channels: u16,
    pub samples_per_sec: u32,
    pub bits_per_sample: u16,
    /// 帧字节数 (channels × bits/8), 帧游标换算基准
    pub block_align: u16,
    pub data: &'a [u8],
}

/// 校验 RIFF/WAVE 容器并提取首个 fmt + data 块 (Java AudioSystem 同取首个;
/// overlay parse_wav_duration 取末个的分歧在规范单块语音包下不可达, 备案)。
///
/// PORT (验收边界): 只认 PCM (wFormatTag=1)、mono/stereo、8/16/24/32-bit ——
/// VoidMei 语音包全为该域; Java 另支持 AU/AIFF/float 等, 项目内无使用面。
fn parse_wav_pcm(bytes: &[u8]) -> Result<WavPcm<'_>, SoundError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut fmt: Option<(u16, u16, u32, u16)> = None; // (tag, ch, rate, bits)
    let mut data: Option<&[u8]> = None;
    // RIFF 块遍历: [4B id][4B LE size][body]; size 奇数时后随 1 字节 pad
    let mut i = 12usize;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let body = i + 8;
        if id == b"fmt " && fmt.is_none() {
            if size < 16 || body + 16 > bytes.len() {
                return Err("malformed fmt chunk".into());
            }
            // fmt 布局: format(2) channels(2) sampleRate(4) byteRate(4) blockAlign(2) bits(2)
            let tag = u16::from_le_bytes([bytes[body], bytes[body + 1]]);
            let channels = u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]);
            let rate = u32::from_le_bytes([
                bytes[body + 4],
                bytes[body + 5],
                bytes[body + 6],
                bytes[body + 7],
            ]);
            let bits = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
            fmt = Some((tag, channels, rate, bits));
        } else if id == b"data" && data.is_none() {
            // 尺寸字段可能虚标超出文件尾, 钳到实际可读长度
            let end = body.saturating_add(size).min(bytes.len());
            data = Some(&bytes[body..end]);
        }
        // 奇数尺寸的 pad 字节一并跳过; 32 位目标上 body+size (u32 虚标)
        // 可使 usize 回绕 → 理论死循环, checked 加法, 溢出视为越过文件尾结束遍历
        match body.checked_add(size).and_then(|n| n.checked_add(size & 1)) {
            Some(n) => i = n,
            None => break,
        }
    }
    let Some((tag, channels, rate, bits)) = fmt else {
        return Err("missing fmt chunk".into());
    };
    let Some(data) = data else {
        return Err("missing data chunk".into());
    };
    if tag != 1 {
        return Err(format!("unsupported wav format tag {tag} (仅 PCM)").into());
    }
    if rate == 0 {
        return Err("zero sample rate".into());
    }
    if !(1..=2).contains(&channels) {
        // waveOut 经典 PCM 路径仅 mono/stereo; 多声道需 WAVEFORMATEXTENSIBLE
        return Err(format!("unsupported channel count {channels}").into());
    }
    if !matches!(bits, 8 | 16 | 24 | 32) {
        return Err(format!("unsupported bits per sample {bits}").into());
    }
    Ok(WavPcm {
        channels,
        samples_per_sec: rate,
        bits_per_sample: bits,
        block_align: channels * (bits / 8),
        data,
    })
}

/// PCM 幅度缩放 (增益腿): 按位宽解包 → ×amp (饱和) → 回写 `dst`。
/// 增益对全部声道统一施加 (= Java MASTER_GAIN 的 uniform 语义)。
/// 8-bit 无符号 / 其余为小端有符号整型 (WAVE_FORMAT_PCM 规范)。
fn scale_pcm_into(src: &[u8], bits: u16, amp: f32, dst: &mut Vec<u8>) {
    dst.clear();
    dst.reserve(src.len());
    match bits {
        8 => {
            // 无符号 8-bit: 中心 128
            for &b in src {
                let v = ((b as f32 - 128.0) * amp + 128.0).round().clamp(0.0, 255.0);
                dst.push(v as u8);
            }
        }
        16 => {
            for c in src.chunks_exact(2) {
                let s = i16::from_le_bytes([c[0], c[1]]) as f32 * amp;
                let v = s.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                dst.extend_from_slice(&v.to_le_bytes());
            }
        }
        24 => {
            for c in src.chunks_exact(3) {
                // 符号扩展: 低 24 位 → i32 (左移 8 后算术右移 8)
                let raw = u32::from_le_bytes([c[0], c[1], c[2], 0]);
                let s = ((raw as i32) << 8) >> 8;
                const MAX: i32 = 0x7F_FFFF;
                const MIN: i32 = -0x80_0000;
                let v = ((s as f32 * amp).round().clamp(MIN as f32, MAX as f32)) as i32;
                dst.extend_from_slice(&v.to_le_bytes()[0..3]);
            }
        }
        32 => {
            for c in src.chunks_exact(4) {
                let s = i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 * amp;
                let v = s.round().clamp(i32::MIN as f32, i32::MAX as f32) as i32;
                dst.extend_from_slice(&v.to_le_bytes());
            }
        }
        _ => unreachable!("bits 已在 parse_wav_pcm 白名单校验"),
    }
}

// =====================================================================
// Windows 腿: waveOut 播放器
// =====================================================================

#[cfg(target_os = "windows")]
mod winmm {
    use std::cell::UnsafeCell;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use std::sync::Mutex;

    use kernel::audio::voice_resource_manager::{SoundClip, SoundError, SoundPlayer};
    use kernel::base::logger;

    use super::{parse_wav_pcm, scale_pcm_into};
    use windows::core::PSTR;
    use windows::Win32::Media::Audio::{
        waveOutClose, waveOutGetErrorTextW, waveOutOpen, waveOutPrepareHeader, waveOutReset,
        waveOutUnprepareHeader, waveOutWrite, CALLBACK_NULL, HWAVEOUT, WAVEFORMATEX, WAVEHDR,
        WAVE_FORMAT_PCM, WAVE_MAPPER, WHDR_DONE,
    };
    use windows::Win32::Media::MMSYSERR_NOERROR;

    /// Java FloatControl.Type.MASTER_GAIN 的典型范围 (VoiceResourceManager.java
    /// "approx 80"/"approx 6" 注释的直译; Direct Audio Device 实测值)
    const GAIN_MIN: f32 = -80.0;
    const GAIN_MAX: f32 = 6.0206;

    /// mmr 错误码 → 可读文本 (waveOutGetErrorTextW 失败时退回裸码)
    fn mmr_text(mmr: u32) -> String {
        let mut buf = [0u16; 256];
        let ok = unsafe { waveOutGetErrorTextW(mmr, &mut buf) };
        if ok == MMSYSERR_NOERROR {
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            String::from_utf16_lossy(&buf[..end])
        } else {
            format!("mmr={mmr}")
        }
    }

    /// start 的缩放缓冲维护 (提出为自由函数: 纯逻辑, 无设备依赖, 可单测)。
    /// 缓存键 = (幅度 f32 位模式, 数据游标): 均未变 → 复用上次缓冲, 跳过全量
    /// PCM 重缩放 (备案收口: 避免重复播放热点的重缩放开销); 任一变化 → 重建。
    fn ensure_scaled(cache: &mut ScaledBuf, src: &[u8], offset: usize, bits: u16, amp: f32) {
        let cur_amp_bits = amp.to_bits();
        if cache.amp_bits != cur_amp_bits || cache.offset != offset {
            scale_pcm_into(&src[offset..], bits, amp, &mut cache.buf);
            cache.amp_bits = cur_amp_bits;
            cache.offset = offset;
        }
    }

    /// javax.sound.sampled.AudioSystem 工厂面的 waveOut 实现。
    pub struct WaveOutPlayer;

    impl SoundPlayer for WaveOutPlayer {
        /// Java: `getAudioInputStream(file)` + `getLine(info)` + `clip.open(audioStream)`
        /// 的合成; 读取/格式/行资源失败 → Err (loadClip catch→None 语义的对端)。
        fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
            let bytes = std::fs::read(path)?;
            let wav = parse_wav_pcm(&bytes)?;
            // 畸形头 rate 可虚标近 u32::MAX, rate×blockAlign 的 u32 乘法
            // 溢出 (debug panic/release 回绕) — 对齐 open_clip 其余错误面: 拒绝 →
            // Err → loadClip catch → None
            let avg_bytes = wav
                .samples_per_sec
                .checked_mul(wav.block_align as u32)
                .ok_or_else(|| {
                    format!(
                        "byte rate 溢出: {} Hz × {} B/帧",
                        wav.samples_per_sec, wav.block_align
                    )
                })?;
            // 行资源获取面 (Java getLine+open / LineUnavailableException):
            // WAVE_MAPPER 按格式自动选设备; CALLBACK_NULL = 无回调 (轮询 WHDR_DONE)
            let wfx = WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_PCM as u16,
                nChannels: wav.channels,
                nSamplesPerSec: wav.samples_per_sec,
                nAvgBytesPerSec: avg_bytes,
                nBlockAlign: wav.block_align,
                wBitsPerSample: wav.bits_per_sample,
                cbSize: 0,
            };
            let mut hwo = HWAVEOUT::default();
            let mmr = unsafe {
                waveOutOpen(Some(&mut hwo), WAVE_MAPPER, &wfx, None, None, CALLBACK_NULL)
            };
            if mmr != MMSYSERR_NOERROR {
                return Err(format!("waveOutOpen 失败: {}", mmr_text(mmr)).into());
            }
            Ok(Box::new(WaveOutClip {
                hwo,
                bits: wav.bits_per_sample,
                block_align: wav.block_align,
                data: wav.data.to_vec(),
                play: Mutex::new(ScaledBuf::new()),
                hdr: UnsafeCell::new(WAVEHDR::default()),
                start_offset: AtomicUsize::new(0),
                gain_db: AtomicU32::new(0.0f32.to_bits()),
                written: AtomicBool::new(false),
                closed: AtomicBool::new(false),
            }))
        }
    }

    /// 增益施加后的提交缓冲 + 生成它的缓存键 (维护逻辑见 [`ensure_scaled`])。
    /// 幅度位模式与 gain_db 一一对应 — powf 是函数且 gain 被 clamp 在无 NaN 域,
    /// 同增益必同幅度; data 不可变, 游标钉住输入切片。
    struct ScaledBuf {
        buf: Vec<u8>,
        amp_bits: u32,
        offset: usize,
    }

    impl ScaledBuf {
        /// 初始键取不可能命中的值: amp_bits=0 是 +0.0 的位模式, 而 clamp 后
        /// 增益的幅度最小 10^(-80/20)=1e-4 永非 0; offset=usize::MAX 超出
        /// start_offset 的 min(data.len()) 钳制域。首播必走缩放分支。
        fn new() -> Self {
            Self {
                buf: Vec::new(),
                amp_bits: 0,
                offset: usize::MAX,
            }
        }
    }

    /// javax.sound.sampled.Clip 的 waveOut 实现 (D7 SoundClip 平台腿)。
    ///
    /// 并发契约 (trait 文档): 消费端 (voice_warning.rs) 经 Mutex 独占访问,
    /// 本类型的 &self 方法不会并发互调; 唯一的外部并发写者是 winmm 后台线程
    /// 对 WAVEHDR.dwFlags 的回写 (is_running 以非对齐裸指针读观测, 见 flags)。
    ///
    /// 内存契约: WAVEHDR.lpData 指向 `play` 堆缓冲 —— start()/set_frame_position()/
    /// close() 均先 retire_pending() 终止 winmm 对旧提交的异步读取, 之后才经
    /// scale_pcm_into 重建缓冲 (clear+reserve, 容量不足会 realloc 换堆块);
    /// 缓存命中路径不改写缓冲 (更弱的操作, 顺序约束天然满足); 提交期间
    /// (written=true) 缓冲不再被改写, 且被 play 锁保活。
    pub struct WaveOutClip {
        /// winmm 输出设备句柄 (open_clip 获取, close/Drop 释放)
        hwo: HWAVEOUT,
        bits: u16,
        block_align: u16,
        /// 原始 PCM 数据 (Java: Clip 将音频数据复制进内存的对应物)
        data: Vec<u8>,
        /// 增益施加后的实际提交缓冲 + 缓存键 (start() 维护; winmm 播放期间异步读取)
        play: Mutex<ScaledBuf>,
        /// winmm 回写 dwFlags 的 FFI 外部变更面 (UnsafeCell 承载 &self 变异;
        /// WAVEHDR 为 packed(1), 字段访问经 addr_of 裸指针避免 packed 引用)
        hdr: UnsafeCell<WAVEHDR>,
        /// setFramePosition 的字节游标 (帧 × blockAlign, 钳到数据尾)
        start_offset: AtomicUsize,
        /// 增益 (dB, f32 位模式) — Java: load 时经 applyVolume 写入控件
        gain_db: AtomicU32,
        /// waveOutWrite 已提交且未 unprepare (is_running 的前置)
        written: AtomicBool,
        /// close 幂等标志 (Java line close 状态机: 已 close 再 close 无副作用)
        closed: AtomicBool,
    }

    impl WaveOutClip {
        /// 头裸指针 (packed 结构, 字段级访问一律经此 + addr_of)
        fn hdr_ptr(&self) -> *mut WAVEHDR {
            self.hdr.get()
        }

        /// 读 dwFlags (winmm 后台线程并发回写 WHDR_DONE; WAVEHDR packed(1) —
        /// 字段可能非对齐, 用 read_unaligned 保证布局健全; 每次调用经裸指针
        /// 现读, 无跨调用缓存面)。
        /// PORT(备案, 审查轮 A-W): read_unaligned 是普通读非 volatile — 与
        /// winmm 线程回写构成理论数据竞争 (std无非对齐 volatile 读 API,
        /// read_volatile 要求对齐); x86 对齐 u32 载荷实践无碍,
        /// WHDR_DONE 观测最坏滞后一轮 (is_running 假阳性一拍), 无正确性面
        fn flags(&self) -> u32 {
            let hdr = self.hdr_ptr();
            unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*hdr).dwFlags)) }
        }

        /// 归还已准备的头 (前置: WHDR_DONE 已置位 —— reset 或自然播完)
        fn unprepare(&self) {
            unsafe {
                let mmr = waveOutUnprepareHeader(
                    self.hwo,
                    self.hdr_ptr(),
                    std::mem::size_of::<WAVEHDR>() as u32,
                );
                if mmr != MMSYSERR_NOERROR {
                    // Java close 路径空 catch 吞掉; WARN 留痕便于排障
                    logger::warn(
                        "VoiceAlert",
                        &format!("waveOutUnprepareHeader: {}", mmr_text(mmr)),
                    );
                }
            }
            self.written.store(false, Ordering::SeqCst);
        }

        /// 停止在播并归还头 (reset 令 winmm 立即标记 WHDR_DONE)
        fn retire_pending(&self) {
            if self.written.load(Ordering::SeqCst) {
                unsafe {
                    let _ = waveOutReset(self.hwo);
                }
                self.unprepare();
            }
        }
    }

    impl SoundClip for WaveOutClip {
        /// Java: `clip.start()` — 非阻塞 (waveOutWrite 即发即忘, winmm 线程播出)。
        fn start(&self) {
            // playOnce 的 catch 吞掉 → debug 日志无声跳过; closed 守卫恢复该语义
            if self.closed.load(Ordering::SeqCst) {
                logger::debug("VoiceAlert", "waveOut 播放失败: line closed");
                return;
            }
            let offset = self
                .start_offset
                .load(Ordering::SeqCst)
                .min(self.data.len());
            // 幅度 = 10^(dB/20) (FloatControl 增益的线性域换算)
            let amp = 10f32.powf(f32::from_bits(self.gain_db.load(Ordering::SeqCst)) / 20.0);
            let mut play = self.play.lock().expect("WaveOutClip play 缓冲锁中毒");
            // 重启路径: 在播的头**先**归还再改写缓冲 (对位 Java setFramePosition(0)
            // 的干净停跳)。顺序是正确性前提 (审查轮 B-B1): scale_pcm_into 会
            // clear+reserve (容量不足即 realloc 释放旧堆块), 而 winmm 设备线程在
            // 头归还 (waveOutReset + unprepare) 前仍持 lpData 异步读取旧提交 —
            // 先改写即"在播尾段被污染"且旧头内指针悬垂 (use-after-free 面)
            self.retire_pending();
            // 增益缓存 (备案收口): 键未变 → 复用上次提交缓冲, 跳过全量重缩放;
            // 键变 (增益或游标) → 重建并更新键
            ensure_scaled(&mut play, &self.data, offset, self.bits, amp);
            let play = &mut play.buf;
            if play.is_empty() {
                // 空数据 (零长 data 块/游标钳到尾): Java 播放无声且 isRunning 即 false
                return;
            }
            unsafe {
                let hdr = self.hdr_ptr();
                std::ptr::addr_of_mut!((*hdr).lpData).write_unaligned(PSTR(play.as_mut_ptr()));
                std::ptr::addr_of_mut!((*hdr).dwBufferLength).write_unaligned(play.len() as u32);
                std::ptr::addr_of_mut!((*hdr).dwBytesRecorded).write_unaligned(0);
                std::ptr::addr_of_mut!((*hdr).dwFlags).write_unaligned(0); // 清 winmm 写回的 DONE
                let mmr =
                    waveOutPrepareHeader(self.hwo, hdr, std::mem::size_of::<WAVEHDR>() as u32);
                if mmr != MMSYSERR_NOERROR {
                    logger::debug(
                        "VoiceAlert",
                        &format!("waveOutPrepareHeader 失败: {}", mmr_text(mmr)),
                    );
                    return;
                }
                let mmr = waveOutWrite(self.hwo, hdr, std::mem::size_of::<WAVEHDR>() as u32);
                if mmr != MMSYSERR_NOERROR {
                    logger::debug(
                        "VoiceAlert",
                        &format!("waveOutWrite 失败: {}", mmr_text(mmr)),
                    );
                    self.unprepare();
                    return;
                }
            }
            self.written.store(true, Ordering::SeqCst);
        }

        /// Java: `clip.stop()` — 停止播放但行保留, 可再 start。
        fn stop(&self) {
            // Java stop() 对非运行中的 line 是 no-op; waveOutReset 是设备局部停
            // (每 clip 独立 HWAVEOUT, 不影响他路 — PlaySound 全局停的旧缺陷不复存)
            self.retire_pending();
        }

        /// Java: `clip.isRunning()` — WHDR_DONE 未置位即在播 (winmm 的完成标记)。
        fn is_running(&self) -> bool {
            self.written.load(Ordering::SeqCst) && (self.flags() & WHDR_DONE) == 0
        }

        /// Java: `clip.setFramePosition(int)` — 停止在播 + 定位游标 (帧×blockAlign,
        /// 钳到数据尾; 负帧按 0)。全库调用点均传 0 (playOnce/试听), 非零一并支持。
        fn set_frame_position(&self, frame: i32) {
            self.retire_pending();
            let off = frame.max(0) as usize * self.block_align as usize;
            self.start_offset
                .store(off.min(self.data.len()), Ordering::SeqCst);
        }

        /// Java: `clip.close()` — 释放行资源并停止播放; 幂等 (Drop 兜底再调)。
        fn close(&self) {
            if self.closed.swap(true, Ordering::SeqCst) {
                return;
            }
            self.retire_pending();
            let mmr = unsafe { waveOutClose(self.hwo) };
            if mmr != MMSYSERR_NOERROR {
                // Java close 异常被吞 (空 catch); WARN 留痕 (句柄泄漏观测点)
                logger::warn("VoiceAlert", &format!("waveOutClose: {}", mmr_text(mmr)));
            }
        }

        /// Java: `(FloatControl) clip.getControl(MASTER_GAIN)` 的 min/max。
        /// waveOut 无 per-line dB 控件 → 报告 Java 典型范围, 增益经样本缩放复刻。
        fn master_gain_range(&self) -> Option<(f32, f32)> {
            Some((GAIN_MIN, GAIN_MAX))
        }

        /// Java: `gainControl.setValue(val)` — FloatControl 自动钳到控件范围。
        fn set_master_gain(&self, value: f32) {
            self.gain_db
                .store(value.clamp(GAIN_MIN, GAIN_MAX).to_bits(), Ordering::SeqCst);
        }
    }

    impl Drop for WaveOutClip {
        fn drop(&mut self) {
            // RAII 兜底契约 (trait 文档): Drop 等价 close(), 显式 close 后幂等
            self.close();
        }
    }

    // Send 安全论证 (SoundClip: Send 的实现前提): HWAVEOUT/WAVEHDR/PSTR 内的
    // 裸指针是 winmm 设备/头句柄, 非线程亲和数据 —— 本类型的 &self 方法全部
    // 经消费端 Mutex 串行化 (trait 文档契约), 跨线程 move 仅转移所有权不产生
    // 并发访问; 唯一的外部并发写者是 winmm 后台线程对 dwFlags 的 u32 回写,
    // 由 flags() 的非对齐现读观测 (见其 PORT 备案)。头/缓冲在提交期间由 play
    // 锁 + retire 先行序保活 (见 struct 内存契约)。
    unsafe impl Send for WaveOutClip {}

}

// =====================================================================
// 平台分派 (组装层唯一入口)
// =====================================================================

/// 组装层播放器工厂: Windows → waveOut 每路独立流 (见模块头裁决);
/// 非 Windows → 显式未移植 Err (x11 波次; 不假成功)。
#[cfg(target_os = "windows")]
pub fn make_player() -> Box<dyn SoundPlayer> {
    Box::new(winmm::WaveOutPlayer)
}

#[cfg(not(target_os = "windows"))]
pub fn make_player() -> Box<dyn SoundPlayer> {
    struct UnportedPlayer;
    impl SoundPlayer for UnportedPlayer {
        fn open_clip(&self, _path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
            Err("语音播放未移植 (非 Windows 平台, x11 波次)".into())
        }
    }
    Box::new(UnportedPlayer)
}

