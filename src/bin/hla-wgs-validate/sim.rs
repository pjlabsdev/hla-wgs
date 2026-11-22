//! Simulated short read pairs from IPD-IMGT/HLA genomic alleles with known gene of origin.
//! sim DBDIR OUTPREFIX [--per N] [--cov X] [--seed S] [--profile P] [--div D]
//!   --profile  illumina: 0.3 % substitutions, few indels, flat quality (default)
//!              lowq:     error rate rising from 0.2 % to 4 % along the read, qualities to match
//!              mgi:      DNB-like: substitutions rising towards the 3' end, almost no indels
//!              hard:     lowq errors plus 2 % of pairs chimeric (mates from two genes)
//!   --div      extra divergence of each source allele (0.01 = one difference per 100 bases,
//!              a third of them small indels), as a new allele far from the panel would be
//! The random numbers follow Python's random module, so a seed gives the same reads as the earlier
//! Python simulator.
use crate::common::*;
use crate::pyrand::PyRandom;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{BufWriter, Write};

const L: usize = 150;

struct Sim {
    rnd: PyRandom,
    profile: String,
    div: f64,
}

impl Sim {
    /// substitution, insertion, deletion probability at read position i
    fn rate(&self, i: usize) -> (f64, f64, f64) {
        let x = i as f64 / (L - 1) as f64;
        match self.profile.as_str() {
            "illumina" => (0.003, 0.0001, 0.0001),
            "lowq" | "hard" => {
                let e = 0.002 + 0.038 * x.powf(3.0);
                (e, e / 30.0, e / 30.0)
            }
            _ => {
                let e = 0.002 + 0.012 * x.powf(2.0);
                (e, e / 200.0, e / 200.0)
            }
        }
    }

    fn qual(&self, i: usize) -> char {
        let (a, b, c) = self.rate(i);
        let e = a + b + c;
        (33 + ((-10.0 * e.log10()) as i64).clamp(2, 41)) as u8 as char
    }

    fn err(&mut self, s: &[u8]) -> (String, String) {
        let mut o: Vec<u8> = Vec::with_capacity(L + 4);
        for &c in s {
            let (ps, pi, pd) = self.rate(o.len());
            let x = self.rnd.random();
            if x < ps {
                let others: Vec<u8> = b"ACGT".iter().copied().filter(|b| *b != c).collect();
                o.push(self.rnd.choice(&others));
            } else if x < ps + pd {
                continue;
            } else if x < ps + pd + pi {
                o.push(c);
                o.push(self.rnd.choice(b"ACGT"));
            } else {
                o.push(c);
            }
        }
        o.extend(std::iter::repeat(b'A').take(L));
        o.truncate(L);
        let q: String = (0..L).map(|i| self.qual(i)).collect();
        (String::from_utf8(o).unwrap(), q)
    }

    fn diverge(&mut self, s: &[u8]) -> Vec<u8> {
        if self.div <= 0.0 {
            return s.to_vec();
        }
        let mut o = vec![];
        let mut i = 0;
        while i < s.len() {
            if self.rnd.random() < self.div {
                let k = self.rnd.random();
                if k < 2.0 / 3.0 {
                    let others: Vec<u8> = b"ACGT".iter().copied().filter(|b| *b != s[i]).collect();
                    o.push(self.rnd.choice(&others));
                    i += 1;
                } else if k < 5.0 / 6.0 {
                    o.push(s[i]);
                    let n = self.rnd.randint(1, 6);
                    for _ in 0..n {
                        o.push(self.rnd.choice(b"ACGT"));
                    }
                    i += 1;
                } else {
                    i += self.rnd.randint(1, 6) as usize;
                }
            } else {
                o.push(s[i]);
                i += 1;
            }
        }
        o
    }
}

fn rc(s: &[u8]) -> Vec<u8> {
    s.iter().rev().map(|c| match c { b'A' => b'T', b'C' => b'G', b'G' => b'C', b'T' => b'A', b'N' => b'N', x => *x }).collect()
}

