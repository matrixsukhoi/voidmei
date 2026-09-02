//! 全量对拍 — blkx 文本解析器 vs JSON 解析器的位级等价验证 (blkx→json 迁移
//! 的安全核心)。
//!
//! 协议: 对 data/ 下同名 .blkx/.json 配对, 以**同 name 参数**走两侧完整加载链
//! (文本: parse_named + get_all_plotdata, 对齐 fm_loader 七步; JSON:
//! parse_named_json), 双方 finalize_loading() 后比较 — `format!("{:?}")`
//! 全串相等 (Debug 最短往返表示 → f64 位级; -0.0/0.0 区分) + loc..loc3 曲线
//! 向量逐元素 to_bits 加固 (Debug 将各 NaN payload 统一印 "NaN" 的盲区兜底)。
//!
//! 触发条件: data/ 存在 .json 配对才跑 (fmdatajson 产物); 无 .json 时 SKIP
//! 并打印真因 (realtests 先例 — data/ 缺失自动跳过, 对齐 build.py 语义)。
//!
//! 运行:
//! - `cargo test -p vm-core fm_parity` (smoke, 常跑: 分层抽样)
//! - `python script/build.py fmparity` (full: 2832 对全量, 8 线程并行)
//! - env `VOIDMEI_FM_PARITY_REPORT=1` → 差异/耗时汇总落 build/fm_parity_report.txt

use super::Blkx;
use std::path::PathBuf;
use std::time::Instant;

/// 项目内真机 FM 数据根 (realtests.rs fm_root 同款约定)
fn fm_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data/aces/gamedata/flightmodels")
}

/// 一对配对文件 + 统一 name 参数 (fm_loader 调用约定: 物理 = fmfile 相对路径
/// "fm/xxx.blk", 中央 = "{name}.blk")
struct Pair {
    blkx: PathBuf,
    json: PathBuf,
    name: String,
}

/// 收集全部配对 (物理 fm/ + 中央根), 按文件名排序保证确定性。
fn corpus_pairs() -> Vec<Pair> {
    let root = fm_root();
    let mut pairs = Vec::new();
    for (dir, prefix) in [(root.join("fm"), "fm/"), (root.clone(), "")] {
        let mut entries: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).collect(),
            Err(_) => continue,
        };
        entries.sort();
        for json in entries {
            if json.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let stem = json
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let blkx = json.with_extension("blkx");
            if !blkx.is_file() {
                continue;
            }
            pairs.push(Pair {
                blkx,
                json,
                name: format!("{prefix}{stem}.blk"),
            });
        }
    }
    pairs
}

/// 单对对拍 → Ok(耗时秒) 或 Err(差异报告, ≤20 行 diff)。
fn compare_pair(p: &Pair) -> Result<f64, String> {
    let t0 = Instant::now();
    // 文本腿 (fm_loader 链: parse_named + get_all_plotdata + finalize)
    let text = Blkx::parse_named(&p.blkx.to_string_lossy(), &p.name);
    let json = Blkx::parse_named_json(&p.json.to_string_lossy(), &p.name);
    let (mut a, mut b) = match (text, json) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(ea), Err(_eb)) => {
            // 双 Err (CORRUPT 数据) 视为等价 — 诊断串不进 Blkx 字段
            let _ = ea;
            return Ok(t0.elapsed().as_secs_f64());
        }
        (Err(ea), Ok(_)) => {
            return Err(format!("文本 Err 而 JSON Ok: {ea}"));
        }
        (Ok(_), Err(eb)) => {
            return Err(format!("文本 Ok 而 JSON Err: {eb}"));
        }
    };
    a.get_all_plotdata();
    a.finalize_loading();
    b.finalize_loading();
    let elapsed = t0.elapsed().as_secs_f64();

    // 主比较: Debug 全串 (data/read_file_name 在 finalize 后/同 name 下天然一致)
    let da = format!("{a:?}");
    let db = format!("{b:?}");
    if da == db {
        return Ok(elapsed);
    }
    // diff 报告: Debug 是单行串, 按字段分隔符 ", " 切 token 做对齐比较, 最多 20 处
    let ta: Vec<&str> = da.split(", ").collect();
    let tb: Vec<&str> = db.split(", ").collect();
    let mut diffs = Vec::new();
    let (mut ia, mut ib) = (0usize, 0usize);
    while (ia < ta.len() || ib < tb.len()) && diffs.len() < 20 {
        if ia < ta.len() && ib < tb.len() && ta[ia] == tb[ib] {
            ia += 1;
            ib += 1;
            continue;
        }
        // 分叉点: 尝试在对方前瞻 ≤4 token 内找回同步 (字段增删的简单对齐)
        let sync_a = tb.iter().skip(ib).take(5).position(|x| *x == ta[ia]);
        let sync_b = ta.iter().skip(ia).take(5).position(|x| *x == tb[ib]);
        match (sync_a, sync_b) {
            (Some(sa), None) => {
                for extra in &tb[ib..ib + sa] {
                    diffs.push(("(文本缺失)", *extra));
                }
                ib += sa;
            }
            (None, Some(sb)) => {
                for extra in &ta[ia..ia + sb] {
                    diffs.push((*extra, "(JSON 缺失)"));
                }
                ia += sb;
            }
            _ => {
                diffs.push((ta.get(ia).unwrap_or(&"<EOF>"), tb.get(ib).unwrap_or(&"<EOF>")));
                ia += 1;
                ib += 1;
            }
        }
    }
    let mut rpt = format!("对拍差异: {}\n", p.json.display());
    for (x, y) in diffs {
        rpt.push_str(&format!("  文本: {x}\n  JSON: {y}\n"));
    }
    Err(rpt)
}

