//! `PREFIX.report.json`: the report part of a typing run (schema `genome.report.v1`). It holds the
//! presentation of the result, derived only from `PREFIX.json` (and, when it is there, the header of
//! the run log): head, key findings, result tables and a chart as data, run quality, method,
//! validation, limits and references. Texts are message keys with arguments; the messages in German
//! and English (Fluent syntax) travel in the part, so either language can be set without a new run.
//! The overall report of Genome assembles such parts and computes nothing itself.
use serde_json::{Map, Value, json};

pub const SCHEMA: &str = "genome.report.v1";

/// validation of this version on assembly truth; a test compares the numbers with
/// validation/results/<id>/eval6.tsv and eval23.tsv
pub struct Val {
    pub id: &'static str,
    pub samples: u64,
    pub alleles: u64,
    pub two: u64,
    pub three: u64,
    pub safe4: u64,
    pub safe4_exact: u64,
}
pub const VALIDATION: [Val; 2] = [
    Val { id: "dev80_main", samples: 80, alleles: 2489, two: 2486, three: 2482, safe4: 1799, safe4_exact: 1780 },
    Val { id: "hgsvc59_main", samples: 59, alleles: 1833, two: 1833, three: 1830, safe4: 1350, safe4_exact: 1334 },
];

/// depth below which fewer fields are supported (the result page says the same)
const DEPTH_MIN: f64 = 18.0;

