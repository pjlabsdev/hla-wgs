//! Pass 3 (exon-transplant refinement) and the phase check of the prototype
use crate::align::{aligner, map, Preset};
use crate::db::Allele;
use crate::kern;
use crate::names::{diff_level, is_null_like, num_key, two};
use crate::stageb::{Ctx, StageB, BG_EDITS, LAM, TIE};
use crate::stagea::np_sum;
use ahash::{AHashMap, AHashSet};

const MAXCLS: usize = 400;
const NEIGH: usize = 3;
const PH_T: f64 = 3.0;

type Exons = Vec<(u32, Vec<u8>)>;

/// per slot: margins at fields 2/3 and their alternatives
pub type RMarg = AHashMap<usize, ([f64; 2], [String; 2])>;

fn exon_map(al: &[Allele], idx: usize) -> Exons {
    let a = &al[idx];
    let mut ex: Exons = a.exons.clone();
    if let Some(par) = &a.imputed_from {
        if let Some(pa) = al.iter().find(|x| &x.name == par) {
            for (k, v) in &pa.exons {
                if !ex.iter().any(|e| e.0 == *k) {
                    ex.push((*k, v.clone()));
                }
            }
        }
    }
    ex.sort_by_key(|e| e.0);
    ex
}

fn find_from(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    if n.is_empty() {
        return Some(from.min(h.len()));
    }
    if from > h.len() || h.len() - from < n.len() {
        return None;
    }
    h[from..].windows(n.len()).position(|w| w == n).map(|p| p + from)
}

/// exon number -> (start, end) on the allele's genomic sequence
pub fn exon_pos(al: &[Allele], idx: usize) -> Vec<(u32, usize, usize)> {
    let gs = &al[idx].genomic;
    let mut out = vec![];
    let mut pos = 0;
    for (k, e) in exon_map(al, idx) {
        let j = find_from(gs, &e, pos).or_else(|| find_from(gs, &e, 0));
        if let Some(j) = j {
            out.push((k, j, j + e.len()));
            pos = j + e.len();
        }
    }
    out
}

pub fn transplant(al: &[Allele], backbone: usize, exons: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut gs = al[backbone].genomic.clone();
    let pos = exon_pos(al, backbone);
    let mut ks: Vec<&(u32, Vec<u8>)> = exons.iter().collect();
    ks.sort_by(|a, b| b.0.cmp(&a.0));
    for (k, e) in ks {
        if let Some(&(_, a, b)) = pos.iter().find(|p| p.0 == *k) {
            let mut n = gs[..a].to_vec();
            n.extend_from_slice(e);
            n.extend_from_slice(&gs[b..]);
            gs = n;
        }
    }
    gs
}

fn rep_of(al: &[Allele], mem: &[usize]) -> usize {
    let full: Vec<usize> = mem.iter().copied().filter(|&x| !al[x].imputed && !al[x].partial).collect();
    let full = if full.is_empty() { mem.to_vec() } else { full };
    let mut best = full[0];
    for &x in &full[1..] {
        if (is_null_like(&al[x].name), num_key(&al[x].name)) < (is_null_like(&al[best].name), num_key(&al[best].name)) {
            best = x;
        }
    }
    best
}

fn min_row(r: &[f64]) -> f64 {
    r.iter().cloned().fold(f64::INFINITY, f64::min)
}

struct Jn {
    orig: Option<(Vec<u8>, f64, String)>,
    new: Option<(Vec<u8>, f64, String)>,
}

