	use super::*;
	use crate::fm::loader;
	use crate::fm::status::FMStatus;
	use std::path::{Path, PathBuf};
	use std::time::{Duration, Instant};

	/// 轮询等待的超时上限（合成文件很小，正常毫秒级完成；10s 是宽松上界）
	const WAIT_TIMEOUT_MS: u64 = 10_000;

	/// DATA_ROOT 的全部可能取值: 默认根 + java_main_sequence 临时注入的根
	/// (store_tests.rs ROOTS 同款铺根方案; 本模块不翻转 DATA_ROOT)
	const ROOTS: [&str; 3] = ["./data", "testroot", "otherroot"];

	/// Java check(boolean, String) 计数式断言 → assert! 宏 (失败即 panic),
	/// 描述逐字保留 (handle.rs 先例)
	fn check(cond: bool, desc: &str) {
		assert!(cond, "FAIL: {desc}");
	}

	/// 轮询等待条件成立（20ms 间隔），超时返回最后一次求值结果
	fn wait_for(mut cond: impl FnMut() -> bool) -> bool {
		let deadline = Instant::now() + Duration::from_millis(WAIT_TIMEOUT_MS);
		let mut v = cond();
		while !v && Instant::now() < deadline {
			thread::sleep(Duration::from_millis(20));
			v = cond();
		}
		v
	}

	// ---- 合成数据 (store_tests.rs 同款, 多根铺数据) ----

	fn fm_dir_of(root: &str) -> String {
		format!("{root}/aces/gamedata/flightmodels")
	}

	/// 合成数据铺设 (blkx→json 迁移终态: 只铺 .json)
	fn write_json(root: &str, rel: &str, json_text: &str) {
		std::fs::write(format!("{}/{rel}.json", fm_dir_of(root)), json_text).unwrap();
	}

	/// 最小中央文件 —— 只需 get_last_string_ci("fmfile") 能命中
	fn write_central(root: &str, name: &str) {
		write_json(
			root,
			name,
			&format!("{{\"model\": \"{name}\", \"fmFile\": \"fm/{name}.blk\"}}"),
		);
	}

	/// 最小物理 FM —— 顶层标量的等价树; getload 对缺失字段全按 0 处理
	/// （无 Jet/Compressor 块 → 按喷气形态、compNumSteps=0，extractStages 返回
	/// null、peakThrust=0），最终 valid=true → READY。
	fn write_physical(root: &str, name: &str) {
		write_json(
			root,
			&format!("fm/{name}"),
			"{\"synthetic-fm\": \"x\", \"EmptyMass\": 1000.0, \"Wingspan\": 11.0}",
		);
	}

	fn setup_synthetic_data() {
		for root in ROOTS {
			std::fs::create_dir_all(format!("{}/fm", fm_dir_of(root))).unwrap();
			// 可加载机型: central 指向 fm/<name>.blk, 物理文件存在
			write_central(root, "plane1");
			write_physical(root, "plane1");
			// ghost: 什么都不写 → MISSING
		}
	}

	/// 清理: 只删本测试落盘的文件; 目录仅在其为空时移除 (绝不触动既有 data/ 内容)
	fn cleanup_synthetic_data() {
		for root in ROOTS {
			let name = "plane1";
			let _ = std::fs::remove_file(format!("{}/{name}.json", fm_dir_of(root)));
			let _ = std::fs::remove_file(format!("{}/fm/{name}.json", fm_dir_of(root)));
			// 自内向外逐层 prune 空目录 (remove_dir 对非空目录失败即止)
			for dir in [
				format!("{}/fm", fm_dir_of(root)),
				fm_dir_of(root),
				format!("{root}/aces/gamedata"),
				format!("{root}/aces"),
				root.to_string(),
			] {
				let _ = std::fs::remove_dir(dir);
			}
		}
	}

	/// Drop 兜底清理 (断言 panic 展栈时也还原落盘的合成文件)
	struct CleanupOnDrop;
	impl Drop for CleanupOnDrop {
		fn drop(&mut self) {
			cleanup_synthetic_data();
		}
	}

	/// 边界补充 (store_tests.rs 注释引用本测试, 不在其内重复): identify 的空名
	/// 守卫 —— "null/空直接忽略" (identify javadoc; 与 FMLoader.load 的空名守卫
	/// 双保险)。落在 UNRESOLVED 哨兵上, 零任务零加载。
	#[test]
	fn identify_null_and_empty_are_ignored() {
		// 挂锁 + 清零 (审查 A/B B2): 断言进程级全局 get_load_count()==0 —— 本用例
		// 自身不碰 DATA_ROOT, 但须与同二进制并行的真加载方 (store_tests/loader
		// 用例的 fetch_add 窗口) 互斥; 持锁先行测试会留下非零计数, 故挂锁后先
		// 清零再断言 (fm_loader.rs W-B2 备案的兑现)
		let _guard = crate::fm::test_support::data_root();
		loader::reset_load_count();
		let m = FMManager::new(Arc::new(EventBus::new()));
		m.identify(None);
		m.identify(Some(""));
		check(
			m.current().status == FMStatus::Unresolved,
			"null/空名 → current 保持 UNRESOLVED",
		);
		check(m.current_target_name().is_none(), "null/空名不建立目标");
		check(
			!m.is_loading() && loader::get_load_count() == 0,
			"null/空名零任务零加载",
		);
	}

	/// 补充 (store_tests 未覆盖): 负缓存**命中分支** —— "READY → 缺失机型 →
	/// 切走重载回 READY → 再切缺失机型" 驱动 `negativeCache.containsKey` 真正拦截:
	/// 同步落 MISSING、零磁盘加载、FM_CHANGED 在 identify 返回前同步送达订阅方
	/// (Java UIStateBus 同步派发语义), 载荷 = 句柄本体 (专用通道消息, 见
	/// FmChangedBus 的 PORT 注)。
	#[test]
	fn negative_cache_hit_branch_and_sync_dispatch() {
		let _guard = crate::fm::test_support::data_root();
		let _cleanup = CleanupOnDrop;
		setup_synthetic_data();

		let bus: Arc<FmChangedBus> = Arc::new(EventBus::new());
		let events: Arc<Mutex<Vec<FMHandle>>> = Arc::new(Mutex::new(Vec::new()));
		let ev = Arc::clone(&events);
		let _sub = bus.subscribe(move |h: &FMHandle| {
			ev.lock().unwrap().push(h.clone());
		});
		let m = FMManager::new(Arc::clone(&bus));
		m.reset();
		loader::reset_load_count();
		events.lock().unwrap().clear();

		m.identify(Some("plane1"));
		check(wait_for(|| m.current().has_fm()), "前置: plane1 READY");

		// 首次 ghost: 走 loader 线程, MISSING 落负缓存
		m.identify(Some("ghost"));
		check(
			wait_for(|| m.current().is_missing_like() && !m.is_loading()),
			"首次 ghost 落定 MISSING 并进负缓存",
		);
		check(loader::get_load_count() == 2, "plane1 + ghost 共 2 次真实加载");

		// 目标切走 (current 已是 MISSING, 句柄不在 → 真实重载回 READY)
		m.identify(Some("plane1"));
		check(wait_for(|| m.current().has_fm()), "切回 plane1 真实重载回 READY");
		check(loader::get_load_count() == 3, "切走又切回放行重载 (护栏不拦)");

		// 再切 ghost: 负缓存命中 → 同步落 MISSING, 不发任务
		events.lock().unwrap().clear();
		m.identify(Some("ghost"));
		check(
			m.current().status == FMStatus::Missing && !m.is_loading(),
			"负缓存命中: identify 返回前同步落 MISSING, 无在途任务",
		);
		check(
			loader::get_load_count() == 3,
			"负缓存命中不再触发磁盘加载",
		);
		// 同步派发: publish 在 identify 调用线程上逐订阅方执行完毕,
		// 订阅方此刻读 current() 即刚发布的句柄 (Java 三处发布点均先写 current
		// 再 publish, publish_fm_changed PORT 注口径)
		let seen = events.lock().unwrap().clone();
		check(
			seen.last().map(|h| (h.status, h.name.clone()))
				== Some((FMStatus::Missing, Some("ghost".to_string()))),
			&format!("订阅方应已同步收到 MISSING(ghost) 句柄载荷 (实际 {seen:?})"),
		);
		check(m.current().status == FMStatus::Missing, "订阅方视角 current() 一致");

		// NOT_AIRCRAFT 短路分支同样同步派发 (零磁盘加载)
		events.lock().unwrap().clear();
		m.identify(Some("tankmodels/germ_panther_ii"));
		check(m.current().status == FMStatus::NotAircraft, "坦克同步落定 NOT_AIRCRAFT");
		check(
			events.lock().unwrap().last().map(|h| h.status) == Some(FMStatus::NotAircraft),
			"NOT_AIRCRAFT 分支同步派发 FM_CHANGED",
		);
		check(loader::get_load_count() == 3, "坦克短路零磁盘加载");
	}

	/// 补充 (Java 测试未覆盖): invalidate 手动作废负缓存 —— 连 lastAttemptMs
	/// 一并移除 (Java invalidate 双 remove), 下次 identify 重新走磁盘加载;
	/// 大小写/空白规范化后命中同一键; null 入参无操作。
	#[test]
	fn invalidate_clears_negative_cache_entry() {
		let _guard = crate::fm::test_support::data_root();
		let _cleanup = CleanupOnDrop;
		setup_synthetic_data();

		let m = FMManager::new(Arc::new(EventBus::new()));
		m.reset();
		loader::reset_load_count();

		m.identify(Some("ghost"));
		check(
			wait_for(|| m.current().status == FMStatus::Missing && !m.is_loading()),
			"前置: ghost 落定 MISSING 进负缓存",
		);

		// 作废 (大小写/空白规范化命中同一键) → 换目标 → 回切 ghost:
		// 缓存未命中 + 护栏时间戳同被清除 → 重新发任务真实加载
		m.invalidate(Some("  GHOST  "));
		m.identify(Some("plane1"));
		check(wait_for(|| m.current().has_fm()), "换目标触发真实加载");
		check(loader::get_load_count() == 2, "作废后回切应重新走磁盘 (plane1+ghost)");
		m.identify(Some("ghost"));
		check(
			wait_for(|| m.current().status == FMStatus::Missing),
			"重新加载后再次落定 MISSING",
		);
		check(loader::get_load_count() == 3, "ghost 第二次真实加载");

		m.invalidate(None); // null 守卫: 无操作不 panic
	}

	/// 真机 data/ blkx 的 identify→current 快照→负缓存命中 断言链 (本波次任务
	/// 规则 2; Java 侧无对应用例 —— TestFMStore 刻意不依赖真机 data/, 真机面由
	/// 本测试补)。
	/// PORT(data/ 缺失跳过): realtests.rs / build.py 语义先例 —— CI 无真机数据时
	/// early-return; 存在时走全量断言, 不放宽阈值。
	/// PORT(不翻转 DATA_ROOT): 默认根 "./data" 相对 cargo 测试 cwd (crate 根),
	/// 真机文件在仓库根 data/ —— 复制真机 spitfire 中央+物理文件进 ROOTS 全部
	/// 取值 (默认根因此可直接命中, java_main_sequence 的临时根翻转也不影响判定),
	/// 全程不动 DATA_ROOT (模块头注的竞态备案)。
	#[test]
	fn real_data_identify_chain() {
		let _guard = crate::fm::test_support::data_root();
		let repo_data = format!("{}/../../../data", env!("CARGO_MANIFEST_DIR"));
		let real_fm_dir = Path::new(&repo_data).join("aces/gamedata/flightmodels");
		if !real_fm_dir.join("spitfire_f24.json").exists() {
			println!("跳过: 真机 data/ 不存在 ({})", real_fm_dir.display());
			return;
		}

		// 多根铺数据: 真机 spitfire 中央+物理 JSON 复制进 DATA_ROOT 全部可能取值
		for root in ROOTS {
			let dst = PathBuf::from(root).join("aces/gamedata/flightmodels");
			std::fs::create_dir_all(dst.join("fm")).unwrap();
			for rel in ["spitfire_f24.json", "fm/spitfire_f24.json"] {
				let src = real_fm_dir.join(rel);
				if src.exists() {
					std::fs::copy(&src, dst.join(rel)).unwrap();
				}
			}
		}

		/// Drop 兜底清理 (断言 panic 展栈时也删复制的真机文件; 只删本测试落的
		/// 文件, prune 空目录 —— remove_dir 非空即止, 不动默认根下其他测试文件)
		struct CleanupCopiesOnDrop;
		impl Drop for CleanupCopiesOnDrop {
			fn drop(&mut self) {
				for root in ROOTS {
					let dst = PathBuf::from(root).join("aces/gamedata/flightmodels");
					let _ = std::fs::remove_file(dst.join("spitfire_f24.json"));
					let _ = std::fs::remove_file(dst.join("fm/spitfire_f24.json"));
					let _ = std::fs::remove_dir(dst.join("fm"));
					let _ = std::fs::remove_dir(dst);
					let _ = std::fs::remove_dir(PathBuf::from(root).join("aces/gamedata"));
					let _ = std::fs::remove_dir(PathBuf::from(root).join("aces"));
					let _ = std::fs::remove_dir(root);
				}
			}
		}
		let _cleanup = CleanupCopiesOnDrop;

		let bus: Arc<FmChangedBus> = Arc::new(EventBus::new());
		let events: Arc<Mutex<Vec<FMHandle>>> = Arc::new(Mutex::new(Vec::new()));
		let ev = Arc::clone(&events);
		let _sub = bus.subscribe(move |h: &FMHandle| {
			ev.lock().unwrap().push(h.clone());
		});
		let m = FMManager::new(Arc::clone(&bus));
		m.reset();
		loader::reset_load_count();

		// identify 真机 (任意大小写) → loader 线程加载 → current() 快照 READY
		m.identify(Some("Spitfire_F24"));
		assert!(
			wait_for(|| m.current().has_fm() && !m.is_loading()),
			"真机 spitfire_f24 应加载到 READY, 实际: {} (加载次数 {})",
			m.current(),
			loader::get_load_count()
		);
		let snap = m.current();
		assert_eq!(snap.name.as_deref(), Some("spitfire_f24"), "机型名规范化为小写");
		assert!(snap.fmdata.is_some(), "READY 快照应携带 fmdata");
		assert_eq!(
			snap.fmdata.as_ref().unwrap().read_file_name.as_deref(),
			Some("fm/spitfire_f24.blk"),
			"物理文件 readFileName 链路锁死"
		);
		assert_eq!(loader::get_load_count(), 1, "真机首载 1 次");
		assert_eq!(m.current_target_name().as_deref(), Some("spitfire_f24"));
		check(
			events.lock().unwrap().last().map(|h| h.status) == Some(FMStatus::Ready),
			"READY 句柄经 FM_CHANGED 广播 (载荷本体)",
		);

		// 同目标重复 identify: 去重, 零加载
		m.identify(Some("spitfire_f24"));
		assert_eq!(loader::get_load_count(), 1, "目标去重应零加载");

		// 缺失机型: 首次走 loader → MISSING 进负缓存
		m.identify(Some("zzfm_no_such_plane"));
		assert!(
			wait_for(|| m.current().status == FMStatus::Missing && !m.is_loading()),
			"缺失机型应落定 MISSING"
		);
		assert_eq!(loader::get_load_count(), 2);

		// 回切真机 (护栏放行: current.name != spitfire) → 重载 READY
		m.identify(Some("spitfire_f24"));
		assert!(wait_for(|| m.current().has_fm()), "回切真机应重新 READY");
		assert_eq!(loader::get_load_count(), 3);

		// 负缓存命中: 再切缺失机型 → 同步落 MISSING, 零加载, 事件同步派发
		events.lock().unwrap().clear();
		m.identify(Some("zzfm_no_such_plane"));
		assert_eq!(m.current().status, FMStatus::Missing, "负缓存命中同步落 MISSING");
		assert!(!m.is_loading(), "负缓存命中无在途任务");
		assert_eq!(loader::get_load_count(), 3, "负缓存命中零磁盘加载");
		check(
			events.lock().unwrap().last().map(|h| h.status) == Some(FMStatus::Missing),
			"FM_CHANGED 同步派发 MISSING 句柄",
		);

		// NOT_AIRCRAFT 短路: 坦克 type 零磁盘加载, 同步落定
		m.identify(Some("tankmodels/us_n4a3e8_76_sherman"));
		assert_eq!(m.current().status, FMStatus::NotAircraft);
		assert_eq!(loader::get_load_count(), 3, "坦克短路零加载");
	}
