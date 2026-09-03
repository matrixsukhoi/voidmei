//! FlightInfoOverlay 的 host 工厂 — POC window.rs 专径收编进组装面。
//!
//! P6 人工验收缺口: 注册面 6/7 (flightInfoSwitch 走 POC bin 专径无窗口条目,
//! 预览全开也轮不到它)。Java 对位 Controller
//! `registerWithPreview("flightInfoSwitch", FlightInfoOverlay, init(this,S,
//! getOverlaySettings("飞行信息")), ...)`。
//!
//! 渲染栈复用 POC 像素对拍过的 fields/layout/render 三件套 (font::Canvas 直通
//! 域), 经 [`PixCanvas::composite_straight_frame`] 整帧桥入 host 的 PixCanvas
//! 体系 (SrcOver 合成, host 预览灰底保留)。
//!
//! 数据面 (对位 Java FieldOverlay 的字段行):
//! - preview: [`fields::FIELDS`] 静态 [`preview_text`](FieldDef::preview_text)
//!   (POC --preview 同源);
//! - live: ServiceData.flight_values (service_loop deriver.step 整包快照) →
//!   [`build_texts`] (visible-when/na-when 求值, POC 同源),
//!   经 [`FlightInfoState::update`] 喂入 (W2: 数据源 = TelemetrySource)。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::layout::RenderCtx;
use vm_core::base::format;
use vm_core::formula::registry::FormulaView;
use vm_core::ui_support::row_def::RowDef;

use crate::overlays::spec_common::keyed_spec;
use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::render::canvas::PixCanvas;
use crate::render::fields::{render_fields_fixed, FieldText, FontTriple, RenderColors};
use crate::render::font::Canvas;
use crate::render::palette::{aa, colors};

/// numHeight 默认值 (POC main.rs 平移): Java 实测校准 24px BOLD Sarasa = 31,
/// 其余字号 1.25×fontSize 近似 (与实测差 ≤1px, 精确值由对拍脚本 --num-height 注入)
pub fn default_num_height(font_add: i32) -> i32 {
    if font_add == 0 {
        31
    } else {
        ((24 + font_add) as f32 * 1.25).round() as i32
    }
}

/// TelemetrySource → 变量数值 (W2: FlightValues 整包快照消解; W10: 统一
/// 短名制 — 变量名 | 公式名 | "X * N" 乘数, Java getter 名不再进内核取数)
pub fn flight_value(s: &dyn FormulaView, target: &str) -> Option<f64> {
    let (var, mult) = vm_core::formula::resolve_target(target)?;
    vm_core::formula::target_value(&var, mult, s)
}

/// 行定义 → (def 索引, 值文本) 行 (visible-when/na-when 求值)。
/// 波22 热路径: label/unit 是 defs 常量, 行只存索引 — 渲染时借用,
/// 免逐帧 clone (20Hz × ~15 行 × 2 String)
pub fn build_texts(defs: &[RowDef], s: &dyn FormulaView) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, f) in defs.iter().enumerate() {
        // 解析不到按 0 处理 (Java 反射 getter 永不失败, 行只受 visible-when
        // 控制; 曾 None→continue 致 7 行整行消失 — live 显示回归根因之一)
        let raw = flight_value(s, &f.source).unwrap_or(0.0);
        if let Some(cond) = &f.visible_when {
            if !cond.eval(s, raw) {
                continue;
            }
        }
        // wing_sweep 的 ×100 在 source 乘数表达式里 ("wing_sweep * 100")
        let text = match &f.na_when {
            Some(cond) if cond.eval(s, raw) => "-".to_string(),
            _ => format::format(raw, f.precision),
        };
        out.push((i, text));
    }
    out
}

/// FlightInfo 共享句柄 (渲染线程内; live 喂数经 [`FlightInfoState::update`])
pub type FlightInfoHandle = Rc<RefCell<FlightInfoState>>;

/// preview 静态行 (工厂初值与 [`FlightInfoState::reset_preview_rows`] 同源,
/// 免两处漂移): 行定义全量, preview 值原样不经格式化
fn preview_rows(defs: &[RowDef]) -> Vec<(usize, String)> {
    defs.iter()
        .enumerate()
        .map(|(i, f)| (i, f.preview_value.clone()))
        .collect()
}

