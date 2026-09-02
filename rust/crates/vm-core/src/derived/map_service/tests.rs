	use super::*;
	use std::io::Read;
	use std::net::{TcpListener, TcpStream};
	use std::time::Duration;

	/// 8111 相关测试串行锁 (cargo test 并行线程共用进程端口空间)
	static PORT8111_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

	/// 固定游标 sink (ControllerMsgIdSource 测试替身)
	struct FixedSink {
		evt: i32,
		dmg: i32,
	}
	impl ControllerMsgIdSource for FixedSink {
		fn last_evt(&self) -> i32 {
			self.evt
		}
		fn last_dmg(&self) -> i32 {
			self.dmg
		}
	}

	/// calculate 场景 R1 (紧凑格式): pla=Player(0.5,0.5,0.274,0.962),
	/// slc=movsel(0.6,0.6,0.7,-0.3), mov0=红(0.55,0.9), mov1=绿(0.53,0.9),
	/// mov2=蓝(0.9,0.9)。cmapmaxsize 由测试直接置 1.0 (mapi 字段 pub)。
	const CALC_R1: &str = "[{\"type\":\"aircraft\",\"color\":\"#faC81E\",\"color[]\":[250,200,30],\"blink\":0,\"icon\":\"Player\",\"icon_bg\":\"none\",\"x\":0.5,\"y\":0.5,\"dx\":0.274,\"dy\":0.962},{\"type\":\"aircraft\",\"color\":\"#f00C00\",\"color[]\":[240,12,0],\"blink\":1,\"icon\":\"EnemyFighter\",\"icon_bg\":\"none\",\"x\":0.55,\"y\":0.9,\"dx\":-0.5,\"dy\":0.25},{\"type\":\"aircraft\",\"color\":\"#0AA00\",\"color[]\":[10,220,5],\"blink\":1,\"icon\":\"Friend\",\"icon_bg\":\"none\",\"x\":0.53,\"y\":0.9,\"dx\":0.1,\"dy\":0.2},{\"type\":\"aircraft\",\"color\":\"#0000FA\",\"color[]\":[0,0,250],\"blink\":0,\"icon\":\"Friend2\",\"icon_bg\":\"none\",\"x\":0.9,\"y\":0.9,\"dx\":0.1,\"dy\":0.2},{\"type\":\"movsel\",\"color\":\"#00AA00\",\"color[]\":[0,170,0],\"blink\":2,\"icon\":\"Squad\",\"icon_bg\":\"selbg\",\"x\":0.6,\"y\":0.6,\"dx\":0.7,\"dy\":-0.3}]";

	/// R2: mov0 移近 (0.55→0.505), mov1 移近 (0.53→0.51), mov2 不动
	/// (0.9 — 第二轮 sdistance 与上轮相等, 严格小于判定不过)
	const CALC_R2: &str = "[{\"type\":\"aircraft\",\"color\":\"#faC81E\",\"color[]\":[250,200,30],\"blink\":0,\"icon\":\"Player\",\"icon_bg\":\"none\",\"x\":0.5,\"y\":0.5,\"dx\":0.274,\"dy\":0.962},{\"type\":\"aircraft\",\"color\":\"#f00C00\",\"color[]\":[240,12,0],\"blink\":1,\"icon\":\"EnemyFighter\",\"icon_bg\":\"none\",\"x\":0.505,\"y\":0.9,\"dx\":-0.5,\"dy\":0.25},{\"type\":\"aircraft\",\"color\":\"#0AA00\",\"color[]\":[10,220,5],\"blink\":1,\"icon\":\"Friend\",\"icon_bg\":\"none\",\"x\":0.51,\"y\":0.9,\"dx\":0.1,\"dy\":0.2},{\"type\":\"aircraft\",\"color\":\"#0000FA\",\"color[]\":[0,0,250],\"blink\":0,\"icon\":\"Friend2\",\"icon_bg\":\"none\",\"x\":0.9,\"y\":0.9,\"dx\":0.1,\"dy\":0.2},{\"type\":\"movsel\",\"color\":\"#00AA00\",\"color[]\":[0,170,0],\"blink\":2,\"icon\":\"Squad\",\"icon_bg\":\"selbg\",\"x\":0.6,\"y\":0.6,\"dx\":0.7,\"dy\":-0.3}]";

	/// 无选择目标场景: update 后 slc.type="" → 主分支整体跳过
	const CALC_NOSEL: &str = "[{\"type\":\"aircraft\",\"color\":\"#faC81E\",\"color[]\":[250,200,30],\"blink\":0,\"icon\":\"Player\",\"icon_bg\":\"none\",\"x\":0.5,\"y\":0.5,\"dx\":0.274,\"dy\":0.962},{\"type\":\"aircraft\",\"color\":\"#f00C00\",\"color[]\":[240,12,0],\"blink\":1,\"icon\":\"EnemyFighter\",\"icon_bg\":\"none\",\"x\":0.55,\"y\":0.9,\"dx\":-0.5,\"dy\":0.25}]";

	/// 真机 /map_info 快照 (map_info.rs 同款, oracle: cmapmaxsize_x/y = -30000)
	const MAP_INFO_MOCK: &str = "{\"grid_size\": [57584.11328125,64194.1953125],\"grid_steps\": [6400.0,6400.0],\"grid_zero\": [-24816.11328125,31426.1953125],\"hud_type\": 0,\"map_generation\": 9,\"map_max\": [32768.0,32768.0],\"map_min\": [-32768.0,-32768.0],\"valid\": true}";

	/// hudmsg 响应 (hud_msg.rs 同款, oracle: 末对象 id=222, msg 含 "热")
	const HUDMSG_MULTI: &str = "{\"events\": [],\"damage\": [{\"id\": 111,\"msg\": \"first\"}, {\"id\": 222,\"msg\": \"second 热msg\"}]}";

	fn calc_fixture() -> OtherService {
		let mut os = OtherService::new("\n");
		os.mapo.init();
		os.mapi.cmapmaxsize_x = 1.0;
		os.mapi.cmapmaxsize_y = 1.0;
		os.dislmt = 1200;
		os
	}

	/// 8111 是否有服务应答 (e2e 先例: 被占即跳过网络相关断言)。
	/// PORT: 用 connect 探测而非 bind —— Windows 下占用者开 SO_REUSEADDR 时
	/// bind 同端口也成功 (本机 MockServer/3.0 实测), bind 探测会误判空闲
	fn port_8111_answered() -> bool {
		TcpStream::connect("127.0.0.1:8111").is_ok()
	}

	// ---- 1. angle_toclock (Java oracle) ----

	#[test]
	fn angle_toclock_oracle() {
		assert_eq!(OtherService::angle_toclock(0.0), 0.0); // 12 → 减 12
		assert!((OtherService::angle_toclock(30.0) - 1.0).abs() < 1e-9);
		assert!((OtherService::angle_toclock(-30.0) - 11.0).abs() < 1e-9);
		assert!((OtherService::angle_toclock(330.0) - 11.0).abs() < 1e-9);
		assert!((OtherService::angle_toclock(360.0) - 12.0).abs() < 1e-9);
		assert!((OtherService::angle_toclock(60.0) - 2.0).abs() < 1e-9);
		assert!((OtherService::angle_toclock(179.9) - 5.996666666667).abs() < 1e-9);
		assert!((OtherService::angle_toclock(-0.1) - 11.996666666667).abs() < 1e-9);
	}

	// ---- 2. dxdy_to_angle: 象限矩阵 + 0 边界 (Java oracle) ----

	#[test]
	fn dxdy_to_angle_quadrants() {
		assert!((OtherService::dxdy_to_angle(1.0, 1.0) - 45.0).abs() < 1e-9);
		assert!((OtherService::dxdy_to_angle(1.0, -1.0) - 315.0).abs() < 1e-9);
		assert!((OtherService::dxdy_to_angle(-1.0, 1.0) - 135.0).abs() < 1e-9);
		assert!((OtherService::dxdy_to_angle(-1.0, -1.0) - 225.0).abs() < 1e-9);
		assert!((OtherService::dxdy_to_angle(2.0, 3.0) - 56.309932474020).abs() < 1e-9);
		assert!((OtherService::dxdy_to_angle(-2.0, 3.0) - 123.690067525980).abs() < 1e-9);
		assert!((OtherService::dxdy_to_angle(3.0, -2.0) - 326.309932474020).abs() < 1e-9);
		assert!((OtherService::dxdy_to_angle(-3.0, -2.0) - 213.690067525980).abs() < 1e-9);
	}

	#[test]
	fn dxdy_to_angle_zero_edges_double_fire() {
		// dy=0 同时满足 >=0 与 <=0 → 两个 if 连续叠加 (Java 顺序执行保真)
		assert!((OtherService::dxdy_to_angle(1.0, 0.0) - 360.0).abs() < 1e-9); // 仅第三 if
		assert!((OtherService::dxdy_to_angle(-1.0, 0.0) - 360.0).abs() < 1e-9); // 第一+第二 if
		assert!((OtherService::dxdy_to_angle(0.0, 1.0) - 270.0).abs() < 1e-9); // atan(+∞)=90
		// dx=0 同时 <=0 与 >=0: 第二 if 后 90, 第三 if 再叠 → 450
		assert!((OtherService::dxdy_to_angle(0.0, -1.0) - 450.0).abs() < 1e-9);
		// 0/0 = NaN → atan(NaN)=NaN, NaN 比较恒 false → 原样返回 NaN
		assert!(OtherService::dxdy_to_angle(0.0, 0.0).is_nan());
	}

	// ---- 3. send_get: 本地回环服务器 (临时端口, 不占 8111) ----

	/// 起一次性服务器: 收首个请求后回固定响应并关闭
	fn one_shot_server(response: &'static str) -> (std::net::SocketAddr, std::thread::JoinHandle<(String, String)>) {
		let listener = TcpListener::bind("127.0.0.1:0").unwrap();
		let addr = listener.local_addr().unwrap();
		let handle = std::thread::spawn(move || {
			let (mut sock, _) = listener.accept().unwrap();
			let mut req = Vec::new();
			let mut b = [0u8; 256];
			while req.iter().filter(|c| **c == b'\n').count() < 2 {
				let n = sock.read(&mut b).unwrap_or(0);
				if n == 0 {
					break;
				}
				req.extend_from_slice(&b[..n]);
			}
			let text = String::from_utf8_lossy(&req).to_string();
			let l1 = text.lines().next().unwrap_or("").to_string();
			let l2 = text.lines().nth(1).unwrap_or("").to_string();
			sock.write_all(response.as_bytes()).unwrap();
			(l1, l2)
		});
		(addr, handle)
	}

	#[test]
	fn send_get_assembles_body_and_strips_newlines() {
		// 6 行响应头 + 空行 + 两行 body (换行被 readLine 剥离后拼接)
		let (addr, srv) = one_shot_server(
			"HTTP/1.1 200 OK\r\nServer: mock\r\nDate: x\r\nContent-Type: json\r\nContent-Length: 14\r\nConnection: close\r\n\r\n{\"a\": 1,\n\"b\": 2}\n",
		);
		let os = OtherService::new("\n");
		let result = os.send_get("127.0.0.1", addr.port(), "/map_obj.json").unwrap();
		assert_eq!(result, "{\"a\": 1,\"b\": 2}");
		let (l1, l2) = srv.join().unwrap();
		assert_eq!(l1, "GET /map_obj.json HTTP/1.1");
		assert_eq!(l2, "Host: 127.0.0.1");
	}

	#[test]
	fn send_get_short_response_returns_empty() {
		// 6 行丢弃读取遇 EOF: Java readLine null 无循环体 → 空串
		let (addr, srv) = one_shot_server("HTTP/1.1 200 OK\r\nServer: mock\r\n");
		let os = OtherService::new("\n");
		let result = os.send_get("127.0.0.1", addr.port(), "/x").unwrap();
		assert_eq!(result, "");
		srv.join().unwrap();
	}

	#[test]
	fn send_get_refused_connection_is_err() {
		// 临时端口 bind 后立即 drop → 连接被拒 (Java IOException ↔ Err)
		let listener = TcpListener::bind("127.0.0.1:0").unwrap();
		let port = listener.local_addr().unwrap().port();
		drop(listener);
		let os = OtherService::new("\n");
		assert!(os.send_get("127.0.0.1", port, "/x").is_err());
	}

	// ---- 4. calculate (纯逻辑, 无网络) ----

	#[test]
	fn calculate_round1_oracle() {
		let mut os = calc_fixture();
		os.mapo.update(CALC_R1);
		// dt = 1000ms ± 执行耗时 (enemyspeed 容差断言)
		os.speed_check_mili = current_time_millis() - 1000;
		os.calculate();
		// slc 分支 (type != "", dx != 0, distance < dislmt)
		assert!((os.distance - 0.141421389955).abs() < 1e-12);
		assert_eq!(os.enemycount, 1);
		assert_eq!(os.friendcount, 0);
		assert!((os.aot - 97.300405405).abs() < 1e-6);
		assert!((os.azi - 11.029939543).abs() < 1e-6);
		assert!((os.enemyspeed - 0.848528171).abs() < 1e-3);
		// 首轮 mov[i].distance 初值 0: sdistance < 0 恒假 → 不计数 (Java 原行为)
		// 但 distance 字段仍被写入
		assert!((os.mapo.mov[0].distance - 0.050000011921).abs() < 1e-12);
		assert!((os.mapo.mov[1].distance - 0.029999971390).abs() < 1e-12);
		assert!((os.mapo.mov[2].distance - 0.399999976158).abs() < 1e-12);
		// speed_check_mili 已刷新
		assert!(os.speed_check_mili >= current_time_millis() - 100);
	}

	#[test]
	fn calculate_round2_counts_approaching_only() {
		let mut os = calc_fixture();
		os.mapo.update(CALC_R1);
		os.speed_check_mili = current_time_millis() - 1000;
		os.calculate();
		// run() 每轮头部行为: 计数清零; 尾部行为: p_x/p_y = slc 坐标
		os.enemycount = 0;
		os.friendcount = 0;
		os.p_x = os.mapo.slc.x;
		os.p_y = os.mapo.slc.y;
		os.mapo.update(CALC_R2);
		os.speed_check_mili = current_time_millis() - 1000;
		os.calculate();
		// mov0 红 240>200 接近中 → 敌 +1; mov1 绿 220>200 接近中 → 友 +1;
		// mov2 蓝未接近 (sdistance == 上轮, 严格 < 不过) → 不计
		assert_eq!(os.enemycount, 2); // slc 分支 1 + mov0 1
		assert_eq!(os.friendcount, 1);
		// p_x/p_y 已同步 slc 坐标 → enemyspeed 分子 = 0 (slc 静止)
		assert_eq!(os.enemyspeed, 0.0);
	}

	#[test]
	fn calculate_no_selection_skips_main_branch() {
		let mut os = calc_fixture();
		os.mapo.update(CALC_NOSEL);
		os.calculate();
		// slc.type = "" → 主分支跳过 (§2.6)
		assert_eq!(os.distance, 0.0);
		assert_eq!(os.enemycount, 0);
		assert_eq!(os.friendcount, 0);
		assert_eq!(os.aot, 0.0);
		assert_eq!(os.azi, 0.0);
		// mov 循环仍执行: 首轮 distance 初值 0 → 不计数但写入
		assert!((os.mapo.mov[0].distance - 0.050000011921).abs() < 1e-12);
	}

	// ---- 5. close / init ----

	#[test]
	fn close_clears_is_run() {
		let os = OtherService::new("\n");
		assert!(!os.is_run.load(Ordering::SeqCst));
		os.is_run.store(true, Ordering::SeqCst); // init 的置位动作
		os.close();
		assert!(!os.is_run.load(Ordering::SeqCst));
	}

	#[test]
	fn init_sets_fields_and_takes_controller_ids() {
		let _g = PORT8111_LOCK.lock().unwrap();
		let answered = port_8111_answered();
		let mut os = OtherService::new("\n");
		os.init(Arc::new(FixedSink { evt: 7, dmg: 9 }));
		assert!(os.is_run.load(Ordering::SeqCst));
		assert_eq!(os.last_evt.load(Ordering::SeqCst), 7); // 覆盖死存储 0
		assert_eq!(os.last_dmg.load(Ordering::SeqCst), 9);
		assert_eq!(os.dislmt, 1200);
		assert_eq!(os.p_x, 0.0);
		assert_eq!(os.p_y, 0.0);
		assert!(os.is_get_msg);
		assert!(os.is_get_map_obj);
		assert!(!os.is_overheat);
		assert_eq!(os.mapo.mov.len(), 500); // init 分配
		assert!(os.msg.dmg.is_some());
		if !answered {
			// 8111 无应答 → 连接被拒 → 空 sMapInfo, mapi 保持默认 0
			assert!(os.s_map_info.is_none());
			assert_eq!(os.mapi.cmapmaxsize_x, 0.0);
		}
	}

	// ---- 6. run: 8111 不可达时线程死亡 (Java NPE 保真) ----

	#[test]
	fn run_thread_dies_like_java_npe_when_fetch_fails() {
		let _g = PORT8111_LOCK.lock().unwrap();
		let listener = match TcpListener::bind("127.0.0.1:8111") {
			Ok(l) => l,
			Err(_) => {
				eprintln!("跳过: 8111 绑定失败 (e2e 先例)");
				return;
			}
		};
		listener.set_nonblocking(true).unwrap();
		let stop = Arc::new(AtomicBool::new(false));
		let stop_srv = stop.clone();
		// RST 服务器: 只读 1 字节即关闭 → 未读数据触发 RST → 客户端 read 得
		// ConnectionReset → send_get Err。PORT: 本机常驻 MockServer(SO_REUSEADDR)
		// 使"连接拒绝"不可构造, RST 与拒绝对 run() 等价 (fetch 失败 → IOException)
		let server = std::thread::spawn(move || {
			while !stop_srv.load(Ordering::SeqCst) {
				match listener.accept() {
					Ok((mut sock, _)) => {
						sock.set_nonblocking(false).unwrap();
						let mut b = [0u8; 1];
						let _ = sock.read(&mut b);
						drop(sock);
					}
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
						std::thread::sleep(Duration::from_millis(5));
					}
					Err(_) => break,
				}
			}
		});
		// 自证: 探针读必须得到 Err(RST) — 若拿到数据说明连接被外部占用者应答
		{
			let mut probe = match TcpStream::connect("127.0.0.1:8111") {
				Ok(s) => s,
				Err(_) => {
					stop.store(true, Ordering::SeqCst);
					server.join().unwrap();
					eprintln!("跳过: 8111 探针连接失败 (e2e 先例)");
					return;
				}
			};
			let _ = probe.write_all(b"GET /__probe HTTP/1.1\r\n\r\n");
			let _ = probe.set_read_timeout(Some(Duration::from_millis(2000)));
			let mut buf = Vec::new();
			if probe.read_to_end(&mut buf).is_ok() {
				stop.store(true, Ordering::SeqCst);
				server.join().unwrap();
				eprintln!("跳过: 8111 连接由外部占用者应答 (e2e 先例)");
				return;
			}
		}

		let mut os = OtherService::new("\n");
		os.init(Arc::new(FixedSink { evt: 0, dmg: 0 })); // map_info 拉取失败路径
		assert!(os.s_map_info.is_none()); // init 的 fetch 同样失败
		let is_run = os.is_run.clone();
		let handle = std::thread::spawn(move || os.run());
		// 首轮 fetch 失败 → s_map_obj 保持 None → update(null) NPE ↔ unwrap panic
		assert!(handle.join().is_err(), "线程应在 unwrap(None) panic 中死亡");
		assert!(is_run.load(Ordering::SeqCst));

		stop.store(true, Ordering::SeqCst);
		server.join().unwrap();
	}

	// ---- 7. run: mock 8111 完整轮次 ----

	#[test]
	fn run_with_mock_8111_completes_round_and_exits_on_close() {
		let _g = PORT8111_LOCK.lock().unwrap();
		let listener = match TcpListener::bind("127.0.0.1:8111") {
			Ok(l) => l,
			Err(_) => {
				eprintln!("跳过: 8111 绑定失败 (e2e 先例)");
				return;
			}
		};
		listener.set_nonblocking(true).unwrap();
		let stop = Arc::new(AtomicBool::new(false));
		let stop_srv = stop.clone();
		let server = std::thread::spawn(move || {
			while !stop_srv.load(Ordering::SeqCst) {
				match listener.accept() {
					Ok((mut sock, _)) => {
						sock.set_nonblocking(false).unwrap();
						// 读到请求首行 (含 '\n') 即可取 path
						let mut req = Vec::new();
						let mut b = [0u8; 512];
						while !req.contains(&b'\n') {
							let n = sock.read(&mut b).unwrap_or(0);
							if n == 0 {
								break;
							}
							req.extend_from_slice(&b[..n]);
						}
						let text = String::from_utf8_lossy(&req).to_string();
						let path = text
							.lines()
							.next()
							.unwrap_or("")
							.split(' ')
							.nth(1)
							.unwrap_or("")
							.to_string();
						let body = if path == "/map_info.json" {
							MAP_INFO_MOCK
						} else if path == "/map_obj.json" {
							CALC_R1
						} else if path == "/__probe" {
							"__VOIDMEI_PROBE__"
						} else {
							HUDMSG_MULTI
						};
						let resp = format!(
							"HTTP/1.1 200 OK\r\nServer: mock\r\nDate: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
							body.len(),
							body
						);
						let _ = sock.write_all(resp.as_bytes());
						drop(sock); // close → 客户端 readLine 到 EOF
					}
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
						std::thread::sleep(Duration::from_millis(5));
					}
					Err(_) => break,
				}
			}
		});

		// 自证: 探针连接必须由本监听器应答 — Windows 下占用者开 SO_REUSEADDR 时
		// 本 bind 也成功但连接可能仍达占用者 (谁收连接无保证), 探针不过即跳过
		{
			let mut probe = match TcpStream::connect("127.0.0.1:8111") {
				Ok(s) => s,
				Err(_) => {
					stop.store(true, Ordering::SeqCst);
					server.join().unwrap();
					eprintln!("跳过: 8111 探针连接失败 (e2e 先例)");
					return;
				}
			};
			let _ = probe.write_all(b"GET /__probe HTTP/1.1\r\n\r\n");
			// 外部占用者若不关连接, read_to_end 会挂死 — 读超时兜底
			let _ = probe.set_read_timeout(Some(Duration::from_millis(2000)));
			let mut buf = Vec::new();
			let _ = probe.read_to_end(&mut buf);
			if !String::from_utf8_lossy(&buf).contains("__VOIDMEI_PROBE__") {
				stop.store(true, Ordering::SeqCst);
				server.join().unwrap();
				eprintln!("跳过: 8111 连接由外部占用者应答 (e2e 先例)");
				return;
			}
		}

		// init: 拉到 map_info (oracle: cmapmaxsize_x = -30000)
		let mut os = OtherService::new("\n");
		os.init(Arc::new(FixedSink { evt: 0, dmg: 0 }));
		assert_eq!(os.mapi.cmapmaxsize_x, -30000.0);
		assert!(os.s_map_info.is_some());

		let is_run = os.is_run.clone();
		let last_dmg = os.last_dmg.clone();
		let handle = std::thread::spawn(move || {
			os.run();
			os // 归还实例供断言 (Java: Controller 持 O 引用可随时读)
		});
		// 覆盖 ≥2 轮 (500ms/轮)
		std::thread::sleep(Duration::from_millis(1400));
		assert_eq!(last_dmg.load(Ordering::SeqCst), 222); // hudmsg 末对象 id
		is_run.store(false, Ordering::SeqCst); // close() 语义
		let os = handle.join().unwrap();
		// hudmsg msg 含 "热" → isOverheat = true (Lang.oSkeyWord1 = "热")
		assert!(os.is_overheat);
		// cmapmaxsize=-30000 → distance≈4242 > dislmt(1200) → 无计数
		assert_eq!(os.enemycount, 0);
		assert_eq!(os.friendcount, 0);
		// p_x/p_y 已同步 slc 坐标 (f32 拓宽)
		assert_eq!(os.p_x, 0.6f32 as f64);
		assert_eq!(os.p_y, 0.6f32 as f64);

		stop.store(true, Ordering::SeqCst);
		server.join().unwrap();
	}
