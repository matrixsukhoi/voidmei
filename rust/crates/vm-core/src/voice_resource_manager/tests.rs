use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex};

static DIR_N: AtomicUsize = AtomicUsize::new(0);

/// 每测试独立临时 voice 根目录 (configuration_service 测试先例)。
/// PID+计数不保证唯一: Windows PID 复用 + 同计数 n 时 create_dir_all 会命中
/// 历史残留目录 (本套测试不清理临时目录, %TEMP% 已积累大量含 jarvis/hal9000
/// 的残留), 首断言"空目录仅 default"必被污染 — 返回前先清残留 (审查 A-B1)
fn tmp_voice_dir() -> PathBuf {
    let n = DIR_N.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("vm_core_vrm_{}_{n}", std::process::id()));
    let _ = fs::remove_dir_all(&p); // 不存在时忽略
    p
}

fn mkdir(p: &Path) -> PathBuf {
    fs::create_dir_all(p).unwrap();
    p.to_path_buf()
}

fn touch(p: &Path, content: &[u8]) {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, content).unwrap();
}

// ---- mock 实现 (D7: trait 的测试替身) ----

struct MockClip {
    range: Option<(f32, f32)>,
    gains: Arc<Mutex<Vec<f32>>>,
    running: AtomicBool,
}

impl SoundClip for MockClip {
    fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
    }
    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    fn set_frame_position(&self, _frame: i32) {}
    fn close(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
    fn master_gain_range(&self) -> Option<(f32, f32)> {
        self.range
    }
    fn set_master_gain(&self, value: f32) {
        self.gains.lock().unwrap().push(value);
    }
}

/// 播放调用记录: 打开过的路径 + 每次 open 产生 clip 的增益写入日志
struct MockPlayer {
    calls: Mutex<Vec<PathBuf>>,
    gain_logs: Mutex<Vec<Arc<Mutex<Vec<f32>>>>>,
    fail: bool,
    range: Option<(f32, f32)>,
}

impl MockPlayer {
    fn new(range: Option<(f32, f32)>) -> Self {
        MockPlayer {
            calls: Mutex::new(Vec::new()),
            gain_logs: Mutex::new(Vec::new()),
            fail: false,
            range,
        }
    }

    fn failing() -> Self {
        let mut m = Self::new(None);
        m.fail = true;
        m
    }
}

impl SoundPlayer for MockPlayer {
    fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
        self.calls.lock().unwrap().push(path.to_path_buf());
        if self.fail {
            return Err("mock open failure".into());
        }
        let gains = Arc::new(Mutex::new(Vec::new()));
        self.gain_logs.lock().unwrap().push(Arc::clone(&gains));
        Ok(Box::new(MockClip {
            range: self.range,
            gains,
            running: AtomicBool::new(false),
        }))
    }
}

fn mgr(dir: &Path, player: MockPlayer) -> (VoiceResourceManager, Arc<MockPlayer>) {
    let player = Arc::new(player);
    // 单例 → 调用方持有; mock 经 Arc 双持以便断言
    let m = VoiceResourceManager::new_with_voice_dir(
        Box::new(PlayerForward(Arc::clone(&player))),
        dir.to_str().unwrap().to_string(),
    );
    (m, player)
}

/// Box<dyn SoundPlayer> 的转发壳 (让测试保留 mock 句柄)
struct PlayerForward(Arc<MockPlayer>);

impl SoundPlayer for PlayerForward {
    fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
        self.0.open_clip(path)
    }
}

// ---- getAvailablePacks ----

#[test]
fn test_get_available_packs() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    // 构造器已建目录 (Java 构造器 mkdirs 语义): 空目录 → 仅 default
    let (m, _) = mgr(&voice, MockPlayer::new(None));
    assert_eq!(m.get_available_packs(), vec!["default".to_string()]);

    // 子目录计入, 普通文件不计
    mkdir(&voice.join("jarvis"));
    mkdir(&voice.join("hal9000"));
    touch(&voice.join("notadir.txt"), b"x");
    let mut packs = m.get_available_packs();
    assert_eq!(packs.remove(0), "default"); // Always includes "default" 且在首位
    packs.sort(); // read_dir 顺序 FS 相关 (见 PORT 注), 断言前排序
    assert_eq!(packs, vec!["hal9000".to_string(), "jarvis".to_string()]);
}

#[test]
fn test_get_available_packs_dir_uncreatable() {
    // 目录建不出来 (父路径是文件) → exists() false → 仅 default;
    // 同时钉住 Java "mkdirs 失败被忽略" 语义
    let base = tmp_voice_dir();
    touch(&base.join("blocker"), b"x");
    let voice = base.join("blocker").join("voice");
    let (m, _) = mgr(&voice, MockPlayer::new(None));
    assert_eq!(m.get_available_packs(), vec!["default".to_string()]);
}

// ---- hasResource / hasResourceStrict ----

