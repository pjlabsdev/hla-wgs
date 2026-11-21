//! G and P groups (alleles identical in the exons of the peptide-binding domains, at nucleotide and
//! protein level) from the WMDA files `hla_nom_g.txt` and `hla_nom_p.txt` of the IPD-IMGT/HLA
//! release, read from the database directory when they are present. Also the expression suffix of
//! an allele name and the pharmacogenetic alleles with a CPIC guideline.
use ahash::AHashMap;

#[derive(Default)]
pub struct Groups {
    g: AHashMap<String, String>,
    p: AHashMap<String, String>,
    pub loaded: bool,
    ser: Serology,
}

/// WHO serological equivalents from the WMDA files `rel_dna_ser.txt` (allele to antigen code) and
/// `rel_ser_ser.txt` (splits of broad antigens, codes of associated antigens)
#[derive(Default)]
pub struct Serology {
    /// allele (with gene and star) to (code, how it was assigned)
    dna: AHashMap<String, (String, u8)>,
    /// (serological locus, code of an associated antigen) to its antigen
    assoc: AHashMap<(String, String), String>,
    /// (serological locus, split) to its broad antigen
    broad: AHashMap<(String, String), String>,
    pub loaded: bool,
}

/// serological locus of a gene: HLA-C antigens are written Cw, DRB3/4/5 give DR52/53/51
fn ser_locus(gene: &str) -> Option<&'static str> {
    Some(match gene {
        "A" => "A",
        "B" => "B",
        "C" => "Cw",
        "DRB1" | "DRB3" | "DRB4" | "DRB5" => "DR",
        "DQB1" => "DQ",
        _ => return None,
    })
}

impl Serology {
    fn parse(dna: &str, ser: &str) -> Serology {
        let mut s = Serology { loaded: true, ..Default::default() };
        for l in dna.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = l.split(';').collect();
            if f.len() < 6 {
                continue;
            }
            // unambiguous, else expert, else assumed; "possible" lists several and is left out
            let pick = [(f[2], 0u8), (f[5], 2), (f[4], 1)].into_iter().find(|(x, _)| !x.is_empty() && *x != "?");
            if let Some((c, k)) = pick {
                s.dna.insert(format!("{}{}", f[0], f[1]), (c.to_string(), k));
            }
        }
        for l in ser.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = l.split(';').collect();
            if f.len() < 4 {
                continue;
            }
            for sp in f[2].split('/').filter(|x| !x.is_empty()) {
                s.broad.insert((f[0].to_string(), sp.to_string()), f[1].to_string());
            }
            for c in f[3].split('/').filter(|x| !x.is_empty()) {
                s.assoc.insert((f[0].to_string(), c.to_string()), f[1].to_string());
            }
        }
        s
    }

    /// antigen of an allele, e.g. "B60 (B40)" for a split with its broad antigen, "–" for a null allele;
    /// the flag says whether the assignment is assumed or by experts rather than tested
    pub fn antigen(&self, allele: &str) -> Option<(String, bool)> {
        let gene = allele.split('*').next()?;
        let loc = ser_locus(gene)?;
        // the name as given, else shorter: a new allele named after its nearest neighbour has its protein
        let mut name = allele.to_string();
        let (code, kind) = loop {
            if let Some(x) = self.dna.get(&name) {
                break x.clone();
            }
            let cut = name.rfind(':')?;
            name.truncate(cut);
            if name.matches(':').count() < 1 {
                return None;
            }
        };
        if code == "0" {
            return Some(("–".to_string(), kind > 0));
        }
        let ag = if let Some(a) = self.assoc.get(&(loc.to_string(), code.clone())) {
            a.clone()
        } else if code.len() == 4 {
            code[..2].trim_start_matches('0').to_string()
        } else {
            code.clone()
        };
        let txt = match self.broad.get(&(loc.to_string(), ag.clone())) {
            Some(b) => format!("{loc}{ag} ({loc}{b})"),
            None => format!("{loc}{ag}"),
        };
        Some((txt, kind > 0))
    }
}

fn parse(text: &str) -> AHashMap<String, String> {
    let mut m = AHashMap::new();
    for l in text.lines() {
        if l.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = l.split(';').collect();
        if f.len() < 2 {
            continue;
        }
        let gene = f[0];
        let name = f.get(2).map(|x| x.trim()).unwrap_or("");
        for a in f[1].split('/') {
            if a.is_empty() {
                continue;
            }
            // an allele alone in its group is its own group
            let grp = if name.is_empty() { String::new() } else { format!("{gene}{name}") };
            m.insert(format!("{gene}{a}"), grp);
        }
    }
    m
}

impl Groups {
    /// `dir`: directory of the database; missing files give an empty table
    pub fn load(dir: &std::path::Path) -> Groups {
        let rd = |n: &str| std::fs::read_to_string(dir.join(n)).ok().map(|t| parse(&t));
        let (g, p) = (rd("hla_nom_g.txt"), rd("hla_nom_p.txt"));
        let loaded = g.is_some() || p.is_some();
        let ser = match (std::fs::read_to_string(dir.join("rel_dna_ser.txt")), std::fs::read_to_string(dir.join("rel_ser_ser.txt"))) {
            (Ok(d), Ok(s)) => Serology::parse(&d, &s),
            _ => Serology::default(),
        };
        Groups { g: g.unwrap_or_default(), p: p.unwrap_or_default(), loaded, ser }
    }
    fn get(m: &AHashMap<String, String>, allele: &str) -> Option<String> {
        m.get(allele).filter(|x| !x.is_empty()).cloned()
    }
    pub fn g(&self, allele: &str) -> Option<String> {
        Self::get(&self.g, allele)
    }
    pub fn p(&self, allele: &str) -> Option<String> {
        Self::get(&self.p, allele)
    }
    pub fn serology(&self) -> &Serology {
        &self.ser
    }
}

