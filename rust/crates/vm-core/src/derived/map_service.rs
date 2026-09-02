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

use crate::base::exception_helper;
use crate::lang::Lang;
use crate::base::logger;
use crate::telemetry::parser::{HudMsg, MapInfo, MapObj};
use crate::base::java_compat::current_time_millis;

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
		temp = 12.0 + angle / 30.0;
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
		Ok(result)
	}

	/// 对应 Java `public void init(Controller c)`。
	pub fn init(&mut self, xc: Arc<dyn ControllerMsgIdSource + Send + Sync>) {
		self.is_run.store(true, Ordering::SeqCst);
		self.xc = Some(xc);
		self.p_x = 0.0;
		self.p_y = 0.0;
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
			// 运行标志极性睡眠 (is_run 是 true=运行, 直传 stop 语义的
			// sleep_quietly 会立即返回 → 热自旋; 备案收口修复), 睡眠中
			// is_run 翻 false 提前返回, while 重查退出
			exception_helper::sleep_while_run(&self.is_run, 500);
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

// ============================================================================
// 测试 (期望值 = Java 8 oracle 手算/Python 复算; f32 解析拓宽已计入)
// ============================================================================

#[cfg(test)]
mod tests;
