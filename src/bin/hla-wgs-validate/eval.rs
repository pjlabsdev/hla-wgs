//! Evaluations of hla-wgs results: against the assembly truth (eval23, eval6, eval8, eval8-exons),
//! against 1000 Genomes laboratory types (eval24), within the GIAB trios (trio23) and on simulated
//! reads (sim-eval).
use crate::common::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Write;

/// truth table: (sample, gene) -> haplotype -> (d_min, best names), haplotypes in file order
fn truth_table(path: &str) -> OMap<(String, String), OMap<String, (i64, Vec<String>)>> {
    let mut t: OMap<(String, String), OMap<String, (i64, Vec<String>)>> = OMap::default();
    for l in read(path).lines() {
        let r: Vec<&str> = l.split('\t').collect();
        if r.len() < 5 || r[2] == "_done" || r[3] == "-" {
            continue;
        }
        let d = r[3].parse().unwrap_or(0);
        t.entry((r[0].to_string(), r[2].to_string())).insert(r[1].to_string(), (d, r[4].split(',').map(String::from).collect()));
    }
    t
}

fn conf_lines(path: &str) -> Vec<Vec<String>> {
    read(path).lines().skip(1).map(|l| l.split('\t').map(String::from).collect()).collect()
}

pub fn eval23(dir: &str) {
    let t = truth_table(&env_or("T4OUT", "truth/truth4.tsv"));
    let samples: BTreeSet<&String> = t.keys.iter().map(|k| &k.0).collect();
    let mut st: OMap<String, Counter> = OMap::default();
    let mut bad: Vec<String> = vec![];
    let empty: OMap<String, (i64, Vec<String>)> = OMap::default();
    for s in samples {
        let cf = format!("{dir}/{s}.conf.tsv");
        if !exists(&cf) {
            continue;
        }
        let mut conf: OMap<String, Vec<String>> = OMap::default();
        for r in conf_lines(&cf) {
            conf.entry(r[0].clone()).push(r[2].clone());
        }
        for (g, c) in conf.iter() {
            let th = t.get(&(s.clone(), g.clone())).unwrap_or(&empty);
            let mut c = c.clone();
            if c.len() == 1 && th.len() == 2 {
                c = vec![c[0].clone(), c[0].clone()];
            }
            let mut hh: Vec<&String> = th.keys.iter().collect();
            hh.sort();
            if c.len() != hh.len() || c.is_empty() || c.iter().any(|x| x == "-") {
                continue;
            }
            for k in [2usize, 3] {
                let ok = |n: &str, h: &String| -> bool {
                    let names = &th.get(h).unwrap().1;
                    let deep: Vec<&String> = names.iter().filter(|x| nfields(x) >= k).collect();
                    deep.is_empty() || deep.iter().any(|x| cut(x, k) == cut(n, k))
                };
                let mut best: Vec<usize> = vec![];
                let mut bs = -1i64;
                for pm in perms(c.len()) {
                    let v = pm.iter().zip(&hh).filter(|(i, h)| ok(&c[**i], h)).count() as i64;
                    if v > bs {
                        bs = v;
                        best = pm;
                    }
                }
                for (i, h) in best.iter().zip(&hh) {
                    let e = st.entry(g.clone());
                    e.add(&format!("n{k}"), 1);
                    let o = ok(&c[*i], h);
                    e.add(&format!("ok{k}"), o as i64);
                    if !o {
                        let tn: Vec<String> = th.get(h).unwrap().1.iter().take(3).cloned().collect();
                        bad.push(format!("{k} {s} {g} {} {}", c[*i], pylist(&tn)));
                    }
                }
            }
        }
    }
    let mut out = String::from("gene\tn\tok2\tok3\n");
    let mut tot = Counter::default();
    for (g, c) in st.iter() {
        tot.update(c);
        out += &format!("{g}\t{}\t{}\t{}\n", c.get("n2"), c.get("ok2"), c.get("ok3"));
    }
    out += &format!("ALL\t{}\t{}\t{}\n", tot.get("n2"), tot.get("ok2"), tot.get("ok3"));
    for b in bad {
        out += &b;
        out.push('\n');
    }
    print!("{out}");
}

