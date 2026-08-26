//! VoiceResourceManager 的 Rust 移植 (src/prog/audio/VoiceResourceManager.java) — 一比一翻译。
//!
//! Manages audio resources and voice packs.
//! Handles loading, caching, and retrieving audio clips.
//!
//! PORT: Java `private static final INSTANCE` 单例 + `getInstance()` → 调用方持有
//! (LIFETIMES §1.1 收敛方案; §2.9 禁再造全局静态), 构造器由 private 改 pub。
//! PORT: 声音播放经 [`SoundPlayer`] trait 注入 (D7 裁决: vm-core 无平台依赖,
//! trait 签名覆盖 Java 用的播放能力; mock 实现做测试, winmm 实现留 P4 平台件)。

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};

use crate::logger;

/// Java: `private static final String VOICE_DIR = "./voice/"`
/// (保留字符串拼接形态: 生成的路径 "./voice/x.wav" 与 Java File.getPath() 一致)
const VOICE_DIR: &str = "./voice/";

// =====================================================================
// 声音播放注入面 (D7 裁决: vm-core 无平台依赖)
// =====================================================================

/// 播放层错误信息面: 对应 Java `catch (Exception e)` 中 `e.getMessage()` 的用途
/// (拼进日志/被吞), 不承载具体异常类型。
pub type SoundError = Box<dyn std::error::Error + Send + Sync>;

/// javax.sound.sampled.Clip 使用面的抽象 (Java Clip 本身即 interface, §1 多实现 → trait dyn)。
///
/// 覆盖全库对 Clip 的消费点 (VoiceResourceManager.applyVolume / VoiceWarning 的
/// start/stop/isRunning/setFramePosition/close)。
///
/// 资源管理说明：
/// AudioInputStream 在 Clip.open() 后可以安全关闭，
/// 因为 Clip 会将音频数据复制到内存中，不再需要流。
/// PORT: 流的开关生命周期整体移入 [`SoundPlayer::open_clip`] 的实现侧
/// (winmm/P4 平台件); 关闭失败时实现应按 Java finally 块语义以 debug 级记录
/// ("VoiceResourceManager", "关闭 AudioInputStream 失败")。
///
/// PORT: RAII 兜底契约 (Java Clip 原生行资源靠 GC finalizer 兜底, Rust 无 GC —
/// LIFETIMES 审查将 Clip 句柄泄漏严重度上调): 实现应在 Drop 中等价调用 close()
/// 释放原生资源; 显式 close() 之后的 Drop 必须幂等 (Java line close 状态机语义)。
///
/// PORT: 本 trait 仅要求 Send 非 Sync 是刻意的最小约束 — 消费端跨线程共享
/// clip 必须经 Mutex 独占访问 (对齐 LIFETIMES §3.2 Java Clip 并发使用面),
/// 不可 RwLock 共享 &clip (play/stop/close 均为独占变异操作)。
pub trait SoundClip: Send {
    /// Java: `clip.start()`
    fn start(&self);
    /// Java: `clip.stop()`
    fn stop(&self);
    /// Java: `clip.isRunning()`
    fn is_running(&self) -> bool;
    /// Java: `clip.setFramePosition(int)`
    fn set_frame_position(&self, frame: i32);
    /// Java: `clip.close()`
    fn close(&self);
    /// Java: `(FloatControl) clip.getControl(FloatControl.Type.MASTER_GAIN)`
    /// 控件的最小/最大增益 `(getMinimum(), getMaximum())`。
    /// 控件不存在/不支持 → None — 对应 Java 抛 IllegalArgumentException 或强转失败,
    /// 被 applyVolume 的空 catch 吞掉 ("Control not supported", §2.7)。
    fn master_gain_range(&self) -> Option<(f32, f32)>;
    /// Java: `gainControl.setValue(val)`
    fn set_master_gain(&self, value: f32);
}

/// javax.sound.sampled.AudioSystem 工厂面的抽象 (D7: 唯一注入点)。
/// vm-core 仅依赖本 trait; 测试用 mock, winmm 实现留 P4 平台件。
pub trait SoundPlayer: Send + Sync {
    /// Java: `AudioSystem.getAudioInputStream(file)` + `getLine(info)` +
    /// `clip.open(audioStream)` 的合成 — 打开音频文件为可播放句柄。
    /// 失败 → Err (对应 Java 该链路上任一异常, 由 loadClip 的 catch 统一处理)。
    fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError>;
}