/// (key, German, English); `{ $name }` is an argument
const MSG: &[(&str, &str, &str)] = &[
    ("title", "HLA-Typisierung", "HLA typing"),
    ("ruo", "Nur für Forschung (RUO). Kein Medizinprodukt. Vor jeder medizinischen Entscheidung ist eine Bestätigung durch ein akkreditiertes HLA-Labor nötig.", "Research use only (RUO). Not a medical device. Any medical decision needs confirmation by an accredited HLA laboratory."),
    ("sec-findings", "Kernaussagen", "Key findings"),
    ("sec-results", "Ergebnisse", "Results"),
    ("sec-quality", "Qualität des Laufs", "Run quality"),
    ("sec-method", "Methode", "Method"),
    ("sec-validation", "Validierungsstand", "Validation"),
    ("sec-limits", "Grenzen", "Limits"),
    ("sec-references", "Literatur", "References"),
    ("hla-genotype", "HLA-{ $gene }: { $a1 } und { $a2 }", "HLA-{ $gene }: { $a1 } and { $a2 }"),
    ("hla-genotype-one", "HLA-{ $gene }: { $a1 }, eine Kopie", "HLA-{ $gene }: { $a1 }, one copy"),
    ("hla-gene-absent", "HLA-{ $gene }: nicht vorhanden", "HLA-{ $gene }: absent"),
    ("hla-gene-not-typed", "HLA-{ $gene }: nicht bestimmt ({ $reason })", "HLA-{ $gene }: not determined ({ $reason })"),
    ("reason-few-reads", "zu wenige Reads", "too few reads"),
    ("reason-no-call", "kein Allelpaar", "no allele pair"),
    ("certainty-fields", "von den Reads gestützt: { $fields } von 4 Feldern", "supported by the reads: { $fields } of 4 fields"),
    ("alternative-next", "{ $allele }: nächstes Allel im Feld { $field } ist { $alternative } (Vorsprung { $margin })", "{ $allele }: nearest allele at field { $field } is { $alternative } (lead { $margin })"),
    ("hla-new-allele", "{ $allele }: mögliches neues Allel (Abweichungen: { $sites }, davon mit Folge für das Protein: { $coding })", "{ $allele }: possible new allele (differences: { $sites }, of them changing the protein: { $coding })"),
    ("pgx-carrier", "Träger von HLA-{ $allele } (Kopien: { $copies }): betrifft { $drugs }", "Carrier of HLA-{ $allele } (copies: { $copies }): concerns { $drugs }"),
    ("pgx-not-carrier", "Kein Träger von HLA-{ $allele } (betrifft { $drugs })", "Not a carrier of HLA-{ $allele } (concerns { $drugs })"),
    ("pgx-unclear", "HLA-{ $allele }: nicht sicher zu entscheiden (betrifft { $drugs })", "HLA-{ $allele }: cannot be decided with certainty (concerns { $drugs })"),
    ("col-gene", "Gen", "Gene"),
    ("col-slot", "Kopie", "Copy"),
    ("col-allele", "Nächstes Allel", "Nearest allele"),
    ("col-safe-call", "Gesicherter Name", "Supported name"),
    ("col-resolution", "Felder", "Fields"),
    ("col-g-group", "G-Gruppe", "G group"),
    ("col-p-group", "P-Gruppe", "P group"),
    ("col-serology", "Serologie", "Serology"),
    ("col-ligand", "Ligand", "Ligand"),
    ("col-new-sites", "Neue Stellen", "New sites"),
    ("col-called", "Bestimmt", "Typed"),
    ("col-copies", "Kopien", "Copies"),
    ("col-reads", "Read-Paare", "Read pairs"),
    ("col-copy-ratio", "Kopienverhältnis", "Copy ratio"),
    ("table-alleles", "Alle Allele", "All alleles"),
    ("table-genes", "Gene", "Genes"),
    ("gl-string", "GL-String (gestützte Felder)", "GL string (supported fields)"),
    ("chart-resolution", "Allele nach gestützten Feldern", "Alleles by supported fields"),
    ("chart-alleles", "Allele", "Alleles"),
    ("qc-depth", "Geschätzte Tiefe über den HLA-Genen", "Estimated depth over the HLA genes"),
    ("qc-depth-req", "mindestens { $min }x für die volle Auflösung", "at least { $min }x for the full resolution"),
    ("qc-mask", "Wiederholungsmaske verwendet", "Repeat mask used"),
    ("qc-mask-req", "nötig, damit Reads aus anderen Teilen des Genoms nicht mitzählen", "needed so that reads from elsewhere in the genome do not count"),
    ("qc-release", "IPD-IMGT/HLA-Version", "IPD-IMGT/HLA release"),
    ("qc-release-req", "Version der Datenbank bekannt", "release of the database known"),
    ("qc-groups", "G- und P-Gruppen geladen", "G and P groups loaded"),
    ("qc-serology", "Serologie geladen", "Serology loaded"),
    ("qc-optional", "optional", "optional"),
    ("qc-typed", "Klinische Loci bestimmt", "Clinical loci typed"),
    ("qc-typed-req", "jeder vorhandene Locus bestimmt", "every present locus typed"),
    ("qc-pairs", "Read-Paare gelesen", "Read pairs read"),
    ("qc-assigned", "Read-Paare einem Gen zugeordnet", "Read pairs assigned to a gene"),
    ("qc-info", "zur Information", "for information"),
    ("method-text", "Reads aus der Gesamtgenom-Sequenzierung werden mit dem eigenen Aligner gegen alle Allele der IPD-IMGT/HLA-Datenbank verglichen. Für jedes Gen wählt das Programm das Allelpaar, das die Reads am besten erklärt, und gibt je Allel nur die Felder an, bei denen der Vorsprung vor dem nächsten Konkurrenten eine feste, an Assemblies geprüfte Schwelle erreicht. Abweichungen, die die Reads einheitlich zeigen, werden als mögliches neues Allel gemeldet.", "Reads from whole-genome sequencing are compared with all alleles of the IPD-IMGT/HLA database by the program's own aligner. For each gene the program chooses the allele pair that explains the reads best and reports, per allele, only the fields where the lead over the nearest competitor reaches a fixed threshold validated on assemblies. Differences the reads show consistently are reported as a possible new allele."),
    ("method-paper", "Forschungsbericht und Software: { $doi }", "Research report and software: { $doi }"),
    ("val-dev80", "{ $samples } Genome mit assemblierten Haplotypen (HPRC, Entwicklung): zwei Felder { $two } von { $alleles } Allelen richtig, drei Felder { $three }; gesicherte vier Felder { $safe4_exact } von { $safe4 } exakt", "{ $samples } genomes with assembled haplotypes (HPRC, development): two fields right for { $two } of { $alleles } alleles, three fields { $three }; supported four fields exact for { $safe4_exact } of { $safe4 }"),
    ("val-hgsvc59", "{ $samples } weitere, nie verwendete Genome (HGSVC3): zwei Felder { $two } von { $alleles } richtig, drei Felder { $three }; gesicherte vier Felder { $safe4_exact } von { $safe4 } exakt", "{ $samples } further genomes never used (HGSVC3): two fields right for { $two } of { $alleles }, three fields { $three }; supported four fields exact for { $safe4_exact } of { $safe4 }"),
    ("limit-phase", "Welche Allele verschiedener Gene auf demselben Chromosom liegen (Haplotyp), lässt sich mit kurzen Reads nicht bestimmen.", "Which alleles of different genes lie on the same chromosome (haplotype) cannot be determined with short reads."),
    ("limit-repeats", "Die Länge langer Einzelbasen-Läufe und Wiederholungen ist nicht messbar; Strecken ohne eindeutig zuordenbare Reads bleiben ungeprüft.", "The length of long single-base runs and repeats cannot be measured; stretches without uniquely placed reads stay unchecked."),
    ("limit-expression", "Ob ein Allel exprimiert wird, folgt nur aus seinem Namen in der Datenbank.", "Whether an allele is expressed follows only from its name in the database."),
    ("limit-new", "Neue Allele sollten mit langen Reads oder Sanger-Sequenzierung bestätigt werden.", "New alleles should be confirmed with long reads or Sanger sequencing."),
    ("limit-low-depth", "Die geschätzte Tiefe liegt mit { $depth }x unter { $min }x: weniger Felder sind gestützt, das vierte Feld ist etwas seltener exakt.", "The estimated depth of { $depth }x is below { $min }x: fewer fields are supported and the fourth field is somewhat less often exact."),
    ("limit-not-typed", "Nicht bestimmt in dieser Probe: { $genes }.", "Not determined in this sample: { $genes }."),
    ("limit-no-mask", "Ohne Wiederholungsmaske können Reads aus anderen Teilen des Genoms mitgezählt worden sein.", "Without the repeat mask, reads from elsewhere in the genome may have been counted."),
];

