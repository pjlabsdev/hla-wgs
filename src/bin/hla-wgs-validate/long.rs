//! Long-read (HiFi) evaluation and the certainty count by resolution used for the depth study.
//! Run in the work folder (truth/, db/). Long-read results are read as DIR/SAMPLE.long.tsv and
//! DIR/SAMPLE.long.fa, or DIR/SAMPLE.TAG.long.* when HLA_LTAG is set.
use crate::aln::Opts;
use crate::common::*;
use std::collections::{BTreeSet, HashMap};

/// truth4.tsv: (sample, gene) -> [(distance of the nearest allele, allele names)]
fn truth4() -> HashMap<(String, String), Vec<(String, Vec<String>)>> {
    let mut t: HashMap<(String, String), Vec<(String, Vec<String>)>> = HashMap::new();
    for l in read(&env_or("T4OUT", "truth/truth4.tsv")).lines() {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() >= 5 {
            t.entry((f[0].to_string(), f[2].to_string()))
                .or_default()
                .push((f[3].to_string(), f[4].split(',').map(|x| x.to_string()).collect()));
        }
    }
    t
}

fn fields(a: &str, k: usize) -> String {
    a.split(':').take(k).collect::<Vec<_>>().join(":")
}

/// long DIR SAMPLES.txt [v]: consensus sequences against both assembled haplotypes (edit distance
/// over the whole sequence, unaligned ends count), haplotypes reproduced exactly, exact calls
/// against the truth allele names and nearest-allele calls right at two fields
pub fn long(dir: &str, list: &str, verbose: bool) {
    let tag = std::env::var("HLA_LTAG").map(|t| format!(".{t}")).unwrap_or_default();
    let t4 = truth4();
    let mut cons: HashMap<String, (i64, i64)> = HashMap::new();
    let mut bad_cons: Vec<String> = vec![];
    let (mut hit, mut near) = (BTreeSet::new(), BTreeSet::new());
    let mut exact: HashMap<String, (i64, i64)> = HashMap::new();
    let mut bad_exact: Vec<String> = vec![];
    let (mut nn, mut nok) = (0, 0);
    for smp in read(list).split_whitespace() {
        let fa = format!("{dir}/{smp}{tag}.long.fa");
        let tsv = format!("{dir}/{smp}{tag}.long.tsv");
        if !exists(&fa) || !exists(&tsv) {
            continue;
        }
        let seqs = fasta(&fa);
        let mut best: HashMap<String, (i64, String)> = HashMap::new();
        for h in ["h1", "h2", "mat", "pat"] {
            let af = format!("truth/{smp}.{h}.mhc.fa");
            if !exists(&af) {
                continue;
            }
            let ix = index_fasta(&af, Opts::asm5());
            for (name, q) in &seqs {
                for p in map_primary(&ix, q, name) {
                    let d = (p.nm + p.qs + p.qlen - p.qe) as i64;
                    let g = name.split("_slot").next().unwrap_or("").to_string();
                    if d == 0 {
                        hit.insert((smp.to_string(), g.clone(), h));
                    }
                    if d <= 2 {
                        near.insert((smp.to_string(), g.clone(), h));
                    }
                    if best.get(name).map_or(true, |b| d < b.0) {
                        best.insert(name.clone(), (d, h.to_string()));
                    }
                }
            }
        }
        let mut names: Vec<&String> = best.keys().collect();
        names.sort();
        for n in names {
            let (d, h) = &best[n];
            let g = n.split("_slot").next().unwrap_or("").to_string();
            let c = cons.entry(g.clone()).or_default();
            c.1 += 1;
            c.0 += (*d == 0) as i64;
            if *d > 0 {
                bad_cons.push(format!("{smp} {g} vs_assembly {d} {h}"));
            }
        }
        for l in read(&tsv).lines().skip(1) {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 10 {
                continue;
            }
            let Some(tr) = t4.get(&(smp.to_string(), f[0].to_string())) else { continue };
            if f[4].starts_with("yes") {
                let e = exact.entry(f[0].to_string()).or_default();
                e.1 += 1;
                let ok = tr.iter().any(|(_, t)| t.iter().any(|x| x == f[2]));
                e.0 += ok as i64;
                if !ok {
                    bad_exact.push(format!("{smp} {} {}", f[0], f[2]));
                }
            } else if f[9] != "minor" {
                nn += 1;
                nok += tr.iter().any(|(_, t)| t.iter().any(|x| fields(x, 2) == fields(f[2], 2))) as i64;
            }
        }
    }
    let line = |m: &HashMap<String, (i64, i64)>| {
        let mut gs: Vec<&String> = m.keys().collect();
        gs.sort();
        let (a, b) = m.values().fold((0, 0), |s, v| (s.0 + v.0, s.1 + v.1));
        (gs.iter().map(|g| format!("{g}:{}/{}", m[*g].0, m[*g].1)).collect::<Vec<_>>().join(" "), a, b)
    };
    let (l, a, b) = line(&cons);
    println!("consensus exact vs assembly: {l}\nconsensus ALL {a} / {b}");
    println!("assembly haplotypes reproduced: exact {} within2 {}", hit.len(), near.len());
    let (l, a, b) = line(&exact);
    println!("exact calls right: {l}\nexact calls ALL {a} / {b}");
    println!("nearest-allele calls right at 2 fields {nok} / {nn}");
    if verbose {
        for x in bad_cons.iter().chain(bad_exact.iter()) {
            println!("{x}");
        }
    }
}

/// eval-res DIR: certain calls right / all at two and three fields (truth4 allele names)
pub fn eval_res(dir: &str) {
    let t4 = truth4();
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    files.retain(|f| f.ends_with(".conf.tsv"));
    files.sort();
    let mut c = [(0i64, 0i64); 2];
    for cf in &files {
        let smp = &cf[..cf.len() - 9];
        for l in read(&format!("{dir}/{cf}")).lines() {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 5 || f[0] == "gene" {
                continue;
            }
            let Some(tr) = t4.get(&(smp.to_string(), f[0].to_string())) else { continue };
            let Ok(res) = f[4].parse::<usize>() else { continue };
            for (i, k) in [2usize, 3].into_iter().enumerate() {
                if res >= k {
                    let n = fields(f[3], k);
                    let ok = tr.iter().any(|(_, t)| t.iter().any(|x| fields(x, k) == n));
                    c[i].0 += ok as i64;
                    c[i].1 += 1;
                }
            }
        }
    }
    println!("{dir} 2:{}/{} 3:{}/{}", c[0].0, c[0].1, c[1].0, c[1].1);
}
