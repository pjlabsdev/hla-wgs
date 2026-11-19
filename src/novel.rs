//! Residual variants: positions where the reads consistently disagree with the called allele.
//! A called allele with such a site is the nearest database allele of a new allele, so its
//! fourth field (or more, if the site is exonic) is not an exact statement.
use crate::align::{Preset, aligner};
use crate::stageb::Ctx;
use ahash::AHashMap;
use minimap2::Strand;

pub struct Site {
    pub pos: usize,
    pub what: String,
    pub alt: usize,
    pub depth: usize,
    pub exon: bool,
    /// seen only in reads that fit both called alleles equally: the site belongs to one of the two
    pub either: bool,
}

struct Aln {
    nm: i64,
    st: usize,
    en: usize,
    /// (position on the allele, description)
    diffs: Vec<(usize, String)>,
    /// edits without the clip penalty
    raw: i64,
    /// allele positions where a long piece of the read was cut off
    clips: Vec<usize>,
}

fn revcomp(s: &[u8]) -> Vec<u8> {
    s.iter()
        .rev()
        .map(|c| match c {
            b'A' => b'T',
            b'C' => b'G',
            b'G' => b'C',
            b'T' => b'A',
            _ => b'N',
        })
        .collect()
}

fn parse_cs(cs: &str, mut pos: usize) -> Vec<(usize, String)> {
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
                out.push((
                    pos,
                    format!("{}>{}", v[..1].to_uppercase(), v[1..].to_uppercase()),
                ));
                pos += 1;
            }
            b'-' => {
                out.push((pos, format!("del{}", v.to_uppercase())));
                pos += v.len();
            }
            b'+' => out.push((pos, format!("ins{}", v.to_uppercase()))),
            _ => {}
        }
    }
    out
}

fn best(al: &minimap2::Aligner<minimap2::Built>, q: &[u8], t: &[u8]) -> Option<Aln> {
    let hs = al.map(q, true, false, None, None, None).ok()?;
    let m = hs
        .into_iter()
        .filter(|m| m.is_primary)
        .max_by_key(|m| m.match_len)?;
    let a = m.alignment.as_ref()?;
    let fwd = m.strand == Strand::Forward;
    // query in target orientation
    let qo: Vec<u8> = if fwd { q.to_vec() } else { revcomp(q) };
    let (mut qs, mut qe) = if fwd {
        (m.query_start as usize, m.query_end as usize)
    } else {
        (
            q.len() - m.query_end as usize,
            q.len() - m.query_start as usize,
        )
    };
    let (mut ts, mut te) = (m.target_start as usize, m.target_end as usize);
    let mut diffs = parse_cs(a.cs.as_deref().unwrap_or(""), ts);
    let mut nm = a.nm as i64;
    // one longer insertion or deletion is one event, not twenty edits: otherwise its reads are thrown out as junk
    if std::env::var("HLA_NOBIG").is_err() {
        for (_, w) in &diffs {
            let l = w.len() as i64 - 3;
            if (w.starts_with("del") || w.starts_with("ins")) && l >= 5 {
                nm -= l - 2;
            }
        }
    }
    let mut clips = vec![];
    let ext = std::env::var("HLA_NOEXT").is_err();
    // a clipped end is extended without gaps when it then fits with at most two mismatches:
    // a variant close to the end of a read or of the allele is otherwise invisible
    if qs > 0 {
        let room = qs.min(ts);
        let mm: Vec<usize> = (1..=room)
            .filter(|&d| qo[qs - d] != t[ts - d])
            .map(|d| ts - d)
            .collect();
        if ext && room > 0 && mm.len() <= 2 && (room == qs || ts == room) {
            for p in &mm {
                diffs.push((
                    *p,
                    format!("{}>{}", t[*p] as char, qo[qs - (ts - *p)] as char),
                ));
            }
            nm += mm.len() as i64;
            ts -= room;
            qs -= room;
        } else if qs >= 12 {
            clips.push(ts);
        }
    }
    if qe < qo.len() {
        let rest = qo.len() - qe;
        let room = rest.min(t.len().saturating_sub(te));
        let mm: Vec<usize> = (0..room)
            .filter(|&d| qo[qe + d] != t[te + d])
            .map(|d| te + d)
            .collect();
        if ext && room > 0 && mm.len() <= 2 && (room == rest || te + room == t.len()) {
            for p in &mm {
                diffs.push((
                    *p,
                    format!("{}>{}", t[*p] as char, qo[qe + (*p - te)] as char),
                ));
            }
            nm += mm.len() as i64;
            te += room;
            qe += room;
        } else if rest >= 12 {
            clips.push(te.min(t.len() - 1));
        }
    }
    // what still hangs over counts as edits so that junk does not look like a clean read;
    // bases beyond the end of the allele are no evidence against it
    let over_l = if ts == 0 { 0 } else { qs as i64 };
    let over_r = if te >= t.len() {
        0
    } else {
        (qo.len() - qe) as i64
    };
    let clip = if ext {
        over_l + over_r
    } else {
        qs as i64 + (qo.len() - qe) as i64
    };
    Some(Aln {
        nm: nm + clip,
        raw: nm,
        st: ts,
        en: te,
        diffs,
        clips,
    })
}

