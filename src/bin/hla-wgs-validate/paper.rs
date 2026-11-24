//! paper: the software report as PDF, typeset with Typst inside the program, from the files under
//! validation/results, so that the text cannot drift from the data.
//!
//!     cargo build --release --features paper
//!     hla-wgs-validate paper [--version V] [--date D] [--doi DOI] [--typ OUT.typ] > docs/paper/hla-wgs-report.pdf
//!
//! The text lives in report/main.typ; «key» marks a value computed here. The figures are SVG drawn
//! here. Fonts: IBM Plex Mono from docs/paper/fonts, compiled in.

use crate::common::{pyfloat, read};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

const INK: &str = "#141413";
const SUB: &str = "#56544C";
const LINE: &str = "#D8D5CC";
const OK: &str = "#2A7350";
const BAD: &str = "#B3261E";
const GOLD: &str = "#a88600";
const BLUE: &str = "#2F5D8A";
const GREY: &str = "#9A978E";
const MONO: &str = "IBM Plex Mono";
const SANS: &str = "IBM Plex Mono";

const MAIN: &str = include_str!("report/main.typ");

const TITLE: &str = "hla-wgs: HLA typing from whole-genome sequencing that states how many fields of each allele are certain";

const GENES: [&str; 18] = ["A", "B", "C", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1", "DRA", "E", "F", "G", "H", "K", "V"];

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


/// text for SVG
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

const REFS: [(&str, &str); 15] = [
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
    ("hgsvc3", "Logsdon GA, Ebert P, Audano PA, et al. Complex genetic variation in nearly complete human genomes. Nature. 2025;644:430–441."),
    ("minimap2", "Li H. Minimap2: pairwise alignment for nucleotide sequences. Bioinformatics. 2018;34(18):3094–3100."),
];


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
    un6: Value,
    un23: Value,
    one: i64,
    many: i64,
    l18: Lab,
    l14: Lab,
    long: HashMap<String, (i64, i64)>,
    depth: HashMap<String, [(i64, i64); 2]>,
}

/// eval24 output: samples, samples with T1K, ours, ours on the T1K subset, T1K, differing sample-gene lines
#[derive(Default)]
struct Lab {
    samples: i64,
    t1k_samples: i64,
    ours: (i64, i64),
    sub: (i64, i64),
    t1k: (i64, i64),
}

/// "[9051, 9090]" -> (9051, 9090)
fn br(s: &str) -> (i64, i64) {
    let v: Vec<i64> = s.trim_matches(|c| c == '[' || c == ']').split(',').map(|x| x.trim().parse().unwrap()).collect();
    (v[0], v[1])
}

fn lab(path: &str) -> Lab {
    let mut l = Lab::default();
    for line in read(path).lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        if line.starts_with("samples ") {
            l.samples = w[1].parse().unwrap();
            l.t1k_samples = w[4].parse().unwrap();
        } else if line.starts_with("total ours ") {
            let t = line.trim_start_matches("total ours ");
            let (a, rest) = t.split_once(" T1K subset ours ").unwrap();
            let (b, c) = rest.split_once(" T1K ").unwrap();
            l.ours = br(a);
            l.sub = br(b);
            l.t1k = br(c);
        }
    }
    l
}

/// "name ... K / N" or "name ... K" lines of `hla-wgs-validate long`
fn long(path: &str) -> HashMap<String, (i64, i64)> {
    let mut m = HashMap::new();
    for line in read(path).lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        if line.starts_with("consensus ALL") {
            m.insert("cons".into(), (w[2].parse().unwrap(), w[4].parse().unwrap()));
        } else if line.starts_with("assembly haplotypes reproduced") {
            m.insert("hap".into(), (w[4].parse().unwrap(), w[6].parse().unwrap()));
        } else if line.starts_with("exact calls ALL") {
            m.insert("exact".into(), (w[3].parse().unwrap(), w[5].parse().unwrap()));
        } else if line.starts_with("nearest-allele calls") {
            m.insert("near".into(), (w[6].parse().unwrap(), w[8].parse().unwrap()));
        }
    }
    m
}

