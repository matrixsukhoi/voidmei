//! PowerInfoOverlay (ui/overlay/PowerInfoOverlay.java) — 动力信息 BOS 字段网格。
//! 重构波2 自 overlays_field1.rs 拆出。
//!
//! BOS 字段网格: 常量表快照 (ui_layout.cfg "动力信息" 段) + FieldOverlay.
//! onFlightData 50ms 节流 + 零 GC 更新路径 + BosStyleRenderer 绘制。
//! "数据 struct + 内容绘制 fn" 形态: 上层把 state 与画布闭包捕获进
//! [`crate::platform::host::OverlaySpec`] 的 render 即挂入 OverlayHost; 文件尾的
//! `*_overlay_spec` 工厂给出 live 喂入形态的现成闭包。

use std::cell::RefCell;
use std::rc::Rc;

use crate::render::canvas::PixCanvas;
use crate::render::renderers::{BosStyleRenderer, Field, OverlayRenderer, RenderContext};
use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::ui_model::DataField;
use vm_core::base::format;
use vm_core::formula::registry::FormulaView;

use crate::overlays::gear_flaps::FIELD_OVERLAY_REFRESH_INTERVAL_MS;

// 字段表已 W-D cfg 驱动化 (vm_core::ui_support::row_def::RowDef, 经 ReinitParams 进线程); 本文件只持状态与渲染。

/// 动力信息面板状态 (Java PowerInfoOverlay 的 fieldManager + bindDynamicFields 产物)。
/// 预览 = 构造后不调 update (FieldOverlay.initPreview 不订阅事件, 字段保持 previewValue)。
pub struct PowerInfoState {
    /// 节流基准 (FieldOverlay.java:39 lastRefreshTime, System.currentTimeMillis 毫秒)
    pub last_refresh_time: i64,
    /// 行定义 (cfg 驱动, 随 ReinitParams 更新)
    pub defs: std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>>,
    /// DataField 承接 (visible/buffer/length/precision/unit 与 BosStyleRenderer 的
    /// Field::Data 通道天然对接)
    fields: Vec<DataField>,
}

impl PowerInfoState {
    /// initFields (FieldOverlay.java:145-155) + DefaultFieldManager.addField:
    /// currentValue = previewValue 原样 (不经 %5s), hideWhenNA=true (EngineInfoConfig
    /// populateFromGroup 固定传 true), hideWhenZero=false (cfg 无 :hide-when-zero)
    pub fn new(defs: std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>>) -> Self {
        let fields = defs
            .iter()
            .map(|def| {
                let mut f = DataField::new(
                    &def.source,
                    &def.label,
                    &def.unit,
                    &def.source, // configKey = :target (write-only, 无人读)
                    true,
                    false,
                );
                f.current_value = def.preview_value.clone();
                f.precision = def.precision as i32;
                f
            })
            .collect();
        PowerInfoState {
            last_refresh_time: 0,
            defs,
            fields,
        }
    }

    pub fn fields(&self) -> &[DataField] {
        &self.fields
    }

    /// 数据面回 previewValue 静态 (Java closeAll = 实例销毁 + refreshPreview
    /// 工厂新建 initPreview 实例的 initFields 段; D8 host 单条目跨重建存活的
    /// 补口 — live 会话残留的 buffer/length 在 preview 重开前清除, 否则预览窗
    /// 显示上次 live 数值而非 previewValue)。reinit 闭包只重建 RenderContext
    /// (字体/列度量), 不动数据面, 故此处显式重置。
    pub fn reset_preview(&mut self) {
        let defs = std::sync::Arc::clone(&self.defs);
        *self = Self::new(defs);
    }

    /// reinit 链: 换行定义并按 preview 值重建 fields (可见态随 update 恢复)
    pub fn rebind_defs(&mut self, defs: std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>>) {
        *self = Self::new(defs);
    }