/// Java: `public class VoiceResourceManager`
///
/// PORT: 跨线程共享 (Java 单例被 Service/EDT 并发调) — 字段均为线程安全类型,
/// struct 自动满足 Send + Sync。
pub struct VoiceResourceManager {
    /// Java: `private static final String VOICE_DIR = "./voice/"`
    /// (实例化以便测试注入; 默认值与常量一致)
    voice_dir: String,
    /// Application 静态字段的消费面 (依赖桩, 非翻译):
    /// Java `public static int voiceVolumn = 100` (Application.java 声明默认值)。
    /// Java 非 volatile 跨线程读写 (配置线程写 / 播放线程读, LIFETIMES §1.2 现存隐患)
    /// → AtomicI32 修正可见性。
    /// PORT: 与 configuration_service::ApplicationState.voice_volumn 是两处消费面,
    /// 统一收口归 vm-app 波次 (§2.9 状态分裂禁令), 落地前由调用方负责同步两者。
    /// 注意对端是裸 pub i32 — 若配置线程写它的同时播放线程读它, 即 Rust 数据竞争
    /// (UB, Java 侧只是非 volatile 可见性隐患); 收口波次必须收敛为单一原子/
    /// ArcSwap 源, 禁止两处各持一份可写副本。
    voice_volumn: AtomicI32,
    /// 声音播放注入 (D7): Java 直接调 javax.sound.sampled.AudioSystem
    player: Box<dyn SoundPlayer>,
}

impl VoiceResourceManager {
    /// Java: `private VoiceResourceManager()` + `public static VoiceResourceManager getInstance()`
    /// PORT: 单例 → 调用方持有; 播放器注入。
    pub fn new(player: Box<dyn SoundPlayer>) -> Self {
        Self::new_with_voice_dir(player, VOICE_DIR.to_string())
    }

    /// 测试注入点 (对齐 FMDataPaths.setDataRoot 先例): 显式指定 voice 根目录。
    /// 其余行为与 [`VoiceResourceManager::new`] 一致 (含构造器建目录语义)。
    pub fn new_with_voice_dir(player: Box<dyn SoundPlayer>, voice_dir: String) -> Self {
        // VOICE_DIR 常量以 '/' 结尾是本类全部路径拼接的前置约定 (Java 隐式依赖);
        // 注入路径缺尾分隔符时补齐 — 对齐 installPack 处 java.io.File(parent, child)
        // 自动补分隔符的行为
        let voice_dir = if voice_dir.ends_with('/') || voice_dir.ends_with('\\') {
            voice_dir
        } else {
            format!("{voice_dir}/")
        };
        let dir = PathBuf::from(&voice_dir);
        if !dir.exists() {
            // PORT: Java mkdirs() 返回值未检查 (失败静默继续)
            let _ = fs::create_dir_all(&dir);
        }
        VoiceResourceManager {
            voice_dir,
            voice_volumn: AtomicI32::new(100), // Application.voiceVolumn 声明默认值
            player,
        }
    }

    /// Application.voiceVolumn 的写入面 (Java 由配置加载线程直写静态字段)
    pub fn set_voice_volumn(&self, v: i32) {
        self.voice_volumn.store(v, Ordering::SeqCst);
    }

    /// Application.voiceVolumn 的读取面
    pub fn voice_volumn(&self) -> i32 {
        self.voice_volumn.load(Ordering::SeqCst)
    }

    /// Lists available voice packs (subdirectories in voice/).
    /// Always includes "default".
    pub fn get_available_packs(&self) -> Vec<String> {
        let mut packs: Vec<String> = Vec::new();
        packs.push("default".to_string());

        let voice_dir = PathBuf::from(&self.voice_dir);
        if voice_dir.exists() && voice_dir.is_dir() {
            // Java: File[] files = voiceDir.listFiles(); if (files != null)
            // listFiles IO 失败返回 null ↔ read_dir 返回 Err, 均跳过循环体
            if let Ok(entries) = fs::read_dir(&voice_dir) {
                // PORT: Java listFiles() 顺序未定义 (NTFS B-tree ≈ 字典序) —
                // read_dir 顺序同样由文件系统决定, 不排序保持同族语义 (§2.5)
                for entry in entries.flatten() {
                    // Java File.isDirectory() 跟随符号链接 (symlink/junction 指向目录
                    // 也计入) → fs::metadata(entry.path()) 跟随语义。
                    // PORT: DirEntry::metadata 是 symlink_metadata 等价 (不遍历链接),
                    // 用它会漏列符号链接语音包, 故显式走 path 级 metadata
                    if fs::metadata(entry.path()).map(|m| m.is_dir()).unwrap_or(false) {
                        packs.push(entry.file_name().to_string_lossy().into_owned());
                    }
                }
            }
        }
        packs
    }

