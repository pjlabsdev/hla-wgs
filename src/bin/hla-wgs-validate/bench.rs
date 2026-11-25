//! Benchmark of the calls against the assembly truth, per gene and number of fields.
//!
//! The denominator is fixed before any result is read: every reference sample of the truth table
//! (or of `SAMPLES`) contributes one expected allele copy per haplotype that carries the gene. A
//! copy is evaluable at k fields when the truth names it unambiguously to k fields. Missing result
//! files, genes without a call, a wrong number of copies and calls without a truth counterpart are
//! counted, never dropped.
//!
//! Three numbers per gene and k:
//! - candidate: the best candidate (`allele`) is right to k fields, over all evaluable copies;
//! - released: `safe_call` names k or more fields (call rate = released / evaluable copies);
//! - precision: released calls right to k fields / released calls, including released calls of
//!   copies the truth does not have; undefined when nothing was released.
use crate::common::*;
use std::collections::BTreeSet;

/// truth of one haplotype for one gene
#[derive(Clone, Debug)]
pub enum Ref {
    /// nearest database alleles (several when tied) and their edit distance to the haplotype
    Alleles { d: i64, names: Vec<String> },
    /// the assembly does not resolve the gene on this haplotype
    Unresolved,
}

/// one row of PREFIX.conf.tsv
#[derive(Clone, Debug)]
pub struct CallRow {
    pub allele: String,
    pub safe: String,
    pub res: usize,
}

#[derive(Default)]
pub struct Truth {
    /// sample -> haplotypes with a finished truth (h1/h2 or mat/pat)
    pub haps: OMap<String, Vec<String>>,
    /// (sample, gene, haplotype) -> reference
    pub refs: std::collections::HashMap<(String, String, String), Ref>,
    /// genes the truth table covers at all; other genes cannot be judged
    pub genes: std::collections::HashSet<String>,
}

pub fn parse_truth(text: &str) -> Truth {
    let mut t = Truth::default();
    for l in text.lines() {
        let r: Vec<&str> = l.split('\t').collect();
        if r.len() < 3 {
            continue;
        }
        if r[2] == "_done" {
            t.haps.entry(r[0].to_string()).push(r[1].to_string());
            continue;
        }
        if r.len() < 5 {
            continue;
        }
        t.genes.insert(r[2].to_string());
        let key = (r[0].to_string(), r[2].to_string(), r[1].to_string());
        let v = if r[3] == "-" { Ref::Unresolved } else { Ref::Alleles { d: r[3].parse().unwrap_or(0), names: r[4].split(',').map(String::from).collect() } };
        t.refs.insert(key, v);
    }
    t
}

/// conf.tsv -> gene -> calls; rows without an allele ("-") are no calls
pub fn parse_conf(text: &str) -> OMap<String, Vec<CallRow>> {
    let mut m: OMap<String, Vec<CallRow>> = OMap::default();
    for l in text.lines().skip(1) {
        let r: Vec<&str> = l.split('\t').collect();
        if r.len() < 5 {
            continue;
        }
        let v = m.entry(r[0].to_string());
        if r[2] == "-" || r[2].is_empty() {
            continue;
        }
        v.push(CallRow { allele: r[2].to_string(), safe: r[3].to_string(), res: r[4].parse().unwrap_or(0) });
    }
    m
}

/// what the truth says at k fields: the one k-field name, or why there is none
#[derive(PartialEq, Debug)]
pub enum At {
    Name(String),
    Ambiguous,
    /// no truth name has k fields
    Short,
    /// k = 4 and the haplotype carries no database allele exactly
    New,
}

pub fn ref_at(d: i64, names: &[String], k: usize) -> At {
    if k == 4 && d > 0 {
        return At::New;
    }
    let deep: BTreeSet<String> = names.iter().filter(|n| nfields(n.split('*').nth(1).unwrap_or("")) >= k).map(|n| cut(n, k)).collect();
    let short = names.iter().any(|n| nfields(n.split('*').nth(1).unwrap_or("")) < k);
    match (deep.len(), short) {
        (0, _) => At::Short,
        (1, false) => At::Name(deep.into_iter().next().unwrap()),
        _ => At::Ambiguous,
    }
}

