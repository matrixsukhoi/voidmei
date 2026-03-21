package ui.layout.renderer;

import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.PropertyBinder;
import ui.layout.renderer.RowRenderer.RenderContext;

/**
 * 配置渲染器的统一读写助手
 * 消除 SliderRowRenderer、ComboRowRenderer、SwitchRowRenderer 中的重复配置读写代码
 *
 * 读取优先级：
 * 1. PropertyBinder（GroupConfig 中的字段绑定）
 * 2. ConfigurationService（全局配置服务）
 * 3. 默认值（来自 row.value）
 *
 * 写入逻辑：
 * 1. 尝试 PropertyBinder 写入
 * 2. 同步到 ConfigurationService（用于 overlay 控制键）
 */
public class RendererConfigHelper {

    /**
     * 读取字符串配置值
     *
     * @param ctx        渲染上下文
     * @param groupConfig 组配置
     * @param row        行配置
     * @param defaultVal 默认值
     * @return 配置值
     */
    public static String readString(RenderContext ctx, GroupConfig groupConfig, RowConfig row, String defaultVal) {
        if (row.property != null && PropertyBinder.hasField(groupConfig, row.property)) {
            return PropertyBinder.getString(groupConfig, row.property, defaultVal);
        } else if (row.property != null) {
            return ctx.getStringFromConfigService(row.property, defaultVal);
        }
        return defaultVal;
    }

    /**
     * 读取整数配置值
     *
     * @param ctx        渲染上下文
     * @param groupConfig 组配置
     * @param row        行配置
     * @param defaultVal 默认值
     * @return 配置值
     */
    public static int readInt(RenderContext ctx, GroupConfig groupConfig, RowConfig row, int defaultVal) {
        if (row.property != null && PropertyBinder.hasField(groupConfig, row.property)) {
            return PropertyBinder.getInt(groupConfig, row.property, defaultVal);
        } else if (row.property != null) {
            String val = ctx.getStringFromConfigService(row.property, Integer.toString(defaultVal));
            try {
                return Integer.parseInt(val);
            } catch (Exception e) {
                return defaultVal;
            }
        }
        return defaultVal;
    }

    /**
     * 读取布尔配置值
     *
     * @param ctx        渲染上下文
     * @param groupConfig 组配置
     * @param row        行配置
     * @param defaultVal 默认值
     * @return 配置值
     */
    public static boolean readBool(RenderContext ctx, GroupConfig groupConfig, RowConfig row, boolean defaultVal) {
        if (row.property != null && PropertyBinder.hasField(groupConfig, row.property)) {
            return PropertyBinder.getBool(groupConfig, row.property, defaultVal);
        } else if (row.property != null) {
            return ctx.getFromConfigService(row.property, defaultVal);
        }
        return defaultVal;
    }

    /**
     * 写入字符串配置值
     *
     * @param ctx        渲染上下文
     * @param groupConfig 组配置
     * @param property   属性名
     * @param value      新值
     * @return 是否成功写入 PropertyBinder
     */
    public static boolean writeString(RenderContext ctx, GroupConfig groupConfig, String property, String value) {
        boolean boundSuccess = PropertyBinder.setString(groupConfig, property, value);
        // 总是同步到 ConfigurationService
        if (property != null) {
            ctx.syncStringToConfigService(property, value);
        }
        return boundSuccess;
    }

    /**
     * 写入整数配置值
     *
     * @param ctx        渲染上下文
     * @param groupConfig 组配置
     * @param property   属性名
     * @param value      新值
     * @return 是否成功写入 PropertyBinder
     */
    public static boolean writeInt(RenderContext ctx, GroupConfig groupConfig, String property, int value) {
        boolean boundSuccess = PropertyBinder.setInt(groupConfig, property, value);
        // 总是同步到 ConfigurationService
        if (property != null) {
            ctx.syncStringToConfigService(property, Integer.toString(value));
        }
        return boundSuccess;
    }

    /**
     * 写入布尔配置值
     *
     * @param ctx        渲染上下文
     * @param groupConfig 组配置
     * @param property   属性名
     * @param value      新值
     * @return 是否成功写入 PropertyBinder
     */
    public static boolean writeBool(RenderContext ctx, GroupConfig groupConfig, String property, boolean value) {
        boolean boundSuccess = PropertyBinder.setBool(groupConfig, property, value);
        // 总是同步到 ConfigurationService
        if (property != null) {
            ctx.syncToConfigService(property, value);
        }
        return boundSuccess;
    }
}
