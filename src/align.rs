//! single-sequence alignments with the own aligner (src/aln.rs)
use crate::aln::{Index, Opts};

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

pub fn aligner(seq: &[u8], p: Preset) -> Index {
    Index::single(seq, match p {
        Preset::Sr => Opts::sr_single(),
        Preset::Asm10 => Opts::asm10(),
    })
}

pub fn map(al: &Index, q: &[u8]) -> Vec<Hit> {
    al.map(q)
        .into_iter()
        .map(|m| Hit { fwd: m.fwd, q_st: m.q_st as i64, q_en: m.q_en as i64, r_st: m.r_st as i64, r_en: m.r_en as i64, mlen: m.mlen as i64, nm: m.nm as i64 })
        .collect()
}

/// first hit with the largest number of matching bases
pub fn best_mlen(hs: &[Hit]) -> Option<&Hit> {
    let mut b: Option<&Hit> = None;
    for h in hs {
        if b.map_or(true, |x| h.mlen > x.mlen) {
            b = Some(h);
        }
    }
    b
}
