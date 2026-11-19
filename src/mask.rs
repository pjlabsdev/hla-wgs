//! `hla-wgs build-mask DBDIR GENOME.fa [EXCLUDE.bed]`: mark the parts of the panel alleles that
//! also occur elsewhere in the reference genome (dispersed repeats such as Alu and L1 inside the
//! introns). A read pair counts for a gene only if one mate aligns outside these intervals.
//!
//! Every 21-mer of the panel is counted in the genome outside the MHC (GRCh38 chr6:28,510,119-
//! 33,480,577 unless an exclude BED is given) and outside `_alt` and `HLA*` contigs. Positions whose
//! 21-mer occurs at least three times are joined over gaps of up to 60 bases; intervals of at least
//! 60 bases are written to `DBDIR/panel.fa.mask.bed` (allele, start, end).
use ahash::AHashMap;
use std::io::{BufRead, BufReader, Write};

const K: usize = 21;
const MIN_COUNT: u32 = 3;
const GAP: usize = 60;
const MIN_LEN: usize = 60;
/// MHC on GRCh38 chromosome 6 (1-based, inclusive end)
const MHC38: (usize, usize) = (28_510_119, 33_480_577);

fn code(b: u8) -> u64 {
    match b {
        b'A' | b'a' => 0,
        b'C' | b'c' => 1,
        b'G' | b'g' => 2,
        b'T' | b't' => 3,
        _ => 4,
    }
}

fn kmers(seq: &[u8], mut f: impl FnMut(usize, u64)) {
    let mask = (1u64 << (2 * K)) - 1;
    let (mut fw, mut rv, mut n) = (0u64, 0u64, 0usize);
    for (i, &b) in seq.iter().enumerate() {
        let c = code(b);
        if c > 3 {
            n = 0;
            fw = 0;
            rv = 0;
            continue;
        }
        fw = ((fw << 2) | c) & mask;
        rv = (rv >> 2) | ((3 - c) << (2 * (K - 1)));
        n += 1;
        if n >= K {
            f(i + 1 - K, fw.min(rv));
        }
    }
}

/// intervals [start, end) of one allele from its per-position 21-mer counts
pub fn intervals(counts: &[u32]) -> Vec<(usize, usize)> {
    let mut iv: Vec<(usize, usize)> = vec![];
    for (i, &c) in counts.iter().enumerate() {
        if c >= MIN_COUNT {
            match iv.last_mut() {
                Some(l) if i <= l.1 + GAP => l.1 = i + K,
                _ => iv.push((i, i + K)),
            }
        }
    }
    iv.into_iter().filter(|x| x.1 - x.0 >= MIN_LEN).map(|(a, b)| (a, b.min(counts.len()))).collect()
}

