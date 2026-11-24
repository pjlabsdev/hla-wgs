//! Rust kernel of v16 (formerly the hlakern binary): reads x sequences matrix of summed mate
//! semi-global edit distances; 999 = mate without alignment. Plus the Python-side normalisation.
use rayon::prelude::*;

const K: usize = 15;
const STEP: usize = 3; // query k-mer sampling step
const BAND: usize = 24; // extra text on each side of the anchored window (indel room)
const NOHIT: u16 = 999;
pub const MAXED: u32 = 20; // per-mate cost above this counts as not aligned (HLAKERN_MAXED=20 in v16)

fn code(c: u8) -> u8 {
    match c {
        b'A' | b'a' => 0,
        b'C' | b'c' => 1,
        b'G' | b'g' => 2,
        b'T' | b't' => 3,
        _ => 4,
    }
}

fn revcomp(s: &[u8]) -> Vec<u8> {
    s.iter()
        .rev()
        .map(|&c| match c {
            0 => 3,
            1 => 2,
            2 => 1,
            3 => 0,
            x => x,
        })
        .collect()
}

fn kmer_at(s: &[u8], i: usize) -> Option<u64> {
    let mut v: u64 = 0;
    for &c in &s[i..i + K] {
        if c > 3 {
            return None;
        }
        v = (v << 2) | c as u64;
    }
    Some(v)
}

struct Target {
    seq: Vec<u8>,
}

/// one k-mer index over all targets: sorted (kmer, target, pos); k-mers occurring more than
/// 8 times within one target are dropped (repeats)
struct Index {
    /// k-mer -> range in hits
    at: ahash::AHashMap<u64, (u32, u32)>,
    hits: Vec<(u32, u32)>,
}

impl Index {
    fn new(targets: &[Target]) -> Self {
        let maxlen = targets.iter().map(|t| t.seq.len()).max().unwrap_or(0);
        if targets.len() >= 1 << TBITS || maxlen >= 1 << PBITS {
            return Self::new_tuples(targets);
        }
        // (k-mer, target, position) packed into one u64 that sorts in the same order as the tuple
        let mut all: Vec<u64> = Vec::with_capacity(targets.iter().map(|t| t.seq.len()).sum());
        let mask = (1u64 << (2 * K)) - 1;
        for (ti, t) in targets.iter().enumerate() {
            let (mut v, mut l) = (0u64, 0usize);
            for (i, &c) in t.seq.iter().enumerate() {
                if c > 3 {
                    l = 0;
                    continue;
                }
                v = ((v << 2) | c as u64) & mask;
                l += 1;
                if l >= K {
                    all.push(v << (TBITS + PBITS) | (ti as u64) << PBITS | (i + 1 - K) as u64);
                }
            }
        }
        radix_sort(&mut all);
        // k-mers occurring more than 8 times within one target are dropped: (k-mer, target) runs
        let pm = (1u64 << PBITS) - 1;
        let mut hits: Vec<(u32, u32)> = Vec::with_capacity(all.len());
        let mut at = ahash::AHashMap::with_capacity(all.len() / 2 + 1);
        let mut a = 0;
        while a < all.len() {
            let k = all[a] >> (TBITS + PBITS);
            let st = hits.len();
            let mut b = a;
            while b < all.len() && all[b] >> (TBITS + PBITS) == k {
                let kt = all[b] >> PBITS;
                let mut c = b;
                while c < all.len() && all[c] >> PBITS == kt {
                    c += 1;
                }
                if c - b <= 8 {
                    let t = (kt & ((1u64 << TBITS) - 1)) as u32;
                    hits.extend(all[b..c].iter().map(|&x| (t, (x & pm) as u32)));
                }
                b = c;
            }
            if hits.len() > st {
                at.insert(k, (st as u32, (hits.len() - st) as u32));
            }
            a = b;
        }
        Index { at, hits }
    }

