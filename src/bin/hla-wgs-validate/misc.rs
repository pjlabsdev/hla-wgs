//! Helpers: MHC contigs of an assembly, thinning read pairs, the two-field frequency prior.
use crate::common::*;
use ahash::AHashSet;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{BufRead, BufWriter, Write};

/// contigs of an assembly (FASTA on stdin) that contain HLA 31-mers; written to OUT
pub fn mhc_contigs(out: &str) {
    const K: usize = 31;
    let w = env_or("HLA_WORK", &std::env::current_dir().unwrap().to_string_lossy());
    let mut anch: AHashSet<Vec<u8>> = AHashSet::new();
    let mut cnt: std::collections::HashMap<String, usize> = Default::default();
    let mut name: Option<String> = None;
    let mut seq = String::new();
    let mut nuc: Vec<(String, String)> = vec![];
    for l in read(&format!("{w}/hla_nuc.fasta")).lines() {
        if let Some(h) = l.strip_prefix('>') {
            if let Some(n) = name.take() {
                nuc.push((n, std::mem::take(&mut seq)));
            }
            name = Some(h.split_whitespace().nth(1).unwrap_or("").to_string());
            seq.clear();
        } else {
            seq.push_str(l.trim());
        }
    }
    if let Some(n) = name.take() {
        nuc.push((n, seq.clone()));
    }
    for (n, s) in &nuc {
        let g = n.split('*').next().unwrap();
        if !["A", "B", "C", "DRB1", "DQB1", "DPB1"].contains(&g) {
            continue;
        }
        let c = cnt.entry(g.to_string()).or_insert(0);
        *c += 1;
        if *c > 40 {
            continue;
        }
        let b = s.as_bytes();
        for j in 0..(b.len() + 1).saturating_sub(K) {
            anch.insert(b[j..j + K].to_vec());
        }
    }
    let rc: Vec<Vec<u8>> = anch.iter().map(|x| x.iter().rev().map(|c| match c { b'A' => b'T', b'C' => b'G', b'G' => b'C', b'T' => b'A', x => *x }).collect()).collect();
    anch.extend(rc);
    let mut o = BufWriter::new(std::fs::File::create(out).unwrap());
    let mut name: Option<String> = None;
    let mut buf: Vec<u8> = vec![];
    let flush = |name: &Option<String>, buf: &mut Vec<u8>, o: &mut BufWriter<std::fs::File>| {
        let Some(n) = name else { return };
        buf.make_ascii_uppercase();
        let mut h = 0;
        let mut j = 0;
        while j + K < buf.len() {
            if anch.contains(&buf[j..j + K]) {
                h += 1;
                if h >= 20 {
                    break;
                }
            }
            j += 25;
        }
        if h >= 20 {
            o.write_all(b">").unwrap();
            o.write_all(n.as_bytes()).unwrap();
            o.write_all(b"\n").unwrap();
            o.write_all(buf).unwrap();
            o.write_all(b"\n").unwrap();
            eprintln!("keep {n} {}", buf.len());
        }
    };
    for l in std::io::stdin().lock().lines() {
        let l = l.unwrap();
        if let Some(h) = l.strip_prefix('>') {
            flush(&name, &mut buf, &mut o);
            name = Some(h.split_whitespace().next().unwrap_or("").to_string());
            buf.clear();
        } else {
            buf.extend_from_slice(l.trim().as_bytes());
        }
    }
    flush(&name, &mut buf, &mut o);
}

/// keep a fraction of the read pairs, chosen by a CRC-32 of the read name (the same pairs as before)
pub fn thin_pairs(src: &str, frac: &str, dst: &str) {
    let frac: f64 = frac.parse().expect("FRAC");
    for m in ["R1", "R2"] {
        let fi = crate::util::open_reader(&format!("{src}.{m}.fq.gz"));
        let mut fo = GzEncoder::new(BufWriter::new(std::fs::File::create(format!("{dst}.{m}.fq.gz")).unwrap()), Compression::new(1));
        let mut lines = fi.split(b'\n');
        loop {
            let rec: Vec<Vec<u8>> = (0..4).filter_map(|_| lines.next().map(|x| x.unwrap())).collect();
            if rec.is_empty() {
                break;
            }
            let first = &rec[0];
            let mut name: &[u8] = first.split(|c| c.is_ascii_whitespace()).find(|x| !x.is_empty()).unwrap_or(b"");
            if name.ends_with(b"/1") || name.ends_with(b"/2") {
                name = &name[..name.len() - 2];
            }
            let mut c = flate2::Crc::new();
            c.update(name);
            if ((c.sum() % 10000) as f64) < frac * 10000.0 {
                for r in &rec {
                    fo.write_all(r).unwrap();
                    fo.write_all(b"\n").unwrap();
                }
            }
        }
        fo.finish().unwrap();
    }
}

/// two-field prior from the 1000 Genomes laboratory types (ambiguities split evenly), samples in the
/// EXCLUDE files left out; reads 20140702_hla_diversity.txt in the current folder
pub fn build_freq(out: &str, exclude: &[&str]) {
    let mut ex = std::collections::HashSet::new();
    for f in exclude {
        for l in read(f).lines() {
            if !l.trim().is_empty() {
                ex.insert(l.split_whitespace().next().unwrap().trim_matches('"').to_string());
            }
        }
    }
    // read as Python does in text mode: universal newlines
    let t = read("20140702_hla_diversity.txt").replace("\r\n", "\n").replace('\r', "\n");
    let rows: Vec<Vec<&str>> = t.split('\n').filter(|l| !l.is_empty()).map(|l| l.split(' ').collect()).collect();
    let hdr: Vec<&str> = rows[0].iter().map(|h| h.trim_matches('"')).collect();
    let genes = ["A", "B", "C", "DRB1", "DQB1"];
    let mut cnt: Vec<OMap<String, f64>> = genes.iter().map(|_| OMap::default()).collect();
    for r in &rows[1..] {
        let r: Vec<&str> = r.iter().map(|x| x.trim_matches('"')).collect();
        if ex.contains(r[0]) {
            continue;
        }
        for (gi, g) in genes.iter().enumerate() {
            for col in [g.to_string(), format!("{g}.1")] {
                let v = r[hdr.iter().position(|h| *h == col).unwrap()];
                if v.is_empty() || v == "NA" {
                    continue;
                }
                let mut al: Vec<String> = vec![];
                for a in v.split('/') {
                    let x = a.split(':').take(2).collect::<Vec<_>>().join(":");
                    if !al.contains(&x) {
                        al.push(x);
                    }
                }
                let n = al.len() as f64;
                for a in al {
                    *cnt[gi].entry(a) += 1.0 / n;
                }
            }
        }
    }
    let mut o = String::from("{");
    for (gi, g) in genes.iter().enumerate() {
        let tot = pysum(cnt[gi].iter().map(|(_, v)| *v));
        let body: Vec<String> = cnt[gi].iter().map(|(a, c)| format!("\"{a}\": {}", pyfloat(c / tot))).collect();
        o += &format!("{}\"{g}\": {{{}}}", if gi > 0 { ", " } else { "" }, body.join(", "));
    }
    o.push('}');
    std::fs::write(out, o).unwrap();
}
