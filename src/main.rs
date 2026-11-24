mod align;
mod aln;
mod bamin;
mod builddb;
mod db;
mod groups;
mod kern;
mod prior;
mod refine;
mod stageb;
mod names;
mod html;
mod long;
mod mask;
mod novel;
mod panel;
mod stagea;
mod output;
mod util;
mod unzip;
mod errors;
mod runlog;
mod vfs;
#[cfg(target_arch = "wasm32")]
mod web;

use ahash::{AHashMap, AHashSet};
use names::{three, two};
use vfs::Instant;
use rayon::prelude::*;

pub const FOCUS: [&str; 26] = ["A", "B", "C", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1", "DRA", "E", "F", "G", "H", "K", "V", "DMA", "DMB", "DOA", "DOB", "MICA", "MICB", "TAP1", "TAP2"];

/// DRB1 exon refinement with the reads of DRB3/4/5 (HLA_DRBREL=0: off)
pub fn drb_rel() -> bool {
    std::env::var("HLA_DRBREL").map_or(true, |v| v != "0")
}

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

/// everything that can be known before the first read is checked here, so that a wrong path
/// or a read-only folder is reported at once and not after minutes of reading
fn preflight(db: &str, panel: &str, input: &panel::Input, out: &str) {
    // codes: not found, empty, folder
    let need_c = |p: &str, what: &str, c: [&str; 3]| match vfs::file_len(p) {
        Err(e) => errors::fail(c[0], &format!("{what} not found: {p} ({e})")),
        Ok(Some(0)) => errors::fail(c[1], &format!("{what} is empty: {p}")),
        Ok(None) => errors::fail(c[2], &format!("{what} is a folder, not a file: {p}")),
        Ok(_) => {}
    };
    let need = |p: &str, what: &str| need_c(p, what, ["HW-E010", "HW-E011", "HW-E012"]);
    need_c(db, "database file", ["HW-E002", "HW-E003", "HW-E003"]);
    need_c(panel, "database panel", ["HW-E002", "HW-E003", "HW-E003"]);
    match input {
        panel::Input::Fastq(r1, r2) => {
            need(r1, "FASTQ R1");
            need(r2, "FASTQ R2");
        }
        panel::Input::Aligned(p, reference) => {
            need(p, "input file");
            let l = p.to_ascii_lowercase();
            let cands: Vec<String> = if l.ends_with(".cram") {
                vec![format!("{p}.crai")]
            } else {
                vec![format!("{p}.bai"), format!("{p}.csi"), format!("{}.bai", &p[..p.len() - 4])]
            };
            if !cands.iter().any(|c| vfs::is_file(c)) {
                errors::fail("HW-E013", &format!("index not found for {p}: expected {} (create it with: samtools index)", cands.join(" or ")));
            }
            if let Some(r) = reference {
                need_c(r, "reference FASTA", ["HW-E017", "HW-E011", "HW-E012"]);
                need_c(&format!("{r}.fai"), "reference FASTA index", ["HW-E017", "HW-E011", "HW-E012"]);
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = out;
    #[cfg(not(target_arch = "wasm32"))]
    {
    let probe = format!("{out}.tsv");
    let existed = std::path::Path::new(&probe).exists();
    match std::fs::OpenOptions::new().append(true).create(true).open(&probe) {
        Ok(_) => {
            if !existed {
                let _ = std::fs::remove_file(&probe);
            }
        }
        Err(e) => errors::fail("HW-E020", &format!("cannot write results to {probe}: {e}")),
    }
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn main() {
    cli(std::env::args().collect());
}

/// the whole program for an argument list as on the command line (also used by the browser build)
pub fn cli(mut a: Vec<String>) {
    errors::install_panic_hook();
    // --trace GENE[,GENE]: follow these genes in detail in the run log (also HLA_TRACE)
    let mut trace: Vec<String> = std::env::var("HLA_TRACE").map(|v| v.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default();
    while let Some(i) = a.iter().position(|x| x == "--trace") {
        if i + 1 >= a.len() {
            errors::fail("HW-E001", "--trace needs a gene, e.g. --trace A or --trace A,DRB1");
        }
        trace.extend(a[i + 1].split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()));
        a.drain(i..i + 2);
    }
    runlog::set_trace(trace.clone());
    match a.get(1).map(|x| x.as_str()) {
        Some("diag") => return runlog::diag(&a[2..]),
        Some("build-db") => return builddb::run(&a[2..]),
        Some("build-mask") => return mask::run(&a[2..]),
        Some("long") => return long::run(&a[2..]),
        Some("unzip") => return unzip::run(&a[2..]),
        // the result page again from PREFIX.json, e.g. after an update of the page
        Some("html") => {
            let Some(j) = a.get(2) else { errors::fail("HW-E001", "usage: hla-wgs html PREFIX.json") };
            let rep: serde_json::Value = serde_json::from_str(&vfs::read_to_string(j).unwrap_or_else(|e| errors::fail("HW-E021", &format!("cannot read {j}: {e}")))).unwrap_or_else(|e| errors::fail("HW-E021", &format!("{j} is not a result file: {e}")));
            let prefix = j.trim_end_matches(".json");
            let sample = std::path::Path::new(prefix).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            vfs::write(&format!("{prefix}.html"), html::render(&rep, &sample).as_bytes()).unwrap_or_else(|e| errors::fail("HW-E020", &format!("cannot write {prefix}.html: {e}")));
            return;
        }
        Some("--version") | Some("-V") => return println!("hla-wgs {}", env!("CARGO_PKG_VERSION")),
        Some("--db-format") => return println!("{}", builddb::DB_FORMAT),
        _ => {}
    }
    if a.len() >= 4 && !vfs::exists(&a[1]) {
        errors::fail("HW-E002", &format!("database not found: {} (build it with: hla-wgs build-db hla.dat freq.json DBDIR)", a[1]));
    }
    // a database directory stands for its two files
    let mut db_format: Option<u32> = None;
    if a.get(1).map_or(false, |d| vfs::is_dir(d)) {
        let d = a[1].trim_end_matches('/').to_string();
        let f: u32 = vfs::read_to_string(&format!("{d}/format")).ok().and_then(|x| x.trim().parse().ok()).unwrap_or(1);
        // release of the database: DBDIR/release (written by build-db since 1.1), else the header of
        // the WMDA G group file of the same release ("# version: IPD-IMGT/HLA 3.66.0")
        let rel = vfs::read_to_string(&format!("{d}/release")).ok().map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).or_else(|| {
            vfs::read_to_string(&format!("{d}/hla_nom_g.txt")).ok().and_then(|t| {
                t.lines().take(10).find_map(|l| l.strip_prefix("# version:").map(|v| v.trim().trim_start_matches("IPD-IMGT/HLA").trim().to_string()))
            })
        });
        if let Some(r) = rel {
            let _ = output::DB_RELEASE.set(r);
        }
        db_format = Some(f);
        if f < builddb::DB_FORMAT {
            eprintln!("[db] {d} was built by an older version (format {f}, current {}): results for DMB, DOB, MICA, MICB, TAP1 and TAP2 are better after rebuilding it with build-db", builddb::DB_FORMAT);
        }
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
        eprintln!("       hla-wgs long DBDIR READS.fastq[.gz]|READS.bam OUTPREFIX [threads]   (PacBio HiFi)");
        eprintln!("       hla-wgs build-db hla.dat freq.json DBDIR");
        eprintln!("       hla-wgs build-mask DBDIR GENOME.fa [EXCLUDE.bed]");
        eprintln!("       hla-wgs diag OUTPREFIX   (run log and version as OUTPREFIX.diag.tar for a bug report)");
        eprintln!("       (DBDIR may also be given as its two files: hladb.json.gz panel.fa; --trace GENE logs one gene in detail)");
        errors::fail("HW-E001", "too few arguments");
    }
    let (input, out, ti) = if aligned {
        (panel::Input::Aligned(a[3].clone(), a.get(6).cloned()), a[4].clone(), 5)
    } else {
        (panel::Input::Fastq(a[3].clone(), a[4].clone()), a[5].clone(), 6)
    };
    // the run log starts first, so that a failed check is in it too (when the folder is writable)
    let _ = runlog::start(&out);
    preflight(&a[1], &a[2], &input, &out);
    // file names only (no folders) for the result page
    let base = |x: &String| std::path::Path::new(x).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let _ = output::INPUTS.set(if aligned { vec![base(&a[3])] } else { vec![base(&a[3]), base(&a[4])] });
    let threads: usize = a.get(ti).and_then(|x| x.parse().ok()).unwrap_or(4);
    // the browser build has no threads: rayon then runs everything on the calling thread
    #[cfg(not(target_arch = "wasm32"))]
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().ok();
    // run log next to the results (PREFIX.log.jsonl, PREFIX.log.txt), opened before the checks
    if !runlog::active() {
        errors::fail("HW-E020", &format!("cannot write the run log {out}.log.jsonl"));
    }
    {
        let mut inputs: Vec<serde_json::Value> = if aligned { vec![runlog::file_info(&a[3])] } else { vec![runlog::file_info(&a[3]), runlog::file_info(&a[4])] };
        if let Some(r) = a.get(6).filter(|_| aligned) {
            inputs.push(runlog::file_info(r));
        }
        let genes_env = std::env::var("GENES").ok();
        runlog::header(
            "short reads",
            &a,
            serde_json::json!({"threads": threads, "input_type": if aligned { "BAM/CRAM" } else { "paired FASTQ" }, "genes": genes_env.unwrap_or_else(|| FOCUS.join(",")), "trace": trace,
                "thresholds": {"two_three_fields": output::t_group(), "four_fields": output::T, "fourth_field_01": std::env::var("HLA_F401").map_or(true, |v| v != "0")}}),
            inputs,
            serde_json::json!({"database": runlog::file_info(&a[1]), "panel": runlog::file_info(&a[2]), "release": output::DB_RELEASE.get(), "format": db_format, "format_current": builddb::DB_FORMAT}),
        );
    }
    if db_format.map_or(false, |f| f < builddb::DB_FORMAT) {
        runlog::warn("database", &format!("built by an older version (format {}, current {}): results for DMB, DOB, MICA, MICB, TAP1 and TAP2 are better after rebuilding it with build-db", db_format.unwrap_or(0), builddb::DB_FORMAT));
    }
    let t0 = Instant::now();
    let log = |s: String| {
        vfs::log(&format!("[{:6.1}s] {}", t0.elapsed().as_secs_f64(), s));
        runlog::note(s.split_whitespace().next().unwrap_or(""), &s);
    };
    let st = runlog::begin("read_and_map");
    let (p, db) = rayon::join(|| panel::run(&a[2], &input, threads), || db::Db::load(&a[1]));
    let reads = &p.reads;
    {
        let ps = &p.stats;
        let assigned: usize = {
            let mut set = AHashSet::new();
            for v in p.assign.values() {
                set.extend(v.iter().copied());
            }
            set.len()
        };
        let per_gene: std::collections::BTreeMap<&str, usize> = p.assign.iter().map(|(g, v)| (g.as_str(), v.len())).collect();
        // pairs in = no shared 21-mer + no alignment + repeat mask + mate rule + mapped; mapped = assigned + below the score
        st.end(serde_json::json!({"pairs_in": ps.pairs_in, "dropped_no_shared_21mer": ps.no_kmer, "dropped_no_alignment": ps.no_hit, "dropped_repeat_mask": ps.in_mask, "dropped_mate_rule": ps.mate_rule, "mapped": reads.names.len(),
            "dropped_below_score": reads.names.len() - assigned, "assigned_pairs": assigned, "assigned_per_gene": per_gene, "database_genes": db.genes.len()}));
    }
    log(format!("panel mapped reads: {}", reads.seq.iter().filter(|r| r[0].is_some() || r[1].is_some()).count()));
    if std::env::var("HLA_ALNSTAT").is_ok() { eprintln!("[aln panel] {}", aln::stat()); }
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
        Ok(v) => FOCUS.iter().copied().filter(|g| v.split(',').any(|x| x == *g)).collect(),
        Err(_) => FOCUS.to_vec(),
    };
    let empty = vec![];
    let ctx = stageb::Ctx::new(&db, reads);
    ctx.load_cache(&a[1]);
    let mut res: AHashMap<String, (stageb::StageB, Option<Vec<String>>, refine::RMarg)> = AHashMap::new();
    let mut depth: AHashMap<String, usize> = AHashMap::new();
    type GeneRes = (stageb::StageB, Option<Vec<String>>, refine::RMarg);
    type Nov = [Vec<novel::Site>; 2];
    // the genes with the most reads first, so that a slow gene does not start last and leave the
    // other threads idle (results do not depend on the order)
    let mut order: Vec<&str> = genes.clone();
    order.sort_by_key(|g| std::cmp::Reverse(p.assign.get(*g).map_or(0, |v| v.len())));
    let st_typing = runlog::begin("typing");
    let ndone = std::sync::atomic::AtomicUsize::new(0);
    let ntot = order.len();
    let t_typing = runlog::secs();
    let outs: Vec<(String, usize, Option<GeneRes>, Option<Nov>)> = order.iter().par_bridge().map(|g| {
        let gst = runlog::begin(&format!("gene {g}"));
        let ndone = &ndone;
        let tick = move |counts: serde_json::Value| {
            gst.end(counts);
            runlog::progress("typing", ndone.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1, ntot, t_typing);
        };
        let dg: usize;
        let qn_a: &Vec<usize> = p.assign.get(*g).unwrap_or(&empty);
        let mut qn: Vec<usize> = qn_a.clone();
        if !related(g).is_empty() {
            let mut set: std::collections::BTreeSet<&str> = qn_a.iter().map(|&q| reads.names[q].as_str()).collect();
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
            runlog::decision("typing", g, "not typed", "fewer than 20 read pairs for the gene and its relatives", serde_json::json!({"pairs": qn.len(), "minimum": 20}));
            tick(serde_json::json!({"pairs": qn.len(), "typed": false}));
            return (g.to_string(), dg, None, None);
        }
        let al = &db.ref_[*g];
        if std::env::var("HLA_TDBG").is_ok() { log(format!("{g} stagea start")); }
        let idx = stagea::build_index(al);
        let sa = stagea::run(g, al, &idx, reads, qn_a);
        log(sa.log.clone());
        log(format!("{} reads {} candidates {}", g, qn.len(), sa.cand.len()));
        if std::env::var("DUMPA").is_ok() {
            println!("{}\t{}", g, sa.cand.iter().map(|&i| al[i].name.as_str()).collect::<Vec<_>>().join(","));
        }
        let cand = sa.cand;
        runlog::trace(g, "stage A candidates", serde_json::json!(cand.iter().map(|&i| al[i].name.as_str()).collect::<Vec<_>>()));
        if cand.is_empty() {
            runlog::decision("typing", g, "not typed", "no database allele explains the reads (stage A found no candidate)", serde_json::json!({"pairs": qn.len()}));
            tick(serde_json::json!({"pairs": qn.len(), "candidates": 0, "typed": false}));
            return (g.to_string(), dg, None, None);
        }
        let ncand1 = cand.len();
        let mut ncand2 = 0usize;
        let tdbg = std::env::var("HLA_TDBG").is_ok();
        if tdbg { log(format!("{g} stageb1 start")); }
        let mut r1 = ctx.stage_b(g, &qn, &cand);
        if tdbg { log(format!("{g} stageb1 end")); }
        if let Some(r) = &r1 {
            // pass 2: expand the exon classes of the best pair(s) to all members
            let mut win: AHashSet<usize> = r.ties.iter().take(6).flat_map(|p| [cand[p.0], cand[p.1]]).collect();
            let byname: AHashMap<&str, usize> = al.iter().enumerate().map(|(i, a)| (a.name.as_str(), i)).collect();
            let par: Vec<usize> = win.iter().filter_map(|&i| al[i].imputed_from.as_deref().and_then(|n| byname.get(n).copied())).collect();
            win.extend(par);
            let mut ws: Vec<usize> = win.iter().copied().collect();
            ws.sort();
            let mut f2l: Vec<String> = ws.iter().map(|&i| two(&al[i].name)).collect();
            let nwin = f2l.len();
            let first_set: AHashSet<&String> = f2l[..nwin].iter().collect();
            let lim = first_set.len().max(6);
            f2l.extend(r.near.iter().flat_map(|p| [two(&al[cand[p.0]].name), two(&al[cand[p.1]].name)]));
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
            let mut c2: std::collections::BTreeSet<usize> = pri.into_iter().chain(sec).take(400).collect();
            c2.extend(ws.iter().copied());
            let cand2: Vec<usize> = c2.into_iter().collect();
            if let Some(x) = ctx.stage_b(g, &qn, &cand2) {
                r1 = Some(x);
            }
            log(format!("{} pass2 candidates {}", g, cand2.len()));
            ncand2 = cand2.len();
            if tdbg { log(format!("{g} pad {}", stageb::Ctx::pad_stat())); }
        }
        if let Some(r) = r1 {
            let n = |k: usize| al[r.cand[k]].name.as_str();
            if std::env::var("DUMPB").is_ok() {
                println!("{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}", g, n(r.pair.0), n(r.pair.1), r.ll, r.gap, r.nreads, r.ties.len());
            }
            if tdbg { log(format!("{g} refine start")); }
            let mut rm = refine::RMarg::new();
            // The panel gives some DRB1 reads to DRB3/4/5, also when those genes are absent: the exon-2
            // reads that tell DRB1*09:01 from its siblings went to DRB5, the DRB1*01:01 reads of
            // 01:01/07:01 carriers to DRB3 and DRB5. The exon refinement of DRB1 therefore also sees
            // the reads of DRB3/4/5. Only there: in stage B and the new-variant search they moved
            // fourth fields and added false sites (80 HPRC: 5 fourth fields wrong). HLA_DRBREL=0: off.
            let qn_ex: Vec<usize> = if *g == "DRB1" && drb_rel() {
                let mut set: std::collections::BTreeSet<usize> = qn.iter().copied().collect();
                for x in ["DRB3", "DRB4", "DRB5"] {
                    set.extend(p.assign.get(x).unwrap_or(&empty).iter().copied());
                }
                set.into_iter().collect()
            } else {
                qn.clone()
            };
            let names = refine::exon_refine(&ctx, g, &qn_ex, &r, &mut rm, &log);
            let fin: [String; 2] = match &names {
                Some(v) => [v[0].clone(), v[1].clone()],
                None => [al[r.cand[r.pair.0]].name.clone(), al[r.cand[r.pair.1]].name.clone()],
            };
            if tdbg { log(format!("{g} novel start")); }
            let nv = if std::env::var("HLA_NONOVEL").is_ok() { None } else { Some(novel::sites(&ctx, g, &qn, [fin[0].as_str(), fin[1].as_str()])) };
            if let Some(n) = &nv {
                for (k, v) in n.iter().enumerate() {
                    if !v.is_empty() {
                        log(format!("{} novel slot{} {} sites {}", g, k + 1, v.len(), v.iter().take(8).map(|s| format!("{}:{}:{}/{}{}", s.pos, s.what, s.alt, s.depth, if s.exon { ":exon" } else { "" })).collect::<Vec<_>>().join(" ")));
                    }
                }
            }
            log(format!("{} done", g));
            runlog::trace(g, "stage B best pair and ties", serde_json::json!({"pair": [n(r.pair.0), n(r.pair.1)], "log_likelihood": r.ll, "gap": r.gap, "reads_scored": r.nreads,
                "ties": r.ties.iter().take(10).map(|p| [n(p.0), n(p.1)]).collect::<Vec<_>>(), "final": names}));
            tick(serde_json::json!({"pairs": qn.len(), "candidates_stage_a": ncand1, "candidates_pass2": ncand2, "reads_scored": r.nreads, "typed": true,
                "new_sites": nv.as_ref().map(|n| [n[0].len(), n[1].len()])}));
            return (g.to_string(), dg, Some((r, names, rm)), nv);
        }
        log(format!("{} done", g));
        runlog::decision("typing", g, "not typed", "stage B found no allele pair", serde_json::json!({"pairs": qn.len(), "candidates": ncand1}));
        tick(serde_json::json!({"pairs": qn.len(), "candidates_stage_a": ncand1, "typed": false}));
        (g.to_string(), dg, None, None)
    }).collect();
    st_typing.end(serde_json::json!({"genes": ntot, "typed": outs.iter().filter(|o| o.2.is_some()).count()}));
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
                        None => [al[r.cand[r.pair.0]].name.clone(), al[r.cand[r.pair.1]].name.clone()],
                    };
                    let k = [novel::db_match(&ctx, &g, &fin[0], &x[0]), novel::db_match(&ctx, &g, &fin[1], &x[1])];
                    for (i, m) in k.iter().enumerate() {
                        if let Some(m) = m {
                            log(format!("{} slot{} {} plus its residual variants is {}", g, i + 1, fin[i], m));
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

    ctx.save_cache(&a[1]);
    let st = runlog::begin("drb_copies");
    let drb = drb_copies(&ctx, &res, &p.assign, &log);
    st.end(serde_json::json!(drb.iter().map(|(g, v)| (g.clone(), (v * 1000.0).round() / 1000.0)).collect::<std::collections::BTreeMap<_, _>>()));
    let grp = groups::Groups::load(std::path::Path::new(&a[1]).parent().unwrap_or(std::path::Path::new(".")));
    let st = runlog::begin("write_results");
    output::write(&out, &genes, &ctx, &res, &depth, &drb, &nov, &known, &grp, &log);
    let mut outputs: Vec<String> = ["tsv", "conf.tsv", "json", "html", "novel.fa", "novel.tsv"].iter().map(|e| format!("{out}.{e}")).filter(|p| std::path::Path::new(p).exists()).collect();
    outputs.iter_mut().for_each(|p| *p = std::path::Path::new(p.as_str()).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    st.end(serde_json::json!({"files": outputs.len()}));
    if std::env::var("HLA_TDBG").is_ok() { log(format!("pad total {}", stageb::Ctx::pad_stat())); }
    log("total".to_string());
    if std::env::var("HLA_ALNSTAT").is_ok() { eprintln!("[aln] {}", aln::stat()); }
    runlog::finish(true, outputs);
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
    gl.extend(ctx.db.genes.iter().map(|x| x.as_str()).filter(|x| x.starts_with("DRB")));
    let mut qs: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for g in gl {
        for &q in assign.get(g).map(|v| v.as_slice()).unwrap_or(&[]) {
            qs.insert(ctx.reads.names[q].as_str(), q);
        }
    }
    let qn: Vec<usize> = qs.values().copied().collect();
    let mut out = AHashMap::new();
    let Some((e, qn_used)) = kern::edit_matrix(ctx.reads, &qn, &seqs) else { return out };
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
        let om = row.iter().zip(owner.iter()).filter(|(_, o)| **o != gb).map(|(v, _)| *v).fold(f64::INFINITY, f64::min);
        if om.is_finite() && om < row[o0] + 3.0 {
            continue;
        }
        *cnt.entry(gb).or_default() += 1;
        if dump {
            eprintln!("DRBUNIQ\t{}\t{}\t{}\t{}", gb, ctx.reads.names[qn_used[ri]], row[o0], ctx.reads.seq[qn_used[ri]].iter().filter(|x| x.is_some()).count());
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
        log(format!("{} joint-DRB unique reads {} ratio per DRB1 copy {:.3}", g, c(g), ratio));
    }
    out
}
