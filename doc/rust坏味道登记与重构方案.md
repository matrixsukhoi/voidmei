# Rust 坏味道登记与重构方案

> 2026-09-04 起，对已完成迁移的 Rust workspace 做一轮《重构》（Martin Fowler）标准的
> 全面坏味道清扫。本文件是登记表（发现 → 裁决 → 处置波次）与分波方案的单一真相。
> 完成一项就把该项标记 ✅ 并注明波次；裁决"保留"的注明理由。

## 总原则

1. **Java 参照系退役**：CLAUDE.md 裁决迁移已完成、不再对齐 Java。注释不再以 Java
   源码/行号为解释主轴，改写为 Rust 自解释；`TODO(port)` 逐条裁决清零。
2. **单一真相路径**：延续波9 为 vm-core 确立的原则——vm-overlay 根 re-export 壳退役，
   全库统一 `vm_overlay::<域>::<模块>`。
3. **收敛点收割**：`base::format` / `render::primitives` / `power_curve` 等收敛点早已
   建立，但各处本地副本未回头清除——逐族收割，并加守卫测试防回归。
4. 每波结束：`cargo test --workspace` 全绿 + clippy 无新增。

## 分波方案

| 波 | 主题 | 主要内容 | 状态 |
|---|---|---|---|
| 12 | 保真残留退役 | 死代码清扫（整文件/整模块/死函数/死字段）、AA 开关真缺陷修复、端口溢出策略收敛、孤儿注释清理 | ✅ 1135082 (净 -2548 行, 1256 绿, clippy 0) |
| 13 | 重复收敛 | java 兼容助手族全库唯一化（base::java_compat）、printf 族归 base::format、像素基元归 primitives、power_curve 切换、WEP 四胞胎、跨 crate 常量/扫描/fm1 规则收敛 | ✅ 7889747 (~1600 行重复出库, 1256 绿, clippy 0) |
| 14 | 长函数拆解·计算层 | getload_from(775)/variabler(376)/calculate(277)/parse_obj(254)/build_registry(233)/extract_stages(184)/process_polling_cycle(176)/identify_inflection(172) | ✅ 45d1827 (编排层化+锁样板 46 处, 1256 绿, clippy 0) |
| 15 | 长函数拆解·装配层 | win32_thread_main(454)/desktop_main(260)/TrayIcon::new(160)/minihud update_components(155)+apply_style(124)/9 段注册样板/9 份 spec 工厂脚手架 | ✅ 0ba5f03 (FontSlot/spec_common/map_inner, 1256 绿, clippy 0) |
| 16 | 结构归位 | vm-overlay 根壳退役、minihud.rs 四拆、commands_windows 三域拆分、config_manager 拆 ui_state_storage/key_text、extras.rs 三拆、win32.rs 更名、AppShell 收口 | ✅ 1a3a9d8+3e750cd (阶段1 四路+阶段2 根壳退役/更名, 1256 绿, clippy 0) |
| 17 | 数据形态升级 | EngineType 枚举化、is_imperial()、F_INVALID 哨兵统一、WarningSlot、CompressorData、GroupConfig 表驱动、DTO 枚举化、镜像字段删除 | ✅ e1f56fa (六域并行; A6 含边角语义变化已备案, 1256 绿, clippy 0) |
| 18 | 文档焕新 | Java 引用注释清扫（3534 处分批）、rust/README 新人导览重写、crate 级架构地图 | ✅ ae8dd3c (行号锚 493→0, README 重写, 1256 绿, clippy 0) |
| 19 | 复核收尾 | fresh-eyes 两路复核揪出的收敛漏网收割 + iced 残留/mirror 行级 zip/命名纠正 + 全库 fmt | ✅ c46d9fe (1256 绿, clippy 0, fmt 干净) |

## 登记表

### A. 真实行为缺陷

| # | 位置 | 问题 | 裁决 | 波 |
|---|---|---|---|---|
| A1 | overlays/gear_flaps.rs:284 | render 闭包 AA 钉死 `true`，`AAEnable` 配置对该窗失效 | 改读 `palette::aa()` | 12 |
| A2 | render/renderers.rs:102 | `RenderContext::new` 钉死 `graph_aa:true,text_aa:true`，power_info 恒 AA | AA 上下文化 | 12 |
| A3 | vm-app/vm-data/vm-core 三处 | 备份端口 `+1111` 溢出策略不一致（panic/wrapping/saturating） | `bkp_port()` 单一函数（saturating） | 12 |
| A4 | vm-ui/main_form.rs:496 | mirror 按位 zip 依赖两树同构，整树替换后静默错位 | 按 title 配对 | 17 |
| A5 | vm-ui/renderer_config_helper.rs:296 | cfg 用户输入越型绑定即 panic 主线程 | 改 warn+忽略 | 12 |
| A6 | vm-core/audio/voice_warning.rs:1103 | `fuel_p_check` 一个计数器被两个不相关告警共用 | 拆独立计数器 | 17 |
| A7 | 4 处"AA 恒开"过时注释 | 审查轮修复后注释未同步 | 清理 | 12 |

