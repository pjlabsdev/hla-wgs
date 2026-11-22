//! Truth tables from assembled haplotypes: the database alleles placed on each haplotype assembly.
//! minimap2 and mappy of the replaced scripts are the program's own aligner here (presets asm5 and
//! asm20 with the same scores); the exon features come from hla.dat (HLA_DAT) instead of the
//! prototype's pickle.
use crate::aln::{Hit, Index, Opts, Score};
use crate::common::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;

/// minimap2 -x asm20 scoring, several alignments per query (secondary ones up to best_n)
fn asm20() -> Opts {
    Opts { sc: Score { a: 1, b: 4, q: 6, e: 2, q2: 26, e2: 1, zdrop: 200, end_bonus: 0, bw: 500 }, max_align: 50, ..Opts::asm10() }
}

/// alignments as mappy returns them: the best and those scoring at least 0.8 of it, at most 1 + best_n
fn mappy_hits(ix: &Index, q: &[u8], best_n: usize) -> Vec<Hit> {
    let hs = ix.map(q);
    let Some(top) = hs.first().map(|h| h.score) else { return hs };
    hs.into_iter().filter(|h| h.score as f64 >= 0.8 * top as f64).take(1 + best_n).collect()
}

fn work() -> String {
    env_or("HLA_WORK", &std::env::current_dir().unwrap().to_string_lossy())
}

/// truth4 / truth8: per haplotype and gene the database alleles with the fewest edits against the
/// assembly (unaligned ends count as edits); appends to T4OUT / T8OUT and skips haplotypes done
pub fn truth4(eight: bool) {
    const G8: [&str; 8] = ["MICA", "MICB", "TAP1", "TAP2", "DMA", "DMB", "DOA", "DOB"];
    let j = db("db/hladb.json.gz");
    let mut gs: OMap<String, Vec<(String, String)>> = OMap::default();
    for g in genes(&j) {
        for a in alleles(g) {
            if b(a, "imputed") {
                continue;
            }
            let sq = own_seq(a);
            if sq.chars().count() >= 200 {
                gs.entry(s(g, "gene").to_string()).push((s(a, "name").to_string(), sq.to_string()));
            }
        }
    }
    // the 2-field groups each haplotype carries, from the exon truth tables
    let mut tr: OMap<(String, String), OMap<String, HashSet<String>>> = OMap::default();
    if !eight {
        for f in env_or("TRUTHSRC", "truth/truth.tsv,truth/truthx.tsv,truth/truthy.tsv").split(',') {
            for l in read(f).lines() {
                let r: Vec<&str> = l.split('\t').collect();
                if r.len() >= 6 && !r[3].is_empty() && r[3] != "-" {
                    let e = tr.entry((r[0].to_string(), r[1].to_string())).entry(r[2].to_string());
                    e.insert(cut(r[3], 2));
                    for x in r[5].split(',') {
                        if !x.is_empty() {
                            e.insert(x.to_string());
                        }
                    }
                }
            }
        }
    }
    let samples: Vec<String> = read(&env_or("SAMPLES", "newg/both.txt")).split_whitespace().map(String::from).collect();
    let t4 = if eight { env_or("T8OUT", "truth/truth8.tsv") } else { env_or("T4OUT", "truth/truth4.tsv") };
    let mut done: HashSet<(String, String)> = HashSet::new();
    if exists(&t4) {
        for l in read(&t4).lines() {
            let r: Vec<&str> = l.split('\t').collect();
            done.insert((r[0].to_string(), r.get(1).unwrap_or(&"").to_string()));
        }
    }
    let mut out = std::fs::OpenOptions::new().create(true).append(true).open(&t4).unwrap_or_else(|e| panic!("{t4}: {e}"));
    let empty = OMap::default();
    for smp in &samples {
        let mut fas: Vec<String> = std::fs::read_dir("truth").unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|f| f.starts_with(&format!("{smp}.")) && f.ends_with(".mhc.fa")).map(|f| format!("truth/{f}")).collect();
        fas.sort();
        for fa in fas {
            let h = fa.split('.').nth(1).unwrap().to_string();
            if done.contains(&(smp.clone(), h.clone())) {
                continue;
            }
            let grp = tr.get(&(smp.clone(), h.clone())).unwrap_or(&empty);
            let want: Vec<String> = if eight { G8.iter().map(|x| x.to_string()).collect() } else { grp.keys.clone() };
            let ix = index_fasta(&fa, Opts::asm5());
            // gene -> allele -> (edits, length), alleles in order of first alignment
            let mut best: HashMap<String, OMap<String, (i64, usize)>> = HashMap::new();
            for g in &want {
                for (n, sq) in gs.get(g).map(|v| v.as_slice()).unwrap_or(&[]) {
                    if !eight && !grp.get(g).unwrap().contains(&cut(n, 2)) {
                        continue;
                    }
                    for p in map_primary(&ix, sq, n) {
                        let d = (p.nm + p.qs + (p.qlen - p.qe)) as i64;
                        let gb = best.entry(n.split('*').next().unwrap().to_string()).or_default();
                        if gb.get(n).map_or(true, |x| d < x.0) {
                            gb.insert(n.clone(), (d, p.qlen));
                        }
                    }
                }
            }
            for g in &want {
                let Some(bg) = best.get(g).filter(|b| b.len() > 0) else {
                    writeln!(out, "{smp}\t{h}\t{g}\t-\t-\t-").unwrap();
                    continue;
                };
                let m = bg.iter().map(|(_, v)| v.0).min().unwrap();
                let mut names: Vec<&String> = bg.iter().filter(|(_, v)| v.0 == m).map(|(n, _)| n).collect();
                names.sort();
                // the longest allele among the best defines the reference name
                names.sort_by_key(|n| std::cmp::Reverse(bg.get(n).unwrap().1));
                writeln!(out, "{smp}\t{h}\t{g}\t{m}\t{}\t{}", names.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(","), bg.len()).unwrap();
            }
            writeln!(out, "{smp}\t{h}\t_done\t\t\t").unwrap();
            out.flush().unwrap();
        }
    }
}

