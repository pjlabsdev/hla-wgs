//! Stage A: exon 31-mer pseudoalignment of the gene's reads to all alleles + EM -> candidates
use crate::db::Allele;
use crate::names::two;
use crate::panel::Reads;
use crate::util::revcomp;
use ahash::AHashMap;

const K: usize = 31;
const NCAND: usize = 40;

fn enc(c: u8) -> Option<u64> {
    match c {
        b'A' => Some(0),
        b'C' => Some(1),
        b'G' => Some(2),
        b'T' => Some(3),
        _ => None,
    }
}

/// all 31-mers of s in order (None for k-mers with a non-ACGT base)
fn kmers(s: &[u8]) -> Vec<Option<u64>> {
    let mut out = Vec::with_capacity(s.len().saturating_sub(K - 1));
    if s.len() < K {
        return out;
    }
    let mask = (1u64 << (2 * K)) - 1;
    let (mut v, mut bad_until) = (0u64, 0usize);
    for (i, &c) in s.iter().enumerate() {
        match enc(c) {
            Some(x) => v = ((v << 2) | x) & mask,
            None => {
                v = (v << 2) & mask;
                bad_until = i + K;
            }
        }
        if i + 1 >= K {
            out.push(if i < bad_until { None } else { Some(v) });
        }
    }
    out
}

pub struct Index {
    pub map: AHashMap<u64, Vec<u32>>,
}

pub fn build_index(al: &[Allele]) -> Index {
    // modal exon length per exon number; skip rare over-long forms
    let mut lencnt: AHashMap<u32, AHashMap<usize, usize>> = AHashMap::new();
    let mut order: AHashMap<u32, Vec<usize>> = AHashMap::new(); // first-seen order of lengths (Counter ties)
    for a in al {
        for (k, e) in &a.exons {
            let c = lencnt.entry(*k).or_default();
            let o = order.entry(*k).or_default();
            if !c.contains_key(&e.len()) {
                o.push(e.len());
            }
            *c.entry(e.len()).or_default() += 1;
        }
    }
    let mut modal: AHashMap<u32, usize> = AHashMap::new();
    let mut total: AHashMap<u32, usize> = AHashMap::new();
    for (k, c) in &lencnt {
        // Counter.most_common(1): highest count, first inserted on ties
        let mut best = (0usize, 0usize);
        for &l in &order[k] {
            if c[&l] > best.1 {
                best = (l, c[&l]);
            }
        }
        modal.insert(*k, best.0);
        total.insert(*k, c.values().sum());
    }
    let mut map: AHashMap<u64, Vec<u32>> = AHashMap::new();
    for (i, a) in al.iter().enumerate() {
        for (k, e) in &a.exons {
            if e.len() > modal[k] + 9 && (lencnt[k][&e.len()] as f64) < 0.005 * total[k] as f64 {
                continue;
            }
            for km in kmers(e).into_iter().flatten() {
                let v = map.entry(km).or_default();
                if v.last() != Some(&(i as u32)) {
                    v.push(i as u32);
                }
            }
        }
    }
    Index { map }
}

pub struct StageA {
    pub cand: Vec<usize>,
    pub log: String,
}

