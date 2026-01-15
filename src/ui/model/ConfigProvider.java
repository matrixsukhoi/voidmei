package ui.model;

/**
 * Interface for configuration access.
 * Abstracts away the specific configuration storage mechanism.
 */
public interface ConfigProvider {

    /**
     * Get a configuration value by key.
     * 
     * @param key Configuration key
     * @return Value or null/empty if not set
     */
    String getConfig(String key);

    /**
     * Set a configuration value.
     * 
     * @param key   Configuration key
     * @param value Value to set
     */
    void setConfig(String key, String value);
}
