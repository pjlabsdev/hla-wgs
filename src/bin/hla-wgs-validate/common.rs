//! Shared pieces: files, the database JSON, allele name fields, an insertion-ordered map and
//! alignment records in the form of the PAF lines the replaced scripts read.
use crate::aln::{Hit, Index, Opts};
use serde_json::Value;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Read;

pub fn read(path: &str) -> String {
    let mut s = String::new();
    crate::util::open_reader(path).read_to_string(&mut s).unwrap_or_else(|e| panic!("{path}: {e}"));
    s
}

pub fn exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

pub fn env_or(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.to_string())
}

/// the database as written by `hla-wgs build-db`
pub fn db(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

pub fn genes(db: &Value) -> &Vec<Value> {
    db["genes"].as_array().expect("genes")
}

pub fn alleles(g: &Value) -> &Vec<Value> {
    g["alleles"].as_array().expect("alleles")
}

pub fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}

pub fn b(v: &Value, k: &str) -> bool {
    match &v[k] {
        Value::Bool(x) => *x,
        Value::Null => false,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(x) => !x.is_empty(),
        Value::Array(x) => !x.is_empty(),
        Value::Object(x) => !x.is_empty(),
    }
}

/// the allele's own sequence: the original one when the database extended it with flanks
pub fn own_seq(a: &Value) -> &str {
    if b(a, "extended") && !s(a, "orig").is_empty() { s(a, "orig") } else { s(a, "genomic") }
}

/// first k fields of an allele name, expression suffix removed (A*02:01:01:01N -> 02:01)
pub fn cut(n: &str, k: usize) -> String {
    let f = n.split('*').nth(1).unwrap_or("");
    let f = f.strip_suffix(|c: char| c.is_ascii_uppercase()).unwrap_or(f);
    f.split(':').take(k).collect::<Vec<_>>().join(":")
}

/// number of fields of a name or field string
pub fn nfields(x: &str) -> usize {
    x.split(':').count()
}

pub fn revcomp(s: &str) -> String {
    s.bytes().rev().map(|c| match c {
        b'A' => 'T', b'C' => 'G', b'G' => 'C', b'T' => 'A',
        b'a' => 't', b'c' => 'g', b'g' => 'c', b't' => 'a',
        x => x as char,
    }).collect()
}

/// a map that remembers the order of first insertion, like a Python dict
#[derive(Clone, Debug)]
pub struct OMap<K: Eq + Hash + Clone, V> {
    pub keys: Vec<K>,
    pub map: HashMap<K, V>,
}

impl<K: Eq + Hash + Clone, V> Default for OMap<K, V> {
    fn default() -> Self {
        OMap { keys: vec![], map: HashMap::new() }
    }
}

impl<K: Eq + Hash + Clone, V> OMap<K, V> {
    pub fn entry(&mut self, k: K) -> &mut V
    where
        V: Default,
    {
        if !self.map.contains_key(&k) {
            self.keys.push(k.clone());
            self.map.insert(k.clone(), V::default());
        }
        self.map.get_mut(&k).unwrap()
    }
    pub fn insert(&mut self, k: K, v: V) {
        if !self.map.contains_key(&k) {
            self.keys.push(k.clone());
        }
        self.map.insert(k, v);
    }
    pub fn get(&self, k: &K) -> Option<&V> {
        self.map.get(k)
    }
    pub fn contains(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().map(|k| (k, &self.map[k]))
    }
    pub fn len(&self) -> usize {
        self.keys.len()
    }
}

/// a counter of named quantities, read as 0 when absent
#[derive(Clone, Debug, Default)]
pub struct Counter(pub HashMap<String, i64>);

impl Counter {
    pub fn add(&mut self, k: &str, v: i64) {
        *self.0.entry(k.to_string()).or_insert(0) += v;
    }
    pub fn get(&self, k: &str) -> i64 {
        *self.0.get(k).unwrap_or(&0)
    }
    pub fn update(&mut self, o: &Counter) {
        for (k, v) in &o.0 {
            self.add(k, *v);
        }
    }
}