pub fn exon_refine(ctx: &Ctx, g: &str, qnames: &[usize], r: &StageB, rmarg: &mut RMarg, log: &dyn Fn(String)) -> Option<Vec<String>> {
    let al = ctx.al(g);
    let cand = &r.cand;
    let win = [cand[r.pair.0], cand[r.pair.1]];
    let mut near2: Vec<String> = vec![];
    for p in r.near.iter().take(40) {
        for a in [p.0, p.1] {
            let t = two(&al[cand[a]].name);
            if !near2.contains(&t) {
                near2.push(t);
            }
        }
    }
    near2.truncate(8);
    for a in &r.alts {
        if !near2.contains(a) {
            near2.push(a.clone());
        }
    }
    // the common groups of the population data join the candidates (not with HLA_NOFREQ)
    let fr: &[(String, f64)] = if std::env::var("HLA_NOFREQ").map_or(false, |v| v != "0") { &[] } else { ctx.db.freq.get(g).map(|v| v.as_slice()).unwrap_or(&[]) };
    for &w in &win {
        let fam = al[w].name.split('*').nth(1).unwrap().split(':').next().unwrap().to_string();
        let mut common: Vec<&(String, f64)> = fr.iter().filter(|(k, _)| k.split(':').next().unwrap() == fam).collect();
        common.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let add: Vec<String> = common.iter().take(3).map(|(k, _)| format!("{g}*{k}")).filter(|x| !near2.contains(x)).collect();
        near2.extend(add);
    }
    let near2set: AHashSet<&String> = near2.iter().collect();
    // 2-field group -> exon class -> members (insertion ordered)
    let mut bygrp: AHashMap<String, Vec<(Exons, Vec<usize>)>> = AHashMap::new();
    for (x, a) in al.iter().enumerate() {
        let g2 = two(&a.name);
        if near2set.contains(&g2) {
            let v = bygrp.entry(g2).or_default();
            match v.iter_mut().find(|e| e.0 == a.exons) {
                Some(e) => e.1.push(x),
                None => v.push((a.exons.clone(), vec![x])),
            }
        }
    }
    let exon_neighbours = |w: usize| -> Vec<(Exons, Vec<usize>)> {
        let we = &al[w].exons;
        let mut out: Vec<(Exons, Vec<usize>)> = vec![];
        let mut pos: AHashMap<&Exons, usize> = AHashMap::new();
        for (x, a) in al.iter().enumerate() {
            let mut d = 0usize;
            let mut ok = true;
            for (k, v) in &a.exons {
                match we.iter().find(|e| e.0 == *k) {
                    Some(u) if u.1.len() == v.len() => {
                        d += u.1.iter().zip(v.iter()).filter(|(p, q)| p != q).count();
                        if d > NEIGH {
                            ok = false;
                            break;
                        }
                    }
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                match pos.get(&a.exons) {
                    Some(&p) => out[p].1.push(x),
                    None => {
                        pos.insert(&a.exons, out.len());
                        out.push((a.exons.clone(), vec![x]));
                    }
                }
            }
        }
        out
    };
    let slot_classes = |w: usize| -> Vec<(Exons, Vec<usize>)> {
        let own = two(&al[w].name);
        let fam = own.split(':').next().unwrap().to_string();
        let mut order = vec![own.clone()];
        order.extend(near2.iter().filter(|x| **x != own && x.split(':').next().unwrap() == fam).cloned());
        order.extend(near2.iter().filter(|x| x.split(':').next().unwrap() != fam).cloned());
        let mut out: Vec<(Exons, Vec<usize>)> = vec![];
        let mut pos: AHashMap<Exons, usize> = AHashMap::new();
        for (k, m) in exon_neighbours(w) {
            if out.len() < MAXCLS {
                pos.insert(k.clone(), out.len());
                out.push((k, m));
            }
        }
        let empty = vec![];
        for gname in &order {
            let grp = bygrp.get(gname).unwrap_or(&empty);
            if !out.is_empty() && out.len() + grp.len() > MAXCLS + 200 {
                continue;
            }
            for (k, m) in grp {
                let p = match pos.get(k) {
                    Some(&p) => p,
                    None => {
                        pos.insert(k.clone(), out.len());
                        out.push((k.clone(), vec![]));
                        out.len() - 1
                    }
                };
                for &x in m {
                    if !out[p].1.contains(&x) {
                        out[p].1.push(x);
                    }
                }
            }
        }
        out
    };
    let nclasses = slot_classes(win[0]).len();
    // only reads that touch an exon of either winner decide the exon class
    let mut ex_q: AHashSet<usize> = AHashSet::new();
    let mut wset = vec![win[0]];
    if win[1] != win[0] {
        wset.push(win[1]);
    }
    for &w in &wset {
        let a = aligner(&al[w].genomic, Preset::Sr);
        let iv = exon_pos(al, w);
        for &q in qnames {
            let Some(rd) = ctx.reads.get(q) else { continue };
            if ex_q.contains(&q) {
                continue;
            }
            for sq in rd.iter().flatten() {
                if map(&a, sq).iter().any(|h| iv.iter().any(|&(_, s, e)| h.r_st < e as i64 && h.r_en > s as i64)) {
                    ex_q.insert(q);
                    break;
                }
            }
        }
    }
    let qall: Vec<usize> = qnames.to_vec();
    let mut qn: Vec<usize> = qnames.iter().copied().filter(|q| ex_q.contains(q)).collect();
    log(format!("{} exon-refine reads {} classes {}", g, qn.len(), nclasses));
    if qn.is_empty() {
        return None;
    }
    if qn.len() > 4000 {
        qn.sort_by(|a, b| ctx.reads.names[*a].cmp(&ctx.reads.names[*b]));
        let step = (qn.len() / 4000).max(1);
        qn = qn.into_iter().step_by(step).collect();
    }
    let mut names = vec![al[win[0]].name.clone(), al[win[1]].name.clone()];
    let slots: Vec<usize> = if win[0] == win[1] { vec![0] } else { vec![0, 1] };
    let mut jn: Vec<Jn> = vec![Jn { orig: None, new: None }, Jn { orig: None, new: None }];
    let bg = (-LAM * BG_EDITS).exp();
    let keep = |e: &[Vec<f64>], used: &[usize]| -> Vec<bool> { ctx.keep_rows(g, &qall_for_decoy(qnames), e, used) };
    for &sl in &slots {
        let other: Option<&[u8]> = if slots.len() == 2 { Some(&al[win[1 - sl]].genomic) } else { None };
        let mut seqs: Vec<Vec<u8>> = vec![];
        let mut seqpos: AHashMap<Vec<u8>, usize> = AHashMap::new();
        let mut owners: Vec<Vec<usize>> = vec![];
        for (key, members) in slot_classes(win[sl]) {
            let sq = transplant(al, win[sl], &key);
            let p = match seqpos.get(&sq) {
                Some(&p) => p,
                None => {
                    seqpos.insert(sq.clone(), seqs.len());
                    seqs.push(sq);
                    owners.push(vec![]);
                    seqs.len() - 1
                }
            };
            owners[p].extend(members);
        }
        let mut all: Vec<&[u8]> = vec![];
        if let Some(o) = other {
            all.push(o);
        }
        all.extend(seqs.iter().map(|s| s.as_slice()));
        let (e, used) = kern::edit_matrix(ctx.reads, &qn, &all)?;
        let kp = keep(&e, &used);
        let e: Vec<&Vec<f64>> = e.iter().zip(kp.iter()).filter(|x| *x.1).map(|x| x.0).collect();
        let p: Vec<Vec<f64>> = e.iter().map(|r| r.iter().map(|v| (-LAM * v.min(12.0)).exp()).collect()).collect();
        let off = if other.is_some() { 1 } else { 0 };
        let mut best: Option<(f64, usize)> = None;
        let mut allk: Vec<(f64, usize)> = vec![];
        for k in 0..seqs.len() {
            let mem = &owners[k];
            let mut rep = rep_of(al, mem);
            let pr = ctx.pri.name_prior(g, &al[rep].name);
            let ll = if other.is_some() {
                np_sum(p.iter().map(|r| (0.5 * r[0] + 0.5 * r[k + off] + bg).ln())) + pr
            } else {
                np_sum(p.iter().map(|r| (r[k + off] + bg).ln())) + pr
            };
            let has_w = mem.contains(&win[sl]);
            if has_w {
                rep = win[sl];
            }
            allk.push((ll, rep));
            if has_w {
                jn[sl].orig = Some((seqs[k].clone(), pr, al[rep].name.clone()));
            }
            if best.map_or(true, |b| ll > b.0 + TIE || (has_w && ll > b.0 - TIE)) {
                best = Some((ll, rep));
                jn[sl].new = Some((seqs[k].clone(), pr, al[rep].name.clone()));
            }
        }
        let best = best.unwrap();
        names[sl] = al[best.1].name.clone();
        let mut m = [999.0f64; 2];
        let mut alt: [String; 2] = Default::default();
        for (ll, rep) in &allk {
            let lv = diff_level(&al[*rep].name, &names[sl]);
            for (ni, n) in [2u32, 3].iter().enumerate() {
                if lv <= *n && best.0 - ll < m[ni] {
                    m[ni] = best.0 - ll;
                    alt[ni] = al[*rep].name.clone();
                }
            }
        }
        rmarg.insert(sl, (m, alt));
        if slots.len() == 1 {
            names[1] = names[0].clone();
        }
        log(format!("{} exon-refine slot {} -> {} classes {}", g, sl, names[sl], seqs.len()));
    }
    // both slots changed: score the four combinations jointly
    if slots.len() == 2 && slots.iter().all(|&sl| jn[sl].orig.is_some() && jn[sl].new.as_ref().unwrap().0 != jn[sl].orig.as_ref().unwrap().0) {
        let (o0, n0, o1, n1) = (jn[0].orig.as_ref().unwrap(), jn[0].new.as_ref().unwrap(), jn[1].orig.as_ref().unwrap(), jn[1].new.as_ref().unwrap());
        let sq: Vec<&[u8]> = vec![&o0.0, &n0.0, &o1.0, &n1.0];
        if let Some((e, used)) = kern::edit_matrix(ctx.reads, &qn, &sq) {
            let kp = keep(&e, &used);
            let p: Vec<Vec<f64>> = e.iter().zip(kp.iter()).filter(|x| *x.1).map(|x| x.0.iter().map(|v| (-LAM * v.min(12.0)).exp()).collect()).collect();
            let mut comb: Vec<(f64, &str, &str)> = vec![];
            for a in ["orig", "new"] {
                for b in ["orig", "new"] {
                    let ca = if a == "orig" { 0 } else { 1 };
                    let cb = if b == "orig" { 2 } else { 3 };
                    let (pa, pb) = (if a == "orig" { o0.1 } else { n0.1 }, if b == "orig" { o1.1 } else { n1.1 });
                    let ll = np_sum(p.iter().map(|r| (0.5 * r[ca] + 0.5 * r[cb] + bg).ln())) + pa + pb;
                    comb.push((ll, a, b));
                }
            }
            // Python: comb.sort(reverse=True) on (ll, a, b)
            comb.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap().then(y.1.cmp(x.1)).then(y.2.cmp(x.2)));
            let nm = |sl: usize, c: &str| -> String {
                let j = if sl == 0 { (o0, n0) } else { (o1, n1) };
                if c == "orig" { j.0 .2.clone() } else { j.1 .2.clone() }
            };
            log(format!(
                "{} exon-refine joint [{}]",
                g,
                comb.iter().map(|(ll, a, b)| format!("({:.1}, '{}', '{}')", ll - comb[0].0, nm(0, a), nm(1, b))).collect::<Vec<_>>().join(", ")
            ));
            names = vec![nm(0, comb[0].1), nm(1, comb[0].2)];
            let jm = comb[0].0 - comb[1].0;
            for (sl, c) in [(0usize, comb[1].1), (1, comb[1].2)] {
                let alt_name = nm(sl, c);
                if alt_name != names[sl] {
                    let lv = diff_level(&alt_name, &names[sl]);
                    let e = rmarg.get_mut(&sl).unwrap();
                    for (ni, n) in [2u32, 3].iter().enumerate() {
                        if lv <= *n && jm < e.0[ni] {
                            e.0[ni] = jm;
                            e.1[ni] = alt_name.clone();
                        }
                    }
                }
            }
        }
    }
    let mut qall = qall;
    if qall.len() > 6000 {
        qall.sort_by(|a, b| ctx.reads.names[*a].cmp(&ctx.reads.names[*b]));
        let step = (qall.len() / 6000).max(1);
        qall = qall.into_iter().step_by(step).collect();
    }
    Some(phase_check(ctx, g, &qall, win, names, rmarg, log))
}