/// differences of an alignment: positions on the forward allele (1-based) with their kind
fn diffs(p: &Paf, q: &str) -> (i64, Vec<(i64, String)>) {
    let ql = q.len();
    let (qs, qe) = (p.qs, p.qe);
    let qq: String = if p.fwd { q[qs..qe].to_string() } else { revcomp(&q[qs..qe]) };
    let qb = qq.as_bytes();
    let mut pos = 0usize;
    let mut out = vec![];
    let cs = p.cs.as_bytes();
    let mut i = 0;
    while i < cs.len() {
        let op = cs[i];
        let mut j = i + 1;
        while j < cs.len() && (cs[j].is_ascii_digit() || cs[j].is_ascii_lowercase()) {
            j += 1;
        }
        let v = &p.cs[i + 1..j];
        i = j;
        if op == b':' {
            pos += v.parse::<usize>().unwrap();
            continue;
        }
        let lo = pos.saturating_sub(8);
        let hi = (pos + 8 + v.len()).min(qb.len());
        let ctxt: Vec<u8> = if lo < hi { qb[lo..hi].to_ascii_uppercase() } else { vec![] };
        let hp = longest_run(&ctxt);
        let strx = tandem(&ctxt);
        let pa = if p.fwd { (qs + pos + 1) as i64 } else { qe as i64 - pos as i64 };
        let kind = match op {
            b'*' => {
                pos += 1;
                "snv"
            }
            b'+' => {
                pos += v.len();
                "indel"
            }
            _ => "indel",
        };
        let suf = if kind == "indel" && hp >= 5 { "_hp" } else if kind == "indel" && strx { "_str" } else { "" };
        out.push((pa, format!("{kind}{suf}")));
    }
    ((qs + (ql - qe)) as i64, out)
}

/// longest run of one of A, C, G, T (1 when there is none, as Python's max(..., default=1))
fn longest_run(s: &[u8]) -> usize {
    let mut best = 0;
    let mut i = 0;
    while i < s.len() {
        if matches!(s[i], b'A' | b'C' | b'G' | b'T') {
            let mut j = i;
            while j < s.len() && s[j] == s[i] {
                j += 1;
            }
            best = best.max(j - i);
            i = j;
        } else {
            i += 1;
        }
    }
    if best == 0 { 1 } else { best }
}

/// a unit of 2, 3 or 4 characters repeated three times in a row anywhere
fn tandem(s: &[u8]) -> bool {
    for u in 2..=4 {
        for st in 0..s.len() {
            if st + 3 * u > s.len() {
                break;
            }
            if s[st..st + u] == s[st + u..st + 2 * u] && s[st..st + u] == s[st + 2 * u..st + 3 * u] {
                return true;
            }
        }
    }
    false
}

