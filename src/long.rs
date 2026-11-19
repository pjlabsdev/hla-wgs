//! `hla-wgs long DBDIR READS OUTPREFIX [threads]`: typing from accurate long reads (PacBio HiFi).
//!
//! A read of 10 to 25 kb usually holds a whole gene, so the gene sequence of each haplotype can be
//! read off directly instead of being inferred:
//! 1. reads are mapped to the panel only to find the part of each read that holds a gene;
//! 2. per gene family these parts are split into haplotypes at positions where two bases are each
//!    carried by a good share of the reads (the reference for this is one of the reads itself);
//! 3. per haplotype one read is lengthened with the overhang of others until it covers the gene and
//!    corrected by majority at every position (consensus);
//! 4. the consensus is compared with every full-length allele of the gene. Distance 0 is an exact
//!    four-field result; otherwise the nearest allele and the number of differences are reported and
//!    the consensus itself is written to `PREFIX.long.fa`.
//!
//! Input: FASTQ (optionally gzip) or an indexed BAM, of which the MHC region is read.
use crate::db::Db;
use crate::util::revcomp;
use ahash::AHashMap;
use minimap2::{Aligner, Built, Strand};
use rayon::prelude::*;
use std::io::Write;

const FLANK: usize = 1000;
/// shortest panel hit that marks a gene on a read, and the least share of the panel allele it must cover
const MIN_HIT: usize = 1000;
const MIN_SPAN: f64 = 0.3;
const SPANNING: f64 = 0.98;

/// one difference of a read against the reference: position, kind (S substitution, I insertion before
/// the position, D deletion), and the new bases (for D the length as text)
type Ev = (usize, u8, String);

struct Seg {
    seq: Vec<u8>,
    span: f64,
    gene: String,
    read: usize,
    a: usize,
    b: usize,
    hit: usize,
}

struct Aln {
    ts: usize,
    te: usize,
    qs: usize,
    qe: usize,
    ev: Vec<Ev>,
}

fn family(g: &str) -> &str {
    match g {
        "DRB1" | "DRB2" | "DRB3" | "DRB4" | "DRB5" | "DRB6" | "DRB7" | "DRB8" | "DRB9" => "DRB",
        "DQA1" | "DQA2" => "DQA",
        "DQB1" | "DQB2" => "DQB",
        "DPA1" | "DPA2" => "DPA",
        "DPB1" | "DPB2" => "DPB",
        "DRA" | "DMA" | "DMB" | "DOA" | "DOB" | "MICA" | "MICB" | "TAP1" | "TAP2" | "HFE" => g,
        _ => "I",
    }
}

fn hifi(seq: &[u8]) -> Aligner<Built> {
    Aligner::builder()
        .map_hifi()
        .with_cigar()
        .with_seq(seq)
        .expect("index")
}

fn parse_cs(cs: &str, mut pos: usize) -> Vec<Ev> {
    let b = cs.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let op = b[i];
        i += 1;
        let s = i;
        while i < b.len() && !matches!(b[i], b':' | b'*' | b'+' | b'-' | b'~') {
            i += 1;
        }
        let v = &cs[s..i];
        match op {
            b':' => pos += v.parse::<usize>().unwrap_or(0),
            b'*' => {
                out.push((pos, b'S', v[1..].to_uppercase()));
                pos += 1;
            }
            b'+' => out.push((pos, b'I', v.to_uppercase())),
            b'-' => {
                out.push((pos, b'D', v.len().to_string()));
                pos += v.len();
            }
            _ => {}
        }
    }
    out
}