    /// the same with tuples, for more than 4095 targets or targets of 4 Mb and more
    fn new_tuples(targets: &[Target]) -> Self {
        let mut all: Vec<(u64, u32, u32)> = targets
            .par_iter()
            .enumerate()
            .flat_map_iter(|(ti, t)| {
                let mut v: Vec<(u64, u32, u32)> = vec![];
                if t.seq.len() >= K {
                    for i in 0..=t.seq.len() - K {
                        if let Some(k) = kmer_at(&t.seq, i) {
                            v.push((k, ti as u32, i as u32));
                        }
                    }
                }
                v.sort_unstable();
                let mut out = vec![];
                let mut a = 0;
                while a < v.len() {
                    let mut b = a;
                    while b < v.len() && v[b].0 == v[a].0 {
                        b += 1;
                    }
                    if b - a <= 8 {
                        out.extend_from_slice(&v[a..b]);
                    }
                    a = b;
                }
                out.into_iter()
            })
            .collect();
        all.par_sort_unstable();
        let mut at = ahash::AHashMap::with_capacity(all.len() / 2 + 1);
        let mut a = 0;
        while a < all.len() {
            let mut b = a + 1;
            while b < all.len() && all[b].0 == all[a].0 {
                b += 1;
            }
            at.insert(all[a].0, (a as u32, (b - a) as u32));
            a = b;
        }
        Index { at, hits: all.iter().map(|x| (x.1, x.2)).collect() }
    }
    fn get(&self, k: u64) -> &[(u32, u32)] {
        match self.at.get(&k) {
            Some(&(a, n)) => &self.hits[a as usize..(a + n) as usize],
            None => &[],
        }
    }
}

const TBITS: u32 = 12;
const PBITS: u32 = 64 - 2 * K as u32 - TBITS;

/// LSD radix sort of u64 values, 16 bits per pass (small inputs: comparison sort)
fn radix_sort(v: &mut Vec<u64>) {
    if v.len() < 1 << 14 {
        v.sort_unstable();
        return;
    }
    let mut tmp = vec![0u64; v.len()];
    let mut cnt = vec![0usize; 1 << 16];
    for pass in 0..4 {
        let sh = pass * 16;
        cnt.iter_mut().for_each(|c| *c = 0);
        for &x in v.iter() {
            cnt[(x >> sh & 0xffff) as usize] += 1;
        }
        let mut sum = 0;
        for c in cnt.iter_mut() {
            let n = *c;
            *c = sum;
            sum += n;
        }
        for &x in v.iter() {
            let d = (x >> sh & 0xffff) as usize;
            tmp[cnt[d]] = x;
            cnt[d] += 1;
        }
        std::mem::swap(v, &mut tmp);
    }
}

/// per target: up to two best (strand, diagonal) anchors for one mate
fn anchors(idx: &Index, nt: usize, fw: &[u8], rc: &[u8]) -> Vec<Vec<(u8, i64)>> {
    // votes as (target, strand, diagonal bucket), grouped by target with a counting sort
    let mut h: Vec<(u32, u8, i64)> = vec![];
    for (st, q) in [fw, rc].iter().enumerate() {
        if q.len() < K {
            continue;
        }
        let mut i = 0;
        while i + K <= q.len() {
            if let Some(k) = kmer_at(q, i) {
                for &(t, p) in idx.get(k) {
                    h.push((t, st as u8, (p as i64 - i as i64).div_euclid(8)));
                }
            }
            i += STEP;
        }
    }
    let mut start = vec![0u32; nt + 1];
    for x in &h {
        start[x.0 as usize + 1] += 1;
    }
    for t in 0..nt {
        start[t + 1] += start[t];
    }
    let mut fill = start.clone();
    let mut g: Vec<(u8, i64)> = vec![(0, 0); h.len()];
    for x in &h {
        let f = &mut fill[x.0 as usize];
        g[*f as usize] = (x.1, x.2);
        *f += 1;
    }
    let mut out: Vec<Vec<(u8, i64)>> = vec![vec![]; nt];
    let mut cnt: Vec<(u32, u8, i64)> = vec![];
    for t in 0..nt {
        let v = &mut g[start[t] as usize..start[t + 1] as usize];
        if v.is_empty() {
            continue;
        }
        v.sort_unstable();
        cnt.clear();
        let mut a = 0;
        while a < v.len() {
            let mut b = a;
            while b < v.len() && v[b] == v[a] {
                b += 1;
            }
            cnt.push(((b - a) as u32, v[a].0, v[a].1));
            a = b;
        }
        // top two diagonals per strand (as many votes first, then lower diagonal)
        cnt.sort_by(|x, y| x.1.cmp(&y.1).then(y.0.cmp(&x.0)).then(x.2.cmp(&y.2)));
        let mut sel = vec![];
        for st in 0..2u8 {
            sel.extend(cnt.iter().filter(|c| c.1 == st).take(2).map(|c| (c.1, c.2 * 8)));
        }
        out[t] = sel;
    }
    out
}

