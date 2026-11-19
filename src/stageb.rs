//! Stage B: every gene read against every candidate (Rust kernel), genotype = best allele pair
//! under a read mixture model with priors. Also candidate padding (shared flanks) and decoys.
use crate::align::{Preset, aligner, best_mlen, map};
use crate::db::{Allele, Db};
use crate::kern;
use crate::names::{diff_level, fields, is_null_like, num_key, two};
use crate::panel::Reads;
use crate::prior::{PRIOR_IMPUTED, PRIOR_NULL, Priors};
use crate::stagea::np_sum;
use ahash::AHashMap;
use rayon::prelude::*;
use std::sync::Mutex;

pub const LAM: f64 = 3.0;
pub const BG_EDITS: f64 = 3.0;
pub const TIE: f64 = 0.05;
const NEAR: f64 = 60.0;
const ALT2: usize = 3;

pub fn decoys(g: &str) -> &'static [&'static str] {
    match g {
        "A" | "B" | "C" => &["Y", "H", "J", "K", "L", "G", "E", "F"],
        "E" => &["A", "B", "C", "F", "G", "H", "J", "K", "L", "Y"],
        "F" => &["A", "B", "C", "E", "G", "H", "J", "K", "L", "Y"],
        "G" => &["A", "B", "C", "E", "F", "H", "J", "K", "L", "Y"],
        "H" => &["A", "B", "C", "E", "F", "G", "J", "K", "L", "Y"],
        "K" => &["A", "B", "C", "E", "F", "G", "H", "J", "L", "Y"],
        "V" => &["A", "B", "C", "E", "F", "G", "H", "J", "K", "L", "Y"],
        "DQA1" => &["DQA2"],
        "DQB1" => &["DQB2"],
        "DPA1" => &["DPA2"],
        "DPB1" => &["DPB2"],
        _ => &[],
    }
}

pub struct Ctx<'a> {
    pub db: &'a Db,
    pub reads: &'a Reads,
    pub pri: Priors<'a>,
    pad: Mutex<AHashMap<(String, usize, usize), Vec<u8>>>,
    donor: Mutex<AHashMap<(String, usize, Vec<usize>), usize>>,
    decoy: Mutex<AHashMap<String, AHashMap<usize, f64>>>,
}

pub struct StageB {
    pub pair: (usize, usize),
    pub ties: Vec<(usize, usize)>,
    pub near: Vec<(usize, usize)>,
    pub alts: Vec<String>,
    /// per slot: margins at fields 2/3/4 and the alternative names
    pub bmargin: [([f64; 3], [String; 3]); 2],
    pub ll: f64,
    pub gap: f64,
    pub nreads: usize,
    pub cand: Vec<usize>,
}

impl<'a> Ctx<'a> {
    pub fn new(db: &'a Db, reads: &'a Reads) -> Self {
        Ctx {
            db,
            reads,
            pri: Priors::new(db),
            pad: Mutex::new(AHashMap::new()),
            donor: Mutex::new(AHashMap::new()),
            decoy: Mutex::new(AHashMap::new()),
        }
    }

