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
    public <T> void register(String configKey,
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
    }

    /**
     * Register an overlay with preview support using config-based activation.
     */
    public <T> void registerWithPreview(String configKey,
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
    }

    /**
     * Register an overlay with a custom activation strategy.
     */
    public <T> void registerWithStrategy(String key,
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
    public void refreshAllPreviews() {
        OverlayContext ctx = OverlayContext.forPreviewMode(tc);
        for (OverlayEntry<?> entry : entries.values()) {
            entry.refreshPreview(ctx);
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
        }

        boolean shouldActivate(OverlayContext ctx) {
            return strategy.shouldActivate(ctx);
        }

        void open(OverlayContext ctx) {
            if (instance != null)
                return;

            instance = factory.get();
            if (gameModeInitializer != null) {
                gameModeInitializer.accept(instance);
            }

            if (needsThread && instance instanceof Runnable) {
                thread = new Thread((Runnable) instance);
                thread.start();
            }
        }

        void refreshPreview(OverlayContext ctx) {
            boolean shouldBeOpen = shouldActivate(ctx);

            if (shouldBeOpen) {
                if (instance == null) {
                    instance = factory.get();
                    if (previewInitializer != null) {
                        previewInitializer.accept(instance);
                    } else if (gameModeInitializer != null) {
                        gameModeInitializer.accept(instance);
                    }
                } else if (reinitializer != null) {
                    reinitializer.accept(instance);
                }
            } else if (instance != null) {
                close();
            }
        }

        void close() {
            if (instance == null)
                return;

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
            }

            instance = null;
            thread = null;
        }
    }
}