#[test]
fn test_has_resource_fallback() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    touch(&voice.join("aoa.wav"), b"d");
    touch(&voice.join("pack").join("gear.wav"), b"p");

    let (m, _) = mgr(&voice, MockPlayer::new(None));
    // 命中 pack
    assert!(m.has_resource("gear", Some("pack")));
    // pack 未命中 → 回退 default 命中
    assert!(m.has_resource("aoa", Some("pack")));
    // 两级都未命中
    assert!(!m.has_resource("nope", Some("pack")));
    // default pack 直查根目录
    assert!(m.has_resource("aoa", Some("default")));
    assert!(!m.has_resource("gear", Some("default")));
    // null pack 跳过 pack 分支
    assert!(m.has_resource("aoa", None));
    // 空 pack 串同 null 处理
    assert!(m.has_resource("aoa", Some("")));
}

#[test]
fn test_has_resource_strict() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    touch(&voice.join("aoa.wav"), b"d");
    touch(&voice.join("pack").join("gear.wav"), b"p");

    let (m, _) = mgr(&voice, MockPlayer::new(None));
    assert!(m.has_resource_strict("aoa", Some("default")));
    assert!(!m.has_resource_strict("gear", Some("default"))); // 无回退
    assert!(m.has_resource_strict("gear", Some("pack")));
    assert!(!m.has_resource_strict("aoa", Some("pack")));
    // null pack → Java 拼 "null" 字面量, 恒不存在
    assert!(!m.has_resource_strict("aoa", None));
    assert!(!m.has_resource_strict("aoa", Some("null")));
}

// ---- installPack ----

/// 造 zip: (条目名, 内容, 是否目录)
fn write_zip(path: &Path, entries: &[(&str, &[u8], bool)]) {
    let file = File::create(path).unwrap();
    let mut w = zip::ZipWriter::new(file);
    for (name, data, is_dir) in entries {
        let opts = zip::write::SimpleFileOptions::default();
        if *is_dir {
            w.add_directory(name.trim_end_matches('/'), opts).unwrap();
        } else {
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
    }
    w.finish().unwrap();
}

#[test]
fn test_install_pack_flattens() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    let zip_path = base.join("myPack.zip");
    write_zip(
        &zip_path,
        &[
            ("sub/dir/aoa.wav", b"aoa-data", false),
            ("gear.wav", b"gear-data", false),
            ("readme.txt", b"skip-me", false),
            ("emptydir/", b"", true),
        ],
    );

    let (m, _) = mgr(&voice, MockPlayer::new(None));
    m.install_pack(&zip_path);

    let pack = voice.join("myPack");
    // 子目录路径被拍平, 仅取文件名
    assert_eq!(fs::read(pack.join("aoa.wav")).unwrap(), b"aoa-data");
    assert_eq!(fs::read(pack.join("gear.wav")).unwrap(), b"gear-data");
    // 非 .wav 条目与目录条目不落地
    let names: Vec<String> = fs::read_dir(&pack)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 2, "仅两个 wav: {names:?}");
    assert!(!names.iter().any(|n| n.contains("readme") || n.contains("emptydir")));
}

#[test]
fn test_install_pack_name_derivation() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    // Java 场景里 zip 来自磁盘任意位置 (文件选择器), 不会落在 voice/ 内 —
    // 无扩展名的 "Beta" 若与目标包目录同路径会自我覆盖, 分开存放
    let zips = mkdir(&base.join("zips"));
    let (m, _) = mgr(&voice, MockPlayer::new(None));

    // ".zip" 大小写不敏感剥离
    let z1 = zips.join("Alpha.ZIP");
    write_zip(&z1, &[("a.wav", b"a", false)]);
    m.install_pack(&z1);
    assert!(voice.join("Alpha").join("a.wav").exists());

    // 无扩展名原样使用
    let z2 = zips.join("Beta");
    write_zip(&z2, &[("b.wav", b"b", false)]);
    m.install_pack(&z2);
    assert!(voice.join("Beta").join("b.wav").exists());

    // 含 CJK 的包名: 剥 ".zip" 按字节切 (码元等价, §2.1)
    let z3 = zips.join("中文包.zip");
    write_zip(&z3, &[("c.wav", b"c", false)]);
    m.install_pack(&z3);
    assert!(voice.join("中文包").join("c.wav").exists());
}

#[test]
fn test_install_pack_bad_zip_no_panic() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    let bad = base.join("broken.zip");
    fs::write(&bad, b"this is not a zip archive").unwrap();

    let (m, _) = mgr(&voice, MockPlayer::new(None));
    m.install_pack(&bad);

    // 包目录在 try 之前已建 (Java mkdirs 先行), 但无 wav 落地
    let pack = voice.join("broken");
    assert!(pack.is_dir());
    assert!(fs::read_dir(&pack).unwrap().next().is_none());
}

// ---- loadClip ----

#[test]
fn test_load_clip_missing_file() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    let (m, player) = mgr(&voice, MockPlayer::new(None));
    assert!(m.load_clip("nope", Some("pack")).is_none());
    // 解析失败短路: 不触达播放器
    assert!(player.calls.lock().unwrap().is_empty());
}

