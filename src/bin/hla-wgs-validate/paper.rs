//! paper: the software report (HTML with inline SVG figures) from the files under validation/results,
//! so that the text cannot drift from the data (formerly docs/paper/build_paper.py).
//!
//!     hla-wgs-validate paper [--fonts DIR] [--version V] [--date D] [--doi DOI] > docs/paper/hla-wgs-report.html
//!
//! --fonts points to a directory with the IBM Plex Mono and Instrument Sans woff2 files (fontsource
//! layout); they are embedded so that the PDF does not depend on installed fonts.
//!
//! The text lives in report/*.html. A placeholder {expr} holds a value from the data: a path such as
//! a6['safe4'] or tr['han']['genes_present'], a difference of two paths, pct(k, n) or pct(*pair),
//! optionally with the format :, (thousands separator); the rest are prepared strings (figures, tables).

use crate::common::{exists, pyfloat, read};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

const INK: &str = "#141413";
const SUB: &str = "#56544C";
const LINE: &str = "#D8D5CC";
const OK: &str = "#2A7350";
const BAD: &str = "#B3261E";
const WARN: &str = "#8F5B00";
const BLUE: &str = "#2F5D8A";
const GREY: &str = "#9A978E";
const MONO: &str = "'IBM Plex Mono', ui-monospace, Menlo, monospace";
const SANS: &str = "'Instrument Sans', 'Helvetica Neue', Arial, sans-serif";

const MAIN: &str = include_str!("report/main.html");
const UNSEEN: &str = include_str!("report/unseen.html");
const STYLE: &str = include_str!("report/style.css");
const LOGO: &str = include_str!("report/logo.svg");

const TITLE: &str = "hla-wgs: HLA typing from whole-genome sequencing that states how many fields of each allele are certain";

const GENES: [&str; 18] = ["A", "B", "C", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1", "DRA", "E", "F", "G", "H", "K", "V"];

const REFS: [(&str, &str); 14] = [
    ("imgt", "Barker DJ, Maccari G, Georgiou X, et al. The IPD-IMGT/HLA Database. Nucleic Acids Res. 2023;51(D1):D1053–D1060."),
    ("nomen", "Marsh SGE, Albert ED, Bodmer WF, et al. Nomenclature for factors of the HLA system, 2010. Tissue Antigens. 2010;75(4):291–455."),
    ("optitype", "Szolek A, Schubert B, Mohr C, Sturm M, Feldhahn M, Kohlbacher O. OptiType: precision HLA typing from next-generation sequencing data. Bioinformatics. 2014;30(23):3310–3316."),
    ("hlahd", "Kawaguchi S, Higasa K, Shimizu M, Yamada R, Matsuda F. HLA-HD: an accurate HLA typing algorithm for next-generation sequencing data. Hum Mutat. 2017;38(7):788–797."),
    ("xhla", "Xie C, Yeo ZX, Wong M, et al. Fast and accurate HLA typing from short-read next-generation sequence data with xHLA. Proc Natl Acad Sci USA. 2017;114(30):8059–8064."),
    ("kourami", "Lee H, Kingsford C. Kourami: graph-guided assembly for novel human leukocyte antigen allele discovery. Genome Biol. 2018;19:16."),
    ("hlala", "Dilthey AT, Mentzer AJ, Carapito R, et al. HLA*LA: HLA typing from linearly projected graph alignments. Bioinformatics. 2019;35(21):4394–4396."),
    ("t1k", "Song L, Bai G, Liu XS, Li B, Li H. Efficient and accurate KIR and HLA genotyping with massively parallel sequencing data. Genome Res. 2023;33(6):923–931."),
    ("hprc", "Liao WW, Asri M, Ebler J, et al. A draft human pangenome reference. Nature. 2023;617:312–324."),
    ("kgp", "Byrska-Bishop M, Evani US, Zhao X, et al. High-coverage whole-genome sequencing of the expanded 1000 Genomes Project cohort including 602 trios. Cell. 2022;185(18):3426–3440."),
    ("gourraud", "Gourraud PA, Khankhanian P, Cereb N, et al. HLA diversity in the 1000 Genomes dataset. PLoS One. 2014;9(7):e97282."),
    ("abirached", "Abi-Rached L, Gouret P, Yeh JH, et al. Immune diversity sheds light on missing variation in worldwide genetic diversity panels. PLoS One. 2018;13(10):e0206512."),
    ("giabdata", "Zook JM, Catoe D, McDaniel J, et al. Extensive sequencing of seven human genomes to characterize benchmark reference materials. Sci Data. 2016;3:160025."),
    ("minimap2", "Li H. Minimap2: pairwise alignment for nucleotide sequences. Bioinformatics. 2018;34(18):3094–3100."),
];

/// html.escape(str, quote=True)
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#x27;"),
            _ => o.push(c),
        }
    }
    o
}

