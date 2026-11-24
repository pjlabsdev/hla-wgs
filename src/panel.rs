//! Stage 0: map the extracted read pairs to the gene panel (own aligner, crate gc-panel) and
//! assign every pair to the gene(s) with the best summed alignment score.
use crate::util::open_reader;
use ahash::AHashMap;
use rayon::prelude::*;
use ahash::AHashSet;
use std::io::Read;

pub struct Reads {
    pub names: Vec<String>,
    /// mate sequences in sequencing orientation; None = mate not aligned as primary
    pub seq: Vec<[Option<Vec<u8>>; 2]>,
}

impl Reads {
    pub fn get(&self, q: usize) -> Option<&[Option<Vec<u8>>; 2]> {
        let r = &self.seq[q];
        if r[0].is_none() && r[1].is_none() { None } else { Some(r) }
    }
}

/// read pairs at each stage of the panel step, for the run log (the counts add up)
#[derive(Default)]
pub struct PanelStats {
    pub pairs_in: u64,
    pub no_kmer: u64,
    pub no_hit: u64,
    /// every hit only inside the repeat mask
    pub in_mask: u64,
    /// dropped by the mate rule (HLA_PAIR)
    pub mate_rule: u64,
}

pub struct PanelResult {
    pub stats: PanelStats,
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
    eprintln!("[mask] {} alleles with repeat intervals from {}", m.len(), p);
    MASK_LOADED.store(!m.is_empty(), std::sync::atomic::Ordering::Relaxed);
    m
}