const G18: [&str; 18] = ["A", "B", "C", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1", "DRA", "E", "F", "G", "H", "K", "V"];
const G8: [&str; 8] = ["MICA", "MICB", "TAP1", "TAP2", "DMA", "DMB", "DOA", "DOB"];

struct Call {
    name: String,
    res: i64,
    det: HashSet<i64>,
    /// sites marked as possibly on the other allele (`:either`, novel.tsv "phase uncertain")
    either: HashSet<i64>,
    nn: i64,
    detail: String,
}

fn tuple_list(v: &[(i64, String)]) -> String {
    format!("[{}]", v.iter().map(|(p, t)| format!("({p}, {})", pyrepr(t))).collect::<Vec<_>>().join(", "))
}

fn int_list(v: &[i64]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

pub fn eval6(dir: &str, verbose: bool, eight: bool) {
    let db = db("db/hladb.json.gz");
    let mut seq: HashMap<String, String> = HashMap::new();
    for g in genes(&db) {
        for a in alleles(g) {
            seq.insert(s(a, "name").to_string(), own_seq(a).to_string());
        }
    }
    let dmin = truth_table(&env_or("T4OUT", "truth/truth4.tsv"));
    let samples: BTreeSet<&String> = dmin.keys.iter().map(|k| &k.0).collect();
    let mut st: HashMap<String, Counter> = HashMap::new();
    let mut ex: Vec<String> = vec![];
    let dump = std::env::var("DUMP").ok();
    let empty: OMap<String, (i64, Vec<String>)> = OMap::default();
    for smp in samples {
        let cf = format!("{dir}/{smp}.conf.tsv");
        if !exists(&cf) {
            continue;
        }
        let mut conf: OMap<String, Vec<Call>> = OMap::default();
        // the table lists at most 20 sites per allele; PREFIX.novel.tsv has all of them
        let nf = format!("{dir}/{smp}.novel.tsv");
        let mut novel: HashMap<(String, String), Vec<(i64, bool)>> = HashMap::new();
        if exists(&nf) {
            for l in read(&nf).lines().filter(|l| !l.starts_with('#') && !l.starts_with("gene\t")) {
                let f: Vec<&str> = l.split('\t').collect();
                if f.len() > 5 {
                    if let Ok(p) = f[4].parse::<i64>() {
                        let unsure = f.get(13).map_or(false, |x| x.starts_with("phase uncertain"));
                        novel.entry((f[0].to_string(), f[1].to_string())).or_default().push((p, unsure));
                    }
                }
            }
        }
        for r in conf_lines(&cf) {
            let mut det = HashSet::new();
            let mut either = HashSet::new();
            let nn = if r.len() > 11 && r[11] != "-" { r[11].parse().unwrap() } else { 0 };
            if r.len() > 12 && r[12] != "-" {
                let src = if r[12].starts_with("known:") { "" } else { r[12].as_str() };
                for x in src.split(',') {
                    if !x.is_empty() && !x.contains("blind") && !x.contains("unphased") && !x.contains("repunverified") {
                        let p = x.split(':').next().unwrap().parse::<i64>().unwrap();
                        det.insert(p);
                        if x.ends_with(":either") {
                            either.insert(p);
                        }
                    }
                }
                let listed = src.split(',').filter(|x| !x.is_empty()).count() as i64;
                if !src.is_empty() && nn > listed {
                    let shown: HashSet<i64> = src.split(',').filter_map(|x| x.split(':').next()?.parse().ok()).collect();
                    for (p, unsure) in novel.get(&(r[0].clone(), r[1].clone())).into_iter().flatten() {
                        if !shown.contains(p) {
                            det.insert(*p);
                            if *unsure {
                                either.insert(*p);
                            }
                        }
                    }
                }
            }
            conf.entry(r[0].clone()).push(Call { name: r[2].clone(), res: r[4].parse().unwrap(), det, either, nn, detail: if r.len() > 12 { r[12].clone() } else { "-".into() } });
        }
        let names: BTreeSet<String> = conf.iter().flat_map(|(_, v)| v.iter().map(|x| x.name.clone())).collect();
        let mut hits: HashMap<(String, String), (i64, Vec<(i64, String)>)> = HashMap::new();
        for h in ["h1", "h2", "mat", "pat"] {
            let fa = format!("truth/{smp}.{h}.mhc.fa");
            if !exists(&fa) {
                continue;
            }
            let ix = index_fasta(&fa, crate::aln::Opts::asm5());
            for n in &names {
                let q = &seq[n];
                for p in map_primary(&ix, q, n) {
                    let (clip, dl) = diffs(&p, q);
                    let d = p.nm as i64 + clip;
                    let k = (h.to_string(), n.clone());
                    if hits.get(&k).map_or(true, |x| d < x.0) {
                        hits.insert(k, (d, dl));
                    }
                }
            }
        }
        let hs: BTreeSet<String> = hits.keys().map(|k| k.0.clone()).collect();
        let none = (999i64, vec![]);
        for (g, c0) in conf.iter() {
            let th = dmin.get(&(smp.clone(), g.clone())).unwrap_or(&empty);
            let idx: Vec<usize> = if c0.len() == 1 && th.len() == 2 { vec![0, 0] } else { (0..c0.len()).collect() };
            let hh: Vec<&String> = hs.iter().filter(|h| th.contains(h)).collect();
            if idx.len() != hh.len() || idx.is_empty() {
                st.entry(g.clone()).or_default().add("skip", 1);
                continue;
            }
            let hit = |h: &String, n: &String| hits.get(&(h.clone(), n.clone())).unwrap_or(&none);
            let mut best: Vec<usize> = vec![];
            let mut bk: Option<(i64, i64, i64)> = None;
            for pm in perms(idx.len()) {
                let mut k0 = 0;
                let mut k1 = 0;
                let mut k2 = 0;
                for (pi, h) in pm.iter().zip(&hh) {
                    let c = &c0[idx[*pi]];
                    let (d, dl) = hit(h, &c.name);
                    k0 += d;
                    k1 -= c.det.iter().filter(|p| dl.iter().any(|(q, _)| (**p - q).abs() <= 2)).count() as i64;
                    k2 += (c.res >= 4 && *d > 0) as i64;
                }
                if bk.map_or(true, |b| (k0, k1, k2) < b) {
                    bk = Some((k0, k1, k2));
                    best = pm;
                }
            }
            for (pi, h) in best.iter().zip(&hh) {
                let c = &c0[idx[*pi]];
                let (dc, dl) = hit(h, &c.name);
                let (dc, dm) = (*dc, th.get(h).unwrap().0);
                let k = st.entry(g.clone()).or_default();
                k.add("n", 1);
                if let Some(f) = &dump {
                    let mut o = std::fs::OpenOptions::new().create(true).append(true).open(f).unwrap();
                    let dls: Vec<String> = dl.iter().take(8).map(|(p, t)| format!("{p}:{t}")).collect();
                    writeln!(o, "{smp}\t{g}\t{}\t{}\t{dc}\t{dm}\t{}\t{}\t{}", c.name, c.res, c.nn, c.detail, dls.join(";")).unwrap();
                }
                let exact = dc == 0;
                let nearest = dc <= dm;
                k.add("exact", exact as i64);
                k.add("nearest", nearest as i64);
                if dm == 0 {
                    k.add("known", 1);
                    k.add("known_ok", exact as i64);
                } else {
                    k.add("new", 1);
                    k.add("new_nearest", nearest as i64);
                }
                let safe = if eight { c.res >= nfields(&c.name) as i64 } else { c.res >= 4 };
                if safe {
                    k.add("safe4", 1);
                    k.add("safe4_exact", exact as i64);
                    if !exact && c.nn == 0 {
                        k.add("safe4_wrong_noflag", 1);
                    }
                    if !exact && (dc > dm || dl.iter().any(|(_, t)| t == "snv" || t == "indel") || dl.is_empty()) {
                        k.add("safe4_wrong_real", 1);
                    }
                }
                let true_snv: HashSet<i64> = dl.iter().filter(|(_, t)| t == "snv").map(|x| x.0).collect();
                // a site marked "either" is true when the other haplotype carries it
                let other: Vec<(i64, String)> = hh.iter().filter(|x| **x != *h).map(|x| hit(x, &c.name).1.clone()).next().unwrap_or_default();
                let tp = c.det.iter().filter(|p| dl.iter().any(|(q, _)| (**p - q).abs() <= 2) || (c.either.contains(p) && other.iter().any(|(q, _)| (**p - q).abs() <= 2))).count() as i64;
                k.add("site_det", c.det.len() as i64);
                k.add("site_det_true", tp);
                k.add("site_snv", true_snv.len() as i64);
                k.add("site_snv_found", true_snv.iter().filter(|q| c.det.iter().any(|p| (p - **q).abs() <= 2)).count() as i64);
                if exact && c.nn > 0 {
                    k.add("flag_on_exact", 1);
                }
                if !exact && c.nn > 0 {
                    k.add("flag_on_inexact", 1);
                }
                if verbose && ((safe && !exact) || (exact && c.nn > 0) || (dm == 0 && !exact)) {
                    let tn: Vec<String> = th.get(h).unwrap().1.iter().take(2).cloned().collect();
                    let mut det: Vec<i64> = c.det.iter().copied().collect();
                    det.sort();
                    det.truncate(5);
                    let dl5: Vec<(i64, String)> = dl.iter().take(5).cloned().collect();
                    ex.push(format!("{smp} {g} {} res {} d_call {dc} d_min {dm} {} true {} det {}", c.name, c.res, pylist(&tn), tuple_list(&dl5), int_list(&det)));
                }
            }
        }
    }
    let cols = ["n", "exact", "nearest", "known", "known_ok", "new", "new_nearest", "safe4", "safe4_exact", "safe4_wrong_noflag", "safe4_wrong_real", "flag_on_exact", "flag_on_inexact", "site_snv", "site_snv_found", "site_det", "site_det_true", "skip"];
    let mut out = format!("gene\t{}\n", cols.join("\t"));
    let mut tot = Counter::default();
    let gl: Vec<&str> = if eight { G8.to_vec() } else { G18.to_vec() };
    let zero = Counter::default();
    for g in gl {
        let c = st.get(g).unwrap_or(&zero);
        tot.update(c);
        out += &format!("{g}\t{}\n", cols.iter().map(|k| c.get(k).to_string()).collect::<Vec<_>>().join("\t"));
    }
    out += &format!("ALL\t{}\n", cols.iter().map(|k| tot.get(k).to_string()).collect::<Vec<_>>().join("\t"));
    let t = &tot;
    out += &format!(
        "known alleles 4F exact {}/{} ({}%) | new haplotypes {}, nearest allele chosen {} | safe4 exact {}/{}, inexact unflagged {}, inexact beyond hp/STR {} | SNV sites found {}/{} | reported sites true {}/{}\n",
        t.get("known_ok"), t.get("known"), fx(100.0 * t.get("known_ok") as f64 / t.get("known").max(1) as f64, 1), t.get("new"), t.get("new_nearest"),
        t.get("safe4_exact"), t.get("safe4"), t.get("safe4_wrong_noflag"), t.get("safe4_wrong_real"), t.get("site_snv_found"), t.get("site_snv"), t.get("site_det_true"), t.get("site_det")
    );
    for e in ex {
        out += &e;
        out.push('\n');
    }
    print!("{out}");
}

pub fn eval8_exons(dir: &str, genes_arg: Option<&str>, verbose: bool) {
    let gl: Vec<String> = match genes_arg {
        Some(x) => x.split(',').map(String::from).collect(),
        None => G8.iter().map(|x| x.to_string()).collect(),
    };
    let db = db("db/hladb.json.gz");
    let mut ex: HashMap<String, Vec<String>> = HashMap::new();
    for g in genes(&db) {
        if !gl.iter().any(|x| x == s(g, "gene")) {
            continue;
        }
        for a in alleles(g) {
            let e: Vec<String> = a["exons"].as_array().map(|v| v.iter().filter_map(|e| e[1].as_str()).filter(|x| x.len() >= 20).map(String::from).collect()).unwrap_or_default();
            ex.insert(s(a, "name").to_string(), e);
        }
    }
    let load = |fa: &str| -> String { read(fa).lines().map(|l| if l.starts_with('>') { "N".to_string() } else { l.trim().to_uppercase() }).collect() };
    let mut st: HashMap<String, Counter> = HashMap::new();
    let empty: Vec<String> = vec![];
    for smp in read("newg/dev80.txt").split_whitespace() {
        let mut haps: Vec<(String, String)> = vec![];
        for h in ["h1", "h2", "mat", "pat"] {
            let f = format!("truth/{smp}.{h}.mhc.fa");
            if exists(&f) {
                let x = load(&f);
                let r = revcomp(&x);
                haps.push((x, r));
            }
        }
        let cf = format!("{dir}/{smp}.conf.tsv");
        if haps.len() != 2 || !exists(&cf) {
            continue;
        }
        let mut conf: OMap<String, Vec<(String, i64)>> = OMap::default();
        for r in conf_lines(&cf) {
            if gl.contains(&r[0]) {
                conf.entry(r[0].clone()).push((r[2].clone(), r[4].parse().unwrap()));
            }
        }
        for (g, c) in conf.iter() {
            let c: Vec<(String, i64)> = if c.len() == 1 { vec![c[0].clone(), c[0].clone()] } else { c.clone() };
            if c.len() != 2 {
                continue;
            }
            let miss = |name: &str, hp: &(String, String)| ex.get(name).unwrap_or(&empty).iter().filter(|e| !hp.0.contains(e.as_str()) && !hp.1.contains(e.as_str())).count();
            let m = [[miss(&c[0].0, &haps[0]), miss(&c[0].0, &haps[1])], [miss(&c[1].0, &haps[0]), miss(&c[1].0, &haps[1])]];
            let pm = if m[0][0] + m[1][1] <= m[0][1] + m[1][0] { [0, 1] } else { [1, 0] };
            for i in 0..2 {
                let (name, res) = (&c[i].0, c[i].1);
                let k = st.entry(g.clone()).or_default();
                k.add("n", 1);
                let nex = ex.get(name.as_str()).map_or(0, |v| v.len());
                let ok = m[i][pm[i]] == 0 && nex > 0;
                k.add("exon_exact", ok as i64);
                if res >= 3 {
                    k.add("res3", 1);
                    k.add("res3_exon_exact", ok as i64);
                    if !ok && verbose {
                        println!("{smp} {g} {name} {res} exons missing {} of {nex}", m[i][pm[i]]);
                    }
                } else {
                    k.add("low", 1);
                    k.add("low_exon_exact", ok as i64);
                }
            }
        }
    }
    let cols = ["n", "exon_exact", "res3", "res3_exon_exact", "low", "low_exon_exact"];
    println!("gene\t{}", cols.join("\t"));
    let mut tot = Counter::default();
    let zero = Counter::default();
    for g in &gl {
        let c = st.get(g).unwrap_or(&zero);
        tot.update(c);
        println!("{g}\t{}", cols.iter().map(|k| c.get(k).to_string()).collect::<Vec<_>>().join("\t"));
    }
    println!("ALL\t{}", cols.iter().map(|k| tot.get(k).to_string()).collect::<Vec<_>>().join("\t"));
}

type Lab = HashMap<String, HashMap<String, Vec<Option<BTreeSet<String>>>>>;

pub fn eval24(src: &str) {
    let h = env_or("HLA_WORK", &std::env::current_dir().unwrap().to_string_lossy());
    let gl = ["A", "B", "C", "DRB1", "DQB1"];
    let f2 = |x: &str| -> String { x.rsplit('*').next().unwrap().split(':').take(2).collect::<Vec<_>>().join(":") };
    let raw = read(&format!("{h}/20140702_hla_diversity.txt")).replace('"', "");
    let rows: Vec<&str> = raw.lines().collect();
    let hdr: Vec<&str> = rows[0].split_whitespace().collect();
    let col = |c: &str| hdr.iter().position(|x| *x == c).unwrap();
    let mut truth: Lab = HashMap::new();
    let mut pop: HashMap<String, String> = HashMap::new();
    for l in &rows[1..] {
        let r: Vec<&str> = l.split_whitespace().collect();
        pop.insert(r[0].into(), r[1].into());
        let mut m = HashMap::new();
        for g in gl {
            let v: Vec<Option<BTreeSet<String>>> = [g.to_string(), format!("{g}.1")].iter().map(|c| {
                let x = r.get(col(c)).copied().unwrap_or("");
                // 0000: not typed; 14XX: first field only, not comparable at two fields
                if x.is_empty() || x == "NA" || x == "0000" || x.ends_with("XX") { None } else { Some(x.split('/').map(|a| a.split(':').take(2).collect::<Vec<_>>().join(":")).collect()) }
            }).collect();
            m.insert(g.to_string(), v);
        }
        truth.insert(r[0].into(), m);
    }
    let amb = |x: &str| -> BTreeSet<String> {
        let x = x.trim().trim_end_matches('*');
        let p: Vec<&str> = x.split('/').collect();
        let fam = p[0].split(':').next().unwrap();
        p.iter().map(|q| if q.contains(':') { q.split(':').take(2).collect::<Vec<_>>().join(":") } else { format!("{fam}:{q}") }).collect()
    };
    let mut t18: Lab = HashMap::new();
    let mut pop18: HashMap<String, String> = HashMap::new();
    for l in read(&format!("{h}/lab/hla2018.txt")).lines().skip(1) {
        let mut r: Vec<&str> = l.split('\t').collect();
        while r.len() < 13 {
            r.push("");
        }
        pop18.insert(r[2].into(), r[1].into());
        let cols = [("A", (3, 4)), ("B", (5, 6)), ("C", (7, 8)), ("DQB1", (9, 10)), ("DRB1", (11, 12))];
        let mut m = HashMap::new();
        for (g, (i, j)) in cols {
            let v = [i, j].iter().map(|&k| { let x = r[k].trim(); if x.is_empty() || x == "-" || x == "None" { None } else { Some(amb(r[k])) } }).collect();
            m.insert(g.to_string(), v);
        }
        t18.insert(r[2].into(), m);
    }
    if src == "2018" {
        truth = t18;
        pop = pop18;
    } else if src == "any" {
        for (k, v) in t18 {
            if !truth.contains_key(&k) {
                pop.insert(k.clone(), pop18[&k].clone());
                truth.insert(k, v);
            }
        }
    }
    let t1k = |smp: &str| -> Option<HashMap<String, [Option<String>; 2]>> {
        let p = format!("{h}/t1kout/{smp}_genotype.tsv");
        if !exists(&p) {
            return None;
        }
        let mut d = HashMap::new();
        for l in read(&p).lines() {
            let r: Vec<&str> = l.split('\t').collect();
            let g = r[0].replace("HLA-", "");
            let a1 = if r[2] != "." { Some(f2(r[2])) } else { None };
            let a2 = if r.len() > 5 && r[5] != "." { Some(f2(r[5])) } else { a1.clone() };
            d.insert(g, [a1, a2]);
        }
        Some(d)
    };
    let score = |call: &[Option<String>; 2], tr: &Vec<Option<BTreeSet<String>>>| -> (i64, i64) {
        let mut best = -1;
        for (a, b) in [(0, 1), (1, 0)] {
            let n = [(&call[a], &tr[0]), (&call[b], &tr[1])].iter().filter(|(x, t)| matches!((x, t), (Some(x), Some(t)) if t.contains(x))).count() as i64;
            best = best.max(n);
        }
        (best, tr.iter().filter(|t| t.is_some()).count() as i64)
    };
    let mut res: HashMap<&str, [i64; 2]> = HashMap::new();
    let mut rt: HashMap<&str, [i64; 2]> = HashMap::new();
    let mut rpair: HashMap<&str, [i64; 2]> = HashMap::new();
    let mut safe = [0i64, 0];
    let mut errs: Vec<String> = vec![];
    let mut done: Vec<String> = vec![];
    let od = env_or("HLA_OUT", &format!("{h}/lab/out24"));
    let mut files: Vec<String> = std::fs::read_dir(&od).map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".tsv")).collect()).unwrap_or_default();
    files.sort_by(|a, b| format!("{od}/{a}").cmp(&format!("{od}/{b}")));
    let mut with_t1k = 0;
    for f in files {
        let smp = f[..f.len() - 4].to_string();
        if smp.ends_with(".conf") || !exists(&format!("{od}/{smp}.conf.tsv")) || !truth.contains_key(&smp) {
            continue;
        }
        done.push(smp.clone());
        let mut d: HashMap<String, [Option<String>; 2]> = HashMap::new();
        for l in read(&format!("{od}/{smp}.tsv")).lines().skip(1) {
            let r: Vec<&str> = l.split('\t').collect();
            d.insert(r[0].into(), [Some(f2(r[1])), Some(if r[2] != "-" { f2(r[2]) } else { f2(r[1]) })]);
        }
        let mut c: HashMap<String, Vec<i64>> = HashMap::new();
        for l in read(&format!("{od}/{smp}.conf.tsv")).lines().skip(1) {
            let r: Vec<&str> = l.split('\t').collect();
            c.entry(r[0].into()).or_default().push(r[4].parse().unwrap());
        }
        let t = t1k(&smp);
        if t.is_some() {
            with_t1k += 1;
        }
        for g in gl {
            let tr = &truth[&smp][g];
            let (k, n) = score(&d[g], tr);
            let e = res.entry(g).or_default();
            e[0] += k;
            e[1] += n;
            if tr.iter().all(|x| x.is_some()) && *c[g].iter().min().unwrap() >= 2 {
                safe[0] += k;
                safe[1] += n;
            }
            if k < n {
                let ds = d[g].iter().map(|x| x.clone().unwrap()).collect::<Vec<_>>().join("/");
                let trs: Vec<String> = tr.iter().map(|x| match x {
                    Some(s) => { let j = s.iter().cloned().collect::<Vec<_>>().join("/"); pyrepr(&j.chars().take(60).collect::<String>()) }
                    None => "None".into(),
                }).collect();
                let tk = match &t {
                    Some(tm) if tm.contains_key(g) => tm[g].iter().map(|x| x.clone().unwrap_or("None".into())).collect::<Vec<_>>().join("/"),
                    _ => "-".into(),
                };
                errs.push(format!("ERR {smp} {} {g} {ds} [{}] T1K {tk}", pop[&smp], trs.join(", ")));
            }
            if let Some(tm) = &t {
                let none = [None, None];
                let (tk, tn) = score(tm.get(g).unwrap_or(&none), tr);
                let e = rt.entry(g).or_default();
                e[0] += tk;
                e[1] += tn;
                let e = rpair.entry(g).or_default();
                e[0] += k;
                e[1] += n;
            }
        }
    }
    println!("samples {} with T1K {with_t1k}", done.len());
    let get = |m: &HashMap<&str, [i64; 2]>, g: &str| m.get(g).copied().unwrap_or([0, 0]);
    for g in gl {
        let (a, p, r) = (get(&res, g), get(&rpair, g), get(&rt, g));
        println!("{g}\tours {}/{}\t| on T1K subset: ours {}/{} T1K {}/{}", a[0], a[1], p[0], p[1], r[0], r[1]);
    }
    let sum = |m: &HashMap<&str, [i64; 2]>, i: usize| gl.iter().map(|g| get(m, g)[i]).sum::<i64>();
    println!("total ours [{}, {}] T1K subset ours [{}, {}] T1K [{}, {}]", sum(&res, 0), sum(&res, 1), sum(&rpair, 0), sum(&rpair, 1), sum(&rt, 0), sum(&rt, 1));
    println!("safe(>=2 field) ours [{}, {}]", safe[0], safe[1]);
    for e in errs {
        println!("{e}");
    }
}

