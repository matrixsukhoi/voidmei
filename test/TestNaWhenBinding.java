import prog.config.ConfigLoader;
import prog.config.ConfigProvider;
import ui.model.DefaultFieldManager;
import ui.model.DataField;
import ui.model.FieldDefinition;
import ui.util.VisibilityExpressionEvaluator;
import java.util.List;
import java.util.ArrayList;

public class TestNaWhenBinding {
    public static void main(String[] args) {
        System.out.println("=== 测试 :na-when 绑定流程 ===\n");
        
        // 模拟 ConfigProvider
        ConfigProvider mockConfig = new ConfigProvider() {
            public String getConfig(String key) { return null; }
            public void setConfig(String key, String value) {}
            public boolean isFieldDisabled(String key) { return false; }
        };
        
        // 创建 FieldManager
        DefaultFieldManager fm = new DefaultFieldManager(mockConfig);
        
        // 模拟 initFields(): 添加转半径字段
        String key = "getTurnRadius";
        fm.addField(key, "转半径", "M", key, true, false, "800", null);
        
        System.out.println("1. 字段添加后检查:");
        DataField df1 = fm.getField(key);
        System.out.println("   fm.getField(\"" + key + "\"): " + (df1 != null ? "找到" : "未找到!"));
        
        // 模拟 bindDynamicFields(): 绑定 naWhen
        List<ConfigLoader.GroupConfig> configs = ConfigLoader.loadConfig("ui_layout.cfg");
        ConfigLoader.RowConfig turnRadiusRow = null;
        
        for (ConfigLoader.GroupConfig group : configs) {
            if (group.title != null && group.title.contains("飞行信息")) {
                turnRadiusRow = findRow(group.rows, "getTurnRadius");
                break;
            }
        }
        
        if (turnRadiusRow != null) {
            System.out.println("\n2. 找到配置行:");
            System.out.println("   property: " + turnRadiusRow.property);
            System.out.println("   naWhen: " + turnRadiusRow.naWhen);
            
            // 模拟绑定
            DataField df = fm.getField(turnRadiusRow.property);
            if (df != null) {
                System.out.println("   字段找到，进行绑定...");
                df.naWhenEvaluator = new VisibilityExpressionEvaluator(turnRadiusRow.naWhen, null);
                System.out.println("   naWhenEvaluator 已设置: " + (df.naWhenEvaluator != null));
                
                // 测试求值
                System.out.println("\n3. 测试求值:");
                System.out.println("   value=800: " + df.naWhenEvaluator.evaluate(800.0) + " (应为 false)");
                System.out.println("   value=9999: " + df.naWhenEvaluator.evaluate(9999.0) + " (应为 false)");
                System.out.println("   value=10000: " + df.naWhenEvaluator.evaluate(10000.0) + " (应为 true)");
                System.out.println("   value=50000: " + df.naWhenEvaluator.evaluate(50000.0) + " (应为 true)");
            } else {
                System.out.println("   [错误] 字段未找到!");
            }
        } else {
            System.out.println("[错误] 未找到转半径配置行!");
        }
    }
    
    static ConfigLoader.RowConfig findRow(List<ConfigLoader.RowConfig> rows, String property) {
        if (rows == null) return null;
        for (ConfigLoader.RowConfig row : rows) {
            if (property.equals(row.property)) return row;
            if (row.children != null) {
                ConfigLoader.RowConfig found = findRow(row.children, property);
                if (found != null) return found;
            }
        }
        return null;
    }
}