pub fn run(args: &[String]) {
    if args.len() < 2 {
        eprintln!("usage: hla-wgs build-mask DBDIR GENOME.fa [EXCLUDE.bed]");
        eprintln!("       GENOME.fa: the reference the reads were (or would be) aligned to, plain FASTA");
        eprintln!("       EXCLUDE.bed: regions to leave out instead of the GRCh38 MHC (for other builds)");
        std::process::exit(2);
    }
    let (dir, genome) = (&args[0], &args[1]);
    let panel = format!("{dir}/panel.fa");
    let mut excl: AHashMap<String, Vec<(usize, usize)>> = AHashMap::new();
    if let Some(bed) = args.get(2) {
        for l in std::fs::read_to_string(bed).unwrap_or_else(|e| panic!("{bed}: {e}")).lines() {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() >= 3 {
                excl.entry(f[0].to_string()).or_default().push((f[1].parse().unwrap(), f[2].parse().unwrap()));
            }
        }
    } else {
        for c in ["chr6", "6"] {
            excl.insert(c.to_string(), vec![(MHC38.0 - 1, MHC38.1)]);
        }
    }
    let mut pan: Vec<(String, Vec<u8>)> = vec![];
    for l in BufReader::new(std::fs::File::open(&panel).unwrap_or_else(|e| panic!("{panel}: {e}"))).lines() {
        let l = l.unwrap();
        if let Some(n) = l.strip_prefix('>') {
            pan.push((n.split_whitespace().next().unwrap().to_string(), vec![]));
        } else if let Some(p) = pan.last_mut() {
            p.1.extend_from_slice(l.trim().as_bytes());
        }
    }
    let mut cnt: AHashMap<u64, u32> = AHashMap::new();
    for (_, s) in &pan {
        kmers(s, |_, k| {
            cnt.entry(k).or_insert(0);
        });
    }
    eprintln!("[mask] {} panel alleles, {} distinct 21-mers", pan.len(), cnt.len());
    let mask = (1u64 << (2 * K)) - 1;
    let mut cur: Vec<(usize, usize)> = vec![];
    let (mut skip, mut pos) = (false, 0usize);
    let (mut fw, mut rv, mut n) = (0u64, 0u64, 0usize);
    let f = std::fs::File::open(genome).unwrap_or_else(|e| panic!("{genome}: {e}"));
    for l in BufReader::with_capacity(1 << 22, f).split(b'\n') {
        let l = l.unwrap();
        if l.first() == Some(&b'>') {
            let name = String::from_utf8_lossy(&l[1..]).split_whitespace().next().unwrap_or("").to_string();
            skip = name.contains("_alt") || name.starts_with("HLA");
            cur = excl.get(&name).cloned().unwrap_or_default();
            pos = 0;
            n = 0;
            fw = 0;
            rv = 0;
            continue;
        }
        if skip {
            continue;
        }
        for &b in &l {
            if b == b'\r' {
                continue;
            }
            pos += 1;
            let c = code(b);
            if c > 3 || (!cur.is_empty() && cur.iter().any(|&(s, e)| pos > s && pos <= e)) {
                n = 0;
                fw = 0;
                rv = 0;
                continue;
            }
            fw = ((fw << 2) | c) & mask;
            rv = (rv >> 2) | ((3 - c) << (2 * (K - 1)));
            n += 1;
            if n >= K {
                if let Some(v) = cnt.get_mut(&fw.min(rv)) {
                    *v = v.saturating_add(1);
                }
            }
        }
    }
    let out = format!("{panel}.mask.bed");
    let mut o = std::io::BufWriter::new(std::fs::File::create(&out).unwrap_or_else(|e| panic!("{out}: {e}")));
    let (mut ni, mut na) = (0usize, 0usize);
    for (name, s) in &pan {
        let mut v = vec![0u32; s.len()];
        kmers(s, |i, k| v[i] = cnt[&k]);
        let iv = intervals(&v);
        if !iv.is_empty() {
            na += 1;
        }
        for (a, b) in iv {
            writeln!(o, "{name}\t{a}\t{b}").unwrap();
            ni += 1;
        }
    }
    eprintln!("[mask] wrote {out}: {ni} intervals on {na} alleles");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_close_hits_and_drops_short_runs() {
        let mut c = vec![0u32; 500];
        // a lone hit gives a 21-base interval: too short
        c[10] = 5;
        // hits 50 apart are joined: 100..(190+21)
        for i in [100, 150, 190] {
            c[i] = 3;
        }
        // below the count threshold
        c[300] = 2;
        assert_eq!(intervals(&c), vec![(100, 211)]);
    }

    #[test]
    fn interval_is_clamped_to_the_sequence() {
        let mut c = vec![0u32; 100];
        for i in 30..100 {
            c[i] = 9;
        }
        assert_eq!(intervals(&c), vec![(30, 100)]);
    }

    #[test]
    fn canonical_kmer_is_strand_independent() {
        let s = b"ACGTTGCAAGGCTTAACCGGTTAAC";
        let rc: Vec<u8> = s.iter().rev().map(|b| match b { b'A' => b'T', b'C' => b'G', b'G' => b'C', _ => b'A' }).collect();
        let (mut a, mut b) = (vec![], vec![]);
        kmers(s, |_, k| a.push(k));
        kmers(&rc, |_, k| b.push(k));
        b.reverse();
        assert_eq!(a, b);
    }
}
