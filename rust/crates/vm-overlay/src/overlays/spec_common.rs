//! spec 工厂公共脚手架 (重构波15 样板收敛): 九份 `*_overlay_spec` 工厂的
//! 字体热换槽 (FontSlot) 与键控 spec 构造 (keyed_spec)。
//! 各工厂只留自己的 state 构建与 render 闭包特有段, 公共骨架收敛于此。

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::platform::host::{OverlaySpec, ReinitFn, RenderFn};
use crate::render::font::LoadedFont;

/// reinit 字体热换失败的统一留痕 — 原五份 (EngineControl/GearFlaps/
/// ControlSurfaces/FMUnpackedData/PowerInfo) 重复文案收敛为一份, 组件标签随调用方
pub(crate) fn log_font_reload_failed(tag: &str, err: &str) {
    vm_core::base::logger::error(tag, &format!("reinit 字体重载失败: {}", err));
}

/// 字体槽: `Rc<RefCell<Rc<LoadedFont>>>` 的 newtype — render 闭包与 reinit 闭包
/// 共享同一槽位, 热换只换槽内 Rc 不换槽外壳 (句柄克隆恒有效)。
/// 构造时钉组件日志标签 (热换失败留痕归属)。
#[derive(Clone)]
pub(crate) struct FontSlot {
    tag: &'static str,
    cell: Rc<RefCell<Rc<LoadedFont>>>,
}

impl FontSlot {
    /// 工厂初装: 即刻装载; 失败 (字体文件缺失/尺寸非法) 向上传播为工厂 Err
    pub(crate) fn new(tag: &'static str, path: &Path, size: i32) -> Result<Self, String> {
        Ok(FontSlot {
            tag,
            cell: Rc::new(RefCell::new(Rc::new(LoadedFont::new(path, size)?))),
        })
    }

    /// reinit 热换单槽: 成功换入新字体返回 true; 失败打统一日志并**保持旧字体**,
    /// 返回 false (调用方据此返回 None — host 不 resize 只清指纹, 语义逐字保留)
    pub(crate) fn reload(&self, path: &Path, size: i32) -> bool {
        match LoadedFont::new(path, size) {
            Ok(f) => {
                *self.cell.borrow_mut() = Rc::new(f);
                true
            }
            Err(e) => {
                log_font_reload_failed(self.tag, &e);
                false
            }
        }
    }

    /// 成组热换 (双/三字体族, gear/axis 先例): 先构造全部, 全部成功才整体落位 —
    /// 任一失败全组保持旧字体。失败仅留痕**首个**错误 (原 tuple-match `(r, _)`
    /// 只报首个 Result 的语义逐字保留; 同文件多档字号时后续错误与首个同源)
    pub(crate) fn reload_group(group: &[(&FontSlot, &Path, i32)]) -> bool {
        let mut built = Vec::with_capacity(group.len());
        for (i, (_, path, size)) in group.iter().enumerate() {
            match LoadedFont::new(path, *size) {
                Ok(f) => built.push(Rc::new(f)),
                Err(e) => {
                    if i == 0 {
                        log_font_reload_failed(group[0].0.tag, &e);
                    }
                    return false;
                }
            }
        }
        for ((slot, _, _), f) in group.iter().zip(built) {
            *slot.cell.borrow_mut() = f;
        }
        true
    }

    /// 槽内字体克隆 (render 闭包/喂数方借用 — 仅 Rc 引用计数, 零堆分配)
    pub(crate) fn get(&self) -> Rc<LoadedFont> {
        Rc::clone(&self.cell.borrow())
    }
}

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
