//! FlightInfoOverlay 的数据/渲染 state (W3: host 挂载面已迁 widgets::fields_grid
//! 的直通管线, 旧 spec 工厂已退役)。
//!
//! 渲染栈复用 POC 像素对拍过的 fields/layout/render 三件套 (font::Canvas 直通
//! 域), 经 [`PixCanvas::composite_straight_frame`] 整帧桥入 host 的 PixCanvas
//! 体系 (SrcOver 合成, host 预览灰底保留)。
//!
//! 数据面 (对位 Java FieldOverlay 的字段行):
//! - preview: 行定义 previewValue 静态 (构造/reset_preview_rows 落位);
//! - live: FormulaView 快照 → [`build_texts`] (visible-when/na-when 求值),
//!   经 [`FlightInfoState::update`] 喂入 (W2: 数据源 = TelemetrySource)。

use std::sync::Arc;

use crate::layout::RenderCtx;
use vm_core::base::format;
use vm_core::formula::registry::FormulaView;
use vm_core::ui_support::row_def::RowDef;

use crate::render::fields::FontTriple;
use crate::render::font::Canvas;

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

/// preview 静态行 (构造初值与 [`FlightInfoState::reset_preview_rows`] 同源,
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
    pub(crate) rows: Vec<(usize, String)>,
    /// POC 渲染栈三件套 (度量 + 字体 + 复用直通画布, 尺寸恒定零重分配)
    pub(crate) ctx: RenderCtx,
    pub(crate) fonts: FontTriple,
    pub(crate) canvas: Canvas,
}

impl FlightInfoState {
    /// 资源直装构造 (widgets::fields_grid 工厂; rows 置空 —
    /// 调用方紧接 reset_preview_rows 填 preview 行)
    pub fn with_resources(
        defs: Arc<Vec<RowDef>>,
        ctx: RenderCtx,
        fonts: FontTriple,
    ) -> Self {
        let (w, h) = (ctx.total_width(), ctx.total_height(defs.len() as i32));
        FlightInfoState {
            defs,
            rows: Vec::new(),
            canvas: Canvas::new(w, h),
            ctx,
            fonts,
        }
    }

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

/// 测试面: 从出厂默认 JSON 编译面板行 (W-D 守卫测试的数据源)
#[cfg(test)]
pub(crate) fn cfg_rows(panel: &str) -> Vec<vm_core::ui_support::row_def::RowDef> {
    let app = vm_core::config::json_store::factory_default();
    let gc = app
        .panels
        .iter()
        .find(|g| g.title == panel)
        .unwrap_or_else(|| panic!("factory_default.json 应含面板 {panel}"));
    vm_core::ui_support::row_def::rows_from_group(gc, &|_| false)
}

#[cfg(test)]
mod tests;