### B. 死代码（迁移保真残留退役）

| # | 位置 | 内容 | 波 |
|---|---|---|---|
| B1 | vm-core/fm/power_curve.rs | 整模块零调用（piston_model 8 个内联副本未切换）→ 波13 切换后复活为真相 | 13 |
| B2 | vm-core/audio/voice_warning.rs | `play_wav`/`get_clip` 全库无调用方 | 12 |
| B3 | vm-core/derived/flight_log.rs:856 | `run()` "Java 中从未启动"保真保留 | 12 |
| B4 | vm-core/config/config_manager.rs:567 | `show_parse_error_dialog` Java 侧即不可达 | 12 |
| B5 | vm-core/config/config_loader.rs:715 | 空 legacy INI 判定块 | 12 |
| B6 | vm-overlay/platform/position.rs | 整文件死（host 走 PositionStore trait） | 12 |
| B7 | vm-overlay/ui_model/gauge_field.rs | GaugeField/LinearGaugePlaceholder/MarkedGaugePlaceholder 零构造 | 12 |
| B8 | vm-overlay/overlays/rows.rs:437 | `HUDFlapsRow` 零实例化 | 12 |
| B9 | vm-overlay/render/canvas.rs:120 | 自由函数 `to_premul_bgra` | 12 |
| B10 | vm-overlay/render/fields.rs:50,131 | `render_fields`/`save_png` 双份 | 12 |
| B11 | vm-overlay/render/renderers.rs:46,54 | `APPLICATION_COLORS`/`WHITE` | 12 |
| B12 | vm-overlay/layout/minihud_layout.rs:491 | `MINIHUD_PANEL_ITEMS` 等仅测试消费（cfg 第二份手工快照） | 12 |
| B13 | vm-ui/renderers/*.rs | 读链函数群（read_display/read_current/is_hex_format/to_hex_string/rgb_to_hsb/hsb_to_rgb）生产零消费 | 12 |
| B14 | vm-ui/row_renderer_registry.rs | RowRenderer trait/BUILTIN_ROW_TYPES/RowRendererRegistry ~100 行死结构（D9 已裁决渲染归 web） | 12 |
| B15 | vm-ui/main_form.rs:79 | `Message::Ignore` 不可达变体 | 12 |
| B16 | vm-app/controller_state.rs:42 | `get/from_legacy_value` 仅测试调用 | 12 |
| B17 | vm-app/env.rs:24 | `Env.app_port_bkp` 零消费（随 A3 收敛） | 12 |
| B18 | vm-data service_fields/frame | `s_loc` 字段恒 None 纯搬运 | 12 |
| B19 | vm-webui/lib.rs:117 | `let _ = &mut app;` 残留 | 12 |
| B20 | vm-core #[allow(dead_code)] ×14 | 逐条裁决：真死删、有意保真标注 `DEAD(kept)` | 12 |
| B21 | vm-overlay/layout/hud_layout_node.rs:233 | `set_ignore_bounds` 零调用 | 12 |
| B22 | vm-core/fm/handle.rs 等 TODO(port) ~20 处 | 逐条裁决清零 | 12/18 |

### C. 重复代码（收敛族）

| # | 族 | 副本数 | 收敛点 | 波 |
|---|---|---|---|---|
| C1 | java_double_to_string / java_float_to_string | 4+1 | base::format | 13 |
| C2 | java_trim | 6 | base::java_compat | 13 |
| C3 | java_parse_boolean | 6 | base::java_compat | 13 |
| C4 | java_parse_int（含 or 变体） | 4 | base::java_compat | 13 |
| C5 | java_round | 3 | base::format（已有） | 13 |
| C6 | current_time_millis | 5 | base | 13 |
| C7 | panic_message | 2 | base::exception_helper | 13 |
| C8 | sleep_while_run | 2 | base::exception_helper（已有） | 13 |
| C9 | config_value_to_string ≈ to_java_string | 2 | config 域内合一 | 13 |
| C10 | printf 族 java_f/pad_width/fmt_d/java_format_f/fmt_pct3/fmt_heading3/FmtArg/java_string_format | 8 | base::format | 13 |
| C11 | power_curve 8 内联副本 vs 正模块 | 9 | fm::power_curve | 13 |
| C12 | WEP 增压器强度四胞胎 + wep_critical_altitude | 5 | fm 内 helper | 13 |
| C13 | 引擎计数循环两份 | 2 | reader::count_engines | 13 |
| C14 | FlightLog save 三胞胎 + write_label 31 平铺 | 3+1 | derived 内 helper | 14 |
| C15 | draw_h_rect | 2 | render::primitives | 13 |
| C16 | text_shade ≡ text_shaded_auto | 1+1 | 删副本 | 13 |
| C17 | draw_rect_perimeter ≈ ring1px / butt_line / hline_butt2 / vline_square2 / stroke_outline ×2 | 6 | render::primitives | 13 |
| C18 | GLOBAL_KEYS/GLOBAL_PREFIXES 跨 crate | 2 | host pub 化 | 13 |
| C19 | FM 目录扫描（GetFmList/load_planes） | 2 | vm-core fm::list_fm_names | 13 |
| C20 | fm1 归一化/标题规则 | 4 | normalize_secondary + title 单源 | 13 |
| C21 | overlay 键集（LIVE_OVERLAYS/OVERLAY_SECTIONS/备案注释） | 3 | keys.rs 单源 | 13 |
| C22 | 9 段 register_live_overlays 样板 | 9 | register_one 泛型 | 15 |
| C23 | 9 份 *_overlay_spec 工厂脚手架 | 9 | spec helper + FontSlot | 15 |
| C24 | minihud RefCell 借用样板 | 29 | with_row/map_inner | 15 |
| C25 | vm-data 锁三段式样板 | 10+ | with_snapshot/apply | 14 |
| C26 | find_group 族四种写法 | 4 | group_by_title | 13 |
| C27 | minihud 占位/正式构造双份 + throttle 更新双份 | 2+2 | 局部 helper | 15 |

### D. 过长函数拆解

| # | 函数 | 行数 | 波 |
|---|---|---|---|
| D1 | fm/data/reader.rs getload_from | 775 | 14 |
| D2 | vm-app/win32.rs win32_thread_main | 454 | 15 |
| D3 | fm/piston_model.rs variabler | 376 | 14 |
| D4 | derived/hud_calculator.rs calculate | 277 | 14 |
| D5 | vm-app/main.rs desktop_main | 260 | 15 |
| D6 | telemetry/parser/map_obj.rs parse_obj | 254 | 14 |
| D7 | formula/registry.rs build_registry | 233 | 14 |
| D8 | fm/power_extractor.rs extract_stages_with_fuel | 184 | 14 |
| D9 | vm-data service_loop.rs process_polling_cycle | 176 | 14 |
| D10 | vm-webui identify_inflection_points_for_curve | 172 | 14 |
| D11 | platform/tray.rs TrayIcon::new | 160 | 15 |
| D12 | overlays/minihud.rs update_components | 155 | 15 |
| D13 | overlays/fm_unpacked.rs generate_lines | 193 | 15 |
| D14 | overlays/minihud.rs apply_style_to_components | 124 | 15 |
| D15 | lang/mod.rs build_lang | 412 | 17（随 Lang 改造） |

### E. 结构/职责归位

| # | 位置 | 问题 | 波 |
|---|---|---|---|
| E1 | vm-overlay/lib.rs 根 re-export 壳 | 2.5 倍于消费面，双路径并存 | 16 |
| E2 | overlays/mod.rs 域级转发面 | 零消费者 | 16 |
| E3 | overlays/minihud.rs 1538 行 | 四职责（printf/ctx/装配/编排） | 16 |
| E4 | vm-webui/commands_windows.rs 1341 行 | 三域合一 + 命名误导 | 16 |
| E5 | vm-core/config/config_manager.rs 891 行 | 本体+UIStateStorage 桩+弹窗转发三职 | 16 |
| E6 | config_loader.rs jnativehook 键码表 139 项 | 与装载无关 | 16 |
| E7 | vm-overlay/platform/extras.rs | DPI+焦点+声音三合一 | 16 |
| E8 | vm-app/win32.rs 名不符实 | 实为渲染线程装配层 | 16（更名） |
| E9 | AppShell 19 字段 + pub ui_cmd_tx | 旁路直发绕过 dispatch | 16 |
| E10 | vm-ui/main_form.rs run_headless 125 行 | 验收工具混在 lib 核心 | 16 |
| E11 | vm-webui 三种状态注入形态 | dispatcher/OnceLock 桥/ABOUT 静态 | 16 |
| E12 | config_manager.rs 桩 + ui_state_storage | TODO(port) 备案待落地 | 16 |

### F. 数据形态（基本类型偏执/平行结构）

| # | 位置 | 问题 | 裁决 | 波 |
|---|---|---|---|---|
| F1 | 引擎类型 i32 常量族 + check_engine_flag | enum EngineType（保留 as i32 序列化兼容） | 17 |
| F2 | `check_alt > 0` 表英制 ×4 处 | `is_imperial()` | 17 |
| F3 | 哨兵 -65535.0/-65534.0 部分硬编码 | 统一 F_INVALID | 17 |
| F4 | VoiceWarning 15 组三元组 | WarningSlot struct | 17 |
| F5 | FmData 增压器 9 平铺 Option<Vec> | CompressorData | 17 |
| F6 | variabler 返回 [f64;5] 魔法下标 | InterpBounds | 14（随 D3） |
| F7 | GroupConfig 字段清单 8 处手写 | macro 表驱动 | 17 |
| F8 | dto win:i32 / kind:String | enum + serde rename（JSON 不变） | 17 |
| F9 | find_in_structure 返回 -1 哨兵 | Option<usize> | 17 |
| F10 | rows.rs len10..len50 六连参数 | [i32;5]+TickScale | 17 |
| F11 | minihud 风格镜像字段双写 | 组件 getter 化 | 17 |
| F12 | Lang 362 字段三点同步 | 删 Java 原文注释（470 行）+ 中期分组评估 | 12(注释)/17 |
| F13 | FlightLogSnapshot 预格式化 String | 保留（快照边界设计权衡），备案 | — |
| F14 | ServiceData↔Frame 53 字段手写拷贝 | DerivedScalars 整组搬运 或 macro | 17 |
| F15 | OverlayInputs↔ReinitParams 平行拷贝 | 分组嵌套 struct | 17 |
| F16 | -1 哨兵/while 手动索引（commands_windows） | position()+for-range | 14（随 D10） |

### G. 文档与注释

| # | 范围 | 问题 | 波 |
|---|---|---|---|
| G1 | 3534 处 Java 引用 | 注释以死去的 Java 实现为主参照系 | 18 |
| G2 | TODO(port) ~20 处 | 迁移完成裁决落地 | 12/18 |
| G3 | vm-data service_loop 孤儿注释尸体 3 段 | 删除后函数的 doc 错位 | 12 |
| G4 | host.rs 锁纪律注释过时（锁已摘） | 措辞更新 | 12 |
| G5 | vm-app/lib.rs 头注还说 iced | D9 后过时 | 12 |
| G6 | fallback_physical_file doc 与实现不符 + 单元素循环 | 改 doc | 12 |
| G7 | rust/README.md 偏历史记录 | 新人架构导览重写 | 18 |
| G8 | Java typo 保真（thurst_percent 等） | 择机更名 | 17 |
| G9 | dto.rs 区块头还写"三窗口"（飞行记录已删） | 更新 | 12 |

### H. 裁决为保留（不动）

| # | 内容 | 理由 |
|---|---|---|
| H1 | stringly Result<_,String> 命令错误面 | Tauri 惯例 |
| H2 | FormMessageDto ↔ Message 平行枚举 | 单点转换+全变体测试锁定 |
| H3 | 11 个 IPC 命令同构一行体 | 宏化反而降清晰度 |
| H4 | web_windows url_escape 手写 | 域内字符集有限有测试 |
| H5 | 全局注入面 static RwLock/Mutex（LEGACY_SCREEN_SIZE 等） | 测试桩设计，注释完备 |
| H6 | FlightLogSnapshot 预格式化 String | 快照边界保真设计 |
| H7 | `#[allow(too_many_arguments)]` 40 处 | Java 签名锚定，批量改稀释可追溯性；仅收拾 len 族（F10） |
| H8 | Java 死字段移植群（minihud throttley 等） | 迁移宪法 §2.10 保真保留，统一 DEAD(kept) 标注 |
| H9 | locate_template_cfg CWD 四级探测 | 仅 headless 工具使用 |
| H10 | commands_windows 直算不下放 blocking 池 | comctl32 依赖实锤备案 |

## 收官结论 (2026-09-04, 波19 后)

- **登记兑现度**: 两路独立 fresh-eyes 复核 (对照本表逐条核验) 报告 ~95% 已真实解决;
  复核揪出的 5 处收敛漏网 (java_round×2/sleep_while_run/FmtArg 第二实现/
  java_format_f4/f1 算法级副本) 与 3 个新问题已在波19 全部收割。
- **剩余 >150 行函数 (16 个) 裁决为可接受**: 均为数据表形态 (build_lang 362 项
  i18n 赋值表 / fm_direct_vars 等公式注册表, 一行一条数据) 或刻意保守拆分的
  物理敏感分支 (variabler_wep, 位级对拍护航); 《重构》关注的复杂度长度而非
  行数长度, 这些不是逻辑函数。
- **验收基线**: 1256 测试全绿 + clippy 0 + cargo fmt 全库统一。
- **A6 边角语义变化备案** (波17): VoiceWarning 的 fuel_p_check 共享计数器拆分后,
  引擎损坏告警严格 10 tick 计满 (原共享下交织场景 5~6 tick), 消除"油压 tick
  加速触发损坏告警"的怪癖; 测试断言已同步并注明依据。

## 增补: 波20 — 8111 链现代化 (2026-09-03, 登记表收官后的独立整改)

波19 收官后用户裁决对 8111 数据链全面整改 (原 telemetry 域整体退役),
Java 保真移植时代的产物换为业界方案。分四步提交, 每步测试全绿:

| 步 | 内容 | 关键退役 |
|---|---|---|
| 20-1 | 死代码清场 | map_service/OtherService 833 行 (无 wiring) + HudMsg 197 行 + MapObj 实例解析路径 ~570 行 (仅留 get_player_loc/dir 正则) + send_get_url/fm_cmd_set_alt/spd 死方法 + string_helper 死函数 + lang oSkeyWord 键 |
| 20-2 | parser serde 化 | 手写子串扫描 (find 键名→扫冒号→取逗号) → serde_json::Value 全等键取数; string_helper 整文件退役 (哨兵常量迁 parser) |
| 20-3 | HTTP ureq 化 | 手写 socket HTTP → ureq 2.12 (default-features=false, 无 TLS); http.rs→client.rs, HttpHelper→GameApiClient; 8111 硬编码→端口注入 |
| 20-4 | 域更名 | telemetry → game_api (19 文件路径) |

### 修好的保真怪癖 (原 oracle 锁定, 波20 裁决为有意变更)

1. **MapInfo +3 偏移 bug**: 手写扫描 `bix = eix + 3` 系统性丢数值首字符/负号
   (6400.0→400.0、-32768.0→32768.0), 下游地图几何首次得到正确输入。
2. **f32 单精度位级复刻退役**: Float.parseFloat 的 f32 拓宽 (0.1→0.10000000149011612)
   → serde f64 直读; w2_deriver 位级 oracle 重录 (第 8~10 位有效数字漂移)。
3. **valid 真实 bool 化**: JSON 里就是 bool, 原字符串比较是手写解析的产物。
4. **pedals 显式映射**: 快照无裸 pedals 键 (手写 needle 子串碰撞实际取 pedals1) → 显式 "pedals1"。
5. **army=="tank" 死分支删除**: 手写时代字符串值带引号永不等于 "tank", 过滤名存实亡。
6. **子串碰撞/值截断/find-rfind 不一致** 随手写扫描器整体消失; 键名对照真机快照
   (script/mock_scenarios/snapshots/) 逐字段核对为全等真键。

### 语义保留 (下游契约不动)

- 哨兵 -65535 (I_INVALID/F_INVALID): hud_calculator/voice_warning/formula registry
  的缺数据守卫判定契约, serde 版保持"缺键→哨兵"产出。
- State::update 返回 -1 = 端口翻转协议 (vm-data 轮询依赖)。
- str_state Arc<Mutex> 测试注入面; /state 失败双双空串复位; 250ms/500ms 超时上限。
- 引擎数组"先写哨兵再 break"产出形态; 哨兵归一化 (rpm_throttle→-1 等)。

### 验收

cargo test --workspace 全绿 (1192 测试, 含 9222 mock_e2e 集成 — ureq 与
mock_8111.py 真 HTTP 互通验证); 未跑 script/rust_e2e.sh / --mock-smoke (项目约束)。
