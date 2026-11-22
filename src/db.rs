//! Reference package: IMGT alleles per gene (genomic, exons), population 2-field frequencies,
//! precomputed donor edit distances.
use ahash::AHashMap;
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize)]
pub struct AlleleJ {
    pub name: String,
    pub genomic: String,
    pub exons: Vec<(u32, String)>,
    pub imputed: bool,
    pub partial: bool,
    pub extended: bool,
    pub imputed_from: Option<String>,
    pub orig: Option<String>,
}

#[derive(Deserialize)]
struct GeneJ {
    gene: String,
    alleles: Vec<AlleleJ>,
}

#[derive(Deserialize)]
struct DbJ {
    genes: Vec<GeneJ>,
    freq: serde_json::Map<String, serde_json::Value>,
    donor_nm: Vec<(String, String, String, Option<i64>)>,
}

pub struct Allele {
    pub name: String,
    pub genomic: Vec<u8>,
    /// exon number -> sequence, sorted by number
    pub exons: Vec<(u32, Vec<u8>)>,
    pub imputed: bool,
    pub partial: bool,
    pub extended: bool,
    pub imputed_from: Option<String>,
    /// sequence before flank extension (extended alleles only)
    pub orig: Option<Vec<u8>>,
}

impl Allele {
    pub fn exon(&self, k: u32) -> Option<&[u8]> {
        self.exons.iter().find(|e| e.0 == k).map(|e| e.1.as_slice())
    }
}

pub struct Db {
    /// genes in package order
    pub genes: Vec<String>,
    pub ref_: AHashMap<String, Vec<Allele>>,
    /// gene -> ordered (2-field, freq)
    pub freq: AHashMap<String, Vec<(String, f64)>>,
    pub donor_nm: AHashMap<(String, String, String), Option<i64>>,
    /// gene -> copies of dispersed repeats elsewhere in the reference genome that resemble
    /// repeat elements inside the gene (repeat_decoys.fa next to the database, optional)
    pub genome_decoy: AHashMap<String, Vec<Vec<u8>>>,
}

/// FASTA with headers ">GENE|location"
fn load_genome_decoys(dbpath: &str) -> AHashMap<String, Vec<Vec<u8>>> {
    let mut out: AHashMap<String, Vec<Vec<u8>>> = AHashMap::new();
    let p = std::env::var("HLA_DECOYS").unwrap_or_else(|_| {
        std::path::Path::new(dbpath).with_file_name("repeat_decoys.fa").to_string_lossy().into_owned()
    });
    let Ok(txt) = crate::vfs::read_to_string(&p) else { return out };
    let mut cur: Option<String> = None;
    for line in txt.lines() {
        if let Some(h) = line.strip_prefix('>') {
            let g = h.split('|').next().unwrap().to_string();
            out.entry(g.clone()).or_default().push(vec![]);
            cur = Some(g);
        } else if let Some(g) = &cur {
            out.get_mut(g).unwrap().last_mut().unwrap().extend(line.trim().bytes().map(|b| b.to_ascii_uppercase()));
        }
    }
    out
}

impl Db {
    pub fn load(path: &str) -> Db {
        let f = crate::vfs::open(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let mut s = String::new();
        if path.ends_with(".gz") {
            flate2::read::GzDecoder::new(f).read_to_string(&mut s).unwrap();
        } else {
            std::io::BufReader::new(f).read_to_string(&mut s).unwrap();
        }
        let j: DbJ = serde_json::from_str(&s).expect("db json");
        let mut genes = vec![];
        let mut ref_ = AHashMap::new();
        for g in j.genes {
            genes.push(g.gene.clone());
            let al = g
                .alleles
                .into_iter()
                .map(|a| Allele {
                    name: a.name,
                    genomic: a.genomic.into_bytes(),
                    exons: a.exons.into_iter().map(|(k, v)| (k, v.into_bytes())).collect(),
                    imputed: a.imputed,
                    partial: a.partial,
                    extended: a.extended,
                    imputed_from: a.imputed_from,
                    orig: a.orig.map(|x| x.into_bytes()),
                })
                .collect();
            ref_.insert(g.gene, al);
        }
        let mut freq = AHashMap::new();
        for (g, v) in j.freq {
            let m = v.as_object().expect("freq");
            freq.insert(g, m.iter().map(|(k, x)| (k.clone(), x.as_f64().unwrap())).collect());
        }
        let donor_nm = j.donor_nm.into_iter().map(|(g, a, b, v)| ((g, a, b), v)).collect();
        Db { genes, ref_, freq, donor_nm, genome_decoy: load_genome_decoys(path) }
    }
}
