//! spec 工厂公共脚手架: 键控 spec 构造。
//! W3 组件化后仅剩本骨架 (各旧 `*_overlay_spec` 工厂的 FontSlot 字体热换槽
//! 已随工厂退役; 现存用户 = minihud 编排器 + widgets::page_overlay)。

use crate::platform::host::{OverlaySpec, ReinitFn, RenderFn};

/// OverlaySpec 骨架 (R3: 唯一构造点)。id = 页文档 id (位置存档/激活探测
/// 关联键统一); config_key = 激活开关键 (host 探测经 id 查策略表, 此键
/// 仅供兴趣过滤默认集)
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
