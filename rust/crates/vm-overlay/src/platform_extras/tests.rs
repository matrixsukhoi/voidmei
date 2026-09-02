use super::*;

// ---------- DPI 纯计算 (跨平台) ----------

#[test]
fn dpi_200pct_logical_and_scale_math() {
    let d = DpiHelper::from_detection(3840, 2160, 2.0, 2.0);
    assert_eq!(d.get_scale_x(), 2.0);
    assert_eq!(d.get_scale_y(), 2.0);
    assert_eq!(d.get_scale(), 2.0, "主因子 = 水平 (Java getScale)");
    assert_eq!(d.get_logical_screen_width(), 1920);
    assert_eq!(d.get_logical_screen_height(), 1080);
    assert_eq!(d.get_physical_screen_width(), 3840);
    assert_eq!(d.get_physical_screen_height(), 2160);
    assert_eq!(d.scale(800), 1600);
    assert_eq!(d.scale(24), 48);
    assert_eq!(d.unscale(48), 24);
    assert!(d.is_high_dpi());
}

#[test]
fn dpi_150pct() {
    let d = DpiHelper::from_detection(3000, 2000, 1.5, 1.5);
    assert_eq!(d.get_logical_screen_width(), 2000);
    assert_eq!(d.get_logical_screen_height(), 1333); // 2000/1.5=1333.33 → round 1333
    assert_eq!(d.scale(100), 150);
    assert!(d.is_high_dpi());
}

#[test]
fn dpi_scale_half_up_rounding_125pct() {
    // §2.3 钉子: Java Math.round(12.5)=13 (floor(x+0.5));
    // Rust f64::round 半偶舍入会给 12, 不可用
    let d = DpiHelper::from_detection(2400, 1350, 1.25, 1.25);
    assert_eq!(d.scale(10), 13);
    assert_eq!(d.unscale(13), 10); // 13/1.25=10.4 → 10
    assert_eq!(d.scale_f64(10.0), 12.5); // double 版不取整
}

#[test]
fn dpi_zero_scale_guard() {
    // Java `if (scaleX > 0)` else 分支: logical=physical;
    // unscale 的 `if (scaleX == 0) return scaledValue` 卫语句
    let d = DpiHelper::from_detection(1920, 1080, 0.0, 0.0);
    assert_eq!(d.get_logical_screen_width(), 1920);
    assert_eq!(d.get_logical_screen_height(), 1080);
    assert!(!d.is_high_dpi());
    assert_eq!(d.unscale(55), 55);
}

#[test]
fn dpi_is_high_dpi_boundary() {
    assert!(!DpiHelper::from_detection(100, 100, 1.0, 1.0).is_high_dpi());
    assert!(!DpiHelper::from_detection(100, 100, 1.01, 1.01).is_high_dpi(), "严格 > 1.01");
    assert!(DpiHelper::from_detection(100, 100, 1.011, 1.0).is_high_dpi());
    assert!(DpiHelper::from_detection(100, 100, 1.0, 1.02).is_high_dpi(), "y 单独超标即高 DPI");
}

#[test]
fn dpi_fallback_defaults() {
    let d = DpiHelper::fallback(1920, 1080, "GetDpiForMonitor: test");
    assert_eq!(d.get_scale_x(), 1.0);
    assert_eq!(d.get_scale_y(), 1.0);
    assert_eq!(d.get_logical_screen_width(), 1920);
    assert_eq!(d.get_logical_screen_height(), 1080);
    assert!(!d.is_high_dpi());
}

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
    b.extend(std::iter::repeat_n(0u8, 1600));
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
    no_fmt.extend(std::iter::repeat_n(0u8, 8000));
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
    use super::super::win::{
        file_name_after_last_backslash, is_war_thunder_focused, process_image_name,
        WinMmSoundPlayer, WindowsFocusDetector,
    };
    use super::super::DpiHelper;
    use super::wav_bytes;
    use vm_core::platform::focus_monitor::FocusDetector;
    use vm_core::audio::voice_resource_manager::SoundPlayer;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::GetCurrentProcessId;

    // -- DPI 检测 (真实主屏) --

    #[test]
    fn dpi_init_real_detection() {
        let d = DpiHelper::init();
        let (pw, ph) = (d.get_physical_screen_width(), d.get_physical_screen_height());
        assert!(pw > 0 && ph > 0, "桌面会话主屏物理尺寸应为正, got {}x{}", pw, ph);
        let (sx, sy) = (d.get_scale_x(), d.get_scale_y());
        assert!(sx > 0.0 && sy > 0.0);
        // logical 语义钉子: Java (int) Math.round(physical / scale) (scale>0 分支)
        assert_eq!(d.get_logical_screen_width(), ((pw as f64 / sx) + 0.5).floor() as i32);
        assert_eq!(d.get_logical_screen_height(), ((ph as f64 / sy) + 0.5).floor() as i32);
        assert_eq!(d.is_high_dpi(), sx > 1.01 || sy > 1.01);
    }

    #[test]
    fn dpi_for_window_invalid_falls_back() {
        // 无效句柄: GetDpiForWindow 返回 0 → Java catch 同款回退 (scale 1.0)
        let d = DpiHelper::for_window(HWND(std::ptr::null_mut()));
        assert_eq!(d.get_scale_x(), 1.0);
        assert_eq!(d.get_scale_y(), 1.0);
        assert_eq!(d.get_logical_screen_width(), d.get_physical_screen_width());
    }

    // -- 焦点检测 --

    #[test]
    fn focus_process_name_of_current_process() {
        // OpenProcess + QueryFullProcessImageNameW 真实链路: 本测试进程
        let pid = unsafe { GetCurrentProcessId() };
        let name = process_image_name(pid).expect("打开自身进程不应失败");
        assert!(
            name.to_ascii_lowercase().ends_with(".exe"),
            "Windows 进程镜像名应以 .exe 结尾, got {}",
            name
        );
    }

    #[test]
    fn focus_filename_extraction_and_aces_compare() {
        assert_eq!(
            file_name_after_last_backslash(r"C:\Games\War Thunder\aces.exe"),
            "aces.exe"
        );
        assert_eq!(file_name_after_last_backslash("aces.exe"), "aces.exe", "无分隔符 → 整串");
        // Java equalsIgnoreCase 比对语义
        assert!("aces.exe".eq_ignore_ascii_case("ACES.EXE"));
        assert!(!"aces.exe".eq_ignore_ascii_case("aces2.exe"));
    }

    #[test]
    fn focus_smoke_safe_degradation() {
        // 平台合同: 任意前台状态都不得 panic, 恒返回 bool (安全降级)
        let _ = is_war_thunder_focused();
        let det = WindowsFocusDetector;
        let _ = det.is_war_thunder_focused();
    }

    // -- winmm 播放 --

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