struct DatAllele {
    name: String,
    partial: bool,
    /// (kind, number, start, end)
    feat: Vec<(String, i64, usize, usize)>,
    seq: String,
    /// exon number -> sequence, in first-insertion order
    exons: Vec<(i64, String)>,
    imputed: bool,
}

/// the alleles of hla.dat per gene, in file order, as the prototype reference read them
fn read_dat(path: &str) -> OMap<String, Vec<DatAllele>> {
    let mut al: Vec<DatAllele> = vec![];
    let mut cur: Option<(Option<String>, bool, Vec<(String, i64, usize, usize)>, String)> = None;
    let flush = |cur: &mut Option<(Option<String>, bool, Vec<(String, i64, usize, usize)>, String)>, al: &mut Vec<DatAllele>| {
        if let Some((name, partial, feat, seq)) = cur.take() {
            let mut exons: Vec<(i64, String)> = vec![];
            for f in feat.iter().filter(|f| f.0 == "exon") {
                let sq = seq.get(f.2..f.3.min(seq.len())).unwrap_or("").to_string();
                match exons.iter_mut().find(|e| e.0 == f.1) {
                    Some(e) => e.1 = sq,
                    None => exons.push((f.1, sq)),
                }
            }
            al.push(DatAllele { name: name.unwrap_or_default(), partial, feat, seq, exons, imputed: false });
        }
    };
    for line in read(path).lines() {
        let tag = line.get(..2).unwrap_or(line);
        if tag == "ID" {
            flush(&mut cur, &mut al);
            cur = Some((None, false, vec![], String::new()));
        } else if let Some(c) = cur.as_mut() {
            if tag == "DE" && c.0.is_none() {
                if let Some(r) = line.strip_prefix("DE   ") {
                    let r = r.strip_prefix("HLA-").unwrap_or(r);
                    let n: String = r.chars().take_while(|&ch| ch != ' ' && ch != ',').collect();
                    if !n.is_empty() {
                        c.0 = Some(n);
                        if line.contains("(partial)") {
                            c.1 = true;
                        }
                    }
                }
            } else if tag == "FT" {
                if let Some(r) = line.strip_prefix("FT   ") {
                    for kind in ["exon", "intron", "UTR"] {
                        if let Some(x) = r.strip_prefix(kind) {
                            let x = x.trim_start();
                            if x.len() == r.len() - kind.len() {
                                continue; // no space after the kind
                            }
                            let d1: String = x.chars().take_while(|ch| ch.is_ascii_digit()).collect();
                            if let Some(rest) = x[d1.len()..].strip_prefix("..") {
                                let d2: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
                                if !d1.is_empty() && !d2.is_empty() {
                                    c.2.push((kind.to_string(), i64::MIN, d1.parse::<usize>().unwrap() - 1, d2.parse().unwrap()));
                                }
                            }
                        }
                    }
                }
                let t = line[2..].trim_start();
                if line[2..].len() > t.len() {
                    if let Some(x) = t.strip_prefix("/number=\"") {
                        let d: String = x.chars().take_while(|ch| ch.is_ascii_digit()).collect();
                        if !d.is_empty() && x[d.len()..].starts_with('"') {
                            if let Some(f) = c.2.last_mut() {
                                if f.1 == i64::MIN {
                                    f.1 = d.parse().unwrap();
                                }
                            }
                        }
                    }
                }
            } else if tag == "  " {
                let w: Vec<&str> = line.split_whitespace().collect();
                if w.len() > 1 {
                    c.3.push_str(&w[..w.len() - 1].concat().to_uppercase());
                }
            }
        }
    }
    flush(&mut cur, &mut al);
    let mut by: OMap<String, Vec<DatAllele>> = OMap::default();
    for a in al {
        by.entry(a.name.split('*').next().unwrap().to_string()).push(a);
    }
    // imputed in the prototype reference: no introns, and a full-length allele of the gene has all its exons
    for (_, v) in by.map.iter_mut() {
        let full: Vec<Vec<i64>> = v.iter().filter(|a| a.feat.iter().any(|f| f.0 == "intron")).map(|a| a.exons.iter().map(|e| e.0).collect()).collect();
        for a in v.iter_mut() {
            if !a.feat.iter().any(|f| f.0 == "intron") && !full.is_empty() {
                a.imputed = full.iter().any(|ks| a.exons.iter().all(|e| ks.contains(&e.0)));
            }
        }
    }
    by
}