/// bases a read must cover on both sides of a repeat to count as spanning it
const REP_FLANK: usize = 10;

fn hp_run(s: &[u8], p: usize) -> usize {
    let lo = p.saturating_sub(8);
    let hi = (p + 8).min(s.len());
    let (mut best, mut run) = (1, 1);
    for k in lo + 1..hi {
        if s[k] == s[k - 1] {
            run += 1;
            best = best.max(run);
        } else {
            run = 1;
        }
    }
    best
}

/// Single-base runs of 8 or more and short tandem repeats (unit 2 to 6, four copies and 12 bases or
/// more) as half-open intervals. Their length is where short reads and assemblies disagree most.
pub fn repeats(s: &[u8]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = vec![];
    let n = s.len();
    for u in 1..=6usize {
        let mut i = 0;
        while i + 2 * u <= n {
            let mut j = i + u;
            while j < n && s[j] == s[j - u] {
                j += 1;
            }
            let len = j - i;
            let ok = if u == 1 {
                len >= 8
            } else {
                len >= 4 * u && len >= 12
            };
            // a unit that is itself a run of one base is the single-base case
            if ok && !(u > 1 && s[i..i + u].iter().all(|&c| c == s[i])) {
                if !out.iter().any(|&(a, b)| i < b && a < j) {
                    out.push((i, j));
                }
                i = j;
            } else {
                i += 1;
            }
        }
    }
    out.sort();
    out
}