/// Myers/Hyyro bit-parallel semi-global edit distance (myers_peq with the pattern's peq_of): pattern q fully aligned,
/// free start/end in text. Text symbol 4 (N / padding) matches everything.
/// match vectors of a pattern per text symbol 0..4; padding rows (beyond m) match everything
fn peq_of(q: &[u8]) -> Vec<[u64; 5]> {
    let m = q.len();
    let w = (m + 63) / 64;
    let mut peq = vec![[0u64; 5]; w];
    for b in 0..w {
        for c in 0..5u8 {
            let mut v = 0u64;
            for r in 0..64 {
                let i = b * 64 + r;
                if i >= m || c == 4 || q[i] == c || q[i] > 3 {
                    v |= 1u64 << r;
                }
            }
            peq[b][c as usize] = v;
        }
    }
    peq
}

fn myers_peq(peq: &[[u64; 5]], m: usize, text: &[u8]) -> u32 {
    let w = peq.len();
    let mut pv = vec![!0u64; w];
    let mut mv = vec![0u64; w];
    let mut score: i64 = m as i64; // score at row m (bit (m-1)%64 of the last block)
    let mut best = score;
    let lastbit = ((m - 1) % 64) as u32;
    for &tc in text {
        let c = if tc > 3 { 4 } else { tc as usize };
        let mut hin: i64 = 0; // free start in text: top row is 0
        let mut dlast: i64 = 0;
        for b in 0..w {
            let mut eq = peq[b][c];
            let p = pv[b];
            let mm = mv[b];
            let xv = eq | mm;
            if hin < 0 {
                eq |= 1;
            }
            let xh = (((eq & p).wrapping_add(p)) ^ p) | eq;
            let mut ph = mm | !(xh | p);
            let mut mh = p & xh;
            let mut hout = 0i64;
            if ph >> 63 == 1 {
                hout = 1;
            }
            if mh >> 63 == 1 {
                hout = -1;
            }
            if b == w - 1 {
                dlast = ((ph >> lastbit) & 1) as i64 - ((mh >> lastbit) & 1) as i64;
            }
            ph <<= 1;
            mh <<= 1;
            if hin < 0 {
                mh |= 1;
            } else if hin > 0 {
                ph |= 1;
            }
            pv[b] = mh | !(xv | ph);
            mv[b] = ph & xv;
            hin = hout;
        }
        score += dlast;
        if score < best {
            best = score;
        }
    }
    best as u32
}

/// best cost of one mate against one target given its anchors; windows already scored for
/// this mate are reused (alleles of one gene share most windows)
fn mate_cost(
    t: &Target,
    q: [&[u8]; 2],
    peq: &[Vec<[u64; 5]>; 2],
    anc: &[(u8, i64)],
    cache: &mut ahash::AHashMap<(u8, Vec<u8>), u32>,
    win: &mut Vec<u8>,
) -> u16 {
    let mut best: Option<u32> = None;
    let l = t.seq.len() as i64;
    for &(st, d) in anc {
        let qq = q[st as usize];
        let s = d - BAND as i64;
        let e = d + qq.len() as i64 + BAND as i64 + 8;
        // text window; positions outside the target are wildcards (free overhang)
        win.clear();
        if s >= 0 && e <= l {
            win.extend_from_slice(&t.seq[s as usize..e as usize]);
        } else {
            for p in s..e {
                win.push(if p < 0 || p >= l { 4 } else { t.seq[p as usize] });
            }
        }
        let key = (st, std::mem::take(win));
        let c = match cache.get(&key) {
            Some(&c) => {
                *win = key.1;
                c
            }
            None => {
                let c = myers_peq(&peq[st as usize], qq.len(), &key.1);
                cache.insert(key, c);
                c
            }
        };
        if best.map_or(true, |b| c < b) {
            best = Some(c);
        }
    }
    match best {
        Some(c) if c <= MAXED => c.min(NOHIT as u32 - 1) as u16,
        _ => NOHIT,
    }
}