pub fn trio23(dir: &str) {
    let load = |smp: &str| -> OMap<String, Vec<(String, i64)>> {
        let mut c: OMap<String, Vec<(String, i64)>> = OMap::default();
        for r in conf_lines(&format!("{dir}/{smp}.conf.tsv")) {
            c.entry(r[0].clone()).push((r[2].clone(), r[4].parse().unwrap()));
        }
        c
    };
    let cutf = |n: &str, k: usize| -> String { if n == "-" { "-".into() } else { n.split('*').nth(1).unwrap_or("").split(':').take(k).collect::<Vec<_>>().join(":") } };
    let cons = |ka: &Vec<String>, f: &Vec<String>, m: &Vec<String>| -> bool {
        let ka = if ka.len() < 2 { ka.iter().chain(ka.iter()).cloned().collect::<Vec<_>>() } else { ka.clone() };
        (f.contains(&ka[0]) && m.contains(&ka[1])) || (f.contains(&ka[1]) && m.contains(&ka[0]))
    };
    let empty: Vec<(String, i64)> = vec![];
    for (kid, fa, mo) in [("HG002", "HG003", "HG004"), ("HG005_BGI", "HG006", "HG007")] {
        let (k_, f_, m_) = (load(kid), load(fa), load(mo));
        for k in [2, 3, 4] {
            let mut ok = 0;
            let mut badl = vec![];
            for (g, kc) in k_.iter() {
                let fc = f_.get(g).unwrap_or(&empty);
                let mc = m_.get(g).unwrap_or(&empty);
                let ka: Vec<String> = kc.iter().map(|x| cutf(&x.0, k)).collect();
                let f: Vec<String> = fc.iter().map(|x| cutf(&x.0, k)).collect();
                let m: Vec<String> = mc.iter().map(|x| cutf(&x.0, k)).collect();
                if cons(&ka, &f, &m) {
                    ok += 1;
                } else {
                    let rs: Vec<String> = kc.iter().chain(fc).chain(mc).map(|x| x.1.to_string()).collect();
                    badl.push(format!("    ({}, {}, {}, {}, [{}])", pyrepr(g), pylist(&ka), pylist(&f), pylist(&m), rs.join(", ")));
                }
            }
            println!("{kid} {k} fields consistent {ok} of {}", k_.len());
            for b in badl {
                println!("{b}");
            }
        }
        let (mut ok, mut n) = (0, 0);
        for (g, kc) in k_.iter() {
            let fc = f_.get(g).unwrap_or(&empty);
            let mc = m_.get(g).unwrap_or(&empty);
            if kc.iter().chain(fc).chain(mc).all(|x| x.1 >= 4) {
                n += 1;
                let c = cons(&kc.iter().map(|x| cutf(&x.0, 4)).collect(), &fc.iter().map(|x| cutf(&x.0, 4)).collect(), &mc.iter().map(|x| cutf(&x.0, 4)).collect());
                ok += c as i64;
                if !c {
                    println!("    SAFE4 conflict {g}");
                }
            }
        }
        println!("{kid} genes where all calls are safe at 4 fields: consistent {ok} of {n}");
    }
}