fn tx(key: &str, args: Value) -> Value {
    if args.as_object().map_or(true, |m| m.is_empty()) { json!({"key": key}) } else { json!({"key": key, "args": args}) }
}

/// the recommendation of a pharmacogenetic allele as message key, from the same text as the page
fn pgx_key(allele: &str) -> String {
    format!("pgx-rec-{}", allele.replace(['*', ':'], ""))
}

use crate::groups::PGX;

fn drugs_key(allele: &str) -> String {
    format!("pgx-drugs-{}", allele.replace(['*', ':'], ""))
}

/// all messages of one language in Fluent syntax
pub fn ftl(de: bool) -> String {
    let mut out = String::new();
    for (k, d, e) in MSG {
        out.push_str(&format!("{k} = {}\n", if de { d } else { e }));
    }
    let t = |d: &'static str, e: &'static str| if de { d } else { e };
    for (a, drugs) in PGX {
        out.push_str(&format!("{} = {}\n", pgx_key(a), crate::html::pgx_rec(a, &t).0));
        out.push_str(&format!("{} = {}\n", drugs_key(a), crate::html::drugs_text(drugs, &t)));
    }
    out
}

/// a message with its arguments, from the Fluent text of the part (one line per message)
pub fn resolve(ftl: &str, t: &Value) -> String {
    let key = t["key"].as_str().unwrap_or("");
    let Some(mut s) = ftl.lines().find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(" = ")).map(|x| x.to_string())) else {
        return format!("[{key}]");
    };
    if let Some(args) = t["args"].as_object() {
        for (k, v) in args {
            let v = if v.is_object() { resolve(ftl, v) } else { v.as_str().map(|x| x.to_string()).unwrap_or_else(|| v.to_string()) };
            s = s.replace(&format!("{{ ${k} }}"), &v);
        }
    }
    s
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn crc(bytes: &[u8]) -> String {
    let mut c = flate2::Crc::new();
    c.update(bytes);
    format!("{:08x}", c.sum())
}

/// the header of the run log next to the result, when there is one: (header, last time stamp)
fn log_info(prefix: &str) -> (Option<Value>, Option<f64>, Option<Value>) {
    let Ok(t) = crate::vfs::read_to_string(&format!("{prefix}.log.jsonl")) else { return (None, None, None) };
    let ev: Vec<Value> = t.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    let head = ev.iter().find(|e| e["kind"] == "header").cloned();
    let end = ev.iter().filter_map(|e| e["t"].as_f64()).fold(None, |a: Option<f64>, x| Some(a.map_or(x, |a| a.max(x))));
    let map = ev.iter().find(|e| e["kind"] == "end" && e["step"] == "read_and_map").map(|e| e["counts"].clone());
    (head, end, map)
}

