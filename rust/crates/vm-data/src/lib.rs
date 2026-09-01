//! vm-data: 8111 轮询与派生量计算
pub mod service_fields;
pub mod service_loop;

/// 测试基础设施: `fm_data_paths::set_data_root` 进程级全局态的串行锁。
/// 本 crate 两个测试 (format_strings 的 nitro 场景注入 tmp 根 / methods_engine
/// 的真机管道注入 repo data 根) 并行注入时互相踩踏 — workspace 全量并行下
/// 复现 flake: identify 等待期全局根被另一测试覆盖 → MISSING 断言失败。
/// 持锁窗口 = 各测试的 set_data_root → 用毕复位 全程 (守卫声明在其余局部
/// 之前, 保证复位后才释放)。生产代码无 DATA_ROOT 运行时写入, 不涉及。
#[cfg(test)]
pub(crate) static DATA_ROOT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
