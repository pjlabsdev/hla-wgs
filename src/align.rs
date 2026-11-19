//! single-sequence minimap2 alignments as used through mappy in the prototype
use minimap2::{Aligner, Strand};

pub struct Hit {
    pub fwd: bool,
    pub q_st: i64,
    pub q_en: i64,
    pub r_st: i64,
    pub r_en: i64,
    pub mlen: i64,
    pub nm: i64,
}

pub enum Preset {
    Sr,
    Asm10,
}

pub fn aligner(seq: &[u8], p: Preset) -> Aligner<minimap2::Built> {
    let b = Aligner::builder();
    let b = match p {
        Preset::Sr => b.sr(),
        Preset::Asm10 => b.asm10(),
    };
    b.with_cigar().with_seq(seq).expect("index")
}

pub fn map(al: &Aligner<minimap2::Built>, q: &[u8]) -> Vec<Hit> {
    al.map(q, false, false, None, None, None)
        .unwrap_or_default()
        .into_iter()
        .map(|m| Hit {
            fwd: m.strand == Strand::Forward,
            q_st: m.query_start as i64,
            q_en: m.query_end as i64,
            r_st: m.target_start as i64,
            r_en: m.target_end as i64,
            mlen: m.match_len as i64,
            nm: m.alignment.as_ref().map(|a| a.nm as i64).unwrap_or(0),
        })
        .collect()
}

/// Python max(hs, key=mlen): first hit with the largest mlen
pub fn best_mlen(hs: &[Hit]) -> Option<&Hit> {
    let mut b: Option<&Hit> = None;
    for h in hs {
        if b.map_or(true, |x| h.mlen > x.mlen) {
            b = Some(h);
        }
    }
    b
}
