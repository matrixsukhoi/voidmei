//! 对应 Java: `src/prog/OtherService.java` (一比一翻译, B 类: 线程 + 8111 轮询)
//!
//! 地图/消息慢速轮询服务 (500ms 一轮, Application.debug 分支专用,
//! Controller.changeS3:228-233 创建): 拉取 /map_obj.json 与 /hudmsg,
//! 计算选择目标距离/速度/AOT/AZI 与周围敌友机数, 维护过热标志。
//!
//! PORT 头注记 (跨文件问题, §6 只标注不越文件修):
//! 1. `Controller xc` (后续批次未翻译): OtherService 的全部使用面 = init 读
//!    `xc.lastEvt`/`xc.lastDmg` → 最小 trait [`ControllerMsgIdSource`] 注入
//!    (flight_log 的 ControllerLogSink 先例)。
//! 2. Java 反向通道: Controller(S4toS1) 跨线程读 `O.lastEvt`/`O.lastDmg` 并调
//!    `O.close()` —— run 线程独占实例 (本 struct move 进线程, flight_log run
//!    先例), 故 last_evt/last_dmg/is_run 三字段用 `Arc` 原子共享, Controller
//!    持克隆即 Java 持引用的等价面。Java 侧 lastDmg 为非 volatile int 的
//!    数据竞争读写, 原子化后数值语义不变。**close() 本体在此设计下对
//!    Controller 不可达** (实例已 move 进线程): Controller 批次落地时应持
//!    is_run 的 Arc 克隆 `store(false)` 表达 close 语义, 勿尝试保留
//!    OtherService 引用调 close(); 本方法保留仅为 Java 成员面保真。
//! 3. `Application.httpHeader` (static, C 类未翻译): sendGet 调用期读取;
//!    Rust 由 `new(http_header)` 注入快照 (http_helper 先例, 值源自
//!    Lang.httpHeader 缺省 "\n", 启动后不变, 构造期值 == 调用期值)。
//! 4. `Lang.oSkeyWord1/oSkeyWord2` 静态字段读取 → init 时取 `Lang::init_lang()`
//!    实例 (flight_log 先例)。等价性依赖 initLang 仅 Application.main 调用一次
//!    (全库无运行期重载调用点); 若后续引入语言热切换需回访此快照。
//!    另: Java 缺键时 oSkeyWord 为 null → `indexOf(null)` NPE 线程死亡; Rust
//!    静态表恒有缺省 "热"/"温" 正常执行 — lang 批次既定决策 (cur.properties
//!    实有该键, 生产不可达, 仅记录)。
//! 5. §2.13 (LIFETIMES 修正9: 本类标志实叫 isRun 非 doit): `volatile boolean
//!    isRun` → `Arc<AtomicBool>`; `Thread.sleep(500)`+InterruptedException→break
//!    → 停机标志分片睡眠, 提前返回后 while 重查退出。既定代价 (良性分歧):
//!    Java close() 后线程睡满 500ms 并执行完最后一整轮 (fetch+calculate+
//!    pX/pY 同步) 才在 while 重查退出; Rust 提前唤醒直接退出, 最后一轮被
//!    跳过。可观察面无差异 — Controller.S4toS1 先读走 lastEvt/lastDmg 再调
//!    close(), 其余字段无外部读者。
//! 6. Java String 字段默认 null (首轮 fetch 失败时 sMapObj/shudMsg 保持 null
//!    传入 update → NPE, run 无 catch → 线程死亡) → `Option<String>` + 消费点
//!    unwrap (panic ↔ NPE, 线程死亡语义保真)。
//! 7. calculate 原有算式怪癖**保真不修**: enemyspeed 的 y 项两因子分别乘
//!    cmapmaxsizeX 与 cmapmaxsizeY (Java 原样); mov 循环 sdistance 的 y 项
//!    第二因子是 `(mov.y - mov.y)` 恒 0 (Java 原样)。

use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::exception_helper;
use crate::lang::Lang;
use crate::logger;
use crate::parser::{HudMsg, MapInfo, MapObj};

/// Controller 的最小消息游标读取面 (PORT 注记 1: 后续批次 Controller 落地时
/// 由其实现, 对应 Java `xc.lastEvt`/`xc.lastDmg` 两个字段的读取)。
pub trait ControllerMsgIdSource: Send + Sync {
    fn last_evt(&self) -> i32;
    fn last_dmg(&self) -> i32;
}

