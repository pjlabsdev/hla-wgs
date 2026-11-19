//! Stage 0: map the extracted read pairs to the gene panel (minimap2 sr, paired, -N 20) and
//! assign every pair to the gene(s) with the best summed alignment score.
use crate::util::open_reader;
use ahash::AHashMap;
use ahash::AHashSet;
use minimap2::Aligner;
use rayon::prelude::*;
use std::io::Read;

pub struct Reads {
    pub names: Vec<String>,
    /// mate sequences in sequencing orientation; None = mate not aligned as primary
    pub seq: Vec<[Option<Vec<u8>>; 2]>,
}

impl Reads {
    pub fn get(&self, q: usize) -> Option<&[Option<Vec<u8>>; 2]> {
        let r = &self.seq[q];
        if r[0].is_none() && r[1].is_none() {
            None
        } else {
            Some(r)
        }
    }
}

pub struct PanelResult {
    pub reads: Reads,
    /// gene -> read ids in first-hit order
    pub assign: AHashMap<String, Vec<usize>>,
}

const ASSIGN_MARGIN: i64 = 12;
/// A pair counts for a gene only if both mates align to it, or the aligned mate lies within this
/// many bases of the end of the gene sequence its partner would run into. Reads of dispersed
/// repeats (Alu, L1) from elsewhere in the genome match repeat copies inside HLA introns with
/// one mate only; whole-genome FASTQ input and BAMs whose aligner leaves repeats unmapped
/// otherwise flood DRB1/DRB4 with tens of thousands of such reads.
const MATE_ROOM: i64 = 500;

pub enum Input {
    Fastq(String, String),
    Aligned(String, Option<String>),
}

const MASK_ANCHOR: i64 = 40;

/// `<panel>.mask.bed` (or env HLA_MASK): allele, start, end. Missing file means no mask.
fn load_mask(panel: &str) -> AHashMap<String, Vec<(i64, i64)>> {
    let p = std::env::var("HLA_MASK").unwrap_or_else(|_| format!("{panel}.mask.bed"));
    let mut m: AHashMap<String, Vec<(i64, i64)>> = AHashMap::new();
    if let Ok(t) = std::fs::read_to_string(&p) {
        for l in t.lines() {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() >= 3 {
                if let (Ok(a), Ok(b)) = (f[1].parse(), f[2].parse()) {
                    m.entry(f[0].to_string()).or_default().push((a, b));
                }
            }
        }
    }
    eprintln!(
        "[mask] {} alleles with repeat intervals from {}",
        m.len(),
        p
    );
    m
}

