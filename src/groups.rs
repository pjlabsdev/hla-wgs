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
        Groups { g: g.unwrap_or_default(), p: p.unwrap_or_default(), loaded }
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
}
