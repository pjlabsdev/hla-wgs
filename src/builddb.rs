//! `hla-wgs build-db hla.dat freq.json OUTDIR`: build the local allele index from the official
//! IPD-IMGT/HLA hla.dat on the user's machine (the IMGT data are CC BY-ND and are not shipped).
//! Port of the prototype steps refbuild.py, extend.py, build_donor.py, panel.py, export_db.py;
//! the result must equal the prototype package for the same hla.dat.
use crate::align::{aligner, best_mlen, map, Preset};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::io::{BufRead, Write};

#[derive(Clone)]
struct Feat {
    kind: String,
    num: Option<u32>,
    st: usize,
    en: usize,
}

#[derive(Clone)]
struct Al {
    name: String,
    gene: String,
    partial: bool,
    feat: Vec<Feat>,
    seq: String,
    /// exon number -> sequence, in first-insertion order of the feature table
    exons: Vec<(u32, String)>,
    genomic: String,
    imputed: bool,
    imputed_from: Option<String>,
    extended: bool,
}

impl Al {
    fn exon(&self, k: u32) -> Option<&str> {
        self.exons.iter().find(|e| e.0 == k).map(|e| e.1.as_str())
    }
    fn two(&self) -> String {
        self.name.split(':').take(2).collect::<Vec<_>>().join(":")
    }
}

