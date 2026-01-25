package prog;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;
import java.util.function.Supplier;

import ui.base.DraggableOverlay;

/**
 * Manages overlay lifecycle: creation, initialization, and disposal.
 * Uses ActivationStrategy for flexible activation conditions.
 */
public class OverlayManager {

    private final Controller tc;
    private final Map<String, OverlayEntry<?>> entries = new LinkedHashMap<>();

    public OverlayManager(Controller tc) {
        this.tc = tc;
    }

    /**
     * Register an overlay with a config-based activation strategy.
     */
    /**
     * Register an overlay with a config-based activation strategy.
     */
    public <T> OverlayManager register(String configKey,
            Supplier<T> factory,
            Consumer<T> initializer,
            boolean needsThread) {
        entries.put(configKey, new OverlayEntry<>(
                configKey,
                factory,
                initializer,
                null,
                null,
                needsThread,
                ActivationStrategy.config(configKey)));
        return this;
    }

    /**
     * Register an overlay with preview support using config-based activation.
     */
    public <T> OverlayManager registerWithPreview(String configKey,
            Supplier<T> factory,
            Consumer<T> gameModeInitializer,
            Consumer<T> previewInitializer,
            Consumer<T> reinitializer,
            boolean needsThread) {
        entries.put(configKey, new OverlayEntry<>(
                configKey,
                factory,
                gameModeInitializer,
                previewInitializer,
                reinitializer,
                needsThread,
                ActivationStrategy.config(configKey)));
        return this;
    }

    /**
     * Register an overlay with a custom activation strategy.
     */
    public <T> OverlayManager registerWithStrategy(String key,
            Supplier<T> factory,
            Consumer<T> gameModeInitializer,
            Consumer<T> previewInitializer,
            Consumer<T> reinitializer,
            boolean needsThread,
            ActivationStrategy strategy) {
        entries.put(key, new OverlayEntry<>(
                key,
                factory,
                gameModeInitializer,
                previewInitializer,
                reinitializer,
                needsThread,
                strategy));
        return this;
    }

    /**
     * Open all registered overlays for game mode.
     */
    public void openAll() {
        OverlayContext ctx = OverlayContext.forGameMode(tc);
        for (OverlayEntry<?> entry : entries.values()) {
            if (entry.shouldActivate(ctx)) {
                entry.open(ctx);
            }
        }
    }

    /**
     * Close all active overlays.
     */
    public void closeAll() {
        for (OverlayEntry<?> entry : entries.values()) {
            entry.close();
        }
    }

    /**
     * Refresh all overlays for preview mode.
     */
    /**
     * Refresh all overlays for preview mode.
     */
    public void refreshAllPreviews() {
        OverlayContext ctx = OverlayContext.forPreviewMode(tc);
        for (OverlayEntry<?> entry : entries.values()) {
            entry.refreshPreview(ctx);
        }
    }

    /**
     * Re-initialize all active overlays (Game Mode).
     */
    public void reinitActiveOverlays() {
        for (OverlayEntry<?> entry : entries.values()) {
            reinitEntry(entry);
        }
    }

    private <T> void reinitEntry(OverlayEntry<T> entry) {
        if (entry.instance != null && entry.reinitializer != null) {
            // prog.util.Logger.info("OverlayManager", "Re-initializing active overlay: " +
            // entry.key);
            entry.reinitializer.accept(entry.instance);
        }
    }

    /**
     * Get all active overlays.
     */
    public List<Object> getActiveOverlays() {
        List<Object> result = new ArrayList<>();
        for (OverlayEntry<?> entry : entries.values()) {
            if (entry.instance != null) {
                result.add(entry.instance);
            }
        }
        return result;
    }

    /**
     * Get a specific overlay by key.
     */
    @SuppressWarnings("unchecked")
    public <T> T get(String key) {
        OverlayEntry<?> entry = entries.get(key);
        return entry != null ? (T) entry.instance : null;
    }

    /**
     * Check if an overlay is active.
     */
    public boolean isActive(String key) {
        OverlayEntry<?> entry = entries.get(key);
        return entry != null && entry.instance != null;
    }

    /**
     * Refresh overlays that are interested in the changed config key.
     */
    public void refreshPreviews(String changedKey) {
        OverlayContext ctx = OverlayContext.forPreviewMode(tc);
        boolean isGlobal = isGlobalConfig(changedKey);

        for (OverlayEntry<?> entry : entries.values()) {
            if (isGlobal || entry.isInterestedIn(changedKey)) {
                entry.refreshPreview(ctx);
            }
        }
    }