pub fn sim_eval(truth_path: &str, assign: &str) {
    let mut truth: OMap<String, Vec<String>> = OMap::default();
    for l in read(truth_path).lines() {
        let w: Vec<String> = l.split_whitespace().map(String::from).collect();
        if let Some(k) = w.first() {
            truth.insert(k.clone(), w);
        }
    }
    let mut asg: HashMap<String, HashSet<String>> = HashMap::new();
    for l in read(assign).lines() {
        let p: Vec<&str> = l.split('\t').collect();
        if p.len() < 3 || p[1].is_empty() || !p[1].bytes().all(|c| c.is_ascii_digit()) {
            continue;
        }
        for r in p[2].split(',') {
            if !r.is_empty() {
                asg.entry(r.into()).or_default().insert(p[0].into());
            }
        }
    }
    let mut st: HashMap<(String, &str), i64> = HashMap::new();
    let none = HashSet::new();
    for (r, w) in truth.iter() {
        let (g, mut inp) = (w[1].clone(), w[3].clone());
        if w.len() > 5 && w[5] != g {
            inp = "chim".into();
        }
        let s_ = asg.get(r).unwrap_or(&none);
        let k = if s_.is_empty() { "unass" } else if s_.len() == 1 && s_.contains(&g) { "only" } else if s_.contains(&g) { "plus" } else { "wrong" };
        *st.entry((inp, k)).or_default() += 1;
        *st.entry(("all".into(), k)).or_default() += 1;
    }
    for (grp, label) in [("all", "all"), ("1", "in panel"), ("0", "unseen"), ("chim", "chimeric")] {
        let tot: i64 = st.iter().filter(|((x, _), _)| x == grp).map(|(_, v)| v).sum();
        if tot == 0 {
            continue;
        }
        let parts: Vec<String> = ["only", "plus", "wrong", "unass"].iter().map(|k| {
            let v = st.get(&(grp.to_string(), *k)).copied().unwrap_or(0);
            format!("{k}={v} ({}%)", fx(100.0 * v as f64 / tot as f64, 2))
        }).collect();
        println!("{label} {tot} {}", parts.join(" "));
    }
}