/// raw matrix (row-major, reads x seqs), exactly as hlakern wrote it
pub fn raw(reads: &[[&[u8]; 2]], seqs: &[&[u8]]) -> Vec<u16> {
    let targets: Vec<Target> = seqs.iter().map(|s| Target { seq: s.iter().map(|&c| code(c)).collect() }).collect();
    let idx = Index::new(&targets);
    let nt = targets.len();
    let rows: Vec<Vec<u16>> = reads
        .par_iter()
        .map(|mates| {
            let mut row = vec![0u16; nt];
            for m in mates.iter() {
                if m.is_empty() {
                    continue;
                }
                let m: Vec<u8> = m.iter().map(|&c| code(c)).collect();
                let rc = revcomp(&m);
                let anc = anchors(&idx, nt, &m, &rc);
                let peq = [peq_of(&m), peq_of(&rc)];
                let mut cache: ahash::AHashMap<(u8, Vec<u8>), u32> = ahash::AHashMap::new();
                let mut win = vec![];
                for (j, t) in targets.iter().enumerate() {
                    let c = mate_cost(t, [m.as_slice(), rc.as_slice()], &peq, &anc[j], &mut cache, &mut win);
                    row[j] = row[j].saturating_add(c);
                }
            }
            row
        })
        .collect();
    rows.concat()
}

pub const UNMAPPED_EXTRA: f64 = 3.0;

/// edit_matrix_rs of the prototype: rows of reads with at least one aligned cell; a cell with
/// an unaligned mate costs row-min + 3 per unaligned mate. Returns (matrix rows, used read ids).
pub fn edit_matrix(reads: &crate::panel::Reads, qnames: &[usize], seqs: &[&[u8]]) -> Option<(Vec<Vec<f64>>, Vec<usize>)> {
    let qs: Vec<usize> = qnames.iter().copied().filter(|&q| reads.get(q).is_some()).collect();
    if qs.is_empty() || seqs.is_empty() {
        return None;
    }
    let mates: Vec<[&[u8]; 2]> = qs
        .iter()
        .map(|&q| {
            let r = &reads.seq[q];
            [r[0].as_deref().unwrap_or(&[]), r[1].as_deref().unwrap_or(&[])]
        })
        .collect();
    let raw = raw(&mates, seqs);
    let n = seqs.len();
    let mut out = vec![];
    let mut used = vec![];
    for (k, &q) in qs.iter().enumerate() {
        let row = &raw[k * n..(k + 1) * n];
        if row.iter().all(|&v| v >= 999) {
            continue;
        }
        let mn = row.iter().filter(|&&v| v < 999).min().copied().unwrap() as f64;
        out.push(row.iter().map(|&v| if v >= 999 { mn + UNMAPPED_EXTRA * (v / 999) as f64 } else { v as f64 }).collect());
        used.push(q);
    }
    if out.is_empty() { None } else { Some((out, used)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the packed radix index equals the tuple index (same k-mers, same hits in the same order)
    #[test]
    fn packed_index_equals_tuple_index() {
        let mut x: u64 = 7;
        let mut rnd = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for case in 0..20 {
            let nt = 1 + (rnd() % 40) as usize;
            let base: Vec<u8> = (0..2000 + case * 300).map(|_| (rnd() % 4) as u8).collect();
            let targets: Vec<Target> = (0..nt)
                .map(|_| {
                    let mut s = base.clone();
                    for _ in 0..(rnd() % 30) {
                        let i = (rnd() as usize) % s.len();
                        s[i] = (rnd() % 5) as u8;
                    }
                    // a repeat so that the more-than-8 filter is used
                    let r: Vec<u8> = s[100..115].to_vec();
                    for _ in 0..(rnd() % 12) {
                        s.extend_from_slice(&r);
                    }
                    Target { seq: s }
                })
                .collect();
            let a = Index::new(&targets);
            let b = Index::new_tuples(&targets);
            assert_eq!(a.at.len(), b.at.len());
            for (k, _) in b.at.iter() {
                assert_eq!(a.get(*k), b.get(*k));
            }
        }
    }
}