/// whether a repeat mask was in use (reported in PREFIX.json and on the result page)
pub static MASK_LOADED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn run(panel: &str, input: &Input, threads: usize) -> PanelResult {
    let envn = |k: &str, d: usize| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let opts = crate::aln::Opts { max_align: envn("HLA_PANEL_N", 100), chain_ratio: 0.6, per_group: envn("HLA_PANEL_G", 25), min_chain: envn("HLA_PANEL_C", 25) as f32, min_score: envn("HLA_PANEL_SE", 60) as i32, win: envn("HLA_PANEL_MW", 11), full: envn("HLA_PANEL_FULL", 80) as f32 / 100.0, ..crate::aln::Opts::sr() };
    let t0 = std::time::Instant::now();
    let al = crate::aln::Index::from_fasta(panel, opts);
    eprintln!("[panel] {} sequences indexed, {:.1}s", al.seqs.len(), t0.elapsed().as_secs_f64());
    // genes whose best alignment of a mate lies within this many points of the mate's best alignment
    let win = envn("HLA_PANEL_W", 0) as i32;
    let clip = envn("HLA_PANEL_CLIP", 0);
    // a hit needs this score, unless the read runs over the start or end of the panel sequence:
    // a mate that overhangs the allele is aligned only in part and is still a read of the gene
    let full = envn("HLA_PANEL_S", 70) as i32;
    let rescue_min = envn("HLA_PANEL_RS", full as usize) as i32;
    let bonus = envn("HLA_PANEL_B", 33) as i32;
    // off by default: on simulated low-quality and MGI reads it changed gene assignment by under 0.05 %
    let qmin = envn("HLA_PANEL_Q", 0).min(60) as u8;
    let rescue = std::env::var("HLA_PANEL_RESCUE").map_or(true, |v| v != "0");
    let rescue_win = envn("HLA_PANEL_RW", 40) as i32;
    // exact 21-mer prefilter: the aligner seeds with exact 21-mers, so a pair without any shared
    // 21-mer cannot get a hit; skipping it changes nothing but saves its seeding
    let pair_rule = std::env::var("HLA_PAIR").is_ok();
    // repeat mask: intervals of panel alleles whose 21-mers recur elsewhere in the genome. A pair counts for a
    // gene only if one mate aligns with at least MASK_ANCHOR bases outside the mask, so reads of repeat copies
    // from other chromosomes cannot enter through an intronic Alu or L1 copy
    let mask = load_mask(panel);
    let set = panel_kmers(panel, &mask);
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
            eprintln!("[input] {path}: {n} records, {pairs} pairs, {single} unpaired dropped, {:.1}s", t.elapsed().as_secs_f64());
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
            let f = std::fs::File::create(format!("{pre}.R{m}.fq.gz")).unwrap_or_else(|e| panic!("{pre}.R{m}.fq.gz: {e}"));
            flate2::write::GzEncoder::new(std::io::BufWriter::with_capacity(1 << 20, f), flate2::Compression::fast())
        };
        (w(1), w(2))
    });
    let mut stats = PanelStats::default();
    let no_hit = std::sync::atomic::AtomicU64::new(0);
    let no_kmer = std::sync::atomic::AtomicU64::new(0);
    let in_mask = std::sync::atomic::AtomicU64::new(0);
    let mate_rule = std::sync::atomic::AtomicU64::new(0);
    for (b1, b2) in rx {
        stats.pairs_in += b1.len() as u64;
        if let Some((w1, w2)) = cache.as_mut() {
            use std::io::Write;
            let keep: Vec<bool> = b1.par_iter().zip(b2.par_iter()).map(|(x, y)| has_kmer(&x.seq, &set) || has_kmer(&y.seq, &set)).collect();
            for ((x, y), k) in b1.iter().zip(b2.iter()).zip(keep) {
                if k {
                    for (w, r) in [(&mut *w1, x), (&mut *w2, y)] {
                        let q: Vec<u8> = if r.qual.len() == r.seq.len() { r.qual.iter().map(|&c| c.min(93) + 33).collect() } else { vec![b'I'; r.seq.len()] };
                        w.write_all(b"@").and_then(|_| w.write_all(r.name.as_bytes())).and_then(|_| w.write_all(b"\n")).and_then(|_| w.write_all(&r.seq)).and_then(|_| w.write_all(b"\n+\n")).and_then(|_| w.write_all(&q)).and_then(|_| w.write_all(b"\n")).expect("read cache write");
                    }
                }
            }
        }
        let res: Vec<Option<(String, [Option<Vec<u8>>; 2], Vec<(String, [i64; 2])>)>> = b1
            .par_iter()
            .zip(b2.par_iter())
            .map(|(x, y)| {
                if !has_kmer(&x.seq, &set) && !has_kmer(&y.seq, &set) {
                    no_kmer.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return None;
                }
                // a mate clipped by more than `clip` bases where the gene sequence goes on is part of
                // another sequence (paralog copy, structural change elsewhere): such hits do not count
                let keep = |q: &[u8], hs: Vec<crate::aln::Hit>| -> Vec<crate::aln::Hit> {
                    let hs: Vec<crate::aln::Hit> = hs
                        .into_iter()
                        .filter(|m| {
                            if m.score >= full {
                                return true;
                            }
                            let (cl, cr) = if m.fwd { (m.q_st, q.len() - m.q_en) } else { (q.len() - m.q_en, m.q_st) };
                            let rl = al.seqs[m.seq].len();
                            (cl > 0 && m.r_st <= 3) || (cr > 0 && rl - m.r_en <= 3)
                        })
                        .collect();
                    if clip == 0 {
                        return hs;
                    }
                    hs.into_iter()
                        .filter(|m| {
                            let (cl, cr) = if m.fwd { (m.q_st, q.len() - m.q_en) } else { (q.len() - m.q_en, m.q_st) };
                            let rl = al.seqs[m.seq].len();
                            cl.min(m.r_st) <= clip && cr.min(rl - m.r_en) <= clip
                        })
                        .collect()
                };
                // with HLA_PANEL_Q=q0, mismatches at bases below quality q0 cost less
                let mapq = |r: &crate::util::Fq| {
                    let mut hs = al.map(&r.seq);
                    if qmin > 0 && !r.qual.is_empty() {
                        for h in hs.iter_mut() {
                            h.score = crate::aln::qual_score(h, &r.qual, opts.sc.b, qmin);
                        }
                        hs.sort_by(|a, b| b.score.cmp(&a.score));
                    }
                    hs
                };
                let (m1, m2) = (keep(&x.seq, mapq(x)), keep(&y.seq, mapq(y)));
                if m1.is_empty() && m2.is_empty() {
                    no_hit.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return None;
                }
                if std::env::var("DBG0").map_or(false, |v| v == x.name) {
                    for (k, ms) in [(1, &m1), (2, &m2)] {
                        for m in ms.iter() {
                            eprintln!("mate{} {} {} {} AS={}", k, al.names[m.seq], m.r_st, m.fwd, m.score);
                        }
                    }
                }
                let mut gs: Vec<(String, [i64; 2])> = vec![];
                let mut room: Vec<[i64; 2]> = vec![];
                let mut anchor: Vec<[bool; 2]> = vec![];
                let mut sq: [Option<Vec<u8>>; 2] = [None, None];
                // hits per mate: (sequence, score, r start, r end, forward)
                let mut hs: [Vec<(usize, i32, usize, usize, bool)>; 2] = [
                    m1.iter().map(|m| (m.seq, m.score, m.r_st, m.r_en, m.fwd)).collect(),
                    m2.iter().map(|m| (m.seq, m.score, m.r_st, m.r_en, m.fwd)).collect(),
                ];
                let gene_of = |q: usize| al.names[q].split('*').next().unwrap();
                // rescue: a mate without a hit on the gene of its partner's best hit is aligned next to the partner
                if rescue {
                    for mate in 0..2 {
                        let other = 1 - mate;
                        // the best hit of the partner in each gene it fits nearly as well as its best (at most 3 genes)
                        let ptop = hs[other].first().map_or(0, |h| h.1);
                        let mut tries: Vec<(usize, i32, usize, usize, bool)> = vec![];
                        for h in hs[other].iter().filter(|h| h.1 >= ptop - rescue_win) {
                            if tries.len() < 3 && !tries.iter().any(|t| gene_of(t.0) == gene_of(h.0)) {
                                tries.push(*h);
                            }
                        }
                        for (ps, _, prs, pre, pfwd) in tries {
                        let g = gene_of(ps);
                        if hs[mate].iter().any(|h| gene_of(h.0) == g) {
                            continue;
                        }
                        let seq = &al.seqs[ps];
                        let (w0, w1) = (prs.saturating_sub(800), (pre + 800).min(seq.len()));
                        let mq = if mate == 0 { &x.seq } else { &y.seq };
                        let qo: Vec<u8> = if pfwd { crate::aln::revcomp(mq) } else { mq.to_ascii_uppercase() };
                        let (sc_, _, _, rs, re) = crate::aln::local_seeded(&qo, &seq[w0..w1], &opts.sc);
                        if sc_ >= rescue_min {
                            hs[mate].push((ps, sc_, w0 + rs, w0 + re, !pfwd));
                        }
                        }
                    }
                }
                // best score of each gene for each mate
                let mut gb: Vec<(&str, [i32; 2])> = vec![];
                for mate in 0..2 {
                    for h in &hs[mate] {
                        let g = gene_of(h.0);
                        match gb.iter().position(|x| x.0 == g) {
                            Some(i) => gb[i].1[mate] = gb[i].1[mate].max(h.1),
                            None => {
                                let mut v = [0, 0];
                                v[mate] = h.1;
                                gb.push((g, v));
                            }
                        }
                    }
                }
                let top = [0, 1].map(|m| gb.iter().map(|x| x.1[m]).max().unwrap_or(0));
                // a proper pair on one gene earns a bonus, as in paired mapping: both mates then go to that gene
                let pscore = gb.iter().filter(|x| x.1[0] > 0 && x.1[1] > 0).map(|x| x.1[0] + x.1[1] + bonus).max().unwrap_or(0);
                let paired = pscore > 0 && pscore >= top[0] + top[1];
                let pair_sum = |g: &str| gb.iter().find(|x| x.0 == g).map_or(0, |x| x.1[0] + x.1[1]);
                for (mate, s) in [(0usize, &x.seq), (1, &y.seq)] {
                    if hs[mate].is_empty() {
                        continue;
                    }
                    let genes: Vec<&str> = if paired {
                        gb.iter().filter(|x| x.1[0] > 0 && x.1[1] > 0 && x.1[0] + x.1[1] + bonus == pscore).map(|x| x.0).collect()
                    } else {
                        // among the genes this mate fits best, those its partner supports most
                        let c: Vec<&str> = gb.iter().filter(|x| x.1[mate] > 0 && x.1[mate] >= top[mate] - win).map(|x| x.0).collect();
                        let pb = c.iter().map(|g| pair_sum(g)).max().unwrap_or(0);
                        c.into_iter().filter(|g| pair_sum(g) == pb).collect()
                    };
                    let gbest = |g: &str| gb.iter().find(|x| x.0 == g).map_or(0, |x| x.1[mate]);
                    for &(q, sc_, rs, re, fw) in hs[mate].iter().filter(|h| genes.contains(&gene_of(h.0)) && h.1 >= gbest(gene_of(h.0)) - win) {
                        let tn = al.names[q].as_str();
                        let gene = gene_of(q);
                        let a = sc_ as i64;
                        // bases left on the gene sequence in the direction of the partner mate
                        let anch = match mask.get(tn) {
                            Some(iv) => {
                                let (s0, e0) = (rs as i64, re as i64);
                                let cov: i64 = iv.iter().map(|&(a, b)| (e0.min(b) - s0.max(a)).max(0)).sum();
                                e0 - s0 - cov >= MASK_ANCHOR
                            }
                            None => true,
                        };
                        let rm = if fw { (al.seqs[q].len() - re) as i64 } else { rs as i64 };
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
                    }
                    if !s.is_empty() {
                        sq[mate] = Some(s.clone());
                    }
                }
                if !mask.is_empty() {
                    let mut k = anchor.iter();
                    let mut k2 = anchor.iter();
                    gs.retain(|_| { let a = k.next().unwrap(); a[0] || a[1] });
                    room.retain(|_| { let a = k2.next().unwrap(); a[0] || a[1] });
                    if gs.is_empty() {
                        in_mask.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
                        mate_rule.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
        w1.finish().and_then(|mut b| std::io::Write::flush(&mut b)).expect("read cache R1");
        w2.finish().and_then(|mut b| std::io::Write::flush(&mut b)).expect("read cache R2");
    }
    let mut assign: AHashMap<String, Vec<usize>> = AHashMap::new();
    for (q, gs) in gscore.iter().enumerate() {
        let tot: Vec<i64> = gs.iter().map(|(_, v)| v[0].max(0) + v[1].max(0)).collect();
        let best = *tot.iter().max().unwrap();
        if best < 60 {
            continue;
        }
        let mut tied: Vec<usize> = (0..gs.len()).filter(|&i| tot[i] >= best - ASSIGN_MARGIN).collect();
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
    stats.no_kmer = no_kmer.into_inner();
    stats.no_hit = no_hit.into_inner();
    stats.in_mask = in_mask.into_inner();
    stats.mate_rule = mate_rule.into_inner();
    PanelResult { stats, reads: Reads { names, seq: seqs }, assign }
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

/// 21-mers of the panel for the prefilter. 21-mers that lie wholly inside a repeat-mask interval are
/// left out: a pair needs 40 aligned bases outside the mask to count for a gene, so a pair whose only
/// seeds are repeat 21-mers (Alu, L1 copies from all over the genome) would be aligned and then
/// dropped. Without them the prefilter passes far fewer pairs of a whole-genome FASTQ.
fn panel_kmers(panel: &str, mask: &AHashMap<String, Vec<(i64, i64)>>) -> KSet {
    let mut fa = String::new();
    open_reader(panel).read_to_string(&mut fa).unwrap();
    let mut set = AHashSet::with_capacity(8 << 20);
    let mut rep: AHashSet<u64> = AHashSet::new();
    let mut seq: Vec<u8> = vec![];
    let mut name = String::new();
    let lean = std::env::var("HLA_FULLKMERS").is_err();
    for l in fa.lines().chain(std::iter::once(">")) {
        if let Some(h) = l.strip_prefix('>') {
            for_kmers(&seq, |k| {
                set.insert(k);
                false
            });
            if lean {
                for &(a, b) in mask.get(name.as_str()).map(|v| v.as_slice()).unwrap_or(&[]) {
                    let (a, b) = ((a.max(0) as usize).min(seq.len()), (b.max(0) as usize).min(seq.len()));
                    if b > a {
                        for_kmers(&seq[a..b], |k| {
                            rep.insert(k);
                            false
                        });
                    }
                }
            }
            seq.clear();
            name = h.split_whitespace().next().unwrap_or("").to_string();
        } else {
            seq.extend_from_slice(l.trim().as_bytes());
        }
    }
    let n = set.len();
    set.retain(|k| !rep.contains(k));
    eprintln!("[prefilter] {} panel 21-mers, {} inside the repeat mask left out", n, n - set.len());
    set.shrink_to_fit();
    let mut bits = vec![0u64; KSET_BITS / 64];
    for &k in &set {
        let b = kset_bit(k);
        bits[b / 64] |= 1 << (b % 64);
    }
    KSet { bits, set }
}

/// the prefilter's 21-mers with a bit array in front: almost every 21-mer of a read is not in the
/// panel, and one look into a 2 MB bit array answers that without probing the hash table
struct KSet {
    bits: Vec<u64>,
    set: AHashSet<u64>,
}

const KSET_BITS: usize = 1 << 24;

fn kset_bit(k: u64) -> usize {
    (k.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as usize
}

impl KSet {
    fn contains(&self, k: u64) -> bool {
        let b = kset_bit(k);
        self.bits[b / 64] >> (b % 64) & 1 == 1 && self.set.contains(&k)
    }
}

fn has_kmer(s: &[u8], set: &KSet) -> bool {
    for_kmers(s, |k| set.contains(k))
}

