use flate2::read::MultiGzDecoder;
use std::io::{BufRead, BufReader};

pub fn open_reader(p: &str) -> Box<dyn BufRead> {
    let f = std::fs::File::open(p).unwrap_or_else(|e| panic!("{p}: {e}"));
    if p.ends_with(".gz") {
        Box::new(BufReader::with_capacity(1 << 20, MultiGzDecoder::new(f)))
    } else {
        Box::new(BufReader::with_capacity(1 << 20, f))
    }
}

pub struct Fq {
    pub name: String,
    pub seq: Vec<u8>,
}

/// next n FASTQ records (name = header up to first space, /1 /2 stripped)
pub fn read_fq(r: &mut dyn BufRead, n: usize) -> Vec<Fq> {
    let mut out = Vec::with_capacity(n);
    let mut l = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for _ in 0..n {
        for (i, x) in l.iter_mut().enumerate() {
            x.clear();
            if r.read_until(b'\n', x).unwrap() == 0 {
                if i == 0 {
                    return out;
                }
                panic!("truncated fastq");
            }
            while x.last().map_or(false, |c| *c == b'\n' || *c == b'\r') {
                x.pop();
            }
        }
        let h = &l[0][1..];
        let e = h.iter().position(|&c| c == b' ' || c == b'\t').unwrap_or(h.len());
        let mut nm = &h[..e];
        if nm.len() > 2 && nm[nm.len() - 2] == b'/' {
            nm = &nm[..nm.len() - 2];
        }
        out.push(Fq { name: String::from_utf8_lossy(nm).into_owned(), seq: l[1].clone() });
    }
    out
}

pub fn revcomp(s: &[u8]) -> Vec<u8> {
    s.iter()
        .rev()
        .map(|&c| match c {
            b'A' => b'T',
            b'C' => b'G',
            b'G' => b'C',
            b'T' => b'A',
            b'a' => b't',
            b'c' => b'g',
            b'g' => b'c',
            b't' => b'a',
            x => x,
        })
        .collect()
}