fn rc_acgtn(s: &str) -> String {
    s.bytes().rev().map(|c| match c { b'A' => 'T', b'C' => 'G', b'G' => 'C', b'T' => 'A', b'N' => 'N', x => x as char }).collect()
}

/// exon number -> contig sequence (allele orientation) through the alignment of the allele
fn project(a: &DatAllele, h: &Hit, ctg: &str) -> Vec<(i64, String)> {
    let l = a.seq.len();
    let mut qmap: Vec<i64> = vec![-1; l + 1];
    let mut qpos = if h.fwd { h.q_st } else { l - h.q_en };
    let mut rpos = h.r_st;
    for &(op, n) in &h.ops {
        let n = n as usize;
        match op {
            b'=' | b'X' | b'M' => {
                for i in 0..n {
                    if qpos + i < qmap.len() {
                        qmap[qpos + i] = (rpos + i) as i64;
                    }
                }
                qpos += n;
                rpos += n;
            }
            b'I' => qpos += n,
            b'D' | b'N' => rpos += n,
            _ => {}
        }
    }
    let mut out: Vec<(i64, String)> = vec![];
    for f in a.feat.iter().filter(|f| f.0 == "exon") {
        let (mut st, mut en) = (f.2, f.3);
        if !h.fwd {
            (st, en) = (l - en, l - st);
        }
        if en == 0 || en > l || qmap[st] < 0 || qmap[en - 1] < 0 {
            continue;
        }
        let (rs, re) = (qmap[st] as usize, qmap[en - 1] as usize + 1);
        let seg = ctg.get(rs.min(re)..re.min(ctg.len())).unwrap_or("");
        let v = if h.fwd { seg.to_string() } else { rc_acgtn(seg) };
        match out.iter_mut().find(|e| e.0 == f.1) {
            Some(e) => e.1 = v,
            None => out.push((f.1, v)),
        }
    }
    out
}

