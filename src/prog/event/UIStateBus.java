package prog.event;
import prog.Application;

import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.function.Consumer;

/**
 * Event Bus for UI State synchronization.
 * Used for decoupled communication between UI panels (e.g., switch State sync).
 * Thread-safe using ConcurrentHashMap + CopyOnWriteArrayList.
 */
public class UIStateBus {

    private static final UIStateBus INSTANCE = new UIStateBus();

    private final Map<String, List<Consumer<Object>>> subscribers = new ConcurrentHashMap<>();

    private UIStateBus() {
    }

    public static UIStateBus getInstance() {
        return INSTANCE;
    }

    /**
     * Subscribe to an event type.
     * 
     * @param eventType The event type identifier (e.g., "fmPrintSwitchChanged")
     * @param handler   The handler to invoke when the event is published
     */
    public void subscribe(String eventType, Consumer<Object> handler) {
        subscribers.computeIfAbsent(eventType, k -> new CopyOnWriteArrayList<>()).add(handler);
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
                    prog.Application.debugPrint("[UIStateBus] Error in handler for " + eventType + ": " + e.getMessage());
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