/// decoy table is cached per gene from its first (stage B) call; the qnames passed here are irrelevant
fn qall_for_decoy(q: &[usize]) -> Vec<usize> {
    q.to_vec()
}

pub fn phase_check(ctx: &Ctx, g: &str, qnames: &[usize], win: [usize; 2], names: Vec<String>, rmarg: &mut RMarg, log: &dyn Fn(String)) -> Vec<String> {
    let al = ctx.al(g);
    if names[0] == names[1] {
        return names;
    }
    // all exon classes of the gene, insertion ordered
    let mut cls: Vec<(Exons, Vec<usize>)> = vec![];
    {
        let mut pos: AHashMap<&Exons, usize> = AHashMap::new();
        for (x, a) in al.iter().enumerate() {
            match pos.get(&a.exons) {
                Some(&p) => cls[p].1.push(x),
                None => {
                    pos.insert(&a.exons, cls.len());
                    cls.push((a.exons.clone(), vec![x]));
                }
            }
        }
    }
    let byname: AHashMap<&str, usize> = al.iter().enumerate().map(|(i, a)| (a.name.as_str(), i)).collect();
    let (fa, fb) = (byname[names[0].as_str()], byname[names[1].as_str()]);
    let (ea, eb) = (&al[fa].exons, &al[fb].exons);
    let get = |e: &Exons, k: u32| e.iter().find(|x| x.0 == k).map(|x| x.1.clone());
    let mut kk: Vec<u32> = ea.iter().map(|e| e.0).filter(|k| eb.iter().any(|x| x.0 == *k)).collect();
    kk.sort();
    if kk.is_empty() || kk.iter().any(|&k| get(ea, k).unwrap().len() != get(eb, k).unwrap().len()) {
        return names;
    }
    let mut d: Vec<(u32, usize)> = vec![];
    for &k in &kk {
        let (x, y) = (get(ea, k).unwrap(), get(eb, k).unwrap());
        for i in 0..x.len() {
            if x[i] != y[i] {
                d.push((k, i));
            }
        }
    }
    if !(2..=8).contains(&d.len()) {
        return names;
    }
    let dk: AHashSet<u32> = d.iter().map(|x| x.0).collect();
    let find = |c: &AHashMap<u32, Vec<u8>>| -> Option<usize> {
        let mut out: Vec<usize> = vec![];
        for (ci, (e, _)) in cls.iter().enumerate() {
            let has = |k: u32| e.iter().any(|x| x.0 == k);
            if dk.iter().all(|&k| has(k)) && kk.iter().all(|&k| !has(k) || get(e, k).unwrap() == c[&k]) {
                out.push(ci);
            }
        }
        // prefer the class defining the most exons (stable)
        out.sort_by(|a, b| cls[*b].0.len().cmp(&cls[*a].0.len()));
        out.first().copied()
    };
    let prior = |x: usize| ctx.pri.name_prior(g, &al[x].name);
    let mut alts: Vec<(usize, usize, Exons, Exons)> = vec![];
    for mask in 1u32..(1u32 << (d.len() - 1)) {
        let mut c: AHashMap<u32, Vec<u8>> = kk.iter().map(|&k| (k, get(ea, k).unwrap())).collect();
        let mut dd: AHashMap<u32, Vec<u8>> = kk.iter().map(|&k| (k, get(eb, k).unwrap())).collect();
        for bit in 0..(d.len() - 1) {
            if mask >> bit & 1 == 1 {
                let (k, i) = d[bit + 1];
                let (x, y) = (get(eb, k).unwrap()[i], get(ea, k).unwrap()[i]);
                c.get_mut(&k).unwrap()[i] = x;
                dd.get_mut(&k).unwrap()[i] = y;
            }
        }
        if let (Some(fc), Some(fd)) = (find(&c), find(&dd)) {
            let to = |m: &AHashMap<u32, Vec<u8>>| -> Exons { kk.iter().map(|&k| (k, m[&k].clone())).collect() };
            alts.push((fc, fd, to(&c), to(&dd)));
        }
    }
    if alts.is_empty() {
        return names;
    }
    let base_a: Exons = kk.iter().map(|&k| (k, get(ea, k).unwrap())).collect();
    let base_b: Exons = kk.iter().map(|&k| (k, get(eb, k).unwrap())).collect();
    let mut sq: Vec<Vec<u8>> = vec![transplant(al, win[0], &base_a), transplant(al, win[1], &base_b)];
    for (_, _, c, dd) in &alts {
        sq.push(transplant(al, win[0], c));
        sq.push(transplant(al, win[1], dd));
    }
    let sr: Vec<&[u8]> = sq.iter().map(|s| s.as_slice()).collect();
    let Some((e, _)) = kern::edit_matrix(ctx.reads, qnames, &sr) else { return names };
    let bg = (-LAM * BG_EDITS).exp();
    let p: Vec<Vec<f64>> = e.iter().filter(|r| min_row(r) <= 10.0).map(|r| r.iter().map(|v| (-LAM * v.min(12.0)).exp()).collect()).collect();
    let ll: Vec<f64> = (0..sq.len() / 2).map(|t| np_sum(p.iter().map(|r| (0.5 * r[2 * t] + 0.5 * r[2 * t + 1] + bg).ln()))).collect();
    let cur = (ll[0], prior(fa) + prior(fb), names[0].clone(), names[1].clone());
    let mut amb: Option<(f64, f64, String, String)> = None;
    for (t, (kc, kd, _, _)) in alts.iter().enumerate() {
        let (rc, rd) = (rep_of(al, &cls[*kc].1), rep_of(al, &cls[*kd].1));
        let cand = (ll[t + 1], prior(rc) + prior(rd), al[rc].name.clone(), al[rd].name.clone());
        log(format!("{} phase alternative {} {} reads {:.1} prior {:.1}", g, cand.2, cand.3, cand.0 - cur.0, cand.1 - cur.1));
        if cand.0 - cur.0 > -PH_T && amb.as_ref().map_or(true, |a| cand.0 + cand.1 > a.0 + a.1) {
            amb = Some(cand);
        }
    }
    let Some(amb) = amb else { return names };
    let (best, other) = if amb.0 - cur.0 > PH_T || amb.0 + amb.1 > cur.0 + cur.1 { (amb.clone(), cur.clone()) } else { (cur.clone(), amb.clone()) };
    let dm = if (amb.0 - cur.0).abs() > PH_T { (amb.0 - cur.0).abs() } else { 0.0 };
    for sl in 0..2 {
        let e = rmarg.entry(sl).or_insert(([999.0, 999.0], Default::default()));
        let (on, bn) = if sl == 0 { (&other.2, &best.2) } else { (&other.3, &best.3) };
        let lv = diff_level(on, bn);
        for (ni, n) in [2u32, 3].iter().enumerate() {
            if lv <= *n && dm < e.0[ni] {
                e.0[ni] = dm;
                e.1[ni] = on.clone();
            }
        }
    }
    log(format!("{} phase-ambiguous: {} {} vs {} {}", g, best.2, best.3, other.2, other.3));
    vec![best.2, best.3]
}