    private boolean isGlobalConfig(String key) {
        if (key == null)
            return true;
        return key.startsWith("Global") || key.equals("AAEnable") || key.equals("simpleFont")
                || key.startsWith("font") || key.equals("Interval") || key.equals("voiceVolume");
    }

    /**
     * Internal class to hold overlay registration info and State.
     */
    private static class OverlayEntry<T> {
        final String key;
        final Supplier<T> factory;
        final Consumer<T> gameModeInitializer;
        final Consumer<T> previewInitializer;
        final Consumer<T> reinitializer;
        final boolean needsThread;
        final ActivationStrategy strategy;
        final List<String> interestedPrefixes = new ArrayList<>();

        T instance;
        Thread thread;

        OverlayEntry(String key, Supplier<T> factory,
                Consumer<T> gameModeInitializer, Consumer<T> previewInitializer,
                Consumer<T> reinitializer, boolean needsThread,
                ActivationStrategy strategy) {
            this.key = key;
            this.factory = factory;
            this.gameModeInitializer = gameModeInitializer;
            this.previewInitializer = previewInitializer;
            this.reinitializer = reinitializer;
            this.needsThread = needsThread;
            this.strategy = strategy;

            // Default interest: own key
            this.interestedPrefixes.add(key);
        }

        void addInterest(String prefix) {
            this.interestedPrefixes.add(prefix);
        }

        boolean isInterestedIn(String changedKey) {
            if (changedKey == null || changedKey.equals(key))
                return true;
            for (String prefix : interestedPrefixes) {
                if (changedKey.startsWith(prefix))
                    return true;
            }
            return false;
        }

        boolean shouldActivate(OverlayContext ctx) {
            return strategy.shouldActivate(ctx);
        }

        void open(OverlayContext ctx) {
            if (instance != null) {
                prog.util.Logger.info("OverlayManager",
                        "Skipping open for " + key + ": already active instance=" + instance);
                return;
            }

            prog.util.Logger.info("OverlayManager", "Opening overlay: " + key);
            instance = factory.get();
            if (gameModeInitializer != null) {
                gameModeInitializer.accept(instance);
            }

            if (needsThread && instance instanceof Runnable) {
                thread = new Thread((Runnable) instance);
                thread.start();
                prog.util.Logger.info("OverlayManager", "Started thread for: " + key);
            }
        }

        void refreshPreview(OverlayContext ctx) {
            boolean shouldBeOpen = shouldActivate(ctx);

            if (shouldBeOpen) {
                if (instance == null) {
                    prog.util.Logger.info("OverlayManager", "Creating preview overlay: " + key);
                    instance = factory.get();
                    if (previewInitializer != null) {
                        previewInitializer.accept(instance);
                    } else if (gameModeInitializer != null) {
                        gameModeInitializer.accept(instance);
                    }

                    if (needsThread && instance instanceof Runnable) {
                        thread = new Thread((Runnable) instance);
                        thread.start();
                        prog.util.Logger.info("OverlayManager", "Started thread for preview: " + key);
                    }
                } else if (reinitializer != null) {
                    // prog.util.Logger.info("OverlayManager", "Re-initializing existing preview: "
                    // + key);
                    reinitializer.accept(instance);
                }
            } else if (instance != null) {
                prog.util.Logger.info("OverlayManager", "Closing preview overlay (inactive strategy): " + key);
                close();
            }
        }

        void close() {
            if (instance == null)
                return;

            prog.util.Logger.info("OverlayManager", "Closing overlay: " + key);

            // Save position if draggable
            if (instance instanceof DraggableOverlay) {
                ((DraggableOverlay) instance).saveCurrentPosition();
            }

            // Set doit = false via reflection
            try {
                java.lang.reflect.Field doitField = instance.getClass().getField("doit");
                doitField.setBoolean(instance, false);
            } catch (Exception e) {
                /* ignore */ }

            // Dispose if Window
            if (instance instanceof java.awt.Window) {
                ((java.awt.Window) instance).dispose();
                prog.util.Logger.info("OverlayManager", "Disposed window: " + key);
            }

            if (thread != null) {
                thread.interrupt();
            }
            instance = null;
            thread = null;
        }
    }

    /**
     * Fluent API to add interest to the last registered entry.
     */
    public OverlayManager withInterest(String... prefixes) {
        // Get the last added entry (insertion order preserved by LinkedHashMap)
        if (!entries.isEmpty()) {
            String lastKey = null;
            for (String k : entries.keySet())
                lastKey = k;

            OverlayEntry<?> entry = entries.get(lastKey);
            if (entry != null) {
                for (String p : prefixes) {
                    entry.addInterest(p);
                }
            }
        }
        return this;
    }
}