pub fn run(panel: &str, input: &Input, threads: usize) -> PanelResult {
    let mut al = Aligner::builder()
        .sr()
        .with_cigar()
        .with_index_threads(threads)
        .with_index(panel, None)
        .expect("panel index");
    al.mapopt.best_n = std::env::var("MM2N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    // exact 21-mer prefilter: minimap2 sr seeds are exact 21-mers, so a pair without any shared
    // 21-mer cannot get a hit; skipping it changes nothing but saves its seeding
    let set = panel_kmers(panel);
    let pair_rule = std::env::var("HLA_PAIR").is_ok();
    // repeat mask: intervals of panel alleles whose 21-mers recur elsewhere in the genome. A pair counts for a
    // gene only if one mate aligns with at least MASK_ANCHOR bases outside the mask, so reads of repeat copies
    // from other chromosomes cannot enter through an intronic Alu or L1 copy
    let mask = load_mask(panel);
    // read the next batch while the current one is mapped
    let (tx, rx) = std::sync::mpsc::sync_channel::<(Vec<crate::util::Fq>, Vec<crate::util::Fq>)>(2);
    let inp = match input {
        Input::Fastq(a, b) => Input::Fastq(a.clone(), b.clone()),
        Input::Aligned(a, b) => Input::Aligned(a.clone(), b.clone()),
    };
    let reader = std::thread::spawn(move || match inp {
        Input::Fastq(r1s, r2s) => {
            let mut src = crate::util::PairedFq::new(&r1s, &r2s);
            loop {
                let (b1, b2) = src.batch(50_000);
                if b1.is_empty() {
                    break;
                }
                tx.send((b1, b2)).unwrap();
            }
        }
        Input::Aligned(path, reference) => {
            let t = std::time::Instant::now();
            let (n, pairs, single) = crate::bamin::stream(&path, reference.as_deref(), threads, tx);
            eprintln!(
                "[input] {path}: {n} records, {pairs} pairs, {single} unpaired dropped, {:.1}s",
                t.elapsed().as_secs_f64()
            );
        }
    });
    let mut names = vec![];
    let mut seqs: Vec<[Option<Vec<u8>>; 2]> = vec![];
    // per read: genes in first-hit order with best AS per mate
    let mut gscore: Vec<Vec<(String, [i64; 2])>> = vec![];
    // optional read cache (env HLA_SAVE_READS=PREFIX): the pairs that pass the 21-mer prefilter
    // are written to PREFIX.R1.fq.gz / PREFIX.R2.fq.gz in input order. Typing these files as
    // FASTQ gives the same result as the original input, without reading the BAM/FASTQ again.
    let mut cache = std::env::var("HLA_SAVE_READS").ok().map(|pre| {
        let w = |m: u8| {
            let f = std::fs::File::create(format!("{pre}.R{m}.fq.gz"))
                .unwrap_or_else(|e| panic!("{pre}.R{m}.fq.gz: {e}"));
            flate2::write::GzEncoder::new(
                std::io::BufWriter::with_capacity(1 << 20, f),
                flate2::Compression::fast(),
            )
        };
        (w(1), w(2))
    });
    for (b1, b2) in rx {
        if let Some((w1, w2)) = cache.as_mut() {
            use std::io::Write;
            let keep: Vec<bool> = b1
                .par_iter()
                .zip(b2.par_iter())
                .map(|(x, y)| has_kmer(&x.seq, &set) || has_kmer(&y.seq, &set))
                .collect();
            for ((x, y), k) in b1.iter().zip(b2.iter()).zip(keep) {
                if k {
                    for (w, r) in [(&mut *w1, x), (&mut *w2, y)] {
                        let q = vec![b'I'; r.seq.len()];
                        w.write_all(b"@")
                            .and_then(|_| w.write_all(r.name.as_bytes()))
                            .and_then(|_| w.write_all(b"\n"))
                            .and_then(|_| w.write_all(&r.seq))
                            .and_then(|_| w.write_all(b"\n+\n"))
                            .and_then(|_| w.write_all(&q))
                            .and_then(|_| w.write_all(b"\n"))
                            .expect("read cache write");
                    }
                }
            }
        }
        let res: Vec<Option<(String, [Option<Vec<u8>>; 2], Vec<(String, [i64; 2])>)>> = b1
            .par_iter()
            .zip(b2.par_iter())
            .map(|(x, y)| {
                if !has_kmer(&x.seq, &set) && !has_kmer(&y.seq, &set) {
                    return None;
                }
                let (m1, m2) = al
                    .map_pair(
                        &x.seq,
                        &y.seq,
                        false,
                        false,
                        None,
                        None,
                        Some(x.name.as_bytes()),
                    )
                    .ok()?;
                if m1.is_empty() && m2.is_empty() {
                    return None;
                }
                if std::env::var("DBG0").map_or(false, |v| v == x.name) {
                    for (k, ms) in [(1, &m1), (2, &m2)] {
                        for m in ms.iter() {
                            eprintln!(
                                "mate{} {} {} {} prim={} sup={} AS={:?}",
                                k,
                                m.target_name.as_ref().unwrap(),
                                m.target_start,
                                m.strand as i32,
                                m.is_primary,
                                m.is_supplementary,
                                m.alignment.as_ref().map(|a| a.alignment_score)
                            );
                        }
                    }
                }
                let mut gs: Vec<(String, [i64; 2])> = vec![];
                let mut room: Vec<[i64; 2]> = vec![];
                let mut anchor: Vec<[bool; 2]> = vec![];
                let mut sq: [Option<Vec<u8>>; 2] = [None, None];
                // SAM order: all records of mate 1, then mate 2
                for (mate, ms, s) in [(0usize, &m1, &x.seq), (1, &m2, &y.seq)] {
                    // paired SAM from the minimap2 CLI carries primary and supplementary records only
                    for m in ms.iter().filter(|m| m.is_primary || m.is_supplementary) {
                        let tn = m.target_name.as_ref().unwrap();
                        let gene = tn.split('*').next().unwrap();
                        let a = m
                            .alignment
                            .as_ref()
                            .and_then(|a| a.alignment_score)
                            .unwrap_or(0) as i64;
                        // bases left on the gene sequence in the direction of the partner mate
                        let anch = match mask.get(tn.as_str()) {
                            Some(iv) => {
                                let (s0, e0) = (m.target_start as i64, m.target_end as i64);
                                let cov: i64 = iv
                                    .iter()
                                    .map(|&(a, b)| (e0.min(b) - s0.max(a)).max(0))
                                    .sum();
                                e0 - s0 - cov >= MASK_ANCHOR
                            }
                            None => true,
                        };
                        let rm = if m.strand == minimap2::Strand::Forward {
                            (m.target_len - m.target_end) as i64
                        } else {
                            m.target_start as i64
                        };
                        match gs.iter().position(|g| g.0 == gene) {
                            Some(i) => {
                                if a > gs[i].1[mate] {
                                    gs[i].1[mate] = a;
                                    room[i][mate] = rm;
                                }
                                anchor[i][mate] |= anch;
                            }
                            None => {
                                let mut v = [-1_000_000_000i64; 2];
                                v[mate] = a;
                                gs.push((gene.to_string(), v));
                                let mut r = [0i64; 2];
                                r[mate] = rm;
                                room.push(r);
                                let mut an = [false; 2];
                                an[mate] = anch;
                                anchor.push(an);
                            }
                        }
                        if m.is_primary && !m.is_supplementary && !s.is_empty() {
                            sq[mate] = Some(s.clone());
                        }
                    }
                }
                if !mask.is_empty() {
                    let mut k = anchor.iter();
                    let mut k2 = anchor.iter();
                    gs.retain(|_| {
                        let a = k.next().unwrap();
                        a[0] || a[1]
                    });
                    room.retain(|_| {
                        let a = k2.next().unwrap();
                        a[0] || a[1]
                    });
                    if gs.is_empty() {
                        return None;
                    }
                }
                if pair_rule {
                    let keep: Vec<bool> = gs
                        .iter()
                        .zip(room.iter())
                        .map(|(g, r)| {
                            let (h0, h1) = (g.1[0] > -1_000_000, g.1[1] > -1_000_000);
                            (h0 && h1) || (h0 && r[0] < MATE_ROOM) || (h1 && r[1] < MATE_ROOM)
                        })
                        .collect();
                    let mut k = keep.iter();
                    gs.retain(|_| *k.next().unwrap());
                    if gs.is_empty() {
                        return None;
                    }
                }
                Some((x.name.clone(), sq, gs))
            })
            .collect();
        for r in res.into_iter().flatten() {
            names.push(r.0);
            seqs.push(r.1);
            gscore.push(r.2);
        }
    }
    reader.join().unwrap();
    if let Some((w1, w2)) = cache {
        w1.finish()
            .and_then(|mut b| std::io::Write::flush(&mut b))
            .expect("read cache R1");
        w2.finish()
            .and_then(|mut b| std::io::Write::flush(&mut b))
            .expect("read cache R2");
    }
    let mut assign: AHashMap<String, Vec<usize>> = AHashMap::new();
    for (q, gs) in gscore.iter().enumerate() {
        let tot: Vec<i64> = gs.iter().map(|(_, v)| v[0].max(0) + v[1].max(0)).collect();
        let best = *tot.iter().max().unwrap();
        if best < 60 {
            continue;
        }
        let mut tied: Vec<usize> = (0..gs.len())
            .filter(|&i| tot[i] >= best - ASSIGN_MARGIN)
            .collect();
        for m in 0..2 {
            let bm = gs.iter().map(|g| g.1[m]).max().unwrap();
            if bm >= 60 {
                for i in 0..gs.len() {
                    if gs[i].1[m] >= bm - 2 && !tied.contains(&i) {
                        tied.push(i);
                    }
                }
            }
        }
        for i in tied {
            assign.entry(gs[i].0.clone()).or_default().push(q);
        }
    }
    PanelResult {
        reads: Reads { names, seq: seqs },
        assign,
    }
}

const KF: usize = 21;

fn code(c: u8) -> u64 {
    match c {
        b'A' | b'a' => 0,
        b'C' | b'c' => 1,
        b'G' | b'g' => 2,
        b'T' | b't' => 3,
        _ => 4,
    }
}

fn for_kmers(s: &[u8], mut f: impl FnMut(u64) -> bool) -> bool {
    let mask = (1u64 << (2 * KF)) - 1;
    let shift = 2 * (KF - 1) as u64;
    let (mut fw, mut rc, mut l) = (0u64, 0u64, 0usize);
    for &c in s {
        let x = code(c);
        if x > 3 {
            l = 0;
            continue;
        }
        fw = ((fw << 2) | x) & mask;
        rc = (rc >> 2) | ((3 - x) << shift);
        l += 1;
        if l >= KF && f(fw.min(rc)) {
            return true;
        }
    }
    false
}

fn panel_kmers(panel: &str) -> AHashSet<u64> {
    let mut fa = String::new();
    open_reader(panel).read_to_string(&mut fa).unwrap();
    let mut set = AHashSet::with_capacity(8 << 20);
    let mut seq: Vec<u8> = vec![];
    for l in fa.lines().chain(std::iter::once(">")) {
        if l.starts_with('>') {
            for_kmers(&seq, |k| {
                set.insert(k);
                false
            });
            seq.clear();
        } else {
            seq.extend_from_slice(l.trim().as_bytes());
        }
    }
    set
}

fn has_kmer(s: &[u8], set: &AHashSet<u64>) -> bool {
    for_kmers(s, |k| set.contains(&k))
}