/// one alignment as minimap2 printed it in PAF (with -c and --cs)
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Paf {
    pub qname: String,
    pub qlen: usize,
    pub qs: usize,
    pub qe: usize,
    pub fwd: bool,
    pub tname: String,
    pub ts: usize,
    pub te: usize,
    pub nm: usize,
    pub cs: String,
    /// (op, length) in target orientation of the query: = X I D
    pub ops: Vec<(u8, u32)>,
}

pub fn paf_of(ix: &Index, q: &str, qname: &str, h: &Hit) -> Paf {
    let qb = q.as_bytes();
    Paf {
        qname: qname.to_string(),
        qlen: qb.len(),
        qs: h.q_st,
        qe: h.q_en,
        fwd: h.fwd,
        tname: ix.names[h.seq].clone(),
        ts: h.r_st,
        te: h.r_en,
        nm: h.nm,
        cs: h.cs(&qb.to_ascii_uppercase(), &ix.seqs[h.seq]),
        ops: h.ops.clone(),
    }
}

/// the alignments minimap2 reports with --secondary=no: the primary and supplementary ones
pub fn map_primary(ix: &Index, q: &str, qname: &str) -> Vec<Paf> {
    let hs = ix.map(q.as_bytes());
    crate::aln::primary_set(&hs).into_iter().map(|i| paf_of(ix, q, qname, &hs[i])).collect()
}

pub fn index_fasta(path: &str, opts: Opts) -> Index {
    Index::from_fasta(path, opts)
}

/// contig sequences of a FASTA in file order (name = first word of the header)
pub fn fasta(path: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![];
    for l in read(path).lines() {
        if let Some(h) = l.strip_prefix('>') {
            out.push((h.split_whitespace().next().unwrap_or("").to_string(), String::new()));
        } else if let Some(x) = out.last_mut() {
            x.1.push_str(l.trim());
        }
    }
    out
}

/// Python's str(list of str): ['a', 'b']
pub fn pylist(v: &[String]) -> String {
    format!("[{}]", v.iter().map(|x| pyrepr(x)).collect::<Vec<_>>().join(", "))
}

pub fn pyrepr(x: &str) -> String {
    if x.contains('\'') && !x.contains('"') { format!("\"{x}\"") } else { format!("'{}'", x.replace('\\', "\\\\").replace('\'', "\\'")) }
}

/// Python's formatting of a float with fixed decimals (round half to even on the exact binary value,
/// as Rust does)
pub fn fx(v: f64, d: usize) -> String {
    format!("{v:.d$}")
}

/// all permutations of 0..n in Python's itertools order
pub fn perms(n: usize) -> Vec<Vec<usize>> {
    fn rec(pool: &mut Vec<usize>, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if pool.is_empty() {
            out.push(cur.clone());
            return;
        }
        for i in 0..pool.len() {
            let x = pool.remove(i);
            cur.push(x);
            rec(pool, cur, out);
            cur.pop();
            pool.insert(i, x);
        }
    }
    let mut out = vec![];
    rec(&mut (0..n).collect(), &mut vec![], &mut out);
    out
}

/// Python's repr of a float (shortest digits that read back; scientific below 1e-4 and from 1e16)
pub fn pyfloat(v: f64) -> String {
    if v.is_nan() {
        return "nan".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    let e = format!("{v:e}"); // d.ddde-x with the shortest digits
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if neg { "-" } else { "" };
    if exp < -4 || exp >= 16 {
        let m = if digits.len() > 1 { format!("{}.{}", &digits[..1], &digits[1..]) } else { digits.clone() };
        return format!("{sign}{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    let n = digits.len() as i32;
    if exp >= 0 {
        if n - 1 <= exp {
            format!("{sign}{}{}.0", digits, "0".repeat((exp - (n - 1)) as usize))
        } else {
            let p = (exp + 1) as usize;
            format!("{sign}{}.{}", &digits[..p], &digits[p..])
        }
    } else {
        format!("{sign}0.{}{}", "0".repeat((-exp - 1) as usize), digits)
    }
}

/// Python 3.12+ sum() of floats (Neumaier compensation)
pub fn pysum(xs: impl Iterator<Item = f64>) -> f64 {
    let (mut f, mut c) = (0.0f64, 0.0f64);
    for x in xs {
        let t = f + x;
        if f.abs() >= x.abs() { c += (f - t) + x; } else { c += (x - t) + f; }
        f = t;
    }
    if c != 0.0 && c.is_finite() { f + c } else { f }
}
