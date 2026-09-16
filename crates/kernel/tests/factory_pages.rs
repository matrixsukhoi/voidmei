//! 出厂页容器化结构黑盒场景: 列表页 (flight-info/power-info) 的容器拓扑。
//! 校验配置可观察面 — 容器根 + 子项顺序 (视觉序) + parent 指向 + 排列 props
//! + 版本号提升 (升级提示链)。布局求解语义归 overlay tests (list_arrange.rs)。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;

use kernel::config::json_model::PageDoc;
use kernel::config::json_store::factory;

fn page(id: &str) -> &'static PageDoc {
    factory()
        .pages
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("出厂页缺失: {id}"))
}

/// 列表页容器拓扑: fields 容器根 (column/gap0) + 全部字段按视觉序挂其下,
/// 无游离根 (链式 parent 全部收编)
#[test]
fn factory_列表页容器拓扑() {
    let show = |id: &str| {
        let p = page(id);
        let lines: Vec<String> = p
            .components
            .iter()
            .map(|c| {
                format!(
                    "{} {} parent={}",
                    c.id,
                    c.r#type,
                    c.parent.as_deref().unwrap_or("-")
                )
            })
            .collect();
        format!(
            "[{}] v{} arrange={} children={}",
            id,
            p.content_version,
            p.components[0].props.get("arrange").and_then(|v| v.as_str()).unwrap_or("?"),
            lines.len() - 1
        )
            + "\n"
            + &lines.join("\n")
    };
    expect![[r#"
        [flight-info-default] v3 arrange=column children=16
        fields core.layout.list parent=-
        ias core.data.field parent=fields
        tas core.data.field parent=fields
        mach core.data.field parent=fields
        compass core.data.field parent=fields
        altitude core.data.field parent=fields
        vario core.data.field parent=fields
        sep core.data.field parent=fields
        acceleration core.data.field parent=fields
        roll_rate core.data.field parent=fields
        ny core.data.field parent=fields
        turn_rate core.data.field parent=fields
        turn_rds core.data.field parent=fields
        aoa core.data.field parent=fields
        aos core.data.field parent=fields
        wing_sweepx100 core.data.field parent=fields
        radio_altitude core.data.field parent=fields
        [power-info-default] v3 arrange=column children=19
        fields core.layout.list parent=-
        horse_power core.data.field parent=fields
        thrust core.data.field parent=fields
        rpm core.data.field parent=fields
        prop_pitch core.data.field parent=fields
        prop_efficiency core.data.field parent=fields
        eff_hp core.data.field parent=fields
        manifold_pressure_display core.data.field parent=fields
        power_percent core.data.field parent=fields
        mass_fuel core.data.field parent=fields
        total_weight core.data.field parent=fields
        fuel_time_milix0_001 core.data.field parent=fields
        wep_kg core.data.field parent=fields
        wep_time core.data.field parent=fields
        booster_fuel_kg core.data.field parent=fields
        booster_fuel_percent core.data.field parent=fields
        water_temp core.data.field parent=fields
        oil_temp core.data.field parent=fields
        heat_tolerance core.data.field parent=fields
        engine_response core.data.field parent=fields"#]]
    .assert_eq(&(show("flight-info-default") + "\n" + &show("power-info-default")));
}

/// 自由拓扑页不受容器化迁移影响 (minihud 手工调参拓扑保持原样)
#[test]
fn factory_自由页保持锚链() {
    let p = page("minihud-default");
    // crosshair 与 row0 链首 aoa 仍是游离根; speed 仍挂 aoa (链未被收编)
    assert!(p.components.iter().any(|c| c.id == "crosshair" && c.parent.is_none()));
    let speed = p.components.iter().find(|c| c.id == "speed").unwrap();
    expect!["speed -> aoa (链保留)"].assert_eq(&format!(
        "speed -> {} (链保留)",
        speed.parent.as_deref().unwrap_or("-")
    ));
    assert!(p.components.iter().all(|c| c.r#type != "core.layout.list"));
}