/// sites per called allele (index 0 = first name, 1 = second; identical names share one entry)
pub fn sites(ctx: &Ctx, g: &str, qnames: &[usize], names: [&str; 2]) -> [Vec<Site>; 2] {
    let al = ctx.al(g);
    let find = |n: &str| al.iter().find(|a| a.name == n);
    let hom = names[0] == names[1] || names[1] == "-";
    let Some(a0) = find(names[0]) else {
        return [vec![], vec![]];
    };
    let a1 = if hom {
        a0
    } else {
        match find(names[1]) {
            Some(x) => x,
            None => return [vec![], vec![]],
        }
    };
    let seqs = [&a0.genomic, &a1.genomic];
    let na = if hom { 1 } else { 2 };
    let aln: Vec<_> = (0..na).map(|k| aligner(seqs[k], Preset::Sr)).collect();
    // per allele: depth and alternative counts, for reads specific to it and for shared reads
    let mut dspec: Vec<Vec<u32>> = (0..na).map(|k| vec![0; seqs[k].len() + 1]).collect();
    let mut dshared = dspec.clone();
    let mut alt: Vec<AHashMap<(usize, String), (u32, u32)>> =
        (0..na).map(|_| AHashMap::new()).collect();
    // pairs that fit the other allele slightly better: depth and differences on the losing allele
    let mut dlose = dspec.clone();
    // reads that prefer this allele by two or more edits: they tie a position to this haplotype
    let mut dlink = dspec.clone();
    let mut clipn: Vec<AHashMap<usize, u32>> = (0..na).map(|_| AHashMap::new()).collect();
    let mut altl: Vec<AHashMap<(usize, String), u32>> = (0..na).map(|_| AHashMap::new()).collect();
    // repeat lengths: per repeat the length change seen in reads that span it with both flanks
    let reps: Vec<Vec<(usize, usize)>> = (0..na)
        .map(|k| {
            if std::env::var("HLA_NOREP").is_ok() {
                vec![]
            } else {
                repeats(seqs[k])
            }
        })
        .collect();
    let mut replen: Vec<Vec<AHashMap<i64, (u32, u32)>>> = (0..na)
        .map(|k| vec![AHashMap::new(); reps[k].len()])
        .collect();
    for &q in qnames {
        let Some(r) = ctx.reads.get(q) else { continue };
        let mut per: Vec<Option<(i64, Vec<Aln>)>> = vec![];
        for k in 0..na {
            let mut tot = 0i64;
            let mut v = vec![];
            let mut ok = true;
            for m in r.iter().flatten() {
                let a = best(&aln[k], m, seqs[k]);
                if let Some(a) = &a {
                    if a.raw <= 2 {
                        for c in &a.clips {
                            *clipn[k].entry(*c).or_insert(0) += 1;
                        }
                    }
                }
                match a {
                    Some(a) => {
                        tot += a.nm;
                        v.push(a);
                    }
                    None => {
                        ok = false;
                    }
                }
            }
            // a pair with a mate that does not align to this allele is no evidence about it
            per.push(if ok && !v.is_empty() {
                Some((tot, v))
            } else {
                None
            });
        }
        let bestnm = per.iter().flatten().map(|x| x.0).min();
        let Some(bn) = bestnm else { continue };
        // a pair may carry several new variants; junk is kept out by the repeat mask and the clip penalty
        if bn > 8 {
            continue;
        }
        let winners: Vec<usize> = (0..na)
            .filter(|&k| per[k].as_ref().map_or(false, |x| x.0 == bn))
            .collect();
        let shared = winners.len() > 1 || hom;
        for k in 0..na {
            if winners.contains(&k) {
                continue;
            }
            if let Some((t, v)) = per[k].as_ref() {
                if *t <= bn + 3 {
                    for a in v {
                        for p in a.st..a.en.min(seqs[k].len()) {
                            dlose[k][p] += 1;
                        }
                        for (p, w) in &a.diffs {
                            if *p < a.st + 2 || *p + 2 > a.en {
                                continue;
                            }
                            *altl[k].entry((*p, w.clone())).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
        if !shared && na == 2 {
            let k = winners[0];
            let o = 1 - k;
            let margin = per[o].as_ref().map_or(99, |x| x.0 - bn);
            if margin >= 2 {
                for a in &per[k].as_ref().unwrap().1 {
                    for p in a.st..a.en.min(seqs[k].len()) {
                        dlink[k][p] += 1;
                    }
                }
            }
        }
        for &k in &winners {
            for a in &per[k].as_ref().unwrap().1 {
                let d = if shared {
                    &mut dshared[k]
                } else {
                    &mut dspec[k]
                };
                for p in a.st..a.en.min(seqs[k].len()) {
                    d[p] += 1;
                }
                let i0 = reps[k].partition_point(|&(_, e)| e + REP_FLANK <= a.st);
                for (ri, &(rs, re)) in reps[k].iter().enumerate().skip(i0) {
                    if rs >= a.en {
                        break;
                    }
                    if a.st + REP_FLANK > rs || re + REP_FLANK > a.en {
                        continue;
                    }
                    let mut d = 0i64;
                    for (p, w) in &a.diffs {
                        if *p + 1 < rs || *p > re {
                            continue;
                        }
                        if w.starts_with("ins") {
                            d += w.len() as i64 - 3;
                        } else if w.starts_with("del") {
                            d -= w.len() as i64 - 3;
                        }
                    }
                    let e = replen[k][ri].entry(d).or_insert((0, 0));
                    if shared { e.1 += 1 } else { e.0 += 1 }
                }
                for (p, w) in &a.diffs {
                    // ignore the last bases of an alignment, where a clipped indel shows as substitutions
                    if *p < a.st + 2 || *p + 2 > a.en {
                        continue;
                    }
                    let e = alt[k].entry((*p, w.clone())).or_insert((0, 0));
                    if shared { e.1 += 1 } else { e.0 += 1 }
                }
            }
        }
    }
    let mut out: [Vec<Site>; 2] = [vec![], vec![]];
    for k in 0..na {
        let a = if k == 0 { a0 } else { a1 };
        // exon intervals on the genomic sequence
        let mut ex: Vec<(usize, usize)> = vec![];
        for (_, e) in &a.exons {
            if e.len() >= 20 {
                if let Some(p) = a.genomic.windows(e.len()).position(|w| w == e.as_slice()) {
                    ex.push((p, p + e.len()));
                }
            }
        }
        // an extended allele carries donor flanks: only its own sequence can show a new variant
        let (own_s, own_e) = match (&a.orig, a.extended) {
            (Some(o), true) if o.len() >= 50 => {
                match a.genomic.windows(o.len()).position(|w| w == o.as_slice()) {
                    Some(p) => (p, p + o.len()),
                    None => (0, 0),
                }
            }
            _ => (0, a.genomic.len()),
        };
        let mut tot: Vec<u32> = dspec[k]
            .iter()
            .zip(dshared[k].iter())
            .map(|(x, y)| x + y)
            .collect();
        tot.sort();
        let med = tot[tot.len() / 2].max(1);
        let mut v: Vec<Site> = vec![];
        let dbg: Option<(String, usize)> = std::env::var("HLA_NOVDBG").ok().and_then(|v| {
            let mut it = v.split(':');
            Some((it.next()?.to_string(), it.next()?.parse().ok()?))
        });
        if let Some((dg, dp)) = &dbg {
            if dg == g {
                for q in dp.saturating_sub(3)..(*dp + 3).min(seqs[k].len()) {
                    let alts: Vec<String> = alt[k]
                        .iter()
                        .filter(|((p, _), _)| *p == q)
                        .map(|((_, w), (a, b))| format!("{w}:{a}+{b}"))
                        .collect();
                    let la: Vec<String> = altl[k]
                        .iter()
                        .filter(|((p, _), _)| *p == q)
                        .map(|((_, w), a)| format!("{w}:{a}"))
                        .collect();
                    eprintln!(
                        "NOVDBG {g} slot{k} {} pos {q} spec {} shared {} lose {} med {med} own {own_s}-{own_e} len {} {:?} lose {:?}",
                        a.name,
                        dspec[k][q],
                        dshared[k][q],
                        dlose[k][q],
                        seqs[k].len(),
                        alts,
                        la
                    );
                }
            }
        }
        for ((p, w), (cs, csh)) in &alt[k] {
            let (ds, dsh) = (dspec[k][*p], dshared[k][*p]);
            let n = cs + csh;
            // reads carrying a longer insertion or deletion are mostly cut off instead of aligned across it,
            // so far fewer of them show it
            let big = (w.starts_with("del") || w.starts_with("ins")) && w.len() >= 8;
            let need = if big {
                0.4 * ds as f64 + 0.2 * dsh as f64
            } else {
                0.7 * ds as f64 + 0.35 * dsh as f64
            };
            if *p < own_s + 2 || *p + 2 > own_e {
                continue;
            }
            if n < 5 || (n as f64) < need || ds + dsh > 2 * med {
                continue;
            }
            // the first and last bases of a gene attract reads of related genes: ask for near-unanimous reads
            if (*p < own_s + 15 || *p + 15 > own_e) && (n as f64) < 0.9 * (ds + dsh) as f64 {
                continue;
            }
            // length changes of single-base runs: short reads stutter there, so ask for clear support
            // and leave long runs alone
            if w.starts_with("del") || w.starts_with("ins") {
                let hp = hp_run(seqs[k], *p);
                if hp > 12
                    || (hp >= 6 && (n < 6 || (n as f64) < 0.8 * ds as f64 + 0.4 * dsh as f64))
                {
                    continue;
                }
            }
            // thin coverage: enough to doubt the fourth field, not enough to doubt the protein
            let thin = ((ds + dsh) as f64) < 0.4 * med as f64;
            v.push(Site {
                pos: *p,
                what: w.clone(),
                alt: n as usize,
                depth: (ds + dsh) as usize,
                exon: !thin && ex.iter().any(|&(s, e)| *p >= s && *p < e),
                either: !hom && *cs == 0,
            });
        }
        // repeat length: the reads that span a repeat agree on another length than the allele has
        for (ri, &(rs, re)) in reps[k].iter().enumerate() {
            if rs < own_s + 10 || re + 10 > own_e {
                continue;
            }
            let (mut ns, mut nsh) = (0u32, 0u32);
            for (_, (a, b)) in &replen[k][ri] {
                ns += a;
                nsh += b;
            }
            let Some((d, (cs, csh))) = replen[k][ri]
                .iter()
                .filter(|(d, _)| **d != 0)
                .max_by_key(|(d, (a, b))| (a + b, -d.abs()))
                .map(|(d, c)| (*d, *c))
            else {
                continue;
            };
            let n = cs + csh;
            if n < 4 || (n as f64) < 0.6 * ns as f64 + 0.3 * nsh as f64 || ns + nsh > 2 * med {
                continue;
            }
            if v.iter().any(|s| s.pos + 1 >= rs && s.pos <= re) {
                continue;
            }
            v.push(Site {
                pos: rs,
                what: format!("rep{:+}", d),
                alt: n as usize,
                depth: (ns + nsh) as usize,
                exon: ex.iter().any(|&(s, e)| rs < e && s < re),
                either: !hom && cs == 0,
            });
        }
        if std::env::var("HLA_REPDUMP").is_ok() {
            for (ri, &(rs, re)) in reps[k].iter().enumerate() {
                let (a0, b0) = replen[k][ri].get(&0).copied().unwrap_or((0, 0));
                let oth: u32 = replen[k][ri]
                    .iter()
                    .filter(|(d, _)| **d != 0)
                    .map(|(_, (x, y))| x + y)
                    .sum();
                eprintln!(
                    "REPSPAN\t{g}\t{k}\t{}\t{rs}\t{re}\t{}\t{}\t{oth}\t{med}",
                    a.name, a0, b0
                );
            }
        }
        // long repeat whose length too few reads confirm: its length is not verified, so neither is the fourth field
        if let Ok(x) = std::env::var("HLA_REPV") {
            let (minlen, frac): (usize, f64) = {
                let mut it = x.split(':');
                (
                    it.next().and_then(|v| v.parse().ok()).unwrap_or(15),
                    it.next().and_then(|v| v.parse().ok()).unwrap_or(0.35),
                )
            };
            for (ri, &(rs, re)) in reps[k].iter().enumerate() {
                if re - rs < minlen || rs < own_s + 10 || re + 10 > own_e {
                    continue;
                }
                if v.iter().any(|s| s.pos + 1 >= rs && s.pos <= re) {
                    continue;
                }
                let (a0, b0) = replen[k][ri].get(&0).copied().unwrap_or((0, 0));
                let span = a0 + b0;
                if (span as f64) < frac * med as f64 || span < 3 {
                    v.push(Site {
                        pos: rs,
                        what: format!("repunverified{}bp", re - rs),
                        alt: span as usize,
                        depth: med as usize,
                        exon: false,
                        either: false,
                    });
                }
            }
        }
        if std::env::var("HLA_NOV20").is_err() {
            // phase conflict: reads that fit both alleles equally carry the other base where this allele's own reads do not
            for ((p, w), (cs, csh)) in alt[k].iter().filter(|_| std::env::var("HLA_PHASE").is_ok())
            {
                let (ds, dsh) = (dspec[k][*p], dshared[k][*p]);
                if *p < own_s + 10 || *p + 10 > own_e || v.iter().any(|s| s.pos == *p) {
                    continue;
                }
                if *cs == 0
                    && ds >= 5
                    && *csh >= 4
                    && (*csh as f64) >= 0.35 * dsh as f64
                    && !w.starts_with("del")
                    && !w.starts_with("ins")
                    && ds + dsh <= 2 * med
                {
                    v.push(Site {
                        pos: *p,
                        what: format!("{w}(phase)"),
                        alt: *csh as usize,
                        depth: (ds + dsh) as usize,
                        exon: ex.iter().any(|&(s, e)| *p >= s && *p < e),
                        either: true,
                    });
                }
            }
            // unsupported base: no read backs this allele here, the reads that could all prefer the other allele
            for ((p, w), n) in &altl[k] {
                let own = dspec[k][*p] + dshared[k][*p];
                if *p < own_s + 10 || *p + 10 > own_e || v.iter().any(|s| s.pos == *p) {
                    continue;
                }
                if w.starts_with("del") || w.starts_with("ins") {
                    continue;
                }
                if own <= (med / 8).max(2)
                    && *n >= 7
                    && (*n as f64) >= 0.7 * dlose[k][*p] as f64
                    && dlose[k][*p] <= 2 * med
                {
                    v.push(Site {
                        pos: *p,
                        what: format!("{w}(unsupported)"),
                        alt: *n as usize,
                        depth: (own + dlose[k][*p]) as usize,
                        exon: ex.iter().any(|&(s, e)| *p >= s && *p < e),
                        either: false,
                    });
                }
            }
        }
        if std::env::var("HLA_BREAK").is_ok() {
            // many reads cut off at the same place: a larger insertion or deletion
            let mut cl: Vec<(usize, u32)> = clipn[k].iter().map(|(p, n)| (*p, *n)).collect();
            cl.sort();
            let mut last = usize::MAX;
            for (p, _) in &cl {
                let n: u32 = cl
                    .iter()
                    .filter(|(q, _)| *q + 3 > *p && *q < *p + 3)
                    .map(|(_, n)| *n)
                    .sum();
                let d =
                    dspec[k][(*p).min(seqs[k].len() - 1)] + dshared[k][(*p).min(seqs[k].len() - 1)];
                if *p < own_s + 20 || *p + 20 > own_e || (last != usize::MAX && *p < last + 30) {
                    continue;
                }
                if n >= 6 && (n as f64) >= 0.35 * (d + n) as f64 && d <= 2 * med {
                    v.push(Site {
                        pos: *p,
                        what: "break".to_string(),
                        alt: n as usize,
                        depth: (d + n) as usize,
                        exon: ex.iter().any(|&(s, e)| *p >= s && *p < e),
                        either: na == 2,
                    });
                    last = *p;
                }
            }
        }
        if na == 2 && std::env::var("HLA_UNPH").is_ok() {
            // a sibling with the same first three fields that differs only where no read ties the base to
            // this haplotype: the base may as well be a new variant of the other allele
            let f3 = |n: &str| n.split(':').take(3).collect::<Vec<_>>().join(":");
            let me3 = f3(&a.name);
            'sib: for b in al.iter() {
                if b.name == a.name
                    || b.name == names[1 - k]
                    || b.genomic.len() != a.genomic.len()
                    || f3(&b.name) != me3
                {
                    continue;
                }
                let mut diff = vec![];
                for i in 0..a.genomic.len() {
                    if a.genomic[i] != b.genomic[i] {
                        diff.push(i);
                        if diff.len() > 3 {
                            continue 'sib;
                        }
                    }
                }
                if diff.is_empty() {
                    continue;
                }
                if diff.iter().all(|&i| {
                    dlink[k][i] < 2 && dspec[k][i] >= 3 && dshared[k][i] <= dspec[k][i] / 3
                }) {
                    v.push(Site {
                        pos: diff[0],
                        what: format!("unphased:{}", b.name),
                        alt: dlink[k][diff[0]] as usize,
                        depth: dspec[k][diff[0]] as usize,
                        exon: false,
                        either: true,
                    });
                    break;
                }
            }
        }
        if std::env::var("HLA_NOBLIND").is_err() && own_e > own_s + 20 {
            // stretches without any read: a new variant there cannot be seen, so the fourth field is not verified
            let (mut run, mut best, mut bstart, mut total) = (0usize, 0usize, 0usize, 0usize);
            for p in own_s + 10..own_e - 10 {
                if dspec[k][p] + dshared[k][p] + dlose[k][p] == 0 {
                    run += 1;
                    total += 1;
                    if run > best {
                        best = run;
                        bstart = p + 1 - run;
                    }
                } else {
                    run = 0;
                }
            }
            if best >= 30 && total >= 300 {
                v.push(Site {
                    pos: bstart,
                    what: format!("blind{total}bp"),
                    alt: 0,
                    depth: 0,
                    exon: false,
                    either: false,
                });
            }
        }
        v.sort_by_key(|s| s.pos);
        out[k] = v;
    }
    if hom {
        // one allele called twice: a site carried by about half the reads belongs to one copy only
        out[1] = out[0]
            .iter()
            .map(|s| Site {
                pos: s.pos,
                what: s.what.clone(),
                alt: s.alt,
                depth: s.depth,
                exon: s.exon,
                either: false,
            })
            .collect();
        out[0].retain(|s| {
            s.alt as f64
                >= if (s.what.starts_with("del") || s.what.starts_with("ins")) && s.what.len() >= 8
                {
                    0.5
                } else {
                    0.75
                } * s.depth as f64
        });
    }
    out
}

/// The database allele that equals the called allele with its residual variants applied, if there
/// is one: a rare known allele outside the candidate groups (for example a null allele that differs
/// from the common one by a single intron base). Only plain substitutions are considered, all of
/// them clearly assigned to this allele, and the match must be exact over the other allele's own
/// sequence.
pub fn db_match(ctx: &Ctx, g: &str, name: &str, sites: &[Site]) -> Option<String> {
    let al = ctx.al(g);
    let a = al.iter().find(|a| a.name == name)?;
    let mut sn: Vec<(usize, u8, u8)> = vec![];
    for s in sites {
        if s.what.starts_with("blind") {
            continue;
        }
        let w = s.what.as_bytes();
        let plain =
            w.len() >= 3 && w[1] == b'>' && (w.len() == 3 || &s.what[3..] == "(unsupported)");
        if !plain || s.either || s.alt < 5 {
            return None;
        }
        sn.push((s.pos, w[0], w[2]));
    }
    if sn.is_empty() || sn.len() > 4 {
        return None;
    }
    let mut q = a.genomic.clone();
    for &(p, r, v) in &sn {
        if p >= q.len() || q[p] != r {
            return None;
        }
        q[p] = v;
    }
    let mut hits: Vec<&str> = vec![];
    for b in al.iter() {
        if b.imputed
            || b.name == a.name
            || b.genomic.len() != q.len()
            || sn.iter().any(|&(p, _, v)| b.genomic[p] != v)
        {
            continue;
        }
        let (s0, e0) = match (&b.orig, b.extended) {
            (Some(o), true) if o.len() >= 50 => {
                match b.genomic.windows(o.len()).position(|w| w == o.as_slice()) {
                    Some(p) => (p, p + o.len()),
                    None => continue,
                }
            }
            _ => (0, b.genomic.len()),
        };
        if sn.iter().all(|&(p, _, _)| p >= s0 && p < e0) && q[s0..e0] == b.genomic[s0..e0] {
            hits.push(b.name.as_str());
        }
    }
    hits.sort();
    hits.first().map(|x| x.to_string())
}

#[cfg(test)]
mod tests {
    use super::repeats;
    #[test]
    fn finds_runs_and_tandem_repeats() {
        let s = b"ACGTTTTTTTTTTGCACACACACACAGGATCCATG";
        let r = repeats(s);
        assert_eq!(r, vec![(3, 13), (14, 26)]);
        assert!(repeats(b"ACGTTTTTTTGCA").is_empty());
    }
}