/// truth-exon x|y|z FA...: place the full-length alleles on the haplotype, read the assembly's own
/// exons and look them up among all alleles (also exon-only ones) -> S.hap.exon{x,y,z}.tsv
pub fn truth_exon(v: &str, fas: &[&str]) {
    let gl: &[&str] = match v {
        "x" => &["DRA", "E", "F", "G"],
        "y" => &["H", "K", "V"],
        "z" => &["DMA", "DMB", "DOA", "DOB", "MICA", "MICB", "TAP1", "TAP2"],
        _ => panic!("truth-exon x|y|z"),
    };
    let w = work();
    let refd = read_dat(&env_or("HLA_DAT", &format!("{w}/hla.dat")));
    let none: Vec<DatAllele> = vec![];
    let gal = |g: &str| refd.get(&g.to_string()).unwrap_or(&none);
    let full: Vec<Vec<&DatAllele>> = gl.iter().map(|g| gal(g).iter().filter(|a| !a.imputed && a.feat.first().map_or(false, |f| f.0 == "UTR") && a.feat.last().map_or(false, |f| f.0 == "UTR")).collect()).collect();
    println!("{{{}}}", gl.iter().zip(&full).map(|(g, f)| format!("'{g}': {}", f.len())).collect::<Vec<_>>().join(", "));
    // per gene: exon number -> sequence -> allele names
    let ext: Vec<HashMap<i64, HashMap<&str, Vec<&str>>>> = gl
        .iter()
        .map(|g| {
            let mut t: HashMap<i64, HashMap<&str, Vec<&str>>> = HashMap::new();
            for a in gal(g) {
                for (k, sq) in &a.exons {
                    t.entry(*k).or_default().entry(sq.as_str()).or_default().push(a.name.as_str());
                }
            }
            t
        })
        .collect();
    for fa in fas {
        let seqs: HashMap<String, String> = fasta(fa).into_iter().collect();
        let ix = index_fasta(fa, asm20());
        let outname = fa.replace(".mhc.fa", &format!(".exon{v}.tsv"));
        let mut rows: Vec<String> = vec![];
        for (gi, g) in gl.iter().enumerate() {
            let byname: HashMap<&str, &DatAllele> = gal(g).iter().map(|a| (a.name.as_str(), a)).collect();
            let mut loci: OMap<(String, usize), Vec<(i64, &DatAllele, Hit)>> = OMap::default();
            for a in &full[gi] {
                let l = a.seq.len();
                for h in mappy_hits(&ix, a.seq.as_bytes(), 20) {
                    if ((h.q_en - h.q_st) as f64) / (l as f64) < 0.9 {
                        continue;
                    }
                    let ed = (h.nm + (l - (h.q_en - h.q_st))) as i64;
                    loci.entry((ix.names[h.seq].clone(), h.r_st / 20000)).push((ed, a, h));
                }
            }
            for key in loci.keys.clone() {
                let mut vv = loci.map.remove(&key).unwrap();
                vv.sort_by_key(|x| x.0);
                let (ed, a, h) = (&vv[0].0, vv[0].1, &vv[0].2);
                let exs = project(a, h, &seqs[&key.0]);
                let mut cnt: OMap<&str, usize> = OMap::default();
                for (k, sq) in &exs {
                    if let Some(ns) = ext[gi].get(k).and_then(|t| t.get(sq.as_str())) {
                        let mut seen: HashSet<&str> = HashSet::new();
                        for n in ns {
                            if seen.insert(n) {
                                *cnt.entry(n) += 1;
                            }
                        }
                    }
                }
                let mut matched: Vec<&str> = vec![];
                for (n, c) in cnt.iter() {
                    let al = byname[n];
                    let ks = al.exons.iter().filter(|e| exs.iter().any(|x| x.0 == e.0)).count();
                    if *c == al.exons.len() && *c == ks {
                        matched.push(n);
                    }
                }
                let mut fullm: Vec<&str> = matched.iter().copied().filter(|n| !byname[n].partial).collect();
                fullm.sort();
                matched.sort();
                let core = if ["A", "B", "C"].contains(g) { [2, 3].iter().all(|k| exs.iter().any(|x| x.0 == *k)) } else { exs.iter().any(|x| x.0 == 2) };
                let j = |v: &Vec<&str>| if v.is_empty() { "-".to_string() } else { v.join(",") };
                rows.push(format!("{g}\t{}:{}\t{ed}\t{}\t{}\t{}\t{}\t{}\t{}", key.0, h.r_st, a.name, j(&fullm), j(&matched), exs.len(), core as i32, if vv.len() > 1 { vv[1].0 } else { -1 }));
            }
        }
        let tmp = format!("{outname}.tmp");
        let mut o = std::fs::File::create(&tmp).unwrap();
        for r in &rows {
            writeln!(o, "{r}").unwrap();
        }
        drop(o);
        std::fs::rename(&tmp, &outname).unwrap();
        println!("{outname} {}", rows.len());
    }
}