#[test]
fn test_load_clip_resolves_and_applies_volume() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    touch(&voice.join("pack").join("aoa.wav"), b"wav-bytes");
    // Java 典型 MASTER_GAIN 范围: min≈-80, max≈6 (源码注释)
    let (m, player) = mgr(&voice, MockPlayer::new(Some((-80.0, 6.0))));

    let clip = m.load_clip("aoa", Some("pack")).expect("应加载成功");
    // 路径解析命中 pack (拼接形态与 Java File.getPath() 一致;
    // Path 相等按组件比较, 混合 '/' 与 '\\' 分隔符不影响)
    assert_eq!(player.calls.lock().unwrap()[0], voice.join("pack").join("aoa.wav"));
    // 默认音量 100 → 对数衰减: -80 + log10(100)*80/2 = 0.0
    let logs = player.gain_logs.lock().unwrap();
    assert_eq!(logs.len(), 1);
    let gains = logs[0].lock().unwrap();
    assert_eq!(gains.len(), 1);
    assert!((gains[0] - 0.0f32).abs() < 1e-4, "gain={}", gains[0]);
    // 句柄可用 (trait 面冒烟)
    assert!(!clip.is_running());
}

#[test]
fn test_load_clip_open_failure() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    touch(&voice.join("aoa.wav"), b"wav-bytes");
    let (m, player) = mgr(&voice, MockPlayer::failing());
    assert!(m.load_clip("aoa", None).is_none());
    assert_eq!(player.calls.lock().unwrap().len(), 1); // 尝试过打开
}

// ---- applyVolume 数学 (f32 链逐分支) ----

fn applied_gain(m: &VoiceResourceManager, clip: &MockClip) -> Option<f32> {
    m.apply_volume(Some(clip));
    let gains = clip.gains.lock().unwrap();
    gains.last().copied()
}

#[test]
fn test_apply_volume_attenuation_branch() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    let (m, _) = mgr(&voice, MockPlayer::new(None));

    // v=100 (默认): log10(100)=2 → -80 + 2*80/2 = 0
    let clip = MockClip {
        range: Some((-80.0, 6.0)),
        gains: Arc::new(Mutex::new(Vec::new())),
        running: AtomicBool::new(false),
    };
    assert!((applied_gain(&m, &clip).unwrap() - 0.0f32).abs() < 1e-4);

    // v=0: max(1,0)=1 → log10(1)=0 → 钳在下限 -80
    m.set_voice_volumn(0);
    assert!((applied_gain(&m, &clip).unwrap() - (-80.0f32)).abs() < 1e-4);

    // v=50: log10(50)≈1.69897 → -80 + 1.69897*80/2 ≈ -12.0412
    m.set_voice_volumn(50);
    let g = applied_gain(&m, &clip).unwrap();
    assert!((g - (-12.0412f32)).abs() < 1e-3, "v=50 gain={g}");
}

#[test]
fn test_apply_volume_amplification_branch() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    let (m, _) = mgr(&voice, MockPlayer::new(None));
    let clip = MockClip {
        range: Some((-80.0, 6.0)),
        gains: Arc::new(Mutex::new(Vec::new())),
        running: AtomicBool::new(false),
    };

    // v=101: (1)*6/100 = 0.06
    m.set_voice_volumn(101);
    assert!((applied_gain(&m, &clip).unwrap() - 0.06f32).abs() < 1e-6);

    // v=150: 50*6/100 = 3.0
    m.set_voice_volumn(150);
    assert!((applied_gain(&m, &clip).unwrap() - 3.0f32).abs() < 1e-6);

    // v=10000: 9900*6/100 = 594 → 钳在上限 6
    m.set_voice_volumn(10000);
    assert!((applied_gain(&m, &clip).unwrap() - 6.0f32).abs() < 1e-6);
}

#[test]
fn test_apply_volume_control_not_supported_and_null_clip() {
    let base = tmp_voice_dir();
    let voice = mkdir(&base);
    let (m, _) = mgr(&voice, MockPlayer::new(None));

    // 控件不支持 → 空 catch: 不设置、不 panic
    let clip = MockClip {
        range: None,
        gains: Arc::new(Mutex::new(Vec::new())),
        running: AtomicBool::new(false),
    };
    m.apply_volume(Some(&clip));
    assert!(clip.gains.lock().unwrap().is_empty());

    // null clip 直接返回
    m.apply_volume(None);
}

// ---- 辅助函数 file_name_of ----

#[test]
fn test_file_name_of() {
    assert_eq!(file_name_of("sub/dir/x.wav"), "x.wav");
    assert_eq!(file_name_of("x.wav"), "x.wav");
    // Windows 分隔符 '\' 同样切分 (Java File 语义)
    assert_eq!(file_name_of("sub\\x.wav"), "x.wav");
    // '.'/'..' 段不做规范化 (忠于 Java 字符串级 basename)
    assert_eq!(file_name_of("dir/."), ".");
    assert_eq!(file_name_of(""), "");
}
