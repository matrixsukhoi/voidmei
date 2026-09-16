//! 组件库缩略图工厂: 注册表条目 + default_props → 离屏静态小样。
//! "试驾场里的展示机" — 实时渲染的组件样张 (一次性会话期缓存, 数据不进小样)。
//! (组件面板简化版未接线 — 暂无调用方留作待接线库, 见 doc/试驾场原生化方案.md)

use crate::render::canvas::PixCanvas;
use crate::render::palette;

use super::env::FactoryCtx;
use super::registry::WidgetMeta;

/// 小样底色 (侧栏 BG_RAISED 同族略提亮, 与行底区隔出样张框)
const THUMB_BG: [u8; 4] = [45, 45, 52, 255];
/// preferred 度量消毒上限 (超大 = 异常度量, 防中心偏移 i32 溢出)
const MAX_MEASURE: i32 = 65535;

/// 注册表条目 → 离屏静态小样: default_props 解析 → 工厂构造 → preferred
/// 度量 → 画布铺底 → 组件原尺寸居中绘制 (v1 不缩放, 超出画布自然裁掉)。
/// preview 静态语义: 构造后不喂数据 (组件构造期 preview 静态值已就位,
/// 与 PageOverlay 的 preview 态一致 — 页面也不推模板不喂帧)。
/// 工厂 Err / props 非法 / 度量异常 (零负/超大) → None (调用方画兜底色块)。
pub fn render_thumbnail(
    meta: &WidgetMeta,
    fctx: &FactoryCtx,
    w: i32,
    h: i32,
) -> Option<PixCanvas> {
    let props = serde_json::from_str(meta.default_props).ok()?;
    let mut comp = (meta.factory)(&props, fctx).ok()?;
    let pref = comp.preferred_size(&fctx.fonts);
    if pref.width <= 0 || pref.height <= 0 || pref.width > MAX_MEASURE || pref.height > MAX_MEASURE
    {
        return None; // 零/负 = 无内容可画, 超大 = 异常度量
    }
    let mut cv = PixCanvas::new(w, h).ok()?;
    cv.fill_rect(0, 0, w, h, THUMB_BG);
    // 原尺寸居中 (组件比画布大 → 偏移为负, 两侧裁掉)
    comp.draw(
        &mut cv,
        (w - pref.width) / 2,
        (h - pref.height) / 2,
        &fctx.fonts,
        palette::aa(),
    );
    Some(cv)
}
