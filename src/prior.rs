//! allele priors of the prototype (population 2-field frequency, 3-field and 4-field weak priors)
use crate::db::Db;
use crate::names::{fields, is_null_like};
use ahash::AHashMap;
use std::sync::Mutex;

const W_FREQ: f64 = 1.0;
const W3: f64 = 1.0;
const W4: f64 = 0.3;
pub const PRIOR_NULL: f64 = 3.0;
pub const PRIOR_IMPUTED: f64 = 1.0;
static NOFREQ: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var("HLA_NOFREQ").map_or(false, |v| v != "0"));

pub struct Priors<'a> {
    db: &'a Db,
    freq: AHashMap<String, AHashMap<String, f64>>,
    n2: Mutex<AHashMap<String, (AHashMap<String, usize>, usize)>>,
    n3: Mutex<AHashMap<String, (AHashMap<String, usize>, AHashMap<String, usize>)>>,
}

fn two_nogene(name: &str) -> String {
    name.split('*').nth(1).unwrap_or("").split(':').take(2).collect::<Vec<_>>().join(":")
}

impl<'a> Priors<'a> {
    pub fn new(db: &'a Db) -> Self {
        let freq = db.freq.iter().map(|(g, v)| (g.clone(), v.iter().cloned().collect())).collect();
        Priors { db, freq, n2: Mutex::new(AHashMap::new()), n3: Mutex::new(AHashMap::new()) }
    }

    pub fn freq_prior(&self, gene: &str, name: &str) -> f64 {
        // HLA_NOFREQ=1: no population prior at all (every two-field group equal), to measure its effect
        if *NOFREQ {
            return 0.0;
        }
        let fr = self.freq.get(gene).filter(|f| !f.is_empty());
        let two = two_nogene(name);
        match fr {
            None => {
                let mut n2 = self.n2.lock().unwrap();
                let e = n2.entry(gene.to_string()).or_insert_with(|| {
                    let mut c: AHashMap<String, usize> = AHashMap::new();
                    for a in &self.db.ref_[gene] {
                        *c.entry(two_nogene(&a.name)).or_default() += 1;
                    }
                    let mx = *c.values().max().unwrap();
                    (c, mx)
                });
                let c = *e.0.get(&two).unwrap_or(&0) as f64;
                W_FREQ * (-3.0f64).max(0.5 * (c / e.1 as f64).ln())
            }
            Some(f) => W_FREQ * (f.get(&two).copied().unwrap_or(0.0) + 2e-4).ln(),
        }
    }

    pub fn sub_prior(&self, gene: &str, name: &str) -> f64 {
        let f = fields(name);
        if f.len() < 3 {
            return 0.0;
        }
        let mut n3 = self.n3.lock().unwrap();
        let (c3, c2) = n3.entry(gene.to_string()).or_insert_with(|| {
            let mut c3: AHashMap<String, usize> = AHashMap::new();
            for a in &self.db.ref_[gene] {
                let ff = fields(&a.name);
                *c3.entry(ff[..ff.len().min(3)].join(":")).or_default() += 1;
            }
            let mut c2: AHashMap<String, usize> = AHashMap::new();
            for (k, v) in &c3 {
                let k2 = k.split(':').take(2).collect::<Vec<_>>().join(":");
                let e = c2.entry(k2).or_default();
                *e = (*e).max(*v);
            }
            (c3, c2)
        });
        let k3 = f[..3].join(":");
        let k2 = f[..2].join(":");
        let a = *c3.get(&k3).unwrap_or(&0) as f64;
        let b = (*c2.get(&k2).unwrap_or(&0)).max(1) as f64;
        W3 * (-4.0f64).max((a / b).ln())
    }

    pub fn f4_prior(&self, name: &str) -> f64 {
        let f = fields(name);
        if f.len() < 4 || f[3].is_empty() || !f[3].bytes().all(|c| c.is_ascii_digit()) {
            return 0.0;
        }
        -W4 * (1.0 + f[3].parse::<f64>().unwrap()).log2()
    }

    /// prior used in exon refinement and phase check
    pub fn name_prior(&self, gene: &str, name: &str) -> f64 {
        self.freq_prior(gene, name) + self.sub_prior(gene, name) - if is_null_like(name) { PRIOR_NULL } else { 0.0 }
    }
}
