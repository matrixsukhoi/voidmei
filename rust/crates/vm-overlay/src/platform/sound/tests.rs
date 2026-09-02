use super::*;

// ---------- WAV 解析 (跨平台纯函数) ----------

/// 构造最小合法 WAV (PCM 头 + data_len 字节静音)
fn wav_bytes(data_len: u32, byte_rate: u32, sample_rate: u32, channels: u16, bits: u16) -> Vec<u8> {
    let data = data_len.min(1_000_000) as usize;
    let mut b = Vec::with_capacity(44 + data);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&sample_rate.to_le_bytes());
    b.extend_from_slice(&byte_rate.to_le_bytes());
    b.extend_from_slice(&(channels * bits / 8).to_le_bytes()); // block align
    b.extend_from_slice(&bits.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data as u32).to_le_bytes());
    b.extend(std::iter::repeat_n(0u8, data)); // 静音
    b
}

#[test]
fn wav_duration_basic() {
    // 8000 B/s × 3200 B = 0.4s
    let b = wav_bytes(3200, 8000, 8000, 1, 8);
    let d = parse_wav_duration(&b).expect("合法 WAV 应解析");
    assert!((d - 0.4).abs() < 1e-9, "got {}", d);
}

#[test]
fn wav_extra_chunk_and_odd_padding() {
    // LIST 块 (奇数尺寸 3) 在 fmt 前: 必须按 RIFF 规范补 1 字节 pad
    // 才能继续走到 fmt/data
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes()); // RIFF 尺寸字段不参与本解析
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"LIST");
    b.extend_from_slice(&3u32.to_le_bytes());
    b.extend_from_slice(b"abc");
    b.push(0); // pad 字节
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // format
    b.extend_from_slice(&1u16.to_le_bytes()); // channels
    b.extend_from_slice(&8000u32.to_le_bytes()); // sampleRate
    b.extend_from_slice(&8000u32.to_le_bytes()); // byteRate
    b.extend_from_slice(&1u16.to_le_bytes()); // blockAlign
    b.extend_from_slice(&8u16.to_le_bytes()); // bits
    b.extend_from_slice(b"data");
    b.extend_from_slice(&1600u32.to_le_bytes());
    b.extend(std::iter::repeat_n(0, 1600));
    let d = parse_wav_duration(&b).expect("含额外块的 WAV 应解析");
    assert!((d - 0.2).abs() < 1e-9, "got {}", d);
}

#[test]
fn wav_rejects_junk() {
    assert!(parse_wav_duration(b"").is_err(), "空文件");
    assert!(parse_wav_duration(b"RIFF").is_err(), "不足 12 字节头");
    assert!(parse_wav_duration(b"RIFF____WAVX").is_err(), "非 WAVE 标签");
    assert!(parse_wav_duration(b"this is not a wav file at all!").is_err(), "纯文本");
}

#[test]
fn wav_rejects_missing_chunks() {
    // 有 fmt 无 data
    let mut no_data = wav_bytes(1600, 8000, 8000, 1, 8);
    no_data.truncate(36); // RIFF 头 + fmt 块之后截断
    assert!(parse_wav_duration(&no_data).is_err());
    // 有 data 无 fmt
    let mut no_fmt = Vec::new();
    no_fmt.extend_from_slice(b"RIFF");
    no_fmt.extend_from_slice(&100u32.to_le_bytes());
    no_fmt.extend_from_slice(b"WAVE");
    no_fmt.extend_from_slice(b"data");
    no_fmt.extend_from_slice(&8000u32.to_le_bytes());
    no_fmt.extend(std::iter::repeat_n(0, 8000));
    assert!(parse_wav_duration(&no_fmt).is_err());
}

#[test]
fn wav_clamps_data_size_to_file() {
    // data 尺寸字段虚标 0xFFFF, 实际文件尾只有 800 B → 800/8000 = 0.1s
    let mut b = wav_bytes(0xFFFF, 8000, 8000, 1, 8);
    b.truncate(36 + 8 + 800);
    let d = parse_wav_duration(&b).expect("虚标尺寸应按文件尾钳制而非报错");
    assert!((d - 0.1).abs() < 1e-9, "got {}", d);
}

// ---------- Windows 平台腿 (真实 API) ----------
#[cfg(target_os = "windows")]
mod win_tests {
    use super::super::win::WinMmSoundPlayer;
    use super::wav_bytes;
    use vm_core::audio::voice_resource_manager::SoundPlayer;

    fn temp_wav(tag: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("vm_overlay_p4_{}_{}.wav", tag, std::process::id()));
        std::fs::write(&p, bytes).expect("写临时文件失败");
        p
    }

    #[test]
    fn sound_open_rejects_missing_and_junk() {
        let player = WinMmSoundPlayer;
        // 文件不存在 (Java IOException 面)
        let missing = std::env::temp_dir().join("vm_overlay_p4_no_such_file_9f3.wav");
        assert!(player.open_clip(&missing).is_err());
        // 非音频内容 (Java UnsupportedAudioFileException 面)
        let junk = temp_wav("junk", b"this is not a wav file at all");
        let r = player.open_clip(&junk);
        let _ = std::fs::remove_file(&junk);
        assert!(r.is_err());
    }

    #[test]
    fn sound_clip_lifecycle() {
        // 环境依赖注: 无波形设备的会话 (如 CI windows runner 无音频端点) 上
        // PlaySoundW 返回 FALSE → playing 不置位 → 下方 isRunning 断言诚实
        // 失败, 不做条件跳过 (no-fake-test 纪律)
        let player = WinMmSoundPlayer;
        // 8000 B/s 静音 0.4s (源文件在播放期间保留: SND_ASYNC 下 winmm 仍读文件)
        let wav = temp_wav("life", &wav_bytes(3200, 8000, 8000, 1, 8));
        let clip = player.open_clip(&wav).expect("合法 WAV 应打开成功");
        assert!(!clip.is_running(), "未 start 前 isRunning=false (Java 新建 Clip 同)");
        assert_eq!(clip.master_gain_range(), None, "PlaySound 无增益面");
        clip.set_frame_position(0); // no-op, 不 panic
        clip.set_master_gain(0.0); // 同上
        clip.start();
        assert!(clip.is_running(), "SND_ASYNC start 后立即 isRunning=true");
        clip.stop(); // PlaySound(NULL) 终止
        assert!(!clip.is_running());
        clip.close();
        clip.close(); // 幂等 (Drop 前显式双 close)
        // 已 close 的 clip 再 start(): Java 在已 close line 上抛异常被吞 →
        // 无声; closed 守卫恢复该语义 (此路径不发 PlaySoundW, 无需音频设备)
        clip.start();
        assert!(!clip.is_running(), "close 后 start 不得出声/置位");
        drop(clip);
        let _ = std::fs::remove_file(&wav);
    }

    #[test]
    fn sound_clip_natural_completion_by_duration() {
        let player = WinMmSoundPlayer;
        // 0.05s 静音: 无 stop, 播完后 isRunning 按时长近似判 false
        let wav = temp_wav("dur", &wav_bytes(400, 8000, 8000, 1, 8));
        let clip = player.open_clip(&wav).expect("打开失败");
        clip.start();
        assert!(clip.is_running());
        std::thread::sleep(std::time::Duration::from_millis(300)); // 300ms >> 50ms
        assert!(!clip.is_running(), "自然播完: 时长近似判 false");
        drop(clip);
        let _ = std::fs::remove_file(&wav);
    }
}