    /// Checks if a resource exists for the given pack and key.
    // PORT: Java String 入参可为 null → Option<&str> (§1 null 映射)
    pub fn has_resource(&self, warning_name: &str, pack_name: Option<&str>) -> bool {
        // Java: packName != null && !packName.isEmpty() && !"default".equals(packName)
        if let Some(p) = pack_name {
            if !p.is_empty() && p != "default" {
                // Java: new File(VOICE_DIR + packName + "/" + warningName + ".wav")
                let file = PathBuf::from(format!("{}{p}/{warning_name}.wav", self.voice_dir));
                if file.exists() {
                    return true;
                }
            }
        }
        // Check default
        PathBuf::from(format!("{}{warning_name}.wav", self.voice_dir)).exists()
    }

    /// Exactly checks if the specfic pack has the resource (without fallback check).
    pub fn has_resource_strict(&self, warning_name: &str, pack_name: Option<&str>) -> bool {
        // Java: "default".equals(packName) — null 安全 (null 不等于 "default")
        if pack_name == Some("default") {
            return PathBuf::from(format!("{}{warning_name}.wav", self.voice_dir)).exists();
        }
        // PORT: Java 字符串拼接 null → 字面 "null" ("./voice/null/x.wav"), 逐字保真
        let p = pack_name.unwrap_or("null");
        PathBuf::from(format!("{}{p}/{warning_name}.wav", self.voice_dir)).exists()
    }

    /// Java: `public void installPack(File zipFile)` — 解压 zip 内全部 .wav 到
    /// voice/&lt;包名&gt;/ (拍平子目录), 失败仅记日志不抛出。
    pub fn install_pack(&self, zip_file: &Path) {
        let mut pack_name = zip_file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(); // Java: getName() 对空路径返回 ""
        // PORT: 路径以 '.'/'..' 结尾时 Java getName() 返回 "."/"..", Rust
        // Path::file_name() 对 CurDir/ParentDir 返回 None → ""; 调用源为
        // JFileChooser 文件选择器 (VoiceGlobalRenderer), 该边界不可达
        // Java: packName.toLowerCase().endsWith(".zip")
        // PORT: Java toLowerCase() 用系统默认 locale, Rust to_lowercase 为 locale
        // 无关 (ROOT 语义; 仅 tr 等 locale 的 'I'→'ı' 映射差异, 不复刻)
        if pack_name.to_lowercase().ends_with(".zip") {
            // Java: packName.substring(0, packName.length() - 4)
            // PORT: 命中处尾 4 字节必为 ASCII ".zip" 变体, 字节切片 == UTF-16 码元切片 (§2.1)
            let new_len = pack_name.len() - 4;
            pack_name.truncate(new_len);
        }

        // Java: new File(VOICE_DIR, packName) — VOICE_DIR 以 '/' 结尾, 等价于直接拼接
        let pack_dir = PathBuf::from(format!("{}{pack_name}", self.voice_dir));
        if !pack_dir.exists() {
            // PORT: Java mkdirs() 返回值未检查 (失败静默继续)
            let _ = fs::create_dir_all(&pack_dir);
        }

        if let Err(e) = self.extract_pack(zip_file, &pack_dir) {
            // Java: Logger.error(..., "Failed to install pack: " + e.getMessage());
            //      e.printStackTrace();
            // PORT: 消息面等价 — Java 两参版手工拼 "msg: {message}" 与
            // error_with_throwable 的 "{msg}: {t}" 模板逐字相同; 堆栈输出收窄:
            // Java printStackTrace 无条件, error_with_throwable 带 DEBUG 级闸门
            // (logger 侧约定, 默认 INFO 级下不打)
            logger::error_with_throwable("VoiceResourceManager", "Failed to install pack", &*e);
        }
    }

