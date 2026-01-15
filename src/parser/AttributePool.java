package parser;

import java.util.Map;
import java.util.Set;
import java.util.HashSet;
import java.util.concurrent.ConcurrentHashMap;
import java.lang.reflect.Field;

/**
 * A dedicated Global Variable Pool for the application.
 * Supports efficient storage, dot-notation access, and observer pattern for
 * event-driven updates.
 * 
 * Replaces ad-hoc storage in blkx or service adapters.
 */
public class AttributePool {

    // --- Observer Pattern ---

    /**
     * Listener interface for pool updates.
     */
    public interface PoolListener {
        /**
         * Called when subscribed keys have been updated.
         * 
         * @param changedKeys The set of keys that changed since last notification.
         */
        void onPoolUpdated(Set<String> changedKeys);
    }

    // Listeners and their subscribed keys
    private final Map<PoolListener, Set<String>> listeners = new ConcurrentHashMap<>();

    // Keys changed during current batch
    private final Set<String> pendingChanges = ConcurrentHashMap.newKeySet();

    // Batch mode flag
    private volatile boolean batchMode = false;

    /**
     * Subscribe a listener to specific keys.
     * 
     * @param listener The listener to notify.
     * @param keys     The keys to watch. If empty/null, listener receives ALL
     *                 changes.
     */
    public void subscribe(PoolListener listener, String... keys) {
        if (listener == null)
            return;
        Set<String> keySet = new HashSet<>();
        if (keys != null) {
            for (String k : keys) {
                if (k != null)
                    keySet.add(k);
            }
        }
        listeners.put(listener, keySet);
    }

    /**
     * Unsubscribe a listener.
     */
    public void unsubscribe(PoolListener listener) {
        if (listener != null) {
            listeners.remove(listener);
        }
    }

    /**
     * Start a batch update. Changes will be accumulated and listeners notified on
     * commit.
     */
    public void beginBatch() {
        batchMode = true;
        pendingChanges.clear();
    }

    /**
     * Commit a batch update. Notifies all relevant listeners.
     */
    public void commitBatch() {
        batchMode = false;
        if (pendingChanges.isEmpty())
            return;

        // Copy to avoid concurrent modification
        Set<String> changes = new HashSet<>(pendingChanges);
        pendingChanges.clear();

        // Notify listeners
        notifyListeners(changes);
    }

    private void notifyListeners(Set<String> changedKeys) {
        for (Map.Entry<PoolListener, Set<String>> entry : listeners.entrySet()) {
            PoolListener listener = entry.getKey();
            Set<String> subscribedKeys = entry.getValue();

            // If subscribed keys is empty, listener wants ALL changes
            if (subscribedKeys.isEmpty()) {
                listener.onPoolUpdated(changedKeys);
            } else {
                // Check if any subscribed key is in changed keys
                Set<String> relevantChanges = new HashSet<>();
                for (String key : changedKeys) {
                    if (subscribedKeys.contains(key)) {
                        relevantChanges.add(key);
                    }
                }
                if (!relevantChanges.isEmpty()) {
                    listener.onPoolUpdated(relevantChanges);
                }
            }
        }
    }

    // --- Core Storage ---

    private final Map<String, Object> pool = new ConcurrentHashMap<>();

    public AttributePool() {
    }

    /**
     * Put a value into the pool.
     */
    public void put(String key, Object value) {
        if (key == null)
            return;
        if (value == null) {
            pool.remove(key);
        } else {
            pool.put(key, value);
        }

        // Track change
        if (batchMode) {
            pendingChanges.add(key);
        } else {
            // Immediate notification (single key)
            notifyListeners(java.util.Collections.singleton(key));
        }
    }

    /**
     * Put all values from a map. Recommended to use with batch mode.
     */
    public void putAll(Map<String, Object> map) {
        if (map == null)
            return;
        pool.putAll(map);

        if (batchMode) {
            pendingChanges.addAll(map.keySet());
        } else {
            notifyListeners(map.keySet());
        }
    }

    /**
     * Get a value supporting dot-notation path.
     * e.g. "TAS", "state.TAS", "NoFlapsWing.CdMin"
     */
    public Object getValue(String path) {
        if (path == null)
            return null;

        // 1. Exact match (O(1))
        Object val = pool.get(path);
        if (val != null)
            return val;

        // 2. Traversal
        if (path.indexOf('.') != -1) {
            return traverse(path);
        }

        return null;
    }

    private Object traverse(String path) {
        String[] parts = path.split("\\.");
        if (parts.length == 0)
            return null;

        Object current = pool.get(parts[0]);
        if (current == null)
            return null;

        for (int i = 1; i < parts.length; i++) {
            if (current == null)
                return null;

            if (current instanceof Map) {
                current = ((Map<?, ?>) current).get(parts[i]);
            } else {
                try {
                    Field f = current.getClass().getField(parts[i]);
                    current = f.get(current);
                } catch (Exception e) {
                    return null;
                }
            }
        }
        return current;
    }

    public void clear() {
        pool.clear();
    }

    public int size() {
        return pool.size();
    }
}
