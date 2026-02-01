package prog.hotkey;

import com.github.kwhat.jnativehook.GlobalScreen;
import com.github.kwhat.jnativehook.NativeHookException;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;
import com.github.kwhat.jnativehook.keyboard.NativeKeyListener;

import prog.Application;
import prog.event.UIStateBus;
import prog.util.Logger;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Unified global hotkey manager.
 * Centralizes all hotkey listeners to avoid duplicate registrations.
 */
public class HotkeyManager {

    private static HotkeyManager INSTANCE;

    private final Map<Integer, String> keyBindings = new ConcurrentHashMap<>();
    private boolean initialized = false;
    private NativeKeyListener keyListener;

    private HotkeyManager() {
    }

    public static synchronized HotkeyManager getInstance() {
        if (INSTANCE == null) {
            INSTANCE = new HotkeyManager();
        }
        return INSTANCE;
    }

    /**
     * Initialize the native hook and register the key listener.
     * Safe to call multiple times - only initializes once.
     */
    public synchronized void init() {
        if (initialized) {
            Logger.debug("HotkeyManager", "Already initialized, skipping");
            return;
        }

        try {
            Application.silenceNativeHookLogger();
            if (!GlobalScreen.isNativeHookRegistered()) {
                GlobalScreen.registerNativeHook();
                Logger.info("HotkeyManager", "Native hook registered");
            }
        } catch (NativeHookException ex) {
            Logger.error("HotkeyManager", "Failed to register native hook: " + ex.getMessage());
            return;
        }

        keyListener = new NativeKeyListener() {
            @Override
            public void nativeKeyPressed(NativeKeyEvent e) {
                int code = e.getKeyCode();

                // Filter out spurious NumLock events
                if (code == NativeKeyEvent.VC_NUM_LOCK) {
                    return;
                }

                String eventType = keyBindings.get(code);
                if (eventType != null) {
                    Logger.debug("HotkeyManager", "Hotkey pressed: " + code + " -> " + eventType);
                    UIStateBus.getInstance().publish(eventType, HotkeyManager.this, code);
                }
            }
        };

        GlobalScreen.addNativeKeyListener(keyListener);
        initialized = true;
        Logger.info("HotkeyManager", "Initialized with native key listener");
    }

    /**
     * Bind a key code to an event type.
     * When the key is pressed, the event will be published to UIStateBus.
     *
     * @param keyCode   The native key code (from NativeKeyEvent)
     * @param eventType The event type to publish (from UIStateEvents)
     */
    public void bind(int keyCode, String eventType) {
        if (keyCode == 0) {
            Logger.debug("HotkeyManager", "Ignoring bind for keyCode 0");
            return;
        }
        keyBindings.put(keyCode, eventType);
        Logger.info("HotkeyManager", "Bound key " + keyCode + " -> " + eventType);
    }

    /**
     * Unbind a key code.
     *
     * @param keyCode The native key code to unbind
     */
    public void unbind(int keyCode) {
        String removed = keyBindings.remove(keyCode);
        if (removed != null) {
            Logger.info("HotkeyManager", "Unbound key " + keyCode + " (was " + removed + ")");
        }
    }

    /**
     * Rebind a key from old code to new code.
     *
     * @param oldKeyCode The old key code to remove
     * @param newKeyCode The new key code to bind
     * @param eventType  The event type
     */
    public void rebind(int oldKeyCode, int newKeyCode, String eventType) {
        unbind(oldKeyCode);
        bind(newKeyCode, eventType);
        Logger.info("HotkeyManager", "Rebound " + oldKeyCode + " -> " + newKeyCode + " for " + eventType);
    }

    /**
     * Check if a key code is currently bound.
     */
    public boolean isBound(int keyCode) {
        return keyBindings.containsKey(keyCode);
    }

    /**
     * Get the event type bound to a key code.
     */
    public String getBinding(int keyCode) {
        return keyBindings.get(keyCode);
    }

    /**
     * Shutdown the hotkey manager.
     */
    public synchronized void shutdown() {
        if (!initialized) {
            return;
        }

        if (keyListener != null) {
            GlobalScreen.removeNativeKeyListener(keyListener);
            keyListener = null;
        }

        keyBindings.clear();
        initialized = false;
        Logger.info("HotkeyManager", "Shutdown complete");
    }
}