pub struct OtherService {
	s_map_info: Option<String>,
	s_map_obj: Option<String>,
	shud_msg: Option<String>,
	/// PORT: Java `Controller xc` — init 后无读取点, 字段保真保留
	xc: Option<Arc<dyn ControllerMsgIdSource>>,
	pub mapi: MapInfo,
	pub mapo: MapObj,

	pub distance: f64,
	pub enemyspeed: f64,
	pub aot: f64,
	pub azi: f64,

	pub enemycount: i32,
	pub friendcount: i32,

	pub dislmt: i32,
	p_x: f64,
	p_y: f64,
	speed_check_mili: i64,

	msg: HudMsg,
	/// PORT: Java 非 volatile int, 被 Controller 跨线程读 (模块头注记 2)
	pub last_evt: Arc<AtomicI32>,
	pub last_dmg: Arc<AtomicI32>,
	/// PORT: Java `public volatile boolean isRun` → Arc<AtomicBool> (§2.13)
	pub is_run: Arc<AtomicBool>,
	is_get_msg: bool,
	is_get_map_obj: bool,
	is_overheat: bool,

	/// PORT: Application.httpHeader 注入快照 (模块头注记 3)
	http_header: String,
	/// PORT: Lang 静态字段读取面 (模块头注记 4)
	lang: Lang,
}

impl OtherService {
	/// 对应 Java `new OtherService()` (默认构造): 数值 0/引用 null 的隐式
	/// 初始化显式化 (§2.10)。
	/// PORT: http_header 参数为注记 3 的注入点。
	pub fn new(http_header: &str) -> Self {
		OtherService {
			s_map_info: None,
			s_map_obj: None,
			shud_msg: None,
			xc: None,
			mapi: MapInfo::new(),
			mapo: MapObj::new(),
			distance: 0.0,
			enemyspeed: 0.0,
			aot: 0.0,
			azi: 0.0,
			enemycount: 0,
			friendcount: 0,
			dislmt: 0,
			p_x: 0.0,
			p_y: 0.0,
			speed_check_mili: 0,
			msg: HudMsg::new(),
			last_evt: Arc::new(AtomicI32::new(0)),
			last_dmg: Arc::new(AtomicI32::new(0)),
			is_run: Arc::new(AtomicBool::new(false)),
			is_get_msg: false,
			is_get_map_obj: false,
			is_overheat: false,
			http_header: http_header.to_string(),
			lang: Lang::default(),
		}
	}

	/// 对应 Java `public double angleToclock(double angle)` (无 this 使用 →
	/// 关联函数, http_helper 先例)。
	pub fn angle_toclock(angle: f64) -> f64 {
		let mut temp: f64;
		// PORT §2.12: Java `12 + angle / 30.0f` — float 字面量提升 double,
		// 30.0f32 as f64 == 30.0 精确
		temp = 12.0 + angle / 30.0f32 as f64;
		if temp >= 12.0 {
			temp -= 12.0;
		}
		temp
	}

	/// 对应 Java `public double dxdyToangle(double dx, double dy)`。
	/// 注意: Java 的三个 if 顺序执行且 0 同时满足 >=0 与 <=0, 边界输入会
	/// 连续叠加 (如 dy=0,dx=-0 → 360) — 逐字保真。
	pub fn dxdy_to_angle(dx: f64, dy: f64) -> f64 {
		let mut tems: f64;
		// Java: Math.atan(dy / dx) * 180 / Math.PI (左结合: (atan*180)/PI)
		tems = (dy / dx).atan() * 180.0 / std::f64::consts::PI;
		if dy >= 0.0 && dx <= 0.0 {
			tems += 180.0;
		}
		if dy <= 0.0 && dx <= 0.0 {
			tems += 180.0;
		}
		if dy <= 0.0 && dx >= 0.0 {
			tems += 360.0;
		}
		tems
	}

