//! PowerInfoOverlay (ui/overlay/PowerInfoOverlay.java) — 动力信息 BOS 字段网格。
//! 重构波2 自 overlays_field1.rs 拆出。
//!
//! BOS 字段网格: 常量表快照 (ui_layout.cfg "动力信息" 段) + FieldOverlay.
//! onFlightData 50ms 节流 + 零 GC 更新路径 + BosStyleRenderer 绘制。
//! "数据 struct + 内容绘制 fn" 形态: W3 起 host 挂载面 = widgets::fields_grid
//! 的 BOS 管线 (包本 state), 旧 spec 工厂已退役。

use crate::render::canvas::PixCanvas;
use crate::render::renderers::{BosStyleRenderer, Field, OverlayRenderer, RenderContext};
use crate::ui_model::DataField;
use vm_core::base::format;
use vm_core::formula::registry::FormulaView;

use crate::overlays::gear_flaps::FIELD_OVERLAY_REFRESH_INTERVAL_MS;

// 字段表已 W-D cfg 驱动化 (vm_core::ui_support::row_def::RowDef, 经 ReinitParams 进线程); 本文件只持状态与渲染。

/// 动力信息面板状态 (Java PowerInfoOverlay 的 fieldManager + bindDynamicFields 产物)。
/// 预览 = 构造后不调 update (FieldOverlay.initPreview 不订阅事件, 字段保持 previewValue)。
pub struct PowerInfoState {
    /// 节流基准 (FieldOverlay lastRefreshTime, System.currentTimeMillis 毫秒)
    pub last_refresh_time: i64,
    /// 行定义 (cfg 驱动, 随 ReinitParams 更新)
    pub defs: std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>>,
    /// DataField 承接 (visible/buffer/length/precision/unit 与 BosStyleRenderer 的
    /// Field::Data 通道天然对接)
    fields: Vec<DataField>,
}

impl PowerInfoState {
    /// initFields (FieldOverlay) + DefaultFieldManager.addField:
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

    /// FieldOverlay.onFlightData (FieldOverlay) 的单事件语义:
    /// 50ms 节流闩 → (数据回调内) 零 GC 路径: 取值 →
    /// visible-when → 动态精度 → 动态单位 → 可见时格式化 (na-when → "-",
    /// TIME_MM_SS → formatTime, 其余 format(val, precision))。
    /// System.currentTimeMillis 由调用方注入 now_ms (field2 先例, 便于测试);
    /// 返回值 = 是否执行了更新 (false = 节流跳过, Java 原方法 void, 宿主可据此省重绘)
    pub fn update(&mut self, now_ms: i64, s: &dyn FormulaView) -> bool {
        // 节流防高频事件任务堆积
        if now_ms - self.last_refresh_time < FIELD_OVERLAY_REFRESH_INTERVAL_MS {
            return false; // Skip this update, too soon
        }
        self.last_refresh_time = now_ms;
        for (def, field) in self.defs.iter().zip(self.fields.iter_mut()) {
            // 1. 取值 (visibilitySupplier 求值需要) — 统一解析 (短名 | 公式名 | 乘数)
            let val = vm_core::formula::resolve_target(&def.source)
                .and_then(|(var, mult)| vm_core::formula::target_value(&var, mult, s))
                .unwrap_or(0.0);
            // 2. 可见性: 无 :visible-when 恒可见
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
                // 缓冲内容为 ASCII 数字域, 字符数 = UTF-16 码元数
                field.length = field.buffer.chars().count() as i32;
            }
        }
        true
    }

    /// 首选尺寸 = BosStyleRenderer.calculatePreferredSize (只读 ctx + 可见计数,
    /// 无渲染器状态参与 — BOSStyleRenderer)
    pub fn preferred_size(&self, ctx: &RenderContext) -> (i32, i32) {
        let visible = self.fields.iter().filter(|f| f.visible).count() as i32;
        (ctx.geom.total_width(), ctx.geom.total_height(visible))
    }

    /// 内容绘制 (FieldOverlay.paintComponent → renderer.render; PowerInfo 的
    /// createRenderer = BOSStyleRenderer)
    pub fn draw(&self, cv: &mut PixCanvas, ctx: &RenderContext, renderer: &mut BosStyleRenderer) {
        // Java BosStyleRenderer 直接迭代 fieldManager 列表零分配; Rust render
        // 契约收 `&[Field]` 且 Field 借用 DataField — 缓冲无法与 state 同域复用
        // (state 内自引用 / 渲染闭包内不变性, 均编译期否决), 故每帧 collect 19 项
        // (20Hz 下一笔小分配)。零分配化需 render 契约改迭代器/Rc 化 — 留惯用化 pass
        let fields: Vec<Field> = self.fields.iter().map(Field::Data).collect();
        let mut offset = [0, 0];
        OverlayRenderer::render(renderer, cv, &fields, ctx, &mut offset);
    }
}