pub fn run(gene: &str, al: &[Allele], idx: &Index, reads: &Reads, qnames: &[usize]) -> StageA {
    let n = al.len();
    let nw = (n + 63) / 64;
    let mut bits: AHashMap<u64, Vec<u64>> = AHashMap::new();
    let mut eq_keys: Vec<Vec<u64>> = vec![];
    let mut eq_cnt: Vec<usize> = vec![];
    let mut eq_pos: AHashMap<Vec<u64>, usize> = AHashMap::new();
    for &q in qnames {
        let Some(r) = reads.get(q) else { continue };
        let mut cur: Option<Vec<u64>> = None;
        for s in r.iter().flatten() {
            for strand in [s.clone(), revcomp(s)] {
                let ks: Vec<u64> = kmers(&strand).into_iter().flatten().filter(|k| idx.map.contains_key(k)).collect();
                if ks.len() < 5 {
                    continue;
                }
                for km in ks {
                    let b = bits.entry(km).or_insert_with(|| {
                        let mut v = vec![0u64; nw];
                        for &i in &idx.map[&km] {
                            v[i as usize / 64] |= 1u64 << (i % 64);
                        }
                        v
                    });
                    let nb: Vec<u64> = match &cur {
                        None => b.clone(),
                        Some(c) => c.iter().zip(b.iter()).map(|(x, y)| x & y).collect(),
                    };
                    if nb.iter().any(|&x| x != 0) {
                        cur = Some(nb);
                    }
                }
                break;
            }
        }
        if let Some(c) = cur {
            match eq_pos.get(&c) {
                Some(&p) => eq_cnt[p] += 1,
                None => {
                    eq_pos.insert(c.clone(), eq_keys.len());
                    eq_keys.push(c);
                    eq_cnt.push(1);
                }
            }
        }
    }
    if eq_keys.is_empty() {
        return StageA { cand: vec![], log: String::new() };
    }
    let lens: Vec<f64> = al.iter().map(|a| a.exons.iter().map(|e| e.1.len()).sum::<usize>().max(1) as f64).collect();
    let classes: Vec<(Vec<usize>, f64)> = eq_keys
        .iter()
        .zip(eq_cnt.iter())
        .map(|(k, &c)| ((0..n).filter(|&i| k[i / 64] >> (i % 64) & 1 == 1).collect(), c as f64))
        .collect();
    let mut theta = vec![1.0 / n as f64; n];
    for _ in 0..200 {
        let mut acc = vec![0.0f64; n];
        let mut w = vec![0.0f64; n];
        for (m, c) in &classes {
            // numpy: w = theta * m / L over all alleles; s = w.sum(); acc += c * w / s
            for x in w.iter_mut() {
                *x = 0.0;
            }
            for &i in m {
                w[i] = theta[i] / lens[i];
            }
            let s = pairwise(&w);
            if s > 0.0 {
                for &i in m {
                    acc[i] += c * w[i] / s;
                }
            }
        }
        let tot = np_sum(acc.iter().copied());
        let new: Vec<f64> = acc.iter().map(|x| x / tot).collect();
        let d = np_sum(new.iter().zip(theta.iter()).map(|(a, b)| (a - b).abs()));
        theta = new;
        if d < 1e-6 {
            break;
        }
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| theta[b].partial_cmp(&theta[a]).unwrap());
    // 2-field group masses in first-allele order; most_common(8) with stable ties
    let mut gnames: Vec<String> = vec![];
    let mut gpos: AHashMap<String, usize> = AHashMap::new();
    let mut gval: Vec<f64> = vec![];
    for i in 0..n {
        let t = two(&al[i].name);
        let p = *gpos.entry(t.clone()).or_insert_with(|| {
            gnames.push(t.clone());
            gval.push(0.0);
            gnames.len() - 1
        });
        gval[p] += theta[i];
    }
    let tot: f64 = gval.iter().sum();
    let mut gi: Vec<usize> = (0..gnames.len()).collect();
    gi.sort_by(|&a, &b| gval[b].partial_cmp(&gval[a]).unwrap());
    let top: Vec<usize> = gi.into_iter().take(8).collect();
    let log = format!(
        "{} groups [{}] classes {} reads_in_eq {}",
        gene,
        top.iter().map(|&g| format!("('{}', {:.3})", gnames[g], gval[g] / tot)).collect::<Vec<_>>().join(", "),
        eq_keys.len(),
        eq_cnt.iter().sum::<usize>()
    );
    let groups: Vec<&String> = top.iter().filter(|&&g| gval[g] >= 0.01 * tot).map(|&g| &gnames[g]).collect();
    let mut cand: Vec<usize> = vec![];
    for g in groups {
        let mut reps: Vec<(&[(u32, Vec<u8>)], usize, bool)> = vec![];
        for &i in order.iter().filter(|&&i| two(&al[i].name) == *g) {
            let a = &al[i];
            match reps.iter_mut().find(|r| r.0 == a.exons.as_slice()) {
                None => reps.push((a.exons.as_slice(), i, a.imputed)),
                Some(r) => {
                    if r.2 && !a.imputed {
                        r.1 = i;
                        r.2 = a.imputed;
                    }
                }
            }
        }
        let mut rl: Vec<usize> = reps.iter().map(|r| r.1).collect();
        rl.sort_by(|&a, &b| theta[b].partial_cmp(&theta[a]).unwrap());
        cand.extend(rl.into_iter().take(30));
    }
    for &i in order.iter().take(NCAND) {
        if theta[i] > 0.0 && !cand.contains(&i) {
            cand.push(i);
        }
    }
    cand.truncate(200);
    StageA { cand, log }
}

/// numpy-style pairwise summation (blocks of 8, unrolled) for bit-closer parity with np.sum
pub fn np_sum(it: impl Iterator<Item = f64>) -> f64 {
    let v: Vec<f64> = it.collect();
    pairwise(&v)
}
fn pairwise(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut s = 0.0;
        for &x in a {
            s += x;
        }
        s
    } else if n <= 128 {
        let mut r = [a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]];
        let mut i = 8;
        while i < n - n % 8 {
            for j in 0..8 {
                r[j] += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        pairwise(&a[..n2]) + pairwise(&a[n2..])
    }
}
