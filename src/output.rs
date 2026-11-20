//! .tsv / .conf.tsv exactly as the prototype, plus a JSON report for the Genome app
use crate::names::{diff_level, fields, two};
use crate::refine::RMarg;
use crate::stageb::{Ctx, StageB};
use ahash::AHashMap;
use std::io::Write;

const T: [f64; 3] = [6.0, 6.0, 4.0];

/// Highest resolution that validation against assembly truth supports per gene. DMB and TAP2 stay
/// at three fields: on 80 HPRC samples their fourth field equalled the assembly in only 88 of 106
/// and 38 of 44 alleles that would have been reported certain (the other genes: about 99 of 100),
/// while their three-field calls were exact at exon level (158 of 158, 116 of 122). HLA-E was
/// capped until 1.1 and is not any more.
fn max_res(g: &str) -> usize {
    match g {
        "DMB" | "TAP2" => 3,
        _ => 4,
    }
}

pub fn write(
    prefix: &str,
    genes: &[&str],
    ctx: &Ctx,
    res: &AHashMap<String, (StageB, Option<Vec<String>>, RMarg)>,
    depth: &AHashMap<String, usize>,
    drb: &AHashMap<String, f64>,
    nov: &AHashMap<String, [Vec<crate::novel::Site>; 2]>,
    known: &AHashMap<String, [Option<String>; 2]>,
    grp: &crate::groups::Groups,
    log: &dyn Fn(String),
) {
    let mut fh = std::fs::File::create(format!("{prefix}.tsv")).unwrap();
    let mut cf = std::fs::File::create(format!("{prefix}.conf.tsv")).unwrap();
    writeln!(cf, "gene\tslot\tallele\tsafe_call\tresolution\tmargin2\tmargin3\tmargin4\talt2\talt3\talt4\tnew_sites\tnew_detail").unwrap();
    writeln!(fh, "gene\tallele1\tallele2\tcall1\tcall2\tgap\treads\tcopies").unwrap();
    let mut js: Vec<serde_json::Value> = vec![];
    let mut fasta = String::new();
    // per reported allele: (name, resolution) for the pharmacogenetic summary
    let mut called: Vec<(String, usize)> = vec![];
    for &g in genes {
        // HLA-H and HLA-K sit in a segment that is deleted on some haplotypes (seen on 19 of 80
        // HPRC samples); copies = reads relative to the always-diploid neighbours F, G, V
        // (not E: its introns collect repeat reads, which made the ratio input-dependent).
        // On assembly truth: 1 copy 0.84-1.16, 2 copies 1.72-2.22 after scaling by 0.2.
        let hk_ratio = if g == "H" || g == "K" {
            let refs: Vec<usize> = ["F", "G", "V"].iter().filter_map(|x| res.get(*x).map(|t| t.0.nreads as usize)).collect();
            let own = res.get(g).map(|t| t.0.nreads as usize).unwrap_or(0);
            let tot: usize = refs.iter().sum();
            if refs.len() == 3 && tot > 0 { Some(own as f64 / tot as f64 / 0.2) } else { None }
        } else {
            None
        };
        if let Some(rt) = hk_ratio {
            if rt < 0.4 {
                let n = res.get(g).map(|t| t.0.nreads as usize).unwrap_or_else(|| depth.get(g).copied().unwrap_or(0));
                writeln!(fh, "{g}\t-\t-\t-\t-\t0\t{n}\t0").unwrap();
                // absence is a result, written like a gene absent by the copy model
                js.push(serde_json::json!({"gene": g, "called": true, "copies": 0, "reads": n, "alleles": [], "copy_ratio": rt}));
                continue;
            }
        }
        let Some((r, refined, rm)) = res.get(g) else {
            // no result: the number of copies is unknown, not 0
            writeln!(fh, "{g}\t-\t-\t-\t-\t-\t{}\t-", depth.get(g).copied().unwrap_or(0)).unwrap();
            js.push(serde_json::json!({"gene": g, "called": false, "reads": depth.get(g).copied().unwrap_or(0)}));
            continue;
        };
        let al = ctx.al(g);
        let cand = &r.cand;
        let (i, j) = r.pair;
        let (mut a1, mut a2) = (al[cand[i]].name.clone(), al[cand[j]].name.clone());
        if let Some(n) = refined {
            a1 = n[0].clone();
            a2 = n[1].clone();
        }
        let (mut set1, mut set2) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
        for &(a, b) in &r.ties {
            let (mut x, mut y) = (al[cand[a]].name.clone(), al[cand[b]].name.clone());
            if two(&x) == two(&a2) && two(&y) == two(&a1) && two(&a1) != two(&a2) {
                std::mem::swap(&mut x, &mut y);
            }
            set1.insert(x);
            set2.insert(y);
        }
        let fmt = |s: &std::collections::BTreeSet<String>| s.iter().take(6).map(|x| format!("'{x}'")).collect::<Vec<_>>().join(", ");
        log(format!("{} ties {} [{}] [{}]", g, r.ties.len(), fmt(&set1), fmt(&set2)));
        let mut copies = 2;
        let mut ratio = None;
        if ["DRB3", "DRB4", "DRB5"].contains(&g) {
            let rt = drb.get(g).copied().unwrap_or(0.0);
            ratio = Some(rt);
            copies = if rt < 0.35 { 0 } else if rt >= 1.65 { 2 } else { 1 };
            if copies == 1 {
                a2 = "-".to_string();
            }
        }
        if let Some(rt) = hk_ratio {
            ratio = Some(rt);
            copies = if rt >= 1.45 { 2 } else { 1 };
            if copies == 1 {
                a2 = "-".to_string();
            }
        }
        if copies == 0 {
            // gene absent on both chromosomes: no allele is reported, only the evidence for absence
            writeln!(fh, "{g}\t-\t-\t-\t-\t{:.1}\t{}\t0", r.gap, r.nreads).unwrap();
            let mut o = serde_json::json!({"gene": g, "called": true, "copies": 0, "reads": r.nreads, "gap": r.gap, "alleles": []});
            if let Some(rt) = ratio {
                o["copy_ratio"] = serde_json::json!(rt);
            }
            js.push(o);
            continue;
        }
        // the residual variants of a call turn it exactly into another database allele: that one is reported
        let kn = |sl: usize, n: &String| -> String {
            if n == "-" { return n.clone(); }
            known.get(g).and_then(|k| k[sl].clone()).unwrap_or_else(|| n.clone())
        };
        let (s1, s2) = (kn(0, &a1), kn(1, &a2));
        let mut safes = [String::from("-"), String::from("-")];
        let (b1, b2) = (al[cand[i]].name.clone(), al[cand[j]].name.clone());
        let mut jal = vec![];
        for (sl, (fin, bname)) in [(&a1, &b1), (&a2, &b2)].into_iter().enumerate() {
            if fin == "-" {
                continue;
            }
            let (bm, balt) = &r.bmargin[sl];
            let mut m = *bm;
            let mut alt = balt.clone();
            let lv = diff_level(fin, bname);
            if lv != 9 {
                for (ni, n) in [2u32, 3, 4].iter().enumerate() {
                    if *n >= lv {
                        m[ni] = if *n == 4 { 0.0 } else { 999.0 };
                    }
                }
            }
            if let Some((rmm, ralt)) = rm.get(&(if i != j { sl } else { 0 })) {
                for ni in 0..2 {
                    if rmm[ni] < m[ni] {
                        m[ni] = rmm[ni];
                        alt[ni] = ralt[ni].clone();
                    }
                }
            }
            m[1] = m[1].min(m[0]);
            m[2] = m[2].min(m[1]);
            let shown: &String = if sl == 0 { &s1 } else { &s2 };
            let renamed = shown != fin;
            let f = fields(shown);
            let res_full = if m[2] >= T[2] && m[1] >= T[1] && m[0] >= T[0] {
                4
            } else if m[1] >= T[1] && m[0] >= T[0] {
                3
            } else if m[0] >= T[0] {
                2
            } else {
                1
            };
            // a new variant makes the name the nearest known allele: outside the exons the first three
            // fields still hold, inside an exon only the allele group does
            let ns: Option<&Vec<crate::novel::Site>> = nov.get(g).map(|v| &v[sl]);
            let new_cap = match ns {
                _ if renamed => 4,
                Some(v) if v.iter().any(|s| s.exon) => 1,
                Some(v) if !v.is_empty() => 3,
                _ => 4,
            };
            // resolution never exceeds the gene cap or the number of fields the allele name has
            let res_ = res_full.min(max_res(g)).min(f.len().max(1)).min(new_cap);
            let gene_cap = res_full.min(f.len().max(1)).min(new_cap) > res_;
            let safe = format!("{}*{}", g, f[..res_.min(f.len()).max(1)].join(":"));
            // no competing allele at a level: print "-" / null instead of the internal 999
            let ms = |x: f64| if x >= 999.0 { "-".to_string() } else { format!("{x:.1}") };
            let mj = |x: f64| if x >= 999.0 { serde_json::Value::Null } else { serde_json::json!(x) };
            // positions where the reads consistently differ from this allele: it is then the nearest
            // database allele of a new one (HLA_NONOVEL: column "-")
            let nd = match ns {
                Some(v) => v.iter().take(20).map(|s| format!("{}:{}:{}/{}{}{}", s.pos + 1, s.what, s.alt, s.depth, if s.exon { ":exon" } else { "" }, if s.either { ":either" } else { "" })).collect::<Vec<_>>().join(","),
                None => String::new(),
            };
            let nd = if renamed { format!("known:{fin}+{nd}") } else { nd };
            let nc = if renamed { "0".to_string() } else { ns.map(|v| v.len().to_string()).unwrap_or_else(|| "-".to_string()) };
            writeln!(cf, "{g}\t{}\t{shown}\t{safe}\t{res_}\t{}\t{}\t{}\t{}\t{}\t{}\t{nc}\t{}", sl + 1, ms(m[0]), ms(m[1]), ms(m[2]), alt[0], alt[1], alt[2], if nd.is_empty() { "-" } else { nd.as_str() }).unwrap();
            if !renamed {
                if let (Some(v), Some(a)) = (ns, al.iter().find(|a| a.name == *fin)) {
                    if let Some(rec) = novel_record(g, sl + 1, fin, &a.genomic, v) { fasta.push_str(&rec); }
                }
            }
            called.push((shown.clone(), res_));
            safes[sl] = safe.split('*').nth(1).unwrap_or("-").to_string();
            jal.push(serde_json::json!({
                "allele": shown, "safe_call": safe, "resolution": res_, "gene_cap": gene_cap,
                "g_group": grp.g(shown), "p_group": grp.p(shown),
                "expression": crate::groups::expression(shown),
                "renamed_from": if renamed { serde_json::json!(fin) } else { serde_json::Value::Null },
                "margin": {"2": mj(m[0]), "3": mj(m[1]), "4": mj(m[2])},
                "alternative": {"2": alt[0], "3": alt[1], "4": alt[2]},
                "new_sites": if renamed { Some(vec![]) } else { ns.map(|v| v.iter().map(|s| serde_json::json!({"pos": s.pos + 1, "change": s.what, "reads": s.alt, "depth": s.depth, "exon": s.exon, "either_allele": s.either})).collect::<Vec<_>>()) },
            }));
        }
        // call1/call2: the safe call (the fields the reads support), without the gene prefix
        writeln!(fh, "{g}\t{s1}\t{s2}\t{}\t{}\t{:.1}\t{}\t{copies}", safes[0], safes[1], r.gap, r.nreads).unwrap();
        let mut o = serde_json::json!({"gene": g, "called": true, "copies": copies, "reads": r.nreads, "gap": r.gap, "alleles": jal});
        if let Some(rt) = ratio {
            o["copy_ratio"] = serde_json::json!(rt);
        }
        js.push(o);
    }
    let rep = serde_json::json!({
        "tool": "hla-wgs", "version": env!("CARGO_PKG_VERSION"), "schema": "genome.hla.v1",
        "thresholds": {"2": T[0], "3": T[1], "4": T[2]}, "genes": js,
        "groups_loaded": grp.loaded,
        "repeat_mask": crate::panel::MASK_LOADED.load(std::sync::atomic::Ordering::Relaxed),
        "pharmacogenetic_alleles": crate::groups::PGX.iter().map(|(al, drugs)| {
            let hits: Vec<&(String, usize)> = called.iter().filter(|(n, _)| two(n) == *al).collect();
            serde_json::json!({
                "allele": al, "drugs": drugs, "source": "CPIC guideline",
                "copies": hits.len(),
                // the two-field statement holds when every carrying call is certain at two fields or more
                "certain": if hits.is_empty() { let v: Vec<&(String, usize)> = called.iter().filter(|(n, _)| n.split('*').next() == al.split('*').next()).collect(); !v.is_empty() && v.iter().all(|(_, r)| *r >= 2) } else { hits.iter().all(|(_, r)| *r >= 2) },
            })
        }).collect::<Vec<_>>(),
    });
    std::fs::write(format!("{prefix}.json"), serde_json::to_string_pretty(&rep).unwrap()).unwrap();
    let sample = std::path::Path::new(prefix).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    std::fs::write(format!("{prefix}.html"), crate::html::render(&rep, &sample)).unwrap();
    if !fasta.is_empty() {
        std::fs::write(format!("{prefix}.novel.fa"), fasta).unwrap();
    }
}

