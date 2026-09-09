//! spec 工厂公共脚手架: 键控 spec 构造 (keyed_spec)。
//! W3 组件化后仅剩本骨架 (各旧 `*_overlay_spec` 工厂的 FontSlot 字体热换槽
//! 已随工厂退役; 现存用户 = minihud 编排器 + widgets::page_overlay)。

use crate::platform::host::{OverlaySpec, ReinitFn, RenderFn};

/// OverlaySpec 骨架: 键恒 id==config_key (Java 三个 register 重载均以 configKey
/// 作 LinkedHashMap 键的既有约定), 尺寸/render/reinit 由各工厂特化注入
pub(crate) fn keyed_spec(
    key: &str,
    width: i32,
    height: i32,
    render: RenderFn,
    reinit: Option<ReinitFn>,
) -> OverlaySpec {
    OverlaySpec {
        id: key.to_string(),
        config_key: key.to_string(),
        width,
        height,
        render,
        reinit,
    }
}

/// id 与 config_key 分叉的 spec 构造 — 用户页专用: 实例键 = 页文档 id
/// (同 switch_key 的多页并存双窗), 激活探测仍引用 config_key; 出厂页
/// 恒等 id==config_key 走 keyed_spec (位置存档/断言面零变化)
pub(crate) fn keyed_spec_id(
    id: &str,
    key: &str,
    width: i32,
    height: i32,
    render: RenderFn,
    reinit: Option<ReinitFn>,
) -> OverlaySpec {
    OverlaySpec {
        id: id.to_string(),
        config_key: key.to_string(),
        width,
        height,
        render,
        reinit,
    }
}