    /// installPack 的解压主体 (Java try-with-resources 块, 异常上抛给调用方 catch)。
    /// PORT: Java ZipInputStream 为本地文件头流式读取; zip crate 走 ZipArchive
    /// (中心目录) — 规范 zip 两者条目集与顺序一致, 纯流式 zip (无中心目录) 除外。
    fn extract_pack(&self, zip_file: &Path, pack_dir: &Path) -> Result<(), SoundError> {
        let file = File::open(zip_file)?;
        let mut zip = zip::ZipArchive::new(file)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            // Java: !zipEntry.isDirectory() && zipEntry.getName().toLowerCase().endsWith(".wav")
            // (toLowerCase 的 locale 差异同 install_pack 处 PORT 注)
            if !entry.is_dir() && entry.name().to_lowercase().ends_with(".wav") {
                // Flatten: Ignore parent path in zip, use only filename
                let file_name = file_name_of(entry.name());
                // Java: new File(packDir, fileName)
                let new_file = pack_dir.join(file_name);

                // Java: try (FileOutputStream fos = new FileOutputStream(newFile))
                let mut fos = File::create(&new_file)?;
                // Java: byte[] buffer = new byte[1024]
                let mut buffer = [0u8; 1024];
                loop {
                    // Java: while ((len = zis.read(buffer)) > 0) —
                    // Rust read 的 Ok(0) 即 EOF (对应 Java -1); Java read 返回 0
                    // 仅在 len==0 时出现, 实际不发生, "> 0" 判定语义等价
                    let len = entry.read(&mut buffer)?;
                    if len == 0 {
                        break;
                    }
                    // Java: fos.write(buffer, 0, len)
                    fos.write_all(&buffer[..len])?;
                }
            }
        }
        // Java: zis.closeEntry() — 收尾当前条目; ZipArchive 按需读取, 无对应动作
        Ok(())
    }

    /// Loads a clip for a specific warning from a specific pack.
    /// Fallbacks to default (root voice dir) if not found in pack.
    ///
    /// 资源管理说明：
    /// AudioInputStream 在 Clip.open() 后可以安全关闭，
    /// 因为 Clip 会将音频数据复制到内存中，不再需要流。
    ///
    /// @param warningName The filename base (e.g. "aoaCrit")
    /// @param packName    The voice pack name (e.g. "jarvis")
    /// @return The loaded Clip, or null if failed.
    // PORT: Clip 句柄 → Box<dyn SoundClip>; Java null 返回值 → Option;
    // AudioInputStream 的 finally 关闭移入 SoundPlayer::open_clip 实现侧
    // (见 SoundClip trait 文档, 原注释保留于彼处)。
    pub fn load_clip(
        &self,
        warning_name: &str,
        pack_name: Option<&str>,
    ) -> Option<Box<dyn SoundClip>> {
        // 解析音频文件路径
        let file = match self.resolve_audio_file(warning_name, pack_name) {
            Some(f) => f,
            None => {
                // Java: packName null 拼接为字面 "null"
                logger::error(
                    "VoiceResourceManager",
                    &format!(
                        "Audio file not found: {} (Pack: {})",
                        warning_name,
                        pack_name.unwrap_or("null")
                    ),
                );
                return None;
            }
        };

        match self.player.open_clip(&file) {
            Ok(audio_clip) => {
                self.apply_volume(Some(&*audio_clip));
                Some(audio_clip)
            }
            Err(e) => {
                // Java: Logger.error(..., "Error loading clip: " + file.getPath() + " -> "
                //      + e.getMessage()); e.printStackTrace();
                // PORT: " -> " 分隔符不符合 error_with_throwable 的 "{msg}: {t}" 模板,
                // 为逐字复刻日志行直接拼 Display。
                // PORT: printStackTrace 收窄非逐字复刻 — Java 无条件打 stderr, 此处
                // 套 logger 侧 DEBUG 级闸门 (对齐 error_with_throwable 的既有约定,
                // 默认 INFO 级下不打); 写入用 `let _ = writeln!` 吞错 (eprintln!
                // 在 GUI 子系统/broken pipe 会 panic, Java PrintStream 永不抛, 见
                // logger.rs log() 内 PORT 注)
                logger::error(
                    "VoiceResourceManager",
                    &format!("Error loading clip: {} -> {}", file.display(), e),
                );
                if logger::get_level().value() <= logger::Level::Debug.value() {
                    let _ = writeln!(std::io::stderr().lock(), "{e:?}");
                }
                None
            }
        }
    }

    /// 解析音频文件路径
    /// 1. 优先尝试指定的 Pack 路径
    /// 2. 回退到 default（根目录）
    ///
    /// @param warningName 告警名称
    /// @param packName 语音包名称
    /// @return 文件对象，如果不存在返回 null
    fn resolve_audio_file(&self, warning_name: &str, pack_name: Option<&str>) -> Option<PathBuf> {
        // 1. 尝试 Pack 路径
        if let Some(p) = pack_name {
            if !p.is_empty() && p != "default" {
                let pack_file =
                    PathBuf::from(format!("{}{p}/{warning_name}.wav", self.voice_dir));
                if pack_file.exists() {
                    return Some(pack_file);
                }
            }
        }

        // 2. 回退到 default
        // Java: return defaultFile.exists() ? defaultFile : null;
        let default_file = PathBuf::from(format!("{}{warning_name}.wav", self.voice_dir));
        if default_file.exists() {
            Some(default_file)
        } else {
            None
        }
    }

    /// Applies global volume setting to a clip.
    // PORT: Java applyVolume(Clip) 的 null 入参 → Option<&dyn SoundClip>
    pub fn apply_volume(&self, clip: Option<&dyn SoundClip>) {
        // Java: if (clip == null) return;
        let clip = match clip {
            Some(c) => c,
            None => return,
        };
        // Java: try { FloatControl gainControl = (FloatControl)
        //      clip.getControl(FloatControl.Type.MASTER_GAIN); if (gainControl == null)
        //      return; ... } catch (Exception e) { // Control not supported }
        // — 获取失败面 (抛异常/强转失败/null) 收敛为 master_gain_range() -> None,
        // 空 catch 语义保真 (§2.7)
        let Some((gmin, gmax)) = clip.master_gain_range() else {
            return; // Control not supported
        };

        let rangen = -gmin; // approx 80
        let rangep = gmax; // approx 6
        let volume = self.voice_volumn();

        // Logic copied from VoiceWarning.java
        let val = if volume <= 100 {
            // Logarithmic attenuation
            // Java: gainControl.getMinimum() + (float) Math.log10(
            //      Math.max(1, Application.voiceVolumn)) * rangen / 2.0f
            // — log10 按 f64 计算 (Java 隐式提升), 强转 f32 后整式保持 f32 链
            let v = gmin + f64::log10(volume.max(1) as f64) as f32 * rangen / 2.0f32;
            if v < gmin {
                gmin
            } else {
                v
            }
        } else {
            // Linear amplification
            // Java: (Application.voiceVolumn - 100) * rangep / 100.0f — 全 f32 链
            let v = (volume - 100) as f32 * rangep / 100.0f32;
            if v > gmax {
                gmax
            } else {
                v
            }
        };

        clip.set_master_gain(val);
    }
}