/// the name is right to k fields (it must itself have k fields)
fn right(name: &str, k: usize, r: &At) -> bool {
    match r {
        At::Name(x) => nfields(name.split('*').nth(1).unwrap_or("")) >= k && cut(name, k) == *x,
        _ => false,
    }
}

pub const LEVELS: [usize; 4] = [1, 2, 3, 4];

/// counts for one gene; keys are suffixed with the number of fields where they depend on it
#[derive(Default)]
pub struct Bench {
    pub st: OMap<String, Counter>,
    pub issues: Vec<String>,
}

impl Bench {
    fn add(&mut self, g: &str, k: &str, v: i64) {
        self.st.entry(g.to_string()).add(k, v);
    }
}

/// evaluates the calls of the given samples; `calls(sample)` is None when the result file is missing
pub fn evaluate(truth: &Truth, samples: &[String], genes: &[&str], calls: &dyn Fn(&str) -> Option<OMap<String, Vec<CallRow>>>) -> Bench {
    let mut b = Bench::default();
    let none: Vec<CallRow> = vec![];
    let genes: Vec<&str> = genes
        .iter()
        .copied()
        .filter(|g| {
            let known = truth.genes.contains(*g);
            if !known {
                b.issues.push(format!("-\t{g}\tgene not in the truth table: not evaluated"));
            }
            known
        })
        .collect();
    for s in samples {
        let haps: Vec<String> = match truth.haps.get(s) {
            Some(h) if h.len() == 2 => {
                let mut h = h.clone();
                h.sort();
                h
            }
            _ => {
                b.issues.push(format!("{s}\tsample\tno finished truth for two haplotypes: not counted"));
                continue;
            }
        };
        let conf = calls(s);
        if conf.is_none() {
            b.issues.push(format!("{s}\tsample\tresult file missing: every expected copy counts as no call"));
        }
        for g in &genes {
            // expected copies: haplotypes that carry the gene
            let mut exp: Vec<(String, Option<(i64, Vec<String>)>)> = vec![];
            for h in &haps {
                match truth.refs.get(&(s.clone(), g.to_string(), h.clone())) {
                    Some(Ref::Alleles { d, names }) => exp.push((h.clone(), Some((*d, names.clone())))),
                    Some(Ref::Unresolved) => exp.push((h.clone(), None)),
                    None => {}
                }
            }
            let c: &Vec<CallRow> = conf.as_ref().and_then(|m| m.get(&g.to_string())).unwrap_or(&none);
            b.add(g, "copies_expected", exp.len() as i64);
            b.add(g, "copies_called", c.len() as i64);
            if conf.is_none() {
                b.add(g, "copies_missing_file", exp.len() as i64);
            } else if c.len() != exp.len() {
                b.add(g, "copy_number_wrong", 1);
                b.issues.push(format!("{s}\t{g}\tcopy number: truth {} called {}", exp.len(), c.len()));
            }
            if c.is_empty() && conf.is_some() && !exp.is_empty() {
                b.add(g, "gene_no_call", 1);
            }
            // pair calls with haplotypes: most right fields of the candidates over all levels
            let n = c.len().max(exp.len());
            let mut best: Vec<usize> = (0..n).collect();
            let mut bs = -1i64;
            for pm in perms(n) {
                let mut v = 0i64;
                for (ci, ei) in pm.iter().enumerate() {
                    if let (Some(cr), Some((_, Some((d, names))))) = (c.get(ci), exp.get(*ei)) {
                        // right candidate fields first; among equal pairings the one with more
                        // right released fields, so that haplotype order cannot decide precision
                        for k in LEVELS {
                            let ok = right(&cr.allele, k, &ref_at(*d, names, k)) as i64;
                            v += 100 * ok + ok * (cr.res >= k) as i64;
                        }
                    }
                }
                if v > bs {
                    bs = v;
                    best = pm;
                }
            }
            for (ci, ei) in best.iter().enumerate() {
                let cr = c.get(ci);
                let e = exp.get(*ei);
                match e {
                    None => {
                        // a call without a copy in the truth: released fields are wrong
                        let cr = cr.unwrap();
                        for k in LEVELS {
                            if cr.res >= k {
                                b.add(g, &format!("released_extra{k}"), 1);
                            }
                        }
                    }
                    Some((_, None)) => {
                        b.add(g, "ref_unresolved", 1);
                    }
                    Some((_, Some((d, names)))) => {
                        for k in LEVELS {
                            let r = ref_at(*d, names, k);
                            match r {
                                At::Ambiguous | At::Short => {
                                    b.add(g, &format!("ref_{}{k}", if r == At::Short { "short" } else { "ambiguous" }), 1);
                                    // a released call here cannot be judged
                                    if cr.map_or(false, |c| c.res >= k) {
                                        b.add(g, &format!("released_unjudged{k}"), 1);
                                    }
                                    continue;
                                }
                                At::New => {
                                    // the haplotype is no database allele: a released four-field name is wrong
                                    b.add(g, &format!("ref_new{k}"), 1);
                                    if let Some(c) = cr.filter(|c| c.res >= k) {
                                        b.add(g, &format!("released_on_new{k}"), 1);
                                        b.issues.push(format!("{s}\t{g}\twrong at {k} fields: released {} on a haplotype that is no database allele (nearest {})", c.safe, names.iter().take(3).cloned().collect::<Vec<_>>().join(",")));
                                    }
                                    continue;
                                }
                                At::Name(_) => {}
                            }
                            b.add(g, &format!("expected{k}"), 1);
                            let Some(cr) = cr else {
                                b.add(g, &format!("no_candidate{k}"), 1);
                                continue;
                            };
                            b.add(g, &format!("candidate_ok{k}"), right(&cr.allele, k, &r) as i64);
                            if cr.res >= k {
                                b.add(g, &format!("released{k}"), 1);
                                let ok = right(&cr.safe, k, &r);
                                b.add(g, &format!("released_ok{k}"), ok as i64);
                                if !ok {
                                    b.issues.push(format!("{s}\t{g}\twrong at {k} fields: released {} (candidate {}), truth {}", cr.safe, cr.allele, names.iter().take(3).cloned().collect::<Vec<_>>().join(",")));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    b
}

fn pct(a: i64, n: i64) -> String {
    if n == 0 { "n/a".to_string() } else { format!("{:.1}", 100.0 * a as f64 / n as f64) }
}

/// one line per gene and number of fields, then a total per number of fields
pub fn render(b: &Bench, genes: &[&str]) -> String {
    let mut out = String::from("gene\tfields\texpected\tcandidate_ok\tcandidate_pct\treleased\treleased_ok\treleased_extra\treleased_on_new\treleased_unjudged\tprecision_pct\tcall_rate_pct\tright_of_expected_pct\tno_candidate\tref_ambiguous\tref_short\tref_new\n");
    let zero = Counter::default();
    for k in LEVELS {
        let mut tot = Counter::default();
        for g in genes.iter().map(|g| g.to_string()).chain(std::iter::once("ALL".to_string())) {
            let c = if g == "ALL" { &tot } else { b.st.get(&g).unwrap_or(&zero) };
            let e = c.get(&format!("expected{k}"));
            let rel = c.get(&format!("released{k}"));
            let rok = c.get(&format!("released_ok{k}"));
            let rx = c.get(&format!("released_extra{k}"));
            let rn = c.get(&format!("released_on_new{k}"));
            out += &format!(
                "{g}\t{k}\t{e}\t{}\t{}\t{rel}\t{rok}\t{rx}\t{rn}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                c.get(&format!("candidate_ok{k}")),
                pct(c.get(&format!("candidate_ok{k}")), e),
                c.get(&format!("released_unjudged{k}")),
                pct(rok, rel + rx + rn),
                pct(rel, e),
                pct(rok, e),
                c.get(&format!("no_candidate{k}")),
                c.get(&format!("ref_ambiguous{k}")),
                c.get(&format!("ref_short{k}")),
                c.get(&format!("ref_new{k}")),
            );
            if g != "ALL" {
                let mut one = Counter::default();
                for key in ["expected", "candidate_ok", "released", "released_ok", "released_extra", "released_on_new", "released_unjudged", "no_candidate", "ref_ambiguous", "ref_short", "ref_new"] {
                    let kk = format!("{key}{k}");
                    one.add(&kk, c.get(&kk));
                }
                tot.update(&one);
            }
        }
    }
    out += "\ngene\tcopies_expected\tcopies_called\tcopies_missing_file\tcopy_number_wrong\tgene_no_call\tref_unresolved\n";
    let mut tot = Counter::default();
    for g in genes.iter().map(|g| g.to_string()).chain(std::iter::once("ALL".to_string())) {
        let keys = ["copies_expected", "copies_called", "copies_missing_file", "copy_number_wrong", "gene_no_call", "ref_unresolved"];
        let c = if g == "ALL" { &tot } else { b.st.get(&g).unwrap_or(&zero) };
        out += &format!("{g}\t{}\n", keys.iter().map(|k| c.get(k).to_string()).collect::<Vec<_>>().join("\t"));
        if g != "ALL" {
            let mut one = Counter::default();
            for k in keys {
                one.add(k, c.get(k));
            }
            tot.update(&one);
        }
    }
    if !b.issues.is_empty() {
        out += "\n# sample\tgene\tissue\n";
        for i in &b.issues {
            out += i;
            out.push('\n');
        }
    }
    out
}

/// `eval23 DIR`: samples from SAMPLES (one per line) or every sample of the truth table
pub fn run(dir: &str, genes: &[&str]) {
    let truth = parse_truth(&read(&env_or("T4OUT", "truth/truth4.tsv")));
    let samples: Vec<String> = match std::env::var("SAMPLES") {
        Ok(f) => read(&f).split_whitespace().map(String::from).collect(),
        Err(_) => truth.haps.keys.clone(),
    };
    let calls = |s: &str| -> Option<OMap<String, Vec<CallRow>>> {
        let f = format!("{dir}/{s}.conf.tsv");
        if exists(&f) { Some(parse_conf(&read(&f))) } else { None }
    };
    let b = evaluate(&truth, &samples, genes, &calls);
    print!("{}", render(&b, genes));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conf(rows: &[(&str, &str, &str, usize)]) -> OMap<String, Vec<CallRow>> {
        let mut m: OMap<String, Vec<CallRow>> = OMap::default();
        for (g, a, s, r) in rows {
            let v = m.entry(g.to_string());
            if *a != "-" {
                v.push(CallRow { allele: a.to_string(), safe: s.to_string(), res: *r });
            }
        }
        m
    }

    fn line<'a>(out: &'a str, gene: &str, k: usize) -> Vec<&'a str> {
        out.lines().map(|l| l.split('\t').collect::<Vec<_>>()).find(|r| r.len() > 2 && r[0] == gene && r[1] == k.to_string()).unwrap()
    }

    /// six reference alleles: two right candidates released to one field only, four without a call.
    /// Two and three fields must report 0 of 6 released, and no precision.
    #[test]
    fn released_counts_against_all_expected_copies() {
        let truth = parse_truth(
            "S1\th1\t_done\nS1\th2\t_done\n\
             S1\th1\tA\t0\tA*01:01:01:01\nS1\th2\tA\t0\tA*02:01:01:01\n\
             S1\th1\tB\t0\tB*07:02:01:01\nS1\th2\tB\t0\tB*08:01:01:01\n\
             S1\th1\tC\t0\tC*07:01:01:01\nS1\th2\tC\t0\tC*07:02:01:01\n",
        );
        let calls = |_: &str| Some(conf(&[("A", "A*01:01:01:01", "A*01", 1), ("A", "A*02:01:01:01", "A*02", 1), ("B", "-", "-", 0), ("C", "-", "-", 0)]));
        let b = evaluate(&truth, &["S1".to_string()], &["A", "B", "C"], &calls);
        let out = render(&b, &["A", "B", "C"]);
        for k in [2, 3] {
            let r = line(&out, "ALL", k);
            assert_eq!((r[2], r[5], r[6]), ("6", "0", "0"), "{k} fields: expected, released, right");
            assert_eq!(r[10], "n/a", "precision without released calls");
            assert_eq!(r[11], "0.0", "call rate");
            assert_eq!(r[3], "2", "right candidates");
        }
        let r = line(&out, "ALL", 1);
        assert_eq!((r[5], r[6], r[10], r[11]), ("2", "2", "100.0", "33.3"));
    }

    #[test]
    fn equal_candidates_pair_by_released_fields() {
        // both candidates name h2's allele; the four-field one must go to h2 whatever the order
        let truth = parse_truth("S1\th1\t_done\nS1\th2\t_done\nS1\th1\tA\t0\tA*01:01:01:01\nS1\th2\tA\t0\tA*01:01:01:02\n");
        let calls = |_: &str| Some(conf(&[("A", "A*01:01:01:02", "A*01:01:01:02", 4), ("A", "A*01:01:01:02", "A*01:01:01", 3)]));
        let b = evaluate(&truth, &["S1".to_string()], &["A"], &calls);
        let r = line(&render(&b, &["A"]), "A", 4).iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!((r[5].as_str(), r[6].as_str(), r[10].as_str()), ("1", "1", "100.0"));
    }

    #[test]
    fn genes_outside_the_truth_are_not_counted() {
        // the truth covers DRB3 only; released DRB4 calls must not count as wrong extra copies
        let truth = parse_truth("S1\th1\t_done\nS1\th2\t_done\nS1\th1\tDRB3\t0\tDRB3*01:01:02:01\n");
        let calls = |_: &str| Some(conf(&[("DRB3", "DRB3*01:01:02:01", "DRB3*01:01:02:01", 4), ("DRB4", "DRB4*01:01:01:01", "DRB4*01:01:01:01", 4)]));
        let b = evaluate(&truth, &["S1".to_string()], &["DRB3", "DRB4"], &calls);
        let out = render(&b, &["DRB3", "DRB4"]);
        let r = line(&out, "ALL", 4);
        assert_eq!((r[2], r[5], r[6], r[7]), ("1", "1", "1", "0"));
        assert_eq!(r[10], "100.0");
        assert!(out.contains("DRB4\tgene not in the truth table"));
    }

    #[test]
    fn missing_file_and_copy_number_are_counted() {
        let truth = parse_truth("S1\th1\t_done\nS1\th2\t_done\nS2\th1\t_done\nS2\th2\t_done\nS1\th1\tDRB3\t0\tDRB3*01:01:02:01\nS2\th1\tDRB3\t0\tDRB3*02:02:01:01\n");
        let calls = |s: &str| if s == "S1" { Some(conf(&[("DRB3", "DRB3*01:01:02:01", "DRB3*01:01:02:01", 4), ("DRB3", "DRB3*02:02:01:01", "DRB3*02:02", 2)])) } else { None };
        let b = evaluate(&truth, &["S1".to_string(), "S2".to_string()], &["DRB3"], &calls);
        let out = render(&b, &["DRB3"]);
        let r = line(&out, "DRB3", 2);
        // two expected copies (one per sample); S2 is missing and counts as no call
        assert_eq!((r[2], r[5], r[6], r[7]), ("2", "1", "1", "1"));
        assert_eq!(r[10], "50.0", "the extra copy of S1 is a wrong released call");
        assert!(out.contains("S2\tsample\tresult file missing"));
        assert!(out.contains("S1\tDRB3\tcopy number: truth 1 called 2"));
    }

    #[test]
    fn ambiguous_and_short_truth_is_not_evaluable() {
        assert_eq!(ref_at(1, &["B*44:03:01:62".into(), "B*44:03:56".into()], 3), At::Ambiguous);
        assert_eq!(ref_at(1, &["B*44:03:01:62".into(), "B*44:03:56".into()], 2), At::Name("44:03".into()));
        assert_eq!(ref_at(0, &["H*02:09".into()], 3), At::Short);
        assert_eq!(ref_at(2, &["A*01:01:01:01".into()], 4), At::New);
        assert_eq!(ref_at(2, &["A*01:01:01:01".into()], 3), At::Name("01:01:01".into()));
    }
}