/// the report part from the result `PREFIX.json` (read from `prefix`)
pub fn build(prefix: &str) -> Result<Value, String> {
    let raw = crate::vfs::read_to_string(&format!("{prefix}.json")).map(String::into_bytes).map_err(|e| format!("{prefix}.json: {e}"))?;
    let rep: Value = serde_json::from_slice(&raw).map_err(|e| format!("{prefix}.json: {e}"))?;
    if rep["schema"] != "genome.hla.v1" {
        return Err(format!("{prefix}.json is not a hla-wgs result"));
    }
    let empty = vec![];
    let genes = rep["genes"].as_array().unwrap_or(&empty);
    let sample = std::path::Path::new(prefix).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (head_log, runtime, mapped) = log_info(prefix);

    // head
    let date = rep["analysis_time"].as_u64().map(|t| {
        let (y, m, d) = crate::html::date(t);
        format!("{y:04}-{m:02}-{d:02}")
    });
    let inputs: Value = head_log.as_ref().map(|h| h["inputs"].clone()).filter(|v| v.is_array()).unwrap_or_else(|| json!(rep["input"].as_array().unwrap_or(&empty).iter().map(|n| json!({"name": n})).collect::<Vec<_>>()));
    let head = json!({
        "title": tx("title", json!({})),
        "tool": "hla-wgs",
        "version": rep["version"],
        "commit": head_log.as_ref().map(|h| h["commit"].clone()).unwrap_or(json!(option_env!("HLA_WGS_COMMIT"))),
        "sample": sample,
        "inputs": inputs,
        "data": [{"name": "IPD-IMGT/HLA", "version": rep["database_release"]}],
        "date": date,
        "runtime_s": runtime.map(|t| (t * 10.0).round() / 10.0),
        "result": {"file": format!("{sample}.json"), "schema": rep["schema"], "crc32": crc(&raw)},
        "notice": tx("ruo", json!({})),
    });

    // key findings: the clinical loci, the pharmacogenetic alleles, new alleles
    let mut findings = vec![];
    let mut not_typed = vec![];
    for (gi, g) in genes.iter().enumerate() {
        let name = s(&g["gene"]);
        if !crate::html::CLINICAL.contains(&name.as_str()) {
            continue;
        }
        let src = json!({"file": format!("{sample}.json"), "path": format!("genes[{gi}]")});
        let copies = g["copies"].as_u64().unwrap_or(2);
        if !g["called"].as_bool().unwrap_or(false) {
            let reason = if g["reads"].as_u64().unwrap_or(0) < 20 { "reason-few-reads" } else { "reason-no-call" };
            // DRB3/4/5 without reads are absent, not untyped
            if matches!(name.as_str(), "DRB3" | "DRB4" | "DRB5") && reason == "reason-few-reads" {
                continue;
            }
            not_typed.push(name.clone());
            findings.push(json!({"id": format!("hla-{name}"), "text": tx("hla-gene-not-typed", json!({"gene": name, "reason": tx(reason, json!({}))})), "certainty": {"level": "none"}, "source": src}));
            continue;
        }
        if copies == 0 {
            if matches!(name.as_str(), "DRB3" | "DRB4" | "DRB5") {
                continue;
            }
            findings.push(json!({"id": format!("hla-{name}"), "text": tx("hla-gene-absent", json!({"gene": name})), "certainty": {"level": "certain"}, "source": src}));
            continue;
        }
        let als: Vec<&Value> = g["alleles"].as_array().unwrap_or(&empty).iter().take(copies.min(2) as usize).collect();
        let res: Vec<u64> = als.iter().map(|a| a["resolution"].as_u64().unwrap_or(0)).collect();
        let text = if als.len() == 1 {
            tx("hla-genotype-one", json!({"gene": name, "a1": s(&als[0]["safe_call"])}))
        } else {
            tx("hla-genotype", json!({"gene": name, "a1": s(&als[0]["safe_call"]), "a2": s(&als.get(1).map_or(json!(""), |a| a["safe_call"].clone()))}))
        };
        let alternatives: Vec<Value> = als.iter().zip(&res).filter(|(_, r)| **r < 4).filter_map(|(a, r)| {
            let f = (r + 1).max(2).to_string();
            let alt = a["alternative"][f.as_str()].as_str()?;
            let m = a["margin"][f.as_str()].as_f64().map(|m| format!("{m:.1}")).unwrap_or_else(|| "-".into());
            Some(json!({"allele": s(&a["safe_call"]), "field": f, "alternative": alt, "margin": a["margin"][f.as_str()], "text": tx("alternative-next", json!({"allele": s(&a["safe_call"]), "field": f, "alternative": alt, "margin": m}))}))
        }).collect();
        let level = if res.iter().all(|r| *r >= 2) { "certain" } else if res.iter().any(|r| *r >= 1) { "partial" } else { "none" };
        findings.push(json!({
            "id": format!("hla-{name}"), "text": text,
            "certainty": {"level": level, "fields": res, "of": 4, "text": tx("certainty-fields", json!({"fields": res.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(" / ")}))},
            "alternatives": alternatives, "source": src,
        }));
    }
    for (pi, p) in rep["pharmacogenetic_alleles"].as_array().unwrap_or(&empty).iter().enumerate() {
        let allele = s(&p["allele"]);
        let copies = p["copies"].as_u64().unwrap_or(0);
        let certain = p["certain"].as_bool().unwrap_or(false);
        // the drug names in the language of the report when the allele is known, else as in the result
        let drugs = if PGX.iter().any(|(a, d)| *a == allele && *d == p["drugs"]) { tx(&drugs_key(&allele), json!({})) } else { p["drugs"].clone() };
        let args = json!({"allele": allele, "drugs": drugs, "copies": copies});
        let key = if !certain { "pgx-unclear" } else if copies > 0 { "pgx-carrier" } else { "pgx-not-carrier" };
        let (_, guideline, url) = crate::html::pgx_rec(&allele, &|_, e| e);
        let mut f = json!({"id": format!("pgx-{}", allele.replace(['*', ':'], "")), "text": tx(key, args), "certainty": {"level": if certain { "certain" } else { "partial" }},
            "source": {"file": format!("{sample}.json"), "path": format!("pharmacogenetic_alleles[{pi}]")}});
        if copies > 0 {
            f["meaning"] = json!({"text": tx(&pgx_key(&allele), json!({})), "guideline": {"name": guideline, "url": url}});
        }
        findings.push(f);
    }
    for (gi, g) in genes.iter().enumerate() {
        for (ai, a) in g["alleles"].as_array().unwrap_or(&empty).iter().enumerate() {
            let sites: Vec<&Value> = a["new_sites"].as_array().map(|v| v.iter().filter(|x| !s(&x["change"]).starts_with("blind") && !x["either_allele"].as_bool().unwrap_or(false)).collect()).unwrap_or_default();
            if sites.is_empty() {
                continue;
            }
            let coding = sites.iter().filter(|x| { let e = s(&x["effect"]); !(e.is_empty() || e == "-" || e == "non-coding" || e.starts_with("synonymous")) }).count();
            findings.push(json!({"id": format!("new-{}-{}", s(&g["gene"]), ai + 1), "text": tx("hla-new-allele", json!({"allele": s(&a["allele"]), "sites": sites.len(), "coding": coding})),
                "certainty": {"level": "partial"}, "source": {"file": format!("{sample}.json"), "path": format!("genes[{gi}].alleles[{ai}].new_sites")}}));
        }
    }

    // results: tables and a chart as data
    let mut rows = vec![];
    let mut grows = vec![];
    let mut by_res = [0u64; 5];
    for g in genes {
        grows.push(json!([g["gene"], g["called"], g["copies"], g["reads"], g.get("copy_ratio").cloned().unwrap_or(Value::Null)]));
        for (ai, a) in g["alleles"].as_array().unwrap_or(&empty).iter().enumerate() {
            let r = a["resolution"].as_u64().unwrap_or(0).min(4);
            by_res[r as usize] += 1;
            rows.push(json!([g["gene"], ai + 1, a["allele"], a["safe_call"], r, a["g_group"], a["p_group"], a["serology"], a["ligand"], a["new_sites"].as_array().map(|v| v.len())]));
        }
    }
    let col = |ks: &[&str]| ks.iter().map(|k| tx(k, json!({}))).collect::<Vec<_>>();
    let results = json!([
        {"id": "gl-string", "kind": "value", "title": tx("gl-string", json!({})), "value": crate::html::gl_string(genes)},
        {"id": "alleles", "kind": "table", "title": tx("table-alleles", json!({})), "columns": col(&["col-gene", "col-slot", "col-allele", "col-safe-call", "col-resolution", "col-g-group", "col-p-group", "col-serology", "col-ligand", "col-new-sites"]), "rows": rows},
        {"id": "genes", "kind": "table", "title": tx("table-genes", json!({})), "columns": col(&["col-gene", "col-called", "col-copies", "col-reads", "col-copy-ratio"]), "rows": grows},
        {"id": "resolution", "kind": "chart", "title": tx("chart-resolution", json!({})), "chart": {"type": "bar", "x": {"label": tx("col-resolution", json!({})), "values": [0, 1, 2, 3, 4]}, "y": {"label": tx("chart-alleles", json!({})), "values": by_res}}},
    ]);

    // run quality
    let depth = rep["depth_estimate"].as_f64();
    let mask = rep["repeat_mask"].as_bool().unwrap_or(false);
    let rel = rep["database_release"].as_str().is_some();
    let mut quality = vec![
        json!({"id": "depth", "label": tx("qc-depth", json!({})), "value": depth, "unit": "x", "ok": depth.map(|d| d >= DEPTH_MIN), "requirement": tx("qc-depth-req", json!({"min": DEPTH_MIN as u64}))}),
        json!({"id": "repeat_mask", "label": tx("qc-mask", json!({})), "value": mask, "ok": mask, "requirement": tx("qc-mask-req", json!({}))}),
        json!({"id": "database_release", "label": tx("qc-release", json!({})), "value": rep["database_release"], "ok": rel, "requirement": tx("qc-release-req", json!({}))}),
        json!({"id": "groups", "label": tx("qc-groups", json!({})), "value": rep["groups_loaded"], "ok": Value::Null, "requirement": tx("qc-optional", json!({}))}),
        json!({"id": "serology", "label": tx("qc-serology", json!({})), "value": rep["serology_loaded"], "ok": Value::Null, "requirement": tx("qc-optional", json!({}))}),
        json!({"id": "loci_typed", "label": tx("qc-typed", json!({})), "value": not_typed.is_empty(), "ok": not_typed.is_empty(), "requirement": tx("qc-typed-req", json!({}))}),
    ];
    if let Some(m) = &mapped {
        quality.push(json!({"id": "pairs_in", "label": tx("qc-pairs", json!({})), "value": m["pairs_in"], "ok": Value::Null, "requirement": tx("qc-info", json!({}))}));
        quality.push(json!({"id": "pairs_assigned", "label": tx("qc-assigned", json!({})), "value": m["assigned_pairs"], "ok": Value::Null, "requirement": tx("qc-info", json!({}))}));
    }
    let all_ok = quality.iter().all(|q| q["ok"].as_bool() != Some(false));

    let validation: Vec<Value> = VALIDATION.iter().map(|v| json!({"id": v.id, "text": tx(if v.id.starts_with("dev80") { "val-dev80" } else { "val-hgsvc59" }, json!({"samples": v.samples, "alleles": v.alleles, "two": v.two, "three": v.three, "safe4": v.safe4, "safe4_exact": v.safe4_exact})), "source": format!("validation/results/{}", v.id)})).collect();

    let mut limits: Vec<Value> = ["limit-phase", "limit-repeats", "limit-expression", "limit-new"].iter().map(|k| json!({"text": tx(k, json!({})), "sample": false})).collect();
    if let Some(d) = depth.filter(|d| *d < DEPTH_MIN) {
        limits.push(json!({"text": tx("limit-low-depth", json!({"depth": format!("{d:.0}"), "min": DEPTH_MIN as u64})), "sample": true}));
    }
    if !not_typed.is_empty() {
        limits.push(json!({"text": tx("limit-not-typed", json!({"genes": not_typed.join(", ")})), "sample": true}));
    }
    if !mask {
        limits.push(json!({"text": tx("limit-no-mask", json!({})), "sample": true}));
    }

    let mut refs = vec![
        json!({"id": "hla-wgs", "apa": format!("Jungwirth, P. (2026). hla-wgs: HLA typing from whole-genome sequencing with a per-allele statement of what is certain (Version {}) [Computer software]. Zenodo.", s(&rep["version"])), "doi": crate::html::DOI}),
        json!({"id": "imgt", "apa": "Barker, D. J., Maccari, G., Georgiou, X., Cooper, M. A., Flicek, P., Robinson, J., & Marsh, S. G. E. (2023). The IPD-IMGT/HLA Database. Nucleic Acids Research, 51(D1), D1053–D1060.", "doi": "10.1093/nar/gkac1011"}),
        json!({"id": "hprc", "apa": "Liao, W.-W., Asri, M., Ebler, J., et al. (2023). A draft human pangenome reference. Nature, 617, 312–324.", "doi": "10.1038/s41586-023-05896-x"}),
    ];
    let pg = rep["pharmacogenetic_alleles"].as_array().unwrap_or(&empty);
    let carries = |a: &str| pg.iter().any(|p| p["allele"] == a && p["copies"].as_u64().unwrap_or(0) > 0);
    if carries("B*57:01") {
        refs.push(json!({"id": "cpic-abacavir", "apa": "Martin, M. A., Hoffman, J. M., Freimuth, R. R., et al. (2014). Clinical Pharmacogenetics Implementation Consortium guidelines for HLA-B genotype and abacavir dosing: 2014 update. Clinical Pharmacology & Therapeutics, 95(5), 499–500.", "doi": "10.1038/clpt.2014.38"}));
    }
    if carries("B*15:02") || carries("A*31:01") {
        refs.push(json!({"id": "cpic-carbamazepine", "apa": "Phillips, E. J., Sukasem, C., Whirl-Carrillo, M., et al. (2018). Clinical Pharmacogenetics Implementation Consortium guideline for HLA genotype and use of carbamazepine and oxcarbazepine: 2017 update. Clinical Pharmacology & Therapeutics, 103(4), 574–581.", "url": "https://pmc.ncbi.nlm.nih.gov/articles/PMC5847474"}));
    }
    if carries("B*58:01") {
        refs.push(json!({"id": "cpic-allopurinol", "apa": "Saito, Y., Stamp, L. K., Caudle, K. E., et al. (2016). Clinical Pharmacogenetics Implementation Consortium (CPIC) guidelines for human leukocyte antigen B (HLA-B) genotype and allopurinol dosing: 2015 update. Clinical Pharmacology & Therapeutics, 99(1), 36–37.", "url": "https://pmc.ncbi.nlm.nih.gov/articles/PMC4675696"}));
    }

    let mut part = Map::new();
    part.insert("schema".into(), json!(SCHEMA));
    part.insert("head".into(), head);
    part.insert("findings".into(), json!(findings));
    part.insert("results".into(), results);
    part.insert("quality".into(), json!({"ok": all_ok, "items": quality}));
    part.insert("method".into(), json!({"text": tx("method-text", json!({})), "paper": {"text": tx("method-paper", json!({"doi": format!("https://doi.org/{}", crate::html::DOI)})), "doi": crate::html::DOI, "url": crate::html::ARTICLE}}));
    part.insert("validation".into(), json!(validation));
    part.insert("limits".into(), json!(limits));
    part.insert("references".into(), json!(refs));
    part.insert("messages".into(), json!({"de": ftl(true), "en": ftl(false)}));
    let body = serde_json::to_string(&Value::Object(part.clone())).unwrap();
    part.insert("checksum".into(), json!({"algorithm": "crc32", "scope": "the part as compact JSON without this field", "value": crc(body.as_bytes())}));
    Ok(Value::Object(part))
}