	/// 对应 Java `public String sendGet(String host, int port, String path)`。
	/// PORT: port int → u16 (http_helper 先例: 非法端口在 InetSocketAddress
	/// 构造即抛, 类型层面排除该值域)。
	pub fn send_get(&self, host: &str, port: u16, path: &str) -> io::Result<String> {
		let dest: SocketAddr = format!("{}:{}", host, port)
			.parse()
			.map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
		// PORT: 无 connect/read 超时 = Java `new Socket()` 无超时 + 阻塞 readLine
		// 读到 EOF 的逐字保真; 对端 accept 后既不发包也不关连接时本线程永久
		// 挂死 (close 的 is_run 永等不到 while 重查) — Java 原有隐患忠实搬运,
		// e2e/Controller 批次保持知晓, 本处不修
		let mut socket = TcpStream::connect(dest)?;
		{
			let mut buffered_writer = io::BufWriter::new(&mut socket);
			buffered_writer.write_all(format!("GET {} HTTP/1.1\r\n", path).as_bytes())?;
			buffered_writer.write_all(format!("Host: {}\r\n", host).as_bytes())?;
			buffered_writer.write_all(self.http_header.as_bytes())?;
			buffered_writer.write_all(b"\r\n")?;
			buffered_writer.flush()?;
		}

		let mut buffered_reader = BufReader::new(socket);

		// bufferedReader.ready();
		// PORT: Java ready() 调用结果被丢弃 (无副作用), 保留为注释

		// 跳过响应头 6 行 (状态行 + 5 个头字段)
		for _ in 0..6 {
			let mut line = String::new();
			if buffered_reader.read_line(&mut line)? == 0 {
				// PORT: Java readLine 返回 null 时无循环体执行, EOF 提前结束等价
				break;
			}
		}

		let mut content_buf = String::new();
		loop {
			let mut raw = Vec::new();
			if buffered_reader.read_until(b'\n', &mut raw)? == 0 {
				break;
			}
			let mut line = String::from_utf8_lossy(&raw).to_string();
			// Java BufferedReader.readLine 剥离 \r\n / \n 行终止符后 append。
			// PORT: 已知差异 (不可达域存档): Java 把孤立的 \r (非行尾、后无 \n)
			// 也视为行终止符拆两行并消费; 本复刻 read_until(b'\n') 仅剥行尾 CR,
			// 行中间的裸 \r 会留在内容里。HTTP 头 + JSON body 无孤立 \r, 不可达
			if line.ends_with('\n') {
				line.pop();
			}
			if line.ends_with('\r') {
				line.pop();
			}
			content_buf.push_str(&line);
		}
		let result = content_buf;
		// Java: bufferedReader/bufferedWriter/socket 显式 close → Rust Drop 等价
		Ok(result)
	}

	/// 对应 Java `public void init(Controller c)`。
	pub fn init(&mut self, xc: Arc<dyn ControllerMsgIdSource + Send + Sync>) {
		self.is_run.store(true, Ordering::SeqCst);
		self.xc = Some(xc);
		self.p_x = 0.0;
		self.p_y = 0.0;
		// Java: lastEvt/lastDmg 先置 0 再被 xc 值覆盖 (死存储, 保形保留)
		self.last_evt.store(0, Ordering::SeqCst);
		self.last_dmg.store(0, Ordering::SeqCst);
		self.last_evt
			.store(self.xc.as_ref().unwrap().last_evt(), Ordering::SeqCst);
		self.last_dmg
			.store(self.xc.as_ref().unwrap().last_dmg(), Ordering::SeqCst);
		self.is_get_msg = true;
		self.is_get_map_obj = true;
		self.is_overheat = false;
		//
		self.dislmt = 1200;
		self.speed_check_mili = current_time_millis();
		self.mapi = MapInfo::new();
		self.mapi.init();
		if self.is_get_map_obj {
			self.mapo = MapObj::new();
			self.mapo.init();
		}
		if self.is_get_msg {
			self.msg = HudMsg::new();
			self.msg.init();
		}
		self.lang = Lang::init_lang();

		// 初始化地图设置，计算尺寸
		match self.send_get("127.0.0.1", 8111, "/map_info.json") {
			Ok(v) => {
				self.s_map_info = Some(v);
				self.mapi.update(self.s_map_info.as_deref().unwrap());
			}
			Err(e) => {
				logger::debug("OtherService", &format!("Failed to get map info: {}", e));
			}
		}
	}