    /// FieldOverlay.onFlightData (FieldOverlay.java:166-217) 的单事件语义:
    /// 50ms 节流闩 → (invokeLater lambda 内) 零 GC 路径 (:178-217): 取值 →
    /// visible-when → 动态精度 → 动态单位 → 可见时格式化 (na-when → "-",
    /// TIME_MM_SS → formatTime, 其余 format(val, precision))。
    /// PORT: System.currentTimeMillis 由调用方注入 now_ms (field2 先例, 便于测试);
    /// 返回值 = 是否执行了更新 (false = 节流跳过, Java 原方法 void, 宿主可据此省重绘)
    pub fn update(&mut self, now_ms: i64, s: &dyn FormulaView) -> bool {
        // Throttling prevents EDT task accumulation
        if now_ms - self.last_refresh_time < FIELD_OVERLAY_REFRESH_INTERVAL_MS {
            return false; // Skip this update, too soon
        }
        self.last_refresh_time = now_ms;
        for (def, field) in self.defs.iter().zip(self.fields.iter_mut()) {
            // 1. 取值 (visibilitySupplier 求值需要) — 统一解析 (短名 | 公式名 | 乘数)
            let val = vm_core::formula::resolve_target(&def.source)
                .and_then(|(var, mult)| vm_core::formula::target_value(&var, mult, s))
                .unwrap_or(0.0);
            // 2. 可见性: 无 :visible-when 恒可见 (PowerInfoOverlay.java:147)
            field.visible = def.visible_when.as_ref().is_none_or(|e| e.eval(s, val));
            // 3+4. 动态精度/单位 (cfg 全表仅进气压 imperial_display 一条:
            //      英制 "P/x.x''"+1 位 / 公制 "Ata"+2 位; 仅变化时写)
            if def.display == vm_core::ui_support::row_def::DisplayMode::ImperialManifold {
                let imperial = s.var_value("is_imperial").unwrap_or(0.0) > 0.0;
                let new_precision = if imperial { 1 } else { 2 };
                if new_precision != field.precision {
                    field.precision = new_precision;
                }
                let new_unit = if imperial {
                    // Java unitSupplier: String.format("P/%.1f''", manifold*760/25.4)
                    let inhg = s.var_value("manifold_pressure").unwrap_or(0.0) * 760.0 / 25.4;
                    format!("P/{}''", format::format(inhg, 1))
                } else {
                    "Ata".to_string()
                };
                if new_unit != field.unit {
                    field.set_unit(&new_unit);
                }
            }
            // 5. 可见才格式化
            if field.visible {
                if let Some(e) = def.na_when.as_ref() {
                    if e.eval(s, val) {
                        // NA 条件满足, 显示 "-"
                        field.buffer.clear();
                        field.buffer.push('-');
                        field.length = 1;
                        continue;
                    }
                }
                if def.format == vm_core::ui_support::row_def::FormatKind::TimeMmSs {
                    field.buffer = format::format_time(val);
                } else {
                    field.buffer = format::format(val, field.precision as u8);
                }
                // 缓冲内容为 ASCII 数字域, 字符数 = UTF-16 码元数 (§2.1)
                field.length = field.buffer.chars().count() as i32;
            }
        }
        true
    }

    /// 首选尺寸 = BosStyleRenderer.calculatePreferredSize (只读 ctx + 可见计数,
    /// 无渲染器状态参与 — BOSStyleRenderer.java:86-87)
    pub fn preferred_size(&self, ctx: &RenderContext) -> (i32, i32) {
        let visible = self.fields.iter().filter(|f| f.visible).count() as i32;
        (ctx.geom.total_width(), ctx.geom.total_height(visible))
    }

    /// 内容绘制 (FieldOverlay.paintComponent → renderer.render; PowerInfo 的
    /// createRenderer = BOSStyleRenderer)
    pub fn draw(&self, cv: &mut PixCanvas, ctx: &RenderContext, renderer: &mut BosStyleRenderer) {
        // PORT: Java BosStyleRenderer 直接迭代 fieldManager 列表零分配; Rust render
        // 契约收 `&[Field]` 且 Field 借用 DataField — 缓冲无法与 state 同域复用
        // (state 内自引用 / 渲染闭包内不变性, 均编译期否决), 故每帧 collect 19 项
        // (20Hz 下一笔小分配)。零分配化需 render 契约改迭代器/Rc 化 — 留惯用化 pass
        let fields: Vec<Field> = self.fields.iter().map(Field::Data).collect();
        let mut offset = [0, 0];
        OverlayRenderer::render(renderer, cv, &fields, ctx, &mut offset);
    }
}