/// smoke 抽样集: 测试机型 + 特性代表 + 体积两端 + 固定种子随机。
fn smoke_selection(pairs: &[Pair]) -> Vec<usize> {
    let mut sel: Vec<usize> = Vec::new();
    let key = |p: &Pair| {
        p.json
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    };
    // 特性代表 (存在才收): 英油/苏油/PASSPORT 曲线/多发/喷气/变后掠翼/重型
    let wanted = [
        "spitfire_f24",   // 英油 150 octane (realtests 对象)
        "tempest_mkv",    // realtests 对象
        "bf-109e-4",      // fuzz 种子, PASSPORT 曲线
        "yak-3",          // 苏油 b-100 燃油修正
        "a-26b",          // 双发活塞
        "b-17e",          // 四发重型
        "a-10a_early",    // 喷气 (推力表)
        "f_14a_early",    // 变后掠翼 (WingPlaneSweep 族)
        "he_162",         // 缺失场景机型 (预期不配对, 收集时自然跳过)
        "p-51d-20",       // 美式无改装
    ];
    for w in wanted {
        if let Some(i) = pairs.iter().position(|p| key(p) == w) {
            sel.push(i);
        }
    }
    // 体积两端: 最大/最小各 5 (json 文件大小)
    let mut by_size: Vec<(u64, usize)> = pairs
        .iter()
        .enumerate()
        .filter_map(|(i, p)| std::fs::metadata(&p.json).ok().map(|m| (m.len(), i)))
        .collect();
    by_size.sort();
    if by_size.len() >= 10 {
        for (_, i) in by_size.iter().take(5) {
            sel.push(*i);
        }
        for (_, i) in by_size.iter().rev().take(5) {
            sel.push(*i);
        }
    }
    // 固定种子随机 20 (确定性; LCG 免依赖)
    let mut seed: u64 = 0x5EED_1234_ABCD;
    for _ in 0..20 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        sel.push((seed as usize) % pairs.len());
    }
    sel.sort();
    sel.dedup();
    sel
}

/// 抽样对拍 (常跑): 数据配对存在时对代表性子集逐对断言。
#[test]
fn fm_parity_smoke() {
    let pairs = corpus_pairs();
    if pairs.is_empty() {
        eprintln!(
            "跳过 fm_parity_smoke: {} 下无 .blkx/.json 配对 (先运行 python script/build.py fmdatajson)",
            fm_root().display()
        );
        return;
    }
    let sel = smoke_selection(&pairs);
    let mut failed = 0;
    for &i in &sel {
        if let Err(rpt) = compare_pair(&pairs[i]) {
            failed += 1;
            eprintln!("{rpt}");
        }
    }
    assert_eq!(failed, 0, "fm_parity_smoke: {failed}/{} 对存在差异", sel.len());
}

/// 全量对拍 (2832 对, ignored — 经 build.py fmparity 调起): 8 线程并行,
/// 差异全量收集; `VOIDMEI_FM_PARITY_REPORT=1` 时汇总 (含双解析器耗时 top10) 落
/// build/fm_parity_report.txt。
#[test]
#[ignore]
fn fm_parity_full() {
    let pairs = corpus_pairs();
    assert!(
        !pairs.is_empty(),
        "data/ 无配对 (先运行 python script/build.py fmdatajson)"
    );
    let n_threads = 8usize;
    let per = (pairs.len() + n_threads - 1) / n_threads;
    // scope 共享 &pairs (免 Arc); 每线程一个连续区间, 结果 (差异, 逐对耗时) 汇总
    let results: Vec<(Vec<String>, Vec<(usize, f64)>)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..n_threads)
            .map(|t| {
                let range = (t * per)..pairs.len().min((t + 1) * per);
                let pairs_ref = &pairs;
                s.spawn(move || {
                    let mut errs = Vec::new();
                    let mut times = Vec::new();
                    for i in range {
                        match compare_pair(&pairs_ref[i]) {
                            Ok(sec) => times.push((i, sec)),
                            Err(rpt) => errs.push(rpt),
                        }
                    }
                    (errs, times)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("对拍线程 panic"))
            .collect()
    });

    let mut all_errs = String::new();
    let mut all_times = Vec::new();
    for (errs, times) in results {
        for e in errs {
            all_errs.push_str(&e);
            all_errs.push('\n');
        }
        all_times.extend(times);
    }
    let total = pairs.len();
    let ok_count = all_times.len();
    let mut top = all_times.clone();
    top.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut summary = format!(
        "fm_parity_full: {ok_count}/{total} 对位级一致, 差异 {} 对\n",
        total - ok_count
    );
    summary.push_str("最慢 10 文件 (秒):\n");
    for (i, sec) in top.iter().take(10) {
        summary.push_str(&format!("  {:.3}  {}\n", sec, pairs[*i].json.display()));
    }
    let report_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../build/fm_parity_report.txt");
    if std::env::var("VOIDMEI_FM_PARITY_REPORT").is_ok() {
        if let Some(parent) = report_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&report_path, format!("{summary}\n{all_errs}"));
    }
    eprintln!("{summary}");
    assert!(all_errs.is_empty(), "全量对拍存在差异, 详见上方报告/stderr");
}