/// a number as Python's str() prints it: whole numbers given as int literals stay ints
#[derive(Clone, Copy)]
enum N {
    I(i64),
    F(f64),
}

impl N {
    fn f(self) -> f64 {
        match self {
            N::I(i) => i as f64,
            N::F(f) => f,
        }
    }
    fn s(self) -> String {
        match self {
            N::I(i) => i.to_string(),
            N::F(f) => pyfloat(f),
        }
    }
}

fn f1(v: f64) -> String {
    format!("{v:.1}")
}

struct T<'a> {
    anchor: &'a str,
    fill: &'a str,
    mono: bool,
    weight: u32,
}

const T0: T = T { anchor: "start", fill: INK, mono: false, weight: 400 };

fn text(x: f64, y: f64, s: &str, size: N, t: &T) -> String {
    let style = format!("font-family:{};font-size:{}px;font-weight:{};", if t.mono { MONO } else { SANS }, size.s(), t.weight);
    format!(r#"<text x="{}" y="{}" text-anchor="{}" fill="{}" style="{style}">{}</text>"#, f1(x), f1(y), t.anchor, t.fill, esc(s))
}

fn svg(w: i64, h: i64, body: &[String]) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="100%" role="img">{}</svg>"#, body.concat())
}

fn b64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                o.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}

/// first file called `name` in dir or below (glob "**/name", recursive)
fn find_file(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let p = dir.join(name);
    if p.is_file() {
        return Some(p);
    }
    let rd = std::fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(h) = find_file(&p, name) {
                return Some(h);
            }
        }
    }
    None
}

fn fonts_css(dir: Option<&str>) -> String {
    let Some(dir) = dir else { return String::new() };
    let wanted: [(&str, &str, &[&str]); 2] = [
        ("IBM Plex Mono", "ibm-plex-mono", &["400-normal", "500-normal", "400-italic"]),
        ("Instrument Sans", "instrument-sans", &["400-normal", "500-normal", "400-italic", "500-italic"]),
    ];
    let mut css = vec![];
    for (family, slug, variants) in wanted {
        for variant in variants {
            let (weight, style) = variant.split_once('-').unwrap();
            for subset in ["latin", "latin-ext"] {
                let Some(hit) = find_file(std::path::Path::new(dir), &format!("{slug}-{subset}-{variant}.woff2")) else { continue };
                let data = b64(&std::fs::read(hit).expect("font"));
                let rng = if subset == "latin" { "U+0000-00FF,U+2013-2014,U+2018-201D,U+2022,U+2026,U+2032,U+2212,U+2264-2265" } else { "U+0100-024F,U+1E00-1EFF" };
                css.push(format!(
                    "@font-face{{font-family:'{family}';font-weight:{weight};font-style:{style};src:url(data:font/woff2;base64,{data}) format('woff2');unicode-range:{rng};}}"
                ));
            }
        }
    }
    css.join("\n")
}

fn style(fonts: Option<&str>) -> String {
    STYLE
        .replace("{fonts_css(fonts)}", &fonts_css(fonts))
        .replace("{INK}", INK)
        .replace("{SUB}", SUB)
        .replace("{LINE}", LINE)
        .replace("{WARN}", WARN)
        .replace("{MONO}", MONO)
        .replace("{SANS}", SANS)
}

fn pct(k: i64, n: i64) -> String {
    format!("{:.1} %", (100 * k) as f64 / n as f64)
}