/// Sequence of a new allele: the called database allele with its residual substitutions, insertions
/// and deletions applied. The header names the nearest allele and the applied changes, and says when
/// the sequence is incomplete (blind stretches, unresolved repeat lengths) or when a change could
/// belong to either allele.
fn novel_record(g: &str, slot: usize, name: &str, genomic: &[u8], sites: &[crate::novel::Site]) -> Option<String> {
    let mut seq = genomic.to_vec();
    let mut applied = vec![];
    let mut open = vec![];
    let mut either = false;
    let mut v: Vec<&crate::novel::Site> = sites.iter().collect();
    v.sort_by_key(|s| std::cmp::Reverse(s.pos));
    for s in v {
        let w = s.what.trim_end_matches("(unsupported)");
        let p = s.pos;
        let ok = if let Some((r, a)) = w.split_once('>') {
            r.len() == 1 && a.len() == 1 && p < seq.len() && seq[p] == r.as_bytes()[0] && { seq[p] = a.as_bytes()[0]; true }
        } else if let Some(d) = w.strip_prefix("del") {
            p + d.len() <= seq.len() && &seq[p..p + d.len()] == d.as_bytes() && { seq.drain(p..p + d.len()); true }
        } else if let Some(i) = w.strip_prefix("ins") {
            !i.is_empty() && i.bytes().all(|c| b"ACGT".contains(&c)) && p <= seq.len() && { seq.splice(p..p, i.bytes()); true }
        } else {
            false
        };
        if ok {
            applied.push(format!("{}:{}", p + 1, w));
            either |= s.either;
        } else {
            open.push(format!("{}:{}", p + 1, s.what));
        }
    }
    if applied.is_empty() {
        return None;
    }
    applied.reverse();
    open.reverse();
    let mut h = format!(">{g}_slot{slot}_new nearest={name} changes={}", applied.join(","));
    if !open.is_empty() {
        h.push_str(&format!(" incomplete={}", open.join(",")));
    }
    if either {
        h.push_str(" phase=uncertain");
    }
    let mut out = h + "\n";
    for c in seq.chunks(70) {
        out.push_str(std::str::from_utf8(c).unwrap_or(""));
        out.push('\n');
    }
    Some(out)
}