fn parse(dat: &str) -> Vec<Al> {
    let f = std::fs::File::open(dat).unwrap_or_else(|e| crate::util::die(&format!("{dat}: {e}")));
    let mut rd = std::io::BufReader::with_capacity(1 << 20, f);
    // an EMBL flat file starts with "ID   HLA....."; anything else is the wrong file
    let head = std::io::BufRead::fill_buf(&mut rd).unwrap_or_else(|e| crate::util::die(&format!("{dat}: {e}")));
    if !head.starts_with(b"ID   HLA") {
        crate::util::die(&format!("not an IPD-IMGT/HLA hla.dat file: {dat} (its first line must start with \"ID   HLA\"; unpack hla.dat.zip first)"));
    }
    let mut out: Vec<Al> = vec![];
    let mut cur: Option<(Option<String>, bool, Vec<Feat>, String)> = None;
    let mut flush = |cur: &mut Option<(Option<String>, bool, Vec<Feat>, String)>, out: &mut Vec<Al>| {
        if let Some((Some(name), partial, feat, seq)) = cur.take() {
            let gene = name.split('*').next().unwrap().to_string();
            let mut exons: Vec<(u32, String)> = vec![];
            for f in feat.iter().filter(|f| f.kind == "exon") {
                let k = f.num.expect("exon without number");
                let s = seq[f.st..f.en].to_string();
                match exons.iter_mut().find(|e| e.0 == k) {
                    Some(e) => e.1 = s,
                    None => exons.push((k, s)),
                }
            }
            out.push(Al { name, gene, partial, feat, seq, exons, genomic: String::new(), imputed: false, imputed_from: None, extended: false });
        }
    };
    for line in rd.lines() {
        let line = line.unwrap_or_else(|e| crate::util::die(&format!("{dat}: {e}")));
        let tag = line.get(..2).unwrap_or("");
        if tag == "ID" {
            flush(&mut cur, &mut out);
            cur = Some((None, false, vec![], String::new()));
        } else if tag == "DE" {
            let c = cur.as_mut().unwrap();
            if c.0.is_none() {
                // DE   (?:HLA-)?([^ ,]+)
                if let Some(rest) = line.strip_prefix("DE   ") {
                    let rest = rest.strip_prefix("HLA-").unwrap_or(rest);
                    let n: String = rest.chars().take_while(|&ch| ch != ' ' && ch != ',').collect();
                    if !n.is_empty() {
                        c.0 = Some(n);
                        if line.contains("(partial)") {
                            c.1 = true;
                        }
                    }
                }
            }
        } else if tag == "FT" {
            let c = cur.as_mut().unwrap();
            // FT   (exon|intron|UTR)\s+(\d+)\.\.(\d+)
            if let Some(rest) = line.strip_prefix("FT   ") {
                for kind in ["exon", "intron", "UTR"] {
                    if let Some(r) = rest.strip_prefix(kind) {
                        if r.starts_with(|ch: char| ch.is_whitespace()) {
                            let r = r.trim_start();
                            let a: String = r.chars().take_while(|ch| ch.is_ascii_digit()).collect();
                            let r2 = &r[a.len()..];
                            if !a.is_empty() && r2.starts_with("..") {
                                let b: String = r2[2..].chars().take_while(|ch| ch.is_ascii_digit()).collect();
                                if !b.is_empty() {
                                    c.2.push(Feat { kind: kind.to_string(), num: None, st: a.parse::<usize>().unwrap() - 1, en: b.parse().unwrap() });
                                }
                            }
                        }
                        break;
                    }
                }
            }
            // FT\s+/number="(\d+)"
            let r = line[2..].trim_start();
            if line[2..].starts_with(|ch: char| ch.is_whitespace()) {
                if let Some(r) = r.strip_prefix("/number=\"") {
                    let d: String = r.chars().take_while(|ch| ch.is_ascii_digit()).collect();
                    if !d.is_empty() && r[d.len()..].starts_with('"') {
                        if let Some(l) = c.2.last_mut() {
                            if l.num.is_none() {
                                l.num = Some(d.parse().unwrap());
                            }
                        }
                    }
                }
            }
        } else if tag == "  " {
            let c = cur.as_mut().unwrap();
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                for p in &parts[..parts.len() - 1] {
                    c.3.push_str(&p.to_ascii_uppercase());
                }
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// refbuild.py: genomic sequence for every allele, exon-only alleles imputed from the closest
/// full-length allele (fewest exon mismatches among equal-length candidates)
fn refbuild(alleles: Vec<Al>) -> Vec<(String, Vec<Al>)> {
    let mut bygene: Vec<(String, Vec<Al>)> = vec![];
    for a in alleles {
        match bygene.iter_mut().find(|g| g.0 == a.gene) {
            Some(g) => g.1.push(a),
            None => bygene.push((a.gene.clone(), vec![a])),
        }
    }
    for (_, al) in bygene.iter_mut() {
        let is_full: Vec<bool> = al.iter().map(|a| a.feat.iter().any(|f| f.kind == "intron")).collect();
        let full: Vec<usize> = (0..al.len()).filter(|&i| is_full[i]).collect();
        if full.is_empty() {
            for a in al.iter_mut() {
                a.genomic = a.seq.clone();
            }
            continue;
        }
        let snapshot: Vec<Al> = al.clone();
        let res: Vec<(String, bool, Option<String>)> = (0..al.len())
            .into_par_iter()
            .map(|i| {
                let a = &snapshot[i];
                if is_full[i] {
                    return (a.seq.clone(), false, None);
                }
                let mut keys: Vec<u32> = a.exons.iter().map(|e| e.0).collect();
                keys.sort();
                let q: Vec<u8> = keys.iter().flat_map(|k| a.exon(*k).unwrap().bytes()).collect();
                let cand: Vec<usize> = full.iter().copied().filter(|&f| keys.iter().all(|k| snapshot[f].exon(*k).is_some())).collect();
                if cand.is_empty() {
                    return (a.seq.clone(), false, None);
                }
                let mut best: Option<(usize, usize)> = None;
                for &f in &cand {
                    let fl: usize = keys.iter().map(|k| snapshot[f].exon(*k).unwrap().len()).sum();
                    if fl != q.len() {
                        continue;
                    }
                    let mut d = 0usize;
                    let mut qi = 0usize;
                    for k in &keys {
                        for &c in snapshot[f].exon(*k).unwrap().as_bytes() {
                            if c != q[qi] {
                                d += 1;
                            }
                            qi += 1;
                        }
                    }
                    if best.map_or(true, |b| d < b.1) {
                        best = Some((f, d));
                    }
                }
                let bi = best.map(|b| b.0).unwrap_or(cand[0]);
                let b = &snapshot[bi];
                let s = &b.seq;
                let mut ex: Vec<&Feat> = b.feat.iter().filter(|f| f.kind == "exon").collect();
                ex.sort_by_key(|f| f.st);
                let mut g = String::with_capacity(s.len());
                let mut pos = 0usize;
                for f in ex {
                    g.push_str(&s[pos..f.st]);
                    match f.num.and_then(|n| a.exon(n)) {
                        Some(x) => g.push_str(x),
                        None => g.push_str(&s[f.st..f.en]),
                    }
                    pos = f.en;
                }
                g.push_str(&s[pos..]);
                (g, true, Some(b.name.clone()))
            })
            .collect();
        for (a, (g, imp, from)) in al.iter_mut().zip(res) {
            a.genomic = g;
            a.imputed = imp;
            a.imputed_from = from;
        }
    }
    bygene
}

const TARGET: [&str; 31] = ["A", "B", "C", "E", "F", "G", "H", "J", "K", "L", "DRA", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1", "DQA2", "DQB2", "DPA2", "DPB2", "DMA", "DMB", "DOA", "DOB", "MICA", "MICB", "TAP1", "TAP2"];

/// numpy.percentile(x, q) with linear interpolation
fn percentile(v: &[usize], q: f64) -> f64 {
    let mut s: Vec<f64> = v.iter().map(|&x| x as f64).collect();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pos = q / 100.0 * (s.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    s[lo] + (s[hi] - s[lo]) * (pos - lo as f64)
}

/// Layout of the built database, written to `DBDIR/format`. 2: short alleles of DMA, DMB, DOA, DOB,
/// MICA, MICB, TAP1 and TAP2 carry donor flanks like those of the other genes (1.1).
pub const DB_FORMAT: u32 = 2;

/// extend.py: give short alleles the 5'/3' flanks of the nearest full-span allele
fn extend(genes: &mut [(String, Vec<Al>)]) {
    for (g, al) in genes.iter_mut() {
        if !TARGET.contains(&g.as_str()) {
            continue;
        }
        let lens: Vec<usize> = al.iter().map(|a| a.genomic.len()).collect();
        let cut = 0.97 * percentile(&lens, 90.0);
        let full: Vec<usize> = (0..al.len()).filter(|&i| al[i].genomic.len() as f64 >= cut && !al[i].imputed).collect();
        if full.is_empty() {
            continue;
        }
        let snapshot: Vec<Al> = al.clone();
        let res: Vec<Option<String>> = (0..al.len())
            .into_par_iter()
            .map(|i| {
                let a = &snapshot[i];
                if a.genomic.len() as f64 >= cut {
                    return None;
                }
                let e2 = a.exon(2).unwrap_or("").as_bytes();
                let mut best: Option<usize> = None;
                let mut bd = usize::MAX;
                for &f in &full {
                    let x = snapshot[f].exon(2).unwrap_or("").as_bytes();
                    if x.len() == e2.len() {
                        let d = x.iter().zip(e2).filter(|(p, q)| p != q).count();
                        if d < bd {
                            bd = d;
                            best = Some(f);
                        }
                        if d == 0 {
                            break;
                        }
                    }
                }
                let b = &snapshot[best.unwrap_or(full[0])].genomic;
                let am = aligner(b.as_bytes(), Preset::Asm10);
                let hs = map(&am, a.genomic.as_bytes());
                let h = hs.first()?;
                if !h.fwd {
                    return None;
                }
                let pre = if h.q_st == 0 { &b[..h.r_st as usize] } else { "" };
                let post = if h.q_en as usize == a.genomic.len() { &b[h.r_en as usize..] } else { "" };
                if pre.is_empty() && post.is_empty() {
                    return None;
                }
                Some(format!("{pre}{}{post}", a.genomic))
            })
            .collect();
        let mut n = 0;
        for (a, r) in al.iter_mut().zip(res) {
            if let Some(gn) = r {
                a.genomic = gn;
                a.extended = true;
                n += 1;
            }
        }
        eprintln!("{g} alleles {} extended {n}", al.len());
    }
}

const DONOR_GENES: [&str; 11] = ["A", "B", "C", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1"];

/// build_donor.py: edit distance of every allele to each longer non-extended sibling of its
/// 2-field group (flank donors), precomputed once per release
fn donors(ext: &[(String, Vec<Al>)], orig: &BTreeMap<(String, String), String>) -> Vec<(String, String, String, Option<i64>)> {
    let mut out = vec![];
    for (g, al) in ext.iter() {
        if !DONOR_GENES.contains(&g.as_str()) {
            continue;
        }
        let og = |a: &Al| -> String {
            if a.extended {
                orig.get(&(g.clone(), a.name.clone())).cloned().unwrap_or_else(|| a.genomic.clone())
            } else {
                a.genomic.clone()
            }
        };
        let mut bygrp: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for b in al.iter().filter(|b| !b.imputed) {
            bygrp.entry(b.two()).or_default().push(b.genomic.len());
        }
        let jobs: Vec<usize> = (0..al.len())
            .filter(|&i| bygrp.get(&al[i].two()).map_or(false, |l| *l.iter().max().unwrap() > og(&al[i]).len() + 20))
            .collect();
        let res: Vec<Vec<(String, String, String, Option<i64>)>> = jobs
            .par_iter()
            .map(|&i| {
                let a = &al[i];
                let s_ = og(a);
                let grp = a.two();
                let longer: Vec<usize> = (0..al.len())
                    .filter(|&j| !al[j].imputed && al[j].two() == grp && !al[j].extended && al[j].genomic.len() > s_.len() + 20)
                    .collect();
                let mut o = vec![];
                if !longer.is_empty() {
                    let am = aligner(s_.as_bytes(), Preset::Asm10);
                    for j in longer {
                        let hs = map(&am, al[j].genomic.as_bytes());
                        let nm = best_mlen(&hs).map(|h| h.nm + (s_.len() as i64 - (h.r_en - h.r_st)));
                        o.push((g.clone(), a.name.clone(), al[j].name.clone(), nm));
                    }
                }
                o
            })
            .collect();
        out.extend(res.into_iter().flatten());
    }
    out
}

/// panel.py: one representative per 2-field group (non-imputed first, by name), max 60 per gene
fn panel(genes: &[(String, Vec<Al>)], out: &str) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(out).unwrap());
    for (_, al) in genes {
        let mut idx: Vec<usize> = (0..al.len()).collect();
        idx.sort_by(|&x, &y| (al[x].imputed, al[x].name.as_bytes()).cmp(&(al[y].imputed, al[y].name.as_bytes())));
        let mut seen = std::collections::HashSet::new();
        let mut k = 0;
        for i in idx {
            let a = &al[i];
            let two = a.name.split('*').nth(1).unwrap().split(':').take(2).collect::<Vec<_>>().join(":");
            if !seen.insert(two) {
                continue;
            }
            writeln!(f, ">{}\n{}", a.name, a.genomic).unwrap();
            k += 1;
            if k >= 60 {
                break;
            }
        }
    }
}

pub fn run(args: &[String]) {
    if args.len() < 3 {
        eprintln!("usage: hla-wgs build-db hla.dat freq.json OUTDIR   (hla.dat from IPD-IMGT/HLA)");
        std::process::exit(2);
    }
    let (dat, freq, outdir) = (&args[0], &args[1], &args[2]);
    std::fs::create_dir_all(outdir).unwrap();
    let t = std::time::Instant::now();
    let alleles = parse(dat);
    eprintln!("[{:.0}s] {} alleles", t.elapsed().as_secs_f64(), alleles.len());
    let base = refbuild(alleles);
    eprintln!("[{:.0}s] genomic sequences built", t.elapsed().as_secs_f64());
    panel(&base, &format!("{outdir}/panel.fa"));
    let orig: BTreeMap<(String, String), String> = base
        .iter()
        .flat_map(|(g, al)| al.iter().filter(|a| !a.imputed).map(move |a| ((g.clone(), a.name.clone()), a.genomic.clone())))
        .collect();
    let mut ext = base.clone();
    extend(&mut ext);
    eprintln!("[{:.0}s] flanks extended", t.elapsed().as_secs_f64());
    let donor = donors(&ext, &orig);
    eprintln!("[{:.0}s] donor distances {}", t.elapsed().as_secs_f64(), donor.len());
    // keep the frequency table as written (exact float text), not re-serialised
    let fr_txt = std::fs::read_to_string(freq).unwrap_or_else(|e| panic!("{freq}: {e}"));
    let fr: Box<serde_json::value::RawValue> = serde_json::value::RawValue::from_string(fr_txt.trim().to_string()).expect("freq json");
    let genes: Vec<serde_json::Value> = ext
        .iter()
        .map(|(g, al)| {
            let rows: Vec<serde_json::Value> = al
                .iter()
                .map(|a| {
                    let mut ex = a.exons.clone();
                    ex.sort_by_key(|e| e.0);
                    let o = if a.extended { orig.get(&(g.clone(), a.name.clone())).cloned() } else { None };
                    serde_json::json!({"name": a.name, "genomic": a.genomic, "exons": ex, "imputed": a.imputed,
                        "partial": a.partial, "extended": a.extended, "imputed_from": a.imputed_from, "orig": o})
                })
                .collect();
            serde_json::json!({"gene": g, "alleles": rows})
        })
        .collect();
    let donor_j: Vec<serde_json::Value> = donor.iter().map(|(g, a, b, v)| serde_json::json!([g, a, b, v])).collect();
    let f = std::fs::File::create(format!("{outdir}/hladb.json.gz")).unwrap();
    let mut gz = flate2::write::GzEncoder::new(f, flate2::Compression::default());
    gz.write_all(b"{\"genes\":").unwrap();
    serde_json::to_writer(&mut gz, &genes).unwrap();
    gz.write_all(b",\"freq\":").unwrap();
    gz.write_all(fr.get().as_bytes()).unwrap();
    gz.write_all(b",\"donor_nm\":").unwrap();
    serde_json::to_writer(&mut gz, &donor_j).unwrap();
    gz.write_all(b"}").unwrap();
    gz.finish().unwrap();
    std::fs::write(format!("{outdir}/format"), format!("{DB_FORMAT}\n")).unwrap_or_else(|e| crate::util::die(&format!("{outdir}/format: {e}")));
    eprintln!("[{:.0}s] wrote {outdir}/hladb.json.gz and {outdir}/panel.fa", t.elapsed().as_secs_f64());
}