/// Replace [@key, @key] by numbers in order of first appearance and build the reference list.
fn number_citations(page: &str) -> String {
    fn key_at(s: &[u8], i: usize) -> Option<usize> {
        // '@' then [a-z0-9]+, returns end
        if s.get(i) != Some(&b'@') {
            return None;
        }
        let mut j = i + 1;
        while j < s.len() && (s[j].is_ascii_lowercase() || s[j].is_ascii_digit()) {
            j += 1;
        }
        (j > i + 1).then_some(j)
    }
    let b = page.as_bytes();
    let mut order: Vec<String> = vec![];
    let mut out = String::new();
    let mut last = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'[' {
            let mut keys = vec![];
            let mut j = i + 1;
            let mut ok = false;
            if let Some(e) = key_at(b, j) {
                keys.push(&page[j + 1..e]);
                j = e;
                loop {
                    if b.get(j) == Some(&b']') {
                        ok = true;
                        break;
                    }
                    if b[j..].starts_with(b", ") {
                        if let Some(e) = key_at(b, j + 2) {
                            keys.push(&page[j + 3..e]);
                            j = e;
                            continue;
                        }
                    }
                    break;
                }
            }
            if ok {
                let mut parts = vec![];
                for k in keys {
                    if !REFS.iter().any(|(r, _)| *r == k) {
                        panic!("unknown reference {k}");
                    }
                    if !order.iter().any(|o| o == k) {
                        order.push(k.to_string());
                    }
                    let n = order.iter().position(|o| o == k).unwrap() + 1;
                    parts.push(format!(r##"<a class="cite" href="#ref-{n}">{n}</a>"##));
                }
                out.push_str(&page[last..i]);
                out.push('[');
                out.push_str(&parts.join(", "));
                out.push(']');
                i = j + 1;
                last = i;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&page[last..]);
    let unused: Vec<&str> = REFS.iter().map(|r| r.0).filter(|k| !order.iter().any(|o| o == k)).collect();
    if !unused.is_empty() {
        panic!("references never cited: {unused:?}");
    }
    let mut items = String::new();
    for (n, k) in order.iter().enumerate() {
        let text = REFS.iter().find(|r| r.0 == k).unwrap().1;
        items.push_str(&format!("<li id=\"ref-{}\">{text}</li>\n", n + 1));
    }
    out.replace("{REFS}", &format!("<ol class=\"refs\">\n{items}</ol>"))
}

fn rows(path: &str) -> Vec<HashMap<String, String>> {
    let txt = read(path);
    let lines: Vec<Vec<&str>> = txt.lines().filter(|l| !l.trim().is_empty()).map(|l| l.split('\t').collect()).collect();
    let head = &lines[0];
    lines[1..].iter().filter(|l| l.len() == head.len()).map(|l| head.iter().zip(l).map(|(a, b)| (a.to_string(), b.to_string())).collect()).collect()
}

fn frac(s: &str) -> (i64, i64) {
    let (a, b) = s.split_once('/').unwrap();
    (a.parse().unwrap(), b.parse().unwrap())
}

/// gene -> column -> int, as a JSON object (keeps the evaluator uniform)
fn gene_cols(path: &str) -> Value {
    let mut m = Map::new();
    let head = read(path).lines().next().unwrap().to_string();
    for r in rows(path) {
        let mut o = Map::new();
        for k in head.split('\t').filter(|k| *k != "gene") {
            o.insert(k.to_string(), json!(r[k].parse::<i64>().unwrap()));
        }
        m.insert(r["gene"].clone(), Value::Object(o));
    }
    Value::Object(m)
}

fn gi(v: &Value, g: &str, k: &str) -> i64 {
    v[g][k].as_i64().unwrap_or_else(|| panic!("missing {g} {k}"))
}

struct Data {
    s: Value,
    strata: Vec<HashMap<String, String>>,
    dev6: Value,
    dev23: Value,
    one: i64,
    many: i64,
    un: Option<(Value, Value)>,
}

impl Data {
    fn new(res: &str) -> Data {
        let s: Value = serde_json::from_str(&read(&format!("{res}/summary.json"))).expect("summary.json");
        let strata = rows(&format!("{res}/heldout_vs_t1k/strata.tsv"));
        let dev6 = gene_cols(&format!("{res}/dev80/eval6.tsv"));
        let dev23 = gene_cols(&format!("{res}/dev80/eval23.tsv"));
        let mut nd = vec![];
        for l in read(&format!("{res}/dev80/truth4.tsv")).split_inclusive('\n') {
            let r: Vec<&str> = l.trim_end_matches('\n').split('\t').collect();
            if r.len() >= 6 && r[2] != "_done" && !r[3].is_empty() && r[3].chars().all(|c| c.is_ascii_digit()) {
                let x: i64 = r[3].parse().unwrap();
                if x > 0 {
                    nd.push(x);
                }
            }
        }
        let one = nd.iter().filter(|&&x| x == 1).count() as i64;
        let many = nd.iter().filter(|&&x| x >= 10).count() as i64;
        let u = format!("{res}/check30/eval6.tsv");
        let un = exists(&u).then(|| (gene_cols(&u), gene_cols(&format!("{res}/check30/eval23.tsv"))));
        Data { s, strata, dev6, dev23, one, many, un }
    }

    fn st(&self, group: &str, tool: &str, col: &str) -> (i64, i64) {
        let r = self.strata.iter().find(|r| r["group"] == group && r["tool"] == tool).unwrap_or_else(|| panic!("no stratum {group} {tool}"));
        frac(&r[col])
    }

    /// 150 samples of rounds 1 to 3 plus the 34 of round 4
    fn total(&self, tool: &str, col: &str) -> (i64, i64) {
        let (k, n) = self.st("all", tool, col);
        let r4 = &self.s["heldout_round4"][col][tool];
        (k + r4[0].as_i64().unwrap(), n + r4[1].as_i64().unwrap())
    }
}

// ------------------------------------------------------------------ figure 1: workflow
fn fig_workflow() -> String {
    let mut b: Vec<String> = vec![];
    let sub_t = T { anchor: "middle", fill: SUB, mono: false, weight: 400 };
    let title_t = T { anchor: "middle", fill: INK, mono: true, weight: 500 };
    let bx = |b: &mut Vec<String>, x: i64, y: i64, title: &str, sub: &[&str], fill: &str| {
        let (bw, bh) = (128, 52);
        b.push(format!(r#"<rect x="{x}" y="{y}" width="{bw}" height="{bh}" rx="5" fill="{fill}" stroke="{LINE}"/>"#));
        b.push(text(x as f64 + bw as f64 / 2.0, (y + 15) as f64, title, N::F(8.5), &title_t));
        for (i, line) in sub.iter().enumerate() {
            b.push(text(x as f64 + bw as f64 / 2.0, (y + 27 + 10 * i as i64) as f64, line, N::F(7.2), &sub_t));
        }
    };
    let arrow = |b: &mut Vec<String>, x1: i64, y1: i64, x2: i64, y2: i64| {
        b.push(format!(r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{SUB}" stroke-width="1" marker-end="url(#ah)"/>"#));
    };
    b.push(format!(
        r#"<defs><marker id="ah" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="{SUB}"/></marker></defs>"#
    ));
    let top: [(&str, [&str; 2]); 5] = [
        ("Input", ["FASTQ, BAM or CRAM", "MHC, ALT, unmapped"]),
        ("Panel mapping", ["one sequence per", "allele group, repeat mask"]),
        ("Candidates", ["k-mer likelihood,", "expectation maximisation"]),
        ("Best pair", ["edit matrix of all reads,", "margin per field"]),
        ("Copy number", ["DRB1/3/4/5 jointly,", "H and K"]),
    ];
    let mut x = 8;
    for (i, (t, sub)) in top.iter().enumerate() {
        bx(&mut b, x, 12, t, sub, "#FFFFFF");
        if i > 0 {
            arrow(&mut b, x - 16, 38, x - 1, 38);
        }
        x += 144;
    }
    arrow(&mut b, 648, 64, 648, 115);
    let low: [(&str, [&str; 2], &str); 4] = [
        ("Report", ["allele, safe call,", "resolution 1 to 4, sites"], "#F3F1EA"),
        ("Known allele?", ["called allele + sites", "= database allele: rename"], "#FFFFFF"),
        ("New-allele sites", ["consistent differences,", "stretches without reads"], "#FFFFFF"),
        ("Realignment", ["reads against the", "two called alleles"], "#FFFFFF"),
    ];
    let mut x = 584 - 3 * 144;
    for (i, (t, sub, fill)) in low.iter().enumerate() {
        bx(&mut b, x, 116, t, sub, fill);
        if i > 0 {
            arrow(&mut b, x - 1, 142, x - 15, 142);
        }
        x += 144;
    }
    b.push(text(8.0, 186.0, "Research use only. A call must be confirmed by an accredited HLA laboratory before any medical use.", N::F(7.4), &T { fill: WARN, ..T0 }));
    svg(720, 196, &b)
}

// ------------------------------------------------------------------ figure 2: held-out comparison
fn fig_heldout(d: &Data) -> String {
    let mut b: Vec<String> = vec![];
    let genes = ["A", "B", "C", "DRB1", "DQA1", "DQB1", "DPA1", "DPB1"];
    let head = T { mono: true, weight: 500, ..T0 };
    let tick = T { anchor: "end", fill: SUB, mono: true, weight: 400 };
    let val = T { anchor: "middle", fill: INK, mono: true, weight: 400 };
    b.push(text(8.0, 14.0, "A  Two-field errors per gene (150 held-out samples, 2 alleles each)", N::F(8.5), &head));
    let (x0, y0, ph) = (40i64, 196i64, 150i64);
    let tools = [("hla-wgs", BLUE), ("T1K", GREY)];
    let errs: Vec<Vec<(i64, i64)>> = tools
        .iter()
        .map(|(t, _)| genes.iter().map(|g| frac(&d.strata.iter().find(|r| r["group"] == format!("gene {g}") && r["tool"] == *t).unwrap()["two_field"])).collect())
        .collect();
    let mut top = errs.iter().flatten().map(|(k, n)| n - k).max().unwrap();
    if top == 0 {
        top = 1;
    }
    for v in (0..=top).step_by(2) {
        let y = y0 as f64 - (ph * v) as f64 / top as f64;
        b.push(format!(r#"<line x1="{x0}" y1="{}" x2="{}" y2="{}" stroke="{LINE}" stroke-width="0.6"/>"#, f1(y), x0 + 304, f1(y)));
        b.push(text((x0 - 5) as f64, y + 3.0, &v.to_string(), N::I(7), &tick));
    }
    for (i, g) in genes.iter().enumerate() {
        let gx = x0 + 8 + i as i64 * 37;
        for (j, (_, col)) in tools.iter().enumerate() {
            let (k, n) = errs[j][i];
            let e = n - k;
            let bh = (ph * e) as f64 / top as f64;
            let j = j as i64;
            b.push(format!(r#"<rect x="{}" y="{}" width="12" height="{}" fill="{col}"/>"#, gx + j * 13, f1(y0 as f64 - bh), f1(bh.max(0.6))));
            b.push(text((gx + j * 13 + 6) as f64, y0 as f64 - bh - 3.0, &e.to_string(), N::F(6.8), &val));
        }
        b.push(text(gx as f64 + 12.5, (y0 + 11) as f64, g, N::F(7.2), &val));
    }
    b.push(text(380.0, 14.0, "B  DRB3/4/5 copy number correct (%)", N::F(8.5), &head));
    let x1 = 420i64;
    for v in [0i64, 25, 50, 75, 100] {
        let y = y0 as f64 - (ph * v) as f64 / 100.0;
        b.push(format!(r#"<line x1="{x1}" y1="{}" x2="{}" y2="{}" stroke="{LINE}" stroke-width="0.6"/>"#, f1(y), x1 + 290, f1(y)));
        b.push(text((x1 - 5) as f64, y + 3.0, &v.to_string(), N::I(7), &tick));
    }
    for (i, p) in ["EUR", "AFR", "EAS", "SAS", "AMR"].iter().enumerate() {
        let gx = x1 + 12 + i as i64 * 56;
        for (j, (t, col)) in tools.iter().enumerate() {
            let (k, n) = d.st(p, t, "copy_number");
            let bh = (ph * k) as f64 / n as f64;
            let j = j as i64;
            b.push(format!(r#"<rect x="{}" y="{}" width="18" height="{}" fill="{col}"/>"#, gx + j * 19, f1(y0 as f64 - bh), f1(bh)));
            b.push(text((gx + j * 19 + 9) as f64, y0 as f64 - bh - 3.0, &format!("{:.0}", (100 * k) as f64 / n as f64), N::F(6.8), &val));
        }
        b.push(text(gx as f64 + 18.5, (y0 + 11) as f64, p, N::F(7.2), &val));
    }
    for (j, (t, col)) in [("hla-wgs", BLUE), ("T1K 1.0.10", GREY)].iter().enumerate() {
        let j = j as i64;
        b.push(format!(r#"<rect x="{}" y="{}" width="9" height="9" fill="{col}"/>"#, x0 + j * 80, y0 + 22));
        b.push(text((x0 + j * 80 + 13) as f64, (y0 + 30) as f64, t, N::F(7.4), &T0));
    }
    svg(720, 232, &b)
}

// ------------------------------------------------------------------ figure 3: four-field statements
fn fig_four(e6: &Value, title: &str) -> String {
    let rh = 13i64;
    let h = 40 + rh * GENES.len() as i64 + 26;
    let mut b = vec![text(8.0, 14.0, title, N::F(8.5), &T { mono: true, weight: 500, ..T0 })];
    let (x0, bw) = (62i64, 520i64);
    let cols = [
        ("certain at four fields, exact", OK),
        ("certain at four fields, not exact", BAD),
        ("not reported as certain, exact", "#A9C7B8"),
        ("not reported as certain, new or different allele", "#D9D6CC"),
    ];
    for (i, g) in GENES.iter().enumerate() {
        let y = 26 + i as i64 * rh;
        let n = gi(e6, g, "n");
        let (s4e, s4, ex) = (gi(e6, g, "safe4_exact"), gi(e6, g, "safe4"), gi(e6, g, "exact"));
        let mut parts = vec![s4e, s4 - s4e, ex - s4e];
        parts.push(n - parts.iter().sum::<i64>());
        b.push(text((x0 - 6) as f64, (y + 9) as f64, g, N::F(7.4), &T { anchor: "end", mono: true, ..T0 }));
        // x starts as the int x0 and becomes a float; it is printed with .1f only
        let mut x = x0 as f64;
        for (v, (_, col)) in parts.iter().zip(cols.iter()) {
            let ww = if n != 0 { (bw * v) as f64 / n as f64 } else { 0.0 };
            if ww > 0.0 {
                b.push(format!(r#"<rect x="{}" y="{y}" width="{}" height="{}" fill="{col}"/>"#, f1(x), f1(ww), rh - 3));
            }
            x += ww;
        }
        b.push(text((x0 + bw + 8) as f64, (y + 9) as f64, &format!("{s4e}/{s4} of {n}"), N::I(7), &T { fill: SUB, mono: true, ..T0 }));
    }
    let y = 26 + GENES.len() as i64 * rh + 12;
    let mut x = N::I(x0);
    for (label, col) in cols {
        b.push(format!(r#"<rect x="{}" y="{}" width="9" height="9" fill="{col}"/>"#, x.s(), y - 8));
        b.push(text(x.f() + 13.0, y as f64, label, N::F(7.2), &T0));
        x = N::F(x.f() + (22.0 + 4.1 * label.chars().count() as f64));
    }
    svg(720, h, &b)
}

fn gene_table(e6: &Value, e23: &Value) -> String {
    let mut out = String::new();
    for g in GENES.iter().chain(["ALL"].iter()) {
        let name = if *g == "ALL" { "All genes".to_string() } else { format!("HLA-{g}") };
        let (r, t) = (|k| gi(e6, g, k), |k| gi(e23, g, k));
        out.push_str(&format!(
            "<tr><td>{name}</td><td>{}</td><td>{} / {}</td><td>{} / {}</td><td>{} / {}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            r("n"),
            t("ok2"),
            t("n"),
            t("ok3"),
            t("n"),
            r("known_ok"),
            r("known"),
            r("new"),
            r("safe4"),
            r("safe4_exact")
        ));
    }
    out
}

// ------------------------------------------------------------------ placeholder evaluation

fn path(ctx: &Map<String, Value>, e: &str) -> Value {
    let e = e.trim();
    let (name, mut rest) = match e.find('[') {
        Some(i) => (&e[..i], &e[i..]),
        None => (e, ""),
    };
    let mut v = ctx.get(name).unwrap_or_else(|| panic!("unknown name {name} in template")).clone();
    while !rest.is_empty() {
        let end = rest.find(']').expect("]");
        let k = &rest[1..end];
        v = if let Some(q) = k.strip_prefix('\'') {
            v[q.trim_end_matches('\'')].clone()
        } else {
            v[k.parse::<usize>().expect("index")].clone()
        };
        if v.is_null() {
            panic!("missing value for {e}");
        }
        rest = &rest[end + 1..];
    }
    v
}

fn pair(ctx: &Map<String, Value>, e: &str) -> (i64, i64) {
    match path(ctx, e) {
        Value::Array(a) if a.len() == 2 => (a[0].as_i64().unwrap(), a[1].as_i64().unwrap()),
        _ => panic!("pct(*{e}) needs a pair"),
    }
}

fn eval(ctx: &Map<String, Value>, e: &str) -> String {
    let (e, spec) = match e.rsplit_once(':') {
        Some((a, s)) if s == "," => (a, true),
        _ => (e, false),
    };
    let e = e.trim();
    if let Some(args) = e.strip_prefix("pct(").and_then(|a| a.strip_suffix(')')) {
        let (k, n) = if let Some(p) = args.strip_prefix('*') {
            pair(ctx, p)
        } else {
            let (a, b) = args.split_once(", ").expect("pct(a, b)");
            (int(ctx, a), int(ctx, b))
        };
        return pct(k, n);
    }
    let v = if let Some((a, b)) = e.split_once(" - ") {
        json!(int(ctx, a) - int(ctx, b))
    } else {
        path(ctx, e)
    };
    match v {
        Value::Number(n) if n.is_i64() => {
            let i = n.as_i64().unwrap();
            if spec { thousands(i) } else { i.to_string() }
        }
        Value::Number(n) => pyfloat(n.as_f64().unwrap()),
        Value::String(s) => s,
        o => panic!("cannot print {o} for {e}"),
    }
}

fn int(ctx: &Map<String, Value>, e: &str) -> i64 {
    if let Some((a, b)) = e.split_once(" - ") {
        return int(ctx, a) - int(ctx, b);
    }
    path(ctx, e).as_i64().unwrap_or_else(|| panic!("{e} is not an int"))
}

fn thousands(i: i64) -> String {
    let s = i.abs().to_string();
    let mut o = String::new();
    for (j, c) in s.chars().enumerate() {
        if j > 0 && (s.len() - j) % 3 == 0 {
            o.push(',');
        }
        o.push(c);
    }
    if i < 0 { format!("-{o}") } else { o }
}

/// fill {expr} placeholders; {REFS} stays for number_citations
fn fill(tpl: &str, ctx: &Map<String, Value>) -> String {
    let mut out = String::with_capacity(tpl.len() * 2);
    let mut rest = tpl;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let end = rest[i..].find('}').expect("unclosed {") + i;
        let e = &rest[i + 1..end];
        if e == "REFS" {
            out.push_str("{REFS}");
        } else {
            out.push_str(&eval(ctx, e));
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

fn build(d: &Data, fonts: Option<&str>, version: &str, date: &str, doi: &str) -> String {
    let s = &d.s;
    let mut c = Map::new();
    let pair = |p: (i64, i64)| json!([p.0, p.1]);
    c.insert("s".into(), s.clone());
    c.insert("a6".into(), d.dev6["ALL"].clone());
    c.insert("a23".into(), d.dev23["ALL"].clone());
    c.insert("h2".into(), json!({"hla-wgs": pair(d.total("hla-wgs", "two_field")), "T1K": pair(d.total("T1K", "two_field"))}));
    c.insert("hc".into(), json!({"hla-wgs": pair(d.total("hla-wgs", "copy_number")), "T1K": pair(d.total("T1K", "copy_number"))}));
    c.insert("n_ho".into(), json!(150 + s["heldout_round4"]["samples"].as_i64().unwrap()));
    c.insert("lab".into(), s["lab"].clone());
    c.insert("tr".into(), s["trios"].clone());
    c.insert("ld".into(), s["lab"]["differences"].clone());
    c.insert("ix".into(), s["dev80_inexact_safe4"].clone());
    c.insert("tm".into(), s["timing"].clone());
    c.insert("d.dev6".into(), d.dev6.clone());
    c.insert("d.one".into(), json!(d.one));
    c.insert("d.many".into(), json!(d.many));
    let rel_txt = match s["timing"].get("release_1.0.0") {
        Some(_) => format!(" The released Rust program needs {} s and {} GB for the same kind of input.", eval(&c, "tm['release_1.0.0']['wall_s']"), eval(&c, "tm['release_1.0.0']['peak_gb']")),
        None => String::new(),
    };
    let st = |c: &mut Map<String, Value>, k: &str, v: String| {
        c.insert(k.into(), Value::String(v));
    };
    st(&mut c, "rel_txt", rel_txt);
    let (unseen_hl, unseen_sum, unseen, unseen_lim);
    if let Some((un6, un23)) = &d.un {
        c.insert("u6".into(), un6["ALL"].clone());
        c.insert("u23".into(), un23["ALL"].clone());
        let n_un = s["check30"]["samples"].as_i64().unwrap();
        c.insert("n_un".into(), json!(n_un));
        let (k, n) = (gi(un6, "ALL", "safe4_exact"), gi(un6, "ALL", "safe4"));
        unseen_hl = format!(
            "<li>On {n_un} further assembled genomes that played no part in developing the certainty rules, {k} of {n} alleles reported as certain at four fields were exact ({}).</li>",
            pct(k, n)
        );
        unseen_sum = format!(" On {n_un} further genomes that played no part in developing these rules the figure was {k} of {n} ({}).", pct(k, n));
        let mut cu = c.clone();
        st(&mut cu, "FIG4UN", fig_four(un6, &format!("Four-field statements per gene, {n_un} further samples")));
        st(&mut cu, "gene_table(d.un6, d.un23)", gene_table(un6, un23));
        unseen = fill(UNSEEN, &cu);
        unseen_lim = "<li>The 30 samples used to check the certainty rules were not untouched: they had been available during development of the two-field typer. A test on genomes never used at any stage is still missing.</li>".to_string();
    } else {
        unseen_hl = String::new();
        unseen_sum = String::new();
        unseen = String::new();
        unseen_lim = "<li>The four-field certainty rules were developed on the same 80 samples on which they are reported. An independent test on unseen assembled genomes is not part of this version.</li>".to_string();
    }
    st(&mut c, "unseen_hl", unseen_hl);
    st(&mut c, "unseen_sum", unseen_sum);
    st(&mut c, "unseen", unseen);
    st(&mut c, "unseen_lim", unseen_lim);
    st(&mut c, "title", TITLE.into());
    st(&mut c, "style(fonts)", style(fonts));
    st(&mut c, "LOGO", LOGO.into());
    st(&mut c, "esc(version)", esc(version));
    st(&mut c, "esc(date)", esc(date));
    st(&mut c, "esc(doi)", esc(doi));
    st(&mut c, "fig_workflow()", fig_workflow());
    st(&mut c, "fig_heldout(d)", fig_heldout(d));
    st(&mut c, "FIG3", fig_four(&d.dev6, "Four-field statements per gene, 80 HPRC samples"));
    st(&mut c, "gene_table(d.dev6, d.dev23)", gene_table(&d.dev6, &d.dev23));
    fill(MAIN, &c)
}

pub fn main(a: &[&str]) {
    let (mut fonts, mut version, mut date, mut doi) = (None, "1.0.0".to_string(), "19 November 2026".to_string(), "10.5281/zenodo.24367470".to_string());
    let mut i = 0;
    while i < a.len() {
        let v = a.get(i + 1).map(|s| s.to_string()).unwrap_or_else(|| panic!("{} needs a value", a[i]));
        match a[i] {
            "--fonts" => fonts = Some(v),
            "--version" => version = v,
            "--date" => date = v,
            "--doi" => doi = v,
            o => panic!("unknown option {o}"),
        }
        i += 2;
    }
    let res = std::env::var("HLA_RESULTS").unwrap_or_else(|_| format!("{}/validation/results", env!("CARGO_MANIFEST_DIR")));
    let page = number_citations(&build(&Data::new(&res), fonts.as_deref(), &version, &date, &doi));
    use std::io::Write;
    std::io::stdout().write_all(page.as_bytes()).unwrap();
}