/// ligand of the natural killer cell receptors, from the protein at residues 77 to 83 of the mature
/// chain (class I signal peptide: 24 residues): Bw4 ends in ...LR at 82-83 (HLA-A and -B), Bw6 in G at 83
/// (HLA-B only); residue 80 (I or T) sets how strongly Bw4 binds KIR3DL1. HLA-C: N80 is C1, K80 is C2.
pub fn ligand(gene: &str, cds: &[u8]) -> Option<String> {
    if !matches!(gene, "A" | "B" | "C") {
        return None;
    }
    let aa = |i: usize| -> Option<u8> {
        let k = (24 + i - 1) * 3;
        cds.get(k..k + 3).map(|c| codon(c))
    };
    let (r80, r82, r83) = (aa(80)?, aa(82)?, aa(83)?);
    match gene {
        "C" => match r80 {
            b'N' => Some("C1".into()),
            b'K' => Some("C2".into()),
            _ => None,
        },
        _ if r82 == b'L' && r83 == b'R' => Some(format!("Bw4-80{}", r80 as char)),
        "B" if r83 == b'G' => Some("Bw6".into()),
        _ => None,
    }
}

fn codon(c: &[u8]) -> u8 {
    const AA: &[u8; 64] = b"KNKNTTTTRSRSIIMIQHQHPPPPRRRRLLLLEDEDAAAAGGGGVVVV*Y*YSSSS*CWCLFLF";
    let v = |b: u8| match b { b'A' => Some(0), b'C' => Some(1), b'G' => Some(2), b'T' => Some(3), _ => None };
    match (v(c[0]), v(c[1]), v(c[2])) {
        (Some(a), Some(b), Some(d)) => AA[a * 16 + b * 4 + d],
        _ => b'X',
    }
}

/// meaning of the letter at the end of an allele name
pub fn expression(allele: &str) -> Option<&'static str> {
    match allele.as_bytes().last() {
        Some(b'N') => Some("null (not expressed)"),
        Some(b'L') => Some("low cell-surface expression"),
        Some(b'S') => Some("secreted only"),
        Some(b'Q') => Some("questionable expression"),
        Some(b'C') => Some("cytoplasm only"),
        Some(b'A') => Some("aberrant expression"),
        _ => None,
    }
}

/// two-field alleles with a CPIC guideline: (allele, drugs)
pub const PGX: [(&str, &str); 4] = [
    ("B*57:01", "abacavir"),
    ("B*15:02", "carbamazepine, oxcarbazepine, phenytoin, fosphenytoin"),
    ("A*31:01", "carbamazepine"),
    ("B*58:01", "allopurinol"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_groups() {
        let m = parse("# c\nA*;01:01:01:01/01:01:01:02N;01:01:01G\nA*;01:01:02;\n");
        assert_eq!(m["A*01:01:01:02N"], "A*01:01:01G");
        assert_eq!(m["A*01:01:02"], "");
        assert_eq!(expression("A*01:01:01:02N"), Some("null (not expressed)"));
        assert_eq!(expression("A*01:01:01:01"), None);
    }
    #[test]
    fn serology_and_ligands() {
        let s = Serology::parse(
            "B*;40:01:01;4001;;;;4001\nB*;15:01:01:01;1501;;;;1501\nA*;01:01:01:02N;0;;;;\nA*;02:01:01:01;2;;;;2\nC*;07:01:01:01;0701;;;;0701\nDRB3*;02:02:01:01;5202;;;;5202\nDQB1*;02:01:01;2;;;;2\n",
            "B;40;60/61;\nB;60;;4001/4016\nB;15;62/63;\nB;62;;1501/1520\nCw;7;;0701/0702\nDR;52;;5201/5202\n",
        );
        assert_eq!(s.antigen("B*40:01:01").unwrap().0, "B60 (B40)");
        assert_eq!(s.antigen("B*15:01:01:01").unwrap().0, "B62 (B15)");
        assert_eq!(s.antigen("A*02:01:01:01").unwrap().0, "A2");
        assert_eq!(s.antigen("A*01:01:01:02N").unwrap().0, "–");
        assert_eq!(s.antigen("C*07:01:01:01").unwrap().0, "Cw7");
        assert_eq!(s.antigen("DRB3*02:02:01:01").unwrap().0, "DR52");
        assert_eq!(s.antigen("DQB1*02:01:01").unwrap().0, "DQ2");
        assert!(s.antigen("DPB1*04:01:01:01").is_none());
        // residues 77-83 of the mature chain after a 24-residue signal peptide
        let mk = |m: &str| { let mut p = "M".repeat(24 + 76); p.push_str(m); p.push_str("WWW"); p.bytes().map(|a| match a { b'M' => "ATG", b'N' => "AAC", b'L' => "CTG", b'R' => "CGC", b'I' => "ATC", b'A' => "GCC", b'S' => "AGC", b'G' => "GGC", b'K' => "AAG", b'T' => "ACC", _ => "TGG" }).collect::<String>() };
        assert_eq!(ligand("B", mk("NLRIALR").as_bytes()).as_deref(), Some("Bw4-80I"));
        assert_eq!(ligand("B", mk("SLRNLRG").as_bytes()).as_deref(), Some("Bw6"));
        assert_eq!(ligand("A", mk("NLGTLRG").as_bytes()), None);
        assert_eq!(ligand("A", mk("NLRIALR").as_bytes()).as_deref(), Some("Bw4-80I"));
        assert_eq!(ligand("C", mk("NLRKLRG").as_bytes()).as_deref(), Some("C2"));
        assert_eq!(ligand("C", mk("SLRNLRG").as_bytes()).as_deref(), Some("C1"));
    }
}