pub fn sim(args: &[&str]) {
    let (dbdir, out) = (args[0], args[1]);
    let (mut per, mut cov, mut seed, mut profile, mut div) = (4usize, 5.0f64, 7u64, "illumina".to_string(), 0.0f64);
    let mut i = 2;
    while i + 1 < args.len() {
        let v = args[i + 1];
        match args[i] {
            "--per" => per = v.parse().unwrap(),
            "--cov" => cov = v.parse().unwrap(),
            "--seed" => seed = v.parse().unwrap(),
            "--profile" => {
                assert!(["illumina", "lowq", "mgi", "hard"].contains(&v), "--profile illumina|lowq|mgi|hard");
                profile = v.to_string()
            }
            "--div" => div = v.parse().unwrap(),
            x => panic!("unknown option {x}"),
        }
        i += 2;
    }
    let mut sm = Sim { rnd: PyRandom::new(seed), profile, div };
    let panel: std::collections::HashSet<String> = read(&format!("{dbdir}/panel.fa")).lines().filter_map(|l| l.strip_prefix('>')).map(|l| l.split_whitespace().next().unwrap_or("").to_string()).collect();
    let j = db(&format!("{dbdir}/hladb.json.gz"));
    let mk = |p: &str| GzEncoder::new(BufWriter::new(std::fs::File::create(p).unwrap()), Compression::default());
    let mut f1 = mk(&format!("{out}.R1.fq.gz"));
    let mut f2 = mk(&format!("{out}.R2.fq.gz"));
    let mut meta = BufWriter::new(std::fs::File::create(format!("{out}.truth.tsv")).unwrap());
    let mut src: Vec<(String, String, i32, Vec<u8>)> = vec![];
    for g in genes(&j) {
        let al: Vec<&serde_json::Value> = alleles(g).iter().filter(|a| !s(a, "genomic").is_empty() && !b(a, "extended") && !b(a, "imputed") && !b(a, "partial") && s(a, "genomic").chars().count() > 1200).collect();
        if al.is_empty() {
            continue;
        }
        let inp: Vec<&serde_json::Value> = al.iter().copied().filter(|a| panel.contains(s(a, "name"))).collect();
        let outp: Vec<&serde_json::Value> = al.iter().copied().filter(|a| !panel.contains(s(a, "name"))).collect();
        let mut pick = sm.rnd.sample(&inp, (per / 2).min(inp.len()));
        let k2 = (per - per / 2).min(outp.len());
        pick.extend(sm.rnd.sample(&outp, k2));
        for a in pick {
            let gs = s(a, "genomic").to_uppercase();
            let d = sm.diverge(gs.as_bytes());
            src.push((s(g, "gene").to_string(), s(a, "name").to_string(), panel.contains(s(a, "name")) as i32, d));
        }
    }
    let mut n = 0usize;
    for si in 0..src.len() {
        let (gene, name_, inp) = (src[si].0.clone(), src[si].1.clone(), src[si].2);
        let slen = src[si].3.len();
        let npairs = (cov * slen as f64 / 300.0) as i64;
        for _ in 0..npairs {
            let fl = 160i64.max(sm.rnd.gauss(400.0, 80.0) as i64);
            if fl as usize >= slen {
                continue;
            }
            let fl = fl as usize;
            let st = sm.rnd.randrange(0, (slen - fl + 1) as i64) as usize;
            let fr = &src[si].3[st..st + fl];
            let mut r1: Vec<u8> = fr[..L].to_vec();
            let mut r2: Vec<u8> = rc(fr)[..L].to_vec();
            let mut g2 = gene.clone();
            if sm.profile == "hard" && sm.rnd.random() < 0.02 {
                // chimeric pair: the second mate from another gene; truth stays with the first mate's gene
                let oi = sm.rnd.randbelow_pub(src.len());
                let o = &src[oi];
                if o.3.len() > L {
                    let p2 = sm.rnd.randrange(0, (o.3.len() - L + 1) as i64) as usize;
                    r2 = rc(&o.3[p2..p2 + L]);
                    g2 = o.0.clone();
                }
            }
            if sm.rnd.random() < 0.5 {
                std::mem::swap(&mut r1, &mut r2);
            }
            let name = format!("s{n}");
            writeln!(meta, "{name}\t{gene}\t{name_}\t{inp}\t{st}\t{g2}").unwrap();
            for (f, r) in [(&mut f1, &r1), (&mut f2, &r2)] {
                let (seq, q) = sm.err(r);
                write!(f, "@{name}\n{seq}\n+\n{q}\n").unwrap();
            }
            n += 1;
        }
    }
    println!("{n} pairs");
    f1.finish().unwrap();
    f2.finish().unwrap();
}