/// eval-res lines "DIR 2:k/n 3:k/n"
fn depth(path: &str) -> HashMap<String, [(i64, i64); 2]> {
    let mut m = HashMap::new();
    for line in read(path).lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.len() == 3 {
            m.insert(w[0].to_string(), [frac(&w[1][2..]), frac(&w[2][2..])]);
        }
    }
    m
}

impl Data {
    fn new(res: &str) -> Data {
        let s: Value = serde_json::from_str(&read(&format!("{res}/summary.json"))).expect("summary.json");
        let strata = rows(&format!("{res}/heldout_vs_t1k/strata.tsv"));
        let mut nd = vec![];
        for l in read(&format!("{res}/dev80/truth4.tsv")).lines() {
            let r: Vec<&str> = l.split('\t').collect();
            if r.len() >= 6 && r[2] != "_done" && !r[3].is_empty() && r[3].chars().all(|c| c.is_ascii_digit()) {
                let x: i64 = r[3].parse().unwrap();
                if x > 0 {
                    nd.push(x);
                }
            }
        }
        Data {
            dev6: gene_cols(&format!("{res}/dev80_main/eval6.tsv")),
            dev23: gene_cols(&format!("{res}/dev80_main/eval23.tsv")),
            un6: gene_cols(&format!("{res}/hgsvc59_main/eval6.tsv")),
            un23: gene_cols(&format!("{res}/hgsvc59_main/eval23.tsv")),
            one: nd.iter().filter(|&&x| x == 1).count() as i64,
            many: nd.iter().filter(|&&x| x >= 10).count() as i64,
            l18: lab(&format!("{res}/lab1000g/eval24_2018.txt")),
            l14: lab(&format!("{res}/lab1000g/eval24_2014.txt")),
            long: long(&format!("{res}/long26_main/long.txt")),
            depth: depth(&format!("{res}/depth_lead4/eval_res.txt")),
            s,
            strata,
        }
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
        ("certain 4F, exact", OK),
        ("certain 4F, not exact", BAD),
        ("not certain, exact", "#A9C7B8"),
        ("not certain, new or different", "#D9D6CC"),
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
        x = N::F(x.f() + (24.0 + 4.4 * label.chars().count() as f64));
    }
    svg(720, h, &b)
}