    pub fn al(&self, g: &str) -> &'a [Allele] {
        &self.db.ref_[g]
    }

    fn orig_seq(&self, g: &str, i: usize) -> &'a [u8] {
        let a = &self.al(g)[i];
        if a.extended {
            if let Some(o) = &a.orig {
                return o;
            }
        }
        &a.genomic
    }

    /// candidate sequence with the flanks of its nearest longer allele; the donor is padded the same
    /// way first, so every member of a group ends up spanning the longest member's extent. Without
    /// that, a slightly shorter allele loses every read pair with one mate beyond its last base.
    fn padded(&self, g: &str, i: usize, pool: &[usize], depth: usize) -> Vec<u8> {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        pool.hash(&mut h);
        let key = (g.to_string(), i, h.finish() as usize);
        if let Some(v) = self.pad.lock().unwrap().get(&key) {
            return v.clone();
        }
        let al = self.al(g);
        let donor = self.best_donor(g, i, pool);
        let s_ = self.orig_seq(g, i);
        let mut out = al[i].genomic.clone();
        if donor != i {
            let d: Vec<u8> = if depth < 6 && std::env::var("HLA_PAD1").is_err() {
                self.padded(g, donor, pool, depth + 1)
            } else {
                al[donor].genomic.clone()
            };
            if d.len() > s_.len() + 20 {
                let a = aligner(&d, Preset::Asm10);
                let hs = map(&a, s_);
                if let Some(h) = best_mlen(&hs) {
                    let ls = s_.len() as i64;
                    if h.fwd && h.q_st < 30 && ls - h.q_en < 30 {
                        let a0 = (h.r_st - h.q_st).max(0) as usize;
                        let b0 = ((d.len() as i64).min(h.r_en + (ls - h.q_en))) as usize;
                        let mut o = d[..a0].to_vec();
                        o.extend_from_slice(s_);
                        o.extend_from_slice(&d[b0.min(d.len())..]);
                        out = o;
                    }
                }
            }
        }
        self.pad.lock().unwrap().insert(key, out.clone());
        out
    }

    fn best_donor(&self, g: &str, i: usize, pool: &[usize]) -> usize {
        let key = (g.to_string(), i, pool.to_vec());
        if let Some(&v) = self.donor.lock().unwrap().get(&key) {
            return v;
        }
        let al = self.al(g);
        let s_ = self.orig_seq(g, i);
        let mut best = (i64::MAX, i);
        let longer: Vec<usize> = pool
            .iter()
            .copied()
            .filter(|&j| !al[j].extended && al[j].genomic.len() > s_.len() + 20)
            .collect();
        let mut a = None;
        for j in longer {
            let ck = (g.to_string(), al[i].name.clone(), al[j].name.clone());
            let nm = match self.db.donor_nm.get(&ck) {
                Some(v) => *v,
                None => {
                    let a = a.get_or_insert_with(|| aligner(s_, Preset::Asm10));
                    let hs = map(a, &al[j].genomic);
                    best_mlen(&hs).map(|h| h.nm + (s_.len() as i64 - (h.r_en - h.r_st)))
                }
            };
            if let Some(nm) = nm {
                if nm < best.0 {
                    best = (nm, j);
                }
            }
        }
        self.donor.lock().unwrap().insert(key, best.1);
        best.1
    }

    pub fn cand_seqs(&self, g: &str, cand: &[usize]) -> Vec<Vec<u8>> {
        let al = self.al(g);
        let mut grp: AHashMap<String, Vec<usize>> = AHashMap::new();
        for &i in cand {
            if !al[i].imputed {
                grp.entry(two(&al[i].name)).or_default().push(i);
            }
        }
        cand.iter()
            .map(|&i| {
                let pool = grp
                    .get(&two(&al[i].name))
                    .map(|v| &v[..v.len().min(60)])
                    .unwrap_or(&[]);
                self.padded(g, i, pool, 0)
            })
            .collect()
    }

    /// per read: best pair edit distance to any decoy allele (computed once per gene)
    pub fn decoy_min(&self, g: &str, qnames: &[usize]) -> AHashMap<usize, f64> {
        if let Some(v) = self.decoy.lock().unwrap().get(g) {
            return v.clone();
        }
        let mut seqs: Vec<&[u8]> = vec![];
        for dg in decoys(g) {
            if let Some(al) = self.db.ref_.get(*dg) {
                let al: Vec<&Allele> = al.iter().filter(|a| !a.imputed).collect();
                let step = (al.len() / 25).max(1);
                seqs.extend(al.iter().step_by(step).map(|a| a.genomic.as_slice()));
            }
        }
        if let Some(gd) = self.db.genome_decoy.get(g) {
            seqs.extend(gd.iter().map(|x| x.as_slice()));
        }
        let mut d = AHashMap::new();
        if !seqs.is_empty() {
            if let Some((e, used)) = kern::edit_matrix(self.reads, qnames, &seqs) {
                for (row, q) in e.iter().zip(used) {
                    d.insert(q, row.iter().cloned().fold(f64::INFINITY, f64::min));
                }
            }
        }
        self.decoy.lock().unwrap().insert(g.to_string(), d.clone());
        d
    }

    pub fn allele_prior(&self, g: &str, a: &Allele) -> f64 {
        (if is_null_like(&a.name) {
            -PRIOR_NULL
        } else {
            0.0
        }) + (if a.imputed { -PRIOR_IMPUTED } else { 0.0 })
            + (if a.extended { -0.3 } else { 0.0 })
            + self.pri.freq_prior(g, &a.name)
            + self.pri.sub_prior(g, &a.name)
            + self.pri.f4_prior(&a.name)
    }

    /// reads kept for scoring: best cell <= 10 edits and not explained better by a decoy
    pub fn keep_rows(
        &self,
        g: &str,
        qnames: &[usize],
        e: &[Vec<f64>],
        used: &[usize],
    ) -> Vec<bool> {
        let mut keep: Vec<bool> = e
            .iter()
            .map(|r| r.iter().cloned().fold(f64::INFINITY, f64::min) <= 10.0)
            .collect();
        if !decoys(g).is_empty() || self.db.genome_decoy.contains_key(g) {
            let dm = self.decoy_min(g, qnames);
            for (k, (r, q)) in e.iter().zip(used).enumerate() {
                let m = r.iter().cloned().fold(f64::INFINITY, f64::min);
                if dm.get(q).copied().unwrap_or(999.0) < m {
                    keep[k] = false;
                }
            }
        }
        keep
    }

    pub fn stage_b(&self, g: &str, qnames: &[usize], cand: &[usize]) -> Option<StageB> {
        let al = self.al(g);
        let seqs = self.cand_seqs(g, cand);
        let sr: Vec<&[u8]> = seqs.iter().map(|s| s.as_slice()).collect();
        let (e, used) = kern::edit_matrix(self.reads, qnames, &sr)?;
        let keep = self.keep_rows(g, qnames, &e, &used);
        // debugging aid: HLA_CMP="GENE|alleleA|alleleB" lists the reads that tell two candidates apart
        if let Ok(v) = std::env::var("HLA_CMP") {
            let f: Vec<&str> = v.split('|').collect();
            if f.len() == 3 && f[0] == g {
                let pos = |n: &str| cand.iter().position(|&i| al[i].name == n);
                if let (Some(x), Some(y)) = (pos(f[1]), pos(f[2])) {
                    eprintln!(
                        "HLACMPLEN\t{}\t{}\t{}\t{}",
                        seqs[x].len(),
                        seqs[y].len(),
                        al[cand[x]].genomic.len(),
                        al[cand[y]].genomic.len()
                    );
                    let n = seqs[x].len().min(seqs[y].len());
                    eprintln!(
                        "HLACMPEND\t{}\t{}",
                        String::from_utf8_lossy(&seqs[x][seqs[x].len() - 80..]),
                        String::from_utf8_lossy(&seqs[y][seqs[y].len() - 80..])
                    );
                    eprintln!(
                        "HLACMPSTART\t{}\t{}\t{}",
                        String::from_utf8_lossy(&seqs[x][..60]),
                        String::from_utf8_lossy(&seqs[y][..60]),
                        n
                    );
                    for ((row, q), k) in e.iter().zip(used.iter()).zip(keep.iter()) {
                        if row[x] != row[y] {
                            eprintln!(
                                "HLACMP\t{}\t{}\t{}\t{}\t{}\t{}",
                                g,
                                self.reads.names[*q],
                                row[x],
                                row[y],
                                k,
                                cand.len()
                            );
                        }
                    }
                } else {
                    eprintln!("HLACMP\t{}\tnot both candidates ({} cands)", g, cand.len());
                }
            }
        }
        let e: Vec<&Vec<f64>> = e
            .iter()
            .zip(keep.iter())
            .filter(|x| *x.1)
            .map(|x| x.0)
            .collect();
        let nc = cand.len();
        let nr = e.len();
        // P column-major for the pair loop
        let mut p = vec![0.0f64; nc * nr];
        for (r, row) in e.iter().enumerate() {
            for c in 0..nc {
                p[c * nr + r] = (-LAM * row[c].min(12.0)).exp();
            }
        }
        let bg = (-LAM * BG_EDITS).exp();
        let prior: Vec<f64> = cand.iter().map(|&i| self.allele_prior(g, &al[i])).collect();
        let pairs: Vec<(usize, usize)> =
            (0..nc).flat_map(|i| (i..nc).map(move |j| (i, j))).collect();
        let lls: Vec<f64> = pairs
            .par_iter()
            .map_init(
                || vec![0.0f64; nr],
                |buf, &(i, j)| {
                    let (pi, pj) = (&p[i * nr..(i + 1) * nr], &p[j * nr..(j + 1) * nr]);
                    for r in 0..nr {
                        buf[r] = (0.5 * pi[r] + 0.5 * pj[r] + bg).ln();
                    }
                    np_sum(buf.iter().copied()) + prior[i] + prior[j]
                },
            )
            .collect();
        let mut scores: AHashMap<(usize, usize), f64> = AHashMap::with_capacity(pairs.len());
        for (k, &pr) in pairs.iter().enumerate() {
            scores.insert(pr, lls[k]);
        }
        let mut ranked: Vec<usize> = (0..pairs.len()).collect();
        ranked.sort_by(|&a, &b| lls[b].partial_cmp(&lls[a]).unwrap());
        let ll0 = lls[ranked[0]];
        let mut ties: Vec<(usize, usize)> = ranked
            .iter()
            .filter(|&&k| lls[k] >= ll0 - TIE)
            .map(|&k| pairs[k])
            .collect();
        let simple = |i: usize| {
            let a = &al[cand[i]];
            let f = fields(&a.name);
            (a.imputed, f.len() < 4, num_key(&a.name))
        };
        ties.sort_by(|x, y| (simple(x.0), simple(x.1)).cmp(&(simple(y.0), simple(y.1))));
        let best = ties[0];
        let second = ranked
            .iter()
            .map(|&k| lls[k])
            .find(|&v| v < ll0 - TIE)
            .unwrap_or(ll0 - 99.0);
        let near: Vec<(usize, usize)> = ranked
            .iter()
            .take(200)
            .filter(|&&k| lls[k] >= ll0 - NEAR)
            .map(|&k| pairs[k])
            .collect();
        let sc = |a: usize, b: usize| scores[&(a.min(b), a.max(b))];
        let mut bmargin: [([f64; 3], [String; 3]); 2] = Default::default();
        for sl in 0..2 {
            let (x, y) = if sl == 0 {
                (best.0, best.1)
            } else {
                (best.1, best.0)
            };
            let mut m = [999.0f64; 3];
            let mut alt: [String; 3] = Default::default();
            for k in 0..nc {
                if k == x {
                    continue;
                }
                let lv = diff_level(&al[cand[k]].name, &al[cand[x]].name);
                if lv == 9 {
                    continue;
                }
                let d = ll0 - sc(k, y);
                for (ni, n) in [2u32, 3, 4].iter().enumerate() {
                    if lv <= *n && d < m[ni] {
                        m[ni] = d;
                        alt[ni] = al[cand[k]].name.clone();
                    }
                }
            }
            bmargin[sl] = (m, alt);
        }
        let mut alts = vec![];
        for sl in 0..2 {
            let y = if sl == 0 { best.1 } else { best.0 };
            let mut order: Vec<usize> = (0..nc).collect();
            order.sort_by(|&a, &b| sc(b, y).partial_cmp(&sc(a, y)).unwrap());
            let mut seen: Vec<String> = vec![];
            for k in order {
                let f2 = two(&al[cand[k]].name);
                if !seen.contains(&f2) {
                    seen.push(f2);
                }
                if seen.len() >= ALT2 {
                    break;
                }
            }
            alts.extend(seen);
        }
        Some(StageB {
            pair: best,
            ties,
            near,
            alts,
            bmargin,
            ll: ll0,
            gap: ll0 - second,
            nreads: nr,
            cand: cand.to_vec(),
        })
    }
}