pub struct FlightInfoState {
    /// 行定义 (cfg 驱动, 随 ReinitParams 更新)
    pub defs: Arc<Vec<RowDef>>,
    /// 行集 (def 索引 + 值文本; preview 静态初值, live 由 update 覆写)
    rows: Vec<(usize, String)>,
    /// POC 渲染栈三件套 (度量 + 字体 + 复用直通画布, 尺寸恒定零重分配)
    ctx: RenderCtx,
    fonts: FontTriple,
    canvas: Canvas,
}

impl FlightInfoState {
    /// live 喂数 (Java FieldOverlay.onFlightData → 字段行更新; host 50ms 渲染
    /// 节拍 + 像素指纹脏检查兜底, 此处纯数据面; W2 起数据源 = TelemetrySource
    /// (ServiceData 散字段, Deriver 整包快照已消解))
    pub fn update(&mut self, s: &dyn FormulaView) {
        self.rows = build_texts(&self.defs, s);
    }

    /// reinitConfig 的资源重建段 (Java FieldOverlay super 段):
    /// 度量/字体/直通画布按新字号/列数重载, rows 保留 (Java 字段行绑定独立于字体)。
    /// 返回新 (w, h) (Java setBounds; 全行高度口径与工厂一致)
    pub fn reinit(
        &mut self,
        fonts_dir: &std::path::Path,
        font_add: i32,
        column: i32,
        defs: Arc<Vec<RowDef>>,
    ) -> Result<(i32, i32), String> {
        let ctx = RenderCtx::new(font_add, column, default_num_height(font_add));
        let fonts = FontTriple::load(fonts_dir, &ctx)?;
        // 行定义随包更新 + rows 回 preview 初值 (live 下一帧覆写; 行开关变更
        // 即时生效)
        self.defs = defs;
        self.rows = preview_rows(&self.defs);
        let (w, h) = (ctx.total_width(), ctx.total_height(self.rows.len() as i32));
        self.canvas = Canvas::new(w, h);
        self.ctx = ctx;
        self.fonts = fonts;
        Ok((w, h))
    }

    /// rows 回 preview 静态初值 (Java closeAll = 实例销毁 + refreshPreview 工厂
    /// 新建实例; D8 单条目跨重建存活的补口 — live 会话残留行在 preview 重开前
    /// 清除, 否则预览窗显示上次 live 数值)。canvas 尺寸同步: live 行经
    /// visible-when 过滤可少于 FIELDS, 回满行高 (reinit 用 rows.len() 度量)。
    pub fn reset_preview_rows(&mut self) {
        self.rows = preview_rows(&self.defs);
        let (w, h) = (
            self.ctx.total_width(),
            self.ctx.total_height(self.rows.len() as i32),
        );
        self.canvas = Canvas::new(w, h);
    }

    /// 行集只读访问 (def 索引 + 值文本; 测试/诊断面)
    pub fn rows(&self) -> &[(usize, String)] {
        &self.rows
    }
}

