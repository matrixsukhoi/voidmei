use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

// 最小 mock: 键值内存表, 模拟"存储机制被抽象掉"的接口用途。
// set_config 为 &self → 存表包 RefCell (实现侧内部可变性的最小示范)
struct MapConfig {
    values: RefCell<HashMap<String, String>>,
}

impl MapConfig {
    fn new() -> Self {
        MapConfig {
            values: RefCell::new(HashMap::new()),
        }
    }
}

impl ConfigProvider for MapConfig {
    fn get_config(&self, key: &str) -> Option<String> {
        // Java map 无键 → null → None
        self.values.borrow().get(key).cloned()
    }

    fn set_config(&self, key: &str, value: &str) {
        self.values
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
    }

    fn is_field_disabled(&self, key: &str) -> bool {
        self.get_config(key).as_deref() == Some("true")
    }
}

// set 后 get 取回原值 (含 CJK, §2.1 UTF-8 边界)
#[test]
fn test_set_get_roundtrip_cjk() {
    let c = MapConfig::new();
    c.set_config("GlobalNumFont", "微软雅黑");
    c.set_config("GlobalTextFont", "DIN Pro 400");
    assert_eq!(c.get_config("GlobalNumFont"), Some("微软雅黑".to_string()));
    assert_eq!(
        c.get_config("GlobalTextFont"),
        Some("DIN Pro 400".to_string())
    );
}

// 未设置键 → None (契约的 null 形态; 与空串 Some("") 区分)
#[test]
fn test_missing_key_is_none_not_empty() {
    let c = MapConfig::new();
    assert_eq!(c.get_config("nope"), None);
}

// 空串值是合法存储值, 不得与 None 混淆 (契约 "null/empty" 两种形态并存)
#[test]
fn test_empty_string_value_distinct_from_none() {
    let c = MapConfig::new();
    c.set_config("k", "");
    assert_eq!(c.get_config("k"), Some(String::new()));
}

// 覆盖写: 同 key 二次 set 覆盖旧值 (Java Map.put 语义)
#[test]
fn test_set_overwrites() {
    let c = MapConfig::new();
    c.set_config("showSpeedBar", "true");
    c.set_config("showSpeedBar", "false");
    assert_eq!(c.get_config("showSpeedBar"), Some("false".to_string()));
}

// is_field_disabled: "true" → 禁用, 其余 (含 None/空串/任意串) → 不禁用
#[test]
fn test_is_field_disabled() {
    let c = MapConfig::new();
    c.set_config("thrust", "true");
    c.set_config("manifold", "false");
    c.set_config("empty", "");
    assert!(c.is_field_disabled("thrust"));
    assert!(!c.is_field_disabled("manifold"));
    assert!(!c.is_field_disabled("empty"));
    assert!(!c.is_field_disabled("never-set"));
}

// trait 须可作 dyn 对象 (调用方以 Box<dyn ConfigProvider> 解耦, 对应 Java 面向接口编程)
#[test]
fn test_dyn_dispatch() {
    let c: Box<dyn ConfigProvider> = Box::new(MapConfig::new());
    c.set_config("alpha", "1");
    assert_eq!(c.get_config("alpha"), Some("1".to_string()));
    assert_eq!(c.get_config("beta"), None);
}

// 泛型静态分发路径同样满足契约
#[test]
fn test_generic_dispatch() {
    fn read_all(cp: &dyn ConfigProvider, keys: &[&str]) -> Vec<Option<String>> {
        keys.iter().map(|k| cp.get_config(k)).collect()
    }
    let c = MapConfig::new();
    c.set_config("a", "x");
    assert_eq!(read_all(&c, &["a", "b"]), vec![Some("x".to_string()), None]);
}

// : 配置经 Arc<ConfigStore> 共享, 写路径必须经共享引用可用 —
// &mut self 签名在此调用形状下无法编译, 该架构约束由本测试钉死
#[test]
fn test_write_through_shared_reference() {
    fn tweak(cp: &dyn ConfigProvider) {
        cp.set_config("shared", "1");
    }
    let c = MapConfig::new();
    tweak(&c);
    assert_eq!(c.get_config("shared").as_deref(), Some("1"));
}

// Java setConfig 写后同步 publish CONFIG_CHANGED, handler 内联重入 get_config
// (ConfigurationService.java:295/322 + UIStateBus.java:58-70) — 实现须以
// 短作用域内部借用支撑该重入形状, 不 panic 不死锁
#[test]
fn test_set_config_reentrant_read_during_publish() {
    struct ReentrantConfig {
        values: RefCell<HashMap<String, String>>,
        reads_during_publish: RefCell<Vec<Option<String>>>,
    }

    impl ConfigProvider for ReentrantConfig {
        fn get_config(&self, key: &str) -> Option<String> {
            self.values.borrow().get(key).cloned()
        }

        fn set_config(&self, key: &str, value: &str) {
            // 短锁: 写入即释放借用
            self.values
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
            // 模拟同步广播后 handler 的重入读
            self.reads_during_publish
                .borrow_mut()
                .push(self.get_config(key));
        }

        fn is_field_disabled(&self, _key: &str) -> bool {
            false
        }
    }

    let c = ReentrantConfig {
        values: RefCell::new(HashMap::new()),
        reads_during_publish: RefCell::new(Vec::new()),
    };
    c.set_config("k", "v");
    assert_eq!(c.reads_during_publish.borrow().len(), 1);
    assert_eq!(c.reads_during_publish.borrow()[0].as_deref(), Some("v"));
}