/// write `PREFIX.report.json`
pub fn write(prefix: &str) -> Result<String, String> {
    let part = build(prefix)?;
    let out = format!("{prefix}.report.json");
    crate::vfs::write(&out, (serde_json::to_string_pretty(&part).unwrap() + "\n").as_bytes()).map_err(|e| format!("{out}: {e}"))?;
    Ok(out)
}

/// the part as plain text in one language (what a report shows, without layout)
pub fn text(part: &Value, de: bool) -> String {
    let m = s(&part["messages"][if de { "de" } else { "en" }]);
    let r = |t: &Value| resolve(&m, t);
    let h = &part["head"];
    let mut o = format!("{} · {} · hla-wgs {} ({})\n{}\n", r(&h["title"]), s(&h["sample"]), s(&h["version"]), s(&h["commit"]), r(&h["notice"]));
    let empty = vec![];
    let list = |v: &Value| v.as_array().unwrap_or(&empty).clone();
    o.push_str(&format!("\n{}\n", r(&json!({"key": "sec-findings"}))));
    for f in list(&part["findings"]) {
        o.push_str(&format!("- {}", r(&f["text"])));
        if f["certainty"]["text"].is_object() {
            o.push_str(&format!(" [{}]", r(&f["certainty"]["text"])));
        }
        o.push('\n');
        for a in list(&f["alternatives"]) {
            o.push_str(&format!("    {}\n", r(&a["text"])));
        }
        if f["meaning"].is_object() {
            o.push_str(&format!("    {} ({}, {})\n", r(&f["meaning"]["text"]), s(&f["meaning"]["guideline"]["name"]), s(&f["meaning"]["guideline"]["url"])));
        }
    }
    o.push_str(&format!("\n{}\n", r(&json!({"key": "sec-results"}))));
    for x in list(&part["results"]) {
        match x["kind"].as_str() {
            Some("value") => o.push_str(&format!("{}: {}\n", r(&x["title"]), s(&x["value"]))),
            Some("table") => {
                o.push_str(&format!("{}\n  {}\n", r(&x["title"]), list(&x["columns"]).iter().map(|c| r(c)).collect::<Vec<_>>().join(" | ")));
                for row in list(&x["rows"]) {
                    o.push_str(&format!("  {}\n", list(&row).iter().map(|v| v.as_str().map(|s| s.to_string()).unwrap_or_else(|| if v.is_null() { "-".into() } else { v.to_string() })).collect::<Vec<_>>().join(" | ")));
                }
            }
            Some("chart") => o.push_str(&format!("{}: {}\n", r(&x["title"]), list(&x["chart"]["x"]["values"]).iter().zip(list(&x["chart"]["y"]["values"])).map(|(a, b)| format!("{a}: {b}")).collect::<Vec<_>>().join(", "))),
            _ => {}
        }
    }
    o.push_str(&format!("\n{}\n", r(&json!({"key": "sec-quality"}))));
    for q in list(&part["quality"]["items"]) {
        let mark = match q["ok"].as_bool() { Some(true) => "ok", Some(false) => "!!", None => "  " };
        o.push_str(&format!("{mark} {}: {} ({})\n", r(&q["label"]), if q["value"].is_null() { "-".into() } else { q["value"].as_str().map(|x| x.to_string()).unwrap_or_else(|| q["value"].to_string()) }, r(&q["requirement"])));
    }
    o.push_str(&format!("\n{}\n{}\n{}\n", r(&json!({"key": "sec-method"})), r(&part["method"]["text"]), r(&part["method"]["paper"]["text"])));
    o.push_str(&format!("\n{}\n", r(&json!({"key": "sec-validation"}))));
    for v in list(&part["validation"]) {
        o.push_str(&format!("- {}\n", r(&v["text"])));
    }
    o.push_str(&format!("\n{}\n", r(&json!({"key": "sec-limits"}))));
    for l in list(&part["limits"]) {
        o.push_str(&format!("- {}\n", r(&l["text"])));
    }
    o.push_str(&format!("\n{}\n", r(&json!({"key": "sec-references"}))));
    for x in list(&part["references"]) {
        let link = x["doi"].as_str().map(|d| format!("https://doi.org/{d}")).unwrap_or_else(|| s(&x["url"]));
        o.push_str(&format!("- {} {link}\n", s(&x["apa"])));
    }
    o
}