// ---------------------------------------------------------------------------
// live 喂数形态工厂 (minihud_overlay_spec 先例: render 闭包与喂入方共享句柄)
// ---------------------------------------------------------------------------
// PORT(重构波2): POC 时代的三个 preview_spec 工厂 (state move 进闭包的静态
// 预览专径) 已退役 — 生产预览/live 统一走下方 overlay_spec 工厂 (host 单条目
// 双形态), 测试面经手工 OverlaySpec 顶位。
// Java 各 overlay init(S) 时自订 FlightDataBus (LIFETIMES §2.1), preview 实例
// (initPreview) 不订阅保持 previewValue 静态。Rust host 单条目跨 open/refresh_preview
// 存活 (D8), 两形态共用一份 state — live 喂入由 win32 线程持句柄执行, preview 期
// 喂入门控见 app_shell 的 feed_overlays_live (overlay_ctx_preview 标志)。

/// 动力信息共享句柄 (render 闭包 + 喂入方各持克隆)
pub type PowerInfoHandle = Rc<RefCell<PowerInfoState>>;

/// 动力信息 OverlaySpec + live 句柄 (Java Controller.java:662 注册键 engineInfoSwitch)。
/// 初始态 = previewValue (PowerInfoState::new), 游戏模式由喂入方 update 推进。
/// PORT(WYSIWYG): 字号/列数随 [`ReinitParams`] 仓 — render 闭包经共享 ctx 单元
/// 读取, reinit 闭包重建 RenderContext (Java reinitConfig 的 super 段: 字体 +
/// 列布局重载) 并返回新 preferred_size (setBounds 副作用)
pub fn power_info_overlay_spec(
    fonts_dir: &std::path::Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(PowerInfoHandle, OverlaySpec), String> {
    let (font_add, column_num) = {
        let p = params.borrow();
        (p.font_add_power, p.power_columns)
    };
    let ctx = Rc::new(RefCell::new(RenderContext::load(fonts_dir, font_add, column_num)?));
    let state = PowerInfoState::new({ let p = params.borrow(); std::sync::Arc::clone(&p.power_rows) });
    let (w, h) = state.preferred_size(&ctx.borrow());
    let handle: PowerInfoHandle = Rc::new(RefCell::new(state));
    let render_handle = Rc::clone(&handle);
    let mut renderer = BosStyleRenderer::default();
    // reinit 闭包: 重建 ctx (字体/列度量) → 新 preferred_size (Java setBounds)
    let reinit_handle = Rc::clone(&handle);
    let reinit_ctx = Rc::clone(&ctx);
    let reinit_fonts = fonts_dir.to_path_buf();
    let reinit_params = Rc::clone(params);
    let reinit: ReinitFn = Box::new(move || {
        let (fa, col, defs) = {
            let p = reinit_params.borrow();
            (p.font_add_power, p.power_columns, std::sync::Arc::clone(&p.power_rows))
        };
        // 行定义随包更新 (行开关变更即时生效); preview 值回填, live 下一帧覆写
        reinit_handle.borrow_mut().rebind_defs(defs);
        let new_ctx = match RenderContext::load(&reinit_fonts, fa, col) {
            Ok(c) => c,
            Err(e) => {
                // 字体重载失败: 保持旧 ctx (Java 字体族随包分发, 此路径不可达;
                // 显式留痕不静默)
                vm_core::base::logger::error("PowerInfo", &format!("reinit 字体重载失败: {}", e));
                return None;
            }
        };
        *reinit_ctx.borrow_mut() = new_ctx;
        Some(reinit_handle.borrow().preferred_size(&reinit_ctx.borrow()))
    });
    Ok((
        handle,
        OverlaySpec {
            id: "engineInfoSwitch".to_string(),
            config_key: "engineInfoSwitch".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                render_handle.borrow().draw(cv, &ctx.borrow(), &mut renderer);
            }),
            reinit: Some(reinit),
        },
    ))
}
