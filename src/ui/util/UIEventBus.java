package ui.util;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;

/**
 * A simple Event Bus for decoupled UI component communication.
 * Components can publish events and subscribe to events without direct
 * references.
 */
public class UIEventBus {

    private static final UIEventBus INSTANCE = new UIEventBus();

    private final Map<String, List<Consumer<Object>>> subscribers = new HashMap<>();

    private UIEventBus() {
    }

    public static UIEventBus getInstance() {
        return INSTANCE;
    }

    /**
     * Subscribe to an event type.
     * 
     * @param eventType The event type identifier (e.g., "fmPrintSwitchChanged")
     * @param handler   The handler to invoke when the event is published
     */
    public void subscribe(String eventType, Consumer<Object> handler) {
        subscribers.computeIfAbsent(eventType, k -> new ArrayList<>()).add(handler);
    }

    /**
     * Unsubscribe a handler from an event type.
     */
    public void unsubscribe(String eventType, Consumer<Object> handler) {
        List<Consumer<Object>> handlers = subscribers.get(eventType);
        if (handlers != null) {
            handlers.remove(handler);
        }
    }

    /**
     * Publish an event to all subscribers.
     * 
     * @param eventType The event type identifier
     * @param data      The event payload
     */
    public void publish(String eventType, Object data) {
        List<Consumer<Object>> handlers = subscribers.get(eventType);
        if (handlers != null) {
            for (Consumer<Object> handler : handlers) {
                try {
                    handler.accept(data);
                } catch (Exception e) {
                    prog.app.debugPrint("[UIEventBus] Error in handler for " + eventType + ": " + e.getMessage());
                }
            }
        }
    }

    /**
     * Clear all subscribers (useful for cleanup/testing).
     */
    public void clear() {
        subscribers.clear();
    }
}
