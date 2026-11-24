//! .tsv / .conf.tsv exactly as the prototype, plus a JSON report for the Genome app
use crate::names::{diff_level, fields, two};
use crate::refine::RMarg;
use crate::stageb::{Ctx, StageB};
use ahash::AHashMap;
use std::io::Write;

const T: [f64; 3] = [6.0, 6.0, 4.0];

/// lead that makes two or three fields certain when the fourth is not. At 10x the leads shrink with
/// the reads, and most correct calls stopped at fewer fields than they deserved: on 80 HPRC samples
/// thinned to 10x a lead of 4 adds about 200 certain two-field and 50 three-field calls, none wrong.
/// A four-field call keeps the stricter leads of `T`. HLA_TGRP=6 restores the old value.
fn t_group() -> f64 {
    std::env::var("HLA_TGRP").ok().and_then(|v| v.parse().ok()).unwrap_or(4.0)
}

/// IPD-IMGT/HLA release of the database (file DBDIR/release, written by build-db)
pub static DB_RELEASE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
/// names of the input files, without folders
pub static INPUTS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

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
    let mut sheet = String::new();
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
            // DRB1: a competitor that differs in the first two fields differs in the exons, and the exon
            // refinement judges it with the reads that the panel gave to DRB3/4/5 (main.rs). Stage B
            // without them saw 01:01/07:01 as nearly homozygous 07:01 (lead 0.9, group withheld).
            // 80 HPRC: 13 DRB1 calls one or two fields deeper, all right, nothing else changed.
            if g == "DRB1" && crate::drb_rel() && rm.contains_key(&(if i != j { sl } else { 0 })) {
                for ni in 0..2 {
                    if !alt[ni].is_empty() && diff_level(&alt[ni], fin) <= 2 {
                        m[ni] = 999.0;
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
            // The first-described allele of a three-field group (fourth field 01) is mostly the common one;
            // against a rarer sibling a lead of 1 at field 4 is enough. 80 HPRC samples: 38 more exact
            // four-field calls, 2 wrong; 59 unseen HGSVC3 samples: 22 more exact, 1 wrong. HLA_F401=0: off.
            let f401 = std::env::var("HLA_F401").map_or(true, |v| v != "0") && fields(shown).get(3).map_or(false, |x| x == "01") && m[2] >= 1.0;
            let res_full = if (m[2] >= T[2] || f401) && m[1] >= T[1] && m[0] >= T[0] {
                4
            } else if m[1] >= t_group() && m[0] >= t_group() {
                3
            } else if m[0] >= t_group() {
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
            let mut res_ = res_full.min(max_res(g)).min(f.len().max(1)).min(new_cap);
            let gene_cap = res_full.min(f.len().max(1)).min(new_cap) > res_;
            // The first field has no lead of its own. When the best competitor at two fields belongs to
            // another allele group and explains the reads about as well (lead below 1), the group is
            // a coin toss: no field is certain. On 80 HPRC samples such near ties named the wrong group
            // in 3 of 7 alleles at 30x and 6 of 23 at 10x; a lead up to 6 would also have withheld many
            // right groups (137 right for 20 wrong at 10x). HLA_FIELD1=0: group always named.
            let group = |n: &str| n.split('*').nth(1).and_then(|x| x.split(':').next()).unwrap_or("").to_string();
            if res_full == 1 && m[0] < 1.0 && !renamed && alt[0].contains('*') && group(&alt[0]) != group(shown) && std::env::var("HLA_FIELD1").map_or(true, |v| v != "0") {
                res_ = 0;
            }
            let safe = if res_ == 0 { g.to_string() } else { format!("{}*{}", g, f[..res_.min(f.len()).max(1)].join(":")) };
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
            let fa = al.iter().find(|a| a.name == *fin);
            if !renamed {
                if let (Some(v), Some(a)) = (ns, fa) {
                    if let Some((rec, applied)) = novel_record(g, sl + 1, fin, &a.genomic, v) {
                        fasta.push_str(&rec);
                        sheet.push_str(&novel_rows(g, sl + 1, a, v, &applied));
                    }
                }
            }
            called.push((shown.clone(), res_));
            safes[sl] = safe.split('*').nth(1).unwrap_or("?").to_string();
            jal.push(serde_json::json!({
                "allele": shown, "safe_call": safe, "resolution": res_, "gene_cap": gene_cap,
                "g_group": grp.g(shown), "p_group": grp.p(shown),
                "expression": crate::groups::expression(shown),
                // serology and receptor ligand follow from the protein: only with two supported fields
                "serology": if res_ >= 2 { grp.serology().antigen(shown).map(|x| x.0) } else { None },
                "serology_assumed": if res_ >= 2 { grp.serology().antigen(shown).map(|x| x.1) } else { None },
                "ligand": if res_ >= 2 { al.iter().find(|a| a.name == *shown).and_then(|a| {
                    let ex: Option<Vec<u8>> = (1..=3).map(|k| a.exon(k).map(|e| e.to_vec())).collect::<Option<Vec<_>>>().map(|v| v.concat());
                    ex.and_then(|c| crate::groups::ligand(g, &c))
                }) } else { None },
                "renamed_from": if renamed { serde_json::json!(fin) } else { serde_json::Value::Null },
                "margin": {"2": mj(m[0]), "3": mj(m[1]), "4": mj(m[2])},
                "alternative": {"2": alt[0], "3": alt[1], "4": alt[2]},
                "new_sites": if renamed { Some(vec![]) } else { ns.map(|v| v.iter().map(|s| {
                    let an = fa.map(|a| annotate(a, s.pos, &s.what));
                    serde_json::json!({"pos": s.pos + 1, "change": s.what, "reads": s.alt, "depth": s.depth, "exon": s.exon, "either_allele": s.either,
                        "region": an.as_ref().map(|x| x.region.clone()), "effect": an.as_ref().map(|x| x.effect.clone())})
                }).collect::<Vec<_>>()) },
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
        "thresholds": {"2": t_group(), "3": t_group(), "4": T[2], "4_lead_2_3": T[0], "4_01": if std::env::var("HLA_F401").map_or(true, |v| v != "0") { serde_json::json!(1.0) } else { serde_json::Value::Null }}, "genes": js,
        "database_release": DB_RELEASE.get(),
        "input": INPUTS.get(),
        "analysis_time": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "groups_loaded": grp.loaded,
        "serology_loaded": grp.serology().loaded,
        "repeat_mask": crate::panel::MASK_LOADED.load(std::sync::atomic::Ordering::Relaxed),
        "depth_estimate": depth_estimate(res),
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
        let head = format!("# hla-wgs {}: candidates for new alleles, one line per difference from the nearest database allele\n\
# pos counts from the first base of that allele's genomic sequence, cds_pos and the codon number from the start codon\n\
# a proposal from short reads is not a confirmed allele: before a submission to IPD-IMGT/HLA the sequence normally has to be\n\
# confirmed independently (for example by long reads or Sanger sequencing of the separated allele); see the database's current guidelines\n\
gene\tslot\trecord\tnearest_allele\tpos\tchange\tregion\tcds_pos\teffect\treads\tdepth\tassignment\tin_sequence\tstatus\n", env!("CARGO_PKG_VERSION"));
        std::fs::write(format!("{prefix}.novel.tsv"), head + &sheet).unwrap();
    }
}

/// Sequence of a new allele: the called database allele with its residual substitutions, insertions
/// and deletions applied. The header names the nearest allele and the applied changes, and says when
/// the sequence is incomplete (blind stretches, unresolved repeat lengths) or when a change could
/// belong to either allele.
fn novel_record(g: &str, slot: usize, name: &str, genomic: &[u8], sites: &[crate::novel::Site]) -> Option<(String, Vec<String>)> {
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
    Some((out, applied))
}

/// Where a position of an allele's genomic sequence lies and what a change there does to the protein.
pub struct Ann {
    pub region: String,
    /// 1-based position in the coding sequence (exons joined, from the start codon)
    pub cds_pos: Option<usize>,
    pub effect: String,
}

fn amino(c: &[u8]) -> char {
    const T: &[u8] = b"FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG";
    let ix = |b: u8| match b { b'T' => Some(0), b'C' => Some(1), b'A' => Some(2), b'G' => Some(3), _ => None };
    match (ix(c[0]), ix(c[1]), ix(c[2])) {
        (Some(a), Some(b), Some(d)) => T[a * 16 + b * 4 + d] as char,
        _ => 'X',
    }
}

/// Region (exon, intron, untranslated end) of position `p` and the effect of the change `what`.
/// Codons are counted from the start codon; each change is judged on its own.
pub fn annotate(a: &crate::db::Allele, p: usize, what: &str) -> Ann {
    // exons located on the genomic sequence, in exon order
    let mut ex: Vec<(u32, usize, usize)> = vec![];
    let mut all = !a.exons.is_empty();
    for (k, e) in &a.exons {
        match a.genomic.windows(e.len().max(1)).position(|w| w == e.as_slice()) {
            Some(s) if !e.is_empty() => ex.push((*k, s, s + e.len())),
            _ => all = false,
        }
    }
    ex.sort_by_key(|x| x.1);
    let none = |region: String, effect: &str| Ann { region, cds_pos: None, effect: effect.to_string() };
    if ex.is_empty() {
        return none("-".to_string(), "-");
    }
    let w = what.trim_end_matches("(unsupported)");
    let Some(i) = ex.iter().position(|&(_, s, e)| p >= s && p < e) else {
        if p < ex[0].1 { return none("5' UTR".to_string(), "non-coding"); }
        if p >= ex[ex.len() - 1].2 { return none("3' UTR".to_string(), "non-coding"); }
        let j = ex.iter().rposition(|&(_, _, e)| p >= e).unwrap_or(0);
        // the two bases next to an exon inside an intron
        let splice = p < ex[j].2 + 2 || ex.get(j + 1).map_or(false, |n| p + 2 >= n.1);
        return none(format!("intron {}", ex[j].0), if splice { "splice site" } else { "non-coding" });
    };
    let region = format!("exon {}", ex[i].0);
    let cds: Vec<u8> = ex.iter().flat_map(|&(_, s, e)| a.genomic[s..e].iter().copied()).collect();
    if !all || !cds.starts_with(b"ATG") {
        return none(region, "-");
    }
    let off: usize = ex[..i].iter().map(|&(_, s, e)| e - s).sum::<usize>() + (p - ex[i].1);
    let stop = (0..cds.len() / 3).find(|&c| amino(&cds[c * 3..c * 3 + 3]) == '*');
    let codon = off / 3;
    let effect = if stop.map_or(false, |s| codon > s) {
        "after an early stop codon".to_string()
    } else if let Some((r, v)) = w.split_once('>') {
        if r.len() == 1 && v.len() == 1 && codon * 3 + 3 <= cds.len() && cds[off] == r.as_bytes()[0] {
            let mut alt = cds[codon * 3..codon * 3 + 3].to_vec();
            alt[off % 3] = v.as_bytes()[0];
            let (x, y) = (amino(&cds[codon * 3..codon * 3 + 3]), amino(&alt));
            let n = codon + 1;
            if x == y { format!("synonymous p.{x}{n}=") } else if codon == 0 { format!("start codon lost p.{x}{n}{y}") } else if y == '*' { format!("stop gained p.{x}{n}*") } else if x == '*' { format!("stop lost p.*{n}{y}") } else { format!("missense p.{x}{n}{y}") }
        } else {
            "-".to_string()
        }
    } else {
        let len = if let Some(d) = w.strip_prefix("del").or_else(|| w.strip_prefix("ins")) { Some(d.len()) } else { w.strip_prefix("rep").and_then(|d| d.parse::<i64>().ok()).map(|d| d.unsigned_abs() as usize) };
        match len {
            Some(l) if l % 3 != 0 => "frameshift".to_string(),
            Some(_) => "in-frame length change".to_string(),
            None => "-".to_string(),
        }
    };
    Ann { region, cds_pos: Some(off + 1), effect }
}

/// lines of PREFIX.novel.tsv for one new allele
fn novel_rows(g: &str, slot: usize, a: &crate::db::Allele, sites: &[crate::novel::Site], applied: &[String]) -> String {
    let inseq = |s: &crate::novel::Site| applied.contains(&format!("{}:{}", s.pos + 1, s.what.trim_end_matches("(unsupported)")));
    let status = if sites.iter().any(|s| !inseq(s)) { "incomplete" } else if sites.iter().any(|s| s.either) { "phase uncertain" } else { "complete" };
    let mut out = String::new();
    for s in sites {
        let an = annotate(a, s.pos, &s.what);
        let blind = s.what.starts_with("blind");
        out.push_str(&format!("{g}\t{slot}\t{g}_slot{slot}_new\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{status}\n",
            a.name, s.pos + 1, s.what, if blind { "-" } else { an.region.as_str() }, an.cds_pos.map(|x| x.to_string()).unwrap_or_else(|| "-".to_string()),
            if blind { "-" } else { an.effect.as_str() }, s.alt, s.depth, if s.either { "either allele" } else { "this allele" }, if inseq(s) { "yes" } else { "no" }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::annotate;
    use crate::db::Allele;

    fn allele() -> Allele {
        // 5' UTR, exon 1 (ATGGCC), intron, exon 2 (AAATAG), 3' UTR
        let genomic = b"CCCCATGGCCGTAAGTTTCAGAAATAGTTTT".to_vec();
        Allele { name: "X*01:01".into(), genomic, exons: vec![(1, b"ATGGCC".to_vec()), (2, b"AAATAG".to_vec())], imputed: false, partial: false, extended: false, imputed_from: None, orig: None }
    }

    #[test]
    fn regions_and_effects() {
        let a = allele();
        let r = |p: usize, w: &str| { let x = annotate(&a, p, w); (x.region, x.cds_pos, x.effect) };
        assert_eq!(r(1, "C>T"), ("5' UTR".to_string(), None, "non-coding".to_string()));
        assert_eq!(r(9, "C>T"), ("exon 1".to_string(), Some(6), "synonymous p.A2=".to_string()));
        assert_eq!(r(8, "C>T"), ("exon 1".to_string(), Some(5), "missense p.A2V".to_string()));
        assert_eq!(r(10, "G>A"), ("intron 1".to_string(), None, "splice site".to_string()));
        assert_eq!(r(14, "G>A"), ("intron 1".to_string(), None, "non-coding".to_string()));
        assert_eq!(r(21, "A>T"), ("exon 2".to_string(), Some(7), "stop gained p.K3*".to_string()));
        assert_eq!(r(22, "delA"), ("exon 2".to_string(), Some(8), "frameshift".to_string()));
        assert_eq!(r(28, "T>C"), ("3' UTR".to_string(), None, "non-coding".to_string()));
    }
}

/// Approximate sequencing depth over the HLA genes: read pairs of ten genes that are always present
/// in two copies, against their median on the 80 HPRC development samples at 30x (8056 pairs, 150 bp
/// reads). Only a rough guide: read length and library change the count.
fn depth_estimate(res: &AHashMap<String, (StageB, Option<Vec<String>>, RMarg)>) -> Option<f64> {
    const GENES: [&str; 10] = ["A", "B", "C", "DPA1", "DPB1", "DQA1", "DQB1", "E", "F", "G"];
    let mut n = 0f64;
    for g in GENES {
        n += res.get(g)?.0.nreads as f64;
    }
    Some((n / 8056.0 * 30.0 * 10.0).round() / 10.0)
}
