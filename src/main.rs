mod align;
mod bamin;
mod builddb;
mod db;
mod groups;
mod html;
mod kern;
mod long;
mod mask;
mod names;
mod novel;
mod output;
mod panel;
mod prior;
mod refine;
mod stagea;
mod stageb;
mod util;

use ahash::{AHashMap, AHashSet};
use names::{three, two};
use rayon::prelude::*;
use std::time::Instant;

pub const FOCUS: [&str; 26] = [
    "A", "B", "C", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1", "DRA", "E", "F",
    "G", "H", "K", "V", "DMA", "DMB", "DOA", "DOB", "MICA", "MICB", "TAP1", "TAP2",
];

pub fn related(g: &str) -> &'static [&'static str] {
    match g {
        "A" => &["Y", "H", "J"],
        "DQA1" => &["DQA2"],
        "DQB1" => &["DQB2"],
        "DPA1" => &["DPA2"],
        "DPB1" => &["DPB2"],
        _ => &[],
    }
}

fn main() {
    let mut a: Vec<String> = std::env::args().collect();
    match a.get(1).map(|x| x.as_str()) {
        Some("build-db") => return builddb::run(&a[2..]),
        Some("build-mask") => return mask::run(&a[2..]),
        Some("long") => return long::run(&a[2..]),
        Some("--version") | Some("-V") => return println!("hla-wgs {}", env!("CARGO_PKG_VERSION")),
        _ => {}
    }
    // a database directory stands for its two files
    if a.get(1).map_or(false, |d| std::path::Path::new(d).is_dir()) {
        let d = a[1].trim_end_matches('/').to_string();
        a[1] = format!("{d}/hladb.json.gz");
        a.insert(2, format!("{d}/panel.fa"));
    }
    let aligned = a.get(3).map_or(false, |x| {
        let l = x.to_ascii_lowercase();
        l.ends_with(".bam") || l.ends_with(".cram")
    });
    if a.len() < if aligned { 5 } else { 6 } {
        eprintln!("usage: hla-wgs DBDIR R1.fq.gz R2.fq.gz OUTPREFIX [threads]");
        eprintln!("       hla-wgs DBDIR INPUT.bam|INPUT.cram OUTPREFIX [threads] [REF.fa (CRAM)]");
        eprintln!(
            "       hla-wgs long DBDIR READS.fastq[.gz]|READS.bam OUTPREFIX [threads]   (PacBio HiFi)"
        );
        eprintln!("       hla-wgs build-db hla.dat freq.json DBDIR");
        eprintln!("       hla-wgs build-mask DBDIR GENOME.fa [EXCLUDE.bed]");
        eprintln!("       (DBDIR may also be given as its two files: hladb.json.gz panel.fa)");
        std::process::exit(2);
    }
    let (input, out, ti) = if aligned {
        (
            panel::Input::Aligned(a[3].clone(), a.get(6).cloned()),
            a[4].clone(),
            5,
        )
    } else {
        (
            panel::Input::Fastq(a[3].clone(), a[4].clone()),
            a[5].clone(),
            6,
        )
    };
    let threads: usize = a.get(ti).and_then(|x| x.parse().ok()).unwrap_or(4);
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .ok();
    let t0 = Instant::now();
    let log = |s: String| eprintln!("[{:6.1}s] {}", t0.elapsed().as_secs_f64(), s);
    let (p, db) = rayon::join(
        || panel::run(&a[2], &input, threads),
        || db::Db::load(&a[1]),
    );
    let reads = &p.reads;
    log(format!(
        "panel mapped reads: {}",
        reads
            .seq
            .iter()
            .filter(|r| r[0].is_some() || r[1].is_some())
            .count()
    ));
    if std::env::var("DUMP0").is_ok() {
        let mut gg: Vec<_> = p.assign.iter().collect();
        gg.sort();
        for (gene, qs) in gg {
            let nm: Vec<&str> = qs.iter().map(|&q| reads.names[q].as_str()).collect();
            println!("{}\t{}\t{}", gene, qs.len(), nm.join(","));
        }
        let mut h = 0u64;
        for (q, r) in reads.seq.iter().enumerate() {
            for m in r.iter() {
                if let Some(s) = m {
                    h = h.wrapping_mul(31).wrapping_add(q as u64 + s.len() as u64);
                }
            }
        }
        println!("READS\t{}", h);
    }
    let genes: Vec<&str> = match std::env::var("GENES") {
        Ok(v) => FOCUS
            .iter()
            .copied()
            .filter(|g| v.split(',').any(|x| x == *g))
            .collect(),
        Err(_) => FOCUS.to_vec(),
    };
    let empty = vec![];
    let ctx = stageb::Ctx::new(&db, reads);
    let mut res: AHashMap<String, (stageb::StageB, Option<Vec<String>>, refine::RMarg)> =
        AHashMap::new();
    let mut depth: AHashMap<String, usize> = AHashMap::new();
    type GeneRes = (stageb::StageB, Option<Vec<String>>, refine::RMarg);
    type Nov = [Vec<novel::Site>; 2];
    let outs: Vec<(String, usize, Option<GeneRes>, Option<Nov>)> = genes
        .par_iter()
        .map(|g| {
            let mut dg = 0usize;
            let qn_a: &Vec<usize> = p.assign.get(*g).unwrap_or(&empty);
            let mut qn: Vec<usize> = qn_a.clone();
            if !related(g).is_empty() {
                let mut set: std::collections::BTreeSet<&str> =
                    qn_a.iter().map(|&q| reads.names[q].as_str()).collect();
                let mut byname: AHashMap<&str, usize> = AHashMap::new();
                for x in std::iter::once(*g).chain(related(g).iter().copied()) {
                    for &q in p.assign.get(x).unwrap_or(&empty) {
                        set.insert(reads.names[q].as_str());
                        byname.insert(reads.names[q].as_str(), q);
                    }
                }
                qn = set.into_iter().map(|n| byname[n]).collect();
            }
            dg = qn.len();
            if qn.len() < 20 {
                return (g.to_string(), dg, None, None);
            }
            let al = &db.ref_[*g];
            let idx = stagea::build_index(al);
            let sa = stagea::run(g, al, &idx, reads, qn_a);
            log(sa.log.clone());
            log(format!(
                "{} reads {} candidates {}",
                g,
                qn.len(),
                sa.cand.len()
            ));
            if std::env::var("DUMPA").is_ok() {
                println!(
                    "{}\t{}",
                    g,
                    sa.cand
                        .iter()
                        .map(|&i| al[i].name.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
            let cand = sa.cand;
            if cand.is_empty() {
                return (g.to_string(), dg, None, None);
            }
            let mut r1 = ctx.stage_b(g, &qn, &cand);
            if let Some(r) = &r1 {
                // pass 2: expand the exon classes of the best pair(s) to all members
                let mut win: AHashSet<usize> = r
                    .ties
                    .iter()
                    .take(6)
                    .flat_map(|p| [cand[p.0], cand[p.1]])
                    .collect();
                let byname: AHashMap<&str, usize> = al
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (a.name.as_str(), i))
                    .collect();
                let par: Vec<usize> = win
                    .iter()
                    .filter_map(|&i| {
                        al[i]
                            .imputed_from
                            .as_deref()
                            .and_then(|n| byname.get(n).copied())
                    })
                    .collect();
                win.extend(par);
                let mut ws: Vec<usize> = win.iter().copied().collect();
                ws.sort();
                let mut f2l: Vec<String> = ws.iter().map(|&i| two(&al[i].name)).collect();
                let nwin = f2l.len();
                let first_set: AHashSet<&String> = f2l[..nwin].iter().collect();
                let lim = first_set.len().max(6);
                f2l.extend(
                    r.near
                        .iter()
                        .flat_map(|p| [two(&al[cand[p.0]].name), two(&al[cand[p.1]].name)]),
                );
                let mut uniq: Vec<String> = vec![];
                for x in &f2l {
                    if !uniq.contains(x) {
                        uniq.push(x.clone());
                    }
                }
                let mut f2: AHashSet<String> = uniq.into_iter().take(lim).collect();
                f2.extend(r.alts.iter().cloned());
                let f3: AHashSet<String> = ws.iter().map(|&i| three(&al[i].name)).collect();
                let (mut pri, mut sec) = (vec![], vec![]);
                for (i, a) in al.iter().enumerate() {
                    if f3.contains(&three(&a.name)) {
                        pri.push(i);
                    } else if f2.contains(&two(&a.name)) && !a.imputed {
                        sec.push(i);
                    }
                }
                let mut c2: std::collections::BTreeSet<usize> =
                    pri.into_iter().chain(sec).take(400).collect();
                c2.extend(ws.iter().copied());
                let cand2: Vec<usize> = c2.into_iter().collect();
                if let Some(x) = ctx.stage_b(g, &qn, &cand2) {
                    r1 = Some(x);
                }
                log(format!("{} pass2 candidates {}", g, cand2.len()));
            }
            if let Some(r) = r1 {
                let n = |k: usize| al[r.cand[k]].name.as_str();
                if std::env::var("DUMPB").is_ok() {
                    println!(
                        "{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}",
                        g,
                        n(r.pair.0),
                        n(r.pair.1),
                        r.ll,
                        r.gap,
                        r.nreads,
                        r.ties.len()
                    );
                }
                let mut rm = refine::RMarg::new();
                let names = refine::exon_refine(&ctx, g, &qn, &r, &mut rm, &log);
                let fin: [String; 2] = match &names {
                    Some(v) => [v[0].clone(), v[1].clone()],
                    None => [
                        al[r.cand[r.pair.0]].name.clone(),
                        al[r.cand[r.pair.1]].name.clone(),
                    ],
                };
                let nv = if std::env::var("HLA_NONOVEL").is_ok() {
                    None
                } else {
                    Some(novel::sites(
                        &ctx,
                        g,
                        &qn,
                        [fin[0].as_str(), fin[1].as_str()],
                    ))
                };
                if let Some(n) = &nv {
                    for (k, v) in n.iter().enumerate() {
                        if !v.is_empty() {
                            log(format!(
                                "{} novel slot{} {} sites {}",
                                g,
                                k + 1,
                                v.len(),
                                v.iter()
                                    .take(8)
                                    .map(|s| format!(
                                        "{}:{}:{}/{}{}",
                                        s.pos,
                                        s.what,
                                        s.alt,
                                        s.depth,
                                        if s.exon { ":exon" } else { "" }
                                    ))
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            ));
                        }
                    }
                }
                log(format!("{} done", g));
                return (g.to_string(), dg, Some((r, names, rm)), nv);
            }
            log(format!("{} done", g));
            (g.to_string(), dg, None, None)
        })
        .collect();
    let mut nov: AHashMap<String, Nov> = AHashMap::new();
    let mut known: AHashMap<String, [Option<String>; 2]> = AHashMap::new();
    for (g, d, r, n) in outs {
        depth.insert(g.clone(), d);
        if let Some(x) = n {
            if let Some((r, names, _)) = &r {
                if std::env::var("HLA_NOKNOWN").is_err() {
                    let al = ctx.al(&g);
                    let fin: [String; 2] = match names {
                        Some(v) => [v[0].clone(), v[1].clone()],
                        None => [
                            al[r.cand[r.pair.0]].name.clone(),
                            al[r.cand[r.pair.1]].name.clone(),
                        ],
                    };
                    let k = [
                        novel::db_match(&ctx, &g, &fin[0], &x[0]),
                        novel::db_match(&ctx, &g, &fin[1], &x[1]),
                    ];
                    for (i, m) in k.iter().enumerate() {
                        if let Some(m) = m {
                            log(format!(
                                "{} slot{} {} plus its residual variants is {}",
                                g,
                                i + 1,
                                fin[i],
                                m
                            ));
                        }
                    }
                    known.insert(g.clone(), k);
                }
            }
            nov.insert(g.clone(), x);
        }
        if let Some(x) = r {
            res.insert(g, x);
        }
    }

    let drb = drb_copies(&ctx, &res, &p.assign, &log);
    let grp = groups::Groups::load(
        std::path::Path::new(&a[1])
            .parent()
            .unwrap_or(std::path::Path::new(".")),
    );
    output::write(
        &out, &genes, &ctx, &res, &depth, &drb, &nov, &known, &grp, &log,
    );
    log("total".to_string());
}

/// joint DRB model: copies of DRB3/4/5 = uniquely explained depth per bp relative to one DRB1 copy
fn drb_copies(
    ctx: &stageb::Ctx,
    res: &AHashMap<String, (stageb::StageB, Option<Vec<String>>, refine::RMarg)>,
    assign: &AHashMap<String, Vec<usize>>,
    log: &dyn Fn(String),
) -> AHashMap<String, f64> {
    let mut seqs: Vec<&[u8]> = vec![];
    let mut owner: Vec<&str> = vec![];
    for g in ["DRB1", "DRB3", "DRB4", "DRB5"] {
        if let Some((r, _, _)) = res.get(g) {
            let mut ks = vec![r.pair.0, r.pair.1];
            ks.sort();
            ks.dedup();
            for k in ks {
                seqs.push(&ctx.al(g)[r.cand[k]].genomic);
                owner.push(g);
            }
        }
    }
    for g in ["DRB2", "DRB6", "DRB7", "DRB8", "DRB9"] {
        if let Some(al) = ctx.db.ref_.get(g) {
            for a in al.iter().filter(|a| !a.imputed) {
                seqs.push(&a.genomic);
                owner.push(g);
            }
        }
    }
    // repeat copies from the rest of the genome compete like the DRB pseudogenes do
    for g in ["DRB1", "DRB3", "DRB4", "DRB5"] {
        if let Some(gd) = ctx.db.genome_decoy.get(g) {
            for x in gd {
                seqs.push(x);
                owner.push("GENOME");
            }
        }
    }
    let mut gl: Vec<&str> = vec!["DRB1", "DRB3", "DRB4", "DRB5"];
    gl.extend(
        ctx.db
            .genes
            .iter()
            .map(|x| x.as_str())
            .filter(|x| x.starts_with("DRB")),
    );
    let mut qs: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for g in gl {
        for &q in assign.get(g).map(|v| v.as_slice()).unwrap_or(&[]) {
            qs.insert(ctx.reads.names[q].as_str(), q);
        }
    }
    let qn: Vec<usize> = qs.values().copied().collect();
    let mut out = AHashMap::new();
    let Some((e, qn_used)) = kern::edit_matrix(ctx.reads, &qn, &seqs) else {
        return out;
    };
    let dump = std::env::var("HLA_DRBDUMP").is_ok();
    let mut cnt: AHashMap<&str, usize> = AHashMap::new();
    for (ri, row) in e.iter().enumerate() {
        let mut o0 = 0;
        for (k, v) in row.iter().enumerate() {
            if *v < row[o0] {
                o0 = k;
            }
        }
        if row[o0] > 3.0 {
            continue;
        }
        let gb = owner[o0];
        let om = row
            .iter()
            .zip(owner.iter())
            .filter(|(_, o)| **o != gb)
            .map(|(v, _)| *v)
            .fold(f64::INFINITY, f64::min);
        if om.is_finite() && om < row[o0] + 3.0 {
            continue;
        }
        *cnt.entry(gb).or_default() += 1;
        if dump {
            eprintln!(
                "DRBUNIQ\t{}\t{}\t{}\t{}",
                gb,
                ctx.reads.names[qn_used[ri]],
                row[o0],
                ctx.reads.seq[qn_used[ri]]
                    .iter()
                    .filter(|x| x.is_some())
                    .count()
            );
        }
    }
    let mut lens: AHashMap<&str, f64> = AHashMap::new();
    for (x, g) in seqs.iter().zip(owner.iter()) {
        let e = lens.entry(g).or_default();
        *e = e.max(x.len() as f64);
    }
    let c = |g: &str| *cnt.get(g).unwrap_or(&0) as f64;
    let per_copy = c("DRB1") / lens.get("DRB1").copied().unwrap_or(0.0).max(1.0) / 2.0;
    for g in ["DRB3", "DRB4", "DRB5"] {
        let l = lens.get(g).copied().unwrap_or(0.0);
        let d = if l > 0.0 { c(g) / l.max(1.0) } else { 0.0 };
        let ratio = if per_copy > 0.0 { d / per_copy } else { 0.0 };
        out.insert(g.to_string(), ratio);
        log(format!(
            "{} joint-DRB unique reads {} ratio per DRB1 copy {:.3}",
            g,
            c(g),
            ratio
        ));
    }
    out
}