	/// 对应 Java `public void calculate()`。
	// clippy::eq_op: mov.y - mov.y 是 Java 原样笔误的机械保真 (PORT 注在表达式处)
	#[allow(clippy::eq_op)]
	pub fn calculate(&mut self) {
		// 计算选择目标的水平相对距离及速度及AOT
		let pys: f64;
		let eys: f64;
		// PORT §2.6: Java `mapo.slc.type != ""` 是引用比较 (update 无选择时写入
		// interned "" 字面量, 有选择时为新 substring 对象) — 值比较在可达路径
		// 等价; None (未 init) 与 Java null 同走进入分支
		if self.mapo.slc.r#type.as_deref() != Some("") {
			self.distance = ((self.mapo.slc.x - self.mapo.pla.x)
				* (self.mapo.slc.x - self.mapo.pla.x)
				* self.mapi.cmapmaxsize_x
				* self.mapi.cmapmaxsize_x
				+ (self.mapo.slc.y - self.mapo.pla.y)
					* (self.mapo.slc.y - self.mapo.pla.y)
					* self.mapi.cmapmaxsize_y
					* self.mapi.cmapmaxsize_y)
				.sqrt();
			// Application.debugPrint(distance);

			if self.mapo.slc.dx != 0.0 && self.distance < self.dislmt as f64 {
				self.enemycount += 1;
			}

			// PORT: y 项两因子分别乘 cmapmaxsizeX 与 cmapmaxsizeY — Java 原样
			// 的不对称 (不修, 见模块头注记 7)
			self.enemyspeed = (((self.mapo.slc.x - self.p_x) * self.mapi.cmapmaxsize_x)
				* ((self.mapo.slc.x - self.p_x) * self.mapi.cmapmaxsize_x)
				+ ((self.mapo.slc.y - self.p_y) * self.mapi.cmapmaxsize_x)
					* ((self.mapo.slc.y - self.p_y) * self.mapi.cmapmaxsize_y))
				.sqrt()
				* 1000.0 / (current_time_millis() - self.speed_check_mili) as f64;
			self.speed_check_mili = current_time_millis();
			pys = Self::dxdy_to_angle(self.mapo.pla.dx, self.mapo.pla.dy);
			eys = Self::dxdy_to_angle(self.mapo.slc.dx, self.mapo.slc.dy);
			self.aot = (pys - eys).abs();
			if self.aot > 180.0 {
				self.aot = 360.0 - self.aot;
			}
			// Application.debugPrint(enemyspeed*3.6 );
			self.azi = Self::angle_toclock(
				Self::dxdy_to_angle(
					self.mapo.slc.x - self.mapo.pla.x,
					self.mapo.slc.y - self.mapo.pla.y,
				) - pys,
			);
			// Application.debugPrint(mapo.slc.dx);
			// Application.debugPrint(enemycount);
		}

