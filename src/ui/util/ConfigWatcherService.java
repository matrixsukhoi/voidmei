package ui.util;

import java.io.File;
import javax.swing.Timer;

/**
 * Service to monitor a configuration file for changes.
 * Decouples file system watching from the UI layer.
 */
public class ConfigWatcherService {
    private final String filePath;
    private final Runnable onReload;
    private long lastModTime;
    private boolean ignoreNext = false;
    private Timer timer;

    public ConfigWatcherService(String filePath, Runnable onReload) {
        this.filePath = filePath;
        this.onReload = onReload;
        File file = new File(filePath);
        this.lastModTime = file.exists() ? file.lastModified() : 0;
    }

    /**
     * Starts monitoring at the specified interval.
     */
    public void start(int intervalMs) {
        if (timer != null)
            timer.stop();
        timer = new Timer(intervalMs, e -> check());
        timer.start();
    }

    /**
     * Stops monitoring.
     */
    public void stop() {
        if (timer != null)
            timer.stop();
    }

    /**
     * Signals the service to ignore the very next file modification event.
     * Useful when the application itself writes to the file.
     */
    public void ignoreNext() {
        this.ignoreNext = true;
    }

    private void check() {
        File file = new File(filePath);
        if (!file.exists())
            return;

        long currentModTime = file.lastModified();

        if (ignoreNext) {
            ignoreNext = false;
            lastModTime = currentModTime;
            return;
        }

        if (currentModTime > lastModTime) {
            lastModTime = currentModTime;
            if (onReload != null) {
                onReload.run();
            }
        }
    }
}