fn fields(a: &str, n: usize) -> String {
    cut(a, n)
}

/// truth-table x|y|z: truth/truth{v}.tsv (sample hap gene allele edits set2 set3) and
/// truth/novel{v}.tsv from the exon projections truth/*.exon{v}.tsv
pub fn truth_table(v: &str) {
    let mut out = std::io::BufWriter::new(std::fs::File::create(format!("truth/truth{v}.tsv")).unwrap());
    let mut nov = std::io::BufWriter::new(std::fs::File::create(format!("truth/novel{v}.tsv")).unwrap());
    let suf = format!(".exon{v}.tsv");
    let mut ps: Vec<String> = std::fs::read_dir("truth").unwrap().filter_map(|e| e.ok()).map(|e| format!("truth/{}", e.file_name().to_string_lossy())).filter(|p| p.ends_with(&suf) && !p.contains(".mhc.")).collect();
    ps.sort();
    type Row = (String, i64, String, i64, String, Vec<String>, Vec<String>, bool);
    let key = |r: &Row| (r.3, -(r.5.len() as i64));
    for p in ps {
        let base = p.rsplit('/').next().unwrap();
        let mut it = base.split('.');
        let (smp, hap) = (it.next().unwrap().to_string(), it.next().unwrap().to_string());
        let mut rows: Vec<Row> = vec![];
        for line in read(&p).lines() {
            let r: Vec<&str> = line.split('\t').collect();
            let (ctg, pos) = r[1].rsplit_once(':').unwrap();
            let sp = |x: &str| if x == "-" { vec![] } else { x.split(',').map(String::from).collect() };
            rows.push((ctg.to_string(), pos.parse().unwrap(), r[0].to_string(), r[2].parse().unwrap(), r[3].to_string(), sp(r[4]), sp(r[5]), r[7].parse::<i64>().unwrap() != 0));
        }
        rows.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
        let mut cl: Vec<Vec<Row>> = vec![];
        for r in rows {
            if let Some(c) = cl.last_mut() {
                if c.last().unwrap().0 == r.0 && (r.1 - c[0].1).abs() < 3000 {
                    c.push(r);
                    continue;
                }
            }
            cl.push(vec![r]);
        }
        let mut win: Vec<Row> = cl
            .into_iter()
            .map(|c| {
                let mut b = 0;
                for i in 1..c.len() {
                    if key(&c[i]) < key(&c[b]) {
                        b = i;
                    }
                }
                c[b].clone()
            })
            .collect();
        win.sort_by(|a, b| (&a.0, &a.2, a.1).cmp(&(&b.0, &b.2, b.1)));
        let mut mg: Vec<Row> = vec![];
        for r in win {
            // the same gene twice within 12 kb on one contig = one locus split by the 20 kb bucketing
            if let Some(m) = mg.last_mut() {
                if m.0 == r.0 && m.2 == r.2 && (r.1 - m.1).abs() < 12000 {
                    if key(&r) < key(m) {
                        *m = r;
                    }
                    continue;
                }
            }
            mg.push(r);
        }
        for (_, _, g, mut ed, best, fullm, matched, core) in mg {
            if ed > 150 || !core {
                continue;
            }
            let a = if ed == 0 { best.clone() } else if !fullm.is_empty() { fullm[0].clone() } else if !matched.is_empty() { matched[0].clone() } else { best.clone() };
            let mut s3: Vec<String> = fullm.iter().map(|x| fields(x, 3)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
            let src = if fullm.is_empty() { &matched } else { &fullm };
            let s2: Vec<String> = src.iter().map(|x| fields(x, 2)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
            if fullm.is_empty() && matched.is_empty() {
                writeln!(nov, "{smp} {hap} {g} {best} {ed}").unwrap();
                ed = 99;
            } else if fullm.is_empty() {
                s3.clear();
            }
            writeln!(out, "{smp}\t{hap}\t{g}\t{a}\t{ed}\t{}\t{}", s2.join(","), s3.join(",")).unwrap();
        }
    }
}

/// mhc-crop FA...: keep the MHC window of the contigs (HLA-A to DPB1, 300 kb either side) -> FA
/// with .fa replaced by .mhc.fa (contig names CONTIG_START)
pub fn mhc_crop(fas: &[&str]) {
    const PROBES: [&str; 10] = ["A*01:01:01:01", "B*07:02:01:01", "C*07:01:01:01", "DRA*01:01:01:01", "DRB1*01:01:01:01", "DQA1*01:01:01:01", "DQB1*02:01:01:01", "DPA1*01:03:01:01", "DPB1*01:01:01:01", "DPB1*04:01:01:01"];
    let w = work();
    let j = db(&env_or("HLA_DB", &format!("{w}/db/hladb.json.gz")));
    let mut probe: Vec<(String, String)> = vec![];
    for g in genes(&j) {
        for a in alleles(g) {
            if PROBES.contains(&s(a, "name")) {
                probe.push((s(a, "name").to_string(), own_seq(a).to_uppercase()));
            }
        }
    }
    for fa in fas {
        let ix = index_fasta(fa, asm20());
        let mut pos: OMap<String, Vec<usize>> = OMap::default();
        for (_, sq) in &probe {
            for h in mappy_hits(&ix, sq.as_bytes(), 5) {
                pos.entry(ix.names[h.seq].clone()).push(h.r_st);
            }
        }
        if pos.len() == 0 {
            println!("cropped {fa}");
            continue;
        }
        let seqs: BTreeMap<String, String> = fasta(fa).into_iter().collect();
        let out = fa.replace(".fa", ".mhc.fa");
        let mut o = std::io::BufWriter::new(std::fs::File::create(&out).unwrap());
        for (c, ps) in pos.iter() {
            let sq = &seqs[c];
            let lo = ps.iter().min().unwrap().saturating_sub(300000);
            let hi = (ps.iter().max().unwrap() + 300000).min(sq.len());
            write!(o, ">{c}_{lo}\n{}\n", &sq[lo..hi]).unwrap();
        }
        println!("cropped {out}");
    }
}

/// allele-vs-assembly S HAP GENE PREFIX...: edits of the named alleles against an assembled haplotype
pub fn allele_vs_assembly(smp: &str, h: &str, g: &str, pre: &[&str]) {
    let j = db("db/hladb.json.gz");
    let ix = index_fasta(&format!("truth/{smp}.{h}.mhc.fa"), Opts::asm5());
    let mut rows: Vec<(i64, String, usize, String, usize)> = vec![];
    for gg in genes(&j).iter().filter(|x| s(x, "gene") == g) {
        for a in alleles(gg) {
            if b(a, "imputed") || !pre.iter().any(|p| s(a, "name").starts_with(p)) {
                continue;
            }
            for p in map_primary(&ix, own_seq(a), s(a, "name")) {
                rows.push(((p.nm + p.qs + p.qlen - p.qe) as i64, p.qname.clone(), p.qlen, p.tname.clone(), p.ts));
            }
        }
    }
    rows.sort();
    for r in rows.iter().take(6) {
        println!("{smp} {h} {} {} {} {} {}", r.0, r.1, r.2, r.3, r.4);
    }
}