// ------------------------------------------------------------------ figure 1: how hla-wgs works
fn fig_workflow() -> String {
    let mut b: Vec<String> = vec![];
    let sub_t = T { anchor: "middle", fill: SUB, mono: true, weight: 400 };
    let title_t = T { anchor: "middle", fill: INK, mono: true, weight: 500 };
    let lane_t = T { anchor: "start", fill: GOLD, mono: true, weight: 500 };
    let bx = |b: &mut Vec<String>, x: i64, y: i64, title: &str, sub: &[&str], gold: bool| {
        let (bw, bh) = (126, 50);
        let stroke = if gold { GOLD } else { LINE };
        b.push(format!(r##"<rect x="{x}" y="{y}" width="{bw}" height="{bh}" rx="4" fill="#FFFFFF" stroke="{stroke}" stroke-width="{}"/>"##, if gold { 1.4 } else { 0.9 }));
        b.push(text(x as f64 + bw as f64 / 2.0, (y + 15) as f64, title, N::F(8.0), &title_t));
        for (i, line) in sub.iter().enumerate() {
            b.push(text(x as f64 + bw as f64 / 2.0, (y + 28 + 10 * i as i64) as f64, line, N::F(6.6), &sub_t));
        }
    };
    let arrow = |b: &mut Vec<String>, x1: i64, y1: i64, x2: i64, y2: i64| {
        b.push(format!(r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{SUB}" stroke-width="0.9"/>"#));
        let (dx, dy) = ((x2 - x1).signum(), (y2 - y1).signum());
        let (px, py) = (-dy, dx);
        b.push(format!(
            r#"<path d="M{x2},{y2} L{},{} L{},{} z" fill="{SUB}"/>"#,
            x2 - 6 * dx + 3 * px,
            y2 - 6 * dy + 3 * py,
            x2 - 6 * dx - 3 * px,
            y2 - 6 * dy - 3 * py
        ));
    };
    // lane 1: once per installation
    b.push(text(8.0, 14.0, "ONCE  reference from the database release", N::F(7.6), &lane_t));
    let once: [(&str, [&str; 2]); 4] = [
        ("hla.dat", ["IPD-IMGT/HLA release,", "downloaded by start file"]),
        ("build-db", ["alleles with exons,", "panel, nearest relatives"]),
        ("build-mask", ["21-mers repeated in", "GRCh38 outside the MHC"]),
        ("reference", ["hladb/ in the tool", "folder, never shipped"]),
    ];
    let mut x = 8;
    for (i, (t, s)) in once.iter().enumerate() {
        bx(&mut b, x, 22, t, s, i == 3);
        if i > 0 {
            arrow(&mut b, x - 18, 47, x - 1, 47);
        }
        x += 144;
    }
    // lane 2: every sample
    b.push(text(8.0, 98.0, "EVERY SAMPLE  short reads", N::F(7.6), &lane_t));
    let run: [(&str, [&str; 2]); 5] = [
        ("input", ["FASTQ, BAM or CRAM:", "MHC, ALT, unmapped"]),
        ("own aligner", ["reads to the panel,", "pairs, mate rescue"]),
        ("candidates", ["k-mer likelihood,", "EM per gene"]),
        ("best pair", ["edit matrix, exons,", "DRB1/3/4/5 jointly"]),
        ("certainty", ["lead per field,", "realign, new sites"]),
    ];
    let mut x = 8;
    for (i, (t, s)) in run.iter().enumerate() {
        bx(&mut b, x, 106, t, s, false);
        if i > 0 {
            arrow(&mut b, x - 18, 131, x - 1, 131);
        }
        x += 144;
    }
    // the reference is what the aligner maps to (and what every later step compares with)
    b.push(format!(r#"<polyline points="503,73 503,88 215,88" fill="none" stroke="{SUB}" stroke-width="0.9"/>"#));
    arrow(&mut b, 215, 88, 215, 105);
    // output of the short-read path: the certainty step writes all result files
    b.push(text(8.0, 184.0, "OUTPUT", N::F(7.6), &lane_t));
    b.push(format!(r#"<polyline points="647,157 647,172 71,172" fill="none" stroke="{SUB}" stroke-width="0.9"/>"#));
    let out: [(&str, [&str; 2]); 4] = [
        ("PREFIX.tsv", ["safe call and", "resolution per allele"]),
        ("conf.tsv / json", ["leads, groups,", "copy number, sites"]),
        ("novel.tsv", ["every new site with", "region and effect"]),
        ("PREFIX.html", ["result page, German", "and English, DOI"]),
    ];
    let mut x = 8;
    for (t, s) in out.iter() {
        bx(&mut b, x, 190, t, s, true);
        arrow(&mut b, x + 63, 172, x + 63, 189);
        x += 144;
    }
    // lane 3: long reads, a path of its own from its own input to its own output
    b.push(text(8.0, 266.0, "EVERY SAMPLE  HiFi reads (hla-wgs long)", N::F(7.6), &lane_t));
    let lr: [(&str, [&str; 2]); 5] = [
        ("HiFi input", ["PacBio HiFi reads,", "FASTQ or BAM"]),
        ("haplotypes", ["reads grouped", "per gene copy"]),
        ("consensus", ["plurality vote; groups", "of one allele joined"]),
        ("name", ["nearest allele,", "exons first"]),
        ("PREFIX.long.*", ["allele sequences", "and names"]),
    ];
    let mut x = 8;
    for (i, (t, s)) in lr.iter().enumerate() {
        bx(&mut b, x, 274, t, s, i == 4);
        if i > 0 {
            arrow(&mut b, x - 18, 299, x - 1, 299);
        }
        x += 144;
    }
    b.push(text(8.0, 342.0, "Research use only. A call must be confirmed by an accredited HLA laboratory before any medical use.", N::F(7.0), &T { fill: SUB, mono: true, ..T0 }));
    svg(720, 350, &b)
}

// ------------------------------------------------------------------ tables (Typst markup)
fn cell(s: &str) -> String {
    format!("[{s}]")
}

fn gene_table(e6: &Value, e23: &Value) -> String {
    let head = ["Gene", "Haplotypes", "Two fields", "Three fields", "Known exact", "New alleles", "Certain 4F", "of these exact"];
    let mut cells: Vec<String> = head.iter().map(|h| cell(&format!("*{h}*"))).collect();
    for g in GENES.iter().chain(["ALL"].iter()) {
        let name = if *g == "ALL" { "All genes".to_string() } else { format!("HLA-{g}") };
        let (r, t) = (|k| gi(e6, g, k), |k| gi(e23, g, k));
        for v in [
            name,
            r("n").to_string(),
            format!("{} / {}", t("ok2"), t("n")),
            format!("{} / {}", t("ok3"), t("n")),
            format!("{} / {}", r("known_ok"), r("known")),
            r("new").to_string(),
            r("safe4").to_string(),
            r("safe4_exact").to_string(),
        ] {
            cells.push(cell(&v));
        }
    }
    format!("table(columns: 8, align: (left, right, right, right, right, right, right, right), {})", cells.join(", "))
}

fn depth_table(d: &Data) -> String {
    let mut cells: Vec<String> = ["Depth", "Lead 6: two fields", "three fields", "Lead 4: two fields", "three fields"].iter().map(|h| cell(&format!("*{h}*"))).collect();
    for (lab, old, new) in [("30×", "newg_dr2", "newg_tg"), ("15×", "newg_l15", "newg_tg15"), ("10×", "newg_l10", "newg_tg10")] {
        let (o, n) = (d.depth[old], d.depth[new]);
        cells.push(cell(lab));
        for (k, m) in [o[0], o[1], n[0], n[1]] {
            cells.push(cell(&format!("{} / {}", thousands(k), thousands(m))));
        }
    }
    format!("table(columns: 5, align: (left, right, right, right, right), {})", cells.join(", "))
}

// ------------------------------------------------------------------ text values

fn thousands(i: i64) -> String {
    let s = i.abs().to_string();
    let mut o = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k) % 3 == 0 {
            o.push(',');
        }
        o.push(c);
    }
    if i < 0 { format!("-{o}") } else { o }
}

fn pct(k: i64, n: i64) -> String {
    format!("{:.1} %", 100.0 * k as f64 / n as f64)
}

/// [@a, @b] -> [1, 2] in order of first citation; the numbered list replaces «refs»
fn cite(page: &str) -> String {
    let mut order: Vec<String> = vec![];
    let mut out = String::new();
    let mut rest = page;
    while let Some(i) = rest.find("[@") {
        let j = rest[i..].find(']').expect("unclosed citation") + i;
        out.push_str(&rest[..i]);
        let nums: Vec<String> = rest[i + 1..j]
            .split(',')
            .map(|k| {
                let k = k.trim().trim_start_matches('@').to_string();
                assert!(REFS.iter().any(|r| r.0 == k), "unknown reference {k}");
                if !order.contains(&k) {
                    order.push(k.clone());
                }
                (order.iter().position(|x| *x == k).unwrap() + 1).to_string()
            })
            .collect();
        out.push_str(&format!("[{}]", nums.join(", ")));
        rest = &rest[j + 1..];
    }
    out.push_str(rest);
    let list: Vec<String> = order.iter().map(|k| format!("+ {}", REFS.iter().find(|r| r.0 == k).unwrap().1.replace('*', "\\*"))).collect();
    out.replace("«refs»", &format!("#set text(size: 7.4pt)\n#set enum(numbering: \"1.\")\n{}", list.join("\n")))
}

fn fill(tpl: &str, v: &HashMap<&str, String>) -> String {
    let mut out = String::new();
    let mut rest = tpl;
    while let Some(i) = rest.find('«') {
        out.push_str(&rest[..i]);
        let j = rest[i..].find('»').expect("unclosed «") + i;
        let k = &rest[i + '«'.len_utf8()..j];
        if k == "refs" {
            out.push_str("«refs»");
        } else {
            out.push_str(v.get(k).unwrap_or_else(|| panic!("no value for «{k}»")));
        }
        rest = &rest[j + '»'.len_utf8()..];
    }
    out.push_str(rest);
    out
}

fn values(d: &Data, version: &str, date: &str, doi: &str) -> HashMap<&'static str, String> {
    let mut v: HashMap<&'static str, String> = HashMap::new();
    let s = &d.s;
    let n = |x: i64| thousands(x);
    v.insert("title", TITLE.into());
    v.insert("version", version.into());
    v.insert("date", date.into());
    v.insert("doi", doi.into());
    v.insert("db", s["database"].as_str().unwrap_or("").trim_start_matches("IPD-IMGT/HLA ").into());
    // held-out rounds against T1K
    let (h, t) = (d.total("hla-wgs", "two_field"), d.total("T1K", "two_field"));
    let (hc, tc) = (d.total("hla-wgs", "copy_number"), d.total("T1K", "copy_number"));
    v.insert("n_ho", (150 + s["heldout_round4"]["samples"].as_i64().unwrap()).to_string());
    v.insert("h2_ok", n(h.0));
    v.insert("h2_n", n(h.1));
    v.insert("h2_pct", pct(h.0, h.1));
    v.insert("h2_err", (h.1 - h.0).to_string());
    v.insert("h2_t1k", n(t.0));
    v.insert("h2_t1k_pct", pct(t.0, t.1));
    v.insert("h2_err_t1k", (t.1 - t.0).to_string());
    v.insert("hc_hla", hc.0.to_string());
    v.insert("hc_n", hc.1.to_string());
    v.insert("hc_t1k", tc.0.to_string());
    // 80 development samples, current version
    let a = |k| gi(&d.dev6, "ALL", k);
    let a23 = |k| gi(&d.dev23, "ALL", k);
    v.insert("a6_n", n(a("n")));
    v.insert("a6_known", n(a("known")));
    v.insert("a6_new", n(a("new")));
    v.insert("pct_new", pct(a("new"), a("n")));
    v.insert("a6_kok", n(a("known_ok")));
    v.insert("pct_kok", pct(a("known_ok"), a("known")));
    v.insert("a6_s4", n(a("safe4")));
    v.insert("a6_s4e", n(a("safe4_exact")));
    v.insert("pct_s4", pct(a("safe4_exact"), a("safe4")));
    v.insert("a6_det", n(a("site_det")));
    v.insert("a6_det_true", n(a("site_det_true")));
    v.insert("a23_ok2", n(a23("ok2")));
    v.insert("a23_ok3", n(a23("ok3")));
    v.insert("a23_n", n(a23("n")));
    v.insert("drb1_new", gi(&d.dev6, "DRB1", "new").to_string());
    v.insert("drb1_n", gi(&d.dev6, "DRB1", "n").to_string());
    v.insert("a_new", gi(&d.dev6, "A", "new").to_string());
    v.insert("c_new", gi(&d.dev6, "C", "new").to_string());
    v.insert("one_diff", d.one.to_string());
    v.insert("many_diff", d.many.to_string());
    // 59 unseen HGSVC samples
    let u = |k| gi(&d.un6, "ALL", k);
    let u23 = |k| gi(&d.un23, "ALL", k);
    v.insert("u6_s4", n(u("safe4")));
    v.insert("u6_s4e", n(u("safe4_exact")));
    v.insert("pct_u4", pct(u("safe4_exact"), u("safe4")));
    v.insert("u6_det", n(u("site_det")));
    v.insert("u6_det_true", n(u("site_det_true")));
    v.insert("u23_ok2", n(u23("ok2")));
    v.insert("u23_ok3", n(u23("ok3")));
    v.insert("u23_n", n(u23("n")));
    // laboratory typing
    let (l, l4) = (&d.l18, &d.l14);
    let lx = &s["lab1000g"];
    v.insert("l18_n_samples", n(l.samples));
    v.insert("l18_ok", n(l.ours.0));
    v.insert("l18_n", n(l.ours.1));
    v.insert("l18_pct", pct(l.ours.0, l.ours.1));
    v.insert("l18_t1k_samples", l.t1k_samples.to_string());
    v.insert("l18_sub", format!("{} of {}", l.sub.0, l.sub.1));
    v.insert("l18_t1k", format!("{} of {}", l.t1k.0, l.t1k.1));
    v.insert("l18_diff", (l.ours.1 - l.ours.0).to_string());
    v.insert("l18_ard", lx["identical_in_typed_exons"].to_string());
    v.insert("l18_real", lx["real"].to_string());
    v.insert("l14_n_samples", n(l4.samples));
    v.insert("l14_ok", n(l4.ours.0));
    v.insert("l14_n", n(l4.ours.1));
    let tr = &s["trios"];
    v.insert("tr_a3", tr["ashkenazi"]["consistent_3"].to_string());
    v.insert("tr_an", tr["ashkenazi"]["genes_present"].to_string());
    v.insert("tr_h3", tr["han"]["consistent_3"].to_string());
    v.insert("tr_hn", tr["han"]["genes_present"].to_string());
    // long reads
    let lr = |k: &str| d.long[k];
    v.insert("lr_ok", n(lr("cons").0));
    v.insert("lr_n", n(lr("cons").1));
    v.insert("lr_hap", n(lr("hap").0));
    v.insert("lr_ex_ok", lr("exact").0.to_string());
    v.insert("lr_ex_n", lr("exact").1.to_string());
    v.insert("lr_near_ok", lr("near").0.to_string());
    v.insert("lr_near_n", lr("near").1.to_string());
    // run time
    let tm = &s["timing_main"];
    let f = |x: &Value| x.as_f64().map(|y| format!("{y:.0}")).unwrap_or_else(|| "?".into());
    v.insert("t_hla_wall", f(&tm["hla-wgs"]["wall_s"]));
    v.insert("t_hla_cpu", f(&tm["hla-wgs"]["cpu_s"]));
    v.insert("t_t1k_wall", f(&tm["T1K_1.0.10"]["wall_s"]));
    v.insert("t_t1k_cpu", f(&tm["T1K_1.0.10"]["cpu_s"]));
    v.insert("t_n", tm["samples"].to_string());
    v.insert("table_dev", gene_table(&d.dev6, &d.dev23));
    v.insert("table_depth", depth_table(d));
    v
}

pub fn main(a: &[&str]) {
    let (mut version, mut date, mut doi, mut typ) = (env!("CARGO_PKG_VERSION").to_string(), "24 November 2026".to_string(), "10.5281/zenodo.24367469".to_string(), None);
    let mut i = 0;
    while i < a.len() {
        let v = a.get(i + 1).map(|s| s.to_string()).unwrap_or_else(|| panic!("{} needs a value", a[i]));
        match a[i] {
            "--version" => version = v,
            "--date" => date = v,
            "--doi" => doi = v,
            "--typ" => typ = Some(v),
            o => panic!("unknown option {o}"),
        }
        i += 2;
    }
    let res = std::env::var("HLA_RESULTS").unwrap_or_else(|_| format!("{}/validation/results", env!("CARGO_MANIFEST_DIR")));
    let d = Data::new(&res);
    let src = cite(&fill(MAIN, &values(&d, &version, &date, &doi)));
    if let Some(p) = typ {
        std::fs::write(&p, &src).expect("write .typ");
    }
    let figs = vec![
        ("fig1.svg".to_string(), fig_workflow().into_bytes()),
        ("fig2.svg".to_string(), fig_heldout(&d).into_bytes()),
        ("fig3.svg".to_string(), fig_four(&d.dev6, "Four-field statements per gene, 80 HPRC development samples").into_bytes()),
        ("fig4.svg".to_string(), fig_four(&d.un6, "Four-field statements per gene, 59 unseen HGSVC samples").into_bytes()),
    ];
    let pdf = pdf_bytes(&src, &figs);
    use std::io::Write;
    std::io::stdout().write_all(&pdf).unwrap();
}

#[cfg(feature = "paper")]
fn pdf_bytes(src: &str, figs: &[(String, Vec<u8>)]) -> Vec<u8> {
    crate::typ::pdf(src, figs, (2026, 11, 24))
}

#[cfg(not(feature = "paper"))]
fn pdf_bytes(_: &str, _: &[(String, Vec<u8>)]) -> Vec<u8> {
    eprintln!("the report needs the Typst feature: cargo build --release --features paper");
    std::process::exit(2)
}
