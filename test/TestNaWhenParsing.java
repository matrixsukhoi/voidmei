import prog.config.ConfigLoader;
import prog.config.SExpParser;
import java.util.List;

public class TestNaWhenParsing {
    public static void main(String[] args) {
        System.out.println("=== 测试 :na-when 解析 ===\n");
        
        // 加载配置
        List<ConfigLoader.GroupConfig> configs = ConfigLoader.loadConfig("ui_layout.cfg");
        
        // 查找飞行信息面板
        for (ConfigLoader.GroupConfig group : configs) {
            if (group.title != null && group.title.contains("飞行信息")) {
                System.out.println("找到面板: " + group.title);
                searchForTurnRadius(group.rows, 0);
            }
        }
    }
    
    static String indent(int depth) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < depth; i++) sb.append("  ");
        return sb.toString();
    }
    
    static void searchForTurnRadius(List<ConfigLoader.RowConfig> rows, int depth) {
        if (rows == null) return;
        String ind = indent(depth);
        
        for (ConfigLoader.RowConfig row : rows) {
            if (row.property != null && row.property.contains("TurnRadius")) {
                System.out.println(ind + "找到转半径字段!");
                System.out.println(ind + "  property: " + row.property);
                System.out.println(ind + "  type: " + row.type);
                System.out.println(ind + "  naWhen: " + row.naWhen);
                System.out.println(ind + "  visibleWhen: " + row.visibleWhen);
                
                if (row.naWhen != null) {
                    System.out.println(ind + "  naWhen 表达式已解析!");
                } else {
                    System.out.println(ind + "  [警告] naWhen 为 null!");
                }
            }
            
            // 递归检查子节点
            if (row.children != null && !row.children.isEmpty()) {
                searchForTurnRadius(row.children, depth + 1);
            }
        }
    }
}