/// FlightInfo OverlaySpec + live 句柄 (Java Controller 注册键
/// flightInfoSwitch; 字号/列数来自 getOverlaySettings("飞行信息") 组字段)。
/// PORT(WYSIWYG): 字号/列数随 [`ReinitParams`] 仓, reinit 闭包走
/// [`FlightInfoState::reinit`] 重建资源并返回新尺寸 (Java setBounds)
pub fn flight_info_overlay_spec(
    fonts_dir: &std::path::Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(FlightInfoHandle, OverlaySpec), String> {
    let (font_add, column) = {
        let p = params.borrow();
        (p.flight.font_add, p.flight.columns)
    };
    let ctx = RenderCtx::new(font_add, column, default_num_height(font_add));
    let fonts = FontTriple::load(fonts_dir, &ctx)?;
    // preview 初值: cfg 行定义的 preview 值
    let defs = {
        let p = params.borrow();
        Arc::clone(&p.flight.rows)
    };
    let rows = preview_rows(&defs);
    // 窗口尺寸: 全行高度 (POC run_live 同款 — visible-when 变化不重建窗口,
    // 空行区域透明无碍)
    let (w, h) = (ctx.total_width(), ctx.total_height(rows.len() as i32));
    let state = FlightInfoState {
        defs,
        rows,
        canvas: Canvas::new(w, h),
        ctx,
        fonts,
    };
    let handle: FlightInfoHandle = Rc::new(RefCell::new(state));
    let render_handle = Rc::clone(&handle);
    let reinit_handle = Rc::clone(&handle);
    let reinit_params = Rc::clone(params);
    let reinit_fonts = fonts_dir.to_path_buf();
    let reinit: ReinitFn = Box::new(move || {
        let (fa, col, defs) = {
            let p = reinit_params.borrow();
            (
                p.flight.font_add,
                p.flight.columns,
                Arc::clone(&p.flight.rows),
            )
        };
        match reinit_handle
            .borrow_mut()
            .reinit(&reinit_fonts, fa, col, defs)
        {
            Ok(size) => Some(size),
            Err(e) => {
                vm_core::base::logger::error("FlightInfo", &format!("reinit 资源重建失败: {}", e));
                None
            }
        }
    });
    Ok((
        handle,
        keyed_spec(
            "flightInfoSwitch",
            w,
            h,
            Box::new(move |cv: &mut PixCanvas| {
                let mut st = render_handle.borrow_mut();
                // 借用拆分: defs/rows 只读 / canvas 可变 (同结构不相交字段)
                let FlightInfoState {
                    defs,
                    rows,
                    canvas,
                    ctx,
                    fonts,
                } = &mut *st;
                // label/unit 经索引向 defs 借用 (波22: 免逐帧 clone)
                let texts: Vec<FieldText> = rows
                    .iter()
                    .map(|(i, v)| FieldText {
                        label: &defs[*i].label,
                        unit: &defs[*i].unit,
                        value: v,
                    })
                    .collect();
                // 清零重绘到直通 Canvas → 整帧 SrcOver 桥入 PixCanvas
                // aa = 运行时全局仓 (cfg AAEnable 可关, 审查轮 1-A)。色板 = 运行时全局五色
                // (Java FieldOverlay 读 Application.colorNum 族; 对拍工具路径
                // 仍用 render::DEFAULT_COLORS 常量基线, 互不影响)
                let pal = RenderColors {
                    num: colors().num,
                    label: colors().label,
                    unit: colors().unit,
                    shade: colors().shade_shape,
                };
                render_fields_fixed(canvas, &texts, ctx, fonts, &pal, aa());
                if !cv.composite_straight_frame(&canvas.buf) {
                    // 不可达 (spec 尺寸 = Canvas 尺寸 = host 画布尺寸); 防御性留痕
                    vm_core::base::logger::warn("FlightInfo", "整帧桥尺寸不符, 本帧丢弃");
                }
            }),
            Some(reinit),
        ),
    ))
}

// =====================================================================
// Tests
// =====================================================================
/// 名字可达性检查 (测试面): registry 名 ∪ 公式名 — 守卫测试用它钉死
/// overlay 全部消费 target 可达, 防 "名字解析断链 → 面板行消失/恒 0" 的
/// live 显示回归。单名制 (W10): 无别名翻译, 查不到即真断链。
#[cfg(test)]
pub(crate) fn canonical_var_name(name: &str) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    let m = MAP.get_or_init(|| {
        let mut m: HashMap<String, String> = HashMap::new();
        let reg = vm_core::formula::registry::registry();
        for v in &reg.vars {
            m.insert(v.name.to_string(), v.name.to_string());
        }
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../formulas.cfg");
        if let Ok(src) = std::fs::read_to_string(path) {
            for d in vm_core::formula::persistence::parse_formulas(&src) {
                m.insert(d.name.clone(), d.name.clone());
            }
        }
        m
    });
    m.get(name).cloned()
}

/// 测试面: 从仓库 ui_layout.cfg 编译面板行 (W-D 守卫测试的数据源)
#[cfg(test)]
pub(crate) fn cfg_rows(panel: &str) -> Vec<vm_core::ui_support::row_def::RowDef> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../ui_layout.cfg");
    let groups = vm_core::config::config_loader::load_config(path);
    let gc = groups
        .iter()
        .find(|g| g.title == panel)
        .unwrap_or_else(|| panic!("ui_layout.cfg 应含面板 {panel}"));
    vm_core::ui_support::row_def::rows_from_group(gc, &|_| false)
}

#[cfg(test)]
mod tests;