		// 统计周围敌机数和友机数
		for i in 0..self.mapo.movcur {
			// PORT: y 项第二因子是 `(mov.y - mov.y)` 恒 0 — Java 原样 (不修)
			let sdistance = ((self.mapo.mov[i as usize].x - self.mapo.pla.x)
				* (self.mapo.mov[i as usize].x - self.mapo.pla.x)
				* self.mapi.cmapmaxsize_x
				* self.mapi.cmapmaxsize_x
				+ (self.mapo.mov[i as usize].y - self.mapo.pla.y)
					* (self.mapo.mov[i as usize].y - self.mapo.mov[i as usize].y)
					// PORT: 上行 mov.y - mov.y == 0 为 Java 原样 (疑似上游笔误, 机械保真; clippy::eq_op 已在函数级 allow)
					* self.mapi.cmapmaxsize_y
					* self.mapi.cmapmaxsize_y)
				.sqrt();
			if sdistance < self.dislmt as f64 && sdistance < self.mapo.mov[i as usize].distance {
				// PORT: Java colorg null → NPE ↔ unwrap panic; [u8;4] = [r,g,b,a]
				let colorg = self.mapo.mov[i as usize].colorg.unwrap();
				if colorg[2] > 200 || colorg[1] > 200 {
					self.friendcount += 1;
					// Application.debugPrint((mapo.mov[i].type+"友军"+i+"距离"+sdistance));
				}
				if colorg[0] > 200 {
					self.enemycount += 1;
				}
			}
			self.mapo.mov[i as usize].distance = sdistance;
		}
		// Application.debugPrint("周围友机数" + friendcount + " 周围敌机数" + enemycount);
	}

	/// 对应 Java `public void close()`。
	/// PORT: 实例 move 进 run 线程后本方法对 Controller 不可达 (模块头注记 2) —
	/// Controller 批次落地时持 is_run Arc 克隆 store(false) 表达同一语义。
	pub fn close(&self) {
		self.is_run.store(false, Ordering::SeqCst);
	}

	/// 对应 Java `public void judgeOverheat()` — 空方法 (原注释保真)。
	pub fn judge_overheat(&mut self) {
		// Overheat detection logic removed - functionality not implemented
	}

	/// 对应 Java `public void run()` (Runnable)。
	/// PORT: 实例 move 进工作线程后调用 (Controller 持 is_run/last_evt/last_dmg
	/// 的 Arc 克隆, 模块头注记 2); Java 的 O1.interrupt() 无调用点, 唯一停机
	/// 路径 close() 即 is_run 置 false。
	pub fn run(&mut self) {
		while self.is_run.load(Ordering::SeqCst) {
			// 500毫秒执行一次
			// PORT §2.13: Thread.sleep(500) + InterruptedException→break ⇒
			// 停机标志分片睡眠, 提前返回后 while 重查 is_run 退出 (标志保持
			// 置位 = Java 恢复中断语义)
			exception_helper::sleep_quietly(&self.is_run, 500);
			// 取得地图数据
			// Application.debugPrint("正在处理地图数据");
			self.enemycount = 0;
			self.friendcount = 0;
			// PORT §2.7: Java 单个 try 覆盖两个 fetch、IOException 空 catch —
			// 首个失败即跳过第二个, 字段保留旧值 (None)
			let fetched: io::Result<()> = 'fetch: {
				if self.is_get_map_obj {
					match self.send_get("127.0.0.1", 8111, "/map_obj.json") {
						Ok(v) => self.s_map_obj = Some(v),
						Err(e) => break 'fetch Err(e),
					}
				}
				if self.is_get_msg {
					match self.send_get(
						"127.0.0.1",
						8111,
						&format!(
							"/hudmsg?lastEvt={}&lastDmg={}",
							self.last_evt.load(Ordering::SeqCst),
							self.last_dmg.load(Ordering::SeqCst)
						),
					) {
						Ok(v) => self.shud_msg = Some(v),
						Err(e) => break 'fetch Err(e),
					}
				}
				Ok(())
			};
			// Network error - game not running or API unavailable
			let _ = fetched; // PORT: Java 空 catch (吞异常继续)
			// Application.debugPrint(sMapObj);
			if self.is_get_map_obj {
				// PORT: Java null (首轮 fetch 失败) 传入 update → MapObj 内 NPE,
				// run 无 catch → 线程死亡; unwrap panic 等价
				self.mapo.update(self.s_map_obj.as_deref().unwrap());
			}
			if self.is_get_msg {
				let last_dmg =
					self.msg
						.update(self.shud_msg.as_deref().unwrap(), self.last_dmg.load(Ordering::SeqCst));
				self.last_dmg.store(last_dmg, Ordering::SeqCst);
				if self.msg.dmg.as_ref().unwrap().updated {
					// Application.debugPrint("过热检查" + msg.dmg.msg.indexOf("热") +
					// "过高检查" + msg.dmg.msg.indexOf("温"));
					// Java String.indexOf != -1 ↔ find().is_some()
					// (空串搜索两者均命中)
					if self
						.msg
						.dmg
						.as_ref()
						.unwrap()
						.msg
						.as_ref()
						.unwrap()
						.find(self.lang.o_skey_word1)
						.is_some()
						|| self
							.msg
							.dmg
							.as_ref()
							.unwrap()
							.msg
							.as_ref()
							.unwrap()
							.find(self.lang.o_skey_word2)
							.is_some()
					{
						self.is_overheat = true;
						// Application.debugPrint("检测到过热标志" + isOverheat);
					}
				} else {
					self.is_overheat = false;
					// Application.debugPrint("检测到不过热标志" + isOverheat);
				}
			}
			// 处理地图数据

			self.calculate();
			self.p_x = self.mapo.slc.x;
			self.p_y = self.mapo.slc.y;

			// 获得HUDMSG消息并通知玩家过热
			self.judge_overheat();
			// Application.debugPrint("otherService执行了");
		}
	}
}

/// Java `System.currentTimeMillis()` 的 crate 先例形态
/// (fm_manager.rs 同款): SystemTime → as_millis u128 → as i64 截断;
/// 时钟早于 epoch 时 Java 可得负值而 duration_since 报错 → 取 0。
/// 时间戳差值域 (epoch 毫秒) 远离 i64 溢出, 普通减法即可 (§2.2 无涉)。
fn current_time_millis() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_millis() as i64)
		.unwrap_or(0)
}

// ============================================================================
// 测试 (期望值 = Java 8 oracle 手算/Python 复算; f32 解析拓宽已计入)
// ============================================================================

#[cfg(test)]
mod tests {
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
		assert!(!os.is_run.load(Ordering::SeqCst)); // Java 默认 false
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
		// Java: 线程死亡时 isRun 仍为 true (close 未被调用)
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
}
