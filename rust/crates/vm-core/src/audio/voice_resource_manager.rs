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

use crate::base::logger;

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
            // listFiles IO 失败返回 null ↔ read_dir 返回 Err, 均跳过循环体
            if let Ok(entries) = fs::read_dir(&voice_dir) {
                // PORT: Java listFiles() 顺序未定义 (NTFS B-tree ≈ 字典序) —
                // read_dir 顺序同样由文件系统决定, 不排序保持同族语义 (§2.5)
                for entry in entries.flatten() {
                    // Java File.isDirectory() 跟随符号链接 (symlink/junction 指向目录
                    // 也计入) → fs::metadata(entry.path()) 跟随语义。
                    // PORT: DirEntry::metadata 是 symlink_metadata 等价 (不遍历链接),
                    // 用它会漏列符号链接语音包, 故显式走 path 级 metadata
                    if fs::metadata(entry.path())
                        .map(|m| m.is_dir())
                        .unwrap_or(false)
                    {
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
        if let Some(p) = pack_name {
            if !p.is_empty() && p != "default" {
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
            .unwrap_or_default();
        // PORT: 路径以 '.'/'..' 结尾时 Java getName() 返回 "."/"..", Rust
        // Path::file_name() 对 CurDir/ParentDir 返回 None → ""; 调用源为
        // JFileChooser 文件选择器 (VoiceGlobalRenderer), 该边界不可达
        // PORT: Java toLowerCase() 用系统默认 locale, Rust to_lowercase 为 locale
        // 无关 (ROOT 语义; 仅 tr 等 locale 的 'I'→'ı' 映射差异, 不复刻)
        if pack_name.to_lowercase().ends_with(".zip") {
            // PORT: 命中处尾 4 字节必为 ASCII ".zip" 变体, 字节切片 == UTF-16 码元切片 (§2.1)
            let new_len = pack_name.len() - 4;
            pack_name.truncate(new_len);
        }

        let pack_dir = PathBuf::from(format!("{}{pack_name}", self.voice_dir));
        if !pack_dir.exists() {
            // PORT: Java mkdirs() 返回值未检查 (失败静默继续)
            let _ = fs::create_dir_all(&pack_dir);
        }

        if let Err(e) = self.extract_pack(zip_file, &pack_dir) {
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
            // (toLowerCase 的 locale 差异同 install_pack 处 PORT 注)
            if !entry.is_dir() && entry.name().to_lowercase().ends_with(".wav") {
                // Flatten: Ignore parent path in zip, use only filename
                let file_name = file_name_of(entry.name());
                let new_file = pack_dir.join(file_name);

                let mut fos = File::create(&new_file)?;
                let mut buffer = [0u8; 1024];
                loop {
                    // Rust read 的 Ok(0) 即 EOF (对应 Java -1); Java read 返回 0
                    // 仅在 len==0 时出现, 实际不发生, "> 0" 判定语义等价
                    let len = entry.read(&mut buffer)?;
                    if len == 0 {
                        break;
                    }
                    fos.write_all(&buffer[..len])?;
                }
            }
        }
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
                let pack_file = PathBuf::from(format!("{}{p}/{warning_name}.wav", self.voice_dir));
                if pack_file.exists() {
                    return Some(pack_file);
                }
            }
        }

        // 2. 回退到 default
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
        let clip = match clip {
            Some(c) => c,
            None => return,
        };
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
mod tests;