/// `hla-wgs report PREFIX.json [--text de|en]`
pub fn run(a: &[String]) {
    let Some(j) = a.first() else { crate::errors::fail("HW-E001", "usage: hla-wgs report PREFIX.json [--text de|en]") };
    let prefix = j.strip_suffix(".json").unwrap_or(j);
    if !std::path::Path::new(&format!("{prefix}.json")).is_file() {
        crate::errors::fail("HW-E021", &format!("{prefix}.json not found"));
    }
    let out = write(prefix).unwrap_or_else(|e| if e.contains("not a hla-wgs") { crate::errors::fail("HW-E021", &e) } else { crate::errors::fail("HW-E020", &e) });
    match a.iter().position(|x| x == "--text") {
        Some(i) => {
            let part: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
            print!("{}", text(&part, a.get(i + 1).map_or(false, |l| l == "de")));
        }
        None => eprintln!("[report] wrote {out}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(s: &str) -> Vec<String> {
        let mut v: Vec<String> = s.match_indices("{ $").map(|(i, _)| s[i + 3..].split(' ').next().unwrap().to_string()).collect();
        v.sort();
        v
    }

    /// both languages have every message with the same arguments
    #[test]
    fn messages_complete() {
        let mut seen = std::collections::HashSet::new();
        for (k, d, e) in MSG {
            assert!(seen.insert(*k), "{k} twice");
            assert_eq!(vars(d), vars(e), "{k}: arguments differ between German and English");
            assert!(!d.is_empty() && !e.is_empty());
        }
        for l in ftl(true).lines().chain(ftl(false).lines()) {
            assert!(l.split_once(" = ").map_or(false, |(_, v)| !v.trim().is_empty()), "empty message: {l}");
        }
    }

    /// the validation numbers are those of validation/results
    #[test]
    fn validation_numbers() {
        let root = env!("CARGO_MANIFEST_DIR");
        for v in &VALIDATION {
            let all = |f: &str| -> std::collections::HashMap<String, u64> {
                let t = std::fs::read_to_string(format!("{root}/validation/results/{}/{f}", v.id)).unwrap();
                let head: Vec<&str> = t.lines().next().unwrap().split('\t').collect();
                let row: Vec<&str> = t.lines().find(|l| l.starts_with("ALL\t")).unwrap().split('\t').collect();
                head.iter().zip(row).skip(1).filter_map(|(h, x)| x.parse().ok().map(|x| (h.to_string(), x))).collect()
            };
            let (e6, e23) = (all("eval6.tsv"), all("eval23.tsv"));
            assert_eq!((v.alleles, v.two, v.three), (e23["n"], e23["ok2"], e23["ok3"]), "{}", v.id);
            assert_eq!((v.safe4, v.safe4_exact), (e6["safe4"], e6["safe4_exact"]), "{}", v.id);
        }
    }

    /// a part from a small result: every message key used exists, the checksum holds, nothing unresolved
    #[test]
    fn part_from_result() {
        let dir = std::env::temp_dir().join(format!("hla-wgs-report-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("S1");
        let p = p.to_str().unwrap();
        let rep = json!({"tool": "hla-wgs", "version": "9.9.9", "schema": "genome.hla.v1", "database_release": "3.66.0", "input": ["S1.bam"], "analysis_time": 1795519947u64,
            "groups_loaded": true, "serology_loaded": false, "repeat_mask": true, "depth_estimate": 14.2,
            "genes": [
                {"gene": "A", "called": true, "copies": 2, "reads": 700, "alleles": [
                    {"allele": "A*24:02:01:01", "safe_call": "A*24:02:01:01", "resolution": 4, "margin": {"2": 50.0, "3": 40.0, "4": 30.0}, "alternative": {"2": "A*24:09N", "3": "A*24:02:192", "4": "A*24:02:01:36"}, "new_sites": []},
                    {"allele": "A*02:01:01:01", "safe_call": "A*02:01", "resolution": 2, "margin": {"2": 9.0, "3": 2.0, "4": 1.0}, "alternative": {"2": "A*02:09", "3": "A*02:01:02", "4": "A*02:01:01:02L"},
                     "new_sites": [{"pos": 100, "change": "C>T", "exon": true, "either_allele": false, "effect": "missense"}]}]},
                {"gene": "DRB4", "called": true, "copies": 0, "reads": 3, "alleles": []},
                {"gene": "DPB1", "called": false, "copies": 2, "reads": 12, "alleles": []},
                {"gene": "DQB1", "called": true, "copies": 1, "reads": 300, "alleles": [{"allele": "DQB1*06:02:01:01", "safe_call": "DQB1*06:02:01", "resolution": 3, "margin": {"2": 9.0, "3": 8.0, "4": 1.0}, "alternative": {"4": "DQB1*06:02:01:02"}, "new_sites": []}]}
            ],
            "pharmacogenetic_alleles": [{"allele": "B*57:01", "drugs": "abacavir", "source": "CPIC guideline", "copies": 1, "certain": true},
                                        {"allele": "A*31:01", "drugs": "carbamazepine", "source": "CPIC guideline", "copies": 0, "certain": true}]});
        std::fs::write(format!("{p}.json"), rep.to_string()).unwrap();
        let part = build(p).unwrap();
        // checksum over the part without itself
        let mut body = part.clone();
        let sum = body.as_object_mut().unwrap().remove("checksum").unwrap();
        assert_eq!(sum["value"], crc(serde_json::to_string(&body).unwrap().as_bytes()));
        // every key resolves in both languages
        fn keys(v: &Value, out: &mut Vec<String>) {
            match v {
                Value::Object(m) => {
                    if let Some(k) = m.get("key").and_then(|k| k.as_str()) {
                        out.push(k.to_string());
                    }
                    m.iter().filter(|(k, _)| *k != "messages").for_each(|(_, x)| keys(x, out));
                }
                Value::Array(a) => a.iter().for_each(|x| keys(x, out)),
                _ => {}
            }
        }
        let mut ks = vec![];
        keys(&part, &mut ks);
        for lang in ["de", "en"] {
            let m = part["messages"][lang].as_str().unwrap();
            for k in &ks {
                assert!(m.lines().any(|l| l.starts_with(&format!("{k} = "))), "{k} missing in {lang}");
            }
        }
        for de in [true, false] {
            let t = text(&part, de);
            assert!(!t.contains("{ $") && !t.contains("[pgx") && !t.contains("[hla"), "unresolved text:\n{t}");
        }
        let en = text(&part, false);
        assert!(en.contains("HLA-A: A*24:02:01:01 and A*02:01"), "{en}");
        assert!(en.contains("A*02:01: nearest allele at field 3 is A*02:01:02"), "{en}");
        assert!(en.contains("HLA-DQB1: DQB1*06:02:01, one copy"), "{en}");
        assert!(en.contains("HLA-DPB1: not determined (too few reads)"), "{en}");
        assert!(!en.contains("HLA-DRB4"), "absent DRB4 is no finding: {en}");
        assert!(en.contains("Carrier of HLA-B*57:01") && en.contains("Do not use abacavir"), "{en}");
        assert!(en.contains("Not a carrier of HLA-A*31:01"), "{en}");
        assert!(en.contains("possible new allele (differences: 1, of them changing the protein: 1)"), "{en}");
        assert!(en.contains("below 18x"), "{en}");
        assert!(en.contains("10.1038/clpt.2014.38") && !en.contains("PMC5847474"), "{en}");
        assert_eq!(part["quality"]["ok"], false);
        let de = text(&part, true);
        assert!(de.contains("HLA-A: A*24:02:01:01 und A*02:01") && de.contains("Abacavir nicht anwenden") && de.contains("betrifft Carbamazepin)"), "{de}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
