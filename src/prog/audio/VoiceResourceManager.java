package prog.audio;

import java.io.File;
import java.util.ArrayList;
import java.util.List;
import javax.sound.sampled.AudioFormat;
import javax.sound.sampled.AudioInputStream;
import javax.sound.sampled.AudioSystem;
import javax.sound.sampled.Clip;
import javax.sound.sampled.DataLine;
import javax.sound.sampled.FloatControl;

import prog.Application;

/**
 * Manages audio resources and voice packs.
 * Handles loading, caching, and retrieving audio clips.
 */
public class VoiceResourceManager {

    private static final VoiceResourceManager INSTANCE = new VoiceResourceManager();

    private static final String VOICE_DIR = "./voice/";

    private VoiceResourceManager() {
    }

    public static VoiceResourceManager getInstance() {
        return INSTANCE;
    }

    /**
     * Lists available voice packs (subdirectories in voice/).
     * Always includes "default".
     */
    public List<String> getAvailablePacks() {
        List<String> packs = new ArrayList<>();
        packs.add("default");

        File voiceDir = new File(VOICE_DIR);
        if (voiceDir.exists() && voiceDir.isDirectory()) {
            File[] files = voiceDir.listFiles();
            if (files != null) {
                for (File f : files) {
                    if (f.isDirectory()) {
                        packs.add(f.getName());
                    }
                }
            }
        }
        return packs;
    }

    /**
     * Loads a clip for a specific warning from a specific pack.
     * Fallbacks to default (root voice dir) if not found in pack.
     * 
     * @param warningName The filename base (e.g. "aoaCrit")
     * @param packName    The voice pack name (e.g. "jarvis")
     * @return The loaded Clip, or null if failed.
     */
    public Clip loadClip(String warningName, String packName) {
        // 1. Try Pack Path
        File file = null;
        if (packName != null && !packName.isEmpty() && !"default".equals(packName)) {
            file = new File(VOICE_DIR + packName + "/" + warningName + ".wav");
        }

        // 2. Fallback to default path
        if (file == null || !file.exists()) {
            file = new File(VOICE_DIR + warningName + ".wav");
        }

        if (!file.exists()) {
            prog.util.Logger.error("VoiceResourceManager",
                    "Audio file not found: " + warningName + " (Pack: " + packName + ")");
            return null;
        }

        try {
            AudioInputStream audioStream = AudioSystem.getAudioInputStream(file);
            AudioFormat format = audioStream.getFormat();
            DataLine.Info info = new DataLine.Info(Clip.class, format);
            Clip audioClip = (Clip) AudioSystem.getLine(info);
            audioClip.open(audioStream);

            applyVolume(audioClip);

            return audioClip;
        } catch (Exception e) {
            prog.util.Logger.error("VoiceResourceManager",
                    "Error loading clip: " + file.getPath() + " -> " + e.getMessage());
            e.printStackTrace();
            return null;
        }
    }

    /**
     * Applies global volume setting to a clip.
     */
    public void applyVolume(Clip clip) {
        if (clip == null)
            return;
        try {
            FloatControl gainControl = (FloatControl) clip.getControl(FloatControl.Type.MASTER_GAIN);
            if (gainControl == null)
                return;

            float rangen = -gainControl.getMinimum(); // approx 80
            float rangep = gainControl.getMaximum(); // approx 6
            float val = 0.0f;

            // Logic copied from VoiceWarning.java
            if (Application.voiceVolumn <= 100) {
                // Logarithmic attenuation
                val = gainControl.getMinimum()
                        + (float) Math.log10(Math.max(1, Application.voiceVolumn)) * rangen / 2.0f;
                if (val < gainControl.getMinimum())
                    val = gainControl.getMinimum();
            } else {
                // Linear amplification
                val = (Application.voiceVolumn - 100) * rangep / 100.0f;
                if (val > gainControl.getMaximum())
                    val = gainControl.getMaximum();
            }

            gainControl.setValue(val);
        } catch (Exception e) {
            // Control not supported
        }
    }
}
