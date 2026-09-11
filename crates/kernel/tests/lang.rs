//! i18n 静态表黑盒场景 (kernel::lang)。
//! Lang::init_lang = lang/cur.properties 360 键的静态快照 (table.rs);
//! 本文件钉代表键的真值 + Java 语义怪癖 (update_language 的 dft 永不生效 /
//! 未赋值 Option 字段保持 null)。
#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)

use expect_test::expect;
use kernel::lang::{Lang, Config};

/// 代表键值表: 界面主文案 / HTTP 配置 / 各面板标签抽样 (与 cur.properties 对拍)
#[test]
fn init_lang_代表键值表() {
    let l = Lang::init_lang();
    let actual = [
        l.app_name,          // 应用名 (菜单/托盘)
        l.app_tooltips,      // 托盘提示 (注意源文件值带前导空格)
        l.close,             // 英文段键
        l.m_start,           // 全角空格文案 ("开　始")
        l.http_ip,           // 网络配置键
        l.m_flight_info,     // 面板名 (全角空格缩进)
        l.f_ias,             // FlightInfo 行标签
        l.e_overheat,        // EngineInfo 行标签
        l.g_gear_down,       // GearAndFlaps 行标签
        l.b_cd_min,          // FM 解包调试行
        l.fm_missing_toast,  // FM 缺失 toast (后加键)
        l.noblkx,            // json.rs 守卫用
    ]
    .join("\n");
    expect![[r#"
        VoidMei
        WT8111端口信息分析、显示、记录工具
        Close
        开　始
        127.0.0.1
        　　飞行状态
        表　速
        耐热时
        收起落
        零升阻力系数: %.3f

        没有对应的 FM 数据文件
        可能是新出的飞机, FM 数据尚未更新
        找不到blkx文件
        请使用最新WT拆包aces.vromfs.bin"#]]
    .assert_eq(&actual);
}

/// Config::get_value: 未知键缺省空串; update_language 空值回退 "" —
/// Java 源码把 dft 覆写为 "" 再返回, 传入的默认值实际永不生效 (原行为保真)
#[test]
fn 未知键与默认值永不生效() {
    let cfg = Config::new("./lang/cur.properties"); // 参数仅保留调用点原貌, 不读文件
    assert_eq!(cfg.get_value("nonexistent.key"), "", "未知键 → 空串");
    assert_eq!(cfg.get_value("appName"), "VoidMei", "已知键正常取值");

    // dft 形参永不读取: 传任意值都返回 "" (Java bug 保真)
    assert_eq!(Lang::update_language(&cfg, "nonexistent.key", "fallback"), "");
}

/// Lang::default(): 字段全空 (Java 静态字段隐式 null 的初态) —
/// 消费方只应使用 init_lang() 的产物; c_* 三个键 Java 端从未赋值 → 恒 None
#[test]
fn lang_default_字段全空() {
    let l = Lang::default();
    assert_eq!(l.app_name, "");
    assert_eq!(l.m_start, "");
    assert_eq!(l.l1, "");
    // PORT 注保真: Java initLang 从未赋值这三键 → Option 保持 None
    assert_eq!(l.c_enginedmg, None);
    assert_eq!(l.c_warn1min, None);
    assert_eq!(l.c_eng_bomb, None);
}