/// Java: `new File(zipEntry.getName()).getName()` — zip 条目名取末段 (拍平用)。
/// PORT: Java 在 Windows 上 '/' 与 '\\' 均为路径分隔符 — 手工取末段对齐该行为
/// ('.'/'..' 段不做组件规范化, 忠于 Java 字符串级 basename; Rust Path::file_name
/// 会吞并 CurDir 段, 语义不同故不用)。'/'/'\\' 均 ASCII, i+1 必为字符边界 (§2.1)。
/// PORT: 尾部 '/' 的条目名 (如 "a/b/") 此处返回 "" 而 Java getName() 返回 "b" —
/// 该形态条目已被上方 isDirectory 过滤, 不可达。
fn file_name_of(path: &str) -> &str {
    match path.rfind(['/', '\\']) {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

// =====================================================================
// Tests — Java 侧无对应单测 (VoiceResourceManager 手动验证), 本组为 B 类
// 行为钉子: 目录扫描/回退解析/zip 拍平安装/音量数学 (mock SoundPlayer)。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::sync::{Arc, Mutex};

    static DIR_N: AtomicUsize = AtomicUsize::new(0);

    /// 每测试独立临时 voice 根目录 (configuration_service 测试先例)
    fn tmp_voice_dir() -> PathBuf {
        let n = DIR_N.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("vm_core_vrm_{}_{n}", std::process::id()))
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
        m.install_pack(&bad); // Java: catch(Exception) 仅记日志

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
}
