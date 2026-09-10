//! X11 平台实现 (Linux): depth-32 ARGB visual + override-redirect 窗口
//! 设计要点 (与 Windows 版等价的能力):
//! - 透明: depth-32 TrueColor visual + CWColormap|CWBorderPixel (防 BadMatch), 需合成器
//! - 置顶: override-redirect=true (不参与 WM 层叠; WT Linux 跑 X11/XWayland 场景稳定)
//! - 穿透: XShapeCombineRegion(ShapeInput, 空区域); 恢复 = XShapeCombineRegion(None)
//! - present: XPutImage(ZPixmap, depth 32, 预乘 ARGB32 对齐 XRender 语义)
//! 注意: 本文件在非 Windows 平台编译; 开发机为 Windows, 逻辑经评审未运行验证 (见迁移文档遗留项)

use super::{OverlayEvent, WindowConfig};

pub fn create(_cfg: WindowConfig) -> Result<Unsupported, String> {
    Err("X11 实现待接线 x11rb (见 doc/overlay_java_to_rust_migration.md 路线与遗留项)".into())
}

/// 占位 (接线 x11rb 后替换为真实实现)
pub struct Unsupported;