/// best forward alignment of every sequence to one reference
fn align(r: &[u8], segs: &[Seg], ids: &[usize]) -> AHashMap<usize, Aln> {
    let al = hifi(r);
    ids.par_iter()
        .filter_map(|&i| {
            let hs = al.map(&segs[i].seq, true, false, None, None, None).ok()?;
            let m = hs
                .into_iter()
                .filter(|m| m.strand == Strand::Forward)
                .max_by_key(|m| m.match_len)?;
            let a = m.alignment.as_ref()?;
            let ts = m.target_start as usize;
            Some((
                i,
                Aln {
                    ts,
                    te: m.target_end as usize,
                    qs: m.query_start as usize,
                    qe: m.query_end as usize,
                    ev: parse_cs(a.cs.as_deref().unwrap_or(""), ts),
                },
            ))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .collect()
}

fn tally(aln: &AHashMap<usize, Aln>, ids: &[usize], len: usize) -> (Vec<u32>, AHashMap<Ev, u32>) {
    let mut cov = vec![0u32; len + 1];
    let mut cnt: AHashMap<Ev, u32> = AHashMap::new();
    for i in ids {
        if let Some(a) = aln.get(i) {
            for c in &mut cov[a.ts..a.te.min(len)] {
                *c += 1;
            }
            for e in &a.ev {
                *cnt.entry(e.clone()).or_insert(0) += 1;
            }
        }
    }
    (cov, cnt)
}

fn apply(seq: &[u8], evs: &mut Vec<Ev>) -> Vec<u8> {
    evs.sort_by(|x, y| {
        (x.0, x.1 != b'I')
            .cmp(&(y.0, y.1 != b'I'))
            .then(x.2.cmp(&y.2))
    });
    let mut out = Vec::with_capacity(seq.len() + 64);
    let mut p = 0;
    for (pos, k, v) in evs.iter() {
        if *pos < p || *pos > seq.len() {
            continue;
        }
        out.extend_from_slice(&seq[p..*pos]);
        p = *pos;
        match k {
            b'S' => {
                out.extend_from_slice(v.as_bytes());
                p += 1;
            }
            b'I' => out.extend_from_slice(v.as_bytes()),
            _ => p += v.parse::<usize>().unwrap_or(0),
        }
    }
    out.extend_from_slice(&seq[p.min(seq.len())..]);
    out
}

/// lengthen a template with the overhang of reads that run past its ends
fn extend(mut r: Vec<u8>, segs: &[Seg], ids: &[usize]) -> Vec<u8> {
    for _ in 0..4 {
        let aln = align(&r, segs, ids);
        let l = r.len();
        let (mut right, mut left): (&[u8], &[u8]) = (&[], &[]);
        let mut keys: Vec<&usize> = aln.keys().collect();
        keys.sort();
        for i in keys {
            let a = &aln[i];
            let s = &segs[*i].seq;
            if a.te - a.ts < 1500 {
                continue;
            }
            let (over_r, over_l) = (a.qe + (l - a.te), a.qs.saturating_sub(a.ts));
            if a.te + 100 >= l && over_r < s.len() && s.len() - over_r > right.len() {
                right = &s[over_r..];
            }
            if a.ts <= 100 && over_l > left.len() {
                left = &s[..over_l];
            }
        }
        if right.len() < 200 {
            right = &[];
        }
        if left.len() < 200 {
            left = &[];
        }
        if right.is_empty() && left.is_empty() {
            break;
        }
        let mut n = left.to_vec();
        n.extend_from_slice(&r);
        n.extend_from_slice(right);
        r = n;
    }
    r
}

/// majority consensus; returns the sequence and the positions where a minority of 30 % or more disagrees
fn polish(mut t: Vec<u8>, segs: &[Seg], ids: &[usize]) -> (Vec<u8>, Vec<usize>) {
    let mut unsure = vec![];
    for round in 0..5 {
        let aln = align(&t, segs, ids);
        let (cov, cnt) = tally(&aln, ids, t.len());
        let d = |p: usize| cov[p.min(t.len() - 1)];
        let mut maj: Vec<Ev> = cnt
            .iter()
            .filter(|(e, c)| d(e.0) >= 3 && 2 * **c > d(e.0))
            .map(|(e, _)| e.clone())
            .collect();
        if maj.is_empty() || round == 4 {
            unsure = cnt
                .iter()
                .filter(|(e, c)| d(e.0) >= 4 && 10 * **c >= 3 * d(e.0))
                .map(|(e, _)| e.0)
                .collect();
            unsure.sort();
            unsure.dedup();
            break;
        }
        t = apply(&t, &mut maj);
    }
    (t, unsure)
}

fn template(segs: &[Seg], ids: &[usize]) -> usize {
    let mut sp: Vec<usize> = ids
        .iter()
        .copied()
        .filter(|&i| segs[i].span >= SPANNING)
        .collect();
    if sp.is_empty() {
        return *ids
            .iter()
            .max_by(|&&x, &&y| {
                segs[x]
                    .span
                    .partial_cmp(&segs[y].span)
                    .unwrap()
                    .then(y.cmp(&x))
            })
            .unwrap();
    }
    sp.sort_by_key(|&i| (segs[i].seq.len(), i));
    sp[sp.len() / 2]
}

/// recursive two-way split at substitution sites
fn split(ids: Vec<usize>, segs: &[Seg], depth: usize) -> Vec<Vec<usize>> {
    if ids.len() < 8 || depth > 9 {
        return vec![ids];
    }
    let r = extend(segs[template(segs, &ids)].seq.clone(), segs, &ids);
    let aln = align(&r, segs, &ids);
    let (names, lost): (Vec<usize>, Vec<usize>) = ids.iter().partition(|i| aln.contains_key(i));
    // reads that do not align at all belong to another gene of the family
    let rest = if lost.len() >= 4 {
        split(lost, segs, depth + 1)
    } else {
        vec![]
    };
    let (cov, cnt) = tally(&aln, &names, r.len());
    let mut het: Vec<Ev> = cnt
        .iter()
        .filter(|(e, c)| {
            let d = cov[e.0.min(r.len() - 1)];
            e.1 == b'S' && **c >= 4 && d - **c >= 4 && 5 * **c >= d && 5 * **c <= 4 * d
        })
        .map(|(e, _)| e.clone())
        .collect();
    het.sort();
    if het.len() == 1 {
        let c = cnt[&het[0]];
        let d = cov[het[0].0.min(r.len() - 1)];
        if !(c >= 6 && d - c >= 6 && 10 * c >= 3 * d && 10 * c <= 7 * d) {
            het.clear();
        }
    }
    let done = |names: Vec<usize>, mut rest: Vec<Vec<usize>>| {
        let mut v = vec![names];
        v.append(&mut rest);
        v
    };
    if het.is_empty() {
        return done(names, rest);
    }
    let vec_of = |i: usize| -> Vec<i32> {
        let a = &aln[&i];
        het.iter()
            .map(|e| {
                if a.ts <= e.0 && e.0 < a.te {
                    if a.ev.contains(e) { 1 } else { -1 }
                } else {
                    0
                }
            })
            .collect()
    };
    let vecs: AHashMap<usize, Vec<i32>> = names.iter().map(|&i| (i, vec_of(i))).collect();
    let seed = *names
        .iter()
        .max_by_key(|&&i| {
            (
                vecs[&i].iter().map(|x| x.abs()).sum::<i32>(),
                std::cmp::Reverse(i),
            )
        })
        .unwrap();
    let mut c = vecs[&seed].clone();
    let (mut a, mut b) = (vec![], vec![]);
    for _ in 0..10 {
        a.clear();
        b.clear();
        for &i in &names {
            if vecs[&i].iter().zip(&c).map(|(x, y)| x * y).sum::<i32>() >= 0 {
                a.push(i)
            } else {
                b.push(i)
            }
        }
        let c2: Vec<i32> = (0..het.len())
            .map(|k| {
                if a.iter().map(|i| vecs[i][k]).sum::<i32>()
                    - b.iter().map(|i| vecs[i][k]).sum::<i32>()
                    >= 0
                {
                    1
                } else {
                    -1
                }
            })
            .collect();
        if c2 == c {
            break;
        }
        c = c2;
    }
    if a.len() < 4 || b.len() < 4 {
        return done(names, rest);
    }
    let mut out = split(a, segs, depth + 1);
    out.append(&mut split(b, segs, depth + 1));
    out.append(&mut { rest });
    out
}

/// every eighth 21-mer (by hash) of a sequence, forward strand
fn sketch(s: &[u8]) -> Vec<u64> {
    let (mut v, mut n, mut out) = (0u64, 0usize, vec![]);
    for &c in s {
        let x = match c {
            b'A' => 0,
            b'C' => 1,
            b'G' => 2,
            b'T' => 3,
            _ => {
                n = 0;
                continue;
            }
        };
        v = ((v << 2) | x) & ((1u64 << 42) - 1);
        n += 1;
        if n >= 21 {
            let h = v.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            if (h >> 40) & 7 == 0 {
                out.push(v)
            }
        }
    }
    out
}

fn read_fastq(path: &str) -> Vec<Vec<u8>> {
    let mut r = crate::util::open_reader(path);
    let mut out = vec![];
    loop {
        let b = crate::util::read_fq_res(&mut *r, 10_000).unwrap_or_else(|e| panic!("{path}: {e}"));
        if b.is_empty() {
            break;
        }
        out.extend(b.into_iter().map(|f| f.seq));
    }
    out
}

/// primary records of the MHC region of an indexed BAM
fn read_bam(path: &str, threads: usize) -> std::io::Result<Vec<Vec<u8>>> {
    use noodles::sam::alignment::record::Flags;
    use noodles::{bam, bgzf};
    let workers = std::num::NonZero::new(threads.clamp(1, 8)).unwrap();
    let f = std::fs::File::open(path)?;
    let mut rd = bam::io::Reader::from(bgzf::io::MultithreadedReader::with_worker_count(
        workers,
        std::io::BufReader::with_capacity(8 << 20, f),
    ));
    let header = rd.read_header()?;
    let index = bam::fs::read_associated_index(path).or_else(|_| {
        let alt = path
            .strip_suffix(".bam")
            .map(|b| format!("{b}.bai"))
            .unwrap_or_default();
        noodles::bam::bai::fs::read(&alt).map(noodles::bam::Index::Bai)
    })?;
    let refs: Vec<(String, usize)> = header
        .reference_sequences()
        .iter()
        .map(|(n, m)| (n.to_string(), m.length().get()))
        .collect();
    let mut out = vec![];
    for reg in crate::bamin::regions(&refs) {
        let region = reg.parse().unwrap();
        for r in rd.query(&header, &index, &region)?.records() {
            let r = r?;
            if r.flags()
                .intersects(Flags::SECONDARY.union(Flags::SUPPLEMENTARY))
            {
                continue;
            }
            let seq: Vec<u8> = r.sequence().iter().collect();
            if seq.len() >= MIN_HIT {
                out.push(seq);
            }
        }
    }
    Ok(out)
}

struct Call {
    gene: String,
    reads: usize,
    spanning: usize,
    dist: i64,
    names: Vec<String>,
    unsure: usize,
    seq: Vec<u8>,
}

pub fn run(args: &[String]) {
    if args.len() < 3 {
        eprintln!("usage: hla-wgs long DBDIR READS.fastq[.gz]|READS.bam OUTPREFIX [threads]");
        eprintln!(
            "       accurate long reads (PacBio HiFi); of a BAM the MHC region is read (index needed)"
        );
        std::process::exit(2);
    }
    let dir = args[0].trim_end_matches('/');
    let (input, out) = (&args[1], &args[2]);
    let threads: usize = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(4);
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .ok();
    let t0 = std::time::Instant::now();
    let log = |s: String| eprintln!("[{:6.1}s] {}", t0.elapsed().as_secs_f64(), s);
    let (db, mut reads) = rayon::join(
        || Db::load(&format!("{dir}/hladb.json.gz")),
        || {
            if input.to_ascii_lowercase().ends_with(".bam") {
                read_bam(input, threads).unwrap_or_else(|e| panic!("{input}: {e}"))
            } else {
                read_fastq(input)
            }
        },
    );
    for r in &mut reads {
        r.make_ascii_uppercase();
    }
    log(format!(
        "{} reads, {:.1} Mb",
        reads.len(),
        reads.iter().map(|r| r.len()).sum::<usize>() as f64 / 1e6
    ));
    // 1. parts of reads that hold a gene
    let panel = Aligner::builder()
        .map_hifi()
        .with_cigar()
        .with_index_threads(threads)
        .with_index(format!("{dir}/panel.fa"), None)
        .expect("panel.fa");
    let hits: Vec<Vec<(String, bool, usize, usize, usize, usize, usize)>> = reads
        .par_iter()
        .map(|s| {
            panel
                .map(s, false, false, None, None, None)
                .unwrap_or_default()
                .into_iter()
                .filter(|m| m.is_primary || m.is_supplementary)
                .filter_map(|m| {
                    let (ts, te, tl) = (
                        m.target_start as usize,
                        m.target_end as usize,
                        m.target_len as usize,
                    );
                    if te - ts < MIN_HIT || ((te - ts) as f64) < MIN_SPAN * tl as f64 {
                        return None;
                    }
                    let g = m.target_name.as_ref()?.split('*').next()?.to_string();
                    Some((
                        g,
                        m.strand == Strand::Forward,
                        m.query_start as usize,
                        m.query_end as usize,
                        ts,
                        te,
                        tl,
                    ))
                })
                .collect()
        })
        .collect();
    let mut fams: std::collections::BTreeMap<String, Vec<Seg>> = Default::default();
    for (ri, hs) in hits.iter().enumerate() {
        let s = &reads[ri];
        for (g, fwd, qs, qe, ts, te, tl) in hs {
            let (lo, hi) = if *fwd { (*ts, tl - te) } else { (tl - te, *ts) };
            let a = qs.saturating_sub(FLANK + lo);
            let b = (qe + FLANK + hi).min(s.len());
            let v = fams.entry(family(g).to_string()).or_default();
            // one part per read region: a longer hit replaces an overlapping shorter one
            let clash = v.iter().position(|x| {
                x.read == ri
                    && 2 * (b.min(x.b) as i64 - a.max(x.a) as i64) > (b - a).min(x.b - x.a) as i64
            });
            if let Some(k) = clash {
                if v[k].hit >= te - ts {
                    continue;
                }
                v.swap_remove(k);
            }
            let seq = if *fwd {
                s[a..b].to_vec()
            } else {
                revcomp(&s[a..b])
            };
            v.push(Seg {
                seq,
                span: (te - ts) as f64 / *tl as f64,
                gene: g.clone(),
                read: ri,
                a,
                b,
                hit: te - ts,
            });
        }
    }
    for v in fams.values_mut() {
        v.sort_by_key(|x| (x.read, x.a));
    }
    log(format!(
        "gene parts: {}",
        fams.iter()
            .map(|(f, v)| format!("{f} {}", v.len()))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    // 2.-4. haplotypes, consensus, naming
    let mut calls: Vec<Call> = vec![];
    for (fam, segs) in &fams {
        let tf = std::time::Instant::now();
        let clusters = split((0..segs.len()).collect(), segs, 0);
        let (t_split, mut t_pol, mut t_db) = (tf.elapsed().as_secs_f64(), 0.0, 0.0);
        let fgenes: Vec<&String> = db.genes.iter().filter(|g| family(g) == fam).collect();
        let mut found: Vec<Call> = vec![];
        for cl in clusters.iter().filter(|c| c.len() >= 4) {
            let tp = std::time::Instant::now();
            let (cons, unsure) = polish(
                extend(segs[template(segs, cl)].seq.clone(), segs, cl),
                segs,
                cl,
            );
            t_pol += tp.elapsed().as_secs_f64();
            let tp = std::time::Instant::now();
            // genes worth comparing: the whole family, for class I those the panel named for these reads
            let mut lab: AHashMap<&str, usize> = AHashMap::new();
            for &i in cl {
                *lab.entry(segs[i].gene.as_str()).or_insert(0) += 1;
            }
            let genes: Vec<&String> = if fam == "I" {
                fgenes
                    .iter()
                    .copied()
                    .filter(|g| 10 * lab.get(g.as_str()).copied().unwrap_or(0) >= cl.len())
                    .collect()
            } else {
                fgenes.clone()
            };
            let al = Aligner::builder()
                .asm5()
                .with_cigar()
                .with_seq(&cons)
                .expect("index");
            // only alleles that share most sampled 21-mers with the consensus are aligned
            let cset: ahash::AHashSet<u64> = sketch(&cons).into_iter().collect();
            let cand: Vec<&crate::db::Allele> = genes
                .iter()
                .flat_map(|g| db.ref_[g.as_str()].iter())
                .filter(|a| !a.imputed && !a.partial)
                .collect();
            let mut share: Vec<(f64, usize)> = cand
                .par_iter()
                .enumerate()
                .map(|(i, a)| {
                    let q: &[u8] = if a.extended {
                        a.orig.as_deref().unwrap_or(&a.genomic)
                    } else {
                        &a.genomic
                    };
                    let k = sketch(q);
                    (
                        k.iter().filter(|x| cset.contains(x)).count() as f64
                            / k.len().max(1) as f64,
                        i,
                    )
                })
                .collect();
            share.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap().then(x.1.cmp(&y.1)));
            let top = share.first().map_or(0.0, |x| x.0);
            let cut = (top - 0.02).min(share.get(49).map_or(0.0, |x| x.0));
            let pick: Vec<&crate::db::Allele> = share
                .iter()
                .filter(|x| x.0 >= cut && x.0 > 0.5)
                .map(|x| cand[x.1])
                .collect();
            let mut best: Vec<(i64, &str, usize, usize)> = pick
                .par_iter()
                .filter_map(|a| {
                    let q: &[u8] = if a.extended {
                        a.orig.as_deref().unwrap_or(&a.genomic)
                    } else {
                        &a.genomic
                    };
                    let hs = al.map(q, false, false, None, None, None).ok()?;
                    let m = hs
                        .into_iter()
                        .filter(|m| m.strand == Strand::Forward)
                        .max_by_key(|m| m.match_len)?;
                    let d = m.alignment.as_ref()?.nm as i64 + m.query_start as i64 + q.len() as i64
                        - m.query_end as i64;
                    Some((
                        d,
                        a.name.as_str(),
                        m.target_start as usize,
                        m.target_end as usize,
                    ))
                })
                .collect();
            best.sort();
            t_db += tp.elapsed().as_secs_f64();
            let Some(&(d0, n0, s0, e0)) = best.first() else {
                continue;
            };
            // parts of reads only, copies of pseudogenes and mixtures stay far from every allele
            if d0 > 20.max((e0 - s0) as i64 / 200) {
                continue;
            }
            found.push(Call {
                gene: n0.split('*').next().unwrap().to_string(),
                reads: cl.len(),
                spanning: cl.iter().filter(|&&i| segs[i].span >= SPANNING).count(),
                dist: d0,
                names: best
                    .iter()
                    .take_while(|b| b.0 == d0)
                    .map(|b| b.1.to_string())
                    .collect(),
                unsure: unsure.iter().filter(|&&u| s0 <= u && u < e0).count(),
                seq: cons[s0..e0].to_vec(),
            });
        }
        if std::env::var("HLA_LONG_TIME").is_ok() {
            log(format!(
                "{fam}: {} groups, split {t_split:.1}s, consensus {t_pol:.1}s, naming {t_db:.1}s",
                clusters.len()
            ));
        }
        // groups that name the same allele are one haplotype split by read noise
        found.sort_by(|x, y| y.reads.cmp(&x.reads).then(x.names.cmp(&y.names)));
        let mut kept: Vec<Call> = vec![];
        for c in found {
            if let Some(k) = kept
                .iter_mut()
                .find(|k| k.gene == c.gene && k.names.iter().any(|n| c.names.contains(n)))
            {
                k.reads += c.reads;
                k.spanning += c.spanning;
            } else {
                kept.push(c);
            }
        }
        calls.append(&mut kept);
    }
    calls.sort_by(|x, y| x.gene.cmp(&y.gene).then(y.reads.cmp(&x.reads)));
    let mut tsv =
        std::io::BufWriter::new(std::fs::File::create(format!("{out}.long.tsv")).unwrap());
    let mut fa = std::io::BufWriter::new(std::fs::File::create(format!("{out}.long.fa")).unwrap());
    writeln!(tsv, "gene\tslot\tallele\tdifferences\texact\treads\tspanning\tunsure_sites\tequal_alternatives\tnote").unwrap();
    let mut n = 0;
    let mut gene_slot: AHashMap<String, (usize, usize)> = AHashMap::new();
    for c in &calls {
        let e = gene_slot.entry(c.gene.clone()).or_insert((0, c.reads));
        e.0 += 1;
        let (slot, top) = *e;
        let note = if slot > 2 || c.reads < 5.max(top / 5) {
            "minor"
        } else {
            ""
        };
        writeln!(
            tsv,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            c.gene,
            slot,
            c.names[0],
            c.dist,
            if c.dist == 0 && c.unsure == 0 {
                "yes"
            } else if c.dist == 0 {
                "yes, with unsure sites"
            } else {
                "no"
            },
            c.reads,
            c.spanning,
            c.unsure,
            if c.names.len() > 1 {
                c.names[1..c.names.len().min(6)].join(",")
            } else {
                "-".into()
            },
            if note.is_empty() { "-" } else { note }
        )
        .unwrap();
        if note.is_empty() {
            writeln!(
                fa,
                ">{}_slot{} nearest={} differences={} reads={}\n{}",
                c.gene,
                slot,
                c.names[0],
                c.dist,
                c.reads,
                String::from_utf8_lossy(&c.seq)
            )
            .unwrap();
            n += 1;
        }
    }
    log(format!(
        "{n} sequences written to {out}.long.fa, table {out}.long.tsv"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_are_read_and_applied() {
        let ev = parse_cs(":3*ag:2+tt:1-c:2", 10);
        assert_eq!(
            ev,
            vec![
                (13, b'S', "G".to_string()),
                (16, b'I', "TT".to_string()),
                (17, b'D', "1".to_string())
            ]
        );
        let t = b"AAAAAAAAAACCCACCGCAA".to_vec();
        let mut e = ev.clone();
        assert_eq!(apply(&t, &mut e), b"AAAAAAAAAACCCGCCTTGAA".to_vec());
    }

    #[test]
    fn sketch_is_a_subset_of_kmers() {
        let s: Vec<u8> = (0..400).map(|i| b"ACGT"[(i * 7 + i / 3) % 4]).collect();
        let k = sketch(&s);
        assert!(k.len() < 380);
        assert_eq!(k, sketch(&s));
    }
}
